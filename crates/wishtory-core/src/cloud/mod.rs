//! Cloud backup to the person's own Google Drive, in the way Swift Backup does it: sign in once,
//! then back up and restore.
//!
//! What this is built around, in order of importance:
//!
//! 1. **Privacy.** A backup is encrypted on this PC with a passphrase only the person knows
//!    ([`vault`]) before it is uploaded. Google, and anyone who gets into the Drive account, only
//!    ever sees an opaque file.
//! 2. **Least access.** The single permission requested is Drive's *app data* folder
//!    ([`oauth::SCOPE`]): a hidden folder only Wishtory can open. No other file in Drive is visible.
//! 3. **Secrets in the OS credential store** ([`secrets`]), never in a file.
//! 4. **Nothing happens unless asked.** Signing in, and each automatic daily backup, are opt-in.
//!
//! The backup contents are the same UIGF v4 file the Export page writes, so a backup can also be
//! read by other trackers once decrypted. Restoring merges into the existing history (the import
//! already skips what is stored), so it can never remove a pull.

pub mod drive;
pub mod oauth;
pub mod secrets;
pub mod vault;

#[cfg(test)]
mod testutil;

use crate::{
    about,
    catalog::{BannerStore, CatalogStore},
    db::Db,
    error::AppError,
    logging::Level,
    net, ops,
    settings::{CloudSettings, SettingsStore},
    uigf::{self, ImportSummary},
};
use drive::{Drive, DriveError};
use oauth::{Access, Credentials, Endpoints, Loopback, OAuth, TokenError};
use secrets::{OsKeyring, SecretStore, PASSPHRASE, REFRESH_TOKEN};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Notify};
use zeroize::Zeroizing;

pub use drive::BackupFile;

/// How many backups are kept in Drive; older ones are deleted after each new one.
pub const KEEP_BACKUPS: usize = 5;
/// A backup file larger than this is refused (a full three-game history is a few MB at most).
const MAX_BACKUP_BYTES: usize = 64 * 1024 * 1024;
const NOTHING_TO_BACK_UP: &str =
    "There is no wish history to back up yet. Refresh from the game or import a file first.";
/// Automatic backups run at most this often.
const AUTO_INTERVAL: chrono::Duration = chrono::Duration::hours(24);

/// What the Settings page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudStatus {
    /// False in builds made without a Google client ID.
    pub available: bool,
    pub signed_in: bool,
    pub account: Option<String>,
    pub auto_backup: bool,
    pub last_backup: Option<String>,
}

struct Parts {
    oauth: OAuth,
    drive: Drive,
}

pub struct Cloud {
    parts: Option<Parts>,
    secrets: Arc<dyn SecretStore>,
    /// The short-lived Drive token, kept in memory only.
    access: Mutex<Option<Access>>,
    /// One backup, restore or sign-in at a time.
    busy: Mutex<()>,
    cancel: Notify,
}

impl Cloud {
    /// The real service. Without a Google client ID in this build it is present but unavailable.
    pub fn new() -> Self {
        let parts = Credentials::built_in().and_then(|credentials| {
            let http = net::client_builder()
                .timeout(Duration::from_secs(60))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .ok()?;
            Some(Parts {
                oauth: OAuth::new(http.clone(), Endpoints::GOOGLE, credentials),
                drive: Drive::new(http),
            })
        });
        Self::with(parts, Arc::new(OsKeyring))
    }

    fn with(parts: Option<Parts>, secrets: Arc<dyn SecretStore>) -> Self {
        Self {
            parts,
            secrets,
            access: Mutex::new(None),
            busy: Mutex::new(()),
            cancel: Notify::new(),
        }
    }

    fn parts(&self) -> Result<&Parts, AppError> {
        self.parts.as_ref().ok_or_else(|| {
            AppError::Message(
                "This build of Wishtory was made without cloud backup. See docs/cloud-backup.md \
                 to build it with your own Google client ID."
                    .into(),
            )
        })
    }

