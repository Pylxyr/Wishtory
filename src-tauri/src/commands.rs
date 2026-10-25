//! Tauri commands. Game files are only touched (read-only) via discover.rs / authkey.rs.

use crate::{
    authkey,
    catalog::{BannerSet, BannerStore, BannerUpdate, Catalog, CatalogStore},
    db::Db,
    discover,
    error::AppError,
    fetch,
    logging::Level,
    models::{Account, Game, GameInstall, Pull},
    pity::{self, BannerPity},
    uigf::{self, ImportSummary},
};
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use tauri::{async_runtime::spawn_blocking, Emitter, Manager, State};

/// Import files above this size are refused (a full multi-game UIGF file is a few MB at most).
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
/// How many cached history links are tried before giving up (each failed try is one cheap request).
const MAX_LINK_TRIES: usize = 4;
static EXPORT_SEQ: AtomicU64 = AtomicU64::new(0);

fn join_err(e: tauri::Error) -> AppError {
    AppError::Io(e.to_string())
}

#[tauri::command]
pub async fn discover_games() -> Result<Vec<GameInstall>, AppError> {
    spawn_blocking(discover::discover_all)
        .await
        .map_err(join_err)
}

/// Returns the most likely history URL found in the game's web cache (read-only).
#[tauri::command]
pub async fn extract_authkey(game: Game) -> Result<String, AppError> {
    spawn_blocking(move || authkey::extract(game))
        .await
        .map_err(join_err)?
}

#[derive(Serialize)]
pub struct FetchSummary {
    game: String,
    uid: String,
    fetched: usize,
    added: usize,
    /// Present when the download stopped early; the saved part is still usable.
    warning: Option<String>,
}

/// Status lines for the refresh UI. Plain text only: never a link or authkey.
fn progress_to(app: &tauri::AppHandle) -> impl Fn(String) + '_ {
    move |msg| {
        let _ = app.emit("refresh-progress", msg);
    }
}

fn store(db: &Db, game: Game, f: fetch::Fetched) -> Result<FetchSummary, AppError> {
    let added = db.merge(game.as_str(), &f.uid, f.timezone, &f.pulls)?;
    wlog!(
        Level::Info,
        "{}: saved UID {} ({} fetched, {} new)",
        game.as_str(),
        f.uid,
        f.pulls.len(),
        added
    );
    Ok(FetchSummary {
        game: game.as_str().into(),
        uid: f.uid,
        fetched: f.pulls.len(),
        added,
        warning: f.incomplete,
    })
}

/// Fetch with a pasted history link.
#[tauri::command]
pub async fn fetch_history(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    game: Game,
    url: String,
) -> Result<FetchSummary, AppError> {
    let progress = progress_to(&app);
    progress("Checking your link…".into());
    wlog!(
        Level::Debug,
        "{}: fetching with a pasted link",
        game.as_str()
    );
    let f = fetch::fetch_all(game, &url, &progress)
        .await // network first; never hold the DB lock across await
        .inspect_err(|e| wlog!(Level::Warn, "{}: fetch failed: {e}", game.as_str()))?;
    progress(format!("Saving {} pulls to this PC…", f.pulls.len()));
    store(&db, game, f)
}

