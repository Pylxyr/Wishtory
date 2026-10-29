//! Automatic game discovery.
//! SAFETY: READ-ONLY. We only read the log files the game itself writes under
//! %USERPROFILE%\AppData\LocalLow and inspect directory metadata. Nothing is written.

use crate::{
    fsutil,
    models::{Game, GameInstall},
};
use regex::Regex;
use std::sync::LazyLock;
use std::{fs, path::PathBuf, time::SystemTime};

fn locallow() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join("AppData").join("LocalLow"))
}

fn modified(p: &PathBuf) -> SystemTime {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Existing log files for a game, newest first (global + CN folder names).
fn log_candidates(game: Game) -> Vec<PathBuf> {
    let Some(low) = locallow() else { return vec![] };
    let (dirs, files): (&[&str], &[&str]) = match game {
        Game::Hk4e => (
            &["miHoYo/Genshin Impact", "miHoYo/原神"],
            &[
                "Player.log",
                "output_log.txt",
                "Player-prev.log",
                "output_log.txt.last",
            ],
        ),
        Game::Hkrpg => (
            &["Cognosphere/Star Rail", "miHoYo/崩坏：星穹铁道"],
            &["Player.log", "Player-prev.log"],
        ),
        Game::Nap => (
            &["miHoYo/ZenlessZoneZero", "miHoYo/绝区零"],
            &["Player.log", "Player-prev.log"],
        ),
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

pub fn data_dir_names(game: Game) -> &'static [&'static str] {
    match game {
        Game::Hk4e => &["GenshinImpact_Data", "YuanShen_Data"],
        Game::Hkrpg => &["StarRail_Data"],
        Game::Nap => &["ZenlessZoneZero_Data"],
    }
}

/// True when `name` is the name of any known game data folder (case-insensitive).
pub fn is_game_data_dir_name(name: &str) -> bool {
    Game::ALL
        .iter()
        .flat_map(|g| data_dir_names(*g))
        .any(|n| n.eq_ignore_ascii_case(name))
}

/// Last install path mentioned in a Unity log text, if any.
fn last_data_dir_in(text: &str, re: &Regex) -> Option<PathBuf> {
    re.captures_iter(text)
        .last()
        .map(|cap| PathBuf::from(cap[1].replace('/', "\\")))
}

/// Unity writes the install path in the first lines of `Player.log`; the rest of a log can be
/// several MB of gameplay noise that is not worth reading on every discovery or export.
const LOG_HEAD_BYTES: u64 = 1024 * 1024;

fn data_dir_pattern(game: Game) -> Regex {
    let alternatives = data_dir_names(game)
        .iter()
        .map(|n| regex::escape(n))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&format!(
        r"(?i)([A-Za-z]:[/\\][^\r\n\x22]*?(?:{alternatives}))"
    ))
    .expect("the data folder pattern is valid")
}

/// One pattern per game, compiled once.
static DATA_DIR_PATTERNS: LazyLock<Vec<(Game, Regex)>> = LazyLock::new(|| {
    Game::ALL
        .iter()
        .map(|&g| (g, data_dir_pattern(g)))
        .collect()
});

/// All existing `*_Data` directories mentioned in the game's logs (read-only), newest log
/// first, without duplicates. Several entries mean several installs (for example global + CN).
pub fn find_data_dirs(game: Game) -> Vec<PathBuf> {
    let Some((_, re)) = DATA_DIR_PATTERNS.iter().find(|(g, _)| *g == game) else {
        return vec![];
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    for log in log_candidates(game) {
        let Ok(bytes) = fsutil::read_shared_head(&log, LOG_HEAD_BYTES) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        if let Some(dir) = last_data_dir_in(&text, re) {
            if dir.is_dir() && !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    dirs
}

/// The newest install, if any.
pub fn find_data_dir(game: Game) -> Option<PathBuf> {
    find_data_dirs(game).into_iter().next()
}

/// Every discovered `*_Data` folder plus its parent (the install root that holds the game exe).
/// Used only to refuse exports into a game folder.
pub fn known_game_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for game in Game::ALL {
        for dir in find_data_dirs(game) {
            if let Some(parent) = dir.parent().filter(|p| p.file_name().is_some()) {
                roots.push(parent.to_path_buf());
            }
            roots.push(dir);
        }
    }
    roots
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_names_are_recognised_case_insensitively() {
        assert!(is_game_data_dir_name("GenshinImpact_Data"));
        assert!(is_game_data_dir_name("starrail_data"));
        assert!(!is_game_data_dir_name("my_data"));
    }

    #[test]
    fn picks_the_last_install_path_in_a_log() {
        let re = data_dir_pattern(Game::Hk4e);
        let log = "Loading C:/Old/Genshin Impact Game/GenshinImpact_Data/x\nSubsystems at D:/Games/Genshin Impact Game/GenshinImpact_Data/UnitySubsystems\n";
        assert_eq!(
            last_data_dir_in(log, &re).unwrap(),
            PathBuf::from("D:\\Games\\Genshin Impact Game\\GenshinImpact_Data")
        );
    }
}
