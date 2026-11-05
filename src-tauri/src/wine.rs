//! Wine and Proton support (Linux): finding the prefixes the games run in, and turning the Windows
//! paths a game writes into its log back into real files.
//! SAFETY: READ-ONLY, like the rest of discovery. This module lists folders, reads symbolic links
//! and looks at metadata. It never creates, changes or deletes anything and never starts Wine.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

/// How many folder entries one search may look at, so a huge or looping home folder cannot stall
/// discovery.
const MAX_ENTRIES_PER_SEARCH: usize = 5_000;

/// Folders that are never worth descending into when looking for prefixes.
const SKIP: &[&str] = &[
    "drive_c",
    "dosdevices",
    "steamapps",
    "node_modules",
    "shadercache",
    "flatpak",
    "icons",
    "applications",
    "fonts",
    "themes",
    "trash",
    ".git",
];

/// A Wine prefix: the folder that holds `drive_c`, `dosdevices` and the registry files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefix {
    pub root: PathBuf,
}

impl Prefix {
    /// `…/drive_c/users/<name>/AppData/LocalLow` for every Wine user that has one. The `Public`
    /// profile is skipped; games write their logs into the profile of the user that runs them.
    pub fn locallow_dirs(&self) -> Vec<PathBuf> {
        let Ok(users) = fs::read_dir(self.root.join("drive_c").join("users")) else {
            return vec![];
        };
        let mut found: Vec<PathBuf> = users
            .flatten()
            .filter(|u| {
                !u.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("public")
            })
            .filter_map(|u| {
                let app_data = ci_join(&u.path(), "AppData")?;
                ci_join(&app_data, "LocalLow")
            })
            .filter(|p| p.is_dir())
            .collect();
        found.sort();
        found
    }

    /// A Windows path as written in a game log (`C:/Program Files/…` or `Z:\home\…`) as the real
    /// file or folder on this machine. `None` when it does not exist here. Case differences are
    /// forgiven, because Windows paths are case-insensitive and Linux ones are not.
    pub fn resolve(&self, windows_path: &str) -> Option<PathBuf> {
        let (drive, rest) = split_drive(windows_path)?;
        let mut current = self.drive_root(drive)?;
        for part in rest
            .split(['/', '\\'])
            .filter(|p| !p.is_empty() && *p != ".")
        {
            if part == ".." {
                return None; // a log never needs it, and refusing keeps the lookup inside the drive
            }
            current = ci_join(&current, part)?;
        }
        Some(current)
    }

    /// The real folder a drive letter points to, through the prefix's `dosdevices` links. Links are
    /// usually relative (`c: -> ../drive_c`), so the folder is returned in its canonical form.
    fn drive_root(&self, letter: char) -> Option<PathBuf> {
        let letter = letter.to_ascii_lowercase();
        let devices = self.root.join("dosdevices");
        // `join` with an absolute link target replaces `devices`, which is what is wanted.
        let linked = fs::read_link(devices.join(format!("{letter}:")))
            .ok()
            .map(|target| devices.join(target));
        // Wine always maps C: to drive_c and Z: to the Unix root, even when the link is missing.
        let fallback = match letter {
            'c' => Some(self.root.join("drive_c")),
            'z' => Some(PathBuf::from("/")),
            _ => None,
        };
        [linked, fallback]
            .into_iter()
            .flatten()
            .filter_map(|base| base.canonicalize().ok())
            .find(|base| base.is_dir())
    }
}

/// `C:/x/y` → (`C`, `/x/y`).
fn split_drive(path: &str) -> Option<(char, &str)> {
    let mut chars = path.chars();
    let letter = chars.next().filter(char::is_ascii_alphabetic)?;
    (chars.next()? == ':').then(|| (letter, &path[2..]))
}

/// `dir/name`, matching an existing entry whose name differs only in case when there is no exact one.
fn ci_join(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    let wanted = name.to_lowercase();
    let mut matches: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().to_lowercase() == wanted)
        .map(|e| e.path())
        .collect();
    matches.sort();
    matches.into_iter().next()
}

