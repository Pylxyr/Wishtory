//! The calls the Flutter app makes. One opaque [`WishtoryCore`] handle holds the open database and
//! the cached feeds; a few app-level helpers are free functions.

use super::{error::ApiError, types::*};
use crate::convert::update_status;
use crate::frb_generated::StreamSink;
use std::{path::PathBuf, sync::Arc};
use wishtory_core::{about, logging, models::Game, ops, update, Core};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshEventKind {
    /// `message` holds a short, link-free status line ("Downloading Character Event: page 2…").
    Progress,
    /// `summary` is set.
    Finished,
    /// `error` is set.
    Failed,
}

/// What a refresh reports while it runs. The stream always ends with `Finished` or `Failed`.
/// (A plain struct rather than an enum with data, so the generated Dart needs no `freezed`.)
#[derive(Debug, Clone)]
pub struct RefreshEvent {
    pub kind: RefreshEventKind,
    pub message: String,
    pub summary: Option<FetchSummary>,
    pub error: Option<ApiError>,
}

fn event(
    kind: RefreshEventKind,
    message: String,
    summary: Option<FetchSummary>,
    error: Option<ApiError>,
) -> RefreshEvent {
    RefreshEvent {
        kind,
        message,
        summary,
        error,
    }
}

pub struct WishtoryCore {
    inner: Arc<Core>,
}

impl WishtoryCore {
    /// Opens (creating if needed) Wishtory's data. `data_dir` is for tests; `None` uses the same
    /// folder every earlier release used, so saved history carries over.
    pub async fn open(data_dir: Option<String>) -> Result<WishtoryCore, ApiError> {
        let dir = match data_dir {
            Some(d) => PathBuf::from(d),
            None => wishtory_core::default_data_dir()?,
        };
        Ok(WishtoryCore {
            inner: Arc::new(Core::open(&dir)?),
        })
    }

