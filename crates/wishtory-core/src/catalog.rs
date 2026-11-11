use crate::rules::{RuleSet, RulesStore};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock, RwLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const MAX_CATALOG_BYTES: usize = 8 * 1024 * 1024;
const MAX_ITEMS: usize = 20_000;
const MAX_BANNER_BYTES: usize = 2 * 1024 * 1024;
const MAX_BANNERS: usize = 5_000;
const UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// Every signed feed is published on this rolling release as `<asset>.envelope.json` (see
/// `.github/workflows/catalog-release.yml`). Copies named `catalog.json` and `banners.json` are
/// published too for app versions up to 0.0.2, which asked for those names.
const FEED_BASE_URL: &str = "https://github.com/Pylxyr/Wishtory/releases/download/catalog-v1";
const PINNED_PUBLIC_KEY: &str = include_str!("../../../catalog/catalog-public-key.txt");

// ---------------------------------------------------------------------------------------------
// Signed feeds: one implementation of "bundled copy + signed, versioned, cached updates"
// ---------------------------------------------------------------------------------------------

/// A JSON document the app ships with and can update at runtime from a signed envelope.
pub trait Feed: Clone + PartialEq + Send + Sync + 'static {
    /// Name used in error messages.
    const LABEL: &'static str;
    /// Release asset (`<ASSET>.envelope.json`), cache folder (`<ASSET>-cache`) and check file
    /// (`<ASSET>-last-check`) prefix.
    const ASSET: &'static str;
    /// Largest accepted document; the envelope limit is derived from it.
    const MAX_BYTES: usize;
    fn bundled() -> Self;
    fn parse_and_validate(bytes: &[u8]) -> Result<Self, String>;
    /// Monotonic; an update is only installed when this is higher than the current one.
    fn version(&self) -> u64;
}

fn max_envelope_bytes<F: Feed>() -> usize {
    F::MAX_BYTES * 4 / 3 + 2048
}

#[cfg(test)]
fn feed_url<F: Feed>() -> String {
    asset_url(F::ASSET)
}

/// Checks the signature against the pinned key and returns the validated document together
/// with the signed payload bytes.
pub fn verify_envelope<F: Feed>(envelope: &[u8]) -> Result<(F, Vec<u8>), String> {
    let public_key = BASE64
        .decode(PINNED_PUBLIC_KEY.trim())
        .map_err(|_| "invalid pinned catalog key".to_string())?;
    verify_envelope_with_key(envelope, &public_key)
}

