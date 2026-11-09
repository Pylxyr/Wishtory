//! The few Google Drive calls Wishtory makes, all against the hidden *app data* folder
//! (`spaces=appDataFolder`). That folder belongs to this app alone: it holds only Wishtory's own
//! backup files and Wishtory cannot see anything else in the person's Drive.

use crate::{cloud::oauth::random_token, error::AppError};
use serde::Deserialize;
use serde_json::json;

const API: &str = "https://www.googleapis.com/drive/v3";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
const NAME_PREFIX: &str = "wishtory-backup-";
const NAME_SUFFIX: &str = ".wshb";
const MAX_LIST_PAGES: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupFile {
    pub id: String,
    /// When Drive stored it, as RFC 3339 text.
    pub created: String,
    pub size: u64,
}

#[derive(Debug)]
pub enum DriveError {
    /// The access token was rejected; refreshing it and trying once more is worth it.
    Unauthorized,
    Failed(AppError),
}

impl From<AppError> for DriveError {
    fn from(e: AppError) -> Self {
        Self::Failed(e)
    }
}
impl From<reqwest::Error> for DriveError {
    fn from(e: reqwest::Error) -> Self {
        Self::Failed(e.into())
    }
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<RawFile>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct RawFile {
    id: String,
    name: String,
    /// Drive sends sizes as strings.
    size: Option<String>,
    #[serde(rename = "createdTime")]
    created_time: Option<String>,
}

impl RawFile {
    fn into_backup(self) -> Option<BackupFile> {
        (self.name.starts_with(NAME_PREFIX) && self.name.ends_with(NAME_SUFFIX)).then(|| {
            BackupFile {
                id: self.id,
                created: self.created_time.unwrap_or_default(),
                size: self.size.and_then(|s| s.parse().ok()).unwrap_or(0),
            }
        })
    }
}

/// Backup file ids come from Drive and travel through the UI. Only the characters Drive uses are
/// accepted, so an id can never change the path of a request.
pub fn is_valid_id(id: &str) -> bool {
    (6..=128).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `wishtory-backup-20261109T041500Z.wshb`
pub fn backup_name(now: chrono::DateTime<chrono::Utc>) -> String {
    format!("{NAME_PREFIX}{}{NAME_SUFFIX}", now.format("%Y%m%dT%H%M%SZ"))
}

pub struct Drive {
    http: reqwest::Client,
    api: String,
    upload: String,
}

impl Drive {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            api: API.into(),
            upload: UPLOAD.into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn at(http: reqwest::Client, base: &str) -> Self {
        Self {
            http,
            api: format!("{base}/drive/v3"),
            upload: format!("{base}/upload/drive/v3/files"),
        }
    }

    /// The Google account's address, so the person can see which account is connected.
    pub async fn account_email(&self, token: &str) -> Result<String, DriveError> {
        #[derive(Deserialize)]
        struct About {
            user: Option<User>,
        }
        #[derive(Deserialize)]
        struct User {
            #[serde(rename = "emailAddress")]
            email: Option<String>,
        }
        let response = self
            .http
            .get(format!("{}/about", self.api))
            .query(&[("fields", "user(emailAddress)")])
            .bearer_auth(token)
            .send()
            .await?;
        let about: About = check(response).await?.json().await?;
        Ok(about.user.and_then(|u| u.email).unwrap_or_default())
    }

    /// Newest first.
    pub async fn list(&self, token: &str) -> Result<Vec<BackupFile>, DriveError> {
        let mut out = Vec::new();
        let mut page: Option<String> = None;
        for _ in 0..MAX_LIST_PAGES {
            let mut query = vec![
                ("spaces", "appDataFolder".to_string()),
                (
                    "q",
                    format!("name contains '{NAME_PREFIX}' and trashed = false"),
                ),
                ("orderBy", "createdTime desc".into()),
                ("pageSize", "100".into()),
                (
                    "fields",
                    "nextPageToken,files(id,name,size,createdTime)".into(),
                ),
            ];
            if let Some(p) = &page {
                query.push(("pageToken", p.clone()));
            }
            let response = self
                .http
                .get(format!("{}/files", self.api))
                .query(&query)
                .bearer_auth(token)
                .send()
                .await?;
            let list: FileList = check(response).await?.json().await?;
            out.extend(list.files.into_iter().filter_map(RawFile::into_backup));
            match list.next_page_token {
                Some(next) => page = Some(next),
                None => break,
            }
        }
        Ok(out)
    }

    pub async fn upload(
        &self,
        token: &str,
        name: &str,
        bytes: &[u8],
    ) -> Result<BackupFile, DriveError> {
        let boundary = format!("wishtory{}", random_token(18)?.replace(['-', '_'], "x"));
        let metadata = json!({
            "name": name,
            "parents": ["appDataFolder"],
            "mimeType": "application/octet-stream",
        });
        let mut body = Vec::with_capacity(bytes.len() + 512);
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata}\r\n\
                 --{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());
        let response = self
            .http
            .post(&self.upload)
            .query(&[
                ("uploadType", "multipart"),
                ("fields", "id,name,size,createdTime"),
            ])
            .bearer_auth(token)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/related; boundary={boundary}"),
            )
            .body(body)
            .send()
            .await?;
        let file: RawFile = check(response).await?.json().await?;
        file.into_backup().ok_or_else(|| {
            DriveError::Failed(AppError::Api(
                "Drive stored the backup under an unexpected name".into(),
            ))
        })
    }

