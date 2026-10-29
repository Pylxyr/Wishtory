//! Pity counters, 50/50 state and basic statistics, computed from stored pulls.
//!
//! Everything that changes with the games (hard and soft pity, the permanent pool, items that
//! were also featured, Zenless Custom Search) is data in `rules.rs`, not code here, so a rule
//! change ships as a signed update instead of an app release.
//!
//! Win/loss tracking ("odds") exists for the banners whose off-banner result is a permanent-pool
//! item: character banners (50/50) and weapon / Light Cone / W-Engine banners (75/25). A result
//! counts as a loss only when the 5-star is in the permanent pool; any other 5-star counts as
//! featured, including a non-selected weapon under Epitomized Path. Chronicled, Bangboo and the
//! Star Rail collaboration warps have no tracked result. A permanent-pool item that was itself a
//! rate-up item counts as featured: either because the signed banner list features it on the
//! banner the pull was made on, or (as a fallback that needs no banner list) because the rules
//! list a featured window for it.

use crate::{
    catalog::BannerEntry,
    models::{Game, Pull},
    rules::{GameRules, RuleSet},
};
use serde::Serialize;
use std::collections::HashMap;

/// Banners that only exist for part of the game's life stay hidden until they have a pull.
fn hidden_when_empty(key: &str) -> bool {
    key == "beginner" || key.starts_with("collab")
}

/// Lower-case, straight-apostrophe form so `Amos’ Bow` and `Amos' Bow` compare equal.
fn fold(name: &str) -> String {
    name.trim().replace('\u{2019}', "'").to_lowercase()
}

/// HoYoverse's live history API frequently omits `item_id`, even when it provides a usable
/// localized `name`. Older records must still identify permanent-pool items by name.
fn is_standard_five(rules: &GameRules, banner: &str, pull: &Pull) -> bool {
    let pool = if banner == "weapon" {
        &rules.standard.weapon
    } else {
        &rules.standard.character
    };
    let name = fold(&pull.name);
    pool.ids
        .iter()
        .any(|id| id.eq_ignore_ascii_case(&pull.item_id))
        || pool.names.iter().any(|n| fold(n) == name)
}

/// True when a pull matches a featured window of the rules and was made inside its dates.
fn was_featured(rules: &GameRules, pull: &Pull) -> bool {
    let Some(day) = pull.time.get(..10) else {
        return false;
    };
    let name = fold(&pull.name);
    rules.featured_windows.iter().any(|w| {
        (w.id.eq_ignore_ascii_case(&pull.item_id) || fold(&w.name) == name)
            && w.from.as_str() <= day
            && day <= w.to.as_str()
    })
}

/// Zenless records carry `gacha_id` "0" and Genshin records none: neither identifies a banner.
fn has_gacha_id(pull: &Pull) -> bool {
    !pull.gacha_id.is_empty() && pull.gacha_id != "0"
}

/// The banner of every pull, in order. Star Rail and Zenless run concurrent banners under one pool
/// type, and only a featured 5★ can place a pull on one of them. Every pull that carries a
/// `gacha_id` shared with a placed pull joins that banner, so 3★ and 4★ pulls are labelled as soon
/// as any 5★ from the same banner is known. A `gacha_id` claimed by two different banners is
/// ignored.
pub fn resolve_banners<'a>(
    banners: &'a [BannerEntry],
    game: Game,
    pulls: &[Pull],
) -> Vec<Option<&'a BannerEntry>> {
    let mut placed: Vec<Option<&'a BannerEntry>> = pulls
        .iter()
        .map(|p| resolve_banner(banners, game, p))
        .collect();
    let mut learned: HashMap<&str, Option<&'a BannerEntry>> = HashMap::new();
    for (pull, found) in pulls.iter().zip(placed.iter().copied()) {
        let Some(found) = found else { continue };
        if !has_gacha_id(pull) {
            continue;
        }
        learned
            .entry(pull.gacha_id.as_str())
            .and_modify(|known| {
                if known.is_some_and(|k| k.id != found.id) {
                    *known = None;
                }
            })
            .or_insert(Some(found));
    }
    for (pull, slot) in pulls.iter().zip(placed.iter_mut()) {
        if slot.is_some() || !has_gacha_id(pull) {
            continue;
        }
        if let Some(Some(banner)) = learned.get(pull.gacha_id.as_str()) {
            if banner_key(game, &pull.gacha_type) == Some(banner.pool.as_str()) {
                *slot = Some(*banner);
            }
        }
    }
    placed
}