/// One-click flow: read the cache (read-only), then try the cached links newest first until one
/// is still valid. Handles a stale link sitting behind a fresh one in the cache file.
#[tauri::command]
pub async fn refresh_from_game(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    game: Game,
) -> Result<FetchSummary, AppError> {
    let progress = progress_to(&app);
    progress("Looking for your game's cache…".into());
    wlog!(
        Level::Debug,
        "{}: reading the game's cache (read-only)",
        game.as_str()
    );
    let urls = spawn_blocking(move || authkey::extract_all(game))
        .await
        .map_err(join_err)?
        .inspect_err(|e| wlog!(Level::Warn, "{}: cache read failed: {e}", game.as_str()))?;
    wlog!(
        Level::Debug,
        "{}: {} cached history link(s) found (trying up to {MAX_LINK_TRIES})",
        game.as_str(),
        urls.len()
    );
    let tries = urls.len().min(MAX_LINK_TRIES);
    progress(format!(
        "Found {tries} saved link{}. Checking the latest…",
        if tries == 1 { "" } else { "s" }
    ));
    let mut last = AppError::HistoryPageNotOpened;
    for (i, url) in urls.into_iter().take(MAX_LINK_TRIES).enumerate() {
        if i > 0 {
            progress(format!(
                "That link expired. Trying the next one ({} of {tries})…",
                i + 1
            ));
        }
        match fetch::fetch_all(game, &url, &progress).await {
            Ok(f) => {
                wlog!(Level::Debug, "{}: link {} accepted", game.as_str(), i + 1);
                progress(format!("Saving {} pulls to this PC…", f.pulls.len()));
                return store(&db, game, f);
            }
            Err(AppError::AuthExpired) => {
                wlog!(
                    Level::Debug,
                    "{}: link {} has expired",
                    game.as_str(),
                    i + 1
                );
                last = AppError::AuthExpired;
            }
            Err(e) => {
                wlog!(Level::Warn, "{}: fetch failed: {e}", game.as_str());
                return Err(e);
            }
        }
    }
    wlog!(
        Level::Warn,
        "{}: no usable history link ({last})",
        game.as_str()
    );
    Err(last)
}

#[tauri::command]
pub async fn list_accounts(db: State<'_, Db>) -> Result<Vec<Account>, AppError> {
    db.list_accounts()
}

#[tauri::command]
pub async fn delete_account(db: State<'_, Db>, game: Game, uid: String) -> Result<(), AppError> {
    db.delete_account(game.as_str(), &uid).inspect(|_| {
        wlog!(
            Level::Info,
            "{}: removed UID {uid} from Wishtory",
            game.as_str()
        )
    })
}

#[tauri::command]
pub async fn clear_all_history(db: State<'_, Db>) -> Result<usize, AppError> {
    db.clear_all_history()
        .inspect(|n| wlog!(Level::Info, "cleared all saved history ({n} pulls)"))
}

#[tauri::command]
pub async fn get_catalog(catalog: State<'_, CatalogStore>) -> Result<Catalog, String> {
    Ok(catalog.current())
}

#[tauri::command]
pub async fn refresh_catalog(catalog: State<'_, CatalogStore>) -> Result<String, String> {
    let result = catalog.refresh().await;
    match &result {
        Ok(status) => wlog!(Level::Debug, "catalog refresh: {status}"),
        Err(e) => wlog!(Level::Warn, "catalog refresh failed: {e}"),
    }
    result
}

#[tauri::command]
pub async fn get_banners(banners: State<'_, BannerStore>) -> Result<BannerSet, String> {
    Ok(banners.current())
}

/// The last installed banner update, so the app can show what changed after a restart.
#[tauri::command]
pub async fn get_banner_update(
    banners: State<'_, BannerStore>,
) -> Result<Option<BannerUpdate>, String> {
    Ok(banners.last_update())
}

/// `force` skips the once-a-day gate (the "Update banners" button).
#[tauri::command]
pub async fn refresh_banners(
    banners: State<'_, BannerStore>,
    force: bool,
) -> Result<BannerUpdate, String> {
    let result = banners.refresh(force).await;
    match &result {
        Ok(update) => wlog!(Level::Debug, "banner refresh: {}", update.status),
        Err(e) => wlog!(Level::Warn, "banner refresh failed: {e}"),
    }
    result
}

