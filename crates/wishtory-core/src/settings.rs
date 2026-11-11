//! Small user settings kept next to the database (`settings.json` in the app data folder):
//! extra folders to search for game installs (which Linux needs because games run inside Wine
//! prefixes that can live anywhere), and the cloud backup switches.
//! SAFETY: the folders are only ever read (see wine.rs and discover.rs). Nothing secret is kept
//! here: the Google sign-in and the backup passphrase live in the operating system's credential store.

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
};

/// More than this is almost certainly a mistake, and every folder is searched on each refresh.
const MAX_LOCATIONS: usize = 16;
/// A settings file larger than this is not ours; it is ignored instead of read.
const MAX_FILE_BYTES: u64 = 64 * 1024;

/// What discovery reads. Set when the app starts and whenever the person changes the list.
static LOCATIONS: RwLock<Vec<PathBuf>> = RwLock::new(Vec::new());

/// The extra folders to search for game installs.
#[cfg_attr(not(unix), allow(dead_code))] // only Linux discovery reads them
pub fn game_locations() -> Vec<PathBuf> {
    LOCATIONS.read().map(|l| l.clone()).unwrap_or_default()
}

fn publish(locations: &[PathBuf]) {
    if let Ok(mut current) = LOCATIONS.write() {
        *current = locations.to_vec();
    }
}

/// The non-secret cloud backup state.
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudSettings {
    /// Back up by itself, at most once a day, while the app is open.
    #[serde(default)]
    pub auto_backup: bool,
    /// When the last backup finished, RFC 3339.
    #[serde(default)]
    pub last_backup: Option<String>,
    /// The Google account that is connected, for display.
    #[serde(default)]
    pub account: Option<String>,
}

/// How game data updates are applied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogSettings {
    /// Apply a release by itself when it would change no saved pull. A release that would
    /// change any pull always waits for the person, whatever this says.
    #[serde(default = "yes")]
    pub auto_apply: bool,
}

fn yes() -> bool {
    true
}