    pub async fn discover_games(&self) -> Result<Vec<GameInstall>, ApiError> {
        Ok(self
            .inner
            .discover_games()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
    pub async fn get_game_locations(&self) -> Result<GameLocations, ApiError> {
        Ok(self.inner.get_game_locations().await?.into())
    }
    pub async fn set_game_locations(
        &self,
        locations: Vec<String>,
    ) -> Result<GameLocations, ApiError> {
        Ok(self.inner.set_game_locations(locations).await?.into())
    }

    /// Read-only: finds the history link the game already cached, then downloads.
    /// `full` downloads everything the server still keeps instead of stopping at what is saved.
    pub async fn refresh_from_game(
        &self,
        game: GameId,
        full: bool,
        events: StreamSink<RefreshEvent>,
    ) {
        let progress = progress_sink(&events);
        let result = self
            .inner
            .refresh_from_game(game.into(), full, progress)
            .await;
        finish(&events, result);
    }

    /// Same, with a link the person pasted.
    pub async fn fetch_history(
        &self,
        game: GameId,
        url: String,
        full: bool,
        events: StreamSink<RefreshEvent>,
    ) {
        let progress = progress_sink(&events);
        let result = self
            .inner
            .fetch_history(game.into(), url, full, progress)
            .await;
        finish(&events, result);
    }

    pub async fn list_accounts(&self) -> Result<Vec<Account>, ApiError> {
        Ok(self
            .inner
            .list_accounts()
            .await?
            .into_iter()
            .filter_map(Account::from_core)
            .collect())
    }
    pub async fn delete_account(&self, game: GameId, uid: String) -> Result<(), ApiError> {
        Ok(self.inner.delete_account(game.into(), uid).await?)
    }
    /// Returns how many pulls were removed.
    pub async fn clear_all_history(&self) -> Result<i64, ApiError> {
        Ok(self.inner.clear_all_history().await? as i64)
    }

    /// Newest first. `banner` is one of the keys from [`game_meta`], or `None` for every banner.
    pub async fn get_history(
        &self,
        game: GameId,
        uid: String,
        banner: Option<String>,
    ) -> Result<Vec<HistoryPull>, ApiError> {
        Ok(self
            .inner
            .get_history(game.into(), uid, banner)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
    pub async fn get_pity(&self, game: GameId, uid: String) -> Result<Vec<BannerPity>, ApiError> {
        Ok(self
            .inner
            .get_pity(game.into(), uid)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn get_catalog(&self) -> Result<GameCatalog, ApiError> {
        let catalog = self.inner.get_catalog().await?;
        Ok(GameCatalog::from(&*catalog))
    }
    /// Updates the signed catalog and pity rules. `force` skips the once-a-day gate.
    pub async fn refresh_catalog(&self, force: bool) -> Result<UpdateStatus, ApiError> {
        Ok(update_status(&self.inner.refresh_catalog(force).await?))
    }
    pub async fn get_banners(&self) -> Result<BannerList, ApiError> {
        let banners = self.inner.get_banners().await?;
        Ok(BannerList::from(&*banners))
    }
    pub async fn get_banner_update(&self) -> Result<Option<BannerUpdate>, ApiError> {
        Ok(self.inner.get_banner_update().await?.map(Into::into))
    }
    pub async fn refresh_banners(&self, force: bool) -> Result<BannerUpdate, ApiError> {
        Ok(self.inner.refresh_banners(force).await?.into())
    }

    // ---- Cloud backup: the person's own Google Drive, encrypted on this PC ----

    pub fn cloud_status(&self) -> CloudStatus {
        self.inner.cloud_status().into()
    }
    /// Opens Google's sign-in page in the browser and waits (up to five minutes) for the person to
    /// finish. `passphrase` encrypts every backup and is kept in the system's credential store.
    pub async fn cloud_sign_in(&self, passphrase: String) -> Result<CloudStatus, ApiError> {
        Ok(self.inner.cloud_sign_in(passphrase).await?.into())
    }
    /// Gives up a sign-in that is waiting for the browser.
    pub fn cloud_cancel_sign_in(&self) {
        self.inner.cloud_cancel_sign_in();
    }
    /// Disconnects from Google. `delete_backups` also deletes the backup files from Drive first.
    pub async fn cloud_sign_out(&self, delete_backups: bool) -> Result<CloudStatus, ApiError> {
        Ok(self.inner.cloud_sign_out(delete_backups).await?.into())
    }
    pub fn cloud_set_auto_backup(&self, enabled: bool) -> Result<CloudStatus, ApiError> {
        Ok(self.inner.cloud_set_auto_backup(enabled)?.into())
    }
    pub async fn cloud_backup_now(&self) -> Result<CloudBackup, ApiError> {
        Ok(self.inner.cloud_backup_now().await?.into())
    }
    /// The automatic backup: runs only if switched on and the last one is over a day old.
    pub async fn cloud_backup_if_due(&self) -> Result<Option<CloudBackup>, ApiError> {
        Ok(self.inner.cloud_backup_if_due().await?.map(Into::into))
    }
    /// Newest first.
    pub async fn cloud_list_backups(&self) -> Result<Vec<CloudBackup>, ApiError> {
        Ok(self
            .inner
            .cloud_list_backups()
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }
    /// Merges a backup into the saved history. `passphrase` is only for a backup made under a
    /// different passphrase than the one saved on this PC.
    pub async fn cloud_restore(
        &self,
        id: String,
        passphrase: Option<String>,
    ) -> Result<ImportSummary, ApiError> {
        Ok(self.inner.cloud_restore(id, passphrase).await?.into())
    }

    pub async fn import_file(&self, path: String) -> Result<ImportSummary, ApiError> {
        Ok(self.inner.import_file(path).await?.into())
    }
    /// `format` is one of the ids the Import / Export page offers; `game` and `uid` select one
    /// account, or both `None` for everything.
    pub async fn export_file(
        &self,
        path: String,
        format: String,
        game: Option<GameId>,
        uid: Option<String>,
    ) -> Result<(), ApiError> {
        Ok(self
            .inner
            .export_file(path, format, game.map(Game::from), uid)
            .await?)
    }
}

fn progress_sink(events: &StreamSink<RefreshEvent>) -> wishtory_core::Progress {
    let events = events.clone();
    Arc::new(move |line: String| {
        let _ = events.add(event(RefreshEventKind::Progress, line, None, None));
    })
}

fn finish(
    events: &StreamSink<RefreshEvent>,
    result: Result<ops::FetchSummary, wishtory_core::error::AppError>,
) {
    let event = match result.map_err(ApiError::from).and_then(|s| {
        FetchSummary::from_core(s).ok_or_else(|| ApiError {
            code: super::error::ErrorCode::Other,
            message: "The history belongs to a game this version does not know.".into(),
        })
    }) {
        Ok(summary) => event(
            RefreshEventKind::Finished,
            String::new(),
            Some(summary),
            None,
        ),
        Err(e) => event(RefreshEventKind::Failed, e.message.clone(), None, Some(e)),
    };
    let _ = events.add(event);
}

// ---- App-level helpers that need no open database ----

/// Static per-game naming (the History banner filter), kept in Rust so Dart holds no copy.
pub fn game_meta() -> Vec<GameMeta> {
    ops::get_game_meta()
        .into_iter()
        .filter_map(GameMeta::from_core)
        .collect()
}
pub fn get_about() -> AboutInfo {
    about::get_about().into()
}
/// Opens one of the links Wishtory shows. Anything outside the fixed allow-list is refused.
pub fn open_external(url: String) -> Result<(), ApiError> {
    Ok(about::open_external(url)?)
}
pub async fn check_app_update() -> Result<AppUpdate, ApiError> {
    Ok(update::check_app_update().await?.into())
}
pub fn set_logging_enabled(enabled: bool) {
    logging::set_logging_enabled(enabled);
}
pub fn get_logs() -> Vec<LogEntry> {
    logging::get_logs().into_iter().map(Into::into).collect()
}
pub fn clear_logs() {
    logging::clear_logs();
}
