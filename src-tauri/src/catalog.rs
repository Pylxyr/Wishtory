use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        RwLock,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const MAX_CATALOG_BYTES: usize = 8 * 1024 * 1024;
const MAX_ENVELOPE_BYTES: usize = MAX_CATALOG_BYTES * 4 / 3 + 2048;
const MAX_ITEMS: usize = 20_000;
const UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const CATALOG_URL: &str =
    "https://github.com/Pylxyr/Wishtory/releases/download/catalog-v1/catalog.json";
const PINNED_PUBLIC_KEY: &str = include_str!("../../catalog/catalog-public-key.txt");

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
}

impl Catalog {
    pub fn bundled() -> Self {
        Self::parse_and_validate(include_bytes!("../../catalog/catalog.json"))
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

    pub fn verify_envelope(envelope: &[u8]) -> Result<(Self, Vec<u8>), String> {
        let public_key = BASE64
            .decode(PINNED_PUBLIC_KEY.trim())
            .map_err(|_| "invalid pinned catalog key".to_string())?;
        Self::verify_envelope_with_key(envelope, &public_key)
    }

    fn verify_envelope_with_key(
        envelope: &[u8],
        public_key: &[u8],
    ) -> Result<(Self, Vec<u8>), String> {
        if envelope.is_empty() || envelope.len() > MAX_ENVELOPE_BYTES {
            return Err("catalog envelope is empty or exceeds the size limit".into());
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SignedEnvelope {
            payload: String,
            signature: String,
        }
        let envelope: SignedEnvelope = serde_json::from_slice(envelope)
            .map_err(|e| format!("invalid catalog envelope: {e}"))?;
        let payload = BASE64
            .decode(envelope.payload)
            .map_err(|_| "invalid catalog payload encoding".to_string())?;
        let signature = BASE64
            .decode(envelope.signature.trim())
            .map_err(|_| "invalid catalog signature encoding".to_string())?;
        if !verify_signature_with_key(&payload, &signature, public_key) {
            return Err("catalog signature verification failed".into());
        }
        let catalog = Self::parse_and_validate(&payload)?;
        Ok((catalog, payload))
    }

    pub fn item_by_paimon_key(&self, key: &str) -> Option<&CatalogItem> {
        self.items
            .iter()
            .find(|item| item.game == "hk4e" && item.paimon_key.as_deref() == Some(key))
    }

    pub fn item_by_english_name(&self, game: &str, name: &str) -> Option<&CatalogItem> {
        let normalized = normalize_name(name);
        self.items.iter().find(|item| {
            item.game == game
                && (normalize_name(&item.name) == normalized
                    || item.aliases.iter().any(|a| normalize_name(a) == normalized))
        })
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

fn verify_signature_with_key(bytes: &[u8], signature: &[u8], public_key: &[u8]) -> bool {
    UnparsedPublicKey::new(&ED25519, public_key)
        .verify(bytes, signature)
        .is_ok()
}

pub struct CatalogStore {
    catalog: RwLock<Catalog>,
    cache_dir: PathBuf,
    check_file: PathBuf,
    checking: AtomicBool,
}

impl CatalogStore {
    pub fn open(app_data: &Path) -> Self {
        let cache_dir = app_data.join("catalog-cache");
        let cached = std::fs::read_dir(&cache_dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|entry| {
                let path = entry.path();
                let metadata = std::fs::metadata(&path).ok()?;
                if !metadata.is_file() || metadata.len() > MAX_ENVELOPE_BYTES as u64 {
                    return None;
                }
                std::fs::read(path).ok()
            })
            .filter_map(|bytes| {
                Catalog::verify_envelope(&bytes)
                    .ok()
                    .map(|(catalog, _)| catalog)
            })
            .max_by_key(|catalog| catalog.catalog_version);
        let embedded = Catalog::bundled();
        let current = cached
            .filter(|c| c.catalog_version >= embedded.catalog_version)
            .unwrap_or(embedded);
        Self {
            catalog: RwLock::new(current),
            cache_dir,
            check_file: app_data.join("catalog-last-check"),
            checking: AtomicBool::new(false),
        }
    }

    pub fn current(&self) -> Catalog {
        self.catalog
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub async fn refresh(&self) -> Result<String, String> {
        if self.checking.swap(true, Ordering::AcqRel) {
            return Ok("already-checking".into());
        }
        let _checking = ResetBool(&self.checking);
        if !self.check_due() {
            return Ok("up-to-date".into());
        }
        // Count a failed download as a check too, so an offline start is not retried on every launch.
        let _ = std::fs::write(&self.check_file, unix_now().to_string());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| e.to_string())?;
        let mut response = client
            .get(CATALOG_URL)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("catalog download returned {}", response.status()));
        }
        if response
            .content_length()
            .is_some_and(|len| len > MAX_ENVELOPE_BYTES as u64)
        {
            return Err("catalog download exceeds the size limit".into());
        }
        let mut envelope_bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if envelope_bytes.len() + chunk.len() > MAX_ENVELOPE_BYTES {
                return Err("catalog download exceeds the size limit".into());
            }
            envelope_bytes.extend_from_slice(&chunk);
        }
        self.install_envelope(&envelope_bytes)
    }

    fn install_envelope(&self, envelope_bytes: &[u8]) -> Result<String, String> {
        let (incoming, _) = Catalog::verify_envelope(envelope_bytes)?;
        let current_version = self
            .catalog
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .catalog_version;
        if incoming.catalog_version <= current_version {
            let _ = std::fs::write(&self.check_file, unix_now().to_string());
            return Ok("up-to-date".into());
        }
        write_versioned(&self.cache_dir, incoming.catalog_version, envelope_bytes)?;
        *self.catalog.write().unwrap_or_else(|e| e.into_inner()) = incoming;
        let _ = std::fs::write(&self.check_file, unix_now().to_string());
        Ok("updated".into())
    }

    fn check_due(&self) -> bool {
        let last = std::fs::read_to_string(&self.check_file)
            .ok()
            .and_then(|v| v.parse::<u64>().ok());
        last.is_none_or(|v| unix_now().saturating_sub(v) >= UPDATE_INTERVAL.as_secs())
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

fn write_versioned(dir: &Path, version: u64, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let target = dir.join(format!("{version}.json"));
    if target.exists() {
        if Catalog::verify_envelope(&std::fs::read(&target).map_err(|e| e.to_string())?).is_ok() {
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

const MAX_BANNER_BYTES: usize = 2 * 1024 * 1024;
const MAX_BANNER_ENVELOPE_BYTES: usize = MAX_BANNER_BYTES * 4 / 3 + 2048;
const MAX_BANNERS: usize = 5_000;
const BANNERS_URL: &str =
    "https://github.com/Pylxyr/Wishtory/releases/download/catalog-v1/banners.json";

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

/// What a banner refresh changed, by banner id. Saved so the app can show it after a restart.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BannerUpdate {
    /// "updated", "up-to-date" or "already-checking".
    pub status: String,
    pub banners_version: u64,
    /// Unix seconds when this update was installed (0 when nothing was installed).
    pub installed_at: u64,
    pub added: Vec<String>,
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

fn is_clock(value: &str) -> bool {
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
        Self::parse_and_validate(include_bytes!("../../catalog/banners.json"))
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

    pub fn verify_envelope(envelope: &[u8]) -> Result<Self, String> {
        let public_key = BASE64
            .decode(PINNED_PUBLIC_KEY.trim())
            .map_err(|_| "invalid pinned catalog key".to_string())?;
        Self::verify_envelope_with_key(envelope, &public_key)
    }

    fn verify_envelope_with_key(envelope: &[u8], public_key: &[u8]) -> Result<Self, String> {
        if envelope.is_empty() || envelope.len() > MAX_BANNER_ENVELOPE_BYTES {
            return Err("banner envelope is empty or exceeds the size limit".into());
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct SignedEnvelope {
            payload: String,
            signature: String,
        }
        let envelope: SignedEnvelope = serde_json::from_slice(envelope)
            .map_err(|e| format!("invalid banner envelope: {e}"))?;
        let payload = BASE64
            .decode(envelope.payload)
            .map_err(|_| "invalid banner payload encoding".to_string())?;
        let signature = BASE64
            .decode(envelope.signature.trim())
            .map_err(|_| "invalid banner signature encoding".to_string())?;
        if !verify_signature_with_key(&payload, &signature, public_key) {
            return Err("banner signature verification failed".into());
        }
        Self::parse_and_validate(&payload)
    }

    /// Banner ids that were added, changed (any field differs) or removed going from `self`
    /// to `newer`.
    fn diff(&self, newer: &BannerSet) -> (Vec<String>, Vec<String>, Vec<String>) {
        let old: HashMap<&str, &BannerEntry> =
            self.banners.iter().map(|b| (b.id.as_str(), b)).collect();
        let new: HashMap<&str, &BannerEntry> =
            newer.banners.iter().map(|b| (b.id.as_str(), b)).collect();
        let mut added = Vec::new();
        let mut changed = Vec::new();
        for banner in &newer.banners {
            match old.get(banner.id.as_str()) {
                None => added.push(banner.id.clone()),
                Some(previous) if *previous != banner => changed.push(banner.id.clone()),
                Some(_) => {}
            }
        }
        let removed = self
            .banners
            .iter()
            .filter(|b| !new.contains_key(b.id.as_str()))
            .map(|b| b.id.clone())
            .collect();
        (added, changed, removed)
    }
}

pub struct BannerStore {
    set: RwLock<BannerSet>,
    cache_dir: PathBuf,
    check_file: PathBuf,
    update_file: PathBuf,
    checking: AtomicBool,
}

impl BannerStore {
    pub fn open(app_data: &Path) -> Self {
        let cache_dir = app_data.join("banners-cache");
        let cached = std::fs::read_dir(&cache_dir)
            .ok()
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            .filter_map(|entry| {
                let path = entry.path();
                let metadata = std::fs::metadata(&path).ok()?;
                if !metadata.is_file() || metadata.len() > MAX_BANNER_ENVELOPE_BYTES as u64 {
                    return None;
                }
                std::fs::read(path).ok()
            })
            .filter_map(|bytes| BannerSet::verify_envelope(&bytes).ok())
            .max_by_key(|set| set.banners_version);
        let embedded = BannerSet::bundled();
        let current = cached
            .filter(|set| set.banners_version >= embedded.banners_version)
            .unwrap_or(embedded);
        Self {
            set: RwLock::new(current),
            cache_dir,
            check_file: app_data.join("banners-last-check"),
            update_file: app_data.join("banners-last-update.json"),
            checking: AtomicBool::new(false),
        }
    }

    pub fn current(&self) -> BannerSet {
        self.set.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The most recent installed update, if any (kept across restarts).
    pub fn last_update(&self) -> Option<BannerUpdate> {
        serde_json::from_slice(&std::fs::read(&self.update_file).ok()?).ok()
    }

    /// Downloads the signed banner list. `force` skips the once-a-day gate (the update button).
    pub async fn refresh(&self, force: bool) -> Result<BannerUpdate, String> {
        let status_only = |status: &str, store: &Self| BannerUpdate {
            status: status.into(),
            banners_version: store.current().banners_version,
            ..BannerUpdate::default()
        };
        if self.checking.swap(true, Ordering::AcqRel) {
            return Ok(status_only("already-checking", self));
        }
        let _checking = ResetBool(&self.checking);
        if !force && !self.check_due() {
            return Ok(status_only("up-to-date", self));
        }
        // Count a failed download as a check too, so an offline start is not retried on every launch.
        let _ = std::fs::write(&self.check_file, unix_now().to_string());
        let envelope = download_limited(BANNERS_URL, MAX_BANNER_ENVELOPE_BYTES).await?;
        self.install_envelope(&envelope)
    }

    fn install_envelope(&self, envelope_bytes: &[u8]) -> Result<BannerUpdate, String> {
        let incoming = BannerSet::verify_envelope(envelope_bytes)?;
        self.install_verified(incoming, envelope_bytes)
    }

    fn install_verified(
        &self,
        incoming: BannerSet,
        envelope_bytes: &[u8],
    ) -> Result<BannerUpdate, String> {
        let current = self.current();
        let _ = std::fs::write(&self.check_file, unix_now().to_string());
        if incoming.banners_version <= current.banners_version {
            return Ok(BannerUpdate {
                status: "up-to-date".into(),
                banners_version: current.banners_version,
                ..BannerUpdate::default()
            });
        }
        write_versioned(&self.cache_dir, incoming.banners_version, envelope_bytes)?;
        let (added, changed, removed) = current.diff(&incoming);
        let update = BannerUpdate {
            status: "updated".into(),
            banners_version: incoming.banners_version,
            installed_at: unix_now(),
            added,
            changed,
            removed,
        };
        *self.set.write().unwrap_or_else(|e| e.into_inner()) = incoming;
        if let Ok(json) = serde_json::to_vec(&update) {
            let _ = std::fs::write(&self.update_file, json);
        }
        Ok(update)
    }

    fn check_due(&self) -> bool {
        let last = std::fs::read_to_string(&self.check_file)
            .ok()
            .and_then(|v| v.parse::<u64>().ok());
        last.is_none_or(|v| unix_now().saturating_sub(v) >= UPDATE_INTERVAL.as_secs())
    }
}

async fn download_limited(url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
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
            Catalog::verify_envelope_with_key(envelope.as_bytes(), key.public_key().as_ref())
                .unwrap();
        assert_eq!(catalog.catalog_version, 1);
        assert_eq!(payload, SAMPLE);
        let mut tampered: serde_json::Value = serde_json::from_str(&envelope).unwrap();
        let mut changed_payload = SAMPLE.to_vec();
        changed_payload[10] ^= 1;
        tampered["payload"] = serde_json::Value::from(BASE64.encode(changed_payload));
        assert!(Catalog::verify_envelope_with_key(
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
        write_versioned(&dir, 7, b"complete signed envelope bytes").unwrap();
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"complete signed envelope bytes"
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn test_store(app_data: &Path) -> CatalogStore {
        CatalogStore {
            catalog: RwLock::new(Catalog::bundled()),
            cache_dir: app_data.join("catalog-cache"),
            check_file: app_data.join("catalog-last-check"),
            checking: AtomicBool::new(false),
        }
    }

    #[test]
    fn newer_valid_cache_wins_but_invalid_cache_falls_back_to_bundle() {
        let app_data =
            std::env::temp_dir().join(format!("wishtory-catalog-open-{}", std::process::id()));
        let cache = app_data.join("catalog-cache");
        let _ = std::fs::remove_dir_all(&app_data);
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(
            cache.join("2.json"),
            include_bytes!("../../catalog/catalog-test-envelope.json"),
        )
        .unwrap();
        assert_eq!(CatalogStore::open(&app_data).current().catalog_version, 2);
        std::fs::write(cache.join("2.json"), b"tampered").unwrap();
        assert_eq!(
            CatalogStore::open(&app_data).current().catalog_version,
            Catalog::bundled().catalog_version
        );
        std::fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn daily_check_is_gated_and_invalid_feed_does_not_replace_current_data() {
        let app_data =
            std::env::temp_dir().join(format!("wishtory-catalog-due-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&app_data);
        std::fs::create_dir_all(&app_data).unwrap();
        let store = test_store(&app_data);
        assert!(store.check_due());
        std::fs::write(&store.check_file, unix_now().to_string()).unwrap();
        assert!(!store.check_due());
        std::fs::write(
            &store.check_file,
            unix_now()
                .saturating_sub(UPDATE_INTERVAL.as_secs() + 1)
                .to_string(),
        )
        .unwrap();
        assert!(store.check_due());
        let before = store.current();
        assert!(store.install_envelope(b"not a signed feed").is_err());
        assert_eq!(store.current(), before);
        let _ = std::fs::remove_dir_all(app_data);
    }

    #[test]
    fn valid_new_envelope_is_cached_before_becoming_current() {
        let app_data =
            std::env::temp_dir().join(format!("wishtory-catalog-install-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&app_data);
        std::fs::create_dir_all(&app_data).unwrap();
        let store = test_store(&app_data);
        // Model an older app bundle so the version-2 fixture is a genuinely new feed.
        store.catalog.write().unwrap().catalog_version = 1;
        let envelope = include_bytes!("../../catalog/catalog-test-envelope.json");
        assert_eq!(store.install_envelope(envelope).unwrap(), "updated");
        assert_eq!(store.current().catalog_version, 2);
        assert!(store.cache_dir.join("2.json").is_file());
        let _ = std::fs::remove_dir_all(app_data);
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
        let set =
            BannerSet::verify_envelope_with_key(envelope.as_bytes(), key.public_key().as_ref())
                .unwrap();
        assert_eq!(set.banners_version, 1);
        let mut changed_payload = BANNER_SAMPLE.to_vec();
        changed_payload[10] ^= 1;
        let mut tampered: serde_json::Value = serde_json::from_str(&envelope).unwrap();
        tampered["payload"] = serde_json::Value::from(BASE64.encode(changed_payload));
        assert!(BannerSet::verify_envelope_with_key(
            tampered.to_string().as_bytes(),
            key.public_key().as_ref()
        )
        .is_err());
        let other = ring::signature::Ed25519KeyPair::from_seed_unchecked(&[8u8; 32]).unwrap();
        assert!(BannerSet::verify_envelope_with_key(
            envelope.as_bytes(),
            other.public_key().as_ref()
        )
        .is_err());
    }

    #[test]
    fn banner_update_records_added_changed_and_removed_ids() {
        let app_data =
            std::env::temp_dir().join(format!("wishtory-banners-install-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&app_data);
        std::fs::create_dir_all(&app_data).unwrap();
        let store = BannerStore::open(&app_data);
        *store.set.write().unwrap() = BannerSet::parse_and_validate(BANNER_SAMPLE).unwrap();
        let newer = String::from_utf8(BANNER_SAMPLE.to_vec())
            .unwrap()
            .replace("\"banners_version\":1", "\"banners_version\":2")
            .replace("\"name\":\"Event A\"", "\"name\":\"Event A renamed\"")
            .replace("{\"id\":\"nap-character-b\"", "{\"id\":\"nap-character-c\"");
        let incoming = BannerSet::parse_and_validate(newer.as_bytes()).unwrap();
        let update = store
            .install_verified(incoming.clone(), b"signed bytes")
            .unwrap();
        assert_eq!(update.status, "updated");
        assert_eq!(update.banners_version, 2);
        assert_eq!(update.added, vec!["nap-character-c".to_string()]);
        assert_eq!(update.changed, vec!["hk4e-character-a".to_string()]);
        assert_eq!(update.removed, vec!["nap-character-b".to_string()]);
        assert_eq!(store.current(), incoming);
        assert_eq!(store.last_update(), Some(update));
        assert!(store.cache_dir.join("2.json").is_file());
        // The same version is not installed twice and leaves the saved update untouched.
        let again = store.install_verified(incoming, b"signed bytes").unwrap();
        assert_eq!(again.status, "up-to-date");
        assert_eq!(store.last_update().unwrap().status, "updated");
        let _ = std::fs::remove_dir_all(app_data);
    }
}