fn verify_envelope_with_key<F: Feed>(
    envelope: &[u8],
    public_key: &[u8],
) -> Result<(F, Vec<u8>), String> {
    let label = F::LABEL;
    if envelope.is_empty() || envelope.len() > max_envelope_bytes::<F>() {
        return Err(format!(
            "{label} envelope is empty or exceeds the size limit"
        ));
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SignedEnvelope {
        payload: String,
        signature: String,
    }
    let envelope: SignedEnvelope =
        serde_json::from_slice(envelope).map_err(|e| format!("invalid {label} envelope: {e}"))?;
    let payload = BASE64
        .decode(envelope.payload)
        .map_err(|_| format!("invalid {label} payload encoding"))?;
    let signature = BASE64
        .decode(envelope.signature.trim())
        .map_err(|_| format!("invalid {label} signature encoding"))?;
    if !verify_signature_with_key(&payload, &signature, public_key) {
        return Err(format!("{label} signature verification failed"));
    }
    let document = F::parse_and_validate(&payload)?;
    Ok((document, payload))
}

fn verify_signature_with_key(bytes: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    UnparsedPublicKey::new(&ED25519, public_key)
        .verify(bytes, signature)
        .is_ok()
}

/// Why a download did not produce a document.
#[derive(Debug, PartialEq, Eq)]
pub enum SourceError {
    /// The server has no such file (a component the manifest names is not published yet).
    NotFound,
    Failed(String),
}

/// Where signed envelopes come from. A request names a published asset and nothing else, so no
/// request can carry anything about the person's saved history. Tests supply their own source.
pub trait Source: Send + Sync {
    fn get(
        &self,
        asset: &str,
        max_bytes: usize,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, SourceError>> + Send;
}

/// The repository's own release assets over HTTPS.
pub struct HttpSource;

fn asset_url(asset: &str) -> String {
    format!("{FEED_BASE_URL}/{asset}.envelope.json")
}

impl Source for HttpSource {
    fn get(
        &self,
        asset: &str,
        max_bytes: usize,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, SourceError>> + Send {
        let url = asset_url(asset);
        async move {
            download_limited(&url, max_bytes).await.map_err(|e| {
                if e.contains("404") {
                    SourceError::NotFound
                } else {
                    SourceError::Failed(e)
                }
            })
        }
    }
}

/// The current copy of one feed plus its on-disk cache of verified envelopes. It never downloads
/// by itself: a release (see `ReleaseStore`) decides which copy of each feed is current.
pub struct FeedStore<F: Feed> {
    current: RwLock<Arc<F>>,
    cache_dir: PathBuf,
    /// `None` checks signatures against the pinned key; tests use their own.
    key: Option<Vec<u8>>,
}

fn verify_with<F: Feed>(envelope: &[u8], key: Option<&[u8]>) -> Result<(F, Vec<u8>), String> {
    match key {
        None => verify_envelope::<F>(envelope),
        Some(key) => verify_envelope_with_key::<F>(envelope, key),
    }
}

impl<F: Feed> FeedStore<F> {
    /// The copy bundled in this app.
    #[cfg(test)]
    pub fn open(app_data: &Path) -> Self {
        Self::new(app_data, F::bundled(), None)
    }

    /// The copy a release names: the verified cached envelope with exactly `version`, or the
    /// bundled copy when that is the version. `None` when neither exists, so a release that cannot
    /// be loaded whole is never loaded in part.
    fn open_exact(app_data: &Path, version: u64, bundled: F, key: Option<Vec<u8>>) -> Option<Self> {
        if bundled.version() == version {
            return Some(Self::new(app_data, bundled, key));
        }
        let cache_dir = app_data.join(format!("{}-cache", F::ASSET));
        let path = cache_dir.join(format!("{version}.json"));
        let metadata = std::fs::metadata(&path).ok()?;
        if !metadata.is_file() || metadata.len() > max_envelope_bytes::<F>() as u64 {
            return None;
        }
        let (doc, _) = verify_with::<F>(&std::fs::read(path).ok()?, key.as_deref()).ok()?;
        (doc.version() == version).then(|| Self::new(app_data, doc, key))
    }

    /// Cached envelopes are named `<version>.json` and are never pruned, so they are tried newest
    /// first and the first one that verifies wins: older copies are not read, verified and parsed
    /// at every start (each can be several MB).
    fn newest_cached(cache_dir: &Path, key: Option<&[u8]>) -> Option<F> {
        let mut files: Vec<(u64, PathBuf)> = std::fs::read_dir(cache_dir)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .map(|path| {
                let version = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.parse().ok())
                    .unwrap_or(0);
                (version, path)
            })
            .collect();
        files.sort_by_key(|(version, _)| std::cmp::Reverse(*version));
        files.into_iter().find_map(|(_, path)| {
            let metadata = std::fs::metadata(&path).ok()?;
            if !metadata.is_file() || metadata.len() > max_envelope_bytes::<F>() as u64 {
                return None;
            }
            let bytes = std::fs::read(path).ok()?;
            verify_with::<F>(&bytes, key).ok().map(|(doc, _)| doc)
        })
    }

    fn new(app_data: &Path, current: F, key: Option<Vec<u8>>) -> Self {
        Self {
            current: RwLock::new(Arc::new(current)),
            cache_dir: app_data.join(format!("{}-cache", F::ASSET)),
            key,
        }
    }

    /// Cheap: callers share one immutable copy.
    pub fn current(&self) -> Arc<F> {
        self.current
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn verify(&self, envelope: &[u8]) -> Result<(F, Vec<u8>), String> {
        verify_with::<F>(envelope, self.key.as_deref())
    }

    /// Keeps a verified envelope on disk (atomically) without using it.
    fn cache(&self, version: u64, envelope: &[u8]) -> Result<(), String> {
        write_versioned::<F>(&self.cache_dir, version, envelope, self.key.as_deref())
    }

    fn set_current(&self, doc: Arc<F>) {
        *self.current.write().unwrap_or_else(|e| e.into_inner()) = doc;
    }
}

struct ResetBool<'a>(&'a AtomicBool);
impl Drop for ResetBool<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn write_versioned<F: Feed>(
    dir: &Path,
    version: u64,
    bytes: &[u8],
    key: Option<&[u8]>,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let target = dir.join(format!("{version}.json"));
    if target.exists() {
        if verify_with::<F>(&std::fs::read(&target).map_err(|e| e.to_string())?, key).is_ok() {
            return Ok(());
        }
        std::fs::remove_file(&target).map_err(|e| e.to_string())?;
    }
    let temp = dir.join(format!(".{version}-{}.tmp", std::process::id()));
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    if let Err(error) = std::fs::rename(&temp, &target) {
        let _ = std::fs::remove_file(&temp);
        return Err(error.to_string());
    }
    Ok(())
}

/// GitHub answers release-asset downloads with a redirect to its own asset hosts
/// (`release-assets.githubusercontent.com`, formerly `objects.githubusercontent.com`), so redirects
/// cannot simply be switched off here as they are for the history API. They are followed only over
/// HTTPS and only to GitHub-owned hosts, never to an arbitrary site.
fn redirect_allowed(target: &url::Url, hops_so_far: usize) -> bool {
    hops_so_far < 5
        && target.scheme() == "https"
        && target.host_str().is_some_and(|host| {
            host == "github.com"
                || host == "api.github.com"
                || host.ends_with(".githubusercontent.com")
        })
}

pub(crate) async fn download_limited(url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
    let client = crate::net::client_builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if redirect_allowed(attempt.url(), attempt.previous().len()) {
                attempt.follow()
            } else {
                attempt.stop() // the 3xx answer is returned and reported as a failed download
            }
        }))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("download returned {}", response.status()));
    }
    if response
        .content_length()
        .is_some_and(|len| len > max_bytes as u64)
    {
        return Err("download exceeds the size limit".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > max_bytes {
            return Err("download exceeds the size limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------------------------
// Game catalog
// ---------------------------------------------------------------------------------------------

pub type CatalogStore = FeedStore<Catalog>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogItem {
    pub game: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub rarity: u8,
    pub kind: String,
    #[serde(default)]
    pub traits: BTreeMap<String, String>,
    #[serde(default)]
    pub paimon_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub catalog_version: u64,
    pub generated_at: String,
    pub items: Vec<CatalogItem>,
    #[serde(skip)]
    lookup: Lookup,
}

/// Hash indexes over `items`, built on first use (a catalog can hold thousands of items and is
/// searched once per imported or exported pull).
#[derive(Debug)]
struct CatalogIndex {
    by_paimon_key: HashMap<String, usize>,
    /// game -> normalized name or alias -> position of the first item that carries it.
    by_name: HashMap<String, HashMap<String, usize>>,
}

#[derive(Default)]
struct Lookup(OnceLock<CatalogIndex>);

impl Clone for Lookup {
    fn clone(&self) -> Self {
        Self::default()
    }
}
/// The index is derived data, so it never makes two otherwise equal catalogs differ.
impl PartialEq for Lookup {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for Lookup {}
impl std::fmt::Debug for Lookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Lookup")
    }
}

impl Catalog {
    pub fn bundled() -> Self {
        Self::parse_and_validate(include_bytes!("../../../catalog/catalog.json"))
            .expect("bundled game catalog must validate")
    }

    pub fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_CATALOG_BYTES {
            return Err("catalog size is empty or exceeds 8 MiB".into());
        }
        let catalog: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("invalid catalog JSON: {e}"))?;
        if catalog.schema_version != 1 {
            return Err("unsupported catalog schema".into());
        }
        if catalog.catalog_version == 0 || catalog.generated_at.trim().is_empty() {
            return Err("catalog version and generation time are required".into());
        }
        if catalog.items.is_empty() || catalog.items.len() > MAX_ITEMS {
            return Err("catalog has an invalid item count".into());
        }
        let mut keys = HashSet::with_capacity(catalog.items.len());
        for item in &catalog.items {
            if !["hk4e", "hkrpg", "nap"].contains(&item.game.as_str())
                || item.id.trim().is_empty()
                || item.id.len() > 80
                || item.name.trim().is_empty()
                || item.name.len() > 160
                || !(1..=5).contains(&item.rarity)
                || !["character", "weapon", "light_cone", "bangboo"].contains(&item.kind.as_str())
                || !keys.insert((item.game.as_str(), item.id.as_str()))
            {
                return Err("catalog contains an invalid or duplicate item".into());
            }
            if item.aliases.len() > 32
                || item
                    .aliases
                    .iter()
                    .any(|a| a.trim().is_empty() || a.len() > 160)
            {
                return Err("catalog contains invalid aliases".into());
            }
            for (key, value) in &item.traits {
                let allowed = match (item.game.as_str(), key.as_str()) {
                    ("hk4e", "element") => {
                        ["anemo", "cryo", "dendro", "electro", "geo", "hydro", "pyro"]
                            .contains(&value.as_str())
                    }
                    ("hkrpg", "element") => [
                        "physical",
                        "fire",
                        "ice",
                        "lightning",
                        "wind",
                        "quantum",
                        "imaginary",
                    ]
                    .contains(&value.as_str()),
                    ("hkrpg", "path") => [
                        "destruction",
                        "hunt",
                        "erudition",
                        "harmony",
                        "nihility",
                        "preservation",
                        "abundance",
                        "remembrance",
                        "elation",
                    ]
                    .contains(&value.as_str()),
                    ("nap", "element") => [
                        "physical", "ice", "fire", "electric", "ether", "wind", "lumen",
                    ]
                    .contains(&value.as_str()),
                    ("nap", "specialty") => [
                        "attack", "stun", "support", "anomaly", "defense", "rupture", "armorer",
                    ]
                    .contains(&value.as_str()),
                    _ => false,
                };
                if !allowed {
                    return Err("catalog contains an unsupported game trait".into());
                }
            }
            let valid_kinds = match item.game.as_str() {
                "hk4e" => ["character", "weapon"].contains(&item.kind.as_str()),
                "hkrpg" => ["character", "light_cone"].contains(&item.kind.as_str()),
                "nap" => ["character", "weapon", "bangboo"].contains(&item.kind.as_str()),
                _ => false,
            };
            if !valid_kinds {
                return Err("catalog item kind does not match its game".into());
            }
            let trait_is_set =
                |key: &str| item.traits.get(key).is_some_and(|value| !value.is_empty());
            let complete = match (item.game.as_str(), item.kind.as_str()) {
                ("hk4e", "character") => trait_is_set("element"),
                ("hkrpg", "character") => trait_is_set("element") && trait_is_set("path"),
                ("nap", "character") => trait_is_set("element") && trait_is_set("specialty"),
                _ => true,
            };
            if !complete {
                return Err("catalog character is missing required game traits".into());
            }
            if item
                .paimon_key
                .as_ref()
                .is_some_and(|key| item.game != "hk4e" || key.trim().is_empty() || key.len() > 100)
            {
                return Err("catalog contains an invalid Paimon item key".into());
            }
        }
        Ok(catalog)
    }

    fn index(&self) -> &CatalogIndex {
        self.lookup.0.get_or_init(|| {
            let mut by_paimon_key = HashMap::new();
            let mut by_name = HashMap::<String, HashMap<String, usize>>::new();
            for (position, item) in self.items.iter().enumerate() {
                if item.game == "hk4e" {
                    if let Some(key) = &item.paimon_key {
                        by_paimon_key.entry(key.clone()).or_insert(position);
                    }
                }
                let names = by_name.entry(item.game.clone()).or_default();
                for name in std::iter::once(&item.name).chain(&item.aliases) {
                    names.entry(normalize_name(name)).or_insert(position);
                }
            }
            CatalogIndex {
                by_paimon_key,
                by_name,
            }
        })
    }

    pub fn item_by_paimon_key(&self, key: &str) -> Option<&CatalogItem> {
        self.items.get(*self.index().by_paimon_key.get(key)?)
    }

    pub fn item_by_english_name(&self, game: &str, name: &str) -> Option<&CatalogItem> {
        let position = *self.index().by_name.get(game)?.get(&normalize_name(name))?;
        self.items.get(position)
    }
}

impl Feed for Catalog {
    const LABEL: &'static str = "catalog";
    const ASSET: &'static str = "catalog";
    const MAX_BYTES: usize = MAX_CATALOG_BYTES;
    fn bundled() -> Self {
        Catalog::bundled()
    }
    fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        Catalog::parse_and_validate(bytes)
    }
    fn version(&self) -> u64 {
        self.catalog_version
    }
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------------------------
// Banner list
// ---------------------------------------------------------------------------------------------

/// One banner event window. `start` and `end` are inclusive server-local clock times
/// (`YYYY-MM-DD HH:MM:SS`), the same clock the game uses for the `time` of each wish record, so
/// they can be compared with a pull's time directly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BannerEntry {
    pub id: String,
    pub game: String,
    /// Banner key, matching `pity::banner_key` ("character", "weapon", "collab", ...).
    pub pool: String,
    pub name: String,
    /// `gacha_id` values the game reports for this banner (Star Rail and Zenless only).
    #[serde(default)]
    pub gacha_ids: Vec<String>,
    pub start: String,
    pub end: String,
    #[serde(default)]
    pub featured_5: Vec<String>,
    #[serde(default)]
    pub featured_4: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BannerSet {
    pub schema_version: u32,
    pub banners_version: u64,
    pub generated_at: String,
    pub banners: Vec<BannerEntry>,
}

pub(crate) fn is_clock(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 19 {
        return false;
    }
    let shape = b.iter().enumerate().all(|(i, c)| match i {
        4 | 7 => *c == b'-',
        10 => *c == b' ',
        13 | 16 => *c == b':',
        _ => c.is_ascii_digit(),
    });
    let part = |range: std::ops::Range<usize>| value[range].parse::<u32>().ok();
    shape
        && part(5..7).is_some_and(|m| (1..=12).contains(&m))
        && part(8..10).is_some_and(|d| (1..=31).contains(&d))
        && part(11..13).is_some_and(|h| h < 24)
        && part(14..16).is_some_and(|m| m < 60)
        && part(17..19).is_some_and(|s| s < 60)
}

impl BannerSet {
    pub fn bundled() -> Self {
        Self::parse_and_validate(include_bytes!("../../../catalog/banners.json"))
            .expect("bundled banner list must validate")
    }

    pub fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_BANNER_BYTES {
            return Err("banner list size is empty or exceeds 2 MiB".into());
        }
        let set: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("invalid banner list JSON: {e}"))?;
        if set.schema_version != 1 {
            return Err("unsupported banner list schema".into());
        }
        if set.banners_version == 0 || set.generated_at.trim().is_empty() {
            return Err("banner list version and generation time are required".into());
        }
        if set.banners.is_empty() || set.banners.len() > MAX_BANNERS {
            return Err("banner list has an invalid banner count".into());
        }
        let mut ids = HashSet::with_capacity(set.banners.len());
        for banner in &set.banners {
            let pool_ok = match banner.game.as_str() {
                "hk4e" | "nap" => ["character", "weapon"].contains(&banner.pool.as_str()),
                "hkrpg" => ["character", "weapon", "collab", "collab_weapon"]
                    .contains(&banner.pool.as_str()),
                _ => false,
            };
            if !pool_ok
                || banner.id.trim().is_empty()
                || banner.id.len() > 80
                || banner.name.trim().is_empty()
                || banner.name.len() > 160
                || !is_clock(&banner.start)
                || !is_clock(&banner.end)
                || banner.start > banner.end
                || banner.featured_5.is_empty()
                || banner.featured_5.len() > 8
                || banner.featured_4.len() > 16
                || banner.gacha_ids.len() > 8
                || !ids.insert(banner.id.as_str())
            {
                return Err("banner list contains an invalid or duplicate banner".into());
            }
            if banner
                .featured_5
                .iter()
                .chain(&banner.featured_4)
                .chain(&banner.gacha_ids)
                .any(|value| value.trim().is_empty() || value.len() > 80)
            {
                return Err("banner list contains an invalid item or pool id".into());
            }
        }
        Ok(set)
    }
}

