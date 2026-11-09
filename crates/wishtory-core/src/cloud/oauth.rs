//! Google sign-in for a desktop app: the OAuth 2.0 authorization-code flow with PKCE (RFC 7636) and
//! a loopback redirect (RFC 8252). Wishtory asks for exactly one permission, the Drive *app data*
//! folder: a hidden folder that only this app can read, and that cannot see any other Drive file.
//!
//! The sign-in page is opened in the person's own browser, so Wishtory never sees the Google
//! password. The redirect goes to a one-shot listener on `127.0.0.1` that checks the `Host` header
//! and a random `state`, so another web page cannot feed it a code.

use crate::error::AppError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::{
    digest,
    rand::{SecureRandom, SystemRandom},
};
use serde::Deserialize;
use std::time::{Duration, Instant};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use url::Url;
use zeroize::Zeroizing;

/// The only permission requested.
pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";

/// How long the person has to finish signing in.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_REQUEST_BYTES: usize = 8 * 1024;
/// A browser that connects but never sends a request must not stall the listener.
const READ_TIMEOUT: Duration = Duration::from_secs(5);
/// Treat an access token as expired this long before it really is.
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct Endpoints {
    pub auth: &'static str,
    pub token: &'static str,
    pub revoke: &'static str,
}

impl Endpoints {
    pub const GOOGLE: Endpoints = Endpoints {
        auth: "https://accounts.google.com/o/oauth2/v2/auth",
        token: "https://oauth2.googleapis.com/token",
        revoke: "https://oauth2.googleapis.com/revoke",
    };
}

/// The app's OAuth client. Google's own guidance is that a desktop client secret is not confidential
/// (it ships inside every copy of the app); PKCE is what protects the exchange.
#[derive(Clone)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
}

impl Credentials {
    /// Set when the app is built (`WISHTORY_GOOGLE_CLIENT_ID` and `WISHTORY_GOOGLE_CLIENT_SECRET`).
    /// Builds without them have no cloud backup.
    pub fn built_in() -> Option<Credentials> {
        let id = option_env!("WISHTORY_GOOGLE_CLIENT_ID")?.trim();
        let secret = option_env!("WISHTORY_GOOGLE_CLIENT_SECRET")?.trim();
        (!id.is_empty() && !secret.is_empty()).then(|| Credentials {
            client_id: id.into(),
            client_secret: secret.into(),
        })
    }
}

/// A short-lived token for calling Drive.
pub struct Access {
    pub token: Zeroizing<String>,
    expires_at: Instant,
}

impl Access {
    pub fn is_fresh(&self) -> bool {
        self.expires_at > Instant::now() + EXPIRY_MARGIN
    }
}

pub struct SignedIn {
    pub refresh_token: Zeroizing<String>,
    pub access: Access,
}

/// Why a token could not be refreshed.
#[derive(Debug)]
pub enum TokenError {
    /// Google no longer honours the refresh token (access removed, or the sign-in expired).
    Revoked,
    Failed(AppError),
}

impl From<AppError> for TokenError {
    fn from(e: AppError) -> Self {
        Self::Failed(e)
    }
}
impl From<reqwest::Error> for TokenError {
    fn from(e: reqwest::Error) -> Self {
        Self::Failed(e.into())
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct TokenErrorBody {
    error: Option<String>,
}

pub struct Pkce {
    pub verifier: Zeroizing<String>,
    pub challenge: String,
}

pub fn random_token(bytes: usize) -> Result<String, AppError> {
    let mut buf = vec![0u8; bytes];
    SystemRandom::new()
        .fill(&mut buf)
        .map_err(|_| AppError::Io("The system could not provide random numbers.".into()))?;
    Ok(URL_SAFE_NO_PAD.encode(buf))
}

pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(digest::digest(&digest::SHA256, verifier.as_bytes()))
}

pub fn new_pkce() -> Result<Pkce, AppError> {
    let verifier = Zeroizing::new(random_token(48)?); // 64 characters; the RFC allows 43 to 128
    let challenge = challenge_for(&verifier);
    Ok(Pkce {
        verifier,
        challenge,
    })
}

pub struct OAuth {
    http: reqwest::Client,
    endpoints: Endpoints,
    credentials: Credentials,
}

impl OAuth {
    pub fn new(http: reqwest::Client, endpoints: Endpoints, credentials: Credentials) -> Self {
        Self {
            http,
            endpoints,
            credentials,
        }
    }

