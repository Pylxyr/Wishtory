//! A tiny HTTP server for tests: answers one request and hands back what it received.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

pub async fn mock_once(status: u16, body: &'static [u8]) -> (String, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            let n = s.read(&mut chunk).await.unwrap();
            buf.extend_from_slice(&chunk[..n]);
            let split = buf.windows(4).position(|w| w == b"\r\n\r\n");
            if let Some(at) = split {
                let head = String::from_utf8_lossy(&buf[..at]).to_ascii_lowercase();
                let len = head
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if buf.len() >= at + 4 + len {
                    break;
                }
            }
            if n == 0 {
                break;
            }
        }
        let head = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        s.write_all(head.as_bytes()).await.unwrap();
        s.write_all(body).await.unwrap();
        s.shutdown().await.unwrap();
        buf
    });
    (url, task)
}

// ---- A stateful fake of Google's token and Drive endpoints ----

use std::sync::{Arc, Mutex};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct FakeFile {
    pub id: String,
    pub name: String,
    pub created: String,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct FakeState {
    pub files: Vec<FakeFile>,
    pub next: u32,
    pub token_calls: u32,
    pub revoked: Vec<String>,
    /// Answer token refreshes with `invalid_grant`.
    pub refresh_invalid: bool,
    /// Answer the next Drive call with 401.
    pub reject_next_drive_call: bool,
}

pub struct FakeGoogle {
    pub url: String,
    pub state: Arc<Mutex<FakeState>>,
}

impl FakeGoogle {
    pub async fn start() -> FakeGoogle {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(FakeState::default()));
        let shared = state.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let shared = shared.clone();
                tokio::spawn(async move { serve(stream, shared).await });
            }
        });
        FakeGoogle { url, state }
    }
}

fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

async fn serve(mut s: TcpStream, state: Arc<Mutex<FakeState>>) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let (head, body_start) = loop {
        let n = s.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(at) = find(&buf, b"\r\n\r\n", 0) {
            break (String::from_utf8_lossy(&buf[..at]).into_owned(), at + 4);
        }
    };
    let header = |name: &str| {
        head.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
        })
    };
    let len: usize = header("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    while buf.len() < body_start + len {
        let n = s.read(&mut chunk).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = buf[body_start..].to_vec();
    let mut first = head.lines().next().unwrap_or_default().split(' ');
    let (method, target) = (
        first.next().unwrap_or_default(),
        first.next().unwrap_or_default(),
    );
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    let (status, ctype, out): (u16, &str, Vec<u8>) = {
        let mut st = state.lock().unwrap();
        let drive_route = path.starts_with("/drive/") || path.starts_with("/upload/");
        if drive_route && st.reject_next_drive_call {
            st.reject_next_drive_call = false;
            (401, "application/json", b"{}".to_vec())
        } else if drive_route
            && !header("authorization").is_some_and(|a| a.starts_with("Bearer AT"))
        {
            (401, "application/json", b"{}".to_vec())
        } else {
            match (method, path) {
                ("POST", "/token") => {
                    st.token_calls += 1;
                    if st.refresh_invalid {
                        (
                            400,
                            "application/json",
                            br#"{"error":"invalid_grant"}"#.to_vec(),
                        )
                    } else {
                        let json = format!(
                            r#"{{"access_token":"AT{}","expires_in":3599}}"#,
                            st.token_calls
                        );
                        (200, "application/json", json.into_bytes())
                    }
                }
                ("POST", "/revoke") => {
                    let form = String::from_utf8_lossy(&body).into_owned();
                    st.revoked
                        .push(form.trim_start_matches("token=").to_string());
                    (200, "application/json", b"{}".to_vec())
                }
                ("GET", "/drive/v3/about") => (
                    200,
                    "application/json",
                    br#"{"user":{"emailAddress":"me@example.com"}}"#.to_vec(),
                ),
                ("GET", "/drive/v3/files") => {
                    let mut files = st.files.clone();
                    files.sort_by(|a, b| b.created.cmp(&a.created));
                    let items: Vec<String> = files
                        .iter()
                        .map(|f| {
                            format!(
                                r#"{{"id":"{}","name":"{}","size":"{}","createdTime":"{}"}}"#,
                                f.id,
                                f.name,
                                f.data.len(),
                                f.created
                            )
                        })
                        .collect();
                    (
                        200,
                        "application/json",
                        format!(r#"{{"files":[{}]}}"#, items.join(",")).into_bytes(),
                    )
                }
                ("GET", p) if p.starts_with("/drive/v3/files/") && query.contains("alt=media") => {
                    let id = &p["/drive/v3/files/".len()..];
                    match st.files.iter().find(|f| f.id == id) {
                        Some(f) => (200, "application/octet-stream", f.data.clone()),
                        None => (404, "application/json", b"{}".to_vec()),
                    }
                }
                ("DELETE", p) if p.starts_with("/drive/v3/files/") => {
                    let id = &p["/drive/v3/files/".len()..];
                    st.files.retain(|f| f.id != id);
                    (204, "application/json", Vec::new())
                }
                ("POST", "/upload/drive/v3/files") => {
                    let boundary = header("content-type")
                        .and_then(|c| c.split("boundary=").nth(1).map(str::to_string))
                        .unwrap_or_default();
                    let delim = format!("\r\n--{boundary}");
                    let meta_at = find(&body, b"\r\n\r\n", 0).unwrap() + 4;
                    let meta_end = find(&body, delim.as_bytes(), meta_at).unwrap();
                    let meta: serde_json::Value =
                        serde_json::from_slice(&body[meta_at..meta_end]).unwrap();
                    assert_eq!(meta["parents"][0], "appDataFolder");
                    let data_at = find(&body, b"\r\n\r\n", meta_end).unwrap() + 4;
                    let data_end = find(&body, format!("{delim}--").as_bytes(), data_at).unwrap();
                    st.next += 1;
                    let file = FakeFile {
                        id: format!("FILE{:06}ABCD", st.next),
                        name: meta["name"].as_str().unwrap().to_string(),
                        created: format!(
                            "2026-11-09T04:{:02}:{:02}.000Z",
                            st.next / 60,
                            st.next % 60
                        ),
                        data: body[data_at..data_end].to_vec(),
                    };
                    let json = format!(
                        r#"{{"id":"{}","name":"{}","size":"{}","createdTime":"{}"}}"#,
                        file.id,
                        file.name,
                        file.data.len(),
                        file.created
                    );
                    st.files.push(file);
                    (200, "application/json", json.into_bytes())
                }
                _ => (404, "application/json", b"{}".to_vec()),
            }
        }
    };
    let head = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        out.len()
    );
    let _ = s.write_all(head.as_bytes()).await;
    let _ = s.write_all(&out).await;
    let _ = s.shutdown().await;
}
