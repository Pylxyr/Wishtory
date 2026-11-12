//! The operations the app offers, independent of any UI toolkit. Game files are only touched
//! (read-only) via discover.rs / authkey.rs.

use crate::{
    authkey,
    catalog::{
        BannerEntry, BannerSet, BannerStore, Catalog, CatalogStore, ChangeEntry, CheckOutcome,
        Components, HistoryEntry, HttpSource, ReleaseStore, Source, StagedRelease,
    },
    db::Db,
    discover,
    error::AppError,
    fetch,
    impact::{self, AccountImpact, AffectedPull, Data, Impact, MAX_LISTED},
    logging::Level,
    models::{Account, Game, GameInstall, GameLocations, Pull},
    pity::{self, BannerPity},
    rules::RulesStore,
    settings::SettingsStore,
    uigf::{self, ImportSummary},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::task::{JoinError, spawn_blocking};

/// Import files above this size are refused (a full multi-game UIGF file is a few MB at most).
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
/// How many cached history links are tried before giving up (each failed try is one cheap request).
const MAX_LINK_TRIES: usize = 4;
static EXPORT_SEQ: AtomicU64 = AtomicU64::new(0);

fn join_err(e: JoinError) -> AppError {
    AppError::Io(e.to_string())
}

pub async fn discover_games() -> Result<Vec<GameInstall>, AppError> {
    spawn_blocking(discover::discover_all)
        .await
        .map_err(join_err)
}

/// Where games are searched for: the extra folders the person added and, on Linux, the Wine and
/// Proton prefixes found automatically. `supported` is false on Windows.
async fn game_locations_view(settings: &SettingsStore) -> Result<GameLocations, AppError> {
    let locations = settings.game_locations();
    let detected = spawn_blocking(discover::detect_prefixes)
        .await
        .map_err(join_err)?;
    Ok(GameLocations {
        supported: cfg!(unix),
        locations,
        detected,
    })
}

pub async fn get_game_locations(settings: &SettingsStore) -> Result<GameLocations, AppError> {
    game_locations_view(settings).await
}

pub async fn set_game_locations(
    settings: &SettingsStore,
    locations: Vec<String>,
) -> Result<GameLocations, AppError> {
    settings.set_game_locations(locations)?;
    game_locations_view(settings).await
}

/// Returns the most likely history URL found in the game's web cache (read-only).
pub async fn extract_authkey(game: Game) -> Result<String, AppError> {
    spawn_blocking(move || authkey::extract(game))
        .await
        .map_err(join_err)?
}

#[derive(Serialize)]
pub struct FetchSummary {
    pub game: String,
    pub uid: String,
    pub fetched: usize,
    pub added: usize,
    /// Present when the download stopped early; the saved part is still usable.
    pub warning: Option<String>,
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
        if let Some(game) = parse_game(&account.game) {
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
pub async fn fetch_history(
    progress: &(impl Fn(String) + Send + Sync),
    db: &Db,
    banners: &BannerStore,
    game: Game,
    url: String,
    full: Option<bool>,
) -> Result<FetchSummary, AppError> {
    progress("Checking your link…".into());
    wlog!(
        Level::Debug,
        "{}: fetching with a pasted link",
        game.as_str()
    );
    let known = known_ids(db, game, full.unwrap_or(false));
    let f = fetch::fetch_all(game, &url, progress, &known)
        .await // network first; never hold the DB lock across await
        .inspect_err(|e| wlog!(Level::Warn, "{}: fetch failed: {e}", game.as_str()))?;
    progress(format!("Saving {} pulls to this PC…", f.pulls.len()));
    store(db, banners.current(), game, f).await
}

/// One-click flow: read the cache (read-only), then try the cached links newest first until one
/// is still valid. Handles a stale link sitting behind a fresh one in the cache file.
pub async fn refresh_from_game(
    progress: &(impl Fn(String) + Send + Sync),
    db: &Db,
    banners: &BannerStore,
    game: Game,
    full: Option<bool>,
) -> Result<FetchSummary, AppError> {
    let known = known_ids(db, game, full.unwrap_or(false));
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
        match fetch::fetch_all(game, &url, progress, &known).await {
            Ok(f) => {
                wlog!(Level::Debug, "{}: link {} accepted", game.as_str(), i + 1);
                progress(format!("Saving {} pulls to this PC…", f.pulls.len()));
                return store(db, banners.current(), game, f).await;
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

pub async fn list_accounts(db: &Db) -> Result<Vec<Account>, AppError> {
    db.run(|db| db.list_accounts()).await
}

pub async fn delete_account(db: &Db, game: Game, uid: String) -> Result<(), AppError> {
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

pub async fn clear_all_history(db: &Db) -> Result<usize, AppError> {
    db.run(|db| {
        db.clear_all_history()
            .inspect(|n| wlog!(Level::Info, "cleared all saved history ({n} pulls)"))
    })
    .await
}

pub async fn get_catalog(catalog: &CatalogStore) -> Result<Arc<Catalog>, AppError> {
    Ok(catalog.current())
}

pub async fn get_banners(banners: &BannerStore) -> Result<Arc<BannerSet>, AppError> {
    Ok(banners.current())
}

// ---------------------------------------------------------------------------------------------
// Game catalog releases: check, review, apply
// ---------------------------------------------------------------------------------------------

/// Cheap to clone; everything a catalog operation needs. `auto_apply` is the person's setting
/// as it was when the operation started.
#[derive(Clone)]
pub struct CatalogCtx {
    pub db: Db,
    pub release: Arc<ReleaseStore>,
    pub catalog: Arc<CatalogStore>,
    pub banners: Arc<BannerStore>,
    pub rules: Arc<RulesStore>,
    pub auto_apply: bool,
}

/// A release that is downloaded and verified but not applied.
#[derive(Debug, Clone)]
pub struct WaitingRelease {
    pub manifest_version: u64,
    pub generated_at: String,
    pub major: u32,
    pub minor: u32,
    pub info: u32,
    /// "Not now" was chosen: no start-up notice, but the Settings badge stays.
    pub dismissed: bool,
    pub affected_pulls: u64,
    pub affected_accounts: u32,
}

#[derive(Debug, Clone)]
pub struct CatalogStatus {
    /// The one catalog version.
    pub version: u64,
    pub components: Components,
    pub generated_at: String,
    /// Unix seconds of the last check.
    pub last_check: Option<u64>,
    pub auto_apply: bool,
    pub waiting: Option<WaitingRelease>,
}

#[derive(Debug, Clone)]
pub struct CatalogCheck {
    /// "up-to-date", "already-checking", "not-ready", "applied" or "needs-review".
    pub status: String,
    /// Set for "not-ready": which part is not published yet.
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CatalogReview {
    pub manifest_version: u64,
    pub current_version: u64,
    pub generated_at: String,
    pub changes: Vec<ChangeEntry>,
    /// Pass it back to `apply_catalog`: it says which analysis the person looked at.
    pub fingerprint: String,
    pub accounts: Vec<AccountImpact>,
    pub total_affected: u64,
    pub not_listed: u64,
    pub dismissed: bool,
}

#[derive(Debug, Clone)]
pub struct ApplyResult {
    pub applied: bool,
    /// The saved pulls or the waiting release changed since the review; nothing was applied and
    /// a fresh review is ready.
    pub stale: bool,
}

fn parse_game(text: &str) -> Option<Game> {
    Game::ALL.iter().copied().find(|g| g.as_str() == text)
}

/// The impact of the staged release on the saved pulls, reused while nothing has changed.
async fn impact_for(
    ctx: &CatalogCtx,
    staged: &Arc<StagedRelease>,
) -> Result<Arc<Impact>, AppError> {
    let current = ctx.release.current().manifest_version;
    let staged_version = staged.manifest.manifest_version;
    let fingerprint = ctx
        .db
        .run(move |db| impact::fingerprint(db, current, staged_version))
        .await?;
    let memo = ctx
        .release
        .impact
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(memo) = memo.filter(|m| m.fingerprint == fingerprint) {
        return Ok(memo);
    }
    let (catalog, banners, rules) = (
        ctx.catalog.current(),
        ctx.banners.current(),
        ctx.rules.current(),
    );
    let staged = staged.clone();
    let computed = ctx
        .db
        .run(move |db| {
            impact::compute(
                db,
                Data {
                    version: current,
                    catalog: &catalog,
                    banners: &banners,
                    rules: &rules,
                },
                Data {
                    version: staged.manifest.manifest_version,
                    catalog: &staged.catalog,
                    banners: &staged.banners,
                    rules: &staged.rules,
                },
                &staged.manifest.changes,
                MAX_LISTED,
            )
        })
        .await?;
    let computed = Arc::new(computed);
    *ctx.release.impact.lock().unwrap_or_else(|e| e.into_inner()) = Some(computed.clone());
    Ok(computed)
}

pub async fn get_catalog_status(ctx: &CatalogCtx) -> Result<CatalogStatus, AppError> {
    let current = ctx.release.current();
    let waiting = match ctx.release.staged() {
        Some(staged) => {
            let impact = impact_for(ctx, &staged).await?;
            let (major, minor, info) = staged.manifest.counts();
            Some(WaitingRelease {
                manifest_version: staged.manifest.manifest_version,
                generated_at: staged.manifest.generated_at.clone(),
                major,
                minor,
                info,
                dismissed: ctx.release.dismissed() == Some(staged.manifest.manifest_version),
                affected_pulls: impact.total_affected,
                affected_accounts: impact.affected_accounts(),
            })
        }
        None => None,
    };
    Ok(CatalogStatus {
        version: current.manifest_version,
        components: current.components,
        generated_at: current.generated_at.clone(),
        last_check: ctx.release.last_check(),
        auto_apply: ctx.auto_apply,
        waiting,
    })
}

/// Asks for a newer release. A complete one is downloaded and verified but not installed. When
/// it would change no saved pull and the setting allows, it is applied; otherwise it waits for
/// the person. `force` skips the once-a-day gate (the Check for updates button).
pub async fn check_catalog(ctx: &CatalogCtx, force: bool) -> Result<CatalogCheck, AppError> {
    check_catalog_with(ctx, &HttpSource, force).await
}

pub(crate) async fn check_catalog_with(
    ctx: &CatalogCtx,
    source: &impl Source,
    force: bool,
) -> Result<CatalogCheck, AppError> {
    let outcome = ctx
        .release
        .check(source, force)
        .await
        .inspect_err(|e| wlog!(Level::Warn, "catalog check failed: {e}"))
        .map_err(AppError::Message)?;
    wlog!(Level::Debug, "catalog check: {}", outcome.status());
    let mut check = CatalogCheck {
        status: outcome.status().into(),
        message: None,
    };
    match &outcome {
        CheckOutcome::Busy => return Ok(check),
        CheckOutcome::NotReady(why) => {
            wlog!(Level::Info, "catalog release not ready: {why}");
            check.message = Some(why.clone());
        }
        _ => {}
    }
    // Whatever the check found, a release staged earlier (even before a restart) is looked at.
    if let Some(staged) = ctx.release.staged() {
        let impact = impact_for(ctx, &staged).await?;
        if impact.total_affected == 0 && ctx.auto_apply {
            apply_staged(ctx, &staged, true, 0).await?;
            check.status = "applied".into();
        } else {
            check.status = "needs-review".into();
        }
    }
    Ok(check)
}

/// Installs the staged release and re-derives every saved pull's banner in ONE database
/// transaction. If any step fails the previous release stays in use.
async fn apply_staged(
    ctx: &CatalogCtx,
    staged: &Arc<StagedRelease>,
    automatic: bool,
    affected: u64,
) -> Result<(), AppError> {
    let (c, st) = (ctx.clone(), staged.clone());
    c.db.clone()
        .run(move |db| {
            let mut rows = Vec::new();
            for account in db.list_accounts()? {
                let Some(game) = parse_game(&account.game) else {
                    continue;
                };
                let pulls = db.pulls_for(&account.game, &account.uid)?;
                let placed = pity::resolve_banners(&st.banners.banners, game, &pulls);
                rows.extend(pulls.iter().zip(placed).filter_map(|(p, b)| {
                    b.map(|b| (p.game.clone(), p.uid.clone(), p.id.clone(), b.id.clone()))
                }));
            }
            let marker = format!("banners:{}", st.banners.banners_version);
            let mut installed = None;
            let result = db.replace_pull_banners(&rows, &marker, || {
                installed = Some(
                    c.release
                        .install(&st, &c.catalog, &c.banners, &c.rules)
                        .map_err(AppError::Message)?,
                );
                Ok(())
            });
            if result.is_err()
                && let Some(applied) = installed.take()
            {
                c.release.revert(applied, &c.catalog, &c.banners, &c.rules);
            }
            result
        })
        .await
        .inspect_err(|e| wlog!(Level::Warn, "applying the catalog release failed: {e}"))?;
    wlog!(
        Level::Info,
        "applied catalog release {}",
        staged.manifest.manifest_version
    );
    ctx.release.finish(staged, automatic, affected);
    Ok(())
}

/// The change log and the affected pulls of the waiting release, for the review page.
pub async fn get_catalog_review(ctx: &CatalogCtx) -> Result<Option<CatalogReview>, AppError> {
    let Some(staged) = ctx.release.staged() else {
        return Ok(None);
    };
    let impact = impact_for(ctx, &staged).await?;
    let mut changes = staged.manifest.changes.clone();
    changes.sort_by(|a, b| a.severity.cmp(&b.severity).then_with(|| a.id.cmp(&b.id)));
    Ok(Some(CatalogReview {
        manifest_version: staged.manifest.manifest_version,
        current_version: impact.current_version,
        generated_at: staged.manifest.generated_at.clone(),
        changes,
        fingerprint: impact.fingerprint.clone(),
        accounts: impact.accounts.clone(),
        total_affected: impact.total_affected,
        not_listed: impact.not_listed,
        dismissed: ctx.release.dismissed() == Some(staged.manifest.manifest_version),
    }))
}

/// One page of the affected pulls of one account.
pub async fn get_impact_page(
    ctx: &CatalogCtx,
    game: Game,
    uid: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<AffectedPull>, AppError> {
    let Some(staged) = ctx.release.staged() else {
        return Ok(Vec::new());
    };
    let impact = impact_for(ctx, &staged).await?;
    Ok(impact.page(game.as_str(), &uid, offset, limit.min(200)))
}

/// Applies the waiting release after the person confirmed. `reviewed` is the version and
/// fingerprint of the review they saw: when the saved pulls or the waiting release changed since,
/// nothing is applied and `stale` is set.
pub async fn apply_catalog(
    ctx: &CatalogCtx,
    reviewed: Option<(u64, String)>,
) -> Result<ApplyResult, AppError> {
    let staged = ctx
        .release
        .staged()
        .ok_or_else(|| AppError::Message("There is no game data update waiting.".into()))?;
    let impact = impact_for(ctx, &staged).await?;
    if let Some((version, fingerprint)) = reviewed
        && (version != staged.manifest.manifest_version || fingerprint != impact.fingerprint)
    {
        return Ok(ApplyResult {
            applied: false,
            stale: true,
        });
    }
    apply_staged(ctx, &staged, false, impact.total_affected).await?;
    Ok(ApplyResult {
        applied: true,
        stale: false,
    })
}

/// "Not now": keeps the release staged and stops the start-up notice for it.
pub async fn dismiss_catalog(ctx: &CatalogCtx) -> Result<(), AppError> {
    if let Some(staged) = ctx.release.staged() {
        ctx.release.dismiss(staged.manifest.manifest_version);
    }
    Ok(())
}

pub async fn get_catalog_history(ctx: &CatalogCtx) -> Result<Vec<HistoryEntry>, AppError> {
    Ok(ctx.release.history())
}

/// Static, per-game UI naming that the backend already owns (so the frontend keeps no copy).
#[derive(Serialize)]
pub struct GameMeta {
    pub game: &'static str,
    /// History-page banner filter: `(key, label)` in display order.
    pub banners: Vec<BannerOption>,
}

#[derive(Serialize)]
pub struct BannerOption {
    pub key: &'static str,
    pub label: &'static str,
}

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
    pub pull: Pull,
    /// The banner event the pull was made on, e.g. "Ballad in Goblets · Venti"; `None` when the
    /// banner list cannot place it.
    pub banner_name: Option<String>,
    /// Pulls since the last 5★ (5★ rows) or last 4★/5★ (4★ rows) when this pull was made.
    pub pity: Option<u32>,
    /// 3 / 4 / 5 for every game (Zenless reports B / A / S as 2 / 3 / 4 in `rank_type`).
    pub rank: u8,
    /// Short pool name for the table, e.g. "Character" or "W-Engine".
    pub pool: String,
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
pub async fn get_history(
    db: &Db,
    catalog: &CatalogStore,
    banners: &BannerStore,
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

pub async fn get_pity(
    db: &Db,
    banners: &BannerStore,
    rules: &RulesStore,
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
pub async fn import_file(
    db: &Db,
    catalog: &CatalogStore,
    banners: &BannerStore,
    path: String,
) -> Result<ImportSummary, AppError> {
    let (catalog, banners) = (catalog.current(), banners.current());
    // File read, JSON parse and database merge can take a while: keep them off the async runtime.
    db.run(move |db| {
        let text = decode_text(&read_import_file(&path, MAX_IMPORT_BYTES)?)?;
        merge_import(db, &text, &catalog, &banners)
    })
    .await
}

/// Imports UIGF text that is already in memory (a backup restored from the cloud).
pub async fn import_text(
    db: &Db,
    catalog: &CatalogStore,
    banners: &BannerStore,
    text: String,
) -> Result<ImportSummary, AppError> {
    let (catalog, banners) = (catalog.current(), banners.current());
    db.run(move |db| merge_import(db, &text, &catalog, &banners))
        .await
}

fn merge_import(
    db: &Db,
    text: &str,
    catalog: &Catalog,
    banners: &BannerSet,
) -> Result<ImportSummary, AppError> {
    let summary = uigf::import_with_catalog(db, text, catalog)
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
    if let Err(e) = assign_all_banners(db, banners) {
        wlog!(Level::Warn, "banner assignment failed: {e}");
    }
    Ok(summary)
}

pub async fn export_file(
    db: &Db,
    catalog: &CatalogStore,
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
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
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
        assert!(
            ensure_not_game_dir(
                "D:\\Games\\Other\\..\\Genshin Impact Game\\out.json",
                &roots
            )
            .is_err()
        );
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

#[cfg(test)]
mod catalog_tests {
    use super::*;
    use crate::catalog::testkit::*;
    use crate::models::Pull;
    use crate::rules::RuleSet;

    fn pull(id: u32, time: &str) -> Pull {
        Pull {
            id: id.to_string(),
            game: "hk4e".into(),
            uid: "1".into(),
            gacha_type: "301".into(),
            gacha_id: String::new(),
            item_id: "x".into(),
            name: "Item".into(),
            item_type: "Character".into(),
            rank_type: 3,
            time: time.into(),
            lang: "en".into(),
            count: 1,
        }
    }

    fn banners(version: u64, end: &str) -> BannerSet {
        let text = format!(
            r#"{{"schema_version":1,"banners_version":{version},"generated_at":"2026-01-01T00:00:00Z","banners":[{{"id":"e1","game":"hk4e","pool":"character","name":"Event","start":"2026-01-01 18:00:00","end":"{end}","featured_5":["10000022"]}}]}}"#
        );
        BannerSet::parse_and_validate(text.as_bytes()).unwrap()
    }

    struct World {
        ctx: CatalogCtx,
        dir: std::path::PathBuf,
        source: Fake,
    }

    impl World {
        fn new(tag: &str, auto_apply: bool) -> World {
            let dir = scratch(&format!("ops-{tag}"));
            let feeds = open_with_banners(&dir, banners(1, "2026-01-20 17:59:59"));
            let db = Db::open(&dir.join("wishtory.db")).unwrap();
            db.merge(
                "hk4e",
                "1",
                8,
                &[
                    pull(1, "2026-01-05 12:00:00"),
                    pull(2, "2026-01-25 12:00:00"),
                ],
            )
            .unwrap();
            let ctx = CatalogCtx {
                db,
                release: Arc::new(feeds.release),
                catalog: Arc::new(feeds.catalog),
                banners: Arc::new(feeds.banners),
                rules: Arc::new(feeds.rules),
                auto_apply,
            };
            World {
                ctx,
                dir,
                source: Fake::default(),
            }
        }

        /// Release 2 with a banner list ending at `end` (pull 2, on 25 January, is inside only
        /// when the window was stretched).
        fn publish(&self, end: &str) {
            let mut c = Catalog::bundled();
            c.catalog_version = 5;
            let mut r = RuleSet::bundled();
            r.rules_version = 3;
            self.source.publish(&envelopes_of(2, c, banners(6, end), r));
        }

        fn check(&self, force: bool) -> CatalogCheck {
            run(check_catalog_with(&self.ctx, &self.source, force)).unwrap()
        }

        fn versions(&self) -> (u64, u64, u64, u64) {
            (
                self.ctx.release.current().manifest_version,
                self.ctx.catalog.current().catalog_version,
                self.ctx.banners.current().banners_version,
                self.ctx.rules.current().rules_version,
            )
        }

        fn pulls(&self) -> Vec<String> {
            self.ctx
                .db
                .pulls_for("hk4e", "1")
                .unwrap()
                .iter()
                .map(|p| format!("{p:?}"))
                .collect()
        }
    }

    impl Drop for World {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn a_release_that_changes_no_pull_is_applied_by_itself_and_recorded() {
        let w = World::new("auto", true);
        let before = w.pulls();
        w.publish("2026-01-20 17:59:59"); // same dates, new versions
        assert_eq!(w.check(true).status, "applied");
        assert_eq!(w.versions(), (2, 5, 6, 3));
        assert!(w.ctx.release.staged().is_none());
        let history = run(get_catalog_history(&w.ctx)).unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].automatic);
        assert_eq!(history[0].affected_pulls, 0);
        assert_eq!(w.pulls(), before, "saved pulls are never rewritten");
        assert_eq!(
            w.ctx
                .db
                .pull_banners_for("hk4e", "1")
                .unwrap()
                .get("1")
                .map(String::as_str),
            Some("e1")
        );
        let status = run(get_catalog_status(&w.ctx)).unwrap();
        assert_eq!((status.version, status.waiting.is_none()), (2, true));
    }

    #[test]
    fn the_setting_can_turn_automatic_updates_off() {
        let w = World::new("manual", false);
        w.publish("2026-01-20 17:59:59");
        assert_eq!(w.check(true).status, "needs-review");
        assert_eq!(w.versions(), (1, 1, 1, 1));
        let status = run(get_catalog_status(&w.ctx)).unwrap();
        let waiting = status.waiting.unwrap();
        assert_eq!((waiting.manifest_version, waiting.affected_pulls), (2, 0));
        assert!(run(apply_catalog(&w.ctx, None)).unwrap().applied);
        assert_eq!(w.versions(), (2, 5, 6, 3));
    }

    #[test]
    fn a_release_that_would_change_a_pull_waits_and_nothing_is_applied_silently() {
        let w = World::new("review", true);
        let before = w.pulls();
        w.publish("2026-01-31 17:59:59"); // stretches the banner over pull 2
        assert_eq!(w.check(true).status, "needs-review");
        assert_eq!(w.versions(), (1, 1, 1, 1));
        // Even the next, unforced check (the gate holds) never applies it.
        assert_eq!(w.check(false).status, "needs-review");
        assert_eq!(w.versions(), (1, 1, 1, 1));
        let review = run(get_catalog_review(&w.ctx)).unwrap().unwrap();
        assert_eq!(review.total_affected, 1);
        assert_eq!(review.accounts[0].affected, 1);
        let page = run(get_impact_page(&w.ctx, Game::Hk4e, "1".into(), 0, 50)).unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].pull_id, "2");
        assert_eq!(page[0].banner_before, None);
        assert_eq!(page[0].banner_after.as_deref(), Some("Event"));
        assert_eq!(w.pulls(), before);
        // "Not now" keeps it waiting.
        run(dismiss_catalog(&w.ctx)).unwrap();
        let status = run(get_catalog_status(&w.ctx)).unwrap();
        assert!(status.waiting.as_ref().unwrap().dismissed);
        assert_eq!(w.versions(), (1, 1, 1, 1));
        // Apply with the review the person saw.
        let result = run(apply_catalog(
            &w.ctx,
            Some((review.manifest_version, review.fingerprint)),
        ))
        .unwrap();
        assert!(result.applied && !result.stale);
        assert_eq!(w.versions(), (2, 5, 6, 3));
        let rows = w.ctx.db.pull_banners_for("hk4e", "1").unwrap();
        assert_eq!(rows.get("2").map(String::as_str), Some("e1"));
        assert_eq!(w.pulls(), before);
        let history = run(get_catalog_history(&w.ctx)).unwrap();
        assert!(!history[0].automatic);
        assert_eq!(history[0].affected_pulls, 1);
    }

    #[test]
    fn if_the_pulls_changed_after_the_review_the_impact_is_recomputed_first() {
        let w = World::new("stale", true);
        w.publish("2026-01-31 17:59:59");
        w.check(true);
        let review = run(get_catalog_review(&w.ctx)).unwrap().unwrap();
        // A sync adds a pull the review did not include.
        w.ctx
            .db
            .merge("hk4e", "1", 8, &[pull(3, "2026-01-26 12:00:00")])
            .unwrap();
        let result = run(apply_catalog(
            &w.ctx,
            Some((review.manifest_version, review.fingerprint.clone())),
        ))
        .unwrap();
        assert!(result.stale && !result.applied);
        assert_eq!(w.versions(), (1, 1, 1, 1));
        let fresh = run(get_catalog_review(&w.ctx)).unwrap().unwrap();
        assert_eq!(fresh.total_affected, 2);
        assert_ne!(fresh.fingerprint, review.fingerprint);
        assert!(
            run(apply_catalog(
                &w.ctx,
                Some((fresh.manifest_version, fresh.fingerprint))
            ))
            .unwrap()
            .applied
        );
    }

    #[test]
    fn a_failed_apply_leaves_the_previous_release_and_the_saved_banners() {
        let w = World::new("fail", true);
        w.publish("2026-01-31 17:59:59");
        w.check(true);
        // Seed the stored banners as the previous release left them.
        run(async {
            w.ctx
                .db
                .clone()
                .run(|db| db.set_pull_banners("hk4e", "1", &[("1".into(), "e1".into())]))
                .await
        })
        .unwrap();
        // Keeping the banner list on disk fails (its cache folder is a file).
        std::fs::write(w.dir.join("banners-cache"), b"in the way").unwrap();
        assert!(run(apply_catalog(&w.ctx, None)).is_err());
        assert_eq!(w.versions(), (1, 1, 1, 1));
        let rows = w.ctx.db.pull_banners_for("hk4e", "1").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows.get("1").map(String::as_str), Some("e1"));
        assert!(
            w.ctx.release.staged().is_some(),
            "still waiting, nothing lost"
        );
        assert_eq!(w.pulls().len(), 2);
        assert_eq!(run(get_catalog_history(&w.ctx)).unwrap().len(), 0);
        // After the obstacle goes away the same release applies.
        std::fs::remove_file(w.dir.join("banners-cache")).unwrap();
        assert!(run(apply_catalog(&w.ctx, None)).unwrap().applied);
    }

    #[test]
    fn a_staged_release_left_by_a_crash_is_picked_up_without_a_download() {
        let w = World::new("resume", true);
        w.publish("2026-01-20 17:59:59");
        // Stage only (as if the app stopped right after downloading).
        run(w.ctx.release.check(&w.source, true)).unwrap();
        assert_eq!(w.versions(), (1, 1, 1, 1));
        w.source
            .offline
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let check = w.check(false); // gate says not due; nothing is downloaded
        assert_eq!(check.status, "applied");
        assert_eq!(w.versions(), (2, 5, 6, 3));
    }

    #[test]
    fn saved_banners_are_forgotten_when_they_belong_to_another_banner_list() {
        let w = World::new("marker", true);
        w.ctx.db.ensure_derived("banners:1").unwrap();
        w.ctx
            .db
            .set_pull_banners("hk4e", "1", &[("1".into(), "e1".into())])
            .unwrap();
        w.ctx.db.ensure_derived("banners:1").unwrap();
        assert_eq!(w.ctx.db.pull_banners_for("hk4e", "1").unwrap().len(), 1);
        w.ctx.db.ensure_derived("banners:2").unwrap();
        assert!(w.ctx.db.pull_banners_for("hk4e", "1").unwrap().is_empty());
        assert_eq!(w.pulls().len(), 2);
    }
}
