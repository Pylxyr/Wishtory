//! History download. The pasted/extracted URL is only used to read parameters; requests
//! always go to the official API base chosen below (never to a host from the input),
//! so the authkey cannot be redirected elsewhere (redirects are disabled as well).

use crate::{
    error::AppError,
    logging::Level,
    models::{Game, Pull},
};
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;
use url::Url;

const PAGE_SIZE: usize = 20;
/// Hard stop per banner (100 000 pulls). Protects against a server that never ends the list.
const MAX_PAGES: u32 = 5_000;

fn next_cursor(
    page_len: usize,
    last_id: Option<String>,
    current: &str,
) -> Result<Option<String>, AppError> {
    if page_len < PAGE_SIZE {
        return Ok(None);
    }
    let id = last_id.filter(|id| !id.is_empty()).ok_or_else(|| {
        AppError::Api("The server returned a full history page without a usable cursor.".into())
    })?;
    if id == current {
        return Err(AppError::Api(
            "The server repeated a history page; open history again and retry.".into(),
        ));
    }
    Ok(Some(id))
}

pub struct Fetched {
    pub uid: String,
    pub timezone: i32,
    pub pulls: Vec<Pull>,
    /// Set when the download stopped early. Only banners that finished are in `pulls`.
    pub incomplete: Option<String>,
}

#[derive(Deserialize)]
struct Resp {
    retcode: i64,
    #[serde(default)]
    message: String,
    data: Option<Data>,
}
#[derive(Deserialize)]
struct Data {
    #[serde(default)]
    list: Vec<Item>,
    region_time_zone: Option<i32>,
}
#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    uid: String,
    #[serde(default)]
    gacha_type: String,
    #[serde(default)]
    gacha_id: String,
    #[serde(default)]
    item_id: String,
    #[serde(default)]
    count: String,
    #[serde(default)]
    time: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    lang: String,
    #[serde(default)]
    item_type: String,
    #[serde(default)]
    rank_type: String,
    #[serde(default)]
    id: String,
}

fn api_base(game: Game, cn: bool) -> &'static str {
    match (game, cn) {
        (Game::Hk4e, false) => {
            "https://public-operation-hk4e-sg.hoyoverse.com/gacha_info/api/getGachaLog"
        }
        (Game::Hk4e, true) => "https://public-operation-hk4e.mihoyo.com/gacha_info/api/getGachaLog",
        (Game::Hkrpg, false) => {
            "https://public-operation-hkrpg-sg.hoyoverse.com/common/gacha_record/api/getGachaLog"
        }
        (Game::Hkrpg, true) => {
            "https://public-operation-hkrpg.mihoyo.com/common/gacha_record/api/getGachaLog"
        }
        (Game::Nap, false) => {
            "https://public-operation-nap-sg.hoyoverse.com/common/gacha_record/api/getGachaLog"
        }
        (Game::Nap, true) => {
            "https://public-operation-nap.mihoyo.com/common/gacha_record/api/getGachaLog"
        }
    }
}

/// ZZZ uses an internal channel id plus its normalized channel id. The endpoint
/// returns an empty list when only `real_gacha_type` is supplied.
fn gacha_params(game: Game, channel: &str) -> Vec<(&'static str, String)> {
    if game == Game::Nap {
        let internal = match channel {
            "1" => "1001",
            "2" => "2001",
            "3" => "3001",
            "5" => "5001",
            _ => channel,
        };
        vec![
            ("gacha_type", internal.to_string()),
            ("real_gacha_type", channel.to_string()),
            ("init_log_gacha_type", internal.to_string()),
            ("init_log_gacha_base_type", channel.to_string()),
        ]
    } else {
        vec![("gacha_type", channel.to_string())]
    }
}

fn is_hoyo_host(h: &str) -> bool {
    ["hoyoverse.com", "mihoyo.com"]
        .iter()
        .any(|d| h == *d || h.ends_with(&format!(".{d}")))
}

/// Fallback only: the API normally reports `region_time_zone` itself.
/// Covers the region names of all three games (os_usa / prod_official_usa / prod_gf_us, ...).
pub fn tz_from_region(region: &str) -> i32 {
    let r = region.to_ascii_lowercase();
    if r.ends_with("_usa") || r.ends_with("_us") {
        -5
    } else if r.ends_with("_euro") || r.ends_with("_eur") || r.ends_with("_eu") {
        1
    } else {
        8
    }
}

fn looks_like_auth_error(code: i64, message: &str) -> bool {
    matches!(code, -100 | -101) || message.to_ascii_lowercase().contains("authkey")
}

