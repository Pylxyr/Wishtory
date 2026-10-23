//! Authkey / history URL extraction.
//! SAFETY: READ-ONLY. We open the game's Chromium cache file (`data_2`) for reading only
//! (with full share flags so a running game is never blocked). If that fails we copy it to
//! OUR temp directory and read the copy. Nothing is ever written to the game directory.

use crate::{discover, error::AppError, models::Game};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub fn extract(game: Game) -> Result<String, AppError> {
    let data_dir = discover::find_data_dir(game).ok_or(AppError::GameNotFound(game.display()))?;
    let cache = newest_data_2(&data_dir).ok_or(AppError::HistoryPageNotOpened)?;
    let bytes = read_shared(&cache)?;
    find_history_url(game, &bytes).ok_or(AppError::HistoryPageNotOpened)
}

/// Newest `webCaches/<version>/Cache/Cache_Data/data_2` by modification time.
fn newest_data_2(data_dir: &Path) -> Option<PathBuf> {
    fs::read_dir(data_dir.join("webCaches"))
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path().join("Cache").join("Cache_Data").join("data_2"))
        .filter(|p| p.is_file())
        .max_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
}

fn read_shared(path: &Path) -> Result<Vec<u8>, AppError> {
    let direct = (|| -> std::io::Result<Vec<u8>> {
        let mut opts = fs::OpenOptions::new();
        opts.read(true); // never write
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            opts.share_mode(0x1 | 0x2 | 0x4); // FILE_SHARE_READ | WRITE | DELETE
        }
        let mut buf = Vec::new();
        opts.open(path)?.read_to_end(&mut buf)?;
        Ok(buf)
    })();
    match direct {
        Ok(b) => Ok(b),
        Err(_) => {
            // Locked? Copy to our own temp folder and read the copy.
            let tmp = std::env::temp_dir().join(format!("wishtory-data_2-{}", std::process::id()));
            fs::copy(path, &tmp)?;
            let data = fs::read(&tmp);
            let _ = fs::remove_file(&tmp);
            Ok(data?)
        }
    }
}

/// Heuristic scan: the cache stores request URLs as plain ASCII. We take the LAST
/// (newest) URL that carries an authkey for this game.
fn find_history_url(game: Game, bytes: &[u8]) -> Option<String> {
    let re = regex::bytes::Regex::new(r"https://[\x21-\x7e]+").ok()?;
    let biz = format!("game_biz={}", game.as_str());
    re.find_iter(bytes)
        .filter_map(|m| std::str::from_utf8(m.as_bytes()).ok())
        .filter(|u| u.contains("authkey=") && u.contains(&biz))
        .last()
        .map(|s| s.to_string())
}
