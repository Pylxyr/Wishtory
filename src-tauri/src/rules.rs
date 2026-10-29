//! Game rules that change over time: pity limits, the permanent pool, items that were also
//! featured at some point, and Zenless Custom Search. They used to be tables in `pity.rs`, which
//! meant every rule change needed a new app release. They now live in `catalog/rules.json`,
//! bundled with the app and updated at runtime through the same signed feed as the catalog.
//!
//! Soft pity values are community estimates (the games do not publish them); hard pity values
//! are the in-game guarantees. The Chronicled Wish values follow secondary sources (hard 90,
//! soft near 74) and are the least certain entry.

use crate::{
    catalog::{is_clock, Feed, FeedStore},
    models::Game,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

const MAX_RULES_BYTES: usize = 1024 * 1024;
const MAX_ITEMS_PER_SET: usize = 400;
const MAX_WINDOWS: usize = 2_000;

pub type RulesStore = FeedStore<RuleSet>;

/// One banner's pity mechanics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BannerRule {
    /// Matches `pity::banner_key` ("character", "weapon", ...).
    pub key: String,
    pub label: String,
    pub hard: u32,
    #[serde(default)]
    pub soft: Option<u32>,
    /// Percent chance that a non-guaranteed 5★ is featured (50 for characters, 75 for weapons).
    /// `Some` tracks the win / loss / guarantee state against the permanent pool.
    #[serde(default)]
    pub odds: Option<u8>,
}

/// Language-independent ids (and Paimon.moe slugs) plus English names, for records that carry no
/// `item_id`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemSet {
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
}

/// The permanent pool's 5★ items.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StandardPool {
    pub character: ItemSet,
    /// Weapons, Light Cones or W-Engines.
    pub weapon: ItemSet,
}

/// A permanent-pool item that was also a rate-up item on an event banner. A pull inside the
/// window (server-local dates, both inclusive) is the featured result, never a lost 50/50.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FeaturedWindow {
    pub id: String,
    pub name: String,
    pub from: String,
    pub to: String,
}

/// Zenless Custom Search lets players keep limited agents, and their signature W-Engines, in the
/// off-banner pool from `from` (server-local time) onwards.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomSearch {
    pub from: String,
    pub engines: Vec<EngineAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EngineAgent {
    pub engine: String,
    pub agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GameRules {
    pub banners: Vec<BannerRule>,
    pub standard: StandardPool,
    #[serde(default)]
    pub featured_windows: Vec<FeaturedWindow>,
    #[serde(default)]
    pub custom_search: Option<CustomSearch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    pub schema_version: u32,
    pub rules_version: u64,
    pub generated_at: String,
    games: BTreeMap<String, GameRules>,
}

fn is_day(value: &str) -> bool {
    value.len() == 10 && is_clock(&format!("{value} 00:00:00"))
}

fn sane(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && text.len() <= max
}

impl RuleSet {
    pub fn bundled() -> Self {
        Self::parse_and_validate(include_bytes!("../../catalog/rules.json"))
            .expect("bundled game rules must validate")
    }

    /// Rules of one game. Validation guarantees every supported game is present.
    pub fn game(&self, game: Game) -> &GameRules {
        self.games
            .get(game.as_str())
            .expect("validated rule set covers every game")
    }

    #[cfg(test)]
    pub fn game_mut(&mut self, game: Game) -> &mut GameRules {
        self.games
            .get_mut(game.as_str())
            .expect("every game exists")
    }

    pub fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_RULES_BYTES {
            return Err("rules size is empty or exceeds 1 MiB".into());
        }
        let set: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("invalid rules JSON: {e}"))?;
        if set.schema_version != 1 {
            return Err("unsupported rules schema".into());
        }
        if set.rules_version == 0 || set.generated_at.trim().is_empty() {
            return Err("rules version and generation time are required".into());
        }
        if set.games.len() != Game::ALL.len()
            || Game::ALL
                .iter()
                .any(|g| !set.games.contains_key(g.as_str()))
        {
            return Err("rules must cover exactly the supported games".into());
        }
        for rules in set.games.values() {
            rules.validate()?;
        }
        Ok(set)
    }
}

