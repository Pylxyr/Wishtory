//! UIGF v4.x import/export, plus legacy UIGF v2/v3 (hk4e) and SRGF v1.0 (hkrpg).
//! Pure data transformation. Files are read/written by commands.rs at user-chosen paths.

use crate::{db::Db, error::AppError, models::{Game, Pull}};
use serde::Serialize;
use serde_json::{json, Map, Value};

const LANGS: [&str; 15] = [
    "de-de", "en-us", "es-es", "fr-fr", "id-id", "it-it", "ja-jp", "ko-kr", "pt-pt", "ru-ru", "th-th", "tr-tr",
    "vi-vn", "zh-cn", "zh-tw",
];

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

/// Normalises exporter quirks ("2024-1-5 3:04:05", "2024-01-05T03:04:05Z", fractional seconds) to the
/// `YYYY-MM-DD HH:MM:SS` form used everywhere else. No time zone conversion happens: UIGF times are
/// already the account's server-local time. Unparseable text is kept as it was.
fn normalize_time(t: &str) -> String {
    let t = t.trim();
    let cleaned: String = t.replace('T', " ");
    let cleaned = cleaned.trim_end_matches('Z');
    let cleaned = cleaned.split('.').next().unwrap_or(cleaned);
    match chrono::NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%d %H:%M:%S") {
        Ok(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        Err(_) => t.to_string(),
    }
}

/// Server time zone guessed from the UID, used only when a file omits it.
/// Genshin and Star Rail UIDs: 6xxxxxxxx America (UTC-5), 7xxxxxxxx Europe (UTC+1), everything else UTC+8.
fn tz_from_uid(game: &str, uid: &str) -> i32 {
    if game == "nap" || uid.len() != 9 { return 8; }
    match uid.as_bytes().first() { Some(b'6') => -5, Some(b'7') => 1, _ => 8 }
}

fn read_tz(meta: &Value) -> Option<i32> {
    let v = meta.get("timezone").or_else(|| meta.get("region_time_zone"))?;
    match v {
        Value::Number(n) => n.as_i64().map(|n| n as i32),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// UIGF restricts `lang` to a fixed list; map anything else onto it.
fn normalize_lang(l: &str) -> String {
    let l = l.trim().to_ascii_lowercase().replace('_', "-");
    if LANGS.contains(&l.as_str()) { return l; }
    let prefix = l.split('-').next().unwrap_or("");
    LANGS.iter().find(|c| c.split('-').next() == Some(prefix)).map_or("en-us", |c| *c).to_string()
}

fn item_to_pull(game: &str, uid: &str, lang: &str, it: &Value) -> Option<Pull> {
    let id = get(it, "id");
    let time = normalize_time(&get(it, "time"));
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
    let uid = uid.trim().to_string();
    let lang = get(meta, "lang");
    let tz = read_tz(meta).unwrap_or_else(|| tz_from_uid(game, &uid));
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
    normalize_lang(pulls.iter().find(|p| !p.lang.is_empty()).map_or("en-us", |p| p.lang.as_str()))
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

    #[test]
    fn time_strings_are_normalised() {
        assert_eq!(normalize_time("2024-1-5 3:04:05"), "2024-01-05 03:04:05");
        assert_eq!(normalize_time("2024-01-05T03:04:05Z"), "2024-01-05 03:04:05");
        assert_eq!(normalize_time("2024-01-05 03:04:05.123"), "2024-01-05 03:04:05");
        assert_eq!(normalize_time("garbage"), "garbage");
    }

    #[test]
    fn timezone_falls_back_to_the_uid_region() {
        assert_eq!(tz_from_uid("hk4e", "600000001"), -5);
        assert_eq!(tz_from_uid("hkrpg", "700000001"), 1);
        assert_eq!(tz_from_uid("hk4e", "800000001"), 8);
        assert_eq!(tz_from_uid("nap", "1000000001"), 8);
        assert_eq!(read_tz(&json!({"timezone": "-5"})), Some(-5));
        assert_eq!(read_tz(&json!({"region_time_zone": 1})), Some(1));
        assert_eq!(read_tz(&json!({})), None);
    }

    #[test]
    fn languages_are_mapped_into_the_uigf_list() {
        assert_eq!(normalize_lang("EN_US"), "en-us");
        assert_eq!(normalize_lang("en"), "en-us");
        assert_eq!(normalize_lang("ja"), "ja-jp");
        assert_eq!(normalize_lang("klingon"), "en-us");
    }

    #[test]
    fn v4_import_tolerates_optional_fields_and_counts_unusable_rows() {
        let db = temp_db("v4opt");
        let file = r#"{"info":{"export_timestamp":"1700000000","export_app":"x","export_app_version":"1","version":"v4.2"},
          "hkrpg":[{"uid":700000005,"timezone":"1","list":[
            {"gacha_id":"1","gacha_type":"11","item_id":"1003","time":"2024-1-5 3:04:05","id":"100","rank_type":"5"},
            {"gacha_id":"1","gacha_type":"11","item_id":"1004","time":"2024-01-05 03:04:06","id":"101"}]}]}"#;
        let s = import(&db, file).unwrap();
        assert_eq!((s.accounts, s.added, s.skipped), (1, 1, 1), "a row without rank_type cannot be used for pity");
        let rows = db.pulls_for("hkrpg", "700000005").unwrap();
        assert_eq!(rows[0].time, "2024-01-05 03:04:05");
        assert_eq!(db.list_accounts().unwrap()[0].timezone, 1);
    }

    #[test]
    fn uigf4_export_only_contains_games_with_data_and_valid_types() {
        let a = temp_db("shape");
        a.merge("hkrpg", "100000001", 8, &[Pull { game: "hkrpg".into(), uid: "100000001".into(), gacha_type: "11".into(), lang: "en".into(), ..pull("5", "11", 4) }]).unwrap();
        let v: Value = serde_json::from_str(&export(&a, "uigf4", None, None).unwrap()).unwrap();
        assert!(v.get("hk4e").is_none() && v.get("nap").is_none());
        let acc = &v["hkrpg"][0];
        assert!(acc["uid"].is_u64() && acc["timezone"].is_i64());
        assert_eq!(acc["lang"], "en-us");
        assert!(v["info"]["export_timestamp"].is_i64() && v["info"]["version"].as_str().unwrap().starts_with("v4."));
        assert!(acc["list"][0]["gacha_id"].is_string() && acc["list"][0]["id"].is_string());
    }
}