/// The prefix that `dir` is, or that sits in it: Proton keeps its prefix in a `pfx` folder.
fn prefix_at(dir: &Path) -> Option<Prefix> {
    [dir.to_path_buf(), dir.join("pfx")]
        .into_iter()
        .find(|candidate| candidate.join("drive_c").is_dir())
        .map(|root| Prefix { root })
}

/// Prefixes at `root` or at most `depth` folders below it.
pub fn find_prefixes(root: &Path, depth: usize) -> Vec<Prefix> {
    let mut found = Vec::new();
    let mut budget = MAX_ENTRIES_PER_SEARCH;
    let mut seen = HashSet::new();
    walk(root, depth, &mut budget, &mut seen, &mut found);
    found
}

fn walk(
    dir: &Path,
    depth: usize,
    budget: &mut usize,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Prefix>,
) {
    if !seen.insert(dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf())) {
        return; // reached again through a link
    }
    if let Some(prefix) = prefix_at(dir) {
        out.push(prefix);
        return; // everything below a prefix is Windows files, not more prefixes
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut children = Vec::new();
    for entry in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if SKIP.contains(&name.as_str()) || !entry.path().is_dir() {
            continue;
        }
        children.push(entry.path());
    }
    children.sort();
    for child in children {
        walk(&child, depth - 1, budget, seen, out);
    }
}

