//! Authkey / history URL extraction.
//! SAFETY: READ-ONLY. We open the game's Chromium cache file (`data_2`) for reading only
//! (with full share flags so a running game is never blocked). If that fails we copy it to
//! OUR temp directory and read the copy. Nothing is ever written to the game directory.

use crate::{discover, error::AppError, fsutil, models::Game};
use std::{
    fs,
    path::{Path, PathBuf},
};
use url::Url;

/// Candidate history URLs for a game, best guess first.
///
/// Walks the installs found in the game's logs (newest first), and inside each install the
/// `webCaches/<version>/.../data_2` files (newest first). The first cache that contains any
/// history URL wins. A pure cache read; the caller decides which candidate actually works.
pub fn extract_all(game: Game) -> Result<Vec<String>, AppError> {
    let dirs = discover::find_data_dirs(game);
    if dirs.is_empty() {
        return Err(AppError::GameNotFound(game.display()));
    }
    let mut read_error: Option<std::io::Error> = None;
    let mut read_any = false;
    for dir in &dirs {
        for cache in data_2_candidates(dir) {
            match fsutil::read_shared(&cache) {
                Ok(bytes) => {
                    read_any = true;
                    let urls = find_history_urls(game, &bytes);
                    if !urls.is_empty() {
                        return Ok(urls);
                    }
                }
                Err(e) => read_error = Some(e),
            }
        }
    }
    match (read_any, read_error) {
        (false, Some(e)) => Err(e.into()),
        _ => Err(AppError::HistoryPageNotOpened),
    }
}

/// The single most likely URL (kept for the manual "find link" command).
pub fn extract(game: Game) -> Result<String, AppError> {
    extract_all(game)?
        .into_iter()
        .next()
        .ok_or(AppError::HistoryPageNotOpened)
}

/// `webCaches/<version>/Cache/Cache_Data/data_2` files, newest first by modification time.
fn data_2_candidates(data_dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(data_dir.join("webCaches")) else {
        return vec![];
    };
    let mut v: Vec<(std::time::SystemTime, PathBuf)> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("Cache").join("Cache_Data").join("data_2"))
        .filter(|p| p.is_file())
        .map(|p| {
            (
                fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH),
                p,
            )
        })
        .collect();
    v.sort_by_key(|a| std::cmp::Reverse(a.0));
    v.into_iter().map(|(_, p)| p).collect()
}

/// Heuristic scan: the cache stores request URLs as plain ASCII. Returns every URL that carries
/// a non-empty authkey for this game, last occurrence in the file first, one per distinct authkey.
/// The file position is only a hint for recency (the cache reuses blocks), so callers should be
/// ready to try the next candidate when one has expired.
fn find_history_urls(game: Game, bytes: &[u8]) -> Vec<String> {
    let Ok(re) = regex::bytes::Regex::new(r"https://[\x21-\x7e]+") else {
        return vec![];
    };
    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for m in re.find_iter(bytes).collect::<Vec<_>>().into_iter().rev() {
        let Ok(text) = std::str::from_utf8(m.as_bytes()) else {
            continue;
        };
        let Ok(url) = Url::parse(text) else { continue };
        let mut authkey = None;
        let mut biz_ok = false;
        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "authkey" if !v.is_empty() => authkey = Some(v.into_owned()),
                "game_biz" if v.starts_with(game.as_str()) => biz_ok = true,
                _ => {}
            }
        }
        if let (Some(key), true) = (authkey, biz_ok) {
            if !seen.contains(&key) {
                seen.push(key);
                out.push(text.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(parts: &[&str]) -> Vec<u8> {
        let mut b = vec![0u8, 1, 2, 0xff];
        for p in parts {
            b.extend_from_slice(p.as_bytes());
            b.extend_from_slice(&[0, 0, 0, 7]);
        }
        b
    }

    #[test]
    fn finds_newest_first_dedupes_and_filters_other_games() {
        let a = "https://gs.hoyoverse.com/genshin/event/index.html?authkey_ver=1&authkey=AAA%2B&game_biz=hk4e_global#/log";
        let b =
            "https://gs.hoyoverse.com/genshin/event/index.html?authkey=BBB&game_biz=hk4e_global";
        let sr = "https://hkrpg.hoyoverse.com/x?authkey=ZZZ&game_biz=hkrpg_global";
        let nap = "https://zzz.hoyoverse.com/x?authkey=NAP&game_biz=nap_global";
        let no_key = "https://gs.hoyoverse.com/x?game_biz=hk4e_global";
        let data = blob(&[a, no_key, sr, nap, b, a]);
        let urls = find_history_urls(Game::Hk4e, &data);
        assert_eq!(urls, vec![a.to_string(), b.to_string()]);
        assert_eq!(find_history_urls(Game::Hkrpg, &data), vec![sr.to_string()]);
        assert_eq!(find_history_urls(Game::Nap, &data), vec![nap.to_string()]);
    }
}
