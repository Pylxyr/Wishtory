//! Automatic game discovery.
//! SAFETY: READ-ONLY. We only read the log files the game itself writes under
//! %USERPROFILE%\AppData\LocalLow (on Linux: the same folder inside each Wine or Proton prefix)
//! and inspect directory metadata. Nothing is written.

use crate::{
    fsutil,
    models::{DetectedPrefix, Game, GameInstall},
};
use regex::Regex;
use std::sync::LazyLock;
use std::{fs, path::PathBuf, time::SystemTime};

/// Where a game writes its logs (`…/AppData/LocalLow`), and how to turn the Windows paths written
/// inside those logs into real folders. Windows has one; Linux has one per Wine user per prefix.
struct Source {
    locallow: PathBuf,
    #[cfg(unix)]
    prefix: crate::wine::Prefix,
}

impl Source {
    /// A Windows path from a log as the real folder on this machine.
    fn resolve(&self, windows_path: &str) -> Option<PathBuf> {
        #[cfg(unix)]
        {
            self.prefix.resolve(windows_path)
        }
        #[cfg(not(unix))]
        {
            Some(PathBuf::from(windows_path))
        }
    }
}

#[cfg(windows)]
fn sources() -> Vec<Source> {
    std::env::var_os("USERPROFILE")
        .map(|h| Source {
            locallow: PathBuf::from(h).join("AppData").join("LocalLow"),
        })
        .into_iter()
        .collect()
}

#[cfg(unix)]
fn sources() -> Vec<Source> {
    sources_in(&crate::wine::all_prefixes(
        &crate::settings::game_locations(),
    ))
}

#[cfg(unix)]
fn sources_in(prefixes: &[crate::wine::Prefix]) -> Vec<Source> {
    prefixes
        .iter()
        .flat_map(|prefix| {
            prefix
                .locallow_dirs()
                .into_iter()
                .map(move |locallow| Source {
                    locallow,
                    prefix: prefix.clone(),
                })
        })
        .collect()
}

#[cfg(not(any(windows, unix)))]
fn sources() -> Vec<Source> {
    vec![]
}

