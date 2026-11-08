//! Conversions between `wishtory-core` types and the bridge types in `api::types`.

use crate::api::types::*;
use wishtory_core::{
    about::{About as CoreAbout, Credit as CoreCredit, CreditGroup as CoreCreditGroup},
    catalog::{BannerEntry as CoreBanner, BannerSet, BannerUpdate as CoreBannerUpdate, Catalog},
    logging::LogEntry as CoreLogEntry,
    models::{
        Account as CoreAccount, Game, GameInstall as CoreInstall, GameLocations as CoreLocations,
    },
    ops::{FetchSummary as CoreFetch, GameMeta as CoreGameMeta, HistoryPull as CoreHistoryPull},
    pity::{BannerPity as CoreBannerPity, FiveStar as CoreFiveStar, FourStar as CoreFourStar},
    uigf::ImportSummary as CoreImport,
    update::AppUpdate as CoreAppUpdate,
};

impl From<Game> for GameId {
    fn from(g: Game) -> Self {
        match g {
            Game::Hk4e => GameId::Genshin,
            Game::Hkrpg => GameId::StarRail,
            Game::Nap => GameId::Zenless,
        }
    }
}

impl From<GameId> for Game {
    fn from(g: GameId) -> Self {
        match g {
            GameId::Genshin => Game::Hk4e,
            GameId::StarRail => Game::Hkrpg,
            GameId::Zenless => Game::Nap,
        }
    }
}

/// The core stores games as their short ids in records; `None` for a value this build does not know.
pub(crate) fn game_from_str(s: &str) -> Option<GameId> {
    Game::ALL
        .iter()
        .find(|g| g.as_str() == s)
        .map(|g| GameId::from(*g))
}

impl From<CoreInstall> for GameInstall {
    fn from(i: CoreInstall) -> Self {
        Self {
            game: i.game.into(),
            found: i.found,
            log_path: i.log_path,
            data_dir: i.data_dir,
        }
    }
}

impl From<CoreLocations> for GameLocations {
    fn from(l: CoreLocations) -> Self {
        Self {
            supported: l.supported,
            locations: l.locations,
            detected: l
                .detected
                .into_iter()
                .map(|d| DetectedPrefix {
                    path: d.path,
                    games: d.games.into_iter().map(GameId::from).collect(),
                })
                .collect(),
        }
    }
}

impl Account {
    /// `None` when the stored game is one this build does not know (such rows are dropped).
    pub(crate) fn from_core(a: CoreAccount) -> Option<Self> {
        Some(Self {
            game: game_from_str(&a.game)?,
            uid: a.uid,
            nickname: a.nickname,
            timezone: a.timezone,
            pull_count: a.pull_count,
        })
    }
}

impl From<CoreHistoryPull> for HistoryPull {
    fn from(h: CoreHistoryPull) -> Self {
        let banner_key = game_from_str(&h.pull.game)
            .and_then(|g| wishtory_core::pity::banner_key(g.into(), &h.pull.gacha_type))
            .unwrap_or_default()
            .to_string();
        Self {
            id: h.pull.id,
            gacha_type: h.pull.gacha_type,
            gacha_id: h.pull.gacha_id,
            item_id: h.pull.item_id,
            name: h.pull.name,
            item_type: h.pull.item_type,
            time: h.pull.time,
            rank: h.rank,
            banner_key,
            pool: h.pool,
            banner_name: h.banner_name,
            pity: h.pity,
        }
    }
}

impl From<CoreFiveStar> for FiveStar {
    fn from(f: CoreFiveStar) -> Self {
        Self {
            name: f.name,
            pity: f.pity,
            time: f.time,
            off_banner: f.off_banner,
            outcome: f.outcome.as_deref().and_then(|o| match o {
                "win" => Some(PullOutcome::Win),
                "loss" => Some(PullOutcome::Loss),
                "guaranteed" => Some(PullOutcome::Guaranteed),
                _ => None,
            }),
        }
    }
}

impl From<CoreFourStar> for FourStar {
    fn from(f: CoreFourStar) -> Self {
        Self {
            name: f.name,
            pity: f.pity,
            time: f.time,
        }
    }
}

impl From<CoreBannerPity> for BannerPity {
    fn from(p: CoreBannerPity) -> Self {
        Self {
            banner: p.banner,
            label: p.label,
            pity: p.pity,
            hard_pity: p.hard_pity,
            soft_pity: p.soft_pity,
            guaranteed: p.guaranteed,
            guarantee_odds: p.guarantee_odds,
            total: p.total,
            five_stars: p.five_stars,
            four_stars: p.four_stars,
            avg_five_pity: p.avg_five_pity,
            avg_four_pity: p.avg_four_pity,
            recent_fives: p.recent_fives.into_iter().map(Into::into).collect(),
            recent_fours: p.recent_fours.into_iter().map(Into::into).collect(),
        }
    }
}