    pub fn authorization_url(
        &self,
        redirect_uri: &str,
        challenge: &str,
        state: &str,
    ) -> Result<Url, AppError> {
        Url::parse_with_params(
            self.endpoints.auth,
            [
                ("client_id", self.credentials.client_id.as_str()),
                ("redirect_uri", redirect_uri),
                ("response_type", "code"),
                ("scope", SCOPE),
                ("code_challenge", challenge),
                ("code_challenge_method", "S256"),
                ("state", state),
                // `offline` + `consent` make Google issue a refresh token every time.
                ("access_type", "offline"),
                ("prompt", "consent"),
            ],
        )
        .map_err(|e| AppError::Message(format!("Could not build the Google sign-in link: {e}")))
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
    ) -> Result<SignedIn, AppError> {
        let response = self
            .http
            .post(self.endpoints.token)
            .form(&[
                ("client_id", self.credentials.client_id.as_str()),
                ("client_secret", self.credentials.client_secret.as_str()),
                ("code", code),
                ("code_verifier", verifier),
                ("redirect_uri", redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(AppError::Message(
                "Google did not accept the sign-in. Please try again.".into(),
            ));
        }
        let body: TokenResponse = response.json().await?;
        if !body
            .scope
            .as_deref()
            .is_some_and(|granted| granted.split(' ').any(|s| s == SCOPE))
        {
            return Err(AppError::Message(
                "Google did not grant access to Wishtory's backup folder, so nothing was connected."
                    .into(),
            ));
        }
        let refresh_token = body.refresh_token.ok_or_else(|| {
            AppError::Message(
                "Google did not return a lasting sign-in. Remove Wishtory under your Google \
                 account's third-party access and try again."
                    .into(),
            )
        })?;
        Ok(SignedIn {
            refresh_token: Zeroizing::new(refresh_token),
            access: Access {
                token: Zeroizing::new(body.access_token),
                expires_at: Instant::now() + Duration::from_secs(body.expires_in),
            },
        })
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<Access, TokenError> {
        let response = self
            .http
            .post(self.endpoints.token)
            .form(&[
                ("client_id", self.credentials.client_id.as_str()),
                ("client_secret", self.credentials.client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let code = response
                .json::<TokenErrorBody>()
                .await
                .ok()
                .and_then(|b| b.error);
            return Err(if code.as_deref() == Some("invalid_grant") {
                TokenError::Revoked
            } else {
                TokenError::Failed(AppError::Api(format!("Google sign-in returned {status}")))
            });
        }
        let body: TokenResponse = response.json().await?;
        Ok(Access {
            token: Zeroizing::new(body.access_token),
            expires_at: Instant::now() + Duration::from_secs(body.expires_in),
        })
    }

    /// Tells Google to stop honouring a token. Best effort: the caller forgets the token either way.
    pub async fn revoke(&self, token: &str) -> Result<(), AppError> {
        let response = self
            .http
            .post(self.endpoints.revoke)
            .form(&[("token", token)])
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(AppError::Api(format!(
                "Google returned {} while disconnecting",
                response.status()
            )))
        }
    }
}

// ---- The one-shot loopback listener ----

pub struct Loopback {
    listener: TcpListener,
    port: u16,
}

enum Callback {
    /// Not ours (wrong path, host or state): answered with an error page and ignored.
    Ignore,
    Code(String),
    Declined,
}

impl Loopback {
    pub async fn bind() -> Result<Loopback, AppError> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = listener.local_addr()?.port();
        Ok(Loopback { listener, port })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }

    /// Waits for Google to send the browser back, and returns the authorization code.
    pub async fn wait_for_code(self, state: &str) -> Result<String, AppError> {
        self.wait_for_code_within(state, SIGN_IN_TIMEOUT).await
    }

    async fn wait_for_code_within(self, state: &str, limit: Duration) -> Result<String, AppError> {
        let wait = async {
            loop {
                let Ok((mut stream, peer)) = self.listener.accept().await else {
                    continue;
                };
                if !peer.ip().is_loopback() {
                    continue;
                }
                match self.handle(&mut stream, state).await {
                    Callback::Code(code) => return Ok(code),
                    Callback::Declined => {
                        return Err(AppError::Message("Sign-in was cancelled.".into()))
                    }
                    Callback::Ignore => {}
                }
            }
        };
        tokio::time::timeout(limit, wait).await.unwrap_or_else(|_| {
            Err(AppError::Message(
                "Sign-in took too long and was stopped. Try again when you are ready.".into(),
            ))
        })
    }

    async fn handle(&self, stream: &mut TcpStream, state: &str) -> Callback {
        let Some(request) = read_request(stream).await else {
            return Callback::Ignore;
        };
        let outcome = parse_callback(&request, self.port, state);
        let (status, page) = match outcome {
            Callback::Code(_) => ("200 OK", PAGE_OK),
            Callback::Declined => ("200 OK", PAGE_DECLINED),
            Callback::Ignore => ("404 Not Found", PAGE_NOT_FOUND),
        };
        let _ = respond(stream, status, page).await;
        outcome
    }
}

async fn read_request(stream: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    let read = async {
        loop {
            let n = stream.read(&mut chunk).await.ok()?;
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&chunk[..n]);
            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                return Some(());
            }
            if buf.len() > MAX_REQUEST_BYTES {
                return None;
            }
        }
    };
    tokio::time::timeout(READ_TIMEOUT, read).await.ok()??;
    String::from_utf8(buf).ok()
}

fn parse_callback(request: &str, port: u16, state: &str) -> Callback {
    let mut lines = request.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split(' ');
    let (method, target, version) = (first.next(), first.next(), first.next());
    if method != Some("GET") || !version.is_some_and(|v| v.starts_with("HTTP/1.")) {
        return Callback::Ignore;
    }
    // A web page cannot make the browser send another Host to 127.0.0.1 (DNS rebinding).
    let host_ok = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(':'))
        .any(|(name, value)| {
            name.eq_ignore_ascii_case("host") && value.trim() == format!("127.0.0.1:{port}")
        });
    if !host_ok {
        return Callback::Ignore;
    }
    let Some(url) = target.and_then(|t| Url::parse(&format!("http://127.0.0.1{t}")).ok()) else {
        return Callback::Ignore;
    };
    if url.path() != "/callback" {
        return Callback::Ignore;
    }
    let (mut code, mut error, mut got_state) = (None, None, None);
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "error" => error = Some(v.into_owned()),
            "state" => got_state = Some(v.into_owned()),
            _ => {}
        }
    }
    let state_ok = got_state.is_some_and(|s| same_secret(s.as_bytes(), state.as_bytes()));
    match (state_ok, code, error) {
        (true, Some(code), _) if !code.is_empty() => Callback::Code(code),
        (true, _, Some(_)) => Callback::Declined,
        _ => Callback::Ignore,
    }
}

