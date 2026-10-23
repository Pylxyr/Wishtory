//! UIGF v4.x import/export, plus legacy UIGF v2/v3 (hk4e) and SRGF v1.0 (hkrpg).
//! Pure data transformation. Files are read/written by commands.rs at user-chosen paths.

use crate::{db::Db, error::AppError, models::{Game, Pull}};
use serde::Serialize;
use serde_json::{json, Map, Value};

const APP_NAME: &str = "Wishtory";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const UIGF_VERSION: &str = "v4.2";

#[derive(Serialize)]
pub struct ImportSummary { pub format: String, pub accounts: usize, pub added: usize, pub skipped: usize }

// ---------- helpers ----------
fn val_str(v: &Value) -> String {
    match v { Value::String(s) => s.clone(), Value::Number(n) => n.to_string(), _ => String::new() }
}
fn get(v: &Value, k: &str) -> String { v.get(k).map(val_str).unwrap_or_default() }

fn item_to_pull(game: &str, uid: &str, lang: &str, it: &Value) -> Option<Pull> {
    let id = get(it, "id");
    let time = get(it, "time");
    let rank_type = get(it, "rank_type").parse::<u8>().ok()?;
    if id.is_empty() || time.is_empty() { return None; }
    let mut gacha_type = get(it, "gacha_type");
    if gacha_type.is_empty() { gacha_type = get(it, "uigf_gacha_type"); }
    Some(Pull {
        id, game: game.into(), uid: uid.into(), gacha_type, gacha_id: get(it, "gacha_id"),
        item_id: get(it, "item_id"), name: get(it, "name"), item_type: get(it, "item_type"),
        rank_type, time, lang: lang.into(), count: get(it, "count").parse().unwrap_or(1),
    })
}

// ---------- import ----------
pub fn import(db: &Db, text: &str) -> Result<ImportSummary, AppError> {
    let root: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
    let info = root.get("info")
        .ok_or_else(|| AppError::Format("missing `info` block. Is this a UIGF / SRGF file?".into()))?;
    let mut sum = ImportSummary { format: String::new(), accounts: 0, added: 0, skipped: 0 };

    if let Some(v) = info.get("version").and_then(Value::as_str).filter(|v| v.starts_with("v4")) {
        sum.format = format!("UIGF {v}");
        for game in Game::ALL {
            let Some(arr) = root.get(game.as_str()).and_then(Value::as_array) else { continue };
            for acc in arr {
                import_block(db, &mut sum, game.as_str(), acc, acc.get("list"))?;
            }
        }
    } else if let Some(v) = info.get("srgf_version").and_then(Value::as_str) {
        sum.format = format!("SRGF {v}");
        import_block(db, &mut sum, "hkrpg", info, root.get("list"))?;
    } else if let Some(v) = info.get("uigf_version").and_then(Value::as_str) {
        sum.format = format!("UIGF {v}");
        import_block(db, &mut sum, "hk4e", info, root.get("list"))?;
    } else {
        return Err(AppError::Format("unrecognised format (expected UIGF v2-v4 or SRGF v1)".into()));
    }
    if sum.accounts == 0 {
        return Err(AppError::Format("the file contains no wish history".into()));
    }
    Ok(sum)
}

/// `meta` carries uid / lang / timezone (an account entry in v4, the `info` block in legacy files).
fn import_block(db: &Db, sum: &mut ImportSummary, game: &str, meta: &Value, list: Option<&Value>) -> Result<(), AppError> {
    let uid = get(meta, "uid");
    if uid.is_empty() { return Ok(()); }
    let lang = get(meta, "lang");
    let tz = meta.get("timezone").or_else(|| meta.get("region_time_zone")).and_then(Value::as_i64).unwrap_or(8) as i32;
    let mut pulls = Vec::new();
    for it in list.and_then(Value::as_array).into_iter().flatten() {
        match item_to_pull(game, &uid, &lang, it) {
            Some(p) => pulls.push(p),
            None => sum.skipped += 1,
        }
    }
    sum.added += db.merge(game, &uid, tz, &pulls)?;
    sum.accounts += 1;
    Ok(())
}

// ---------- export ----------
fn item_json(game: &str, p: &Pull) -> Value {
    let mut o = json!({
        "gacha_type": p.gacha_type, "item_id": p.item_id, "count": p.count.to_string(),
        "time": p.time, "name": p.name, "item_type": p.item_type,
        "rank_type": p.rank_type.to_string(), "id": p.id
    });
    let m = o.as_object_mut().unwrap();
    if game == "hk4e" {
        let ugt = if p.gacha_type == "400" { "301" } else { p.gacha_type.as_str() };
        m.insert("uigf_gacha_type".into(), Value::from(ugt));
    } else {
        m.insert("gacha_id".into(), Value::from(p.gacha_id.clone()));
    }
    o
}

fn lang_of(pulls: &[Pull]) -> String {
    pulls.iter().find(|p| !p.lang.is_empty()).map(|p| p.lang.clone()).unwrap_or_else(|| "en-us".into())
}