/// Newest first, optionally filtered by banner key ("character", "weapon", ...).
#[tauri::command]
pub async fn get_history(
    db: State<'_, Db>,
    game: Game,
    uid: String,
    banner: Option<String>,
) -> Result<Vec<Pull>, AppError> {
    let mut pulls = db.pulls_for(game.as_str(), &uid)?;
    if let Some(b) = banner.as_deref().filter(|b| !b.is_empty()) {
        pulls.retain(|p| pity::banner_key(game, &p.gacha_type) == Some(b));
    }
    pulls.reverse();
    Ok(pulls)
}

#[tauri::command]
pub async fn get_pity(
    db: State<'_, Db>,
    game: Game,
    uid: String,
) -> Result<Vec<BannerPity>, AppError> {
    Ok(pity::compute(game, &db.pulls_for(game.as_str(), &uid)?))
}

/// Decodes a user-selected file. Windows tools (PowerShell `Out-File`) often write UTF-16.
fn decode_text(bytes: &[u8]) -> Result<String, AppError> {
    let utf16 = |b: &[u8], le: bool| -> String {
        let units: Vec<u16> = b
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| {
                if le {
                    u16::from_le_bytes(*c)
                } else {
                    u16::from_be_bytes(*c)
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Ok(utf16(&bytes[2..], true));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Ok(utf16(&bytes[2..], false));
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| AppError::Format("the file is not UTF-8 or UTF-16 text".into()))
}

/// Reads a user-selected UIGF / SRGF file.
#[tauri::command]
pub async fn import_file(
    app: tauri::AppHandle,
    catalog: State<'_, CatalogStore>,
    path: String,
) -> Result<ImportSummary, AppError> {
    let catalog = catalog.current();
    // File read, JSON parse and database merge can take a while: keep them off the async runtime.
    spawn_blocking(move || -> Result<ImportSummary, AppError> {
        let size = std::fs::metadata(&path)?.len();
        wlog!(Level::Debug, "import: reading a {size}-byte file");
        if size > MAX_IMPORT_BYTES {
            return Err(AppError::Format(
                "the file is larger than 64 MB; that cannot be a wish history".into(),
            ));
        }
        let text = decode_text(&std::fs::read(&path)?)?;
        let db = app.state::<Db>();
        uigf::import_with_catalog(&db, &text, &catalog)
            .inspect(|r| {
                wlog!(
                    Level::Info,
                    "import ({}): {} new, {} skipped, {} account(s)",
                    r.format,
                    r.added,
                    r.skipped,
                    r.accounts
                )
            })
            .inspect_err(|e| wlog!(Level::Warn, "import failed: {e}"))
    })
    .await
    .map_err(join_err)?
}

#[tauri::command]
pub async fn export_file(
    db: State<'_, Db>,
    catalog: State<'_, CatalogStore>,
    path: String,
    format: String,
    game: Option<Game>,
    uid: Option<String>,
) -> Result<(), AppError> {
    let roots: Vec<String> = spawn_blocking(|| {
        discover::known_game_roots()
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    })
    .await
    .map_err(join_err)?;
    wlog!(
        Level::Debug,
        "export: format {format}, {}",
        if uid.is_some() {
            "selected account"
        } else {
            "all accounts"
        }
    );
    ensure_not_game_dir(&path, &roots)
        .inspect_err(|e| wlog!(Level::Warn, "export refused: {e}"))?;
    let json = uigf::export_with_catalog(&db, &format, game, uid.as_deref(), &catalog.current())
        .inspect_err(|e| wlog!(Level::Warn, "export failed: {e}"))?;
    write_export(Path::new(&path), json.as_bytes())
        .inspect(|_| wlog!(Level::Info, "export written ({} bytes)", json.len()))
        .inspect_err(|e| wlog!(Level::Error, "export could not be saved: {e}"))
}

fn write_export(target: &Path, contents: &[u8]) -> Result<(), AppError> {
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .ok_or_else(|| AppError::Io("Please choose a file name for the export.".into()))?;
    let mut temp = None;
    for _ in 0..16 {
        let n = EXPORT_SEQ.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{}.wishtory-{}-{n}.tmp",
            name.to_string_lossy(),
            std::process::id()
        ));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                let result = file.write_all(contents).and_then(|_| file.sync_all());
                if let Err(e) = result {
                    let _ = std::fs::remove_file(&candidate);
                    return Err(e.into());
                }
                temp = Some(candidate);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    let temp =
        temp.ok_or_else(|| AppError::Io("Could not create a temporary export file.".into()))?;
    let result = replace_file(&temp, target);
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), AppError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // Same-volume rename atomically replaces an existing Windows export.
    let ok = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(source: &Path, target: &Path) -> Result<(), AppError> {
    std::fs::rename(source, target).map_err(Into::into)
}

fn norm(p: &str) -> String {
    p.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

fn resolved_path(path: &Path) -> PathBuf {
    let lexical = || {
        let mut out = PathBuf::new();
        for part in path.components() {
            match part {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    out.pop();
                }
                _ => out.push(part.as_os_str()),
            }
        }
        out
    };
    // Save dialogs normally return an existing parent. Canonicalizing it also resolves
    // junctions/symlinks before comparing it with discovered game roots.
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => parent
            .canonicalize()
            .map(|p| p.join(name))
            .unwrap_or_else(|_| lexical()),
        _ => lexical(),
    }
}

