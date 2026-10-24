use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
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
}
