//! Plain data crossing the bridge. Conversions from the `wishtory-core` types live in `convert.rs`,
//! outside `api/`, so the code generator only ever sees these definitions and the public API.
//! Counters that are `u64`/`usize` in the core are `i64` here, which Dart reads as a plain `int`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameId {
    /// Genshin Impact (`hk4e` in saved records)
    Genshin,
    /// Honkai: Star Rail (`hkrpg`)
    StarRail,
    /// Zenless Zone Zero (`nap`)
    Zenless,
}

#[derive(Debug, Clone)]
pub struct GameInstall {
    pub game: GameId,
    pub found: bool,
    pub log_path: Option<String>,
    pub data_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DetectedPrefix {
    pub path: String,
    pub games: Vec<GameId>,
}

#[derive(Debug, Clone)]
pub struct GameLocations {
    /// False on Windows, where nothing needs configuring.
    pub supported: bool,
    pub locations: Vec<String>,
    pub detected: Vec<DetectedPrefix>,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub game: GameId,
    pub uid: String,
    pub nickname: String,
    /// Server UTC offset in hours.
    pub timezone: i32,
    pub pull_count: i64,
}

/// One saved pull with everything the History table needs, newest first from the backend.
#[derive(Debug, Clone)]
pub struct HistoryPull {
    pub id: String,
    pub gacha_type: String,
    pub gacha_id: String,
    pub item_id: String,
    pub name: String,
    pub item_type: String,
    /// `YYYY-MM-DD HH:MM:SS`, the account's server-local time.
    pub time: String,
    /// 3 / 4 / 5 for every game (Zenless B / A / S are mapped).
    pub rank: u8,
    /// Short pool name for the table, e.g. "Character".
    pub pool: String,
    /// Key of the History banner filter this pull belongs to (see `GameMeta`); empty if none.
    pub banner_key: String,
    /// "Event name · Featured A / Featured B"; `None` when the banner list cannot place the pull.
    pub banner_name: Option<String>,
    /// Pulls since the last 5★ (5★ rows) or last 4★/5★ (4★ rows) when this pull was made.
    pub pity: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullOutcome {
    Win,
    Loss,
    Guaranteed,
}

#[derive(Debug, Clone)]
pub struct FiveStar {
    pub name: String,
    pub pity: u32,
    pub time: String,
    pub off_banner: Option<bool>,
    pub outcome: Option<PullOutcome>,
}

#[derive(Debug, Clone)]
pub struct FourStar {
    pub name: String,
    pub pity: u32,
    pub time: String,
}

/// Pity state of one banner of one account. The numbers come from the Rust pity engine.
#[derive(Debug, Clone)]
pub struct BannerPity {
    pub banner: String,
    pub label: String,
    pub pity: u32,
    pub hard_pity: u32,
    pub soft_pity: Option<u32>,
    /// `Some(true)`: next 5★ is guaranteed featured. `None` when the banner tracks no result.
    pub guaranteed: Option<bool>,
    /// Percent chance a non-guaranteed 5★ is featured (50 / 75); `None` when no result is tracked.
    pub guarantee_odds: Option<u8>,
    pub total: u32,
    pub five_stars: u32,
    pub four_stars: u32,
    pub avg_five_pity: Option<f32>,
    pub avg_four_pity: Option<f32>,
    /// Oldest first.
    pub recent_fives: Vec<FiveStar>,
    /// Oldest first.
    pub recent_fours: Vec<FourStar>,
}

#[derive(Debug, Clone)]
pub struct BannerFilter {
    pub key: String,
    pub label: String,
}

/// Static per-game naming the backend owns: the History banner filter, in display order.
#[derive(Debug, Clone)]
pub struct GameMeta {
    pub game: GameId,
    pub banners: Vec<BannerFilter>,
}

#[derive(Debug, Clone)]
pub struct CatalogItem {
    pub game: String,
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub rarity: u8,
    pub kind: String,
    pub paimon_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GameCatalog {
    pub catalog_version: i64,
    pub generated_at: String,
    pub items: Vec<CatalogItem>,
}

#[derive(Debug, Clone)]
pub struct BannerEntry {
    pub id: String,
    pub game: GameId,
    pub pool: String,
    pub name: String,
    pub gacha_ids: Vec<String>,
    /// Server-local `YYYY-MM-DD HH:MM:SS`, inclusive.
    pub start: String,
    pub end: String,
    pub featured_5: Vec<String>,
    pub featured_4: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BannerList {
    pub banners_version: i64,
    pub generated_at: String,
    pub banners: Vec<BannerEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateStatus {
    Updated,
    UpToDate,
    AlreadyChecking,
}

#[derive(Debug, Clone)]
pub struct BannerUpdate {
    pub status: UpdateStatus,
    pub banners_version: i64,
    /// Unix seconds; 0 when nothing was installed.
    pub installed_at: i64,
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FetchSummary {
    pub game: GameId,
    pub uid: String,
    pub fetched: i64,
    pub added: i64,
    /// Present when the download stopped early; the saved part is still usable.
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub format: String,
    pub accounts: i64,
    pub added: i64,
    pub skipped: i64,
}

/// What the Settings page shows about cloud backup (Google Drive).
#[derive(Debug, Clone)]
pub struct CloudStatus {
    /// False in builds made without a Google client ID: the section explains instead of offering it.
    pub available: bool,
    pub signed_in: bool,
    /// The connected Google account, for display.
    pub account: Option<String>,
    pub auto_backup: bool,
    /// When the last backup finished, RFC 3339.
    pub last_backup: Option<String>,
}

/// One backup file in the person's Drive.
#[derive(Debug, Clone)]
pub struct CloudBackup {
    pub id: String,
    /// When Drive stored it, RFC 3339.
    pub created: String,
    pub size: i64,
}

#[derive(Debug, Clone)]
pub struct Credit {
    pub name: String,
    pub role: String,
    pub url: String,
    pub license: String,
}

#[derive(Debug, Clone)]
pub struct CreditGroup {
    pub title: String,
    pub note: String,
    pub credits: Vec<Credit>,
}

#[derive(Debug, Clone)]
pub struct AboutInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub copyright: String,
    pub license: String,
    pub disclaimers: Vec<String>,
    pub repository: String,
    pub releases: String,
    pub issues: String,
    pub groups: Vec<CreditGroup>,
}

#[derive(Debug, Clone)]
pub struct AppUpdate {
    pub current: String,
    pub latest: String,
    pub available: bool,
    /// A release page of this project; the only kind of link Wishtory opens.
    pub url: String,
    pub prerelease: bool,
    pub published_at: String,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub seq: i64,
    pub time: String,
    pub level: String,
    pub source: String,
    pub message: String,
}
