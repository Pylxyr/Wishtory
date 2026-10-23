//! History download. The pasted/extracted URL is only used to read parameters; requests
//! always go to the official API base chosen below (never to a host from the input),
//! so the authkey cannot be redirected elsewhere.

use crate::{error::AppError, models::{Game, Pull}};
use serde::Deserialize;
use std::{collections::HashMap, time::Duration};
use tokio::time::sleep;
use url::Url;

const PAGE_SIZE: usize = 20;

pub struct Fetched {
    pub uid: String,
    pub timezone: i32,
    pub pulls: Vec<Pull>,
}

#[derive(Deserialize)]
struct Resp { retcode: i64, #[serde(default)] message: String, data: Option<Data> }
#[derive(Deserialize)]
struct Data { #[serde(default)] list: Vec<Item>, region_time_zone: Option<i32> }
#[derive(Deserialize)]
struct Item {
    uid: String,
    gacha_type: String,
    #[serde(default)] gacha_id: String,
    item_id: String,
    #[serde(default)] count: String,
    time: String,
    name: String,
    #[serde(default)] lang: String,
    item_type: String,
    rank_type: String,
    id: String,
}

fn api_base(game: Game, cn: bool) -> &'static str {
    match (game, cn) {
        (Game::Hk4e, false) => "https://public-operation-hk4e-sg.hoyoverse.com/gacha_info/api/getGachaLog",
        (Game::Hk4e, true) => "https://public-operation-hk4e.mihoyo.com/gacha_info/api/getGachaLog",
        (Game::Hkrpg, false) => "https://public-operation-hkrpg-sg.hoyoverse.com/common/gacha_record/api/getGachaLog",
        (Game::Hkrpg, true) => "https://public-operation-hkrpg.mihoyo.com/common/gacha_record/api/getGachaLog",
        (Game::Nap, false) => "https://public-operation-nap-sg.hoyoverse.com/common/gacha_record/api/getGachaLog",
        (Game::Nap, true) => "https://public-operation-nap.mihoyo.com/common/gacha_record/api/getGachaLog",
    }
}

fn is_hoyo_host(h: &str) -> bool {
    ["hoyoverse.com", "mihoyo.com"].iter().any(|d| h == *d || h.ends_with(&format!(".{d}")))
}

fn tz_from_region(region: &str) -> i32 {
    match region { "os_usa" => -5, "os_euro" => 1, _ => 8 }
}

pub async fn fetch_all(game: Game, history_url: &str) -> Result<Fetched, AppError> {
    let parsed = Url::parse(history_url).map_err(|_| AppError::InvalidUrl)?;
    if !parsed.host_str().map(is_hoyo_host).unwrap_or(false) {
        return Err(AppError::InvalidUrl);
    }
    let p: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    let authkey = p.get("authkey").ok_or(AppError::InvalidUrl)?.clone();
    let biz = p.get("game_biz").cloned().unwrap_or_default();
    if !biz.starts_with(game.as_str()) {
        return Err(AppError::InvalidUrl);
    }
    let region = p.get("region").cloned().unwrap_or_default();
    let base = api_base(game, biz.ends_with("_cn"));

    let common: Vec<(&str, String)> = vec![
        ("authkey_ver", p.get("authkey_ver").cloned().unwrap_or_else(|| "1".into())),
        ("sign_type", p.get("sign_type").cloned().unwrap_or_else(|| "2".into())),
        ("lang", p.get("lang").cloned().unwrap_or_else(|| "en".into())),
        ("region", region.clone()),
        ("game_biz", biz),
        ("authkey", authkey),
    ];

    let client = reqwest::Client::builder()
        .user_agent("Wishtory/0.1")
        .timeout(Duration::from_secs(20))
        .build()?;

    // Always a full fetch: ~6 months of data is quick, and merging by id is idempotent.
    let mut pulls = Vec::new();
    let mut tz: Option<i32> = None;
    for gt in game.gacha_types() {
        let mut end_id = "0".to_string();
        for page in 1u32.. {
            let (list, rtz) = request_page(&client, base, &common, gt, page, &end_id).await?;
            tz = tz.or(rtz);
            let n = list.len();
            if let Some(last) = list.last() { end_id = last.id.clone(); }
            for it in list {
                pulls.push(Pull {
                    id: it.id, game: game.as_str().into(), uid: it.uid, gacha_type: it.gacha_type,
                    gacha_id: it.gacha_id, item_id: it.item_id, name: it.name, item_type: it.item_type,
                    rank_type: it.rank_type.parse().unwrap_or(3), time: it.time, lang: it.lang,
                    count: it.count.parse().unwrap_or(1),
                });
            }
            if n < PAGE_SIZE { break; }
            sleep(Duration::from_millis(400)).await; // be polite to the API
        }
        sleep(Duration::from_millis(400)).await;
    }

    let uid = pulls.first().map(|p| p.uid.clone())
        .ok_or_else(|| AppError::Api("No pulls were returned for this account yet.".into()))?;
    Ok(Fetched { uid, timezone: tz.unwrap_or_else(|| tz_from_region(&region)), pulls })
}

async fn request_page(
    client: &reqwest::Client,
    base: &str,
    common: &[(&str, String)],
    gacha_type: &str,
    page: u32,
    end_id: &str,
) -> Result<(Vec<Item>, Option<i32>), AppError> {
    for attempt in 0..3 {
        let mut q = common.to_vec();
        q.push(("gacha_type", gacha_type.to_string()));
        q.push(("page", page.to_string()));
        q.push(("size", PAGE_SIZE.to_string()));
        q.push(("end_id", end_id.to_string()));
        let resp: Resp = client.get(base).query(&q).send().await?.json().await?;
        match resp.retcode {
            0 => {
                let d = resp.data.ok_or_else(|| AppError::Api("empty response".into()))?;
                return Ok((d.list, d.region_time_zone));
            }
            -100 | -101 => return Err(AppError::AuthExpired),
            -110 if attempt < 2 => sleep(Duration::from_millis(2500)).await,
            -110 => return Err(AppError::RateLimited),
            _ => return Err(AppError::Api(resp.message)),
        }
    }
    Err(AppError::RateLimited)
}