impl Default for CatalogSettings {
    fn default() -> Self {
        Self { auto_apply: true }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct SettingsFile {
    #[serde(default)]
    game_locations: Vec<String>,
    #[serde(default)]
    cloud: CloudSettings,
    #[serde(default)]
    catalog: CatalogSettings,
}

pub struct SettingsStore {
    path: PathBuf,
    /// Saving is rare, but two saves must not interleave their temporary files.
    saving: Mutex<()>,
}

impl SettingsStore {
    /// Loads the settings, tolerating a missing, unreadable or damaged file (defaults are used).
    pub fn open(dir: &Path) -> Self {
        let store = Self {
            path: dir.join("settings.json"),
            saving: Mutex::new(()),
        };
        publish(&store.load_locations());
        store
    }

    fn load_file(&self) -> SettingsFile {
        let usable = fs::metadata(&self.path)
            .map(|m| m.is_file() && m.len() <= MAX_FILE_BYTES)
            .unwrap_or(false);
        if !usable {
            return SettingsFile::default();
        }
        fs::read(&self.path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn load_locations(&self) -> Vec<PathBuf> {
        clean(self.load_file().game_locations)
    }

    /// The cloud backup state (defaults when nothing was saved).
    pub fn cloud(&self) -> CloudSettings {
        self.load_file().cloud
    }

    /// Changes the cloud backup state and saves it; returns the new state.
    pub fn update_cloud(
        &self,
        change: impl FnOnce(&mut CloudSettings),
    ) -> Result<CloudSettings, AppError> {
        let _guard = self.saving.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = self.load_file();
        change(&mut file.cloud);
        self.write(&file)?;
        Ok(file.cloud)
    }

    /// How game data updates are applied (defaults when nothing was saved).
    pub fn catalog(&self) -> CatalogSettings {
        self.load_file().catalog
    }

    pub fn set_catalog_auto_apply(&self, auto_apply: bool) -> Result<CatalogSettings, AppError> {
        let _guard = self.saving.lock().unwrap_or_else(|e| e.into_inner());
        let mut file = self.load_file();
        file.catalog.auto_apply = auto_apply;
        self.write(&file)?;
        Ok(file.catalog)
    }

    /// The saved extra folders, as text for the interface.
    pub fn game_locations(&self) -> Vec<String> {
        self.load_locations()
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }

    /// Replaces the extra folders. A folder that is new must be an existing absolute folder;
    /// ones already saved stay even if their disk is not mounted right now.
    pub fn set_game_locations(&self, requested: Vec<String>) -> Result<Vec<String>, AppError> {
        let _guard = self.saving.lock().unwrap_or_else(|e| e.into_inner());
        let existing = self.load_locations();
        let wanted = clean(requested);
        for path in &wanted {
            if !existing.contains(path) && !path.is_dir() {
                return Err(AppError::Message(format!(
                    "{} is not an existing folder.",
                    path.display()
                )));
            }
        }
        let mut file = self.load_file();
        file.game_locations = wanted
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        self.write(&file)?;
        publish(&wanted);
        Ok(file.game_locations)
    }

    /// Writes through a private temporary file and a rename, so a crash never leaves half a file.
    fn write(&self, file: &SettingsFile) -> Result<(), AppError> {
        let bytes = serde_json::to_vec_pretty(file)?;
        let tmp = self
            .path
            .with_extension(format!("json.tmp-{}", std::process::id()));
        let _ = fs::remove_file(&tmp);
        let result = (|| -> std::io::Result<()> {
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)?;
            out.write_all(&bytes)?;
            out.sync_all()?;
            drop(out);
            fs::rename(&tmp, &self.path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result.map_err(Into::into)
    }
}

/// Keeps absolute, non-empty paths once each (in order), at most `MAX_LOCATIONS`.
fn clean(raw: Vec<String>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for text in raw {
        let text = text.trim();
        if text.is_empty() || text.contains('\0') {
            continue;
        }
        let path = PathBuf::from(text);
        if path.is_absolute() && !out.contains(&path) && out.len() < MAX_LOCATIONS {
            out.push(path);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn scratch() -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("wishtory-settings-test-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn clean_keeps_absolute_paths_once_and_caps_the_list() {
        let many: Vec<String> = (0..40).map(|i| format!("/games/{i}")).collect();
        assert_eq!(clean(many).len(), MAX_LOCATIONS);
        let cleaned = clean(vec![
            " /a ".into(),
            "/a".into(),
            "relative/b".into(),
            "".into(),
            "/c\0d".into(),
            "/e".into(),
        ]);
        assert_eq!(cleaned, vec![PathBuf::from("/a"), PathBuf::from("/e")]);
    }

    #[cfg(unix)]
    #[test]
    fn saves_reloads_and_rejects_folders_that_do_not_exist() {
        let dir = scratch();
        let folder = dir.join("Games");
        fs::create_dir_all(&folder).unwrap();
        let store = SettingsStore::open(&dir);
        assert!(store.game_locations().is_empty());
        let saved = store
            .set_game_locations(vec![folder.to_string_lossy().into_owned()])
            .unwrap();
        assert_eq!(saved, vec![folder.to_string_lossy().into_owned()]);
        assert_eq!(game_locations(), vec![folder.clone()]);
        // A new store reads the same file back, as after a restart.
        let again = SettingsStore::open(&dir);
        assert_eq!(again.game_locations(), saved);
        // A missing folder is refused, and the list stays as it was.
        let err = again
            .set_game_locations(vec!["/definitely/not/here".into()])
            .unwrap_err();
        assert!(err.to_string().contains("not an existing folder"), "{err}");
        assert_eq!(again.game_locations(), saved);
        // A folder that was saved earlier stays even if it vanished (an unmounted disk).
        fs::remove_dir_all(&folder).unwrap();
        assert!(again
            .set_game_locations(vec![folder.to_string_lossy().into_owned()])
            .is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn cloud_state_and_game_folders_do_not_overwrite_each_other() {
        let dir = scratch();
        let store = SettingsStore::open(&dir);
        assert_eq!(store.cloud(), CloudSettings::default());
        store
            .update_cloud(|c| {
                c.auto_backup = true;
                c.account = Some("me@example.com".into());
            })
            .unwrap();
        store.set_game_locations(vec![]).unwrap();
        let again = SettingsStore::open(&dir);
        assert!(again.cloud().auto_backup);
        assert_eq!(again.cloud().account.as_deref(), Some("me@example.com"));
        // An older settings file without a `cloud` section still loads.
        fs::write(dir.join("settings.json"), br#"{"game_locations":[]}"#).unwrap();
        assert_eq!(SettingsStore::open(&dir).cloud(), CloudSettings::default());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_or_oversized_file_means_defaults() {
        let dir = scratch();
        fs::write(dir.join("settings.json"), b"{ not json").unwrap();
        assert!(SettingsStore::open(&dir).game_locations().is_empty());
        fs::write(
            dir.join("settings.json"),
            vec![b' '; (MAX_FILE_BYTES + 1) as usize],
        )
        .unwrap();
        assert!(SettingsStore::open(&dir).game_locations().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_temporary_file_is_left_behind() {
        let dir = scratch();
        let store = SettingsStore::open(&dir);
        store.set_game_locations(vec![]).unwrap();
        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["settings.json".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }
}
