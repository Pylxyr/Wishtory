//! Checks this project's GitHub releases for a newer Wishtory. It only compares version numbers:
//! nothing is downloaded or installed, and no account or wish data is sent (the request carries
//! the app name and version in its User-Agent, like every request this app makes to GitHub).

use crate::{about::RELEASES, catalog::download_limited, error::AppError, logging::Level};
use semver::Version;
use serde::{Deserialize, Serialize};

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

/// Reads `1.2.3`, `v1.2.3` and `1.2.3-rc1`, ignoring `+build`. Tags such as `catalog-v1` are
/// `None`. Ordering follows semantic versioning: a release is newer than its own pre-releases.
fn parse_version(tag: &str) -> Option<Version> {
    let tag = tag.trim();
    Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()
}

/// The newest release this build should offer. A release build is only offered releases; a
/// pre-release build is offered both, so it can move on to the next candidate or the final one.
fn evaluate(current: &str, releases: Vec<Release>) -> Result<AppUpdate, AppError> {
    let current_version = parse_version(current)
        .ok_or_else(|| AppError::Message("This build has an unreadable version.".into()))?;
    let releases_only = current_version.pre.is_empty();
    let release_page = format!("{RELEASES}/");
    let newest = releases
        .into_iter()
        .filter(|r| !r.draft && r.html_url.starts_with(&release_page))
        .filter_map(|r| parse_version(&r.tag_name).map(|v| (v, r)))
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
pub async fn check_app_update() -> Result<AppUpdate, AppError> {
    let bytes = download_limited(RELEASES_API, MAX_RESPONSE_BYTES)
        .await
        .map_err(|e| AppError::Message(format!("Could not reach GitHub ({e}).")))?;
    let releases: Vec<Release> = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Message("GitHub sent a response Wishtory could not read.".into()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, prerelease: bool) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: format!("{RELEASES}/{tag}"),
            draft: false,
            prerelease,
            published_at: None,
        }
    }

    #[test]
    fn reads_tags_the_way_releases_are_named() {
        for tag in [
            "1.2.3",
            "v1.2.3",
            " v1.2.3 ",
            "v1.2.3-rc1",
            "v1.2.3+build.5",
        ] {
            assert!(parse_version(tag).is_some(), "{tag}");
        }
        for tag in ["catalog-v1", "v1.2", "v1.2.3.4", "latest", ""] {
            assert!(parse_version(tag).is_none(), "{tag}");
        }
    }

    #[test]
    fn orders_releases_after_their_candidates() {
        // `rc2` is one text identifier, `rc.2` / `rc.10` are dotted numbers that compare as numbers.
        let order = [
            "v0.0.1-rc1",
            "v0.0.1-rc2",
            "v0.0.1",
            "v0.0.2-rc.2",
            "v0.0.2-rc.10",
            "v0.0.2",
        ];
        for pair in order.windows(2) {
            assert!(
                parse_version(pair[0]).unwrap() < parse_version(pair[1]).unwrap(),
                "{} < {}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn offers_the_newest_suitable_release() {
        let list = || {
            vec![
                release("catalog-v1", false),
                release("v0.0.1-rc2", true),
                release("v0.0.2", false),
                release("v0.0.3-rc1", true),
            ]
        };
        // A release build is never offered a candidate, and ignores the rolling catalog tag.
        let update = evaluate("0.0.1", list()).unwrap();
        assert!(update.available);
        assert_eq!(update.latest, "0.0.2");
        // A candidate build may move on to the next candidate.
        let update = evaluate("0.0.2", list()).unwrap();
        assert!(!update.available);
        let update = evaluate("0.0.3-rc0", list()).unwrap();
        assert_eq!(update.latest, "0.0.3-rc1");
        assert!(update.prerelease);
        // Drafts and foreign release pages are ignored.
        let mut foreign = release("v9.9.9", false);
        foreign.html_url = "https://example.com/v9.9.9".into();
        let mut draft = release("v8.8.8", false);
        draft.draft = true;
        assert!(!evaluate("0.0.2", vec![foreign, draft]).unwrap().available);
        assert!(evaluate("not-a-version", vec![]).is_err());
    }
}
