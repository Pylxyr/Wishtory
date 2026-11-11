// Wishtory core: SAFETY MODEL
// -----------------------------------------------------------------------------
// * Game files are accessed READ-ONLY, and only from Rust (discover.rs, authkey.rs).
// * We never attach to / inject into / talk to the game process.
// * We never write to game folders, never set proxies, never require admin rights.
// * Network traffic is the official HoYoverse history API (fetch.rs), the signed catalog, banner
//   list and pity rules (catalog.rs, rules.rs) and a version check of this project's GitHub releases (update.rs). None of
//   them is sent an account or wish data.
// * Cloud backup (cloud/) is OFF until the person signs in to Google. Then, and only then, wish
//   history is uploaded, encrypted on this PC with the person's passphrase, to the hidden app-data
//   folder of their own Google Drive. The Google sign-in and passphrase are kept in the operating
//   system's credential store, never in a file.
// -----------------------------------------------------------------------------
//! UI-agnostic core of Wishtory. A front end (the Flutter app, through `wishtory-bridge`) opens a
//! [`Core`] once and calls its methods; nothing in this crate knows about windows or toolkits.

#[macro_use]
pub mod logging;
pub mod about;
pub mod authkey;
pub mod catalog;
pub mod cloud;
pub mod db;
pub mod discover;
pub mod error;
pub mod fetch;
pub mod fsutil;
pub mod impact;
pub mod models;
pub mod net;
pub mod ops;
pub mod paimon_items;
pub mod pity;
pub mod rules;
pub mod settings;
pub mod uigf;
pub mod update;
#[cfg(unix)]
mod wine;

use catalog::{BannerSet, BannerStore, Catalog, CatalogStore, ReleaseStore};
use db::Db;
use error::AppError;
use models::{Account, Game, GameInstall, GameLocations};
use pity::BannerPity;
use rules::RulesStore;
use settings::SettingsStore;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// The identifier of Wishtory's data folder. It has not changed since the first release, so a person's
/// saved history, cached catalog and settings are found again after every update.
pub const APP_ID: &str = "app.wishtory.desktop";

/// `%APPDATA%\app.wishtory.desktop` on Windows, `~/.local/share/app.wishtory.desktop` on Linux.
pub fn default_data_dir() -> Result<PathBuf, AppError> {
    dirs::data_dir()
        .map(|d| d.join(APP_ID))
        .ok_or_else(|| AppError::Io("Could not find a place to keep Wishtory's data.".into()))
}

/// Everything the app needs, opened once at start-up.
pub struct Core {
    pub db: Db,
    /// The release in use, a staged one and the catalog history.
    pub release: Arc<ReleaseStore>,
    pub catalog: Arc<CatalogStore>,
    pub banners: Arc<BannerStore>,
    pub rules: Arc<RulesStore>,
    pub settings: SettingsStore,
    pub cloud: cloud::Cloud,
}

/// Status lines while a refresh runs. Plain text only: never a link or authkey.
pub type Progress = Arc<dyn Fn(String) + Send + Sync>;

impl Core {
    pub fn open(dir: &Path) -> Result<Self, AppError> {
        std::fs::create_dir_all(dir)?;
        let feeds = ReleaseStore::open(dir);
        let db = Db::open(&dir.join("wishtory.db"))?;
        // Saved banners always belong to the banner list in use (this also repairs a crash
        // between a release becoming current and its banners being re-derived).
        db.ensure_derived(&format!(
            "banners:{}",
            feeds.banners.current().banners_version
        ))?;
        Ok(Self {
            db,
            release: Arc::new(feeds.release),
            catalog: Arc::new(feeds.catalog),
            banners: Arc::new(feeds.banners),
            rules: Arc::new(feeds.rules),
            settings: SettingsStore::open(dir),
            cloud: cloud::Cloud::new(),
        })
    }

