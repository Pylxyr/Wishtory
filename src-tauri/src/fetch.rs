//! History download. The pasted/extracted URL is only used to read parameters; requests
//! always go to the official API base chosen below (never to a host from the input),
//! so the authkey cannot be redirected elsewhere (redirects are disabled as well).

use crate::{
    error::AppError,
    logging::Level,
    models::{Game, Pull},
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
    time::Duration,
};
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

/// The ordinary channel a Zenless rerun channel belongs to: Exclusive Rescreening (102) is the
/// agent channel's selectable rerun, W-Engine Reverberation (103) the W-Engine channel's.
fn rerun_base_channel(game: Game, channel: &str) -> Option<&'static str> {
    match (game, channel) {
        (Game::Nap, "102") => Some("2"),
        (Game::Nap, "103") => Some("3"),
        _ => None,
    }
}

/// The `gacha_type` to store for a record read from `channel`. The history API reports the rerun
/// channels' records with the ordinary channel's type (2 / 3), which would put Rescreening
/// searches into the Exclusive pity counter, so the channel that was asked for is stored instead.
fn stored_gacha_type(game: Game, channel: &str, reported: String) -> String {
    if rerun_base_channel(game, channel).is_some() {
        channel.to_string()
    } else {
        reported
    }
}

/// ZZZ uses an internal channel id plus its normalized channel id. The endpoint
/// returns an empty list when only `real_gacha_type` is supplied.
///
/// The rerun channels (102 / 103) are requested the way the game's own history page asks for
/// them: `real_gacha_type` plus the `init_log_gacha_base_type` of the page the link came from
/// (falling back to the ordinary channel's number). Sending the internal `gacha_type` ids of the
/// ordinary channels with them makes the server ignore the request.
fn gacha_params(
    game: Game,
    channel: &str,
    url_base_type: Option<&str>,
) -> Vec<(&'static str, String)> {
    if let Some(base) = rerun_base_channel(game, channel) {
        return vec![
            ("real_gacha_type", channel.to_string()),
            (
                "init_log_gacha_base_type",
                url_base_type.unwrap_or(base).to_string(),
            ),
        ];
    }
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
    // Servers word this differently ("authkey timeout", "auth key time out"), so ignore spacing.
    let squashed: String = message
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    matches!(code, -100 | -101) || squashed.contains("authkey")
}

/// Banner names for status lines. The part before the first colon in a status line identifies the
/// banner, so names must not contain one.
fn banner_name(game: Game, gacha_type: &str) -> String {
    let name = match (game, gacha_type) {
        (Game::Hk4e, "100") => "Beginners' Wish",
        (Game::Hk4e, "200") => "Standard Wish",
        (Game::Hk4e, "301") => "Character Event Wish",
        (Game::Hk4e, "302") => "Weapon Event Wish",
        (Game::Hk4e, "500") => "Chronicled Wish",
        (Game::Hkrpg, "1") => "Stellar Warp",
        (Game::Hkrpg, "2") => "Departure Warp",
        (Game::Hkrpg, "11") => "Character Event Warp",
        (Game::Hkrpg, "12") => "Light Cone Event Warp",
        (Game::Hkrpg, "21") => "Character Collaboration Warp",
        (Game::Hkrpg, "22") => "Light Cone Collaboration Warp",
        (Game::Nap, "1") => "Standard Channel",
        (Game::Nap, "2") => "Exclusive Channel",
        (Game::Nap, "3") => "W-Engine Channel",
        (Game::Nap, "5") => "Bangboo Channel",
        (Game::Nap, "102") => "Exclusive Rescreening",
        (Game::Nap, "103") => "W-Engine Reverberation",
        _ => return format!("Banner {gacha_type}"),
    };
    name.to_string()
}

/// Everything one history request needs besides the channel and the page.
struct Source<'a> {
    client: &'a reqwest::Client,
    game: Game,
    base: &'a str,
    common: &'a [(&'a str, String)],
    /// `init_log_gacha_base_type` of the link the user's game produced, if it had one.
    url_base_type: Option<&'a str>,
}

/// Ids already stored for an account (given its uid), used to stop at the first page that holds
/// nothing new. Return an empty set to download everything the server still keeps.
pub type KnownIds<'a> = &'a (dyn Fn(&str) -> HashSet<String> + Sync);