    pub async fn download(
        &self,
        token: &str,
        id: &str,
        max_bytes: usize,
    ) -> Result<Vec<u8>, DriveError> {
        check_id(id)?;
        let mut response = check(
            self.http
                .get(format!("{}/files/{id}", self.api))
                .query(&[("alt", "media")])
                .bearer_auth(token)
                .send()
                .await?,
        )
        .await?;
        if response
            .content_length()
            .is_some_and(|n| n > max_bytes as u64)
        {
            return Err(too_big());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > max_bytes {
                return Err(too_big());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    pub async fn delete(&self, token: &str, id: &str) -> Result<(), DriveError> {
        check_id(id)?;
        let response = self
            .http
            .delete(format!("{}/files/{id}", self.api))
            .bearer_auth(token)
            .send()
            .await?;
        // Already gone is as good as deleted.
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        check(response).await.map(|_| ())
    }
}

fn check_id(id: &str) -> Result<(), DriveError> {
    if is_valid_id(id) {
        Ok(())
    } else {
        Err(AppError::Message("That is not a backup Wishtory can open.".into()).into())
    }
}

fn too_big() -> DriveError {
    AppError::Format(
        "the backup file is larger than 64 MB, so it cannot be a Wishtory backup".into(),
    )
    .into()
}

/// Turns Drive's HTTP errors into messages written for the person. Response bodies are not shown:
/// they are Google's text, not ours, and may repeat parts of the request.
async fn check(response: reqwest::Response) -> Result<reqwest::Response, DriveError> {
    use reqwest::StatusCode as S;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    Err(match status {
        S::UNAUTHORIZED => DriveError::Unauthorized,
        S::FORBIDDEN => AppError::Message(
            "Google Drive refused the request. Your Drive storage may be full, or access to \
             Wishtory was changed in your Google account."
                .into(),
        )
        .into(),
        S::NOT_FOUND => {
            AppError::Message("That backup no longer exists in your Drive.".into()).into()
        }
        S::TOO_MANY_REQUESTS => AppError::RateLimited.into(),
        s => AppError::Api(format!("Google Drive returned {s}")).into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::testutil::mock_once;

    fn drive_at(url: &str) -> Drive {
        crate::net::install_tls();
        Drive::at(reqwest::Client::new(), url)
    }

    #[test]
    fn ids_cannot_change_a_request_path() {
        assert!(is_valid_id("1AbC_dEf-GhIjK"));
        for bad in [
            "",
            "short",
            "../../files",
            "a/b/c/d/e/f",
            "id?x=1",
            "id with space",
            "ünïcode-id-12",
        ] {
            assert!(!is_valid_id(bad), "{bad:?}");
        }
    }

    #[test]
    fn names_are_stable_and_sortable() {
        let t = chrono::DateTime::parse_from_rfc3339("2026-11-09T04:15:00Z")
            .unwrap()
            .to_utc();
        assert_eq!(backup_name(t), "wishtory-backup-20261109T041500Z.wshb");
    }

    #[tokio::test]
    async fn lists_only_wishtory_backups_and_asks_for_the_app_folder() {
        crate::net::install_tls();
        let (url, seen) = mock_once(
            200,
            br#"{"files":[
                {"id":"AAAAAAAAAA","name":"wishtory-backup-20261109T041500Z.wshb","size":"1234","createdTime":"2026-11-09T04:15:01.000Z"},
                {"id":"BBBBBBBBBB","name":"something-else.txt","size":"5","createdTime":"2026-11-09T04:15:01.000Z"}]}"#,
        )
        .await;
        let list = drive_at(&url).list("TOKEN").await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].id.as_str(), list[0].size), ("AAAAAAAAAA", 1234));
        let request = String::from_utf8(seen.await.unwrap()).unwrap();
        assert!(request.contains("spaces=appDataFolder"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer token"));
    }

    #[tokio::test]
    async fn uploads_into_the_app_folder_as_multipart() {
        let (url, seen) = mock_once(
            200,
            br#"{"id":"CCCCCCCCCC","name":"wishtory-backup-20261109T041500Z.wshb","size":"3","createdTime":"2026-11-09T04:15:01.000Z"}"#,
        )
        .await;
        let file = drive_at(&url)
            .upload("T", "wishtory-backup-20261109T041500Z.wshb", &[1, 2, 3])
            .await
            .unwrap();
        assert_eq!(file.id, "CCCCCCCCCC");
        let request = seen.await.unwrap();
        let text = String::from_utf8_lossy(&request);
        assert!(text.contains("multipart/related; boundary="));
        assert!(text.contains(r#""parents":["appDataFolder"]"#));
        assert!(request.windows(3).any(|w| w == [1, 2, 3]));
    }

    #[tokio::test]
    async fn maps_http_errors_and_enforces_the_download_cap() {
        let (url, _) = mock_once(401, b"{}").await;
        assert!(matches!(
            drive_at(&url).list("T").await,
            Err(DriveError::Unauthorized)
        ));

        let (url, _) = mock_once(403, b"{}").await;
        assert!(matches!(
            drive_at(&url).list("T").await,
            Err(DriveError::Failed(_))
        ));

        let (url, _) = mock_once(200, &[7u8; 100]).await;
        assert!(drive_at(&url)
            .download("T", "AAAAAAAAAA", 10)
            .await
            .is_err());
        let (url, _) = mock_once(200, &[7u8; 100]).await;
        assert_eq!(
            drive_at(&url)
                .download("T", "AAAAAAAAAA", 1000)
                .await
                .unwrap()
                .len(),
            100
        );

        // An id that is not an id never reaches the network.
        assert!(drive_at("http://127.0.0.1:1")
            .download("T", "../x", 10)
            .await
            .is_err());
        assert!(drive_at("http://127.0.0.1:1")
            .delete("T", "a/b/c/d/e/f")
            .await
            .is_err());
    }

    #[tokio::test]
    async fn deleting_something_already_gone_is_fine() {
        let (url, _) = mock_once(404, b"{}").await;
        assert!(drive_at(&url).delete("T", "AAAAAAAAAA").await.is_ok());
    }
}