    pub fn status(&self, settings: &SettingsStore) -> CloudStatus {
        let saved = settings.cloud();
        CloudStatus {
            available: self.parts.is_some(),
            signed_in: saved.account.is_some(),
            account: saved.account,
            auto_backup: saved.auto_backup,
            last_backup: saved.last_backup,
        }
    }

    // ---- Signing in and out ----

    /// Opens Google's sign-in page in the browser and waits for the person to finish. The passphrase
    /// is checked first, so a weak one is refused before anything is opened.
    pub async fn sign_in(
        &self,
        settings: &SettingsStore,
        passphrase: String,
    ) -> Result<CloudStatus, AppError> {
        let passphrase = Zeroizing::new(passphrase);
        vault::validate_passphrase(&passphrase)?;
        let parts = self.parts()?;
        let _busy = self.busy.try_lock().map_err(|_| already_running())?;

        let loopback = Loopback::bind().await?;
        let redirect = loopback.redirect_uri();
        let pkce = oauth::new_pkce()?;
        let state = oauth::random_token(32)?;
        let url = parts
            .oauth
            .authorization_url(&redirect, &pkce.challenge, &state)?;
        if url.scheme() != "https" || url.host_str() != Some("accounts.google.com") {
            return Err(AppError::Message(
                "Refused to open an unexpected sign-in page.".into(),
            ));
        }
        about::open_in_browser(url.as_str()).map_err(AppError::Message)?;
        wlog!(Level::Info, "cloud: waiting for Google sign-in");

        let code = tokio::select! {
            code = loopback.wait_for_code(&state) => code?,
            () = self.cancel.notified() => {
                return Err(AppError::Message("Sign-in was cancelled.".into()));
            }
        };
        let signed = parts
            .oauth
            .exchange_code(&code, &pkce.verifier, &redirect)
            .await?;

        let email = match parts.drive.account_email(&signed.access.token).await {
            Ok(email) => email,
            Err(DriveError::Unauthorized) => String::new(),
            Err(DriveError::Failed(e)) => return Err(e),
        };
        // Both secrets or neither: a half-saved sign-in would fail later in confusing ways.
        self.secrets.set(PASSPHRASE, &passphrase)?;
        if let Err(e) = self.secrets.set(REFRESH_TOKEN, &signed.refresh_token) {
            let _ = self.secrets.delete(PASSPHRASE);
            return Err(e);
        }
        *self.access.lock().await = Some(signed.access);
        let account = if email.is_empty() {
            "Google account".to_string()
        } else {
            email
        };
        settings.update_cloud(|c| {
            c.account = Some(account);
            c.last_backup = None;
        })?;
        wlog!(Level::Info, "cloud: signed in");
        Ok(self.status(settings))
    }

    /// Gives up a sign-in that is waiting for the browser.
    pub fn cancel_sign_in(&self) {
        self.cancel.notify_waiters();
    }

    /// Disconnects Wishtory from Google. With `delete_backups` the backup files are deleted from
    /// Drive first. Either way the sign-in is revoked with Google and the secrets are removed from
    /// this PC.
    pub async fn sign_out(
        &self,
        settings: &SettingsStore,
        delete_backups: bool,
    ) -> Result<CloudStatus, AppError> {
        let _busy = self.busy.try_lock().map_err(|_| already_running())?;
        if let Some(parts) = &self.parts {
            if delete_backups {
                let files = self
                    .call(settings, |t| async move { parts.drive.list(&t).await })
                    .await?;
                for file in files {
                    let id = file.id;
                    self.call(settings, |t| {
                        let id = id.clone();
                        async move { parts.drive.delete(&t, &id).await }
                    })
                    .await?;
                }
                wlog!(Level::Info, "cloud: backups deleted from Drive");
            }
            if let Some(token) = self.secrets.get(REFRESH_TOKEN)? {
                if let Err(e) = parts.oauth.revoke(&token).await {
                    wlog!(Level::Warn, "cloud: could not revoke the sign-in: {e}");
                }
            }
        }
        self.forget_local(settings).await?;
        wlog!(Level::Info, "cloud: signed out");
        Ok(self.status(settings))
    }