impl Feed for BannerSet {
    const LABEL: &'static str = "banner list";
    const ASSET: &'static str = "banners";
    const MAX_BYTES: usize = MAX_BANNER_BYTES;
    fn bundled() -> Self {
        BannerSet::bundled()
    }
    fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        BannerSet::parse_and_validate(bytes)
    }
    fn version(&self) -> u64 {
        self.banners_version
    }
}

pub type BannerStore = FeedStore<BannerSet>;

// ---------------------------------------------------------------------------------------------
// Manifest: one catalog version and its change log
// ---------------------------------------------------------------------------------------------

pub const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
const MAX_CHANGES: usize = 2_000;
const MAX_CHANGE_IDS: usize = 100;
const GAME_IDS: [&str; 3] = ["hk4e", "hkrpg", "nap"];

/// How much a change can matter for saved pulls; the fixed rules are in the design document and
/// are applied by `scripts/changelog.mjs`. Ordered from most to least important.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Major,
    Minor,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeScope {
    Items,
    Banners,
    Rules,
}

/// One line of the change log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeEntry {
    pub id: String,
    pub scope: ChangeScope,
    pub game: String,
    /// Item ids, banner ids or pity rule keys the change concerns (at most 100).
    pub ids: Vec<String>,
    /// Plain language, shown to the person as written.
    pub summary: String,
    pub severity: Severity,
    /// A maintainer's note from `catalog/changes.json`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The exact feed versions that make up one catalog release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Components {
    pub catalog: u64,
    pub banners: u64,
    pub rules: u64,
}

/// A signed description of a complete catalog release (`manifest.envelope.json`). Older apps never
/// ask for it; they keep reading the three feeds it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    /// The one version the person sees.
    pub manifest_version: u64,
    pub generated_at: String,
    pub components: Components,
    pub changes: Vec<ChangeEntry>,
}

impl Manifest {
    pub fn bundled() -> Self {
        Self::parse_and_validate(include_bytes!("../../../catalog/manifest.json"))
            .expect("bundled manifest must validate")
    }

    pub fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
            return Err("manifest size is empty or exceeds 2 MiB".into());
        }
        let manifest: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("invalid manifest JSON: {e}"))?;
        if manifest.schema_version != 1 {
            return Err("unsupported manifest schema".into());
        }
        let c = manifest.components;
        if manifest.manifest_version == 0
            || manifest.generated_at.trim().is_empty()
            || c.catalog == 0
            || c.banners == 0
            || c.rules == 0
        {
            return Err("manifest versions and generation time are required".into());
        }
        if manifest.changes.len() > MAX_CHANGES {
            return Err("manifest has too many changes".into());
        }
        let sane = |text: &str, max: usize| !text.trim().is_empty() && text.chars().count() <= max;
        let mut seen = HashSet::with_capacity(manifest.changes.len());
        for change in &manifest.changes {
            if !sane(&change.id, 120)
                || !sane(&change.summary, 400)
                || !GAME_IDS.contains(&change.game.as_str())
                || change.ids.len() > MAX_CHANGE_IDS
                || change.ids.iter().any(|id| !sane(id, 160))
                || change.note.as_deref().is_some_and(|n| !sane(n, 500))
                || !seen.insert(change.id.as_str())
            {
                return Err("manifest contains an invalid or duplicate change".into());
            }
        }
        Ok(manifest)
    }

    /// How many entries are major, minor and info.
    pub fn counts(&self) -> (u32, u32, u32) {
        let count = |s: Severity| self.changes.iter().filter(|c| c.severity == s).count() as u32;
        (
            count(Severity::Major),
            count(Severity::Minor),
            count(Severity::Info),
        )
    }
}

impl Feed for Manifest {
    const LABEL: &'static str = "manifest";
    const ASSET: &'static str = "manifest";
    const MAX_BYTES: usize = MAX_MANIFEST_BYTES;
    fn bundled() -> Self {
        Manifest::bundled()
    }
    fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        Manifest::parse_and_validate(bytes)
    }
    fn version(&self) -> u64 {
        self.manifest_version
    }
}

// ---------------------------------------------------------------------------------------------
// Releases: check and stage, apply, history
// ---------------------------------------------------------------------------------------------

const STAGED_DIR: &str = "catalog-staged";
const HISTORY_FILE: &str = "catalog-history.json";
const DISMISSED_FILE: &str = "catalog-dismissed";
const CHECK_FILE: &str = "manifest-last-check";
const HISTORY_KEEP: usize = 20;
const HISTORY_CHANGES_KEPT: usize = 100;
const MAX_HISTORY_BYTES: u64 = 8 * 1024 * 1024;

/// The four signed envelopes of one release, exactly as downloaded.
#[derive(Debug, Clone)]
pub struct Envelopes {
    pub manifest: Vec<u8>,
    pub catalog: Vec<u8>,
    pub banners: Vec<u8>,
    pub rules: Vec<u8>,
}

impl Envelopes {
    const FILES: [(&'static str, fn(&Envelopes) -> &Vec<u8>); 4] = [
        ("manifest.envelope.json", |e| &e.manifest),
        ("catalog.envelope.json", |e| &e.catalog),
        ("banners.envelope.json", |e| &e.banners),
        ("rules.envelope.json", |e| &e.rules),
    ];
}

/// A release that was downloaded in full and verified, but is not in use.
#[derive(Debug)]
pub struct StagedRelease {
    pub manifest: Arc<Manifest>,
    pub catalog: Arc<Catalog>,
    pub banners: Arc<BannerSet>,
    pub rules: Arc<RuleSet>,
    pub envelopes: Envelopes,
}

/// What a check did.
#[derive(Debug)]
pub enum CheckOutcome {
    /// Another check was already running.
    Busy,
    /// The once-a-day gate said it is too early to ask the server.
    NotDue,
    /// The published release is not newer than the one in use.
    Current,
    /// The release is only partly published (a component is missing, older or newer than the
    /// manifest names). Nothing changed; a later check tries again.
    NotReady(String),
    /// A newer release is downloaded, verified and kept on disk, waiting to be applied.
    Staged(Arc<StagedRelease>),
}

impl CheckOutcome {
    /// The status string the UI knows.
    pub fn status(&self) -> &'static str {
        match self {
            CheckOutcome::Busy => "already-checking",
            CheckOutcome::NotDue | CheckOutcome::Current => "up-to-date",
            CheckOutcome::NotReady(_) => "not-ready",
            CheckOutcome::Staged(_) => "staged",
        }
    }
}

/// One applied release, kept for the catalog history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub manifest_version: u64,
    /// Unix seconds.
    pub applied_at: u64,
    /// `true` when nothing needed review and it was applied by itself.
    pub automatic: bool,
    pub affected_pulls: u64,
    /// The whole log has this many entries; `changes` holds the most important ones.
    pub changes_total: u32,
    pub changes: Vec<ChangeEntry>,
}

/// What `ReleaseStore::install` replaced, so a failed commit can put it back.
pub struct Applied {
    previous_manifest: Arc<Manifest>,
    previous_catalog: Arc<Catalog>,
    previous_banners: Arc<BannerSet>,
    previous_rules: Arc<RuleSet>,
    manifest_cache: PathBuf,
}

/// Everything `ReleaseStore::open` builds: the release logic and the three feeds it keeps in step.
pub struct Feeds {
    pub release: ReleaseStore,
    pub catalog: CatalogStore,
    pub banners: BannerStore,
    pub rules: RulesStore,
}