impl GameRules {
    fn validate(&self) -> Result<(), String> {
        if self.banners.is_empty() || self.banners.len() > 16 {
            return Err("rules have an invalid banner count".into());
        }
        let mut keys = HashSet::new();
        for b in &self.banners {
            let odds_ok = b.odds.is_none_or(|o| (1..=99).contains(&o));
            let soft_ok = b.soft.is_none_or(|s| s >= 1 && s < b.hard);
            if !sane(&b.key, 32)
                || !sane(&b.label, 80)
                || !(1..=200).contains(&b.hard)
                || !odds_ok
                || !soft_ok
                || !keys.insert(b.key.as_str())
            {
                return Err("rules contain an invalid or duplicate banner".into());
            }
        }
        for set in [&self.standard.character, &self.standard.weapon] {
            if set.ids.len() > MAX_ITEMS_PER_SET
                || set.names.len() > MAX_ITEMS_PER_SET
                || set.ids.iter().chain(&set.names).any(|v| !sane(v, 160))
            {
                return Err("rules contain an invalid permanent-pool list".into());
            }
        }
        if self.featured_windows.len() > MAX_WINDOWS {
            return Err("rules contain too many featured windows".into());
        }
        for w in &self.featured_windows {
            if !sane(&w.id, 80)
                || !sane(&w.name, 160)
                || !is_day(&w.from)
                || !is_day(&w.to)
                || w.from > w.to
            {
                return Err("rules contain an invalid featured window".into());
            }
        }
        if let Some(search) = &self.custom_search {
            if !is_clock(&search.from)
                || search.engines.len() > MAX_ITEMS_PER_SET
                || search
                    .engines
                    .iter()
                    .any(|e| !sane(&e.engine, 80) || !sane(&e.agent, 80))
            {
                return Err("rules contain an invalid Custom Search entry".into());
            }
        }
        Ok(())
    }
}

impl Feed for RuleSet {
    const LABEL: &'static str = "rules";
    const ASSET: &'static str = "rules";
    const MAX_BYTES: usize = MAX_RULES_BYTES;
    fn bundled() -> Self {
        RuleSet::bundled()
    }
    fn parse_and_validate(bytes: &[u8]) -> Result<Self, String> {
        RuleSet::parse_and_validate(bytes)
    }
    fn version(&self) -> u64 {
        self.rules_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_text() -> String {
        String::from_utf8(include_bytes!("../../catalog/rules.json").to_vec()).unwrap()
    }

    #[test]
    fn bundled_rules_cover_every_game_and_banner_key() {
        let rules = RuleSet::bundled();
        for game in Game::ALL {
            let game_rules = rules.game(game);
            // Every pool the API can report must have a banner row to be counted in.
            for gacha_type in game.gacha_types().iter().chain(&["400"]) {
                if let Some(key) = crate::pity::banner_key(game, gacha_type) {
                    assert!(
                        game_rules.banners.iter().any(|b| b.key == key),
                        "{}: no banner rule for {key}",
                        game.as_str()
                    );
                }
            }
        }
        assert!(rules.game(Game::Nap).custom_search.is_some());
        assert!(rules.game(Game::Hk4e).custom_search.is_none());
        assert!(!rules.game(Game::Hk4e).featured_windows.is_empty());
    }

    #[test]
    fn rejects_malformed_rules() {
        let good = bundled_text();
        assert!(RuleSet::parse_and_validate(good.as_bytes()).is_ok());
        for (from, to) in [
            ("\"schema_version\": 1", "\"schema_version\": 2"),
            ("\"rules_version\": 1", "\"rules_version\": 0"),
            ("\"hard\": 80", "\"hard\": 0"),
            ("\"soft\": 74", "\"soft\": 99"),
            ("\"odds\": 50", "\"odds\": 100"),
            ("\"from\": \"2020-09-28\"", "\"from\": \"2020-13-28\""),
            (
                "\"from\": \"2026-07-28 00:00:00\"",
                "\"from\": \"yesterday\"",
            ),
            ("\"hk4e\": {", "\"other\": {"),
        ] {
            let bad = good.replacen(from, to, 1);
            assert_ne!(bad, good, "fixture no longer contains {from}");
            assert!(
                RuleSet::parse_and_validate(bad.as_bytes()).is_err(),
                "accepted {to}"
            );
        }
        let unknown = good.replacen(
            "\"schema_version\": 1,",
            "\"schema_version\": 1, \"extra\": 1,",
            1,
        );
        assert!(RuleSet::parse_and_validate(unknown.as_bytes()).is_err());
        assert!(RuleSet::parse_and_validate(&vec![b' '; MAX_RULES_BYTES + 1]).is_err());
    }
}