/// `uigf4` exports any selection (all accounts when `game` / `uid` are None).
/// `uigf3` (hk4e) and `srgf1` (hkrpg) need exactly one matching account.
pub fn export(db: &Db, format: &str, game: Option<Game>, uid: Option<&str>) -> Result<String, AppError> {
    let accounts: Vec<_> = db.list_accounts()?.into_iter()
        .filter(|a| game.map_or(true, |g| g.as_str() == a.game) && uid.map_or(true, |u| u == a.uid))
        .collect();
    if accounts.is_empty() {
        return Err(AppError::Format("there is nothing to export yet".into()));
    }
    let ts = chrono::Utc::now().timestamp();

    match format {
        "uigf4" => {
            let mut root = Map::new();
            root.insert("info".into(), json!({
                "export_timestamp": ts, "export_app": APP_NAME,
                "export_app_version": APP_VERSION, "version": UIGF_VERSION
            }));
            for a in &accounts {
                let pulls = db.pulls_for(&a.game, &a.uid)?;
                let list: Vec<Value> = pulls.iter().map(|p| item_json(&a.game, p)).collect();
                let uid_val = a.uid.parse::<u64>().map(Value::from).unwrap_or_else(|_| Value::from(a.uid.clone()));
                let entry = json!({ "uid": uid_val, "timezone": a.timezone, "lang": lang_of(&pulls), "list": list });
                root.entry(a.game.clone()).or_insert_with(|| Value::Array(vec![]))
                    .as_array_mut().unwrap().push(entry);
            }
            Ok(serde_json::to_string_pretty(&Value::Object(root))?)
        }
        "uigf3" | "srgf1" => {
            let want = if format == "uigf3" { "hk4e" } else { "hkrpg" };
            if accounts.len() != 1 || accounts[0].game != want {
                return Err(AppError::Format(format!("this format needs exactly one {want} account selected")));
            }
            let a = &accounts[0];
            let pulls = db.pulls_for(&a.game, &a.uid)?;
            let lang = lang_of(&pulls);
            let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let (info, list): (Value, Vec<Value>) = if format == "uigf3" {
                (
                    json!({ "uid": a.uid, "lang": lang, "export_time": now, "export_timestamp": ts,
                            "export_app": APP_NAME, "export_app_version": APP_VERSION,
                            "uigf_version": "v3.0", "region_time_zone": a.timezone }),
                    pulls.iter().map(|p| item_json("hk4e", p)).collect(),
                )
            } else {
                (
                    json!({ "uid": a.uid, "lang": lang, "region_time_zone": a.timezone, "export_timestamp": ts,
                            "export_app": APP_NAME, "export_app_version": APP_VERSION, "srgf_version": "v1.0" }),
                    pulls.iter().map(|p| json!({
                        "gacha_id": p.gacha_id, "gacha_type": p.gacha_type, "item_id": p.item_id,
                        "time": p.time, "name": p.name, "item_type": p.item_type,
                        "rank_type": p.rank_type.to_string(), "id": p.id, "count": p.count.to_string()
                    })).collect(),
                )
            };
            Ok(serde_json::to_string_pretty(&json!({ "info": info, "list": list }))?)
        }
        other => Err(AppError::Format(format!("unknown export format `{other}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> Db {
        let p = std::env::temp_dir().join(format!("wishtory-test-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        Db::open(&p).unwrap()
    }

    fn pull(id: &str, gt: &str, rank: u8) -> Pull {
        Pull {
            id: id.into(), game: "hk4e".into(), uid: "800000001".into(), gacha_type: gt.into(),
            gacha_id: String::new(), item_id: "10000003".into(), name: "Jean".into(),
            item_type: "Character".into(), rank_type: rank, time: "2024-01-01 10:00:00".into(),
            lang: "en-us".into(), count: 1,
        }
    }

    #[test]
    fn uigf4_roundtrip_is_idempotent() {
        let a = temp_db("a");
        a.merge("hk4e", "800000001", 8, &[pull("1", "301", 5), pull("2", "400", 3)]).unwrap();
        let json = export(&a, "uigf4", None, None).unwrap();
        let b = temp_db("b");
        let s = import(&b, &json).unwrap();
        assert_eq!((s.accounts, s.added), (1, 2));
        assert_eq!(import(&b, &json).unwrap().added, 0, "re-import must not duplicate");
        assert!(json.contains("\"uigf_gacha_type\": \"301\""));
    }

    #[test]
    fn imports_legacy_uigf3_and_srgf() {
        let db = temp_db("legacy");
        let v3 = r#"{"info":{"uid":"800000002","lang":"en-us","uigf_version":"v3.0","region_time_zone":-5},
            "list":[{"gacha_type":"301","item_id":"1","count":"1","time":"2024-01-01 00:00:00","name":"X","item_type":"Weapon","rank_type":"4","id":"9","uigf_gacha_type":"301"}]}"#;
        assert_eq!(import(&db, v3).unwrap().added, 1);
        let sr = r#"{"info":{"uid":"100000001","lang":"en-us","region_time_zone":8,"srgf_version":"v1.0"},
            "list":[{"gacha_id":"1","gacha_type":"11","item_id":"1","time":"2024-01-01 00:00:00","name":"Y","item_type":"Character","rank_type":"5","id":"7","count":"1"}]}"#;
        assert_eq!(import(&db, sr).unwrap().added, 1);
        assert!(import(&db, "{\"info\":{}}").is_err());
    }
}
