//! Tauri commands: each one forwards to `wishtory-api` and nothing more. Arguments arrive from
//! JavaScript in camelCase (Tauri maps them to these `snake_case` names).

use std::sync::Arc;
use tauri::{State, ipc::Channel};
use wishtory_api::api::{
    core_api::{self, Emit, RefreshEvent, WishtoryCore},
    error::ApiError,
    types::{
        AboutInfo, Account, AffectedPull, AppUpdate, ApplyResult, BannerList, BannerPity,
        CatalogCheck, CatalogHistoryEntry, CatalogReview, CatalogStatus, CloudBackup, CloudStatus,
        GameCatalog, GameId, GameInstall, GameLocations, GameMeta, HistoryPull, ImportSummary,
        LogEntry,
    },
};

/// The opened core, or the reason it could not open.
pub struct Shell(Result<WishtoryCore, ApiError>);

impl Shell {
    pub fn new(opened: Result<WishtoryCore, ApiError>) -> Self {
        Self(opened)
    }

    fn core(&self) -> Result<&WishtoryCore, ApiError> {
        self.0.as_ref().map_err(|e| ApiError {
            code: e.code,
            message: e.message.clone(),
        })
    }
}

/// Why Wishtory could not open its data, if it could not. The window shows it instead of a blank page.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its State by value; it cannot be a reference"
)]
pub fn startup_error(shell: State<'_, Shell>) -> Option<ApiError> {
    shell.0.as_ref().err().map(|e| ApiError {
        code: e.code,
        message: e.message.clone(),
    })
}

/// The event for a refresh that could not even start.
fn failed(e: ApiError) -> RefreshEvent {
    core_api::failed_event(e)
}

#[tauri::command]
pub async fn discover_games(shell: State<'_, Shell>) -> Result<Vec<GameInstall>, ApiError> {
    shell.core()?.discover_games().await
}

#[tauri::command]
pub async fn get_game_locations(shell: State<'_, Shell>) -> Result<GameLocations, ApiError> {
    shell.core()?.get_game_locations().await
}

#[tauri::command]
pub async fn set_game_locations(
    shell: State<'_, Shell>,
    locations: Vec<String>,
) -> Result<GameLocations, ApiError> {
    shell.core()?.set_game_locations(locations).await
}

#[tauri::command]
pub async fn list_accounts(shell: State<'_, Shell>) -> Result<Vec<Account>, ApiError> {
    shell.core()?.list_accounts().await
}

#[tauri::command]
pub async fn delete_account(
    shell: State<'_, Shell>,
    game: GameId,
    uid: String,
) -> Result<(), ApiError> {
    shell.core()?.delete_account(game, uid).await
}

#[tauri::command]
pub async fn clear_all_history(shell: State<'_, Shell>) -> Result<i64, ApiError> {
    shell.core()?.clear_all_history().await
}

#[tauri::command]
pub async fn get_history(
    shell: State<'_, Shell>,
    game: GameId,
    uid: String,
    banner: Option<String>,
    min_rank: Option<u8>,
) -> Result<Vec<HistoryPull>, ApiError> {
    shell.core()?.get_history(game, uid, banner, min_rank).await
}

#[tauri::command]
pub async fn get_pity(
    shell: State<'_, Shell>,
    game: GameId,
    uid: String,
) -> Result<Vec<BannerPity>, ApiError> {
    shell.core()?.get_pity(game, uid).await
}

#[tauri::command]
pub async fn get_catalog(shell: State<'_, Shell>) -> Result<GameCatalog, ApiError> {
    shell.core()?.get_catalog().await
}

#[tauri::command]
pub async fn get_banners(shell: State<'_, Shell>) -> Result<BannerList, ApiError> {
    shell.core()?.get_banners().await
}

#[tauri::command]
pub async fn get_catalog_status(shell: State<'_, Shell>) -> Result<CatalogStatus, ApiError> {
    shell.core()?.get_catalog_status().await
}

#[tauri::command]
pub async fn check_catalog(shell: State<'_, Shell>, force: bool) -> Result<CatalogCheck, ApiError> {
    shell.core()?.check_catalog(force).await
}

#[tauri::command]
pub async fn get_catalog_review(
    shell: State<'_, Shell>,
) -> Result<Option<CatalogReview>, ApiError> {
    shell.core()?.get_catalog_review().await
}

#[tauri::command]
pub async fn get_impact_page(
    shell: State<'_, Shell>,
    game: GameId,
    uid: String,
    offset: i64,
    limit: i64,
) -> Result<Vec<AffectedPull>, ApiError> {
    shell
        .core()?
        .get_impact_page(game, uid, offset, limit)
        .await
}

#[tauri::command]
pub async fn apply_catalog(
    shell: State<'_, Shell>,
    version: i64,
    fingerprint: String,
) -> Result<ApplyResult, ApiError> {
    shell.core()?.apply_catalog(version, fingerprint).await
}

#[tauri::command]
pub async fn dismiss_catalog(shell: State<'_, Shell>) -> Result<(), ApiError> {
    shell.core()?.dismiss_catalog().await
}