/// `progress` receives short, link-free status lines for the UI ("Downloading banner 2 of 5…").
pub async fn fetch_all(
    game: Game,
    history_url: &str,
    progress: &impl Fn(String),
) -> Result<Fetched, AppError> {
    let parsed = Url::parse(history_url.trim()).map_err(|_| AppError::InvalidUrl)?;
    if parsed.scheme() != "https" || !parsed.host_str().map(is_hoyo_host).unwrap_or(false) {
        return Err(AppError::InvalidUrl);
    }
    let p: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    let authkey = p
        .get("authkey")
        .filter(|k| !k.is_empty())
        .ok_or(AppError::InvalidUrl)?
        .clone();
    let biz = p.get("game_biz").cloned().unwrap_or_default();
    if !biz.starts_with(game.as_str()) {
        return Err(AppError::InvalidUrl);
    }
    let region = p.get("region").cloned().unwrap_or_default();
    let base = api_base(game, biz.ends_with("_cn"));

    let common: Vec<(&str, String)> = vec![
        (
            "authkey_ver",
            p.get("authkey_ver").cloned().unwrap_or_else(|| "1".into()),
        ),
        (
            "sign_type",
            p.get("sign_type").cloned().unwrap_or_else(|| "2".into()),
        ),
        (
            "auth_appid",
            p.get("auth_appid")
                .cloned()
                .unwrap_or_else(|| "webview_gacha".into()),
        ),
        (
            "lang",
            p.get("lang").cloned().unwrap_or_else(|| "en".into()),
        ),
        ("region", region.clone()),
        ("game_biz", biz),
        ("authkey", authkey),
    ];

    let client = reqwest::Client::builder()
        .user_agent(concat!("Wishtory/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none()) // the authkey must never follow a redirect
        .build()?;

    // Always a full fetch: ~6 months of data is quick, and merging by id is idempotent.
    // A banner is only kept once it has been read to the end, so pity counters never start
    // in the middle of a half-downloaded banner.
    let mut pulls = Vec::new();
    let mut tz: Option<i32> = None;
    let mut failure: Option<(AppError, usize)> = None;
    let total = game.gacha_types().len();
    for (done, gt) in game.gacha_types().iter().enumerate() {
        progress(if pulls.is_empty() {
            format!("Downloading banner {} of {total}…", done + 1)
        } else {
            format!(
                "Downloading banner {} of {total} · {} pulls so far",
                done + 1,
                pulls.len()
            )
        });
        match fetch_banner(&client, game, base, &common, gt).await {
            Ok((items, rtz)) => {
                wlog!(
                    Level::Debug,
                    "{}: banner {gt} read ({} pulls)",
                    game.as_str(),
                    items.len()
                );
                tz = tz.or(rtz);
                pulls.extend(items);
            }
            Err(e) => {
                wlog!(
                    Level::Warn,
                    "{}: banner {gt} stopped after {done} finished banner(s): {e}",
                    game.as_str()
                );
                failure = Some((e, done));
                break;
            }
        }
        sleep(Duration::from_millis(400)).await;
    }

    let incomplete = match failure {
        Some((e, _)) if pulls.is_empty() => return Err(e),
        Some((e, done)) => Some(format!(
            "{e} Saved {done} of {} banners; click Refresh again to fetch the rest.",
            game.gacha_types().len()
        )),
        None => None,
    };

    let uid = pulls
        .first()
        .map(|p| p.uid.clone())
        .ok_or_else(|| AppError::Api("No pulls were returned for this account yet.".into()))?;
    // An authkey belongs to exactly one account. Anything else would corrupt the database.
    pulls.retain(|p| p.uid == uid);
    Ok(Fetched {
        uid,
        timezone: tz.unwrap_or_else(|| tz_from_region(&region)),
        pulls,
        incomplete,
    })
}

async fn fetch_banner(
    client: &reqwest::Client,
    game: Game,
    base: &str,
    common: &[(&str, String)],
    gacha_type: &str,
) -> Result<(Vec<Pull>, Option<i32>), AppError> {
    let mut out = Vec::new();
    let mut tz = None;
    let mut end_id = "0".to_string();
    for page in 1..=MAX_PAGES {
        let (list, rtz) =
            request_page(client, game, base, common, gacha_type, page, &end_id).await?;
        tz = tz.or(rtz);
        let n = list.len();
        let last_id = list.last().map(|i| i.id.clone());
        for it in list {
            if it.id.is_empty() || it.time.is_empty() {
                continue;
            }
            out.push(Pull {
                id: it.id,
                game: game.as_str().into(),
                uid: it.uid,
                gacha_type: it.gacha_type,
                gacha_id: it.gacha_id,
                item_id: it.item_id,
                name: it.name,
                item_type: it.item_type,
                rank_type: it.rank_type.parse().unwrap_or(3),
                time: it.time,
                lang: it.lang,
                count: it.count.parse().unwrap_or(1),
            });
        }
        match next_cursor(n, last_id, &end_id)? {
            Some(id) if page < MAX_PAGES => end_id = id,
            Some(_) => {
                return Err(AppError::Api(
                    "This banner exceeds the 100,000-pull safety limit and was not fully fetched."
                        .into(),
                ))
            }
            None => break,
        }
        sleep(Duration::from_millis(400)).await; // be polite to the API
    }
    Ok((out, tz))
}

async fn request_page(
    client: &reqwest::Client,
    game: Game,
    base: &str,
    common: &[(&str, String)],
    gacha_type: &str,
    page: u32,
    end_id: &str,
) -> Result<(Vec<Item>, Option<i32>), AppError> {
    let mut net_attempts = 0u32;
    let mut rate_attempts = 0u32;
    loop {
        let mut q = common.to_vec();
        q.extend(gacha_params(game, gacha_type));
        q.push(("page", page.to_string()));
        q.push(("size", PAGE_SIZE.to_string()));
        q.push(("end_id", end_id.to_string()));

        let resp = match client.get(base).query(&q).send().await {
            Ok(r) if r.status().is_server_error() && net_attempts < 2 => {
                net_attempts += 1;
                sleep(Duration::from_millis(1000 * u64::from(net_attempts))).await;
                continue;
            }
            Ok(r) => r,
            Err(e) if (e.is_timeout() || e.is_connect()) && net_attempts < 2 => {
                net_attempts += 1;
                sleep(Duration::from_millis(1000 * u64::from(net_attempts))).await;
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        let body: Resp = resp.json().await.map_err(|_| {
            AppError::Network("the server sent a response Wishtory could not read".into())
        })?;
        match body.retcode {
            0 => {
                let d = body
                    .data
                    .ok_or_else(|| AppError::Api("empty response".into()))?;
                return Ok((d.list, d.region_time_zone));
            }
            c if looks_like_auth_error(c, &body.message) => return Err(AppError::AuthExpired),
            -110 if rate_attempts < 2 => {
                rate_attempts += 1;
                sleep(Duration::from_millis(2500)).await;
            }
            -110 => return Err(AppError::RateLimited),
            _ => return Err(AppError::Api(body.message)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_fallback_covers_all_games() {
        for (r, tz) in [
            ("os_usa", -5),
            ("os_euro", 1),
            ("os_asia", 8),
            ("os_cht", 8),
            ("prod_official_usa", -5),
            ("prod_official_eur", 1),
            ("prod_official_asia", 8),
            ("prod_gf_us", -5),
            ("prod_gf_eu", 1),
            ("prod_gf_jp", 8),
            ("cn_gf01", 8),
            ("", 8),
        ] {
            assert_eq!(tz_from_region(r), tz, "{r}");
        }
    }

    #[test]
    fn only_hoyo_hosts_are_accepted() {
        assert!(is_hoyo_host("gs.hoyoverse.com"));
        assert!(is_hoyo_host("webstatic.mihoyo.com"));
        assert!(!is_hoyo_host("evilhoyoverse.com"));
        assert!(!is_hoyo_host("hoyoverse.com.evil.example"));
    }

    #[test]
    fn zenless_requests_use_the_nap_history_api() {
        assert_eq!(
            api_base(Game::Nap, false),
            "https://public-operation-nap-sg.hoyoverse.com/common/gacha_record/api/getGachaLog"
        );
        assert_eq!(
            api_base(Game::Nap, true),
            "https://public-operation-nap.mihoyo.com/common/gacha_record/api/getGachaLog"
        );
        assert_eq!(Game::Nap.gacha_types(), &["1", "2", "3", "5"]);
        assert_eq!(
            gacha_params(Game::Nap, "5"),
            vec![
                ("gacha_type", "5001".into()),
                ("real_gacha_type", "5".into()),
                ("init_log_gacha_type", "5001".into()),
                ("init_log_gacha_base_type", "5".into()),
            ]
        );
        assert_eq!(
            gacha_params(Game::Nap, "2")[0],
            ("gacha_type", "2001".into())
        );
        assert_eq!(
            gacha_params(Game::Hk4e, "301"),
            vec![("gacha_type", "301".into())]
        );
        assert_eq!(
            gacha_params(Game::Hkrpg, "21"),
            vec![("gacha_type", "21".into())]
        );
    }

    #[test]
    fn star_rail_queries_every_current_uigf_warp_type() {
        assert_eq!(
            Game::Hkrpg.gacha_types(),
            &["1", "2", "11", "12", "21", "22"]
        );
    }

    #[test]
    fn auth_errors_are_recognised() {
        assert!(looks_like_auth_error(-101, ""));
        assert!(looks_like_auth_error(-1, "authkey timeout"));
        assert!(!looks_like_auth_error(-110, "visit too frequently"));
    }

    #[test]
    fn pagination_rejects_repeated_or_missing_cursors_instead_of_calling_them_complete() {
        assert_eq!(next_cursor(19, Some("19".into()), "0").unwrap(), None);
        assert_eq!(
            next_cursor(20, Some("20".into()), "0").unwrap(),
            Some("20".into())
        );
        assert!(next_cursor(20, Some("20".into()), "20").is_err());
        assert!(next_cursor(20, None, "0").is_err());
    }
}
