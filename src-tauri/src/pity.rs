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
            Banner {
                key: "character",
                label: "Character Event Wish",
                hard: 90,
                soft: Some(74),
                fifty: true,
            },
            Banner {
                key: "weapon",
                label: "Weapon Event Wish",
                hard: 80,
                soft: Some(63),
                fifty: false,
            },
            Banner {
                key: "standard",
                label: "Standard Wish",
                hard: 90,
                soft: Some(74),
                fifty: false,
            },
            Banner {
                key: "chronicled",
                label: "Chronicled Wish",
                hard: 90,
                soft: Some(74),
                fifty: false,
            },
            Banner {
                key: "beginner",
                label: "Beginners' Wish",
                hard: 90,
                soft: None,
                fifty: false,
            },
        ],
        Game::Hkrpg => &[
            Banner {
                key: "character",
                label: "Character Event Warp",
                hard: 90,
                soft: Some(74),
                fifty: true,
            },
            Banner {
                key: "weapon",
                label: "Light Cone Event Warp",
                hard: 80,
                soft: Some(66),
                fifty: false,
            },
            Banner {
                key: "standard",
                label: "Stellar Warp",
                hard: 90,
                soft: Some(74),
                fifty: false,
            },
            Banner {
                key: "beginner",
                label: "Departure Warp",
                hard: 50,
                soft: None,
                fifty: false,
            },
        ],
        Game::Nap => &[
            Banner {
                key: "character",
                label: "Exclusive Channel",
                hard: 90,
                soft: Some(74),
                fifty: true,
            },
            Banner {
                key: "weapon",
                label: "W-Engine Channel",
                hard: 80,
                soft: Some(65),
                fifty: false,
            },
            Banner {
                key: "standard",
                label: "Standard Channel",
                hard: 90,
                soft: Some(74),
                fifty: false,
            },
            Banner {
                key: "bangboo",
                label: "Bangboo Channel",
                hard: 80,
                soft: Some(65),
                fifty: false,
            },
        ],
    }
}

/// Language-independent item ids and Paimon.moe slugs for permanent-pool 5★ characters.
fn standard_fives(game: Game) -> &'static [&'static str] {
    match game {
        // Paimon.moe local exports store character slugs instead of HoYo numeric IDs.
        Game::Hk4e => &[
            "10000003", "10000016", "10000035", "10000041", "10000042", "10000071", "10000079",
            "diluc", "jean", "keqing", "qiqi", "mona", "tighnari", "dehya",
        ],
        Game::Hkrpg => &["1003", "1004", "1101", "1104", "1107", "1209", "1211"],
        // Permanent S-rank agents in the Stable Channel. These IDs are returned in ZZZ
        // wish records and are language-independent, like the item IDs for the other games.
        Game::Nap => &["1021", "1041", "1101", "1141", "1181", "1211"],
    }
}

/// HoYoverse's live history API frequently omits `item_id`, even when it provides a usable
/// localized `name`. Older records must still identify standard-pool characters by name.
fn is_standard_five(game: Game, pull: &Pull) -> bool {
    let names: &[&str] = match game {
        Game::Hk4e => &[
            "Diluc", "Jean", "Keqing", "Mona", "Qiqi", "Tighnari", "Dehya",
        ],
        Game::Hkrpg => &[
            "Himeko", "Welt", "Bronya", "Gepard", "Clara", "Yanqing", "Bailu",
        ],
        Game::Nap => &[
            "Grace",
            "Koleda",
            "Lycaon",
            "Nekomata",
            "Rina",
            "Soldier 11",
        ],
    };
    standard_fives(game)
        .iter()
        .any(|id| id.eq_ignore_ascii_case(&pull.item_id))
        || names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(pull.name.trim()))
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
        (Game::Hkrpg, "12" | "22") => "weapon",
        (Game::Hkrpg, "21") => "character",
        (Game::Nap, "1") => "standard",
        (Game::Nap, "2") => "character",
        (Game::Nap, "3") => "weapon",
        (Game::Nap, "5") => "bangboo",
        _ => return None,
    })
}

/// Keep stored/API rarity untouched (UIGF represents ZZZ as B/A/S = 2/3/4),
/// while presenting one consistent 3★/4★/5★ scale throughout Wishtory.
pub fn display_rank(game: Game, raw_rank: u8) -> u8 {
    if game == Game::Nap && (2..=4).contains(&raw_rank) {
        raw_rank + 1
    } else {
        raw_rank
    }
}