/// Compares without stopping at the first difference.
fn same_secret(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\n\
         Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await
}

const PAGE_OK: &str = concat!(
    "<!doctype html><meta charset=utf-8><title>Wishtory</title><style>",
    "body{font:16px system-ui,sans-serif;max-width:32rem;margin:15vh auto;padding:0 1rem;color:#1b1f27}",
    "</style><h1>Wishtory is connected</h1><p>You can close this tab and go back to the app.</p>"
);
const PAGE_DECLINED: &str = concat!(
    "<!doctype html><meta charset=utf-8><title>Wishtory</title><style>",
    "body{font:16px system-ui,sans-serif;max-width:32rem;margin:15vh auto;padding:0 1rem;color:#1b1f27}",
    "</style><h1>Nothing was connected</h1><p>You can close this tab.</p>"
);
const PAGE_NOT_FOUND: &str =
    "<!doctype html><meta charset=utf-8><title>Not found</title><p>Not found.</p>";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::testutil::mock_once;

    fn request(path: &str, host: &str) -> String {
        format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: test\r\n\r\n")
    }

    #[test]
    fn pkce_matches_the_rfc_7636_example() {
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let p = new_pkce().unwrap();
        assert!((43..=128).contains(&p.verifier.len()));
        assert_eq!(p.challenge, challenge_for(&p.verifier));
        assert_ne!(new_pkce().unwrap().verifier, p.verifier);
    }

    #[test]
    fn the_authorization_link_asks_for_one_scope_with_pkce_and_state() {
        let oauth = OAuth::new(
            reqwest::Client::new(),
            Endpoints::GOOGLE,
            Credentials {
                client_id: "id".into(),
                client_secret: "secret".into(),
            },
        );
        let url = oauth
            .authorization_url("http://127.0.0.1:5/callback", "CHAL", "STATE")
            .unwrap();
        assert_eq!(url.host_str(), Some("accounts.google.com"));
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q["scope"], SCOPE);
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["code_challenge"], "CHAL");
        assert_eq!(q["state"], "STATE");
        assert!(!url.as_str().contains("secret"));
    }

    #[test]
    fn callback_accepts_only_the_right_host_path_and_state() {
        let ok = request("/callback?code=abc&state=S", "127.0.0.1:4000");
        assert!(matches!(parse_callback(&ok, 4000, "S"), Callback::Code(c) if c == "abc"));

        let declined = request("/callback?error=access_denied&state=S", "127.0.0.1:4000");
        assert!(matches!(
            parse_callback(&declined, 4000, "S"),
            Callback::Declined
        ));

        for bad in [
            request("/callback?code=abc&state=OTHER", "127.0.0.1:4000"),
            request("/callback?code=abc", "127.0.0.1:4000"),
            request("/callback?code=abc&state=S", "evil.example:4000"),
            request("/callback?code=abc&state=S", "127.0.0.1:4001"),
            request("/other?code=abc&state=S", "127.0.0.1:4000"),
            request("/callback?code=&state=S", "127.0.0.1:4000"),
            request("/callback?error=x&state=WRONG", "127.0.0.1:4000"),
            "POST /callback?code=abc&state=S HTTP/1.1\r\nHost: 127.0.0.1:4000\r\n\r\n".into(),
            "garbage".into(),
        ] {
            assert!(
                matches!(parse_callback(&bad, 4000, "S"), Callback::Ignore),
                "{bad:?}"
            );
        }
    }

    async fn browser(port: u16, path: &str, host: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(request(path, host).as_bytes()).await.unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn the_listener_ignores_strays_and_returns_the_real_code() {
        let lb = Loopback::bind().await.unwrap();
        let port = lb.port;
        let waiting = tokio::spawn(lb.wait_for_code("STATE"));
        let host = format!("127.0.0.1:{port}");

        let stray = browser(port, "/favicon.ico", &host).await;
        assert!(stray.starts_with("HTTP/1.1 404"));
        let forged = browser(port, "/callback?code=evil&state=NOPE", &host).await;
        assert!(forged.starts_with("HTTP/1.1 404"));
        let real = browser(port, "/callback?code=good&state=STATE", &host).await;
        assert!(real.starts_with("HTTP/1.1 200"));
        assert!(real.contains("Referrer-Policy: no-referrer"));
        assert_eq!(waiting.await.unwrap().unwrap(), "good");
    }

    #[tokio::test]
    async fn waiting_gives_up_after_the_limit() {
        let lb = Loopback::bind().await.unwrap();
        let err = lb
            .wait_for_code_within("S", Duration::from_millis(50))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("too long"));
    }

    fn oauth_at(url: &str) -> OAuth {
        crate::net::install_tls();
        let leaked: &'static str = Box::leak(format!("{url}/token").into_boxed_str());
        OAuth::new(
            reqwest::Client::new(),
            Endpoints {
                auth: "https://accounts.google.com/o/oauth2/v2/auth",
                token: leaked,
                revoke: leaked,
            },
            Credentials {
                client_id: "the-id".into(),
                client_secret: "the-secret".into(),
            },
        )
    }

    #[tokio::test]
    async fn exchanges_a_code_and_requires_the_scope_and_a_refresh_token() {
        crate::net::install_tls();
        let body: &'static [u8] = Box::leak(
            format!(r#"{{"access_token":"AT","expires_in":3599,"refresh_token":"RT","scope":"{SCOPE}"}}"#)
                .into_bytes()
                .into_boxed_slice(),
        );
        let (url, seen) = mock_once(200, body).await;
        let ok = oauth_at(&url)
            .exchange_code("CODE", "VERIFIER", "http://127.0.0.1:1/callback")
            .await
            .unwrap();
        assert_eq!(&**ok.refresh_token, "RT");
        assert!(ok.access.is_fresh());
        let request = String::from_utf8(seen.await.unwrap()).unwrap();
        assert!(
            request.contains("code_verifier=VERIFIER")
                && request.contains("grant_type=authorization_code")
        );

        let (url, _) = mock_once(
            200,
            r#"{"access_token":"AT","expires_in":3599,"refresh_token":"RT","scope":"email"}"#
                .as_bytes(),
        )
        .await;
        assert!(oauth_at(&url).exchange_code("c", "v", "r").await.is_err());
        let (url, _) = mock_once(200, r#"{"access_token":"AT","expires_in":3599,"scope":"https://www.googleapis.com/auth/drive.appdata"}"#.as_bytes()).await;
        assert!(oauth_at(&url).exchange_code("c", "v", "r").await.is_err());
    }

    #[tokio::test]
    async fn a_revoked_refresh_token_is_reported_as_such() {
        let (url, _) = mock_once(
            400,
            r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#
                .as_bytes(),
        )
        .await;
        assert!(matches!(
            oauth_at(&url).refresh("RT").await,
            Err(TokenError::Revoked)
        ));
        let (url, _) = mock_once(
            200,
            r#"{"access_token":"NEW","expires_in":3599}"#.as_bytes(),
        )
        .await;
        assert_eq!(&**oauth_at(&url).refresh("RT").await.unwrap().token, "NEW");
        let (url, _) = mock_once(500, "{}".as_bytes()).await;
        assert!(matches!(
            oauth_at(&url).refresh("RT").await,
            Err(TokenError::Failed(_))
        ));
    }
}
