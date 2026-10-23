//! Automatic game discovery.
//! SAFETY: READ-ONLY. We only `fs::read` the log files the game itself writes under
//! %USERPROFILE%\AppData\LocalLow and inspect directory metadata. Nothing is written.

use crate::models::{Game, GameInstall};
use regex::Regex;
use std::{fs, path::PathBuf, time::SystemTime};

fn locallow() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData").join("LocalLow"))
}

fn modified(p: &PathBuf) -> SystemTime {
    fs::metadata(p).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Existing log files for a game, newest first (global + CN folder names).
fn log_candidates(game: Game) -> Vec<PathBuf> {
    let Some(low) = locallow() else { return vec![] };
    let (dirs, files): (&[&str], &[&str]) = match game {
        Game::Hk4e => (&["miHoYo/Genshin Impact", "miHoYo/原神"], &["Player.log", "output_log.txt", "Player-prev.log"]),
        Game::Hkrpg => (&["Cognosphere/Star Rail", "miHoYo/崩坏：星穹铁道"], &["Player.log", "Player-prev.log"]),
        Game::Nap => (&["miHoYo/ZenlessZoneZero", "miHoYo/绝区零"], &["Player.log", "Player-prev.log"]),
    };
    let mut found: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|d| files.iter().map(move |f| (d, f)))
        .map(|(d, f)| low.join(d).join(f))
        .filter(|p| p.is_file())
        .collect();
    found.sort_by_key(|p| std::cmp::Reverse(modified(p)));
    found
}

fn data_dir_names(game: Game) -> &'static [&'static str] {
    match game {
        Game::Hk4e => &["GenshinImpact_Data", "YuanShen_Data"],
        Game::Hkrpg => &["StarRail_Data"],
        Game::Nap => &["ZenlessZoneZero_Data"],
    }
}

/// Finds the game's `*_Data` directory by reading the Unity log (read-only).
pub fn find_data_dir(game: Game) -> Option<PathBuf> {
    let alternatives = data_dir_names(game).iter().map(|n| regex::escape(n)).collect::<Vec<_>>().join("|");
    let re = Regex::new(&format!(r"(?i)([A-Za-z]:[/\\][^\r\n\x22]*?(?:{alternatives}))")).ok()?;
    for log in log_candidates(game) {
        let Ok(bytes) = fs::read(&log) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        if let Some(cap) = re.captures_iter(&text).last() {
            let dir = PathBuf::from(cap[1].replace('/', "\\"));
            if dir.is_dir() {
                return Some(dir);
            }
        }
    }
    None
}

pub fn discover_all() -> Vec<GameInstall> {
    Game::ALL
        .iter()
        .map(|&game| {
            let logs = log_candidates(game);
            GameInstall {
                game,
                found: !logs.is_empty(),
                log_path: logs.first().map(|p| p.to_string_lossy().into_owned()),
                data_dir: find_data_dir(game).map(|p| p.to_string_lossy().into_owned()),
            }
        })
        .collect()
}