/// The current release (a manifest and the three feeds it names), a staged one, the catalog
/// history, and the checks that fetch newer releases. A release is only ever used whole: the
/// three feeds are opened at exactly the versions the current manifest names.
pub struct ReleaseStore {
    manifest: FeedStore<Manifest>,
    app_data: PathBuf,
    key: Option<Vec<u8>>,
    checking: AtomicBool,
    staged: Mutex<Option<Arc<StagedRelease>>>,
    /// The impact of the staged release on the saved pulls, kept while nothing has changed.
    pub(crate) impact: Mutex<Option<Arc<crate::impact::Impact>>>,
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

fn read_limited(path: &Path, max: usize) -> Option<Vec<u8>> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > max as u64 {
        return None;
    }
    std::fs::read(path).ok()
}

impl ReleaseStore {
    /// Opens the release the newest valid cached manifest names (or the bundled one). When any
    /// part of that release cannot be loaded, the whole bundled release is used instead.
    pub fn open(app_data: &Path) -> Feeds {
        Self::open_with(
            app_data,
            None,
            Manifest::bundled(),
            Catalog::bundled(),
            BannerSet::bundled(),
            RuleSet::bundled(),
        )
    }

    pub(crate) fn open_with(
        app_data: &Path,
        key: Option<Vec<u8>>,
        bundled_manifest: Manifest,
        bundled_catalog: Catalog,
        bundled_banners: BannerSet,
        bundled_rules: RuleSet,
    ) -> Feeds {
        let cache_dir = app_data.join(format!("{}-cache", Manifest::ASSET));
        let applied = FeedStore::<Manifest>::newest_cached(&cache_dir, key.as_deref())
            .filter(|m| m.manifest_version >= bundled_manifest.manifest_version);
        let named = applied.as_ref().and_then(|m| {
            Some((
                m.clone(),
                FeedStore::open_exact(
                    app_data,
                    m.components.catalog,
                    bundled_catalog.clone(),
                    key.clone(),
                )?,
                FeedStore::open_exact(
                    app_data,
                    m.components.banners,
                    bundled_banners.clone(),
                    key.clone(),
                )?,
                FeedStore::open_exact(
                    app_data,
                    m.components.rules,
                    bundled_rules.clone(),
                    key.clone(),
                )?,
            ))
        });
        let (manifest, catalog, banners, rules) = named.unwrap_or_else(|| {
            (
                bundled_manifest,
                FeedStore::new(app_data, bundled_catalog, key.clone()),
                FeedStore::new(app_data, bundled_banners, key.clone()),
                FeedStore::new(app_data, bundled_rules, key.clone()),
            )
        });
        Feeds {
            release: ReleaseStore {
                manifest: FeedStore::new(app_data, manifest, key.clone()),
                app_data: app_data.to_path_buf(),
                key,
                checking: AtomicBool::new(false),
                staged: Mutex::new(None),
                impact: Mutex::new(None),
            },
            catalog,
            banners,
            rules,
        }
    }

    /// The release in use.
    pub fn current(&self) -> Arc<Manifest> {
        self.manifest.current()
    }

    fn staged_dir(&self) -> PathBuf {
        self.app_data.join(STAGED_DIR)
    }

