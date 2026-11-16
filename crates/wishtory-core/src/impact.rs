//! What a catalog release would do to the person's saved pulls, worked out on this PC.
//!
//! For every saved pull the facts that depend on game data are derived twice, under the data in
//! use and under the staged release, by the same functions the History and Pity pages use
//! (`pity::resolve_banners`, `pity::compute_with_banners`, `pity::pull_pities`): the banner the
//! pull was made on, its won / lost / guaranteed result, its pity number, and the name and rarity
//! shown for it. Every pull where anything differs is listed with the reason and the change log
//! entries behind it. Nothing is written, and nothing here ever leaves the PC.

use crate::{
    catalog::{BannerSet, Catalog, ChangeEntry, ChangeScope, Severity},
    db::Db,
    error::AppError,
    models::{Game, Pull},
    pity,
    rules::RuleSet,
};
use ring::digest;
use std::collections::HashMap;

/// At most this many affected pulls are kept for listing, over all accounts. Pulls beyond it are
/// still counted, so the number the person sees is always the real one.
pub const MAX_LISTED: usize = 5_000;

/// The game data one side of the comparison reads.
#[derive(Clone, Copy)]
pub struct Data<'a> {
    pub version: u64,
    pub catalog: &'a Catalog,
    pub banners: &'a BannerSet,
    pub rules: &'a RuleSet,
}

/// What is shown for an item: the catalog's name and rarity, or the saved record's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub name: String,
    pub rarity: u8,
}

/// Everything a release can change about one pull.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullFacts {
    /// Banner id and name.
    pub banner: Option<(String, String)>,
    /// "win", "loss" or "guaranteed" for a 5★ on a banner that tracks results.
    pub result: Option<String>,
    pub pity: Option<u32>,
    pub shown: Shown,
}