#[tauri::command]
pub async fn get_catalog_history(
    shell: State<'_, Shell>,
) -> Result<Vec<CatalogHistoryEntry>, ApiError> {
    shell.core()?.get_catalog_history().await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its State by value; it cannot be a reference"
)]
pub fn set_catalog_auto_apply(shell: State<'_, Shell>, enabled: bool) -> Result<bool, ApiError> {
    shell.core()?.set_catalog_auto_apply(enabled)
}

#[tauri::command]
pub async fn cloud_sign_in(
    shell: State<'_, Shell>,
    passphrase: String,
) -> Result<CloudStatus, ApiError> {
    shell.core()?.cloud_sign_in(passphrase).await
}

#[tauri::command]
pub async fn cloud_sign_out(
    shell: State<'_, Shell>,
    delete_backups: bool,
) -> Result<CloudStatus, ApiError> {
    shell.core()?.cloud_sign_out(delete_backups).await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its State by value; it cannot be a reference"
)]
pub fn cloud_set_auto_backup(
    shell: State<'_, Shell>,
    enabled: bool,
) -> Result<CloudStatus, ApiError> {
    shell.core()?.cloud_set_auto_backup(enabled)
}

#[tauri::command]
pub async fn cloud_backup_now(shell: State<'_, Shell>) -> Result<CloudBackup, ApiError> {
    shell.core()?.cloud_backup_now().await
}

#[tauri::command]
pub async fn cloud_backup_if_due(shell: State<'_, Shell>) -> Result<Option<CloudBackup>, ApiError> {
    shell.core()?.cloud_backup_if_due().await
}

#[tauri::command]
pub async fn cloud_list_backups(shell: State<'_, Shell>) -> Result<Vec<CloudBackup>, ApiError> {
    shell.core()?.cloud_list_backups().await
}

#[tauri::command]
pub async fn cloud_restore(
    shell: State<'_, Shell>,
    id: String,
    passphrase: Option<String>,
) -> Result<ImportSummary, ApiError> {
    shell.core()?.cloud_restore(id, passphrase).await
}

#[tauri::command]
pub async fn import_file(shell: State<'_, Shell>, path: String) -> Result<ImportSummary, ApiError> {
    shell.core()?.import_file(path).await
}

#[tauri::command]
pub async fn export_file(
    shell: State<'_, Shell>,
    path: String,
    format: String,
    game: Option<GameId>,
    uid: Option<String>,
) -> Result<(), ApiError> {
    shell.core()?.export_file(path, format, game, uid).await
}

/// `cloud_status` cannot fail, so it has no `Result` of its own; an unopened core is reported as an error.
#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its State by value; it cannot be a reference"
)]
pub fn cloud_status(shell: State<'_, Shell>) -> Result<CloudStatus, ApiError> {
    Ok(shell.core()?.cloud_status())
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its State by value; it cannot be a reference"
)]
pub fn cloud_cancel_sign_in(shell: State<'_, Shell>) -> Result<(), ApiError> {
    shell.core()?.cloud_cancel_sign_in();
    Ok(())
}

/// Reads the saved history and reports progress through `on_event`; the last event is always
/// `finished` or `failed`.
#[tauri::command]
pub async fn refresh_from_game(
    shell: State<'_, Shell>,
    game: GameId,
    full: bool,
    on_event: Channel<RefreshEvent>,
) -> Result<(), ApiError> {
    let emit = emitter(on_event);
    match shell.core() {
        Ok(core) => core.refresh_from_game(game, full, emit).await,
        Err(e) => emit(failed(e)),
    }
    Ok(())
}

/// The same with a link the person pasted.
#[tauri::command]
pub async fn fetch_history(
    shell: State<'_, Shell>,
    game: GameId,
    url: String,
    full: bool,
    on_event: Channel<RefreshEvent>,
) -> Result<(), ApiError> {
    let emit = emitter(on_event);
    match shell.core() {
        Ok(core) => core.fetch_history(game, url, full, emit).await,
        Err(e) => emit(failed(e)),
    }
    Ok(())
}

fn emitter(channel: Channel<RefreshEvent>) -> Emit {
    Arc::new(move |event| {
        let _ = channel.send(event);
    })
}

// ---- Calls that need no open database ----

#[tauri::command]
pub fn game_meta() -> Vec<GameMeta> {
    core_api::game_meta()
}

#[tauri::command]
pub fn get_about() -> AboutInfo {
    core_api::get_about()
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), ApiError> {
    core_api::open_external(url)
}

#[tauri::command]
pub async fn check_app_update() -> Result<AppUpdate, ApiError> {
    core_api::check_app_update().await
}

#[tauri::command]
pub fn set_logging_enabled(enabled: bool) {
    core_api::set_logging_enabled(enabled);
}

#[tauri::command]
pub fn get_logs() -> Vec<LogEntry> {
    core_api::get_logs()
}

#[tauri::command]
pub fn clear_logs() {
    core_api::clear_logs();
}