/// True when the banner list shows this pull's 5★ as featured on the banner it was made on.
/// Pulls the list cannot place, and records without an `item_id`, never match.
fn listed_as_featured(banners: &[BannerEntry], game: Game, pull: &Pull) -> bool {
    !pull.item_id.is_empty()
        && resolve_banner(banners, game, pull)
            .is_some_and(|banner| banner.featured_5.contains(&pull.item_id))
}

/// True for a Zenless Exclusive Channel S-Rank agent, or a W-Engine Channel S-Rank engine, that
/// no live event features, once Custom Search exists: a lost 50/50 or 75/25, even when it is not
/// a Stable Channel item. An engine counts as featured while its agent is on a live banner. Needs
/// an `item_id` and at least one live agent event in the banner list; otherwise it cannot say.
fn lost_to_custom_search(
    rules: &GameRules,
    banners: &[BannerEntry],
    game: Game,
    banner: &str,
    pull: &Pull,
) -> bool {
    let Some(search) = &rules.custom_search else {
        return false;
    };
    if !matches!(banner, "character" | "weapon")
        || pull.item_id.is_empty()
        || pull.time.as_str() < search.from.as_str()
    {
        return false;
    }
    let live: Vec<&BannerEntry> = banners
        .iter()
        .filter(|b| b.game == game.as_str() && b.pool == "character")
        .filter(|b| b.start.as_str() <= pull.time.as_str() && pull.time.as_str() <= b.end.as_str())
        .collect();
    if live.is_empty() {
        return false;
    }
    let featured = |id: &str| live.iter().any(|b| b.featured_5.iter().any(|f| f == id));
    if banner == "character" {
        !featured(&pull.item_id)
    } else {
        search
            .engines
            .iter()
            .any(|e| e.engine == pull.item_id && !featured(&e.agent))
    }
}

/// History-page banner filter for one game: `(banner key, label)` in display order.
pub fn history_filters(game: Game) -> &'static [(&'static str, &'static str)] {
    match game {
        Game::Hk4e => &[
            ("character", "Character Event"),
            ("weapon", "Weapon Event"),
            ("standard", "Standard"),
            ("chronicled", "Chronicled"),
            ("beginner", "Beginners'"),
        ],
        Game::Hkrpg => &[
            ("character", "Character Warp"),
            ("weapon", "Light Cone Warp"),
            ("standard", "Stellar Warp"),
            ("collab", "Collaboration Warp"),
            ("collab_weapon", "Collaboration Light Cone Warp"),
            ("beginner", "Departure Warp"),
        ],
        Game::Nap => &[
            ("character", "Exclusive Channel"),
            ("weapon", "W-Engine Channel"),
            ("standard", "Standard Channel"),
            ("bangboo", "Bangboo Channel"),
        ],
    }
}

/// Short pool name for the history table's "Wish" column.
pub fn pool_label(game: Game, gacha_type: &str) -> Option<&'static str> {
    Some(match (game, gacha_type) {
        (Game::Hk4e, "100") => "Beginners'",
        (Game::Hk4e, "200") => "Standard",
        (Game::Hk4e, "301" | "400") => "Character",
        (Game::Hk4e, "302") => "Weapon",
        (Game::Hk4e, "500") => "Chronicled",
        (Game::Hkrpg, "1") => "Stellar",
        (Game::Hkrpg, "2") => "Departure",
        (Game::Hkrpg, "11") => "Character",
        (Game::Hkrpg, "12") => "Light Cone",
        (Game::Hkrpg, "21") => "Collab Character",
        (Game::Hkrpg, "22") => "Collab Light Cone",
        (Game::Nap, "1") => "Standard",
        (Game::Nap, "2") => "Exclusive",
        (Game::Nap, "3") => "W-Engine",
        (Game::Nap, "5") => "Bangboo",
        _ => return None,
    })
}