/// The facts of every pull of one account (oldest first, as `Db::pulls_for` returns them) under
/// one set of game data. This is the single derivation both sides of the comparison use.
pub fn derive(game: Game, pulls: &[Pull], data: Data<'_>) -> Vec<PullFacts> {
    let placed = pity::resolve_banners(&data.banners.banners, game, pulls);
    let pities = pity::pull_pities(game, pulls);
    // `compute_with_banners` walks the same pulls and lists each banner's 5★ in order.
    let mut fives: HashMap<String, std::vec::IntoIter<pity::FiveStar>> =
        pity::compute_with_banners(game, pulls, &data.banners.banners, data.rules)
            .into_iter()
            .map(|b| (b.banner, b.recent_fives.into_iter()))
            .collect();
    let names: HashMap<&str, (&str, u8)> = data
        .catalog
        .items
        .iter()
        .filter(|i| i.game == game.as_str())
        .map(|i| (i.id.as_str(), (i.name.as_str(), i.rarity)))
        .collect();
    pulls
        .iter()
        .zip(placed)
        .zip(pities)
        .map(|((pull, banner), pity)| {
            let rank = pity::display_rank(game, pull.rank_type);
            let result = pity::banner_key(game, &pull.gacha_type)
                .filter(|_| rank == 5)
                .and_then(|key| fives.get_mut(key)?.next())
                .and_then(|five| five.outcome);
            let shown = match names.get(pull.item_id.as_str()) {
                Some((name, rarity)) if !pull.item_id.is_empty() => Shown {
                    name: (*name).to_string(),
                    rarity: *rarity,
                },
                _ => Shown {
                    name: pull.name.clone(),
                    rarity: rank,
                },
            };
            PullFacts {
                banner: banner.map(|b| (b.id.clone(), b.name.clone())),
                result,
                pity,
                shown,
            }
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reason {
    /// Plain language, for the review page.
    pub text: String,
    /// The change log entries behind it.
    pub change_ids: Vec<String>,
}

/// One saved pull that the release would read differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffectedPull {
    pub game: String,
    pub uid: String,
    pub pull_id: String,
    pub time: String,
    pub item_id: String,
    pub name: String,
    pub rank: u8,
    pub banner_before: Option<String>,
    pub banner_after: Option<String>,
    pub result_before: Option<String>,
    pub result_after: Option<String>,
    pub pity_before: Option<u32>,
    pub pity_after: Option<u32>,
    pub shown_before: Shown,
    pub shown_after: Shown,
    pub reasons: Vec<Reason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountImpact {
    pub game: String,
    pub uid: String,
    pub nickname: String,
    pub pulls: u64,
    pub affected: u64,
    /// How many of the affected pulls are kept in `Impact::pulls`.
    pub listed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Impact {
    pub current_version: u64,
    pub staged_version: u64,
    /// Changes when the saved pulls or either release changes; Apply checks it.
    pub fingerprint: String,
    pub accounts: Vec<AccountImpact>,
    pub total_affected: u64,
    /// Affected pulls beyond the cap, counted but not listed.
    pub not_listed: u64,
    /// Account by account, oldest pull first.
    pub pulls: Vec<AffectedPull>,
}

fn game_of(text: &str) -> Option<Game> {
    Game::ALL.iter().copied().find(|g| g.as_str() == text)
}

/// A short stable fingerprint of what the analysis depends on besides the data itself: the two
/// release versions and, per account, how many pulls there are and which are the newest.
pub fn fingerprint(db: &Db, current: u64, staged: u64) -> Result<String, AppError> {
    let mut text = format!("{current}>{staged}");
    for a in db.pull_fingerprints()? {
        text.push_str(&format!(
            "|{}/{}/{}/{}/{}",
            a.game, a.uid, a.pulls, a.newest_time, a.newest_id
        ));
    }
    let hash = digest::digest(&digest::SHA256, text.as_bytes());
    Ok(hash.as_ref().iter().map(|b| format!("{b:02x}")).collect())
}

fn result_label(result: &Option<String>) -> &'static str {
    match result.as_deref() {
        Some("win") => "won",
        Some("loss") => "lost",
        Some("guaranteed") => "guaranteed",
        _ => "not tracked",
    }
}

/// Change log entries of `scope` for `game` that name one of `ids`. A grouped entry names only
/// some of what it covers, so when nothing names the pull the entries of that scope and game that
/// can matter are given instead.
fn linked(changes: &[ChangeEntry], game: &str, scope: ChangeScope, ids: &[&str]) -> Vec<String> {
    let of_scope = || {
        changes
            .iter()
            .filter(move |c| c.game == game && c.scope == scope)
    };
    let named: Vec<String> = of_scope()
        .filter(|c| c.ids.iter().any(|id| ids.contains(&id.as_str())))
        .map(|c| c.id.clone())
        .collect();
    if !named.is_empty() {
        return named;
    }
    of_scope()
        .filter(|c| c.severity != Severity::Info)
        .map(|c| c.id.clone())
        .take(5)
        .collect()
}

fn reasons(
    pull: &Pull,
    game: Game,
    before: &PullFacts,
    after: &PullFacts,
    changes: &[ChangeEntry],
) -> Vec<Reason> {
    let mut out = Vec::new();
    let game_id = game.as_str();
    let banner_ids: Vec<&str> = [&before.banner, &after.banner]
        .into_iter()
        .flatten()
        .map(|(id, _)| id.as_str())
        .collect();
    if before.banner != after.banner {
        let text = match (&before.banner, &after.banner) {
            (None, Some((_, to))) => {
                format!(
                    "The banner list now places this pull on “{to}”. Before, it could not be placed."
                )
            }
            (Some((_, from)), None) => format!(
                "The banner list no longer places this pull on a banner. It was on “{from}”."
            ),
            (Some((_, from)), Some((_, to))) => {
                format!("The banner list moves this pull from “{from}” to “{to}”.")
            }
            (None, None) => unreachable!("equal banners are not a difference"),
        };
        out.push(Reason {
            text,
            change_ids: linked(changes, game_id, ChangeScope::Banners, &banner_ids),
        });
    }
    if before.result != after.result {
        let key = pity::banner_key(game, &pull.gacha_type).unwrap_or("");
        let mut ids = banner_ids.clone();
        ids.push(pull.item_id.as_str());
        let mut change_ids = linked(changes, game_id, ChangeScope::Rules, &[key, &pull.item_id]);
        for id in linked(changes, game_id, ChangeScope::Banners, &ids) {
            if !change_ids.contains(&id) {
                change_ids.push(id);
            }
        }
        out.push(Reason {
            text: format!(
                "Its result changes from {} to {}, because the featured items or the pity rules changed.",
                result_label(&before.result),
                result_label(&after.result)
            ),
            change_ids,
        });
    }
    if before.pity != after.pity {
        let show = |p: Option<u32>| p.map_or("none".to_string(), |p| p.to_string());
        out.push(Reason {
            text: format!(
                "Its pity number changes from {} to {}.",
                show(before.pity),
                show(after.pity)
            ),
            change_ids: linked(changes, game_id, ChangeScope::Rules, &[]),
        });
    }
    if before.shown != after.shown {
        let mut parts = Vec::new();
        if before.shown.name != after.shown.name {
            parts.push(format!(
                "its name is shown as “{}” instead of “{}”",
                after.shown.name, before.shown.name
            ));
        }
        if before.shown.rarity != after.shown.rarity {
            parts.push(format!(
                "it is shown as {}-star instead of {}-star",
                after.shown.rarity, before.shown.rarity
            ));
        }
        let text = parts.join(", and ");
        let mut chars = text.chars();
        let text = chars
            .next()
            .map(|c| c.to_uppercase().chain(chars).collect::<String>())
            .unwrap_or_default();
        out.push(Reason {
            text: format!("{text}."),
            change_ids: linked(changes, game_id, ChangeScope::Items, &[&pull.item_id]),
        });
    }
    out
}

/// Compares every saved pull under `now` and `next`. `changes` is the staged release's change
/// log, used to explain each difference. At most `cap` affected pulls are listed; the rest are
/// counted. Memory is bounded by the largest account plus the cap, not by the whole history.
pub fn compute(
    db: &Db,
    now: Data<'_>,
    next: Data<'_>,
    changes: &[ChangeEntry],
    cap: usize,
) -> Result<Impact, AppError> {
    let mut accounts = Vec::new();
    let mut listed: Vec<AffectedPull> = Vec::new();
    let mut total = 0u64;
    for account in db.list_accounts()? {
        let Some(game) = game_of(&account.game) else {
            continue;
        };
        let pulls = db.pulls_for(&account.game, &account.uid)?;
        let before = derive(game, &pulls, now);
        let after = derive(game, &pulls, next);
        let (mut affected, mut kept) = (0u64, 0u64);
        for ((pull, was), will) in pulls.iter().zip(&before).zip(&after) {
            if was == will {
                continue;
            }
            affected += 1;
            if listed.len() >= cap {
                continue;
            }
            kept += 1;
            listed.push(AffectedPull {
                game: pull.game.clone(),
                uid: pull.uid.clone(),
                pull_id: pull.id.clone(),
                time: pull.time.clone(),
                item_id: pull.item_id.clone(),
                name: pull.name.clone(),
                rank: pity::display_rank(game, pull.rank_type),
                banner_before: was.banner.as_ref().map(|(_, name)| name.clone()),
                banner_after: will.banner.as_ref().map(|(_, name)| name.clone()),
                result_before: was.result.clone(),
                result_after: will.result.clone(),
                pity_before: was.pity,
                pity_after: will.pity,
                shown_before: was.shown.clone(),
                shown_after: will.shown.clone(),
                reasons: reasons(pull, game, was, will, changes),
            });
        }
        total += affected;
        accounts.push(AccountImpact {
            game: account.game,
            uid: account.uid,
            nickname: account.nickname,
            pulls: pulls.len() as u64,
            affected,
            listed: kept,
        });
    }
    Ok(Impact {
        current_version: now.version,
        staged_version: next.version,
        fingerprint: fingerprint(db, now.version, next.version)?,
        accounts,
        total_affected: total,
        not_listed: total - listed.len() as u64,
        pulls: listed,
    })
}

impl Impact {
    /// A page of the affected pulls of one account.
    pub fn page(&self, game: &str, uid: &str, offset: usize, limit: usize) -> Vec<AffectedPull> {
        self.pulls
            .iter()
            .filter(|p| p.game == game && p.uid == uid)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn affected_accounts(&self) -> u32 {
        self.accounts.iter().filter(|a| a.affected > 0).count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{catalog::Manifest, models::Pull};
    use std::path::PathBuf;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("wishtory-impact-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn pull(id: u32, gacha_type: &str, rank: u8, item: &str, time: &str) -> Pull {
        Pull {
            id: id.to_string(),
            game: "hk4e".into(),
            uid: "1".into(),
            gacha_type: gacha_type.into(),
            gacha_id: String::new(),
            item_id: item.into(),
            name: format!("name-{item}"),
            item_type: "Character".into(),
            rank_type: rank,
            time: time.into(),
            lang: "en".into(),
            count: 1,
        }
    }

    fn banner_set(version: u64, json: &str) -> BannerSet {
        let text = format!(
            r#"{{"schema_version":1,"banners_version":{version},"generated_at":"2026-01-01T00:00:00Z","banners":[{json}]}}"#
        );
        BannerSet::parse_and_validate(text.as_bytes()).unwrap()
    }

    fn event(id: &str, start: &str, end: &str, featured: &str) -> String {
        format!(
            r#"{{"id":"{id}","game":"hk4e","pool":"character","name":"Event {id}","start":"{start}","end":"{end}","featured_5":["{featured}"]}}"#
        )
    }

    struct Rig {
        db: Db,
        dir: PathBuf,
        catalog: Catalog,
        rules: RuleSet,
    }

    impl Rig {
        fn new(tag: &str, pulls: Vec<Pull>) -> Rig {
            let dir = scratch(tag);
            let db = Db::open(&dir.join("wishtory.db")).unwrap();
            db.merge("hk4e", "1", 8, &pulls).unwrap();
            Rig {
                db,
                dir,
                catalog: Catalog::bundled(),
                rules: RuleSet::bundled(),
            }
        }

        fn impact(&self, now: &BannerSet, next: &BannerSet) -> Impact {
            self.impact_with(
                Data {
                    version: 1,
                    catalog: &self.catalog,
                    banners: now,
                    rules: &self.rules,
                },
                Data {
                    version: 2,
                    catalog: &self.catalog,
                    banners: next,
                    rules: &self.rules,
                },
                MAX_LISTED,
            )
        }

        fn impact_with(&self, now: Data<'_>, next: Data<'_>, cap: usize) -> Impact {
            compute(&self.db, now, next, &[], cap).unwrap()
        }

        fn dump(&self) -> Vec<Pull> {
            self.db.pulls_for("hk4e", "1").unwrap()
        }
    }

    impl Drop for Rig {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    // Diluc (10000016) and Jean (10000003) are permanent-pool 5★ in the bundled rules.
    const DILUC: &str = "10000016";

    #[test]
    fn a_moved_window_re_places_exactly_the_pulls_it_covers() {
        let pulls = vec![
            pull(1, "301", 3, "x", "2026-01-05 12:00:00"),
            pull(2, "301", 3, "x", "2026-01-21 12:00:00"),
            pull(3, "301", 3, "x", "2026-02-10 12:00:00"),
        ];
        let rig = Rig::new("window", pulls);
        let now = banner_set(
            1,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-20 17:59:59",
                "10000022",
            ),
        );
        let next = banner_set(
            2,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-31 17:59:59",
                "10000022",
            ),
        );
        let impact = rig.impact(&now, &next);
        assert_eq!(impact.total_affected, 1);
        let affected = &impact.pulls[0];
        assert_eq!(affected.pull_id, "2");
        assert_eq!(affected.banner_before, None);
        assert_eq!(affected.banner_after.as_deref(), Some("Event e1"));
        assert!(affected.reasons[0].text.contains("now places this pull"));
        assert_eq!(impact.accounts[0].affected, 1);
        assert_eq!(impact.accounts[0].pulls, 3);
    }

    #[test]
    fn a_featured_change_flips_a_lost_result() {
        // Diluc is a permanent-pool 5★: a loss, unless the banner list features him there.
        let rig = Rig::new(
            "featured",
            vec![pull(1, "301", 5, DILUC, "2026-01-05 12:00:00")],
        );
        let now = banner_set(
            1,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-20 17:59:59",
                "10000022",
            ),
        );
        let next = banner_set(
            2,
            &event("e1", "2026-01-01 18:00:00", "2026-01-20 17:59:59", DILUC),
        );
        let impact = rig.impact(&now, &next);
        assert_eq!(impact.total_affected, 1);
        let affected = &impact.pulls[0];
        assert_eq!(affected.result_before.as_deref(), Some("loss"));
        assert_eq!(affected.result_after, None);
        assert_eq!(affected.banner_before, affected.banner_after);
        assert!(
            affected
                .reasons
                .iter()
                .any(|r| r.text.contains("lost to not tracked"))
        );
    }

    #[test]
    fn a_rules_change_flips_a_result_but_never_a_pity_number() {
        let rig = Rig::new(
            "rules",
            vec![pull(1, "301", 5, DILUC, "2026-01-05 12:00:00")],
        );
        let banners = banner_set(
            1,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-20 17:59:59",
                "10000022",
            ),
        );
        let mut changed = rig.rules.clone();
        {
            let pool = &mut changed.game_mut(Game::Hk4e).standard.character;
            pool.ids
                .retain(|id| !id.eq_ignore_ascii_case(DILUC) && id != "diluc");
            pool.names.retain(|n| n != "Diluc");
        }
        let impact = rig.impact_with(
            Data {
                version: 1,
                catalog: &rig.catalog,
                banners: &banners,
                rules: &rig.rules,
            },
            Data {
                version: 2,
                catalog: &rig.catalog,
                banners: &banners,
                rules: &changed,
            },
            MAX_LISTED,
        );
        assert_eq!(impact.total_affected, 1);
        let affected = &impact.pulls[0];
        assert_eq!(affected.result_before.as_deref(), Some("loss"));
        assert_ne!(affected.result_after.as_deref(), Some("loss"));
        // Pity numbers come from the order of the pulls and their pool, which no feed changes.
        assert_eq!(affected.pity_before, affected.pity_after);
    }

    #[test]
    fn a_rarity_or_name_fix_changes_what_is_shown_for_pulls_of_that_item() {
        let rig = Rig::new(
            "shown",
            vec![
                pull(1, "301", 4, DILUC, "2026-01-05 12:00:00"),
                pull(2, "301", 3, "someone-else", "2026-01-05 12:01:00"),
            ],
        );
        let banners = banner_set(
            1,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-20 17:59:59",
                "10000022",
            ),
        );
        let mut next_catalog = rig.catalog.clone();
        let item = next_catalog
            .items
            .iter_mut()
            .find(|i| i.game == "hk4e" && i.id == DILUC)
            .unwrap();
        item.rarity = 4;
        item.name = "Diluc Ragnvindr".into();
        let impact = rig.impact_with(
            Data {
                version: 1,
                catalog: &rig.catalog,
                banners: &banners,
                rules: &rig.rules,
            },
            Data {
                version: 2,
                catalog: &next_catalog,
                banners: &banners,
                rules: &rig.rules,
            },
            MAX_LISTED,
        );
        assert_eq!(impact.total_affected, 1);
        let affected = &impact.pulls[0];
        assert_eq!(affected.pull_id, "1");
        assert_eq!(affected.shown_before.rarity, 5);
        assert_eq!(affected.shown_after.rarity, 4);
        assert_eq!(affected.shown_after.name, "Diluc Ragnvindr");
        assert!(affected.reasons[0].text.contains("shown as"));
    }

    #[test]
    fn an_unchanged_history_reports_nothing_and_pulls_stay_as_they_were() {
        let rig = Rig::new(
            "same",
            vec![
                pull(1, "301", 5, DILUC, "2026-01-05 12:00:00"),
                pull(2, "301", 3, "x", "2026-01-06 12:00:00"),
            ],
        );
        let now = banner_set(
            1,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-20 17:59:59",
                "10000022",
            ),
        );
        // A new banner far from every saved pull is an addition that touches none of them.
        let next = banner_set(
            2,
            &format!(
                "{},{}",
                event(
                    "e1",
                    "2026-01-01 18:00:00",
                    "2026-01-20 17:59:59",
                    "10000022"
                ),
                event(
                    "e9",
                    "2030-01-01 18:00:00",
                    "2030-01-20 17:59:59",
                    "10000022"
                )
            ),
        );
        let before = rig.dump();
        let impact = rig.impact(&now, &next);
        assert_eq!(impact.total_affected, 0);
        assert!(impact.pulls.is_empty());
        assert_eq!(impact.not_listed, 0);
        assert_eq!(rig.dump().len(), before.len());
        assert!(
            rig.dump()
                .iter()
                .zip(&before)
                .all(|(a, b)| a.id == b.id && a.time == b.time && a.name == b.name)
        );
    }

    #[test]
    fn the_list_is_capped_but_the_count_is_real() {
        let pulls: Vec<Pull> = (1..=6)
            .map(|i| pull(i, "301", 3, "x", &format!("2026-01-{:02} 12:00:00", i + 1)))
            .collect();
        let rig = Rig::new("cap", pulls);
        let now = banner_set(
            1,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-01 19:00:00",
                "10000022",
            ),
        );
        let next = banner_set(
            2,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-31 17:59:59",
                "10000022",
            ),
        );
        let impact = rig.impact_with(
            Data {
                version: 1,
                catalog: &rig.catalog,
                banners: &now,
                rules: &rig.rules,
            },
            Data {
                version: 2,
                catalog: &rig.catalog,
                banners: &next,
                rules: &rig.rules,
            },
            2,
        );
        assert_eq!(impact.total_affected, 6);
        assert_eq!(impact.pulls.len(), 2);
        assert_eq!(impact.not_listed, 4);
        assert_eq!(impact.accounts[0].listed, 2);
        assert_eq!(impact.page("hk4e", "1", 1, 10).len(), 1);
        assert_eq!(impact.page("hk4e", "1", 0, 10)[0].pull_id, "1");
        assert!(impact.page("nap", "1", 0, 10).is_empty());
    }

    #[test]
    fn the_fingerprint_follows_the_saved_pulls_and_the_releases() {
        let rig = Rig::new("print", vec![pull(1, "301", 3, "x", "2026-01-05 12:00:00")]);
        let a = fingerprint(&rig.db, 1, 2).unwrap();
        assert_eq!(a, fingerprint(&rig.db, 1, 2).unwrap());
        assert_ne!(a, fingerprint(&rig.db, 1, 3).unwrap());
        rig.db
            .merge(
                "hk4e",
                "1",
                8,
                &[pull(2, "301", 3, "x", "2026-01-06 12:00:00")],
            )
            .unwrap();
        assert_ne!(a, fingerprint(&rig.db, 1, 2).unwrap());
    }

    #[test]
    fn explanations_name_the_change_entries_behind_a_difference() {
        use crate::catalog::{ChangeEntry, ChangeScope, Severity};
        let rig = Rig::new("link", vec![pull(1, "301", 3, "x", "2026-01-21 12:00:00")]);
        let now = banner_set(
            1,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-20 17:59:59",
                "10000022",
            ),
        );
        let next = banner_set(
            2,
            &event(
                "e1",
                "2026-01-01 18:00:00",
                "2026-01-31 17:59:59",
                "10000022",
            ),
        );
        let changes = vec![
            ChangeEntry {
                id: "banners:window:hk4e:e1".into(),
                scope: ChangeScope::Banners,
                game: "hk4e".into(),
                ids: vec!["e1".into()],
                summary: "The dates changed.".into(),
                severity: Severity::Major,
                note: None,
            },
            ChangeEntry {
                id: "banners:window:hk4e:other".into(),
                scope: ChangeScope::Banners,
                game: "hk4e".into(),
                ids: vec!["other".into()],
                summary: "Another banner.".into(),
                severity: Severity::Major,
                note: None,
            },
        ];
        let impact = compute(
            &rig.db,
            Data {
                version: 1,
                catalog: &rig.catalog,
                banners: &now,
                rules: &rig.rules,
            },
            Data {
                version: 2,
                catalog: &rig.catalog,
                banners: &next,
                rules: &rig.rules,
            },
            &changes,
            MAX_LISTED,
        )
        .unwrap();
        assert_eq!(
            impact.pulls[0].reasons[0].change_ids,
            vec!["banners:window:hk4e:e1".to_string()]
        );
        let _ = Manifest::bundled();
    }
}
