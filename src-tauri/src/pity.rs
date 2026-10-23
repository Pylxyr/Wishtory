//! Pity counters, 50/50 state and basic statistics, computed from stored pulls.
//!
//! The tables below (hard/soft pity, standard-pool item ids) are game data that changes over
//! time. Soft pity values are community estimates (the games do not publish them); hard pity
//! values are the in-game guarantees. Re-check both whenever a game changes its banner rules.
//! The Chronicled Wish values follow secondary sources (hard 90, soft near 74) and are the least
//! certain entry in the table.

use crate::models::{Game, Pull};
use serde::Serialize;

struct Banner {
    key: &'static str,
    label: &'static str,
    hard: u32,
    soft: Option<u32>,
    /// Track the 50/50 "guaranteed" state (needs a standard-5★ id table).
    fifty: bool,
}

fn banners(game: Game) -> &'static [Banner] {
    match game {
        Game::Hk4e => &[
            Banner { key: "character", label: "Character Event Wish", hard: 90, soft: Some(74), fifty: true },
            Banner { key: "weapon", label: "Weapon Event Wish", hard: 80, soft: Some(63), fifty: false },
            Banner { key: "standard", label: "Standard Wish", hard: 90, soft: Some(74), fifty: false },
            Banner { key: "chronicled", label: "Chronicled Wish", hard: 90, soft: Some(74), fifty: false },
            Banner { key: "beginner", label: "Beginners' Wish", hard: 90, soft: None, fifty: false },
        ],
        Game::Hkrpg => &[
            Banner { key: "character", label: "Character Event Warp", hard: 90, soft: Some(74), fifty: true },
            Banner { key: "weapon", label: "Light Cone Event Warp", hard: 80, soft: Some(66), fifty: false },
            Banner { key: "standard", label: "Stellar Warp", hard: 90, soft: Some(74), fifty: false },
            Banner { key: "beginner", label: "Departure Warp", hard: 50, soft: None, fifty: false },
        ],
        Game::Nap => &[
            // fifty: false until standard-agent item ids are added to `standard_fives`.
            Banner { key: "character", label: "Exclusive Channel", hard: 90, soft: Some(74), fifty: false },
            Banner { key: "weapon", label: "W-Engine Channel", hard: 80, soft: Some(65), fifty: false },
            Banner { key: "standard", label: "Standard Channel", hard: 90, soft: Some(74), fifty: false },
            Banner { key: "bangboo", label: "Bangboo Channel", hard: 80, soft: Some(65), fifty: false },
        ],
    }
}

/// Language-independent item ids of permanent-pool 5★ characters (used for 50/50 state).
fn standard_fives(game: Game) -> &'static [&'static str] {
    match game {
        Game::Hk4e => &["10000003", "10000016", "10000035", "10000041", "10000042", "10000071", "10000079"],
        Game::Hkrpg => &["1003", "1004", "1101", "1104", "1107", "1209", "1211"],
        Game::Nap => &[],
    }
}

pub fn banner_key(game: Game, gacha_type: &str) -> Option<&'static str> {
    Some(match (game, gacha_type) {
        (Game::Hk4e, "100") => "beginner",
        (Game::Hk4e, "200") => "standard",
        (Game::Hk4e, "301" | "400") => "character",
        (Game::Hk4e, "302") => "weapon",
        (Game::Hk4e, "500") => "chronicled",
        (Game::Hkrpg, "1") => "standard",
        (Game::Hkrpg, "2") => "beginner",
        (Game::Hkrpg, "11") => "character",
        (Game::Hkrpg, "12") => "weapon",
        (Game::Nap, "1") => "standard",
        (Game::Nap, "2") => "character",
        (Game::Nap, "3") => "weapon",
        (Game::Nap, "5") => "bangboo",
        _ => return None,
    })
}

#[derive(Serialize)]
pub struct FiveStar { pub name: String, pub pity: u32, pub time: String, pub off_banner: Option<bool> }

#[derive(Serialize)]
pub struct BannerPity {
    pub banner: String,
    pub label: String,
    pub pity: u32,
    pub hard_pity: u32,
    pub soft_pity: Option<u32>,
    pub guaranteed: Option<bool>,
    pub total: u32,
    pub five_stars: u32,
    pub four_stars: u32,
    pub avg_five_pity: Option<f32>,
    pub recent_fives: Vec<FiveStar>,
}

