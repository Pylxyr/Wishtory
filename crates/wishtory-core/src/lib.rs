// Wishtory core: SAFETY MODEL
// -----------------------------------------------------------------------------
// * Game files are accessed READ-ONLY, and only from Rust (discover.rs, authkey.rs).
// * We never attach to / inject into / talk to the game process.
// * We never write to game folders, never set proxies, never require admin rights.
// * Network traffic is the official HoYoverse history API (fetch.rs), the signed catalog, banner
//   list and pity rules (catalog.rs, rules.rs) and a version check of this project's GitHub releases (update.rs). None of
//   them is sent an account or wish data.
// -----------------------------------------------------------------------------
//! UI-agnostic core of Wishtory. A front end (the Flutter app, through `wishtory-bridge`) opens a
//! [`Core`] once and calls its methods; nothing in this crate knows about windows or toolkits.

#[macro_use]
pub mod logging;
pub mod about;
pub mod authkey;
pub mod catalog;
pub mod db;
pub mod discover;
pub mod error;
pub mod fetch;
pub mod fsutil;
pub mod models;
pub mod ops;
pub mod paimon_items;
pub mod pity;
pub mod rules;
pub mod settings;
pub mod uigf;
pub mod update;
#[cfg(unix)]
mod wine;

use catalog::{BannerSet, BannerStore, BannerUpdate, Catalog, CatalogStore};
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
    pub catalog: CatalogStore,
    pub banners: BannerStore,
    pub rules: RulesStore,
    pub settings: SettingsStore,
}

/// Status lines while a refresh runs. Plain text only: never a link or authkey.
pub type Progress = Arc<dyn Fn(String) + Send + Sync>;

impl Core {
    pub fn open(dir: &Path) -> Result<Self, AppError> {
        std::fs::create_dir_all(dir)?;
        Ok(Self {
            db: Db::open(&dir.join("wishtory.db"))?,
            catalog: CatalogStore::open(dir),
            banners: BannerStore::open(dir),
            rules: RulesStore::open(dir),
            settings: SettingsStore::open(dir),
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
    pub async fn refresh_catalog(&self, force: bool) -> Result<String, AppError> {
        ops::refresh_catalog(&self.catalog, &self.rules, force).await
    }
    pub async fn get_banners(&self) -> Result<Arc<BannerSet>, AppError> {
        ops::get_banners(&self.banners).await
    }
    pub async fn get_banner_update(&self) -> Result<Option<BannerUpdate>, AppError> {
        ops::get_banner_update(&self.banners).await
    }
    pub async fn refresh_banners(&self, force: bool) -> Result<BannerUpdate, AppError> {
        ops::refresh_banners(&self.db, &self.banners, force).await
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
}