    pub async fn discover_games(&self) -> Result<Vec<GameInstall>, AppError> {
        ops::discover_games().await
    }
    pub async fn get_game_locations(&self) -> Result<GameLocations, AppError> {
        ops::get_game_locations(&self.settings).await
    }
    pub async fn set_game_locations(
        &self,
        locations: Vec<String>,
    ) -> Result<GameLocations, AppError> {
        ops::set_game_locations(&self.settings, locations).await
    }
    pub async fn extract_authkey(&self, game: Game) -> Result<String, AppError> {
        ops::extract_authkey(game).await
    }
    pub async fn fetch_history(
        &self,
        game: Game,
        url: String,
        full: bool,
        progress: Progress,
    ) -> Result<ops::FetchSummary, AppError> {
        let progress = move |m: String| progress(m);
        ops::fetch_history(&progress, &self.db, &self.banners, game, url, Some(full)).await
    }
    pub async fn refresh_from_game(
        &self,
        game: Game,
        full: bool,
        progress: Progress,
    ) -> Result<ops::FetchSummary, AppError> {
        let progress = move |m: String| progress(m);
        ops::refresh_from_game(&progress, &self.db, &self.banners, game, Some(full)).await
    }
    pub async fn list_accounts(&self) -> Result<Vec<Account>, AppError> {
        ops::list_accounts(&self.db).await
    }
    pub async fn delete_account(&self, game: Game, uid: String) -> Result<(), AppError> {
        ops::delete_account(&self.db, game, uid).await
    }
    pub async fn clear_all_history(&self) -> Result<usize, AppError> {
        ops::clear_all_history(&self.db).await
    }
    pub async fn get_catalog(&self) -> Result<Arc<Catalog>, AppError> {
        ops::get_catalog(&self.catalog).await
    }
    pub async fn get_banners(&self) -> Result<Arc<BannerSet>, AppError> {
        ops::get_banners(&self.banners).await
    }
    fn catalog_ctx(&self) -> ops::CatalogCtx {
        ops::CatalogCtx {
            db: self.db.clone(),
            release: self.release.clone(),
            catalog: self.catalog.clone(),
            banners: self.banners.clone(),
            rules: self.rules.clone(),
            auto_apply: self.settings.catalog().auto_apply,
        }
    }
    pub async fn get_catalog_status(&self) -> Result<ops::CatalogStatus, AppError> {
        ops::get_catalog_status(&self.catalog_ctx()).await
    }
    pub async fn check_catalog(&self, force: bool) -> Result<ops::CatalogCheck, AppError> {
        ops::check_catalog(&self.catalog_ctx(), force).await
    }
    pub async fn get_catalog_review(&self) -> Result<Option<ops::CatalogReview>, AppError> {
        ops::get_catalog_review(&self.catalog_ctx()).await
    }
    pub async fn get_impact_page(
        &self,
        game: Game,
        uid: String,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<impact::AffectedPull>, AppError> {
        ops::get_impact_page(&self.catalog_ctx(), game, uid, offset, limit).await
    }
    pub async fn apply_catalog(
        &self,
        reviewed: Option<(u64, String)>,
    ) -> Result<ops::ApplyResult, AppError> {
        ops::apply_catalog(&self.catalog_ctx(), reviewed).await
    }
    pub async fn dismiss_catalog(&self) -> Result<(), AppError> {
        ops::dismiss_catalog(&self.catalog_ctx()).await
    }
    pub async fn get_catalog_history(&self) -> Result<Vec<catalog::HistoryEntry>, AppError> {
        ops::get_catalog_history(&self.catalog_ctx()).await
    }
    pub fn catalog_auto_apply(&self) -> bool {
        self.settings.catalog().auto_apply
    }
    pub fn set_catalog_auto_apply(&self, enabled: bool) -> Result<bool, AppError> {
        Ok(self.settings.set_catalog_auto_apply(enabled)?.auto_apply)
    }
    pub async fn get_history(
        &self,
        game: Game,
        uid: String,
        banner: Option<String>,
    ) -> Result<Vec<ops::HistoryPull>, AppError> {
        ops::get_history(&self.db, &self.catalog, &self.banners, game, uid, banner).await
    }
    pub async fn get_pity(&self, game: Game, uid: String) -> Result<Vec<BannerPity>, AppError> {
        ops::get_pity(&self.db, &self.banners, &self.rules, game, uid).await
    }
    pub async fn import_file(&self, path: String) -> Result<uigf::ImportSummary, AppError> {
        ops::import_file(&self.db, &self.catalog, &self.banners, path).await
    }
    pub async fn export_file(
        &self,
        path: String,
        format: String,
        game: Option<Game>,
        uid: Option<String>,
    ) -> Result<(), AppError> {
        ops::export_file(&self.db, &self.catalog, path, format, game, uid).await
    }

    // ---- Cloud backup (Google Drive app-data folder) ----

    pub fn cloud_status(&self) -> cloud::CloudStatus {
        self.cloud.status(&self.settings)
    }
    pub async fn cloud_sign_in(&self, passphrase: String) -> Result<cloud::CloudStatus, AppError> {
        self.cloud.sign_in(&self.settings, passphrase).await
    }
    pub fn cloud_cancel_sign_in(&self) {
        self.cloud.cancel_sign_in();
    }
    pub async fn cloud_sign_out(
        &self,
        delete_backups: bool,
    ) -> Result<cloud::CloudStatus, AppError> {
        self.cloud.sign_out(&self.settings, delete_backups).await
    }
    pub fn cloud_set_auto_backup(&self, enabled: bool) -> Result<cloud::CloudStatus, AppError> {
        self.cloud.set_auto_backup(&self.settings, enabled)
    }
    pub async fn cloud_backup_now(&self) -> Result<cloud::BackupFile, AppError> {
        self.cloud
            .backup_now(&self.db, &self.catalog, &self.settings)
            .await
    }
    pub async fn cloud_backup_if_due(&self) -> Result<Option<cloud::BackupFile>, AppError> {
        self.cloud
            .backup_if_due(&self.db, &self.catalog, &self.settings)
            .await
    }
    pub async fn cloud_list_backups(&self) -> Result<Vec<cloud::BackupFile>, AppError> {
        self.cloud.list_backups(&self.settings).await
    }
    pub async fn cloud_restore(
        &self,
        id: String,
        passphrase: Option<String>,
    ) -> Result<uigf::ImportSummary, AppError> {
        self.cloud
            .restore(
                &self.db,
                &self.catalog,
                &self.banners,
                &self.settings,
                id,
                passphrase,
            )
            .await
    }
}