/// Extra guard: never write anything inside a game folder, even at the user's request.
/// Blocks (a) any path with a component named like a game data folder and (b) anything inside a
/// discovered install folder. Works on plain strings so it behaves the same everywhere.
fn ensure_not_game_dir(path: &str, game_roots: &[String]) -> Result<(), AppError> {
    let target_path = resolved_path(Path::new(path));
    let target = norm(&target_path.to_string_lossy());
    let by_name = target_path.components().any(|c| {
        matches!(c, std::path::Component::Normal(name) if discover::is_game_data_dir_name(&name.to_string_lossy()))
    });
    let by_root = game_roots
        .iter()
        .map(|r| norm(&resolved_path(Path::new(r)).to_string_lossy()))
        .any(|r| target == r || target.starts_with(&format!("{r}\\")));
    if by_name || by_root {
        return Err(AppError::Io(
            "Please choose a location outside the game's folders.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_folders_are_refused_but_lookalikes_are_not() {
        let roots = vec!["D:\\Games\\Genshin Impact Game".to_string()];
        assert!(ensure_not_game_dir("D:\\Games\\Genshin Impact Game\\out.json", &roots).is_err());
        assert!(ensure_not_game_dir("d:/games/genshin impact game/sub/out.json", &roots).is_err());
        assert!(ensure_not_game_dir("E:\\x\\GenshinImpact_Data\\out.json", &[]).is_err());
        assert!(
            ensure_not_game_dir("C:\\Users\\me\\Documents\\wish_data\\out.json", &roots).is_ok()
        );
        assert!(ensure_not_game_dir("D:\\Games\\Genshin Impact Game 2\\out.json", &roots).is_ok());
        assert!(ensure_not_game_dir(
            "D:\\Games\\Other\\..\\Genshin Impact Game\\out.json",
            &roots
        )
        .is_err());
    }

    #[test]
    fn decodes_utf8_with_bom_and_utf16() {
        assert_eq!(decode_text(b"{\"a\":1}").unwrap(), "{\"a\":1}");
        let mut le = vec![0xFF, 0xFE];
        le.extend("{}".encode_utf16().flat_map(|u| u.to_le_bytes()));
        assert_eq!(decode_text(&le).unwrap(), "{}");
        assert!(decode_text(&[0x80, 0x81]).is_err());
    }

    #[test]
    fn export_replaces_an_existing_file_without_leaving_temp_files() {
        let dir = std::env::temp_dir().join(format!("wishtory-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("existing.json");
        std::fs::write(&target, b"old content").unwrap();
        write_export(&target, b"new content").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"new content");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