/// `pulls` must be sorted oldest → newest (Db::pulls_for does this).
pub fn compute(game: Game, pulls: &[Pull]) -> Vec<BannerPity> {
    let std5 = standard_fives(game);
    banners(game)
        .iter()
        .filter_map(|b| {
            let (mut counter, mut total, mut fives, mut fours, mut sum) = (0u32, 0u32, 0u32, 0u32, 0u32);
            let mut guaranteed = if b.fifty { Some(false) } else { None };
            let mut recent = Vec::new();
            for p in pulls.iter().filter(|p| banner_key(game, &p.gacha_type) == Some(b.key)) {
                counter += 1;
                total += 1;
                match p.rank_type {
                    4 => fours += 1,
                    5 => {
                        fives += 1;
                        sum += counter;
                        // Off-banner 5★ => next 5★ is guaranteed featured. (Mechanics such as
                        // Genshin's "Capturing Radiance" are not modelled.)
                        let off = if b.fifty { Some(std5.contains(&p.item_id.as_str())) } else { None };
                        if b.fifty { guaranteed = off; }
                        recent.push(FiveStar { name: p.name.clone(), pity: counter, time: p.time.clone(), off_banner: off });
                        counter = 0;
                    }
                    _ => {}
                }
            }
            if total == 0 && b.key == "beginner" { return None; }
            recent.reverse();
            recent.truncate(10);
            Some(BannerPity {
                banner: b.key.into(), label: b.label.into(), pity: counter, hard_pity: b.hard, soft_pity: b.soft,
                guaranteed, total, five_stars: fives, four_stars: fours,
                avg_five_pity: if fives > 0 { Some(sum as f32 / fives as f32) } else { None },
                recent_fives: recent,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull(game: &str, gt: &str, id: u32, rank: u8, item: &str) -> Pull {
        Pull {
            id: id.to_string(), game: game.into(), uid: "1".into(), gacha_type: gt.into(),
            gacha_id: String::new(), item_id: item.into(), name: "n".into(), item_type: "Character".into(),
            rank_type: rank, time: format!("2024-01-01 00:{:02}:{:02}", id / 60, id % 60), lang: "en".into(), count: 1,
        }
    }
    fn p(id: u32, rank: u8, item: &str) -> Pull { pull("hk4e", "301", id, rank, item) }
    fn find<'a>(r: &'a [BannerPity], key: &str) -> &'a BannerPity { r.iter().find(|b| b.banner == key).unwrap() }

    #[test]
    fn counts_pity_and_fifty_fifty() {
        // 3 pulls, a standard-pool 5* (loses 50/50), then 2 more pulls.
        let pulls = vec![p(1, 3, "a"), p(2, 4, "b"), p(3, 5, "10000003"), p(4, 3, "c"), p(5, 3, "d")];
        let r = compute(Game::Hk4e, &pulls);
        let c = find(&r, "character");
        assert_eq!((c.pity, c.total, c.five_stars, c.four_stars), (2, 5, 1, 1));
        assert_eq!(c.guaranteed, Some(true));
        assert_eq!(c.avg_five_pity, Some(3.0));
    }

    #[test]
    fn winning_the_fifty_clears_the_guarantee() {
        let pulls = vec![p(1, 5, "10000003"), p(2, 3, "x"), p(3, 5, "10000099")];
        let r = compute(Game::Hk4e, &pulls);
        let c = find(&r, "character");
        assert_eq!((c.pity, c.guaranteed), (0, Some(false)));
        assert_eq!(c.recent_fives[0].off_banner, Some(false)); // newest first
        assert_eq!(c.recent_fives[1].off_banner, Some(true));
    }

    #[test]
    fn second_character_banner_shares_pity_with_the_first() {
        let pulls = vec![pull("hk4e", "301", 1, 3, "a"), pull("hk4e", "400", 2, 3, "b"), pull("hk4e", "301", 3, 3, "c")];
        let r = compute(Game::Hk4e, &pulls);
        assert_eq!((find(&r, "character").pity, find(&r, "character").total), (3, 3));
    }

    #[test]
    fn empty_history_shows_banners_with_zero_and_hides_beginner() {
        let r = compute(Game::Hkrpg, &[]);
        assert!(r.iter().all(|b| b.banner != "beginner"));
        assert!(r.iter().all(|b| b.pity == 0 && b.total == 0 && b.avg_five_pity.is_none()));
    }

    #[test]
    fn star_rail_tracks_fifty_fifty_and_ignores_unknown_banners() {
        let pulls = vec![pull("hkrpg", "11", 1, 5, "1003"), pull("hkrpg", "99", 2, 5, "1")];
        let r = compute(Game::Hkrpg, &pulls);
        let c = find(&r, "character");
        assert_eq!((c.five_stars, c.guaranteed), (1, Some(true)));
        assert_eq!(r.iter().map(|b| b.total).sum::<u32>(), 1, "unknown gacha_type must not be counted");
    }

    #[test]
    fn weapon_banner_has_no_fifty_state() {
        let r = compute(Game::Hk4e, &[pull("hk4e", "302", 1, 5, "15502")]);
        assert_eq!(find(&r, "weapon").guaranteed, None);
    }
}