/// Where prefixes usually live on a Linux desktop, and how many folders deep to look under each:
/// Wine's default, Lutris, the anime-game and honkers launchers, Bottles, Steam (Proton) and
/// Flatpak versions of those.
pub fn default_roots_for(
    home: Option<&Path>,
    xdg_data: Option<&Path>,
    wineprefix: Option<&Path>,
) -> Vec<(PathBuf, usize)> {
    let mut roots = Vec::new();
    if let Some(prefix) = wineprefix {
        roots.push((prefix.to_path_buf(), 0));
    }
    let Some(home) = home else { return roots };
    let data = xdg_data
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".local").join("share"));
    roots.push((home.join(".wine"), 0));
    roots.push((home.join("Games"), 3));
    roots.push((data.clone(), 3));
    for steam in [
        home.join(".steam").join("steam"),
        data.join("Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
    ] {
        roots.push((steam.join("steamapps").join("compatdata"), 1));
    }
    if let Ok(apps) = fs::read_dir(home.join(".var").join("app")) {
        let mut apps: Vec<PathBuf> = apps.flatten().map(|a| a.path().join("data")).collect();
        apps.sort();
        roots.extend(apps.into_iter().map(|p| (p, 3)));
    }
    roots
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// Every prefix found in the usual places, `$WINEPREFIX`, and the folders the user added (those
/// first), without duplicates.
pub fn all_prefixes(extra: &[PathBuf]) -> Vec<Prefix> {
    let mut roots: Vec<(PathBuf, usize)> = extra.iter().map(|p| (p.clone(), 3)).collect();
    roots.extend(default_roots_for(
        env_path("HOME").as_deref(),
        env_path("XDG_DATA_HOME").as_deref(),
        env_path("WINEPREFIX").as_deref(),
    ));
    let mut seen = HashSet::new();
    let mut found = Vec::new();
    for (root, depth) in roots {
        for prefix in find_prefixes(&root, depth) {
            let id = prefix
                .root
                .canonicalize()
                .unwrap_or_else(|_| prefix.root.clone());
            if seen.insert(id) {
                found.push(prefix);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    /// A scratch folder that is removed again, so tests never leave files behind.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let n = SEQ.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("wishtory-wine-test-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
        fn path(&self) -> &Path {
            &self.0
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

    fn make_prefix(root: &Path) -> Prefix {
        fs::create_dir_all(root.join("drive_c").join("users")).unwrap();
        fs::create_dir_all(root.join("dosdevices")).unwrap();
        std::os::unix::fs::symlink("../drive_c", root.join("dosdevices").join("c:")).unwrap();
        Prefix {
            root: root.to_path_buf(),
        }
    }

    #[test]
    fn splits_a_drive_letter() {
        assert_eq!(split_drive("C:/a/b"), Some(('C', "/a/b")));
        assert_eq!(split_drive("z:\\home"), Some(('z', "\\home")));
        assert_eq!(split_drive("/home/a"), None);
        assert_eq!(split_drive("1:/a"), None);
        assert_eq!(split_drive("C"), None);
        assert_eq!(split_drive(""), None);
    }

    #[test]
    fn finds_prefixes_and_proton_pfx_folders() {
        let s = Scratch::new();
        make_prefix(&s.dir("Games/genshin"));
        make_prefix(&s.dir("compat/1234/pfx"));
        s.dir("Games/not-a-prefix/sub");
        let games = find_prefixes(&s.path().join("Games"), 3);
        assert_eq!(games.len(), 1);
        assert!(games[0].root.ends_with("Games/genshin"));
        let proton = find_prefixes(&s.path().join("compat"), 1);
        assert_eq!(proton.len(), 1);
        assert!(proton[0].root.ends_with("compat/1234/pfx"));
    }

    #[test]
    fn a_root_that_is_a_prefix_is_found_at_depth_zero() {
        let s = Scratch::new();
        let root = s.dir("wine");
        make_prefix(&root);
        assert_eq!(find_prefixes(&root, 0).len(), 1);
    }

    #[test]
    fn search_depth_is_limited_and_prefixes_are_not_searched_for_prefixes() {
        let s = Scratch::new();
        make_prefix(&s.dir("a/b/c/d/deep"));
        assert!(find_prefixes(s.path(), 3).is_empty());
        assert_eq!(find_prefixes(s.path(), 5).len(), 1);
        // Folders inside a prefix are never searched, even when they look like one.
        let outer = s.dir("outer");
        make_prefix(&outer);
        make_prefix(&outer.join("drive_c").join("inner"));
        assert_eq!(find_prefixes(&outer, 5).len(), 1);
    }

    #[test]
    fn skipped_folders_are_not_searched() {
        let s = Scratch::new();
        make_prefix(&s.dir("share/Steam/steamapps/common/Proton/pfx"));
        make_prefix(&s.dir("share/flatpak/x"));
        make_prefix(&s.dir("share/real"));
        let found = find_prefixes(&s.path().join("share"), 5);
        assert_eq!(found.len(), 1);
        assert!(found[0].root.ends_with("share/real"));
    }

    #[test]
    fn symlink_loops_do_not_hang_the_search() {
        let s = Scratch::new();
        let base = s.dir("loop");
        std::os::unix::fs::symlink(&base, base.join("again")).unwrap();
        make_prefix(&s.dir("loop/p"));
        assert_eq!(find_prefixes(&base, 6).len(), 1);
    }

    #[test]
    fn local_low_is_found_for_every_user_except_public() {
        let s = Scratch::new();
        let prefix = make_prefix(&s.dir("p"));
        s.dir("p/drive_c/users/steamuser/AppData/LocalLow");
        s.dir("p/drive_c/users/Public/AppData/LocalLow");
        s.dir("p/drive_c/users/other/appdata/locallow"); // lower-case spelling
        s.dir("p/drive_c/users/nolow/AppData/Local");
        let dirs = prefix.locallow_dirs();
        assert_eq!(dirs.len(), 2);
        assert!(dirs
            .iter()
            .any(|d| d.to_string_lossy().contains("steamuser")));
        assert!(dirs.iter().any(|d| d.to_string_lossy().contains("other")));
    }

    #[test]
    fn resolves_c_drive_paths_ignoring_case() {
        let s = Scratch::new();
        let prefix = make_prefix(&s.dir("p"));
        let real =
            s.dir("p/drive_c/Program Files/Genshin Impact/Genshin Impact game/GenshinImpact_Data");
        assert_eq!(
            prefix
                .resolve("C:/Program Files/Genshin Impact/Genshin Impact game/GenshinImpact_Data"),
            Some(real.clone())
        );
        assert_eq!(
            prefix.resolve(
                "c:\\program files\\GENSHIN IMPACT\\genshin impact game\\genshinimpact_data"
            ),
            Some(real)
        );
        assert_eq!(
            prefix.resolve("C:/Program Files/Nope/GenshinImpact_Data"),
            None
        );
    }

    #[test]
    fn c_resolves_even_without_a_dosdevices_link() {
        let s = Scratch::new();
        let root = s.dir("p");
        fs::create_dir_all(root.join("drive_c").join("Games")).unwrap();
        let prefix = Prefix { root: root.clone() };
        assert_eq!(
            prefix.resolve("C:/Games"),
            Some(root.join("drive_c").join("Games"))
        );
    }

    #[test]
    fn other_drives_follow_the_dosdevices_links() {
        let s = Scratch::new();
        let prefix = make_prefix(&s.dir("p"));
        let disk = s.dir("mnt/games");
        let game = s.dir("mnt/games/HoYo/Star Rail Game/StarRail_Data");
        std::os::unix::fs::symlink(&disk, prefix.root.join("dosdevices").join("d:")).unwrap();
        assert_eq!(
            prefix.resolve("D:/HoYo/Star Rail Game/StarRail_Data"),
            Some(game)
        );
        assert_eq!(
            prefix.resolve("E:/HoYo"),
            None,
            "an unmapped drive has no folder"
        );
    }

    #[test]
    fn z_maps_to_the_unix_root_and_real_paths_resolve_through_it() {
        let s = Scratch::new();
        let prefix = make_prefix(&s.dir("p"));
        std::os::unix::fs::symlink("/", prefix.root.join("dosdevices").join("z:")).unwrap();
        let game = s.dir("home/me/Games/ZenlessZoneZero Game/ZenlessZoneZero_Data");
        let as_log = format!("Z:{}", game.display()); // Z:/tmp/…/ZenlessZoneZero_Data
        assert_eq!(prefix.resolve(&as_log), Some(game));
    }

    #[test]
    fn parent_components_are_refused() {
        let s = Scratch::new();
        let prefix = make_prefix(&s.dir("p"));
        s.dir("p/drive_c/a");
        assert_eq!(prefix.resolve("C:/a/../a"), None);
        assert_eq!(prefix.resolve("C:/../../etc"), None);
        assert!(prefix.resolve("C:/a/./").is_some());
    }

    #[test]
    fn default_roots_cover_the_common_launchers() {
        let s = Scratch::new();
        let home = s.dir("home");
        s.dir("home/.var/app/moe.launcher.an-anime-game-launcher/data");
        let roots = default_roots_for(Some(&home), None, Some(Path::new("/opt/wine-x")));
        let paths: Vec<String> = roots
            .iter()
            .map(|(p, _)| p.to_string_lossy().into_owned())
            .collect();
        assert!(paths[0].ends_with("/opt/wine-x"), "WINEPREFIX first");
        for needle in [
            "/.wine",
            "/Games",
            "/.local/share",
            "/.local/share/Steam/steamapps/compatdata",
            "/.steam/steam/steamapps/compatdata",
            "com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata",
            "/.var/app/moe.launcher.an-anime-game-launcher/data",
        ] {
            assert!(
                paths.iter().any(|p| p.ends_with(needle)),
                "missing {needle}: {paths:?}"
            );
        }
        let custom = default_roots_for(Some(&home), Some(Path::new("/data/xdg")), None);
        assert!(custom
            .iter()
            .any(|(p, d)| p == Path::new("/data/xdg") && *d == 3));
        assert!(default_roots_for(None, None, None).is_empty());
    }
}