#[derive(Serialize)]
pub struct FiveStar {
    pub name: String,
    pub pity: u32,
    pub time: String,
    pub off_banner: Option<bool>,
    pub outcome: Option<String>,
}
#[derive(Serialize)]
pub struct FourStar {
    pub name: String,
    pub pity: u32,
    pub time: String,
}

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
    pub avg_four_pity: Option<f32>,
    pub recent_fives: Vec<FiveStar>,
    pub recent_fours: Vec<FourStar>,
}

/// `pulls` must be sorted oldest → newest (Db::pulls_for does this).
pub fn compute(game: Game, pulls: &[Pull]) -> Vec<BannerPity> {
    banners(game)
        .iter()
        .filter_map(|b| {
            let (
                mut counter,
                mut four_counter,
                mut total,
                mut fives,
                mut fours,
                mut sum_five,
                mut sum_four,
            ) = (0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
            // Only infer guarantee state after observing a loss in the imported history.
            // A history may begin mid-stream, so the first featured pull is ambiguous.
            let mut guaranteed: Option<bool> = None;
            let mut recent_fives = Vec::new();
            let mut recent_fours = Vec::new();
            for p in pulls
                .iter()
                .filter(|p| banner_key(game, &p.gacha_type) == Some(b.key))
            {
                counter += 1;
                four_counter += 1;
                total += 1;
                match display_rank(game, p.rank_type) {
                    4 => {
                        fours += 1;
                        sum_four += four_counter;
                        recent_fours.push(FourStar {
                            name: p.name.clone(),
                            pity: four_counter,
                            time: p.time.clone(),
                        });
                        four_counter = 0;
                    }
                    5 => {
                        fives += 1;
                        sum_five += counter;
                        let is_standard = is_standard_five(game, p);
                        let off = if b.fifty { Some(is_standard) } else { None };
                        let outcome = if !b.fifty {
                            None
                        } else if is_standard {
                            guaranteed = Some(true);
                            Some("loss".into())
                        } else if guaranteed == Some(true) {
                            guaranteed = Some(false);
                            Some("guaranteed".into())
                        } else if guaranteed == Some(false) {
                            Some("win".into())
                        } else {
                            // The record alone cannot distinguish a 50/50 win from a
                            // previously established guarantee (or Genshin Capturing Radiance).
                            // Either way, this featured pull consumes the previous state and
                            // leaves the next five-star at the featured-chance state. That lets
                            // later pulls in a partial history be classified accurately.
                            guaranteed = Some(false);
                            None
                        };
                        recent_fives.push(FiveStar {
                            name: p.name.clone(),
                            pity: counter,
                            time: p.time.clone(),
                            off_banner: off,
                            outcome,
                        });
                        counter = 0;
                        four_counter = 0;
                    }
                    _ => {}
                }
            }
            if total == 0 && b.key == "beginner" {
                return None;
            }
            Some(BannerPity {
                banner: b.key.into(),
                label: b.label.into(),
                pity: counter,
                hard_pity: b.hard,
                soft_pity: b.soft,
                guaranteed,
                total,
                five_stars: fives,
                four_stars: fours,
                avg_five_pity: if fives > 0 {
                    Some(sum_five as f32 / fives as f32)
                } else {
                    None
                },
                avg_four_pity: if fours > 0 {
                    Some(sum_four as f32 / fours as f32)
                } else {
                    None
                },
                recent_fives,
                recent_fours,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull(game: &str, gt: &str, id: u32, rank: u8, item: &str) -> Pull {
        Pull {
            id: id.to_string(),
            game: game.into(),
            uid: "1".into(),
            gacha_type: gt.into(),
            gacha_id: String::new(),
            item_id: item.into(),
            name: "n".into(),
            item_type: "Character".into(),
            rank_type: rank,
            time: format!("2024-01-01 00:{:02}:{:02}", id / 60, id % 60),
            lang: "en".into(),
            count: 1,
        }
    }
    fn p(id: u32, rank: u8, item: &str) -> Pull {
        pull("hk4e", "301", id, rank, item)
    }
    fn find<'a>(r: &'a [BannerPity], key: &str) -> &'a BannerPity {
        r.iter().find(|b| b.banner == key).unwrap()
    }

    #[test]
    fn counts_pity_and_fifty_fifty() {
        // 3 pulls, a standard-pool 5* (loses 50/50), then 2 more pulls.
        let pulls = vec![
            p(1, 3, "a"),
            p(2, 4, "b"),
            p(3, 5, "10000003"),
            p(4, 3, "c"),
            p(5, 3, "d"),
        ];
        let r = compute(Game::Hk4e, &pulls);
        let c = find(&r, "character");
        assert_eq!((c.pity, c.total, c.five_stars, c.four_stars), (2, 5, 1, 1));
        assert_eq!(c.guaranteed, Some(true));
        assert_eq!(c.avg_five_pity, Some(3.0));
    }

    #[test]
    fn winning_the_fifty_clears_the_guarantee() {
        let pulls = vec![
            p(1, 5, "10000003"),
            p(2, 3, "x"),
            p(3, 5, "10000099"),
            p(4, 5, "10000099"),
        ];
        let r = compute(Game::Hk4e, &pulls);
        let c = find(&r, "character");
        assert_eq!((c.pity, c.guaranteed), (0, Some(false)));
        assert_eq!(c.recent_fives[0].outcome.as_deref(), Some("loss")); // oldest first
        assert_eq!(c.recent_fives[1].outcome.as_deref(), Some("guaranteed"));
        assert_eq!(c.recent_fives[2].outcome.as_deref(), Some("win"));
    }

    #[test]
    fn paimon_slug_standard_character_is_a_loss_not_a_featured_win() {
        let result = compute(Game::Hk4e, &[p(1, 5, "tighnari")]);
        let c = find(&result, "character");
        assert_eq!(c.guaranteed, Some(true));
        assert_eq!(c.recent_fives[0].outcome.as_deref(), Some("loss"));
    }

    #[test]
    fn missing_item_ids_use_live_history_names_for_genshin_and_star_rail_losses() {
        let mut diluc = p(1, 5, "");
        diluc.name = "Diluc".into();
        let genshin = compute(Game::Hk4e, &[diluc]);
        assert_eq!(
            find(&genshin, "character").recent_fives[0]
                .outcome
                .as_deref(),
            Some("loss")
        );

        let mut clara = pull("hkrpg", "11", 1, 5, "");
        clara.name = "Clara".into();
        let star_rail = compute(Game::Hkrpg, &[clara]);
        assert_eq!(
            find(&star_rail, "character").recent_fives[0]
                .outcome
                .as_deref(),
            Some("loss")
        );
    }

    #[test]
    fn incomplete_history_keeps_first_featured_outcome_unknown_but_knows_next_is_not_guaranteed() {
        let result = compute(Game::Hk4e, &[p(1, 5, "10000099")]);
        let c = find(&result, "character");
        assert_eq!(c.guaranteed, Some(false));
        assert_eq!(c.recent_fives[0].outcome, None);
    }

    #[test]
    fn genshin_classifies_outcomes_after_an_ambiguous_first_featured_pull() {
        let pulls = vec![
            p(1, 5, "10000099"), // unknown: may have been a guaranteed featured pull
            p(2, 5, "10000003"), // standard-pool loss
            p(3, 5, "10000099"), // guaranteed featured
            p(4, 5, "10000099"), // featured chance won
        ];
        let result = compute(Game::Hk4e, &pulls);
        let character = find(&result, "character");
        assert_eq!(
            character
                .recent_fives
                .iter()
                .map(|f| f.outcome.as_deref())
                .collect::<Vec<_>>(),
            vec![None, Some("loss"), Some("guaranteed"), Some("win")]
        );
    }

    #[test]
    fn second_character_banner_shares_pity_with_the_first() {
        let pulls = vec![
            pull("hk4e", "301", 1, 3, "a"),
            pull("hk4e", "400", 2, 3, "b"),
            pull("hk4e", "301", 3, 3, "c"),
        ];
        let r = compute(Game::Hk4e, &pulls);
        assert_eq!(
            (find(&r, "character").pity, find(&r, "character").total),
            (3, 3)
        );
    }

    #[test]
    fn empty_history_shows_banners_with_zero_and_hides_beginner() {
        let r = compute(Game::Hkrpg, &[]);
        assert!(r.iter().all(|b| b.banner != "beginner"));
        assert!(r
            .iter()
            .all(|b| b.pity == 0 && b.total == 0 && b.avg_five_pity.is_none()));
    }

    #[test]
    fn star_rail_classifies_outcomes_after_an_ambiguous_first_featured_pull() {
        let pulls = vec![
            pull("hkrpg", "11", 1, 5, "1201"), // unknown featured state
            pull("hkrpg", "11", 2, 5, "1003"), // Himeko: loss
            pull("hkrpg", "11", 3, 5, "1202"), // guaranteed featured
            pull("hkrpg", "11", 4, 5, "1203"), // featured chance won
            pull("hkrpg", "99", 5, 5, "1"),
        ];
        let r = compute(Game::Hkrpg, &pulls);
        let c = find(&r, "character");
        assert_eq!((c.five_stars, c.guaranteed), (4, Some(false)));
        assert_eq!(
            c.recent_fives
                .iter()
                .map(|f| f.outcome.as_deref())
                .collect::<Vec<_>>(),
            vec![None, Some("loss"), Some("guaranteed"), Some("win")]
        );
        assert_eq!(
            r.iter().map(|b| b.total).sum::<u32>(),
            4,
            "unknown gacha_type must not be counted"
        );
    }

    #[test]
    fn star_rail_current_collaboration_event_types_are_recognized() {
        assert_eq!(banner_key(Game::Hkrpg, "21"), Some("character"));
        assert_eq!(banner_key(Game::Hkrpg, "22"), Some("weapon"));
    }

    #[test]
    fn weapon_banner_has_no_fifty_state() {
        let r = compute(Game::Hk4e, &[pull("hk4e", "302", 1, 5, "15502")]);
        assert_eq!(find(&r, "weapon").guaranteed, None);
    }

    #[test]
    fn zenless_exclusive_channel_tracks_off_banner_guarantee() {
        let pulls = vec![
            pull("nap", "2", 1, 4, "1181"), // ZZZ encodes S-rank as 4
            pull("nap", "2", 2, 2, "a"),    // B-rank
        ];
        let result = compute(Game::Nap, &pulls);
        let exclusive = find(&result, "character");
        assert_eq!(exclusive.guaranteed, Some(true));
        assert_eq!(exclusive.recent_fives[0].off_banner, Some(true));
        assert_eq!(exclusive.pity, 1);
    }

    #[test]
    fn zenless_featured_s_rank_clears_the_exclusive_guarantee() {
        let pulls = vec![
            pull("nap", "2", 1, 4, "1181"),
            pull("nap", "2", 2, 4, "1391"), // featured agent, not in the stable pool
        ];
        let result = compute(Game::Nap, &pulls);
        let exclusive = find(&result, "character");
        assert_eq!(exclusive.guaranteed, Some(false));
        assert_eq!(exclusive.recent_fives[1].off_banner, Some(false));
    }

    #[test]
    fn zenless_raw_ranks_count_s_and_a_ranks_as_five_and_four_stars() {
        let pulls = vec![
            pull("nap", "2", 1, 2, "b-rank"),
            pull("nap", "2", 2, 3, "a-rank"),
            pull("nap", "2", 3, 4, "s-rank"),
        ];
        let result = compute(Game::Nap, &pulls);
        let exclusive = find(&result, "character");
        assert_eq!((exclusive.five_stars, exclusive.four_stars), (1, 1));
        assert_eq!(exclusive.recent_fives[0].name, "n");
        assert_eq!(exclusive.pity, 0);
    }

    #[test]
    fn banner_analysis_reports_five_and_four_star_pull_averages_and_history() {
        let pulls = vec![
            p(1, 3, "one"),
            p(2, 4, "four"),
            p(3, 3, "three"),
            p(4, 5, "five"),
        ];
        let result = compute(Game::Hk4e, &pulls);
        let character = find(&result, "character");
        assert_eq!(character.avg_five_pity, Some(4.0));
        assert_eq!(character.avg_four_pity, Some(2.0));
        assert_eq!(character.recent_fives.len(), 1);
        assert_eq!(character.recent_fours.len(), 1);
        assert_eq!(character.recent_fours[0].name, "n");
    }
}