/// `progress` receives short, link-free status lines for the UI ("Downloading banner 2 of 5…").
pub async fn fetch_all(
    game: Game,
    history_url: &str,
    progress: &impl Fn(String),
    known: KnownIds<'_>,
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

    // History arrives newest first, so once a whole page is already stored everything older is
    // too and the banner is complete: a refresh then costs one request per banner instead of
    // the whole history. (The caller's `known` set only holds fully described rows and is empty
    // for a full re-sync.) A banner is only kept once it is complete, so pity counters never
    // start in the middle of a half-downloaded banner. Merging by id is idempotent.
    let url_base_type = p
        .get("init_log_gacha_base_type")
        .map(String::as_str)
        .filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()));
    let source = Source {
        client: &client,
        game,
        base,
        common: &common,
        url_base_type,
    };
    let stored = OnceLock::<HashSet<String>>::new();
    let is_known = |uid: &str, id: &str| stored.get_or_init(|| known(uid)).contains(id);
    let mut pulls = Vec::new();
    let mut tz: Option<i32> = None;
    let mut failure: Option<(AppError, usize)> = None;
    // Things worth telling the user that did not stop the refresh.
    let mut notes: Vec<String> = Vec::new();
    let total = game.gacha_types().len();
    for (done, gt) in game.gacha_types().iter().enumerate() {
        let name = banner_name(game, gt);
        progress(format!(
            "{name}: starting (banner {} of {total}, {} pulls so far)",
            done + 1,
            pulls.len()
        ));
        match fetch_banner(&source, gt, &name, progress, &is_known).await {
            Ok((items, rtz)) => {
                progress(format!("{name}: done, {} pulls", items.len()));
                wlog!(
                    Level::Debug,
                    "{}: banner {gt} read ({} pulls)",
                    game.as_str(),
                    items.len()
                );
                tz = tz.or(rtz);
                pulls.extend(items);
            }
            // A rerun channel is optional: if the server cannot answer for it, the channels that
            // were read are still saved and the user is told which one is missing.
            Err(AppError::Api(message)) if rerun_base_channel(game, gt).is_some() => {
                wlog!(
                    Level::Warn,
                    "{}: channel {gt} could not be read: {message}",
                    game.as_str()
                );
                progress(format!("{name}: skipped"));
                notes.push(format!("{name} could not be read ({message})."));
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

    for channel in drop_unseparable_rerun_channels(game, &mut pulls) {
        let name = banner_name(game, channel);
        wlog!(
            Level::Warn,
            "{}: channel {channel} repeated another channel's records and was not saved",
            game.as_str()
        );
        notes.push(format!(
            "{name} could not be told apart from the ordinary channel, so it was not saved."
        ));
    }

    let incomplete = match failure {
        Some((e, _)) if pulls.is_empty() => return Err(e),
        Some((e, done)) => Some(format!(
            "{e} Saved {done} of {} banners; click Refresh again to fetch the rest.",
            game.gacha_types().len()
        )),
        None => None,
    };
    let incomplete = match (incomplete, notes.is_empty()) {
        (None, true) => None,
        (first, _) => Some(first.into_iter().chain(notes).collect::<Vec<_>>().join(" ")),
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
    source: &Source<'_>,
    gacha_type: &str,
    name: &str,
    progress: &impl Fn(String),
    is_known: &(impl Fn(&str, &str) -> bool + Sync),
) -> Result<(Vec<Pull>, Option<i32>), AppError> {
    let game = source.game;
    let say = |message: &str| progress(format!("{name}: {message}"));
    let mut out = Vec::new();
    let mut skipped = 0usize;
    let mut tz = None;
    let mut end_id = "0".to_string();
    for page in 1..=MAX_PAGES {
        let (list, rtz) = request_page(source, gacha_type, (page, &end_id), &say).await?;
        tz = tz.or(rtz);
        let n = list.len();
        let last_id = list.last().map(|i| i.id.clone());
        let nothing_new = page_has_nothing_new(&list, is_known);
        for it in list {
            let Ok(rank_type) = it.rank_type.parse() else {
                skipped += 1; // an unreadable rarity must not be stored as 3-star
                continue;
            };
            if it.id.is_empty() || it.time.is_empty() {
                skipped += 1;
                continue;
            }
            out.push(Pull {
                id: it.id,
                game: game.as_str().into(),
                uid: it.uid,
                gacha_type: stored_gacha_type(game, gacha_type, it.gacha_type),
                gacha_id: it.gacha_id,
                item_id: it.item_id,
                name: it.name,
                item_type: it.item_type,
                rank_type,
                time: it.time,
                lang: it.lang,
                count: it.count.parse().unwrap_or(1),
            });
        }
        say(&format!("{} pulls read", out.len()));
        if nothing_new {
            say("everything older is already saved");
            break;
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
    if skipped > 0 {
        wlog!(
            Level::Warn,
            "{}: {skipped} pull(s) of banner {gacha_type} were skipped (unreadable rarity, id or time)",
            game.as_str()
        );
    }
    Ok((out, tz))
}

/// A rerun channel's records are only kept when none of them also came back from an ordinary
/// channel. A server that ignored the request's `real_gacha_type` would answer with the ordinary
/// channel's history again, and labelling that as Rescreening would silently corrupt both pity
/// counters. Returns the channels that were dropped.
fn drop_unseparable_rerun_channels(game: Game, pulls: &mut Vec<Pull>) -> Vec<&'static str> {
    let ordinary: HashSet<String> = pulls
        .iter()
        .filter(|p| rerun_base_channel(game, &p.gacha_type).is_none())
        .map(|p| p.id.clone())
        .collect();
    let mut dropped = Vec::new();
    for channel in game
        .gacha_types()
        .iter()
        .filter(|c| rerun_base_channel(game, c).is_some())
    {
        let repeats = pulls
            .iter()
            .any(|p| p.gacha_type == *channel && ordinary.contains(&p.id));
        if repeats {
            pulls.retain(|p| p.gacha_type != *channel);
            dropped.push(*channel);
        }
    }
    dropped
}

/// A page whose every record is stored already: history comes newest first, so everything older
/// is stored too. An empty page (or a record without a uid) never counts.
fn page_has_nothing_new(list: &[Item], is_known: &impl Fn(&str, &str) -> bool) -> bool {
    !list.is_empty()
        && list
            .iter()
            .all(|i| !i.uid.is_empty() && is_known(&i.uid, &i.id))
}

async fn request_page(
    source: &Source<'_>,
    gacha_type: &str,
    cursor: (u32, &str),
    say: &impl Fn(&str),
) -> Result<(Vec<Item>, Option<i32>), AppError> {
    let (page, end_id) = cursor;
    let mut net_attempts = 0u32;
    let mut rate_attempts = 0u32;
    loop {
        let mut q = source.common.to_vec();
        q.extend(gacha_params(source.game, gacha_type, source.url_base_type));
        q.push(("page", page.to_string()));
        q.push(("size", PAGE_SIZE.to_string()));
        q.push(("end_id", end_id.to_string()));

        let resp = match source.client.get(source.base).query(&q).send().await {
            Ok(r) if r.status().is_server_error() && net_attempts < 2 => {
                net_attempts += 1;
                say(&format!(
                    "the server had a problem, retrying ({net_attempts} of 2)…"
                ));
                sleep(Duration::from_millis(1000 * u64::from(net_attempts))).await;
                continue;
            }
            Ok(r) if r.status().is_server_error() => {
                return Err(AppError::Api(format!(
                    "the history server answered with HTTP {}",
                    r.status().as_u16()
                )));
            }
            Ok(r) if r.status().as_u16() == 429 && rate_attempts < 2 => {
                rate_attempts += 1;
                say(&format!(
                    "the server asked Wishtory to slow down, waiting to retry ({rate_attempts} of 2)…"
                ));
                sleep(Duration::from_millis(2500)).await;
                continue;
            }
            Ok(r) if r.status().as_u16() == 429 => return Err(AppError::RateLimited),
            Ok(r) => r,
            Err(e) if (e.is_timeout() || e.is_connect()) && net_attempts < 2 => {
                net_attempts += 1;
                say(&format!(
                    "connection problem, retrying ({net_attempts} of 2)…"
                ));
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
                say(&format!(
                    "the server asked Wishtory to slow down, waiting to retry ({rate_attempts} of 2)…"
                ));
                sleep(Duration::from_millis(2500)).await;
            }
            -110 => return Err(AppError::RateLimited),
            c => {
                let detail = if body.message.is_empty() {
                    "no details given"
                } else {
                    body.message.as_str()
                };
                return Err(AppError::Api(format!(
                    "{detail} (code {c}). Wait a minute, then click Refresh again. If it keeps happening, reopen the history page in the game first."
                )));
            }
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
        assert_eq!(Game::Nap.gacha_types(), &["1", "2", "3", "5", "102", "103"]);
        assert_eq!(
            gacha_params(Game::Nap, "5", None),
            vec![
                ("gacha_type", "5001".into()),
                ("real_gacha_type", "5".into()),
                ("init_log_gacha_type", "5001".into()),
                ("init_log_gacha_base_type", "5".into()),
            ]
        );
        assert_eq!(
            gacha_params(Game::Nap, "2", Some("2"))[0],
            ("gacha_type", "2001".into())
        );
        assert_eq!(
            gacha_params(Game::Hk4e, "301", None),
            vec![("gacha_type", "301".into())]
        );
        assert_eq!(
            gacha_params(Game::Hkrpg, "21", None),
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
        assert!(looks_like_auth_error(-1, "auth key time out"));
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

    fn item(uid: &str, id: &str) -> Item {
        serde_json::from_value(serde_json::json!({ "uid": uid, "id": id })).unwrap()
    }

    #[test]
    fn stops_only_at_a_page_that_is_entirely_stored() {
        let stored: HashSet<&str> = ["3", "2", "1"].into();
        let known = |_: &str, id: &str| stored.contains(id);
        assert!(page_has_nothing_new(
            &[item("9", "3"), item("9", "2")],
            &known
        ));
        assert!(!page_has_nothing_new(
            &[item("9", "4"), item("9", "3")],
            &known
        ));
        assert!(
            !page_has_nothing_new(&[], &known),
            "an empty page proves nothing"
        );
        assert!(!page_has_nothing_new(&[item("", "3")], &known));
        // A full re-sync passes an empty set, so nothing is ever "known".
        assert!(!page_has_nothing_new(
            &[item("9", "3")],
            &|_: &str, _: &str| false
        ));
    }

    fn pull(id: &str, gacha_type: &str) -> Pull {
        Pull {
            id: id.into(),
            game: "nap".into(),
            uid: "1".into(),
            gacha_type: gacha_type.into(),
            gacha_id: String::new(),
            item_id: String::new(),
            name: String::new(),
            item_type: String::new(),
            rank_type: 2,
            time: "2026-08-20 12:00:00".into(),
            lang: String::new(),
            count: 1,
        }
    }

    #[test]
    fn rerun_channels_are_requested_like_the_games_own_history_page() {
        // Exclusive Rescreening (102) and W-Engine Reverberation (103): the pool goes in
        // real_gacha_type next to the base type, without the ordinary channels' internal ids.
        assert_eq!(
            gacha_params(Game::Nap, "102", None),
            vec![
                ("real_gacha_type", "102".into()),
                ("init_log_gacha_base_type", "2".into()),
            ]
        );
        assert_eq!(
            gacha_params(Game::Nap, "103", None),
            vec![
                ("real_gacha_type", "103".into()),
                ("init_log_gacha_base_type", "3".into()),
            ]
        );
        // The base type of the user's own link wins, whichever tab it came from.
        assert_eq!(
            gacha_params(Game::Nap, "102", Some("5"))[1],
            ("init_log_gacha_base_type", "5".into())
        );
        // The same numbers mean nothing for the other games.
        assert_eq!(
            gacha_params(Game::Hk4e, "102", Some("5")),
            vec![("gacha_type", "102".into())]
        );
        assert_eq!(rerun_base_channel(Game::Hkrpg, "102"), None);
    }

    #[test]
    fn the_requested_channel_is_stored_not_the_reported_type() {
        // The API reports rerun records as gacha_type 2 / 3; the pity pool is the channel asked for.
        assert_eq!(stored_gacha_type(Game::Nap, "102", "2".into()), "102");
        assert_eq!(stored_gacha_type(Game::Nap, "103", "3".into()), "103");
        assert_eq!(stored_gacha_type(Game::Nap, "2", "2".into()), "2");
        // Genshin's second banner is reported as 400 under channel 301 and must stay 400.
        assert_eq!(stored_gacha_type(Game::Hk4e, "301", "400".into()), "400");
    }

    #[test]
    fn a_rerun_channel_that_repeats_an_ordinary_one_is_not_trusted() {
        // Healthy: the rerun channel has records of its own.
        let mut pulls = vec![
            pull("1", "2"),
            pull("2", "2"),
            pull("3", "102"),
            pull("4", "103"),
        ];
        assert!(drop_unseparable_rerun_channels(Game::Nap, &mut pulls).is_empty());
        assert_eq!(pulls.len(), 4);

        // A server that ignored real_gacha_type answered 102 with the Exclusive history again.
        // Relabelling it would move every Exclusive pull into Rescreening, so it is dropped.
        let mut pulls = vec![
            pull("1", "2"),
            pull("2", "2"),
            pull("1", "102"),
            pull("2", "102"),
            pull("9", "103"),
        ];
        assert_eq!(
            drop_unseparable_rerun_channels(Game::Nap, &mut pulls),
            ["102"]
        );
        let kept: Vec<_> = pulls
            .iter()
            .map(|p| (p.id.as_str(), p.gacha_type.as_str()))
            .collect();
        assert_eq!(kept, [("1", "2"), ("2", "2"), ("9", "103")]);

        // Other games have no rerun channels, whatever their types look like.
        let mut pulls = vec![pull("1", "301"), pull("1", "102")];
        assert!(drop_unseparable_rerun_channels(Game::Hk4e, &mut pulls).is_empty());
        assert_eq!(pulls.len(), 2);
    }

    #[test]
    fn rerun_channels_have_their_own_status_names() {
        assert_eq!(banner_name(Game::Nap, "102"), "Exclusive Rescreening");
        assert_eq!(banner_name(Game::Nap, "103"), "W-Engine Reverberation");
        assert!(!banner_name(Game::Nap, "102").contains(':'));
    }
}
