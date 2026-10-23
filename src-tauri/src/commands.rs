//! Tauri commands. Game files are only touched (read-only) via discover.rs / authkey.rs.

use crate::{
    authkey, db::Db, discover, error::AppError, fetch,
    models::{Account, Game, GameInstall, Pull},
    pity::{self, BannerPity},
    uigf::{self, ImportSummary},
};
use serde::Serialize;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn discover_games() -> Result<Vec<GameInstall>, AppError> {
    Ok(discover::discover_all())
}

/// Returns the newest history URL found in the game's web cache (read-only).
#[tauri::command]
pub async fn extract_authkey(game: Game) -> Result<String, AppError> {
    authkey::extract(game)
}

#[derive(Serialize)]
pub struct FetchSummary { game: String, uid: String, fetched: usize, added: usize }

#[tauri::command]
pub async fn fetch_history(db: State<'_, Db>, game: Game, url: String) -> Result<FetchSummary, AppError> {
    let f = fetch::fetch_all(game, &url).await?; // network first; never hold the DB lock across await
    let added = db.merge(game.as_str(), &f.uid, f.timezone, &f.pulls)?;
    Ok(FetchSummary { game: game.as_str().into(), uid: f.uid, fetched: f.pulls.len(), added })
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

/// Reads a user-selected UIGF / SRGF file.
#[tauri::command]
pub async fn import_file(db: State<'_, Db>, path: String) -> Result<ImportSummary, AppError> {
    uigf::import(&db, &std::fs::read_to_string(path)?)
}

#[tauri::command]
pub async fn export_file(db: State<'_, Db>, path: String, format: String, game: Option<Game>, uid: Option<String>) -> Result<(), AppError> {
    ensure_not_game_dir(Path::new(&path))?;
    let json = uigf::export(&db, &format, game, uid.as_deref())?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Extra guard: never write anything inside a game folder, even at the user's request.
fn ensure_not_game_dir(path: &Path) -> Result<(), AppError> {
    let inside = path.ancestors().filter_map(|p| p.file_name()).any(|n| n.to_string_lossy().to_lowercase().ends_with("_data"));
    if inside {
        return Err(AppError::Io("Please choose a location outside the game's folders.".into()));
    }
    Ok(())
}