    /// Unix seconds of the last time the server was asked (a failed attempt counts).
    pub fn last_check(&self) -> Option<u64> {
        std::fs::read_to_string(self.app_data.join(CHECK_FILE))
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    fn mark_checked(&self) {
        let _ = std::fs::write(self.app_data.join(CHECK_FILE), unix_now().to_string());
    }

    fn check_due(&self) -> bool {
        self.last_check()
            .is_none_or(|v| unix_now().saturating_sub(v) >= UPDATE_INTERVAL.as_secs())
    }

    /// Asks for a newer release and, when there is one, downloads and verifies the manifest and
    /// every component and keeps them on disk. Nothing in use changes. A release whose parts are
    /// missing, older or newer than the manifest names is "not ready": the next check tries again.
    /// `force` skips the once-a-day gate.
    pub async fn check(&self, source: &impl Source, force: bool) -> Result<CheckOutcome, String> {
        if self.checking.swap(true, Ordering::AcqRel) {
            return Ok(CheckOutcome::Busy);
        }
        let _checking = ResetBool(&self.checking);
        if !force && !self.check_due() {
            return Ok(CheckOutcome::NotDue);
        }
        // Count a failed download as a check too, so an offline start is not retried on every launch.
        self.mark_checked();
        let fail = |e: SourceError| match e {
            SourceError::NotFound => "the release is not published yet".to_string(),
            SourceError::Failed(message) => message,
        };
        let manifest_bytes = match source
            .get(Manifest::ASSET, max_envelope_bytes::<Manifest>())
            .await
        {
            Ok(bytes) => bytes,
            Err(SourceError::NotFound) => {
                return Ok(CheckOutcome::NotReady("no release is published yet".into()))
            }
            Err(e) => return Err(fail(e)),
        };
        let (manifest, _) = self.manifest.verify(&manifest_bytes)?;
        if manifest.manifest_version <= self.current().manifest_version {
            return Ok(CheckOutcome::Current);
        }
        if let Some(staged) = self.staged() {
            if staged.manifest.manifest_version >= manifest.manifest_version {
                return Ok(CheckOutcome::Staged(staged));
            }
        }
        let mut parts = Vec::with_capacity(3);
        for (asset, max) in [
            (Catalog::ASSET, max_envelope_bytes::<Catalog>()),
            (BannerSet::ASSET, max_envelope_bytes::<BannerSet>()),
            (RuleSet::ASSET, max_envelope_bytes::<RuleSet>()),
        ] {
            match source.get(asset, max).await {
                Ok(bytes) => parts.push(bytes),
                Err(SourceError::NotFound) => {
                    return Ok(CheckOutcome::NotReady(format!(
                        "the {asset} feed of release {} is not published yet",
                        manifest.manifest_version
                    )))
                }
                Err(e) => return Err(fail(e)),
            }
        }
        let [catalog, banners, rules]: [Vec<u8>; 3] = parts.try_into().map_err(|_| "internal")?;
        let envelopes = Envelopes {
            manifest: manifest_bytes,
            catalog,
            banners,
            rules,
        };
        let staged = match self.verify_release(&envelopes, true) {
            Ok(staged) => staged,
            Err(ReleaseProblem::NotReady(why)) => return Ok(CheckOutcome::NotReady(why)),
            Err(ReleaseProblem::Invalid(why)) => return Err(why),
        };
        self.write_staged(&envelopes)?;
        let staged = Arc::new(staged);
        *self.staged.lock().unwrap_or_else(|e| e.into_inner()) = Some(staged.clone());
        *self.impact.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(CheckOutcome::Staged(staged))
    }

    /// Verifies the signatures, schemas and sizes of all four envelopes and that the three
    /// components are exactly the versions the manifest names.
    fn verify_release(
        &self,
        envelopes: &Envelopes,
        newer_only: bool,
    ) -> Result<StagedRelease, ReleaseProblem> {
        let invalid = ReleaseProblem::Invalid;
        let key = self.key.as_deref();
        let (manifest, _) = verify_with::<Manifest>(&envelopes.manifest, key).map_err(invalid)?;
        let (catalog, _) = verify_with::<Catalog>(&envelopes.catalog, key).map_err(invalid)?;
        let (banners, _) = verify_with::<BannerSet>(&envelopes.banners, key).map_err(invalid)?;
        let (rules, _) = verify_with::<RuleSet>(&envelopes.rules, key).map_err(invalid)?;
        let named = manifest.components;
        for (label, have, want) in [
            ("catalog", catalog.version(), named.catalog),
            ("banner list", banners.version(), named.banners),
            ("rules", rules.version(), named.rules),
        ] {
            if have != want {
                return Err(ReleaseProblem::NotReady(format!(
                    "the {label} is at version {have} but release {} names {want}",
                    manifest.manifest_version
                )));
            }
        }
        if newer_only && manifest.manifest_version <= self.current().manifest_version {
            return Err(ReleaseProblem::NotReady("the release is not newer".into()));
        }
        Ok(StagedRelease {
            manifest: Arc::new(manifest),
            catalog: Arc::new(catalog),
            banners: Arc::new(banners),
            rules: Arc::new(rules),
            envelopes: envelopes.clone(),
        })
    }

    fn write_staged(&self, envelopes: &Envelopes) -> Result<(), String> {
        let dir = self.staged_dir();
        let temp = self.app_data.join(format!("{STAGED_DIR}.tmp"));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
        for (name, part) in Envelopes::FILES {
            std::fs::write(temp.join(name), part(envelopes)).map_err(|e| e.to_string())?;
        }
        // A newer release replaces an older staged one. A crash between these two steps leaves
        // nothing staged, which the next check puts right.
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::rename(&temp, &dir).map_err(|e| {
            let _ = std::fs::remove_dir_all(&temp);
            e.to_string()
        })
    }

    /// The staged release, loaded from disk and verified again the first time it is used in this
    /// run. A copy that fails verification, is not newer than the release in use, or no longer
    /// has all its parts is dropped.
    pub fn staged(&self) -> Option<Arc<StagedRelease>> {
        let mut memo = self.staged.lock().unwrap_or_else(|e| e.into_inner());
        let present = self.staged_dir().join(Envelopes::FILES[0].0).is_file();
        if !present {
            *memo = None;
            return None;
        }
        if let Some(staged) = memo.as_ref() {
            if staged.manifest.manifest_version > self.current().manifest_version {
                return Some(staged.clone());
            }
        }
        *memo = None;
        let loaded = self.load_staged();
        match loaded {
            Some(staged) => {
                let staged = Arc::new(staged);
                *memo = Some(staged.clone());
                Some(staged)
            }
            None => {
                drop(memo);
                self.drop_staged();
                None
            }
        }
    }

    fn load_staged(&self) -> Option<StagedRelease> {
        let dir = self.staged_dir();
        let read = |name: &str, max: usize| read_limited(&dir.join(name), max);
        let envelopes = Envelopes {
            manifest: read(Envelopes::FILES[0].0, max_envelope_bytes::<Manifest>())?,
            catalog: read(Envelopes::FILES[1].0, max_envelope_bytes::<Catalog>())?,
            banners: read(Envelopes::FILES[2].0, max_envelope_bytes::<BannerSet>())?,
            rules: read(Envelopes::FILES[3].0, max_envelope_bytes::<RuleSet>())?,
        };
        self.verify_release(&envelopes, true).ok()
    }

    /// Forgets the staged release (a newer one replaced it, it was applied, or it failed to load).
    pub fn drop_staged(&self) {
        *self.staged.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.impact.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let _ = std::fs::remove_dir_all(self.staged_dir());
        let _ = std::fs::remove_file(self.app_data.join(DISMISSED_FILE));
    }

    /// "Not now": the release stays staged but the start-up notice is not shown again for it.
    pub fn dismiss(&self, version: u64) {
        let _ = std::fs::write(self.app_data.join(DISMISSED_FILE), version.to_string());
    }

    pub fn dismissed(&self) -> Option<u64> {
        std::fs::read_to_string(self.app_data.join(DISMISSED_FILE))
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    /// Makes the staged release the one in use, all three feeds and the manifest together. Every
    /// file is kept before the manifest is, and the manifest is what makes the release current,
    /// so a crash at any point leaves either the old release or the new one, never a mix. The
    /// caller runs this inside the database transaction that re-derives saved banners and
    /// calls `revert` if that transaction then fails.
    pub fn install(
        &self,
        staged: &StagedRelease,
        catalog: &CatalogStore,
        banners: &BannerStore,
        rules: &RulesStore,
    ) -> Result<Applied, String> {
        let previous_manifest = self.current();
        if staged.manifest.manifest_version <= previous_manifest.manifest_version {
            return Err("this release is not newer than the one in use".into());
        }
        catalog.cache(staged.catalog.version(), &staged.envelopes.catalog)?;
        banners.cache(staged.banners.version(), &staged.envelopes.banners)?;
        rules.cache(staged.rules.version(), &staged.envelopes.rules)?;
        self.manifest
            .cache(staged.manifest.manifest_version, &staged.envelopes.manifest)?;
        let applied = Applied {
            previous_manifest,
            previous_catalog: catalog.current(),
            previous_banners: banners.current(),
            previous_rules: rules.current(),
            manifest_cache: self
                .manifest
                .cache_dir
                .join(format!("{}.json", staged.manifest.manifest_version)),
        };
        catalog.set_current(staged.catalog.clone());
        banners.set_current(staged.banners.clone());
        rules.set_current(staged.rules.clone());
        self.manifest.set_current(staged.manifest.clone());
        Ok(applied)
    }

    /// Puts the previous release back after `install` when the database could not follow.
    pub fn revert(
        &self,
        applied: Applied,
        catalog: &CatalogStore,
        banners: &BannerStore,
        rules: &RulesStore,
    ) {
        let _ = std::fs::remove_file(&applied.manifest_cache);
        catalog.set_current(applied.previous_catalog);
        banners.set_current(applied.previous_banners);
        rules.set_current(applied.previous_rules);
        self.manifest.set_current(applied.previous_manifest);
    }

    /// Records an applied release and removes it from staging.
    pub fn finish(&self, staged: &StagedRelease, automatic: bool, affected_pulls: u64) {
        let mut changes = staged.manifest.changes.clone();
        changes.sort_by(|a, b| a.severity.cmp(&b.severity).then_with(|| a.id.cmp(&b.id)));
        let entry = HistoryEntry {
            manifest_version: staged.manifest.manifest_version,
            applied_at: unix_now(),
            automatic,
            affected_pulls,
            changes_total: changes.len() as u32,
            changes: changes.into_iter().take(HISTORY_CHANGES_KEPT).collect(),
        };
        let mut history = self.history();
        history.insert(0, entry);
        history.truncate(HISTORY_KEEP);
        if let Ok(json) = serde_json::to_vec(&history) {
            let _ = write_atomic(&self.app_data.join(HISTORY_FILE), &json);
        }
        self.drop_staged();
    }

    /// Applied releases, newest first.
    pub fn history(&self) -> Vec<HistoryEntry> {
        read_limited(
            &self.app_data.join(HISTORY_FILE),
            MAX_HISTORY_BYTES as usize,
        )
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
    }
}

enum ReleaseProblem {
    /// A part is missing, older or newer than the manifest names: try again later.
    NotReady(String),
    /// A signature, schema or size check failed.
    Invalid(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::KeyPair;

    const SAMPLE: &[u8] = br#"{"schema_version":1,"catalog_version":1,"generated_at":"2026-10-24T00:00:00Z","items":[{"game":"hk4e","id":"10000002","name":"Kamisato Ayaka","aliases":["Ayaka"],"rarity":5,"kind":"character","traits":{"element":"cryo"},"paimon_key":"kamisato_ayaka"},{"game":"hkrpg","id":"1001","name":"March 7th","aliases":[],"rarity":4,"kind":"character","traits":{"element":"ice","path":"preservation"}},{"game":"nap","id":"1011","name":"Anby","aliases":[],"rarity":4,"kind":"character","traits":{"element":"electric","specialty":"stun"}}]}"#;

    #[test]
    fn validates_catalog_for_all_supported_games() {
        let catalog = Catalog::parse_and_validate(SAMPLE).unwrap();
        assert_eq!(catalog.catalog_version, 1);
        assert_eq!(catalog.items.len(), 3);
    }

    #[test]
    fn validates_star_rail_elation_path() {
        let with_elation = String::from_utf8(SAMPLE.to_vec())
            .unwrap()
            .replace("\"path\":\"preservation\"", "\"path\":\"elation\"");
        assert!(Catalog::parse_and_validate(with_elation.as_bytes()).is_ok());
    }

    #[test]
    fn validates_new_zenless_attributes_and_specialty() {
        let updated = String::from_utf8(SAMPLE.to_vec())
            .unwrap()
            .replace("\"element\":\"electric\"", "\"element\":\"lumen\"")
            .replace("\"specialty\":\"stun\"", "\"specialty\":\"armorer\"");
        assert!(Catalog::parse_and_validate(updated.as_bytes()).is_ok());
    }

    #[test]
    fn rejects_duplicate_game_item_ids() {
        let duplicate = String::from_utf8(SAMPLE.to_vec()).unwrap().replace(
            "{\"game\":\"nap\",\"id\":\"1011\"",
            "{\"game\":\"hk4e\",\"id\":\"10000002\"",
        );
        assert!(Catalog::parse_and_validate(duplicate.as_bytes()).is_err());
    }

    #[test]
    fn rejects_bad_schema_rarity_and_game_specific_traits() {
        for (from, to) in [
            ("\"schema_version\":1", "\"schema_version\":2"),
            ("\"rarity\":5", "\"rarity\":9"),
            ("\"element\":\"cryo\"", "\"element\":\"electric\""),
        ] {
            let invalid = String::from_utf8(SAMPLE.to_vec())
                .unwrap()
                .replacen(from, to, 1);
            assert!(
                Catalog::parse_and_validate(invalid.as_bytes()).is_err(),
                "accepted {to}"
            );
        }
    }

    #[test]
    fn rejects_oversized_catalogs() {
        assert!(Catalog::parse_and_validate(&vec![b' '; MAX_CATALOG_BYTES + 1]).is_err());
    }

    #[test]
    fn verifies_ed25519_signature_and_rejects_modified_bytes() {
        let key = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[7u8; 32]).unwrap();
        let public_key = key.public_key().as_ref();
        let signature = key.sign(SAMPLE);
        assert!(verify_signature_with_key(
            SAMPLE,
            signature.as_ref(),
            public_key
        ));
        assert!(!verify_signature_with_key(
            b"modified",
            signature.as_ref(),
            public_key
        ));
        assert!(!verify_signature_with_key(SAMPLE, b"invalid", public_key));
    }

    #[test]
    fn signed_envelope_verifies_payload_and_rejects_tampering() {
        let key = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[7u8; 32]).unwrap();
        let signature = BASE64.encode(key.sign(SAMPLE).as_ref());
        let envelope =
            serde_json::json!({"payload": BASE64.encode(SAMPLE), "signature": signature})
                .to_string();
        let (catalog, payload) =
            verify_envelope_with_key::<Catalog>(envelope.as_bytes(), key.public_key().as_ref())
                .unwrap();
        assert_eq!(catalog.catalog_version, 1);
        assert_eq!(payload, SAMPLE);
        let mut tampered: serde_json::Value = serde_json::from_str(&envelope).unwrap();
        let mut changed_payload = SAMPLE.to_vec();
        changed_payload[10] ^= 1;
        tampered["payload"] = serde_json::Value::from(BASE64.encode(changed_payload));
        assert!(verify_envelope_with_key::<Catalog>(
            tampered.to_string().as_bytes(),
            key.public_key().as_ref()
        )
        .is_err());
    }

    #[test]
    fn versioned_cache_recovers_an_interrupted_or_invalid_file() {
        let dir =
            std::env::temp_dir().join(format!("wishtory-catalog-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let target = dir.join("7.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&target, b"partial").unwrap();
        write_versioned::<Catalog>(&dir, 7, b"complete signed envelope bytes", None).unwrap();
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"complete signed envelope bytes"
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn newest_cached_copy_is_tried_first_and_a_bad_one_falls_back_to_an_older_copy() {
        let app_data =
            std::env::temp_dir().join(format!("wishtory-catalog-newest-{}", std::process::id()));
        let cache = app_data.join("catalog-cache");
        let _ = std::fs::remove_dir_all(&app_data);
        std::fs::create_dir_all(&cache).unwrap();
        let good = include_bytes!("../../../catalog/catalog-test-envelope.json");
        std::fs::write(cache.join("2.json"), good).unwrap();
        std::fs::write(cache.join("1.json"), b"older and never needed").unwrap();
        std::fs::write(cache.join("3.json"), b"tampered newest copy").unwrap();
        std::fs::write(cache.join("notes.txt"), b"ignored").unwrap();
        assert_eq!(
            CatalogStore::newest_cached(&cache, None)
                .unwrap()
                .catalog_version,
            2
        );
        std::fs::remove_file(cache.join("2.json")).unwrap();
        assert!(CatalogStore::newest_cached(&cache, None).is_none());
        assert!(CatalogStore::newest_cached(&app_data.join("missing"), None).is_none());
        std::fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn redirects_stay_on_github_over_https() {
        let allowed = |url: &str| redirect_allowed(&url::Url::parse(url).unwrap(), 1);
        assert!(allowed(
            "https://github.com/Pylxyr/Wishtory/releases/download/catalog-v1/a.json"
        ));
        assert!(allowed(
            "https://api.github.com/repos/Pylxyr/Wishtory/releases"
        ));
        assert!(allowed(
            "https://release-assets.githubusercontent.com/github-production-release-asset/1"
        ));
        assert!(allowed(
            "https://objects.githubusercontent.com/github-production-release-asset/1"
        ));
        assert!(!allowed("http://github.com/Pylxyr/Wishtory"));
        assert!(!allowed("https://example.com/catalog.json"));
        assert!(!allowed("https://evilgithubusercontent.com/x"));
        assert!(!allowed("https://github.com.evil.example/x"));
        assert!(!allowed("https://github.com@evil.example/x"));
        assert!(!redirect_allowed(
            &url::Url::parse("https://github.com/x").unwrap(),
            5
        ));
    }

    #[test]
    fn a_release_loads_the_named_version_or_nothing() {
        let app_data =
            std::env::temp_dir().join(format!("wishtory-catalog-open-{}", std::process::id()));
        let cache = app_data.join("catalog-cache");
        let _ = std::fs::remove_dir_all(&app_data);
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(
            cache.join("2.json"),
            include_bytes!("../../../catalog/catalog-test-envelope.json"),
        )
        .unwrap();
        let older_bundle = || Catalog {
            catalog_version: 1,
            ..Catalog::bundled()
        };
        let named = |version| CatalogStore::open_exact(&app_data, version, older_bundle(), None);
        assert_eq!(named(2).unwrap().current().catalog_version, 2);
        // The bundled copy serves its own version; any other version must be on disk.
        assert_eq!(named(1).unwrap().current().catalog_version, 1);
        assert!(named(3).is_none());
        std::fs::write(cache.join("2.json"), b"tampered").unwrap();
        assert!(named(2).is_none());
        std::fs::remove_dir_all(app_data).unwrap();
    }

    const BANNER_SAMPLE: &[u8] = br#"{"schema_version":1,"banners_version":1,"generated_at":"2026-10-25T00:00:00Z","banners":[{"id":"hk4e-character-a","game":"hk4e","pool":"character","name":"Event A","start":"2026-01-01 18:00:00","end":"2026-01-21 17:59:59","featured_5":["10000002"]},{"id":"nap-character-b","game":"nap","pool":"character","name":"Channel B","gacha_ids":["2001"],"start":"2026-01-01 12:00:00","end":"2026-01-21 11:59:59","featured_5":["1011"],"featured_4":["1021"]}]}"#;

    #[test]
    fn bundled_banner_list_only_features_catalog_five_stars() {
        let banners = BannerSet::bundled();
        let catalog = Catalog::bundled();
        assert!(!banners.banners.is_empty());
        for banner in &banners.banners {
            for id in &banner.featured_5 {
                assert!(
                    catalog
                        .items
                        .iter()
                        .any(|i| i.game == banner.game && &i.id == id && i.rarity == 5),
                    "{} features unknown 5-star {id}",
                    banner.id
                );
            }
        }
    }

    #[test]
    fn validates_banner_list_and_rejects_bad_entries() {
        let set = BannerSet::parse_and_validate(BANNER_SAMPLE).unwrap();
        assert_eq!(set.banners.len(), 2);
        assert_eq!(set.banners[1].gacha_ids, vec!["2001".to_string()]);
        for (from, to) in [
            ("\"schema_version\":1", "\"schema_version\":2"),
            (
                "\"start\":\"2026-01-01 18:00:00\"",
                "\"start\":\"2026-02-01 18:00:00\"",
            ),
            ("2026-01-21 17:59:59", "2026-01-21 25:00:00"),
            ("\"featured_5\":[\"10000002\"]", "\"featured_5\":[]"),
            ("\"pool\":\"character\"", "\"pool\":\"bangboo\""),
            ("\"game\":\"hk4e\"", "\"game\":\"other\""),
            ("\"id\":\"nap-character-b\"", "\"id\":\"hk4e-character-a\""),
        ] {
            let invalid = String::from_utf8(BANNER_SAMPLE.to_vec())
                .unwrap()
                .replacen(from, to, 1);
            assert!(
                BannerSet::parse_and_validate(invalid.as_bytes()).is_err(),
                "accepted {to}"
            );
        }
        assert!(BannerSet::parse_and_validate(&vec![b' '; MAX_BANNER_BYTES + 1]).is_err());
    }

    #[test]
    fn clock_strings_must_be_real_server_times() {
        assert!(is_clock("2026-10-25 18:00:00"));
        assert!(!is_clock("2026-10-25T18:00:00"));
        assert!(!is_clock("2026-13-25 18:00:00"));
        assert!(!is_clock("2026-10-25 24:00:00"));
        assert!(!is_clock("2026-10-25"));
    }

    #[test]
    fn signed_banner_envelope_verifies_and_rejects_tampering() {
        let key = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[7u8; 32]).unwrap();
        let signature = BASE64.encode(key.sign(BANNER_SAMPLE).as_ref());
        let envelope =
            serde_json::json!({"payload": BASE64.encode(BANNER_SAMPLE), "signature": signature})
                .to_string();
        let (set, _) =
            verify_envelope_with_key::<BannerSet>(envelope.as_bytes(), key.public_key().as_ref())
                .unwrap();
        assert_eq!(set.banners_version, 1);
        let mut changed_payload = BANNER_SAMPLE.to_vec();
        changed_payload[10] ^= 1;
        let mut tampered: serde_json::Value = serde_json::from_str(&envelope).unwrap();
        tampered["payload"] = serde_json::Value::from(BASE64.encode(changed_payload));
        assert!(verify_envelope_with_key::<BannerSet>(
            tampered.to_string().as_bytes(),
            key.public_key().as_ref()
        )
        .is_err());
        let other = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[8u8; 32]).unwrap();
        assert!(verify_envelope_with_key::<BannerSet>(
            envelope.as_bytes(),
            other.public_key().as_ref()
        )
        .is_err());
    }

    #[test]
    fn feeds_are_requested_by_the_names_the_release_publishes() {
        // Regression: the app asked for catalog.json / banners.json while the workflow only
        // uploads <name>.envelope.json, so every update check ended in a 404.
        assert!(feed_url::<Catalog>().ends_with("/catalog-v1/catalog.envelope.json"));
        assert!(feed_url::<BannerSet>().ends_with("/catalog-v1/banners.envelope.json"));
    }

    #[test]
    fn lookups_use_the_first_matching_item_like_the_old_linear_scan() {
        let catalog = Catalog::parse_and_validate(SAMPLE).unwrap();
        assert_eq!(
            catalog
                .item_by_paimon_key("kamisato_ayaka")
                .map(|i| i.id.as_str()),
            Some("10000002")
        );
        assert!(catalog.item_by_paimon_key("nobody").is_none());
        for name in ["Kamisato Ayaka", "ayaka", "  KAMISATO   ayaka "] {
            assert_eq!(
                catalog
                    .item_by_english_name("hk4e", name)
                    .map(|i| i.id.as_str()),
                Some("10000002"),
                "{name}"
            );
        }
        assert!(catalog.item_by_english_name("hkrpg", "Ayaka").is_none());
        assert_eq!(
            catalog
                .item_by_english_name("hkrpg", "march 7th")
                .map(|i| i.id.as_str()),
            Some("1001")
        );
        // The derived index never affects equality or cloning.
        let copy = catalog.clone();
        assert_eq!(copy, catalog);
        assert!(copy.item_by_english_name("nap", "Anby").is_some());
    }
}

#[cfg(test)]
pub(crate) mod testkit {
    use super::*;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use std::collections::HashMap;

    pub fn keypair() -> Ed25519KeyPair {
        Ed25519KeyPair::from_seed_unchecked(&[7u8; 32]).unwrap()
    }

    pub fn public_key() -> Vec<u8> {
        keypair().public_key().as_ref().to_vec()
    }

    pub fn sign(payload: &[u8]) -> Vec<u8> {
        serde_json::json!({
            "payload": BASE64.encode(payload),
            "signature": BASE64.encode(keypair().sign(payload).as_ref()),
        })
        .to_string()
        .into_bytes()
    }

    /// A signed release at the given versions (bundled data with new version numbers).
    pub fn envelopes(manifest: u64, catalog: u64, banners: u64, rules: u64) -> Envelopes {
        let mut c = Catalog::bundled();
        c.catalog_version = catalog;
        let mut b = BannerSet::bundled();
        b.banners_version = banners;
        let mut r = RuleSet::bundled();
        r.rules_version = rules;
        envelopes_of(manifest, c, b, r)
    }

    /// A signed release made of exactly these documents (their versions are what it names).
    pub fn envelopes_of(manifest: u64, c: Catalog, b: BannerSet, r: RuleSet) -> Envelopes {
        let (catalog, banners, rules) = (c.catalog_version, b.banners_version, r.rules_version);
        let m = Manifest {
            schema_version: 1,
            manifest_version: manifest,
            generated_at: "2026-11-01T00:00:00Z".into(),
            components: Components {
                catalog,
                banners,
                rules,
            },
            changes: vec![ChangeEntry {
                id: "banners:window:hk4e:x".into(),
                scope: ChangeScope::Banners,
                game: "hk4e".into(),
                ids: vec!["x".into()],
                summary: "The dates of a banner changed.".into(),
                severity: Severity::Major,
                note: None,
            }],
        };
        let json = |v: &dyn erased::Json| sign(&v.json());
        Envelopes {
            manifest: json(&m),
            catalog: json(&c),
            banners: json(&b),
            rules: json(&r),
        }
    }

    pub mod erased {
        pub trait Json {
            fn json(&self) -> Vec<u8>;
        }
        impl<T: serde::Serialize> Json for T {
            fn json(&self) -> Vec<u8> {
                serde_json::to_vec(self).unwrap()
            }
        }
    }

    /// A source with fixed answers that remembers what was asked.
    #[derive(Default)]
    pub struct Fake {
        pub files: Mutex<HashMap<String, Option<Vec<u8>>>>,
        pub asked: Mutex<Vec<String>>,
        pub offline: AtomicBool,
    }

    impl Fake {
        pub fn publish(&self, e: &Envelopes) {
            let mut files = self.files.lock().unwrap();
            files.insert("manifest".into(), Some(e.manifest.clone()));
            files.insert("catalog".into(), Some(e.catalog.clone()));
            files.insert("banners".into(), Some(e.banners.clone()));
            files.insert("rules".into(), Some(e.rules.clone()));
        }
        pub fn remove(&self, asset: &str) {
            self.files.lock().unwrap().insert(asset.into(), None);
        }
        pub fn put(&self, asset: &str, bytes: Vec<u8>) {
            self.files.lock().unwrap().insert(asset.into(), Some(bytes));
        }
    }

    impl Source for Fake {
        fn get(
            &self,
            asset: &str,
            _max: usize,
        ) -> impl std::future::Future<Output = Result<Vec<u8>, SourceError>> + Send {
            self.asked.lock().unwrap().push(asset.to_string());
            let answer = if self.offline.load(Ordering::Relaxed) {
                Err(SourceError::Failed("offline".into()))
            } else {
                match self.files.lock().unwrap().get(asset) {
                    Some(Some(bytes)) => Ok(bytes.clone()),
                    _ => Err(SourceError::NotFound),
                }
            };
            async move { answer }
        }
    }

    pub fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("wishtory-release-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The bundled release is version 1 here, so any signed release above it is "newer".
    pub fn open(dir: &Path) -> Feeds {
        open_with_banners(dir, BannerSet::bundled())
    }

    pub fn open_with_banners(dir: &Path, banners: BannerSet) -> Feeds {
        let mut manifest = Manifest::bundled();
        manifest.manifest_version = 1;
        let mut catalog = Catalog::bundled();
        catalog.catalog_version = 1;
        let mut banners = banners;
        banners.banners_version = 1;
        let mut rules = RuleSet::bundled();
        rules.rules_version = 1;
        manifest.components = Components {
            catalog: 1,
            banners: 1,
            rules: 1,
        };
        ReleaseStore::open_with(dir, Some(public_key()), manifest, catalog, banners, rules)
    }

    pub fn versions(f: &Feeds) -> (u64, u64, u64, u64) {
        (
            f.release.current().manifest_version,
            f.catalog.current().catalog_version,
            f.banners.current().banners_version,
            f.rules.current().rules_version,
        )
    }

    pub fn run<T>(f: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(f)
    }
}

#[cfg(test)]
mod release_tests {
    use super::testkit::*;
    use super::*;
    use ring::signature::Ed25519KeyPair;

    #[test]
    fn the_bundled_manifest_names_the_bundled_feeds() {
        let m = Manifest::bundled();
        assert_eq!(m.components.catalog, Catalog::bundled().catalog_version);
        assert_eq!(m.components.banners, BannerSet::bundled().banners_version);
        assert_eq!(m.components.rules, RuleSet::bundled().rules_version);
    }

    #[test]
    fn manifest_validation_rejects_bad_documents() {
        let good = |tweak: &dyn Fn(&mut serde_json::Value)| {
            let mut v = serde_json::json!({
                "schema_version": 1, "manifest_version": 2, "generated_at": "2026-11-01T00:00:00Z",
                "components": {"catalog": 1, "banners": 1, "rules": 1},
                "changes": [{"id": "a", "scope": "items", "game": "hk4e", "ids": ["1"],
                    "summary": "s", "severity": "info"}]
            });
            tweak(&mut v);
            Manifest::parse_and_validate(v.to_string().as_bytes())
        };
        assert!(good(&|_| {}).is_ok());
        assert!(good(&|v| v["schema_version"] = 2.into()).is_err());
        assert!(good(&|v| v["manifest_version"] = 0.into()).is_err());
        assert!(good(&|v| v["components"]["rules"] = 0.into()).is_err());
        assert!(good(&|v| v["extra"] = 1.into()).is_err());
        assert!(good(&|v| v["changes"][0]["severity"] = "urgent".into()).is_err());
        assert!(good(&|v| v["changes"][0]["game"] = "other".into()).is_err());
        assert!(good(&|v| v["changes"][0]["summary"] = "".into()).is_err());
        assert!(good(&|v| v["changes"][0]["unknown"] = 1.into()).is_err());
        assert!(good(&|v| {
            let first = v["changes"][0].clone();
            v["changes"].as_array_mut().unwrap().push(first);
        })
        .is_err());
        assert!(good(&|v| {
            v["changes"] = (0..2001)
                .map(|i| {
                    serde_json::json!({"id": format!("c{i}"), "scope": "rules", "game": "nap",
                    "ids": [], "summary": "s", "severity": "info"})
                })
                .collect::<Vec<_>>()
                .into()
        })
        .is_err());
        assert!(Manifest::parse_and_validate(&vec![b' '; MAX_MANIFEST_BYTES + 1]).is_err());
    }

    #[test]
    fn a_check_stages_a_release_without_installing_it() {
        let dir = scratch("stage");
        let feeds = open(&dir);
        let source = Fake::default();
        source.publish(&envelopes(2, 5, 6, 3));
        let outcome = run(feeds.release.check(&source, true)).unwrap();
        assert_eq!(outcome.status(), "staged");
        // The current data is untouched.
        assert_eq!(versions(&feeds), (1, 1, 1, 1));
        assert!(feeds.release.staged().is_some());
        // A request names only the four fixed files, in order, and nothing else.
        assert_eq!(
            *source.asked.lock().unwrap(),
            ["manifest", "catalog", "banners", "rules"]
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn requests_are_plain_fixed_public_urls() {
        for asset in ["manifest", "catalog", "banners", "rules"] {
            let url = url::Url::parse(&asset_url(asset)).unwrap();
            assert_eq!(url.scheme(), "https");
            assert_eq!(url.host_str(), Some("github.com"));
            assert_eq!(url.query(), None);
            assert_eq!(url.fragment(), None);
            assert!(url.path().ends_with(&format!("/{asset}.envelope.json")));
        }
    }

    #[test]
    fn a_staged_release_survives_a_restart_and_is_verified_again() {
        let dir = scratch("restart");
        let source = Fake::default();
        source.publish(&envelopes(2, 5, 6, 3));
        {
            let feeds = open(&dir);
            run(feeds.release.check(&source, true)).unwrap();
            // The app is closed before the person decided anything.
        }
        let feeds = open(&dir);
        assert_eq!(versions(&feeds), (1, 1, 1, 1));
        assert_eq!(feeds.release.staged().unwrap().manifest.manifest_version, 2);
        // A staged copy that was changed on disk is dropped, and the data in use stays.
        let reopened = open(&dir);
        let file = dir.join(STAGED_DIR).join("catalog.envelope.json");
        let mut bytes = std::fs::read(&file).unwrap();
        let at = bytes.len() / 2;
        bytes[at] ^= 1;
        std::fs::write(&file, bytes).unwrap();
        assert!(reopened.release.staged().is_none());
        assert!(!dir.join(STAGED_DIR).exists());
        assert_eq!(versions(&reopened), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tampered_manifests_and_components_are_rejected_and_nothing_is_staged() {
        let dir = scratch("tamper");
        let feeds = open(&dir);
        let good = envelopes(2, 5, 6, 3);
        // A manifest signed with another key.
        let other = Ed25519KeyPair::from_seed_unchecked(&[9u8; 32]).unwrap();
        let manifest_json: serde_json::Value = {
            let envelope: serde_json::Value = serde_json::from_slice(&good.manifest).unwrap();
            let payload = BASE64
                .decode(envelope["payload"].as_str().unwrap())
                .unwrap();
            serde_json::from_slice(&payload).unwrap()
        };
        let payload = manifest_json.to_string().into_bytes();
        let forged = serde_json::json!({
            "payload": BASE64.encode(&payload),
            "signature": BASE64.encode(other.sign(&payload).as_ref()),
        })
        .to_string()
        .into_bytes();
        let source = Fake::default();
        source.publish(&good);
        source.put("manifest", forged);
        assert!(run(feeds.release.check(&source, true)).is_err());
        // A component whose bytes changed after signing.
        source.publish(&good);
        let mut bad = good.rules.clone();
        let at = bad.len() / 2;
        bad[at] ^= 1;
        source.put("rules", bad);
        assert!(run(feeds.release.check(&source, true)).is_err());
        assert!(feeds.release.staged().is_none());
        assert!(!dir.join(STAGED_DIR).exists());
        assert_eq!(versions(&feeds), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_downgraded_or_repeated_manifest_changes_nothing() {
        let dir = scratch("downgrade");
        let feeds = open(&dir);
        let source = Fake::default();
        // A manifest equal to the release in use is not an update (lower ones are the same case).
        source.publish(&envelopes(1, 5, 6, 3));
        let outcome = run(feeds.release.check(&source, true)).unwrap();
        assert_eq!(outcome.status(), "up-to-date");
        assert!(feeds.release.staged().is_none());
        assert_eq!(versions(&feeds), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_partly_published_release_is_not_ready_and_keeps_the_current_data() {
        let dir = scratch("partial");
        let feeds = open(&dir);
        let source = Fake::default();
        // The manifest names components that are not all there yet.
        for missing in ["catalog", "banners", "rules"] {
            source.publish(&envelopes(2, 5, 6, 3));
            source.remove(missing);
            let outcome = run(feeds.release.check(&source, true)).unwrap();
            assert_eq!(outcome.status(), "not-ready", "{missing}");
            assert!(feeds.release.staged().is_none());
        }
        // A component older than the manifest names (the previous release's file).
        source.publish(&envelopes(2, 5, 6, 3));
        source.put("banners", envelopes(2, 5, 4, 3).banners);
        assert_eq!(
            run(feeds.release.check(&source, true)).unwrap().status(),
            "not-ready"
        );
        // A component newer than the manifest names (the next release is half published).
        source.publish(&envelopes(2, 5, 6, 3));
        source.put("catalog", envelopes(3, 9, 6, 3).catalog);
        assert_eq!(
            run(feeds.release.check(&source, true)).unwrap().status(),
            "not-ready"
        );
        // No manifest at all.
        source.remove("manifest");
        assert_eq!(
            run(feeds.release.check(&source, true)).unwrap().status(),
            "not-ready"
        );
        assert!(feeds.release.staged().is_none());
        assert_eq!(versions(&feeds), (1, 1, 1, 1));
        // The next check, once it is complete, stages it.
        source.publish(&envelopes(2, 5, 6, 3));
        assert_eq!(
            run(feeds.release.check(&source, true)).unwrap().status(),
            "staged"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_download_changes_nothing_and_counts_as_a_check() {
        let dir = scratch("offline");
        let feeds = open(&dir);
        let source = Fake::default();
        source.offline.store(true, Ordering::Relaxed);
        assert!(run(feeds.release.check(&source, false)).is_err());
        assert!(feeds.release.last_check().is_some());
        // The once-a-day gate now holds; a forced check goes through.
        assert_eq!(
            run(feeds.release.check(&source, false)).unwrap().status(),
            "up-to-date"
        );
        assert!(run(feeds.release.check(&source, true)).is_err());
        assert_eq!(versions(&feeds), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_newer_release_replaces_a_staged_one() {
        let dir = scratch("replace");
        let feeds = open(&dir);
        let source = Fake::default();
        source.publish(&envelopes(2, 5, 6, 3));
        run(feeds.release.check(&source, true)).unwrap();
        source.publish(&envelopes(3, 7, 6, 3));
        run(feeds.release.check(&source, true)).unwrap();
        assert_eq!(feeds.release.staged().unwrap().manifest.manifest_version, 3);
        assert_eq!(
            std::fs::read_dir(dir.join(STAGED_DIR)).unwrap().count(),
            4,
            "one release on disk, not two"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    fn install(feeds: &Feeds) -> Applied {
        let staged = feeds.release.staged().unwrap();
        feeds
            .release
            .install(&staged, &feeds.catalog, &feeds.banners, &feeds.rules)
            .unwrap()
    }

    #[test]
    fn applying_installs_all_four_together_and_survives_a_restart() {
        let dir = scratch("apply");
        let feeds = open(&dir);
        let source = Fake::default();
        source.publish(&envelopes(2, 5, 6, 3));
        run(feeds.release.check(&source, true)).unwrap();
        let staged = feeds.release.staged().unwrap();
        install(&feeds);
        assert_eq!(versions(&feeds), (2, 5, 6, 3));
        feeds.release.finish(&staged, false, 4);
        assert!(feeds.release.staged().is_none());
        let history = feeds.release.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].manifest_version, 2);
        assert_eq!(history[0].affected_pulls, 4);
        assert!(!history[0].automatic);
        assert_eq!(history[0].changes_total, 1);
        assert_eq!(versions(&open(&dir)), (2, 5, 6, 3));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_crash_before_the_manifest_is_kept_leaves_the_previous_release() {
        let dir = scratch("crash-early");
        let feeds = open(&dir);
        let source = Fake::default();
        let e = envelopes(2, 5, 6, 3);
        source.publish(&e);
        run(feeds.release.check(&source, true)).unwrap();
        // The components reached the disk, then the app died before the manifest was kept.
        let staged = feeds.release.staged().unwrap();
        feeds.catalog.cache(5, &staged.envelopes.catalog).unwrap();
        feeds.banners.cache(6, &staged.envelopes.banners).unwrap();
        feeds.rules.cache(3, &staged.envelopes.rules).unwrap();
        assert_eq!(versions(&open(&dir)), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_release_that_cannot_be_loaded_whole_is_never_loaded_in_part() {
        let dir = scratch("crash-late");
        let feeds = open(&dir);
        let source = Fake::default();
        source.publish(&envelopes(2, 5, 6, 3));
        run(feeds.release.check(&source, true)).unwrap();
        install(&feeds);
        // A component file is lost or damaged after the release became current.
        std::fs::write(dir.join("banners-cache").join("6.json"), b"damaged").unwrap();
        let reopened = open(&dir);
        assert_eq!(versions(&reopened), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_commit_puts_the_previous_release_back() {
        let dir = scratch("revert");
        let feeds = open(&dir);
        let source = Fake::default();
        source.publish(&envelopes(2, 5, 6, 3));
        run(feeds.release.check(&source, true)).unwrap();
        let applied = install(&feeds);
        assert_eq!(versions(&feeds), (2, 5, 6, 3));
        feeds
            .release
            .revert(applied, &feeds.catalog, &feeds.banners, &feeds.rules);
        assert_eq!(versions(&feeds), (1, 1, 1, 1));
        assert_eq!(versions(&open(&dir)), (1, 1, 1, 1));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_release_that_is_not_newer_is_not_installed() {
        let dir = scratch("older");
        let feeds = open(&dir);
        let source = Fake::default();
        source.publish(&envelopes(2, 5, 6, 3));
        run(feeds.release.check(&source, true)).unwrap();
        let staged = feeds.release.staged().unwrap();
        install(&feeds);
        assert!(feeds
            .release
            .install(&staged, &feeds.catalog, &feeds.banners, &feeds.rules)
            .is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn not_now_is_remembered_until_the_staged_release_goes() {
        let dir = scratch("dismiss");
        let feeds = open(&dir);
        assert_eq!(feeds.release.dismissed(), None);
        feeds.release.dismiss(2);
        assert_eq!(open(&dir).release.dismissed(), Some(2));
        feeds.release.drop_staged();
        assert_eq!(feeds.release.dismissed(), None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn history_keeps_the_latest_twenty_with_the_most_important_changes() {
        let dir = scratch("history");
        let feeds = open(&dir);
        let source = Fake::default();
        for n in 0..22u64 {
            source.publish(&envelopes(2 + n, 5, 6, 3));
            let staged = Arc::new(
                feeds
                    .release
                    .verify_release(&envelopes(2 + n, 5, 6, 3), false)
                    .ok()
                    .unwrap(),
            );
            feeds.release.finish(&staged, true, 0);
        }
        let history = feeds.release.history();
        assert_eq!(history.len(), HISTORY_KEEP);
        assert_eq!(history[0].manifest_version, 23);
        assert!(history[0].automatic);
        let _ = std::fs::remove_dir_all(dir);
    }
}
