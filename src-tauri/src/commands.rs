//! Tauri commands. Game files are only touched (read-only) via discover.rs / authkey.rs.

use crate::{
    authkey,
    catalog::{BannerEntry, BannerSet, BannerStore, BannerUpdate, Catalog, CatalogStore},
    db::Db,
    discover,
    error::AppError,
    fetch,
    logging::Level,
    models::{Account, Game, GameInstall, Pull},
    pity::{self, BannerPity},
    rules::RulesStore,
    uigf::{self, ImportSummary},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tauri::{async_runtime::spawn_blocking, Emitter, State};

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

/// Records which banner each saved pull of an account was made on. Only pulls without a stored
/// banner are resolved, so it is cheap to call after every sync, import and banner update.
fn assign_banners(db: &Db, banners: &BannerSet, game: Game, uid: &str) -> Result<(), AppError> {
    let missing: HashSet<String> = db
        .pulls_without_banner(game.as_str(), uid)?
        .into_iter()
        .map(|p| p.id)
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    // Resolved against the whole history, so a gacha_id seen on one placed pull places the rest.
    let pulls = db.pulls_for(game.as_str(), uid)?;
    let placed = pity::resolve_banners(&banners.banners, game, &pulls);
    let rows: Vec<(String, String)> = pulls
        .iter()
        .zip(placed)
        .filter(|(p, _)| missing.contains(&p.id))
        .filter_map(|(p, b)| b.map(|b| (p.id.clone(), b.id.clone())))
        .collect();
    if rows.is_empty() {
        return Ok(());
    }
    db.set_pull_banners(game.as_str(), uid, &rows)
}

fn assign_all_banners(db: &Db, banners: &BannerSet) -> Result<(), AppError> {
    for account in db.list_accounts()? {
        if let Some(game) = Game::ALL
            .iter()
            .copied()
            .find(|g| g.as_str() == account.game)
        {
            assign_banners(db, banners, game, &account.uid)?;
        }
    }
    Ok(())
}

async fn store(
    db: &Db,
    banners: Arc<BannerSet>,
    game: Game,
    f: fetch::Fetched,
) -> Result<FetchSummary, AppError> {
    db.run(move |db| {
        let added = db.merge(game.as_str(), &f.uid, f.timezone, &f.pulls)?;
        if let Err(e) = assign_banners(db, &banners, game, &f.uid) {
            wlog!(
                Level::Warn,
                "{}: banner assignment failed: {e}",
                game.as_str()
            );
        }
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
    })
    .await
}

/// Ids already stored for the account behind a history link, so a fetch can stop at the first
/// page it already has. `full` disables that and downloads everything the server still keeps.
/// (One small id query; it runs while the first page is being processed.)
fn known_ids(db: &Db, game: Game, full: bool) -> impl Fn(&str) -> HashSet<String> + '_ {
    move |uid| {
        if full {
            HashSet::new()
        } else {
            db.complete_pull_ids_for(game.as_str(), uid)
                .unwrap_or_default()
        }
    }
}

/// Fetch with a pasted history link.
#[tauri::command]
pub async fn fetch_history(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    banners: State<'_, BannerStore>,
    game: Game,
    url: String,
    full: Option<bool>,
) -> Result<FetchSummary, AppError> {
    let progress = progress_to(&app);
    progress("Checking your link…".into());
    wlog!(
        Level::Debug,
        "{}: fetching with a pasted link",
        game.as_str()
    );
    let known = known_ids(&db, game, full.unwrap_or(false));
    let f = fetch::fetch_all(game, &url, &progress, &known)
        .await // network first; never hold the DB lock across await
        .inspect_err(|e| wlog!(Level::Warn, "{}: fetch failed: {e}", game.as_str()))?;
    progress(format!("Saving {} pulls to this PC…", f.pulls.len()));
    store(&db, banners.current(), game, f).await
}

