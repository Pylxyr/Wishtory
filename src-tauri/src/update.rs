//! Checks this project's GitHub releases for a newer Wishtory. It only compares version numbers:
//! nothing is downloaded or installed, and no account or wish data is sent (the request carries
//! the app name and version in its User-Agent, like every request this app makes to GitHub).

use crate::{about::RELEASES, catalog::download_limited, logging::Level};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const RELEASES_API: &str = "https://api.github.com/repos/Pylxyr/Wishtory/releases?per_page=20";
const MAX_RESPONSE_BYTES: usize = 1 << 20;

#[derive(Serialize)]
pub struct AppUpdate {
    current: String,
    latest: String,
    available: bool,
    /// The release page to open. Only ever a release page of this project.
    url: String,
    prerelease: bool,
    published_at: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
}

/// One dot-separated pre-release part. Numbers sort before text, as in semantic versioning.
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Part {
    Num(u64),
    Text(String),
}

#[derive(PartialEq, Eq, Debug)]
struct Version {
    core: [u64; 3],
    pre: Vec<Part>,
}

impl Version {
    /// Reads `1.2.3`, `v1.2.3`, `1.2.3-rc1` and ignores `+build`. Tags such as `catalog-v1` are `None`.
    fn parse(tag: &str) -> Option<Self> {
        let s = tag.trim();
        let s = s.strip_prefix('v').unwrap_or(s);
        let s = s.split('+').next()?;
        let (core, pre) = match s.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (s, None),
        };
        let mut parts = core.split('.');
        let mut nums = [0u64; 3];
        for n in &mut nums {
            *n = parts.next()?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(p) => p
                .split('.')
                .map(|id| {
                    if id.is_empty() {
                        None
                    } else {
                        Some(
                            id.parse::<u64>()
                                .map_or_else(|_| Part::Text(id.to_string()), Part::Num),
                        )
                    }
                })
                .collect::<Option<Vec<_>>>()?,
        };
        Some(Self { core: nums, pre })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| {
            // A release is newer than any pre-release of the same numbers.
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The newest release this build should offer. A release build is only offered releases; a
/// pre-release build is offered both, so it can move on to the next candidate or the final one.
fn evaluate(current: &str, releases: Vec<Release>) -> Result<AppUpdate, String> {
    let current_version = Version::parse(current).ok_or("This build has an unreadable version.")?;
    let releases_only = current_version.pre.is_empty();
    let release_page = format!("{RELEASES}/");
    let newest = releases
        .into_iter()
        .filter(|r| !r.draft && r.html_url.starts_with(&release_page))
        .filter_map(|r| Version::parse(&r.tag_name).map(|v| (v, r)))
        .filter(|(v, r)| !(releases_only && (r.prerelease || !v.pre.is_empty())))
        .max_by(|a, b| a.0.cmp(&b.0));
    Ok(match newest {
        Some((version, release)) if version > current_version => AppUpdate {
            current: current.to_string(),
            latest: release.tag_name.trim_start_matches('v').to_string(),
            available: true,
            url: release.html_url,
            prerelease: release.prerelease || !version.pre.is_empty(),
            published_at: release.published_at.unwrap_or_default(),
        },
        _ => AppUpdate {
            current: current.to_string(),
            latest: current.to_string(),
            available: false,
            url: RELEASES.to_string(),
            prerelease: false,
            published_at: String::new(),
        },
    })
}

#[tauri::command]
pub async fn check_app_update() -> Result<AppUpdate, String> {
    let bytes = download_limited(RELEASES_API, MAX_RESPONSE_BYTES)
        .await
        .map_err(|e| format!("Could not reach GitHub ({e})."))?;
    let releases: Vec<Release> = serde_json::from_slice(&bytes)
        .map_err(|_| "GitHub sent a response Wishtory could not read.".to_string())?;
    let update = evaluate(env!("CARGO_PKG_VERSION"), releases)?;
    wlog!(
        Level::Debug,
        "app update check: current {}, latest {}, available {}",
        update.current,
        update.latest,
        update.available
    );
    Ok(update)
}