    async fn forget_local(&self, settings: &SettingsStore) -> Result<(), AppError> {
        *self.access.lock().await = None;
        let tokens = self.secrets.delete(REFRESH_TOKEN);
        let phrase = self.secrets.delete(PASSPHRASE);
        settings.update_cloud(|c| *c = CloudSettings::default())?;
        tokens.and(phrase)
    }

    pub fn set_auto_backup(
        &self,
        settings: &SettingsStore,
        enabled: bool,
    ) -> Result<CloudStatus, AppError> {
        require_signed_in(settings)?;
        settings.update_cloud(|c| c.auto_backup = enabled)?;
        Ok(self.status(settings))
    }

    // ---- Backing up and restoring ----

    pub async fn backup_now(
        &self,
        db: &Db,
        catalog: &CatalogStore,
        settings: &SettingsStore,
    ) -> Result<BackupFile, AppError> {
        let _busy = self.busy.try_lock().map_err(|_| already_running())?;
        self.backup_locked(db, catalog, settings).await
    }

    /// Runs the automatic backup if it is switched on and the last one is over a day old. Quiet when
    /// there is nothing to do or another operation is running.
    pub async fn backup_if_due(
        &self,
        db: &Db,
        catalog: &CatalogStore,
        settings: &SettingsStore,
    ) -> Result<Option<BackupFile>, AppError> {
        let saved = settings.cloud();
        if self.parts.is_none() || saved.account.is_none() || !saved.auto_backup {
            return Ok(None);
        }
        let due = saved
            .last_backup
            .as_deref()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .is_none_or(|last| chrono::Utc::now() - last.to_utc() >= AUTO_INTERVAL);
        if !due {
            return Ok(None);
        }
        let Ok(_busy) = self.busy.try_lock() else {
            return Ok(None);
        };
        match self.backup_locked(db, catalog, settings).await {
            Ok(file) => Ok(Some(file)),
            // No history yet is not worth a message.
            Err(AppError::Message(m)) if m == NOTHING_TO_BACK_UP => Ok(None),
            Err(e) => Err(e),
        }
    }

    async fn backup_locked(
        &self,
        db: &Db,
        catalog: &CatalogStore,
        settings: &SettingsStore,
    ) -> Result<BackupFile, AppError> {
        let parts = self.parts()?;
        require_signed_in(settings)?;
        let passphrase = self.passphrase()?;
        let catalog = catalog.current();
        let plain = db
            .run(move |db| uigf::export_with_catalog(db, "uigf4", None, None, &catalog))
            .await
            .map_err(|e| match e {
                AppError::Format(m) if m.contains("nothing to export") => {
                    AppError::Message(NOTHING_TO_BACK_UP.into())
                }
                other => other,
            })?;
        let plain = Zeroizing::new(plain);
        // Hashing the passphrase is deliberately slow and memory-hungry: not on the async runtime.
        let sealed =
            tokio::task::spawn_blocking(move || vault::seal(&passphrase, plain.as_bytes()))
                .await
                .map_err(|e| AppError::Io(format!("A background task failed: {e}")))??;

        let name = drive::backup_name(chrono::Utc::now());
        let file = self
            .call(settings, |t| {
                let (name, sealed) = (&name, &sealed);
                async move { parts.drive.upload(&t, name, sealed).await }
            })
            .await?;
        wlog!(
            Level::Info,
            "cloud: backup uploaded ({} bytes)",
            sealed.len()
        );

        // Keep the newest few. A failure here must not fail a backup that already succeeded.
        match self
            .call(settings, |t| async move { parts.drive.list(&t).await })
            .await
        {
            Ok(all) => {
                for old in all.into_iter().skip(KEEP_BACKUPS) {
                    let id = old.id;
                    let removed = self
                        .call(settings, |t| {
                            let id = id.clone();
                            async move { parts.drive.delete(&t, &id).await }
                        })
                        .await;
                    if let Err(e) = removed {
                        wlog!(Level::Warn, "cloud: could not remove an old backup: {e}");
                    }
                }
            }
            Err(e) => wlog!(Level::Warn, "cloud: could not tidy old backups: {e}"),
        }
        settings.update_cloud(|c| c.last_backup = Some(chrono::Utc::now().to_rfc3339()))?;
        Ok(file)
    }

