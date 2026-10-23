//! Tauri commands. Game files are only touched (read-only) via discover.rs / authkey.rs.

use crate::{
    authkey, db::Db, discover, error::AppError, fetch,
    models::{Account, Game, GameInstall, Pull},
    pity::{self, BannerPity},
    uigf::{self, ImportSummary},
};
use serde::Serialize;
use std::path::Path;
use tauri::{async_runtime::spawn_blocking, State};

/// Import files above this size are refused (a full multi-game UIGF file is a few MB at most).
const MAX_IMPORT_BYTES: u64 = 256 * 1024 * 1024;
/// How many cached history links are tried before giving up (each failed try is one cheap request).
const MAX_LINK_TRIES: usize = 4;

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
        let units: Vec<u16> = b.chunks_exact(2)
            .map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) })
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
pub async fn import_file(db: State<'_, Db>, path: String) -> Result<ImportSummary, AppError> {
    let size = std::fs::metadata(&path)?.len();
    if size > MAX_IMPORT_BYTES {
        return Err(AppError::Format("the file is larger than 256 MB; that cannot be a wish history".into()));
    }
    let text = decode_text(&std::fs::read(&path)?)?;
    uigf::import(&db, &text)
}

#[tauri::command]
pub async fn export_file(db: State<'_, Db>, path: String, format: String, game: Option<Game>, uid: Option<String>) -> Result<(), AppError> {
    let roots: Vec<String> = spawn_blocking(|| discover::known_game_roots().into_iter().map(|p| p.to_string_lossy().into_owned()).collect())
        .await
        .map_err(join_err)?;
    ensure_not_game_dir(&path, &roots)?;
    let json = uigf::export(&db, &format, game, uid.as_deref())?;
    // Write next to the target and rename, so a failed write never leaves a truncated export
    // over an existing file.
    let target = Path::new(&path);
    let tmp = target.with_extension("json.wishtory-tmp");
    std::fs::write(&tmp, json)?;
    if let Err(e) = std::fs::rename(&tmp, target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

fn norm(p: &str) -> String {
    p.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Extra guard: never write anything inside a game folder, even at the user's request.
/// Blocks (a) any path with a component named like a game data folder and (b) anything inside a
/// discovered install folder. Works on plain strings so it behaves the same everywhere.
fn ensure_not_game_dir(path: &str, game_roots: &[String]) -> Result<(), AppError> {
    let by_name = path.split(['/', '\\']).any(discover::is_game_data_dir_name);
    let target = norm(path);
    let by_root = game_roots.iter().map(|r| norm(r)).any(|r| target == r || target.starts_with(&format!("{r}\\")));
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
    }

    #[test]
    fn decodes_utf8_with_bom_and_utf16() {
        assert_eq!(decode_text(b"{\"a\":1}").unwrap(), "{\"a\":1}");
        let mut le = vec![0xFF, 0xFE];
        le.extend("{}".encode_utf16().flat_map(|u| u.to_le_bytes()));
        assert_eq!(decode_text(&le).unwrap(), "{}");
        assert!(decode_text(&[0x80, 0x81]).is_err());
    }
}