/// The banner event a pull was made on, or `None` when the banner list cannot say for sure.
/// Star Rail and Zenless report a `gacha_id`; otherwise the pull's server-local time picks the
/// event window of the same banner type. Concurrent banners (two Genshin character banners) are
/// told apart only by a pull of a featured 5★; anything still ambiguous stays unresolved.
pub fn resolve_banner<'a>(
    banners: &'a [BannerEntry],
    game: Game,
    pull: &Pull,
) -> Option<&'a BannerEntry> {
    let pool = banner_key(game, &pull.gacha_type)?;
    let same_pool = |b: &&BannerEntry| b.game == game.as_str() && b.pool == pool;
    if has_gacha_id(pull) {
        if let Some(found) = banners
            .iter()
            .filter(same_pool)
            .find(|b| b.gacha_ids.contains(&pull.gacha_id))
        {
            return Some(found);
        }
    }
    let live: Vec<&BannerEntry> = banners
        .iter()
        .filter(same_pool)
        .filter(|b| b.start.as_str() <= pull.time.as_str() && pull.time.as_str() <= b.end.as_str())
        .collect();
    match live.as_slice() {
        [] => None,
        [only] => Some(*only),
        several => {
            let mut featured = several
                .iter()
                .filter(|b| b.featured_5.contains(&pull.item_id));
            let first = featured.next()?;
            if featured.next().is_some() {
                None
            } else {
                Some(*first)
            }
        }
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
        // Collaboration warps keep their own pity, separate from the regular event warps.
        (Game::Hkrpg, "21") => "collab",
        (Game::Hkrpg, "22") => "collab_weapon",
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

/// Where one pull sits in its pity pool: the counters INCLUDING this pull, before a reset.
struct Step {
    key: &'static str,
    /// Display rank (3 / 4 / 5 for every game).
    rank: u8,
    five: u32,
    four: u32,
}

/// One pass over the history that every pity figure is derived from.
struct Walk {
    /// Aligned with the pulls; `None` for a gacha type that belongs to no known pool.
    steps: Vec<Option<Step>>,
    /// Pulls since the last 5★ and since the last 4★/5★ after the final pull of every pool.
    open: HashMap<&'static str, (u32, u32)>,
}

/// Counts per pity pool. A 5★ resets both counters, a 4★ only the 4★ one. `pulls` must be
/// sorted oldest → newest (`Db::pulls_for` does this).
fn walk(game: Game, pulls: &[Pull]) -> Walk {
    let mut open: HashMap<&'static str, (u32, u32)> = HashMap::new();
    let steps = pulls
        .iter()
        .map(|p| {
            let key = banner_key(game, &p.gacha_type)?;
            let (five, four) = open.entry(key).or_insert((0, 0));
            *five += 1;
            *four += 1;
            let rank = display_rank(game, p.rank_type);
            let step = Step {
                key,
                rank,
                five: *five,
                four: *four,
            };
            match rank {
                5 => {
                    *five = 0;
                    *four = 0;
                }
                4 => *four = 0,
                _ => {}
            }
            Some(step)
        })
        .collect();
    Walk { steps, open }
}

/// Pity of every pull at the moment it was made, aligned with `pulls` (oldest → newest): pulls
/// since the last 5★ for a 5★, since the last 4★ or 5★ for a 4★, `None` for anything else.
pub fn pull_pities(game: Game, pulls: &[Pull]) -> Vec<Option<u32>> {
    walk(game, pulls)
        .steps
        .into_iter()
        .map(|step| {
            let step = step?;
            match step.rank {
                5 => Some(step.five),
                4 => Some(step.four),
                _ => None,
            }
        })
        .collect()
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
    /// Percent chance a non-guaranteed 5★ is featured (50 / 75); `None` when no result is tracked.
    pub guarantee_odds: Option<u8>,
    pub total: u32,
    pub five_stars: u32,
    pub four_stars: u32,
    pub avg_five_pity: Option<f32>,
    pub avg_four_pity: Option<f32>,
    pub recent_fives: Vec<FiveStar>,
    pub recent_fours: Vec<FourStar>,
}

/// `pulls` must be sorted oldest → newest (Db::pulls_for does this).
#[cfg(test)]
pub fn compute(game: Game, pulls: &[Pull]) -> Vec<BannerPity> {
    compute_with_banners(game, pulls, &[], &RuleSet::bundled())
}

/// Pity counters and win / loss state per banner. `banner_list` lets a permanent-pool item that
/// the signed list features on its banner count as featured instead of as a loss.
pub fn compute_with_banners(
    game: Game,
    pulls: &[Pull],
    banner_list: &[BannerEntry],
    rule_set: &RuleSet,
) -> Vec<BannerPity> {
    let rules = rule_set.game(game);
    let Walk { steps, open } = walk(game, pulls);
    rules
        .banners
        .iter()
        .filter_map(|b| {
            let (mut total, mut fives, mut fours, mut sum_five, mut sum_four) =
                (0u32, 0u32, 0u32, 0u32, 0u32);
            // Only infer guarantee state after observing a loss in the imported history.
            // A history may begin mid-stream, so the first featured pull is ambiguous.
            let mut guaranteed: Option<bool> = None;
            let mut recent_fives = Vec::new();
            let mut recent_fours = Vec::new();
            for (p, step) in pulls.iter().zip(&steps) {
                let Some(step) = step.as_ref().filter(|s| s.key == b.key) else {
                    continue;
                };
                total += 1;
                match step.rank {
                    4 => {
                        fours += 1;
                        sum_four += step.four;
                        recent_fours.push(FourStar {
                            name: p.name.clone(),
                            pity: step.four,
                            time: p.time.clone(),
                        });
                    }
                    5 => {
                        fives += 1;
                        sum_five += step.five;
                        let is_standard = (is_standard_five(rules, &b.key, p)
                            && !was_featured(rules, p)
                            && !listed_as_featured(banner_list, game, p))
                            || lost_to_custom_search(rules, banner_list, game, &b.key, p);
                        let off = b.odds.map(|_| is_standard);
                        let outcome = if b.odds.is_none() {
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
                            pity: step.five,
                            time: p.time.clone(),
                            off_banner: off,
                            outcome,
                        });
                    }
                    _ => {}
                }
            }
            if total == 0 && hidden_when_empty(&b.key) {
                return None;
            }
            Some(BannerPity {
                banner: b.key.clone(),
                label: b.label.clone(),
                pity: open.get(b.key.as_str()).map_or(0, |(five, _)| *five),
                hard_pity: b.hard,
                soft_pity: b.soft,
                guaranteed,
                guarantee_odds: b.odds,
                total,
                five_stars: fives,
                four_stars: fours,
                avg_five_pity: (fives > 0).then(|| sum_five as f32 / fives as f32),
                avg_four_pity: (fours > 0).then(|| sum_four as f32 / fours as f32),
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
    fn permanent_items_pulled_while_featured_are_not_losses() {
        let at = |mut x: Pull, time: &str| {
            x.time = time.into();
            x
        };
        // Skyward Atlas (14501) was a rate-up weapon in version 1.2 (2020-12-23 to 2021-01-12).
        let weapon = |time: &str| at(pull("hk4e", "302", 1, 5, "14501"), time);
        let r = compute(Game::Hk4e, &[weapon("2021-01-05 12:00:00")]);
        let w = find(&r, "weapon");
        assert_eq!(
            (w.guaranteed, w.recent_fives[0].outcome.as_deref()),
            (Some(false), None)
        );
        let r = compute(Game::Hk4e, &[weapon("2024-01-05 12:00:00")]);
        let w = find(&r, "weapon");
        assert_eq!(w.recent_fives[0].outcome.as_deref(), Some("loss"));
        // Tighnari (10000069) headlined the 3.0 banner (2022-08-24 to 2022-09-09), before he
        // joined the permanent pool.
        let r = compute(
            Game::Hk4e,
            &[at(p(1, 5, "10000069"), "2022-09-01 12:00:00")],
        );
        let c = find(&r, "character");
        assert_eq!(
            (c.guaranteed, c.recent_fives[0].outcome.as_deref()),
            (Some(false), None)
        );
    }

    #[test]
    fn paimon_slug_standard_character_is_a_loss_not_a_featured_win() {
        let result = compute(Game::Hk4e, &[p(1, 5, "tighnari")]);
        let c = find(&result, "character");
        assert_eq!(c.guaranteed, Some(true));
        assert_eq!(c.recent_fives[0].outcome.as_deref(), Some("loss"));

        // Numeric ids: Tighnari (10000069) is permanent, Cyno (10000071) is a featured 5-star.
        let result = compute(Game::Hk4e, &[p(1, 5, "10000069")]);
        let c = find(&result, "character");
        assert_eq!(c.recent_fives[0].outcome.as_deref(), Some("loss"));
        let result = compute(Game::Hk4e, &[p(1, 5, "10000071")]);
        let c = find(&result, "character");
        assert_eq!(
            (c.guaranteed, c.recent_fives[0].outcome.as_deref()),
            (Some(false), None)
        );
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
    fn star_rail_collaboration_warps_have_their_own_pity() {
        assert_eq!(banner_key(Game::Hkrpg, "11"), Some("character"));
        assert_eq!(banner_key(Game::Hkrpg, "12"), Some("weapon"));
        assert_eq!(banner_key(Game::Hkrpg, "21"), Some("collab"));
        assert_eq!(banner_key(Game::Hkrpg, "22"), Some("collab_weapon"));

        let pulls = vec![
            pull("hkrpg", "11", 1, 3, "a"),
            pull("hkrpg", "21", 2, 3, "b"),
            pull("hkrpg", "21", 3, 5, "9001"),
            pull("hkrpg", "22", 4, 3, "c"),
            pull("hkrpg", "12", 5, 3, "d"),
        ];
        let r = compute(Game::Hkrpg, &pulls);
        assert_eq!(
            (find(&r, "character").pity, find(&r, "character").total),
            (1, 1)
        );
        let collab = find(&r, "collab");
        assert_eq!((collab.pity, collab.total, collab.five_stars), (0, 2, 1));
        assert_eq!(collab.guaranteed, None);
        assert_eq!(collab.recent_fives[0].outcome, None);
        assert_eq!(find(&r, "collab_weapon").total, 1);
        assert_eq!((find(&r, "weapon").pity, find(&r, "weapon").total), (1, 1));
    }

    #[test]
    fn collaboration_warps_are_hidden_until_used() {
        let r = compute(Game::Hkrpg, &[pull("hkrpg", "11", 1, 3, "a")]);
        assert!(r.iter().all(|b| !b.banner.starts_with("collab")));
    }

    fn weapon_outcomes(game: Game, gacha_type: &str, fives: &[&str]) -> Vec<Option<String>> {
        let pulls: Vec<Pull> = fives
            .iter()
            .enumerate()
            .map(|(i, item)| pull(game.as_str(), gacha_type, i as u32 + 1, 5, item))
            .collect();
        let r = compute(game, &pulls);
        find(&r, "weapon")
            .recent_fives
            .iter()
            .map(|f| f.outcome.clone())
            .collect()
    }

    #[test]
    fn genshin_weapon_banner_loses_only_to_the_standard_weapon_pool() {
        // Unknown first featured pull, standard Skyward Harp (loss), guaranteed featured,
        // then a featured pull that was not guaranteed. A featured weapon always counts as
        // a win, even when it is not the one selected on Epitomized Path.
        let outcomes = weapon_outcomes(Game::Hk4e, "302", &["14515", "15501", "14516", "14515"]);
        let expected: Vec<Option<String>> = vec![
            None,
            Some("loss".into()),
            Some("guaranteed".into()),
            Some("win".into()),
        ];
        assert_eq!(outcomes, expected);

        let r = compute(Game::Hk4e, &[pull("hk4e", "302", 1, 5, "12502")]);
        let w = find(&r, "weapon");
        assert_eq!((w.guaranteed, w.guarantee_odds), (Some(true), Some(75)));
        assert_eq!(w.recent_fives[0].off_banner, Some(true));
    }

    #[test]
    fn star_rail_light_cone_banner_tracks_standard_light_cones() {
        let outcomes = weapon_outcomes(Game::Hkrpg, "12", &["23013", "23020", "23021"]);
        let expected: Vec<Option<String>> = vec![
            Some("loss".into()),
            Some("guaranteed".into()),
            Some("win".into()),
        ];
        assert_eq!(outcomes, expected);
        let r = compute(Game::Hkrpg, &[pull("hkrpg", "12", 1, 5, "23013")]);
        assert_eq!(find(&r, "weapon").guarantee_odds, Some(75));
    }

    #[test]
    fn zenless_w_engine_channel_tracks_standard_w_engines() {
        let outcomes = weapon_outcomes(Game::Nap, "3", &["14102", "14999"]);
        let expected: Vec<Option<String>> = vec![Some("loss".into()), Some("guaranteed".into())];
        assert_eq!(outcomes, expected);
    }

    #[test]
    fn weapon_losses_are_recognized_by_name_when_item_id_is_missing() {
        let mut harp = pull("hk4e", "302", 1, 5, "");
        harp.name = "Amos\u{2019} Bow".into(); // typographic apostrophe
        let mut lc = pull("hkrpg", "12", 1, 5, "");
        lc.name = "time waits for no one".into();
        let mut engine = pull("nap", "3", 1, 4, "");
        engine.name = "Weeping Cradle".into();
        for (game, pull) in [(Game::Hk4e, harp), (Game::Hkrpg, lc), (Game::Nap, engine)] {
            let r = compute(game, &[pull]);
            assert_eq!(
                find(&r, "weapon").recent_fives[0].outcome.as_deref(),
                Some("loss")
            );
        }
    }

    #[test]
    fn a_standard_weapon_is_not_a_standard_character_loss() {
        // The permanent character list must not classify weapon ids, and vice versa.
        let r = compute(Game::Hk4e, &[pull("hk4e", "301", 1, 5, "15502")]);
        assert_eq!(
            find(&r, "character").recent_fives[0].outcome,
            None,
            "a weapon id on the character banner is not a standard character"
        );
        let r = compute(Game::Hk4e, &[pull("hk4e", "302", 1, 5, "10000003")]);
        assert_eq!(find(&r, "weapon").recent_fives[0].outcome, None);
    }

    #[test]
    fn genshin_mizuki_is_a_permanent_pool_loss() {
        let r = compute(Game::Hk4e, &[p(1, 5, "10000109")]);
        let c = find(&r, "character");
        assert_eq!(c.recent_fives[0].outcome.as_deref(), Some("loss"));
        assert_eq!(c.guaranteed, Some(true));
    }

    #[test]
    fn chronicled_and_bangboo_banners_track_no_result() {
        let r = compute(
            Game::Hk4e,
            &[
                pull("hk4e", "500", 1, 5, "10000003"),
                pull("hk4e", "500", 2, 5, "x"),
            ],
        );
        let chronicled = find(&r, "chronicled");
        assert_eq!(chronicled.guaranteed, None);
        assert_eq!(chronicled.guarantee_odds, None);
        assert!(chronicled.recent_fives.iter().all(|f| f.outcome.is_none()));

        let r = compute(Game::Nap, &[pull("nap", "5", 1, 4, "14102")]);
        let bangboo = find(&r, "bangboo");
        assert_eq!((bangboo.guaranteed, bangboo.guarantee_odds), (None, None));
        assert_eq!(bangboo.recent_fives[0].outcome, None);
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

    fn event(
        id: &str,
        game: &str,
        pool: &str,
        start: &str,
        end: &str,
        featured: &[&str],
    ) -> BannerEntry {
        BannerEntry {
            id: id.into(),
            game: game.into(),
            pool: pool.into(),
            name: id.into(),
            gacha_ids: vec![],
            start: start.into(),
            end: end.into(),
            featured_5: featured.iter().map(|f| f.to_string()).collect(),
            featured_4: vec![],
        }
    }

    fn at(mut pull: Pull, time: &str) -> Pull {
        pull.time = time.into();
        pull
    }

    #[test]
    fn banner_is_resolved_from_the_server_local_event_window() {
        let list = vec![
            event(
                "a",
                "hk4e",
                "character",
                "2026-01-01 18:00:00",
                "2026-01-21 17:59:59",
                &["1"],
            ),
            event(
                "b",
                "hk4e",
                "character",
                "2026-01-21 18:00:00",
                "2026-02-10 17:59:59",
                &["2"],
            ),
            event(
                "w",
                "hk4e",
                "weapon",
                "2026-01-01 18:00:00",
                "2026-02-10 17:59:59",
                &["3"],
            ),
        ];
        let id = |pull: Pull| resolve_banner(&list, Game::Hk4e, &pull).map(|b| b.id.clone());
        assert_eq!(
            id(at(p(1, 3, "x"), "2026-01-21 17:59:59")),
            Some("a".into())
        );
        assert_eq!(
            id(at(p(1, 3, "x"), "2026-01-21 18:00:00")),
            Some("b".into())
        );
        assert_eq!(id(at(p(1, 3, "x"), "2025-12-31 23:59:59")), None);
        assert_eq!(id(at(p(1, 3, "x"), "2026-02-10 18:00:00")), None);
        // The weapon banner is a separate pool, and standard / chronicled pulls have no event.
        assert_eq!(
            id(at(pull("hk4e", "302", 1, 3, "x"), "2026-01-05 12:00:00")),
            Some("w".into())
        );
        assert_eq!(
            id(at(pull("hk4e", "200", 1, 3, "x"), "2026-01-05 12:00:00")),
            None
        );
        assert_eq!(
            id(at(pull("hk4e", "500", 1, 3, "x"), "2026-01-05 12:00:00")),
            None
        );
    }

    #[test]
    fn concurrent_banners_need_a_gacha_id_or_a_featured_five_star() {
        let mut first = event(
            "one",
            "nap",
            "character",
            "2026-01-01 12:00:00",
            "2026-01-21 11:59:59",
            &["10"],
        );
        first.gacha_ids = vec!["2001".into()];
        let mut second = event(
            "two",
            "nap",
            "character",
            "2026-01-01 12:00:00",
            "2026-01-21 11:59:59",
            &["20"],
        );
        second.gacha_ids = vec!["2002".into()];
        let list = vec![first, second];
        let id = |pull: Pull| resolve_banner(&list, Game::Nap, &pull).map(|b| b.id.clone());
        let base = || at(pull("nap", "2", 1, 2, "x"), "2026-01-05 12:00:00");
        // The pull's own gacha_id decides, even when no featured item is involved.
        let mut with_id = base();
        with_id.gacha_id = "2002".into();
        assert_eq!(id(with_id), Some("two".into()));
        // Without a gacha_id the two windows overlap: only a featured 5-star can tell them apart.
        assert_eq!(id(base()), None);
        assert_eq!(
            id(at(pull("nap", "2", 1, 4, "10"), "2026-01-05 12:00:00")),
            Some("one".into())
        );
        assert_eq!(
            id(at(pull("nap", "2", 1, 4, "20"), "2026-01-05 12:00:00")),
            Some("two".into())
        );
        // A gacha_id that no listed banner knows falls back to the time window.
        let mut unknown = at(pull("nap", "2", 1, 4, "20"), "2026-01-05 12:00:00");
        unknown.gacha_id = "9999".into();
        assert_eq!(id(unknown), Some("two".into()));
    }

    #[test]
    fn banner_list_makes_a_featured_permanent_pool_item_a_win() {
        let list = vec![event(
            "jean-rerun",
            "hk4e",
            "character",
            "2030-01-01 18:00:00",
            "2030-01-21 17:59:59",
            &["10000003"],
        )];
        let jean = |time: &str| at(p(1, 5, "10000003"), time);
        let outcome = |banner_list: &[BannerEntry], pull: Pull| {
            let r = compute_with_banners(Game::Hk4e, &[pull], banner_list, &RuleSet::bundled());
            let c = find(&r, "character");
            (
                c.recent_fives[0].outcome.clone(),
                c.recent_fives[0].off_banner,
            )
        };
        // Without the list a permanent-pool character is a loss; inside its listed window it is not.
        assert_eq!(
            outcome(&[], jean("2030-01-05 12:00:00")),
            (Some("loss".to_string()), Some(true))
        );
        assert_eq!(
            outcome(&list, jean("2030-01-05 12:00:00")),
            (None, Some(false))
        );
        // Outside the window, or for a record without an item_id, the usual rule still applies.
        assert_eq!(
            outcome(&list, jean("2030-02-05 12:00:00")).0,
            Some("loss".to_string())
        );
        let mut by_name = jean("2030-01-05 12:00:00");
        by_name.item_id = String::new();
        by_name.name = "Jean".into();
        assert_eq!(outcome(&list, by_name).0, Some("loss".to_string()));
    }

    #[test]
    fn banner_list_features_apply_to_weapon_banners_too() {
        let list = vec![event(
            "atlas-rerun",
            "hk4e",
            "weapon",
            "2030-01-01 18:00:00",
            "2030-01-21 17:59:59",
            &["14501"],
        )];
        let pulls = [
            at(pull("hk4e", "302", 1, 5, "14501"), "2030-01-05 12:00:00"),
            at(pull("hk4e", "302", 2, 5, "14501"), "2030-03-05 12:00:00"),
        ];
        let r = compute_with_banners(Game::Hk4e, &pulls, &list, &RuleSet::bundled());
        let fives = &find(&r, "weapon").recent_fives;
        assert_eq!(fives[0].outcome, None);
        assert_eq!(fives[1].outcome.as_deref(), Some("loss"));
    }

    #[test]
    fn a_known_five_star_places_every_pull_sharing_its_gacha_id() {
        let list = vec![
            event(
                "one",
                "nap",
                "character",
                "2026-01-01 12:00:00",
                "2026-01-21 11:59:59",
                &["10"],
            ),
            event(
                "two",
                "nap",
                "character",
                "2026-01-01 12:00:00",
                "2026-01-21 11:59:59",
                &["20"],
            ),
        ];
        let with = |id: u32, rank: u8, item: &str, gacha_id: &str| {
            let mut pull = at(pull("nap", "2", id, rank, item), "2026-01-05 12:00:00");
            pull.gacha_id = gacha_id.into();
            pull
        };
        let pulls = vec![
            with(1, 2, "x", "2001"),
            with(2, 2, "x", "2002"),
            with(3, 2, "x", "3000"),
            with(4, 4, "10", "2001"),
            with(5, 4, "20", "2002"),
        ];
        let ids: Vec<Option<String>> = resolve_banners(&list, Game::Nap, &pulls)
            .iter()
            .map(|b| b.map(|b| b.id.clone()))
            .collect();
        let expected: Vec<Option<String>> = vec![
            Some("one".into()),
            Some("two".into()),
            None,
            Some("one".into()),
            Some("two".into()),
        ];
        assert_eq!(ids, expected);
        // One gacha_id claimed by two different banners teaches nothing.
        let clash = vec![
            with(1, 4, "10", "2001"),
            with(2, 4, "20", "2001"),
            with(3, 2, "x", "2001"),
        ];
        let ids = resolve_banners(&list, Game::Nap, &clash);
        assert!(ids[2].is_none());
    }

    #[test]
    fn a_gacha_id_of_zero_never_identifies_a_banner() {
        let list = vec![
            event(
                "one",
                "nap",
                "character",
                "2026-01-01 12:00:00",
                "2026-01-21 11:59:59",
                &["10"],
            ),
            event(
                "two",
                "nap",
                "character",
                "2026-01-05 12:00:00",
                "2026-01-21 11:59:59",
                &["20"],
            ),
        ];
        let zero = |id: u32, item: &str, time: &str| {
            let mut pull = at(pull("nap", "2", id, 4, item), time);
            pull.gacha_id = "0".into();
            pull
        };
        // Only the featured 5-star pull is placed; the "0" it shares with the others teaches nothing.
        let pulls = vec![
            zero(1, "x", "2026-01-02 12:00:00"),
            zero(2, "10", "2026-01-06 12:00:00"),
            zero(3, "x", "2026-01-07 12:00:00"),
        ];
        let ids: Vec<Option<String>> = resolve_banners(&list, Game::Nap, &pulls)
            .iter()
            .map(|b| b.map(|b| b.id.clone()))
            .collect();
        let expected: Vec<Option<String>> = vec![Some("one".into()), Some("one".into()), None];
        assert_eq!(ids, expected);
    }

    #[test]
    fn outcomes_and_limits_follow_the_rules_data_not_code() {
        let pulls = [p(1, 5, "diluc"), p(2, 5, "hu_tao")];
        let r = compute_with_banners(Game::Hk4e, &pulls, &[], &RuleSet::bundled());
        let fives = &find(&r, "character").recent_fives;
        assert_eq!(fives[0].outcome.as_deref(), Some("loss"));
        assert_eq!(fives[1].outcome.as_deref(), Some("guaranteed"));
        assert_eq!(find(&r, "character").hard_pity, 90);

        // A rules update changes the result with no code change: here Diluc leaves the
        // permanent pool and the character banner's hard pity moves.
        let mut rules = RuleSet::bundled();
        rules.game_mut(Game::Hk4e).standard.character = crate::rules::ItemSet::default();
        rules.game_mut(Game::Hk4e).banners[0].hard = 50;
        let r = compute_with_banners(Game::Hk4e, &pulls, &[], &rules);
        assert_eq!(find(&r, "character").recent_fives[0].outcome, None);
        assert_eq!(find(&r, "character").hard_pity, 50);
    }

    #[test]
    fn per_pull_pity_and_the_banner_summary_come_from_one_walk() {
        let pulls: Vec<Pull> = (1..=25)
            .map(|i| {
                p(
                    i,
                    if i == 7 {
                        4
                    } else if i == 20 {
                        5
                    } else {
                        3
                    },
                    "x",
                )
            })
            .collect();
        let per_pull = pull_pities(Game::Hk4e, &pulls);
        assert_eq!(per_pull[6], Some(7), "4★ at pull 7");
        assert_eq!(per_pull[19], Some(20), "a 4★ does not reset the 5★ counter");
        assert_eq!(per_pull[0], None);
        let r = compute(Game::Hk4e, &pulls);
        let banner = find(&r, "character");
        assert_eq!(banner.recent_fours[0].pity, 7);
        assert_eq!(banner.recent_fives[0].pity, 20);
        assert_eq!(banner.pity, 5, "pulls since the 5★");
        // Unknown pools are skipped by both.
        let odd = [pull("hk4e", "999", 1, 5, "x")];
        assert_eq!(pull_pities(Game::Hk4e, &odd), vec![None]);
    }

    #[test]
    fn ui_names_exist_for_every_pool_the_api_reports() {
        for game in Game::ALL {
            for gacha_type in game.gacha_types() {
                assert!(pool_label(game, gacha_type).is_some(), "{gacha_type}");
                if let Some(key) = banner_key(game, gacha_type) {
                    assert!(
                        history_filters(game).iter().any(|(k, _)| *k == key),
                        "{key} has no history filter"
                    );
                }
            }
        }
    }
}