/// One-click flow: read the cache (read-only), then try the cached links newest first until one
/// is still valid. Handles a stale link sitting behind a fresh one in the cache file.
#[tauri::command]
pub async fn refresh_from_game(
    app: tauri::AppHandle,
    db: State<'_, Db>,
    banners: State<'_, BannerStore>,
    game: Game,
    full: Option<bool>,
) -> Result<FetchSummary, AppError> {
    let progress = progress_to(&app);
    let known = known_ids(&db, game, full.unwrap_or(false));
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
        match fetch::fetch_all(game, &url, &progress, &known).await {
            Ok(f) => {
                wlog!(Level::Debug, "{}: link {} accepted", game.as_str(), i + 1);
                progress(format!("Saving {} pulls to this PC…", f.pulls.len()));
                return store(&db, banners.current(), game, f).await;
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
    db.run(|db| db.list_accounts()).await
}

#[tauri::command]
pub async fn delete_account(db: State<'_, Db>, game: Game, uid: String) -> Result<(), AppError> {
    db.run(move |db| {
        db.delete_account(game.as_str(), &uid).inspect(|_| {
            wlog!(
                Level::Info,
                "{}: removed UID {uid} from Wishtory",
                game.as_str()
            )
        })
    })
    .await
}

#[tauri::command]
pub async fn clear_all_history(db: State<'_, Db>) -> Result<usize, AppError> {
    db.run(|db| {
        db.clear_all_history()
            .inspect(|n| wlog!(Level::Info, "cleared all saved history ({n} pulls)"))
    })
    .await
}

#[tauri::command]
pub async fn get_catalog(catalog: State<'_, CatalogStore>) -> Result<Arc<Catalog>, AppError> {
    Ok(catalog.current())
}

/// Updates the signed game catalog and the signed pity rules (two independent feeds). The status
/// is "updated" when either changed; a failure is only reported when nothing could be updated.
#[tauri::command]
pub async fn refresh_catalog(
    catalog: State<'_, CatalogStore>,
    rules: State<'_, RulesStore>,
    force: bool,
) -> Result<String, AppError> {
    let outcomes = [
        ("catalog", catalog.refresh(force).await.map(|r| r.status())),
        ("rules", rules.refresh(force).await.map(|r| r.status())),
    ];
    let mut status = "up-to-date";
    let mut failure = None;
    for (name, outcome) in outcomes {
        match outcome {
            Ok(s) => {
                wlog!(Level::Debug, "{name} refresh: {s}");
                if s == "updated" || (s == "already-checking" && status != "updated") {
                    status = s;
                }
            }
            Err(e) => {
                wlog!(Level::Warn, "{name} refresh failed: {e}");
                failure.get_or_insert(e);
            }
        }
    }
    match failure {
        Some(e) if status != "updated" => Err(AppError::Message(e)),
        _ => Ok(status.into()),
    }
}

#[tauri::command]
pub async fn get_banners(banners: State<'_, BannerStore>) -> Result<Arc<BannerSet>, AppError> {
    Ok(banners.current())
}

/// The last installed banner update, so the app can show what changed after a restart.
#[tauri::command]
pub async fn get_banner_update(
    banners: State<'_, BannerStore>,
) -> Result<Option<BannerUpdate>, AppError> {
    Ok(banners.last_update())
}

/// `force` skips the once-a-day gate (the "Update banners" button).
#[tauri::command]
pub async fn refresh_banners(
    db: State<'_, Db>,
    banners: State<'_, BannerStore>,
    force: bool,
) -> Result<BannerUpdate, AppError> {
    let update = banners
        .refresh(force)
        .await
        .inspect_err(|e| wlog!(Level::Warn, "banner refresh failed: {e}"))
        .map_err(AppError::Message)?;
    wlog!(Level::Debug, "banner refresh: {}", update.status);
    // A newer list can move windows, so every saved pull is matched against it again.
    if update.status == "updated" {
        let list = banners.current();
        db.run(move |db| {
            db.clear_pull_banners()?;
            assign_all_banners(db, &list)
        })
        .await?;
    }
    Ok(update)
}

/// Static, per-game UI naming that the backend already owns (so the frontend keeps no copy).
#[derive(Serialize)]
pub struct GameMeta {
    game: &'static str,
    /// History-page banner filter: `(key, label)` in display order.
    banners: Vec<BannerOption>,
}

#[derive(Serialize)]
pub struct BannerOption {
    key: &'static str,
    label: &'static str,
}

#[tauri::command]
pub fn get_game_meta() -> Vec<GameMeta> {
    Game::ALL
        .iter()
        .map(|&game| GameMeta {
            game: game.as_str(),
            banners: pity::history_filters(game)
                .iter()
                .map(|&(key, label)| BannerOption { key, label })
                .collect(),
        })
        .collect()
}

#[derive(Serialize)]
pub struct HistoryPull {
    #[serde(flatten)]
    pull: Pull,
    /// The banner event the pull was made on, e.g. "Ballad in Goblets · Venti"; `None` when the
    /// banner list cannot place it.
    banner_name: Option<String>,
    /// Pulls since the last 5★ (5★ rows) or last 4★/5★ (4★ rows) when this pull was made.
    pity: Option<u32>,
    /// 3 / 4 / 5 for every game (Zenless reports B / A / S as 2 / 3 / 4 in `rank_type`).
    rank: u8,
    /// Short pool name for the table, e.g. "Character" or "W-Engine".
    pool: String,
}

/// "Event name · Featured A / Featured B", with item names looked up in the catalog.
fn banner_label(entry: &BannerEntry, names: &HashMap<(&str, &str), &str>) -> String {
    let featured: Vec<&str> = entry
        .featured_5
        .iter()
        .map(|id| {
            names
                .get(&(entry.game.as_str(), id.as_str()))
                .copied()
                .unwrap_or(id.as_str())
        })
        .collect();
    format!("{} · {}", entry.name, featured.join(" / "))
}

/// Newest first, optionally filtered by banner key ("character", "weapon", ...).
#[tauri::command]
pub async fn get_history(
    db: State<'_, Db>,
    catalog: State<'_, CatalogStore>,
    banners: State<'_, BannerStore>,
    game: Game,
    uid: String,
    banner: Option<String>,
) -> Result<Vec<HistoryPull>, AppError> {
    let (catalog, banner_list) = (catalog.current(), banners.current());
    db.run(move |db| history(db, &catalog, &banner_list, game, &uid, banner.as_deref()))
        .await
}

fn history(
    db: &Db,
    catalog: &Catalog,
    banner_list: &BannerSet,
    game: Game,
    uid: &str,
    banner: Option<&str>,
) -> Result<Vec<HistoryPull>, AppError> {
    if let Err(e) = assign_banners(db, banner_list, game, uid) {
        wlog!(
            Level::Warn,
            "{}: banner assignment failed: {e}",
            game.as_str()
        );
    }
    let assigned = db.pull_banners_for(game.as_str(), uid)?;
    let names: HashMap<(&str, &str), &str> = catalog
        .items
        .iter()
        .map(|i| ((i.game.as_str(), i.id.as_str()), i.name.as_str()))
        .collect();
    let entries: HashMap<&str, &BannerEntry> = banner_list
        .banners
        .iter()
        .map(|b| (b.id.as_str(), b))
        .collect();
    let pulls = db.pulls_for(game.as_str(), uid)?;
    // Pity runs over the whole pool history, so it is counted before any banner filter.
    let pities = pity::pull_pities(game, &pulls);
    let mut rows: Vec<(Pull, Option<u32>)> = pulls.into_iter().zip(pities).collect();
    if let Some(b) = banner.filter(|b| !b.is_empty()) {
        rows.retain(|(p, _)| pity::banner_key(game, &p.gacha_type) == Some(b));
    }
    rows.reverse();
    Ok(rows
        .into_iter()
        .map(|(pull, pity)| {
            let banner_name = assigned
                .get(&pull.id)
                .and_then(|id| entries.get(id.as_str()))
                .map(|entry| banner_label(entry, &names));
            HistoryPull {
                rank: pity::display_rank(game, pull.rank_type),
                pool: pity::pool_label(game, &pull.gacha_type)
                    .map_or_else(|| pull.gacha_type.clone(), str::to_string),
                pull,
                banner_name,
                pity,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn get_pity(
    db: State<'_, Db>,
    banners: State<'_, BannerStore>,
    rules: State<'_, RulesStore>,
    game: Game,
    uid: String,
) -> Result<Vec<BannerPity>, AppError> {
    let (banners, rules) = (banners.current(), rules.current());
    db.run(move |db| {
        let pulls = db.pulls_for(game.as_str(), &uid)?;
        Ok(pity::compute_with_banners(
            game,
            &pulls,
            &banners.banners,
            &rules,
        ))
    })
    .await
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

/// Reads a user-selected import file, never more than `max` bytes. The limit is applied to the
/// bytes actually read, so a file that grows after the size check, or a device or pipe that
/// reports no size, cannot exhaust memory.
fn read_import_file(path: &str, max: u64) -> Result<Vec<u8>, AppError> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    wlog!(Level::Debug, "import: reading a {size}-byte file");
    let too_big =
        || AppError::Format("the file is larger than 64 MB; that cannot be a wish history".into());
    if size > max {
        return Err(too_big());
    }
    let mut bytes = Vec::new();
    file.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(too_big());
    }
    Ok(bytes)
}

/// Reads a user-selected UIGF / SRGF file.
#[tauri::command]
pub async fn import_file(
    db: State<'_, Db>,
    catalog: State<'_, CatalogStore>,
    banners: State<'_, BannerStore>,
    path: String,
) -> Result<ImportSummary, AppError> {
    let (catalog, banners) = (catalog.current(), banners.current());
    // File read, JSON parse and database merge can take a while: keep them off the async runtime.
    db.run(move |db| {
        let text = decode_text(&read_import_file(&path, MAX_IMPORT_BYTES)?)?;
        let summary = uigf::import_with_catalog(db, &text, &catalog)
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
            .inspect_err(|e| wlog!(Level::Warn, "import failed: {e}"))?;
        if let Err(e) = assign_all_banners(db, &banners) {
            wlog!(Level::Warn, "banner assignment failed: {e}");
        }
        Ok(summary)
    })
    .await
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
    let catalog = catalog.current();
    // Discovery, the database read, serialization and the fsync'd write are all blocking.
    db.run(move |db| {
        let roots: Vec<String> = discover::known_game_roots()
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
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
        let json = uigf::export_with_catalog(db, &format, game, uid.as_deref(), &catalog)
            .inspect_err(|e| wlog!(Level::Warn, "export failed: {e}"))?;
        write_export(Path::new(&path), json.as_bytes())
            .inspect(|_| wlog!(Level::Info, "export written ({} bytes)", json.len()))
            .inspect_err(|e| wlog!(Level::Error, "export could not be saved: {e}"))
    })
    .await
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

/// Backslash-separated, lower-case and with `.` / `..` resolved, so the comparison behaves the same
/// on every platform (the paths come from a Windows app and from log files).
fn norm(p: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in p.split(['/', '\\']) {
        match part {
            "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("\\").trim_end_matches('\\').to_lowercase()
}

/// Resolves junctions and symlinks of the (existing) parent folder before comparing.
fn resolved_path(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => parent
            .canonicalize()
            .map(|p| p.join(name))
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

/// Extra guard: never write anything inside a game folder, even at the user's request.
/// Blocks (a) any path with a component named like a game data folder and (b) anything inside a
/// discovered install folder. Works on plain strings so it behaves the same everywhere.
fn ensure_not_game_dir(path: &str, game_roots: &[String]) -> Result<(), AppError> {
    let target = norm(&resolved_path(Path::new(path)).to_string_lossy());
    let by_name = target.split('\\').any(discover::is_game_data_dir_name);
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
    fn import_reads_are_bounded_by_the_limit() {
        let path =
            std::env::temp_dir().join(format!("wishtory-import-{}.json", std::process::id()));
        std::fs::write(&path, b"0123456789").unwrap();
        let path_str = path.to_string_lossy();
        assert_eq!(read_import_file(&path_str, 10).unwrap(), b"0123456789");
        assert!(read_import_file(&path_str, 9).is_err());
        assert!(read_import_file("/definitely/not/here.json", 10).is_err());
        std::fs::remove_file(path).unwrap();
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