    /// The backups in Drive, newest first.
    pub async fn list_backups(
        &self,
        settings: &SettingsStore,
    ) -> Result<Vec<BackupFile>, AppError> {
        let parts = self.parts()?;
        require_signed_in(settings)?;
        self.call(settings, |t| async move { parts.drive.list(&t).await })
            .await
    }

    /// Downloads, decrypts and merges a backup into the history. `passphrase` is only for a backup
    /// that was made under a different passphrase than the one saved on this PC.
    pub async fn restore(
        &self,
        db: &Db,
        catalog: &CatalogStore,
        banners: &BannerStore,
        settings: &SettingsStore,
        id: String,
        passphrase: Option<String>,
    ) -> Result<ImportSummary, AppError> {
        let parts = self.parts()?;
        require_signed_in(settings)?;
        let _busy = self.busy.try_lock().map_err(|_| already_running())?;
        let passphrase = match passphrase {
            Some(p) => Zeroizing::new(p),
            None => self.passphrase()?,
        };
        let sealed = self
            .call(settings, |t| {
                let id = &id;
                async move { parts.drive.download(&t, id, MAX_BACKUP_BYTES).await }
            })
            .await?;
        let plain = tokio::task::spawn_blocking(move || vault::open(&passphrase, &sealed))
            .await
            .map_err(|e| AppError::Io(format!("A background task failed: {e}")))??;
        let text = String::from_utf8(plain.to_vec())
            .map_err(|_| AppError::Format("the backup does not contain wish history".into()))?;
        let summary = ops::import_text(db, catalog, banners, text).await?;
        wlog!(
            Level::Info,
            "cloud: backup restored ({} new)",
            summary.added
        );
        Ok(summary)
    }

    // ---- Tokens ----

    fn passphrase(&self) -> Result<Zeroizing<String>, AppError> {
        self.secrets.get(PASSPHRASE)?.ok_or_else(|| {
            AppError::Message(
                "The backup passphrase is not saved on this PC. Sign in again to set it.".into(),
            )
        })
    }

    async fn access_token(&self, settings: &SettingsStore) -> Result<Zeroizing<String>, AppError> {
        let parts = self.parts()?;
        let mut cached = self.access.lock().await;
        if let Some(access) = cached.as_ref().filter(|a| a.is_fresh()) {
            return Ok(access.token.clone());
        }
        let refresh = self.secrets.get(REFRESH_TOKEN)?.ok_or_else(|| {
            AppError::Message(
                "Wishtory is not signed in to Google on this PC. Sign in again.".into(),
            )
        })?;
        match parts.oauth.refresh(&refresh).await {
            Ok(access) => {
                let token = access.token.clone();
                *cached = Some(access);
                Ok(token)
            }
            Err(TokenError::Revoked) => {
                drop(cached);
                // Google no longer honours the sign-in: forget it here too, so the page offers to
                // sign in again instead of failing every time.
                let _ = self.forget_local(settings).await;
                Err(AppError::Message(
                    "Your Google sign-in has expired or was removed. Sign in again to keep backing up."
                        .into(),
                ))
            }
            Err(TokenError::Failed(e)) => Err(e),
        }
    }