fn modified(p: &PathBuf) -> SystemTime {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Existing log files for a game with the source each came from, newest first (global + CN folder
/// names).
fn log_candidates(sources: &[Source], game: Game) -> Vec<(PathBuf, usize)> {
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
    let mut found: Vec<(PathBuf, usize)> = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        for dir in dirs {
            for file in files {
                let path = source.locallow.join(dir).join(file);
                if path.is_file() {
                    found.push((path, index));
                }
            }
        }
    }
    found.sort_by_key(|(path, _)| std::cmp::Reverse(modified(path)));
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

/// Last install path mentioned in a Unity log text, if any, as the Windows path the game wrote.
fn last_data_dir_in(text: &str, re: &Regex) -> Option<String> {
    re.captures_iter(text)
        .last()
        .map(|cap| cap[1].replace('/', "\\"))
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
fn data_dirs_in(sources: &[Source], game: Game) -> Vec<PathBuf> {
    let Some((_, re)) = DATA_DIR_PATTERNS.iter().find(|(g, _)| *g == game) else {
        return vec![];
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    for (log, index) in log_candidates(sources, game) {
        let Ok(bytes) = fsutil::read_shared_head(&log, LOG_HEAD_BYTES) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let dir = last_data_dir_in(&text, re)
            .and_then(|written| sources[index].resolve(&written))
            .filter(|dir| dir.is_dir());
        if let Some(dir) = dir {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    dirs
}

pub fn find_data_dirs(game: Game) -> Vec<PathBuf> {
    data_dirs_in(&sources(), game)
}

/// Every discovered `*_Data` folder plus its parent (the install root that holds the game exe).
/// Used only to refuse exports into a game folder.
pub fn known_game_roots() -> Vec<PathBuf> {
    let sources = sources();
    let mut roots = Vec::new();
    for game in Game::ALL {
        for dir in data_dirs_in(&sources, game) {
            if let Some(parent) = dir.parent().filter(|p| p.file_name().is_some()) {
                roots.push(parent.to_path_buf());
            }
            roots.push(dir);
        }
    }
    roots
}

pub fn discover_all() -> Vec<GameInstall> {
    let sources = sources();
    Game::ALL
        .iter()
        .map(|&game| {
            let logs = log_candidates(&sources, game);
            GameInstall {
                game,
                found: !logs.is_empty(),
                log_path: logs.first().map(|(p, _)| p.to_string_lossy().into_owned()),
                data_dir: data_dirs_in(&sources, game)
                    .into_iter()
                    .next()
                    .map(|p| p.to_string_lossy().into_owned()),
            }
        })
        .collect()
}

/// Wine and Proton prefixes found on this machine (Linux), and which games have a log in each.
/// Shown in Settings so a missing game can be tracked down; always empty elsewhere.
#[cfg(unix)]
pub fn detect_prefixes() -> Vec<DetectedPrefix> {
    crate::wine::all_prefixes(&crate::settings::game_locations())
        .into_iter()
        .take(50)
        .map(|prefix| {
            let sources = sources_in(std::slice::from_ref(&prefix));
            DetectedPrefix {
                path: prefix.root.to_string_lossy().into_owned(),
                games: Game::ALL
                    .into_iter()
                    .filter(|&game| !log_candidates(&sources, game).is_empty())
                    .collect(),
            }
        })
        .collect()
}

#[cfg(not(unix))]
pub fn detect_prefixes() -> Vec<DetectedPrefix> {
    vec![]
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
            "D:\\Games\\Genshin Impact Game\\GenshinImpact_Data"
        );
    }

    /// The Linux path end to end: a prefix with a game log, found through the Wine path mapping.
    #[cfg(unix)]
    mod linux {
        use super::*;
        use crate::wine::Prefix;
        use std::fs;
        use std::sync::atomic::{AtomicU64, Ordering};

        static SEQ: AtomicU64 = AtomicU64::new(0);

        struct Scratch(PathBuf);
        impl Scratch {
            fn new() -> Self {
                let n = SEQ.fetch_add(1, Ordering::Relaxed);
                let dir = std::env::temp_dir()
                    .join(format!("wishtory-discover-test-{}-{n}", std::process::id()));
                fs::create_dir_all(&dir).unwrap();
                Scratch(dir)
            }
            fn dir(&self, rel: &str) -> PathBuf {
                let p = self.0.join(rel);
                fs::create_dir_all(&p).unwrap();
                p
            }
        }
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        fn prefix(s: &Scratch, rel: &str) -> Prefix {
            let root = s.dir(rel);
            fs::create_dir_all(root.join("drive_c").join("users")).unwrap();
            fs::create_dir_all(root.join("dosdevices")).unwrap();
            std::os::unix::fs::symlink("../drive_c", root.join("dosdevices").join("c:")).unwrap();
            std::os::unix::fs::symlink("/", root.join("dosdevices").join("z:")).unwrap();
            Prefix { root }
        }

        fn write_log(prefix: &Prefix, user: &str, folder: &str, file: &str, text: &str) {
            let dir = prefix
                .root
                .join("drive_c/users")
                .join(user)
                .join("AppData/LocalLow")
                .join(folder);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(file), text).unwrap();
        }

        #[test]
        fn finds_a_game_installed_on_c_inside_a_prefix() {
            let s = Scratch::new();
            let p = prefix(&s, "wine/genshin");
            let data = s.dir("wine/genshin/drive_c/Program Files/Genshin Impact/Genshin Impact game/GenshinImpact_Data");
            fs::create_dir_all(data.join("webCaches")).unwrap();
            write_log(
                &p,
                "steamuser",
                "miHoYo/Genshin Impact",
                "Player.log",
                "Unity\nSubsystems at C:/Program Files/Genshin Impact/Genshin Impact game/GenshinImpact_Data/UnitySubsystems\n",
            );
            let sources = sources_in(&[p]);
            assert_eq!(sources.len(), 1);
            assert_eq!(data_dirs_in(&sources, Game::Hk4e), vec![data]);
            assert!(data_dirs_in(&sources, Game::Hkrpg).is_empty());
            assert_eq!(log_candidates(&sources, Game::Hk4e).len(), 1);
        }

        #[test]
        fn finds_a_game_on_another_disk_through_z_and_forgives_case() {
            let s = Scratch::new();
            let p = prefix(&s, "wine/hsr");
            let data = s.dir("games/Star Rail/Games/StarRail_Data");
            // The log spells every part of the path in lower case, and the data folder in upper case.
            let written = format!("Z:{}", data.display())
                .to_lowercase()
                .replace("starrail_data", "STARRAIL_DATA");
            write_log(
                &p,
                "me",
                "Cognosphere/Star Rail",
                "Player.log",
                &format!("Loaded {written}/Data\n"),
            );
            let sources = sources_in(&[p]);
            assert_eq!(data_dirs_in(&sources, Game::Hkrpg), vec![data]);
        }

        #[test]
        fn a_log_that_names_a_missing_folder_gives_nothing() {
            let s = Scratch::new();
            let p = prefix(&s, "wine/zzz");
            write_log(
                &p,
                "me",
                "miHoYo/ZenlessZoneZero",
                "Player.log",
                "C:/Games/ZenlessZoneZero_Data/x\n",
            );
            assert!(data_dirs_in(&sources_in(&[p]), Game::Nap).is_empty());
        }

        #[test]
        fn the_newest_log_across_prefixes_comes_first() {
            let s = Scratch::new();
            let old = prefix(&s, "wine/old");
            let new = prefix(&s, "wine/new");
            let old_data = s.dir("wine/old/drive_c/Games/GenshinImpact_Data");
            let new_data = s.dir("wine/new/drive_c/Games/GenshinImpact_Data");
            write_log(
                &old,
                "u",
                "miHoYo/Genshin Impact",
                "Player.log",
                "C:/Games/GenshinImpact_Data/x\n",
            );
            std::thread::sleep(std::time::Duration::from_millis(1100));
            write_log(
                &new,
                "u",
                "miHoYo/Genshin Impact",
                "Player.log",
                "C:/Games/GenshinImpact_Data/x\n",
            );
            let sources = sources_in(&[old, new]);
            assert_eq!(data_dirs_in(&sources, Game::Hk4e), vec![new_data, old_data]);
        }

        #[test]
        fn prefixes_without_a_log_have_no_sources_and_no_games() {
            let s = Scratch::new();
            let p = prefix(&s, "wine/empty");
            assert!(sources_in(&[p]).is_empty());
        }
    }
}