impl GameMeta {
    pub(crate) fn from_core(m: CoreGameMeta) -> Option<Self> {
        Some(Self {
            game: game_from_str(m.game)?,
            banners: m
                .banners
                .into_iter()
                .map(|b| BannerFilter {
                    key: b.key.to_string(),
                    label: b.label.to_string(),
                })
                .collect(),
        })
    }
}

impl From<&Catalog> for GameCatalog {
    fn from(c: &Catalog) -> Self {
        Self {
            catalog_version: c.catalog_version as i64,
            generated_at: c.generated_at.clone(),
            items: c
                .items
                .iter()
                .map(|i| CatalogItem {
                    game: i.game.clone(),
                    id: i.id.clone(),
                    name: i.name.clone(),
                    aliases: i.aliases.clone(),
                    rarity: i.rarity,
                    kind: i.kind.clone(),
                    paimon_key: i.paimon_key.clone(),
                })
                .collect(),
        }
    }
}

impl BannerEntry {
    fn from_core(b: &CoreBanner) -> Option<Self> {
        Some(Self {
            id: b.id.clone(),
            game: game_from_str(&b.game)?,
            pool: b.pool.clone(),
            name: b.name.clone(),
            gacha_ids: b.gacha_ids.clone(),
            start: b.start.clone(),
            end: b.end.clone(),
            featured_5: b.featured_5.clone(),
            featured_4: b.featured_4.clone(),
        })
    }
}

impl From<&BannerSet> for BannerList {
    fn from(s: &BannerSet) -> Self {
        Self {
            banners_version: s.banners_version as i64,
            generated_at: s.generated_at.clone(),
            banners: s
                .banners
                .iter()
                .filter_map(BannerEntry::from_core)
                .collect(),
        }
    }
}

impl From<CoreBannerUpdate> for BannerUpdate {
    fn from(u: CoreBannerUpdate) -> Self {
        Self {
            status: update_status(&u.status),
            banners_version: u.banners_version as i64,
            installed_at: u.installed_at as i64,
            added: u.added,
            changed: u.changed,
            removed: u.removed,
        }
    }
}

pub(crate) fn update_status(s: &str) -> UpdateStatus {
    match s {
        "updated" => UpdateStatus::Updated,
        "already-checking" => UpdateStatus::AlreadyChecking,
        _ => UpdateStatus::UpToDate,
    }
}

impl FetchSummary {
    pub(crate) fn from_core(f: CoreFetch) -> Option<Self> {
        Some(Self {
            game: game_from_str(&f.game)?,
            uid: f.uid,
            fetched: f.fetched as i64,
            added: f.added as i64,
            warning: f.warning,
        })
    }
}

impl From<CoreImport> for ImportSummary {
    fn from(i: CoreImport) -> Self {
        Self {
            format: i.format,
            accounts: i.accounts as i64,
            added: i.added as i64,
            skipped: i.skipped as i64,
        }
    }
}

impl From<CoreAbout> for AboutInfo {
    fn from(a: CoreAbout) -> Self {
        let credit = |c: &CoreCredit| Credit {
            name: c.name.into(),
            role: c.role.into(),
            url: c.url.into(),
            license: c.license.into(),
        };
        let group = |g: &CoreCreditGroup| CreditGroup {
            title: g.title.into(),
            note: g.note.into(),
            credits: g.credits.iter().map(credit).collect(),
        };
        Self {
            name: a.name.into(),
            version: a.version.into(),
            description: a.description.into(),
            copyright: a.copyright.into(),
            license: a.license.into(),
            disclaimers: a.disclaimers.iter().map(|d| d.to_string()).collect(),
            repository: a.repository.into(),
            releases: a.releases.into(),
            issues: a.issues.into(),
            groups: a.groups.iter().map(group).collect(),
        }
    }
}

impl From<CoreAppUpdate> for AppUpdate {
    fn from(u: CoreAppUpdate) -> Self {
        Self {
            current: u.current,
            latest: u.latest,
            available: u.available,
            url: u.url,
            prerelease: u.prerelease,
            published_at: u.published_at,
        }
    }
}

impl From<CoreLogEntry> for LogEntry {
    fn from(l: CoreLogEntry) -> Self {
        Self {
            seq: l.seq as i64,
            time: l.time,
            level: l.level.into(),
            source: l.source,
            message: l.message,
        }
    }
}