    /// Runs one Drive call with a fresh token, retrying once if Drive says the token is stale.
    async fn call<T, F, Fut>(&self, settings: &SettingsStore, op: F) -> Result<T, AppError>
    where
        F: Fn(Zeroizing<String>) -> Fut,
        Fut: Future<Output = Result<T, DriveError>>,
    {
        for attempt in 0..2 {
            let token = self.access_token(settings).await?;
            match op(token).await {
                Ok(value) => return Ok(value),
                Err(DriveError::Unauthorized) if attempt == 0 => {
                    *self.access.lock().await = None;
                }
                Err(DriveError::Unauthorized) => break,
                Err(DriveError::Failed(e)) => return Err(e),
            }
        }
        Err(AppError::Message(
            "Google rejected the saved sign-in. Sign in again to keep backing up.".into(),
        ))
    }
}

impl Default for Cloud {
    fn default() -> Self {
        Self::new()
    }
}

fn already_running() -> AppError {
    AppError::Message(
        "Another cloud backup operation is already running. Wait for it to finish.".into(),
    )
}

fn require_signed_in(settings: &SettingsStore) -> Result<(), AppError> {
    if settings.cloud().account.is_some() {
        Ok(())
    } else {
        Err(AppError::Message("Sign in to Google first.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cloud::secrets::MemoryStore, cloud::testutil::FakeGoogle, models::Pull};
    use std::path::PathBuf;

    const PASS: &str = "correct horse battery";

    fn pull(id: &str, name: &str, time: &str) -> Pull {
        Pull {
            id: id.into(),
            game: "hk4e".into(),
            uid: "100000001".into(),
            gacha_type: "301".into(),
            gacha_id: String::new(),
            item_id: "10000089".into(),
            name: name.into(),
            item_type: "Character".into(),
            rank_type: 5,
            time: time.into(),
            lang: "en-us".into(),
            count: 1,
        }
    }

    struct Rig {
        cloud: Cloud,
        fake: FakeGoogle,
        secrets: Arc<MemoryStore>,
        db: Db,
        catalog: CatalogStore,
        banners: BannerStore,
        settings: SettingsStore,
        dir: PathBuf,
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("wishtory-cloud-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A signed-in rig whose Google is the fake. `tag` keeps scratch folders apart.
    async fn rig(tag: &str, fake: Option<FakeGoogle>) -> Rig {
        net::install_tls();
        let fake = match fake {
            Some(f) => f,
            None => FakeGoogle::start().await,
        };
        let http = reqwest::Client::new();
        let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
        let oauth = OAuth::new(
            http.clone(),
            Endpoints {
                auth: "https://accounts.google.com/o/oauth2/v2/auth",
                token: leak(format!("{}/token", fake.url)),
                revoke: leak(format!("{}/revoke", fake.url)),
            },
            Credentials {
                client_id: "id".into(),
                client_secret: "secret".into(),
            },
        );
        let drive = Drive::at(http, &fake.url);
        let secrets = Arc::new(MemoryStore::default());
        let cloud = Cloud::with(Some(Parts { oauth, drive }), secrets.clone());
        let dir = scratch(tag);
        let settings = SettingsStore::open(&dir);
        secrets.set(REFRESH_TOKEN, "RT").unwrap();
        secrets.set(PASSPHRASE, PASS).unwrap();
        settings
            .update_cloud(|c| c.account = Some("me@example.com".into()))
            .unwrap();
        Rig {
            cloud,
            fake,
            secrets,
            db: Db::open(&dir.join("wishtory.db")).unwrap(),
            catalog: CatalogStore::open(&dir),
            banners: BannerStore::open(&dir),
            settings,
            dir,
        }
    }

    fn seed(r: &Rig) {
        r.db.merge(
            "hk4e",
            "100000001",
            8,
            &[
                pull("1700000000000000002", "Furina", "2024-03-02 10:00:00"),
                pull("1700000000000000001", "Navia", "2024-03-01 10:00:00"),
            ],
        )
        .unwrap();
    }

    #[tokio::test]
    async fn a_backup_is_encrypted_and_restores_into_a_fresh_install() {
        let r = rig("roundtrip", None).await;
        seed(&r);
        let file = r
            .cloud
            .backup_now(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap();

        // What Google holds is opaque: no names, no UID.
        let stored = r.fake.state.lock().unwrap().files[0].data.clone();
        assert_eq!(&stored[..4], b"WSHB");
        for secret in ["Furina", "Navia", "100000001"] {
            assert!(
                !stored.windows(secret.len()).any(|w| w == secret.as_bytes()),
                "{secret}"
            );
        }
        assert_eq!(
            r.cloud.list_backups(&r.settings).await.unwrap(),
            vec![file.clone()]
        );
        assert!(r.settings.cloud().last_backup.is_some());

        // A second PC with the same Google account and passphrase.
        let other = rig(
            "roundtrip-2",
            Some(FakeGoogle {
                url: r.fake.url.clone(),
                state: r.fake.state.clone(),
            }),
        )
        .await;
        assert!(other.db.pulls_for("hk4e", "100000001").unwrap().is_empty());
        let summary = other
            .cloud
            .restore(
                &other.db,
                &other.catalog,
                &other.banners,
                &other.settings,
                file.id.clone(),
                None,
            )
            .await
            .unwrap();
        assert_eq!(summary.added, 2);
        let names: Vec<_> = other
            .db
            .pulls_for("hk4e", "100000001")
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Navia", "Furina"]);

        // Restoring again changes nothing.
        let again = other
            .cloud
            .restore(
                &other.db,
                &other.catalog,
                &other.banners,
                &other.settings,
                file.id,
                None,
            )
            .await
            .unwrap();
        assert_eq!(again.added, 0);
        let _ = (
            std::fs::remove_dir_all(&r.dir),
            std::fs::remove_dir_all(&other.dir),
        );
    }

    #[tokio::test]
    async fn only_the_newest_backups_are_kept() {
        let r = rig("prune", None).await;
        seed(&r);
        for _ in 0..KEEP_BACKUPS + 2 {
            r.cloud
                .backup_now(&r.db, &r.catalog, &r.settings)
                .await
                .unwrap();
        }
        let files = r.fake.state.lock().unwrap().files.clone();
        assert_eq!(files.len(), KEEP_BACKUPS);
        // The two oldest (ids 1 and 2) are gone.
        assert!(files.iter().all(|f| f.id.as_str() > "FILE000002ABCD"));
        let _ = std::fs::remove_dir_all(&r.dir);
    }

    #[tokio::test]
    async fn a_wrong_passphrase_cannot_restore() {
        let r = rig("wrongpass", None).await;
        seed(&r);
        let file = r
            .cloud
            .backup_now(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap();
        let err = r
            .cloud
            .restore(
                &r.db,
                &r.catalog,
                &r.banners,
                &r.settings,
                file.id.clone(),
                Some("not the passphrase".into()),
            )
            .await
            .err()
            .unwrap();
        assert!(err.to_string().contains("passphrase"), "{err}");
        // The saved passphrase still works.
        assert!(r
            .cloud
            .restore(&r.db, &r.catalog, &r.banners, &r.settings, file.id, None)
            .await
            .is_ok());
        let _ = std::fs::remove_dir_all(&r.dir);
    }

    #[tokio::test]
    async fn a_stale_token_is_refreshed_once_and_the_call_retried() {
        let r = rig("stale", None).await;
        seed(&r);
        r.fake.state.lock().unwrap().reject_next_drive_call = true;
        r.cloud
            .backup_now(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap();
        // One refresh for the first token, one after the 401.
        assert_eq!(r.fake.state.lock().unwrap().token_calls, 2);
        let _ = std::fs::remove_dir_all(&r.dir);
    }

    #[tokio::test]
    async fn a_revoked_sign_in_is_forgotten_instead_of_failing_forever() {
        let r = rig("revoked", None).await;
        seed(&r);
        r.fake.state.lock().unwrap().refresh_invalid = true;
        let err = r
            .cloud
            .backup_now(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("Sign in again"), "{err}");
        let status = r.cloud.status(&r.settings);
        assert!(!status.signed_in && !status.auto_backup);
        assert!(r.secrets.is_empty());
        let _ = std::fs::remove_dir_all(&r.dir);
    }

    #[tokio::test]
    async fn signing_out_revokes_and_can_delete_the_backups() {
        let r = rig("signout", None).await;
        seed(&r);
        r.cloud
            .backup_now(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap();
        r.cloud.set_auto_backup(&r.settings, true).unwrap();
        let status = r.cloud.sign_out(&r.settings, true).await.unwrap();
        assert!(!status.signed_in && !status.auto_backup && status.last_backup.is_none());
        {
            let state = r.fake.state.lock().unwrap();
            assert!(state.files.is_empty());
            assert_eq!(state.revoked, ["RT"]);
        }
        assert!(r.secrets.is_empty());

        // Signing out without deleting leaves the backups where they are.
        let keep = rig("signout-keep", None).await;
        seed(&keep);
        keep.cloud
            .backup_now(&keep.db, &keep.catalog, &keep.settings)
            .await
            .unwrap();
        keep.cloud.sign_out(&keep.settings, false).await.unwrap();
        assert_eq!(keep.fake.state.lock().unwrap().files.len(), 1);
        assert!(keep.secrets.is_empty());
        let _ = (
            std::fs::remove_dir_all(&r.dir),
            std::fs::remove_dir_all(&keep.dir),
        );
    }

    #[tokio::test]
    async fn the_daily_backup_is_opt_in_and_runs_once_a_day() {
        let r = rig("auto", None).await;
        seed(&r);
        // Off by default.
        assert!(r
            .cloud
            .backup_if_due(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap()
            .is_none());
        r.cloud.set_auto_backup(&r.settings, true).unwrap();
        assert!(r
            .cloud
            .backup_if_due(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap()
            .is_some());
        assert!(r
            .cloud
            .backup_if_due(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap()
            .is_none());
        // A day later it is due again.
        r.settings
            .update_cloud(|c| {
                c.last_backup =
                    Some((chrono::Utc::now() - chrono::Duration::hours(25)).to_rfc3339())
            })
            .unwrap();
        assert!(r
            .cloud
            .backup_if_due(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap()
            .is_some());
        let _ = std::fs::remove_dir_all(&r.dir);
    }

    #[tokio::test]
    async fn with_no_history_there_is_nothing_to_back_up() {
        let r = rig("empty", None).await;
        let err = r
            .cloud
            .backup_now(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no wish history"), "{err}");
        r.cloud.set_auto_backup(&r.settings, true).unwrap();
        assert!(r
            .cloud
            .backup_if_due(&r.db, &r.catalog, &r.settings)
            .await
            .unwrap()
            .is_none());
        assert!(r.fake.state.lock().unwrap().files.is_empty());
        let _ = std::fs::remove_dir_all(&r.dir);
    }

    #[tokio::test]
    async fn without_a_google_client_id_or_a_sign_in_nothing_runs() {
        let dir = scratch("unavailable");
        let settings = SettingsStore::open(&dir);
        let db = Db::open(&dir.join("wishtory.db")).unwrap();
        let catalog = CatalogStore::open(&dir);
        let cloud = Cloud::with(None, Arc::new(MemoryStore::default()));
        let status = cloud.status(&settings);
        assert!(!status.available && !status.signed_in);
        assert!(cloud.sign_in(&settings, PASS.into()).await.is_err());
        assert!(cloud.backup_now(&db, &catalog, &settings).await.is_err());
        assert!(cloud
            .backup_if_due(&db, &catalog, &settings)
            .await
            .unwrap()
            .is_none());
        assert!(cloud.set_auto_backup(&settings, true).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn a_weak_passphrase_is_refused_before_the_browser_opens() {
        let r = rig("weak", None).await;
        let err = r
            .cloud
            .sign_in(&r.settings, "short".into())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("at least"), "{err}");
        let _ = std::fs::remove_dir_all(&r.dir);
    }
}
