//! Tauri commands. Game files are only touched (read-only) via discover.rs / authkey.rs.

use crate::{
    authkey, catalog::{Catalog, CatalogStore}, db::Db, discover, error::AppError, fetch,
    models::{Account, Game, GameInstall, Pull},
    pity::{self, BannerPity},
    uigf::{self, ImportSummary},
};
use serde::Serialize;
use std::{io::Write, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};
use tauri::{async_runtime::spawn_blocking, State};

/// Import files above this size are refused (a full multi-game UIGF file is a few MB at most).
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
/// How many cached history links are tried before giving up (each failed try is one cheap request).
const MAX_LINK_TRIES: usize = 4;
static EXPORT_SEQ: AtomicU64 = AtomicU64::new(0);

fn join_err(e: tauri::Error) -> AppError { AppError::Io(e.to_string()) }

#[tauri::command]
pub async fn discover_games() -> Result<Vec<GameInstall>, AppError> {
    spawn_blocking(discover::discover_all).await.map_err(join_err)
}

/// Returns the most likely history URL found in the game's web cache (read-only).
#[tauri::command]
pub async fn extract_authkey(game: Game) -> Result<String, AppError> {
    spawn_blocking(move || authkey::extract(game)).await.map_err(join_err)?
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

fn store(db: &Db, game: Game, f: fetch::Fetched) -> Result<FetchSummary, AppError> {
    let added = db.merge(game.as_str(), &f.uid, f.timezone, &f.pulls)?;
    Ok(FetchSummary { game: game.as_str().into(), uid: f.uid, fetched: f.pulls.len(), added, warning: f.incomplete })
}

/// Fetch with a pasted history link.
#[tauri::command]
pub async fn fetch_history(db: State<'_, Db>, game: Game, url: String) -> Result<FetchSummary, AppError> {
    let f = fetch::fetch_all(game, &url).await?; // network first; never hold the DB lock across await
    store(&db, game, f)
}

/// One-click flow: read the cache (read-only), then try the cached links newest first until one
/// is still valid. Handles a stale link sitting behind a fresh one in the cache file.
#[tauri::command]
pub async fn refresh_from_game(db: State<'_, Db>, game: Game) -> Result<FetchSummary, AppError> {
    let urls = spawn_blocking(move || authkey::extract_all(game)).await.map_err(join_err)??;
    let mut last = AppError::HistoryPageNotOpened;
    for url in urls.into_iter().take(MAX_LINK_TRIES) {
        match fetch::fetch_all(game, &url).await {
            Ok(f) => return store(&db, game, f),
            Err(AppError::AuthExpired) => last = AppError::AuthExpired,
            Err(e) => return Err(e),
        }
    }
    Err(last)
}

#[tauri::command]
pub async fn list_accounts(db: State<'_, Db>) -> Result<Vec<Account>, AppError> {
    db.list_accounts()
}

#[tauri::command]
pub async fn delete_account(db: State<'_, Db>, game: Game, uid: String) -> Result<(), AppError> {
    db.delete_account(game.as_str(), &uid)
}

#[tauri::command]
pub async fn clear_all_history(db: State<'_, Db>) -> Result<usize, AppError> {
    db.clear_all_history()
}

#[tauri::command]
pub async fn get_catalog(catalog: State<'_, CatalogStore>) -> Result<Catalog, String> { Ok(catalog.current()) }

#[tauri::command]
pub async fn refresh_catalog(catalog: State<'_, CatalogStore>) -> Result<String, String> {
    catalog.refresh().await
}

/// Newest first, optionally filtered by banner key ("character", "weapon", ...).
#[tauri::command]
pub async fn get_history(db: State<'_, Db>, game: Game, uid: String, banner: Option<String>) -> Result<Vec<Pull>, AppError> {
    let mut pulls = db.pulls_for(game.as_str(), &uid)?;
    if let Some(b) = banner.as_deref().filter(|b| !b.is_empty()) {
        pulls.retain(|p| pity::banner_key(game, &p.gacha_type) == Some(b));
    }
    pulls.reverse();
    Ok(pulls)
}

#[tauri::command]
pub async fn get_pity(db: State<'_, Db>, game: Game, uid: String) -> Result<Vec<BannerPity>, AppError> {
    Ok(pity::compute(game, &db.pulls_for(game.as_str(), &uid)?))
}

/// Decodes a user-selected file. Windows tools (PowerShell `Out-File`) often write UTF-16.
fn decode_text(bytes: &[u8]) -> Result<String, AppError> {
    let utf16 = |b: &[u8], le: bool| -> String {
        let units: Vec<u16> = b.as_chunks::<2>().0.iter()
            .map(|c| if le { u16::from_le_bytes(*c) } else { u16::from_be_bytes(*c) })
            .collect();
        String::from_utf16_lossy(&units)
    };
    if bytes.starts_with(&[0xFF, 0xFE]) { return Ok(utf16(&bytes[2..], true)); }
    if bytes.starts_with(&[0xFE, 0xFF]) { return Ok(utf16(&bytes[2..], false)); }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| AppError::Format("the file is not UTF-8 or UTF-16 text".into()))
}

/// Reads a user-selected UIGF / SRGF file.
#[tauri::command]
pub async fn import_file(db: State<'_, Db>, catalog: State<'_, CatalogStore>, path: String) -> Result<ImportSummary, AppError> {
    let size = std::fs::metadata(&path)?.len();
    if size > MAX_IMPORT_BYTES {
        return Err(AppError::Format("the file is larger than 64 MB; that cannot be a wish history".into()));
    }
    let text = decode_text(&std::fs::read(&path)?)?;
    uigf::import_with_catalog(&db, &text, &catalog.current())
}

#[tauri::command]
pub async fn export_file(db: State<'_, Db>, catalog: State<'_, CatalogStore>, path: String, format: String, game: Option<Game>, uid: Option<String>) -> Result<(), AppError> {
    let roots: Vec<String> = spawn_blocking(|| discover::known_game_roots().into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
        .await
        .map_err(join_err)?;
    ensure_not_game_dir(&path, &roots)?;
    let json = uigf::export_with_catalog(&db, &format, game, uid.as_deref(), &catalog.current())?;
    write_export(Path::new(&path), json.as_bytes())
}

fn write_export(target: &Path, contents: &[u8]) -> Result<(), AppError> {
    let parent = target.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let name = target.file_name().ok_or_else(|| AppError::Io("Please choose a file name for the export.".into()))?;
    let mut temp = None;
    for _ in 0..16 {
        let n = EXPORT_SEQ.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".{}.wishtory-{}-{n}.tmp", name.to_string_lossy(), std::process::id()));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&candidate) {
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
    let temp = temp.ok_or_else(|| AppError::Io("Could not create a temporary export file.".into()))?;
    let result = replace_file(&temp, target);
    if result.is_err() { let _ = std::fs::remove_file(&temp); }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, target: &Path) -> Result<(), AppError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // Same-volume rename atomically replaces an existing Windows export.
    let ok = unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) };
    if ok == 0 { Err(std::io::Error::last_os_error().into()) } else { Ok(()) }
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
                std::path::Component::CurDir => {},
                std::path::Component::ParentDir => { out.pop(); },
                _ => out.push(part.as_os_str()),
            }
        }
        out
    };
    // Save dialogs normally return an existing parent. Canonicalizing it also resolves
    // junctions/symlinks before comparing it with discovered game roots.
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => parent.canonicalize().map(|p| p.join(name)).unwrap_or_else(|_| lexical()),
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
    let by_root = game_roots.iter().map(|r| norm(&resolved_path(Path::new(r)).to_string_lossy()))
        .any(|r| target == r || target.starts_with(&format!("{r}\\")));
    if by_name || by_root {
        return Err(AppError::Io("Please choose a location outside the game's folders.".into()));
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
        assert!(ensure_not_game_dir("C:\\Users\\me\\Documents\\wish_data\\out.json", &roots).is_ok());
        assert!(ensure_not_game_dir("D:\\Games\\Genshin Impact Game 2\\out.json", &roots).is_ok());
        assert!(ensure_not_game_dir("D:\\Games\\Other\\..\\Genshin Impact Game\\out.json", &roots).is_err());
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
