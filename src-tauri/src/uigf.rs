//! UIGF v4.x import/export, plus legacy UIGF v2/v3 (hk4e) and SRGF v1.0 (hkrpg).
//! Pure data transformation. Files are read/written by commands.rs at user-chosen paths.

use crate::{
    catalog::Catalog,
    db::Db,
    error::AppError,
    models::{Account, Game, Pull},
};
use serde::Serialize;
use serde_json::{json, Map, Value};

const LANGS: [&str; 15] = [
    "de-de", "en-us", "es-es", "fr-fr", "id-id", "it-it", "ja-jp", "ko-kr", "pt-pt", "ru-ru",
    "th-th", "tr-tr", "vi-vn", "zh-cn", "zh-tw",
];

const APP_NAME: &str = "Wishtory";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const UIGF_VERSION: &str = "v4.2";

#[derive(Serialize)]
pub struct ImportSummary {
    pub format: String,
    pub accounts: usize,
    pub added: usize,
    pub skipped: usize,
}

// ---------- helpers ----------
fn val_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}
fn get(v: &Value, k: &str) -> String {
    v.get(k).map(val_str).unwrap_or_default()
}

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
    if game == "nap" || uid.len() != 9 {
        return 8;
    }
    match uid.as_bytes().first() {
        Some(b'6') => -5,
        Some(b'7') => 1,
        _ => 8,
    }
}

fn read_tz(meta: &Value) -> Option<i32> {
    let v = meta
        .get("timezone")
        .or_else(|| meta.get("region_time_zone"))?;
    match v {
        Value::Number(n) => n.as_i64().map(|n| n as i32),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// UIGF restricts `lang` to a fixed list; map anything else onto it.
fn normalize_lang(l: &str) -> String {
    let l = l.trim().to_ascii_lowercase().replace('_', "-");
    if LANGS.contains(&l.as_str()) {
        return l;
    }
    let prefix = l.split('-').next().unwrap_or("");
    LANGS
        .iter()
        .find(|c| c.split('-').next() == Some(prefix))
        .map_or("en-us", |c| *c)
        .to_string()
}

fn item_to_pull(game: &str, uid: &str, lang: &str, it: &Value) -> Option<Pull> {
    let id = get(it, "id");
    let time = normalize_time(&get(it, "time"));
    let rank_type = get(it, "rank_type").parse::<u8>().ok()?;
    if id.is_empty() || time.is_empty() {
        return None;
    }
    let mut gacha_type = get(it, "gacha_type");
    if gacha_type.is_empty() {
        gacha_type = get(it, "uigf_gacha_type");
    }
    if gacha_type.is_empty() {
        return None; // pity cannot place a pull without a banner
    }
    Some(Pull {
        id,
        game: game.into(),
        uid: uid.into(),
        gacha_type,
        gacha_id: get(it, "gacha_id"),
        item_id: get(it, "item_id"),
        name: get(it, "name"),
        item_type: get(it, "item_type"),
        rank_type,
        time,
        lang: lang.into(),
        count: get(it, "count").parse().unwrap_or(1),
    })
}

// ---------- import ----------
#[cfg(test)]
pub fn import(db: &Db, text: &str) -> Result<ImportSummary, AppError> {
    import_with_catalog(db, text, &Catalog::bundled())
}

pub fn import_with_catalog(
    db: &Db,
    text: &str,
    catalog: &Catalog,
) -> Result<ImportSummary, AppError> {
    let root: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
    if root.get("wish-uid").is_some()
        || root
            .as_object()
            .is_some_and(|o| o.keys().any(|k| k.starts_with("wish-counter-")))
    {
        return import_paimon(db, &root, catalog);
    }
    let info = root.get("info").ok_or_else(|| {
        AppError::Format("missing `info` block. Is this a UIGF / SRGF file?".into())
    })?;
    let mut sum = ImportSummary {
        format: String::new(),
        accounts: 0,
        added: 0,
        skipped: 0,
    };

    if let Some(v) = info
        .get("version")
        .and_then(Value::as_str)
        .filter(|v| v.starts_with("v4"))
    {
        sum.format = format!("UIGF {v}");
        for game in Game::ALL {
            let Some(arr) = root.get(game.as_str()).and_then(Value::as_array) else {
                continue;
            };
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
        return Err(AppError::Format(
            "unrecognised format (expected UIGF v2-v4 or SRGF v1)".into(),
        ));
    }
    if sum.accounts == 0 {
        return Err(AppError::Format("the file contains no wish history".into()));
    }
    Ok(sum)
}

const PAIMON_BANNERS: [(&str, &str); 5] = [
    ("beginners", "100"),
    ("standard", "200"),
    ("character-event", "301"),
    ("weapon-event", "302"),
    ("chronicled", "500"),
];

/// Ids sort by `CAST(id AS INTEGER)` when pulls share a timestamp, so pulls of one banner and
/// time share a hash prefix and `seq` (their position in the backup) decides the order.
fn paimon_stable_id(uid: &str, banner: &str, time: &str, seq: u32) -> String {
    let source = format!("{uid}\0{banner}\0{time}");
    let hash = source.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    });
    (1_000_000_000_000_000_000u64
        + (hash % 7_000_000_000_000_000u64) * 1_000
        + u64::from(seq.min(999)))
    .to_string()
}

/// Reads Paimon.moe's native local-data backup without changing the source file. Its wish
/// records contain Paimon item keys rather than HoYoverse IDs, so the bundled item table supplies
/// the display name and rarity needed by Wishtory's normalized pull model.
fn import_paimon(db: &Db, root: &Value, catalog: &Catalog) -> Result<ImportSummary, AppError> {
    let mut sum = ImportSummary {
        format: "Paimon.moe local data".into(),
        accounts: 0,
        added: 0,
        skipped: 0,
    };
    let mut accounts: Vec<(String, String)> = Vec::new();
    let main_uid = get(root, "wish-uid").trim().to_string();
    if !main_uid.is_empty() {
        accounts.push((String::new(), main_uid));
    }
    if let Some(names) = root.get("accounts").and_then(Value::as_str) {
        for name in names
            .split(',')
            .map(str::trim)
            .filter(|n| !n.is_empty() && *n != "main")
        {
            let uid = get(root, &format!("{name}-wish-uid")).trim().to_string();
            if uid.is_empty() {
                // Without a UID the pulls cannot be filed under a real account.
                sum.skipped += 1;
                continue;
            }
            accounts.push((name.to_string(), uid));
        }
    }
    if accounts.is_empty() {
        return Err(AppError::Format(
            "the Paimon.moe file has no wish account".into(),
        ));
    }

    let lang = normalize_lang(&get(root, "locale"));
    for (account, uid) in accounts {
        let mut pulls = Vec::new();
        let mut found_collection = false;
        for (banner, default_type) in PAIMON_BANNERS {
            let key = if account.is_empty() {
                format!("wish-counter-{banner}")
            } else {
                format!("{account}-wish-counter-{banner}")
            };
            let Some(collection) = root.get(&key) else {
                continue;
            };
            let Some(rows) = collection.get("pulls").and_then(Value::as_array) else {
                sum.skipped += 1;
                continue;
            };
            found_collection = true;
            // Backups are normally oldest first; a newest-first file is read back to front so the
            // sequence numbers below always follow the order the pulls were made in.
            let mut rows: Vec<&Value> = rows.iter().collect();
            if let (Some(first), Some(last)) = (rows.first(), rows.last()) {
                if normalize_time(&get(first, "time")) > normalize_time(&get(last, "time")) {
                    rows.reverse();
                }
            }
            let mut same_time = std::collections::HashMap::<String, u32>::new();
            for row in rows {
                let item_id = get(row, "id");
                let time = normalize_time(&get(row, "time"));
                let declared_kind = get(row, "type").to_ascii_lowercase();
                let item_data = catalog
                    .item_by_paimon_key(&item_id)
                    .map(|item| (item.name.clone(), item.rarity, item.kind.clone()))
                    .or_else(|| {
                        crate::paimon_items::lookup(&item_id)
                            .map(|(n, r, k)| (n.to_string(), r, k.to_string()))
                    });
                let Some((name, rank_type, kind)) = item_data else {
                    sum.skipped += 1;
                    continue;
                };
                if item_id.is_empty() || time.is_empty() || declared_kind != kind {
                    sum.skipped += 1;
                    continue;
                }
                let gacha_type = match get(row, "code").as_str() {
                    "100" | "200" | "301" | "302" | "400" | "500" => get(row, "code"),
                    _ => default_type.into(),
                };
                let seq = same_time
                    .entry(time.clone())
                    .and_modify(|n| *n += 1)
                    .or_insert(0);
                let id = paimon_stable_id(&uid, banner, &time, *seq);
                pulls.push(Pull {
                    id,
                    game: "hk4e".into(),
                    uid: uid.clone(),
                    gacha_type,
                    gacha_id: String::new(),
                    item_id,
                    name,
                    item_type: if kind == "character" {
                        "Character"
                    } else {
                        "Weapon"
                    }
                    .into(),
                    rank_type,
                    time,
                    lang: lang.clone(),
                    count: 1,
                });
            }
        }
        if found_collection {
            sum.added += db.merge_guessed_tz("hk4e", &uid, tz_from_uid("hk4e", &uid), &pulls)?;
            sum.accounts += 1;
        }
    }
    if sum.accounts == 0 {
        return Err(AppError::Format(
            "the Paimon.moe file contains no wish history".into(),
        ));
    }
    Ok(sum)
}

/// `meta` carries uid / lang / timezone (an account entry in v4, the `info` block in legacy files).
fn import_block(
    db: &Db,
    sum: &mut ImportSummary,
    game: &str,
    meta: &Value,
    list: Option<&Value>,
) -> Result<(), AppError> {
    let uid = get(meta, "uid");
    if uid.is_empty() {
        return Ok(());
    }
    let uid = uid.trim().to_string();
    let lang = get(meta, "lang");
    let file_tz = read_tz(meta);
    let tz = file_tz.unwrap_or_else(|| tz_from_uid(game, &uid));
    let mut pulls = Vec::new();
    for it in list.and_then(Value::as_array).into_iter().flatten() {
        match item_to_pull(game, &uid, &lang, it) {
            Some(p) => pulls.push(p),
            None => sum.skipped += 1,
        }
    }
    sum.added += if file_tz.is_some() {
        db.merge(game, &uid, tz, &pulls)?
    } else {
        db.merge_guessed_tz(game, &uid, tz, &pulls)?
    };
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
        let ugt = if p.gacha_type == "400" {
            "301"
        } else {
            p.gacha_type.as_str()
        };
        m.insert("uigf_gacha_type".into(), Value::from(ugt));
    } else {
        m.insert("gacha_id".into(), Value::from(p.gacha_id.clone()));
    }
    o
}

fn export_paimon(db: &Db, account: &Account, catalog: &Catalog) -> Result<String, AppError> {
    let pulls = db.pulls_for("hk4e", &account.uid)?;
    let mut root = Map::new();
    root.insert("wish-uid".into(), Value::from(account.uid.clone()));
    for (banner, types) in [
        ("beginners", &["100"][..]),
        ("standard", &["200"][..]),
        ("character-event", &["301", "400"][..]),
        ("weapon-event", &["302"][..]),
        ("chronicled", &["500"][..]),
    ] {
        let rows: Vec<&Pull> = pulls
            .iter()
            .filter(|p| types.contains(&p.gacha_type.as_str()))
            .collect();
        if rows.is_empty() {
            continue;
        }
        let mut rare_pity = 0u32;
        let mut legendary = 0u32;
        let mut rare = 0u32;
        let mut list = Vec::with_capacity(rows.len());
        for p in rows {
            let catalog_item = catalog
                .items
                .iter()
                .find(|item| item.game == "hk4e" && item.id == p.item_id)
                .and_then(|item| {
                    item.paimon_key
                        .as_ref()
                        .map(|key| (key.clone(), item.rarity, item.kind.as_str()))
                })
                .or_else(|| {
                    catalog
                        .item_by_english_name("hk4e", &p.name)
                        .and_then(|item| {
                            item.paimon_key
                                .as_ref()
                                .map(|key| (key.clone(), item.rarity, item.kind.as_str()))
                        })
                })
                .or_else(|| {
                    crate::paimon_items::lookup(&p.item_id)
                        .map(|(_, rarity, kind)| (p.item_id.clone(), rarity, kind))
                        .or_else(|| {
                            if p.lang.to_ascii_lowercase().starts_with("en") {
                                crate::paimon_items::lookup_by_english_name(&p.name)
                                    .map(|(id, _, rarity, kind)| (id, rarity, kind))
                            } else {
                                None
                            }
                        })
                });
            let Some((paimon_id, _, kind)) = catalog_item else {
                return Err(AppError::Format("Paimon.moe export needs Paimon item keys; these records were imported from a different source.".into()));
            };
            rare_pity += 1;
            let paimon_pity = if p.rank_type == 4 { rare_pity } else { 1 };
            if p.rank_type == 5 {
                legendary += 1;
            }
            if p.rank_type == 4 {
                rare += 1;
            }
            list.push(json!({
                "type": kind, "code": p.gacha_type, "id": paimon_id,
                "time": p.time, "pity": paimon_pity
            }));
            if p.rank_type >= 4 {
                rare_pity = 0;
            }
        }
        root.insert(
            format!("wish-counter-{banner}"),
            json!({
                "total": list.len(), "legendary": legendary, "rare": rare, "pulls": list
            }),
        );
    }
    Ok(serde_json::to_string_pretty(&Value::Object(root))?)
}

fn lang_of(pulls: &[Pull]) -> String {
    normalize_lang(
        pulls
            .iter()
            .find(|p| !p.lang.is_empty())
            .map_or("en-us", |p| p.lang.as_str()),
    )
}

/// `uigf4` exports any selection (all accounts when `game` / `uid` are None).
/// `uigf3` (hk4e) and `srgf1` (hkrpg) need exactly one matching account.
#[cfg(test)]
pub fn export(
    db: &Db,
    format: &str,
    game: Option<Game>,
    uid: Option<&str>,
) -> Result<String, AppError> {
    export_with_catalog(db, format, game, uid, &Catalog::bundled())
}

pub fn export_with_catalog(
    db: &Db,
    format: &str,
    game: Option<Game>,
    uid: Option<&str>,
    catalog: &Catalog,
) -> Result<String, AppError> {
    let accounts: Vec<_> = db
        .list_accounts()?
        .into_iter()
        .filter(|a| game.is_none_or(|g| g.as_str() == a.game) && uid.is_none_or(|u| u == a.uid))
        .collect();
    if accounts.is_empty() {
        return Err(AppError::Format("there is nothing to export yet".into()));
    }
    let ts = chrono::Utc::now().timestamp();

    match format {
        "paimon" => {
            if accounts.len() != 1 || accounts[0].game != "hk4e" {
                return Err(AppError::Format(
                    "Paimon.moe local-data export needs exactly one Genshin account selected"
                        .into(),
                ));
            }
            export_paimon(db, &accounts[0], catalog)
        }
        "uigf4" => {
            let mut root = Map::new();
            root.insert(
                "info".into(),
                json!({
                    "export_timestamp": ts, "export_app": APP_NAME,
                    "export_app_version": APP_VERSION, "version": UIGF_VERSION
                }),
            );
            for a in &accounts {
                let pulls = db.pulls_for(&a.game, &a.uid)?;
                let list: Vec<Value> = pulls.iter().map(|p| item_json(&a.game, p)).collect();
                let uid_val = a
                    .uid
                    .parse::<u64>()
                    .map(Value::from)
                    .unwrap_or_else(|_| Value::from(a.uid.clone()));
                let entry = json!({ "uid": uid_val, "timezone": a.timezone, "lang": lang_of(&pulls), "list": list });
                root.entry(a.game.clone())
                    .or_insert_with(|| Value::Array(vec![]))
                    .as_array_mut()
                    .unwrap()
                    .push(entry);
            }
            Ok(serde_json::to_string_pretty(&Value::Object(root))?)
        }
        "uigf3" | "srgf1" => {
            let want = if format == "uigf3" { "hk4e" } else { "hkrpg" };
            if accounts.len() != 1 || accounts[0].game != want {
                return Err(AppError::Format(format!(
                    "this format needs exactly one {want} account selected"
                )));
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
            Ok(serde_json::to_string_pretty(
                &json!({ "info": info, "list": list }),
            )?)
        }
        other => Err(AppError::Format(format!("unknown export format `{other}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> Db {
        let p =
            std::env::temp_dir().join(format!("wishtory-test-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        Db::open(&p).unwrap()
    }

    fn pull(id: &str, gt: &str, rank: u8) -> Pull {
        Pull {
            id: id.into(),
            game: "hk4e".into(),
            uid: "800000001".into(),
            gacha_type: gt.into(),
            gacha_id: String::new(),
            item_id: "10000003".into(),
            name: "Jean".into(),
            item_type: "Character".into(),
            rank_type: rank,
            time: "2024-01-01 10:00:00".into(),
            lang: "en-us".into(),
            count: 1,
        }
    }

    #[test]
    fn uigf4_roundtrip_is_idempotent() {
        let a = temp_db("a");
        a.merge(
            "hk4e",
            "800000001",
            8,
            &[pull("1", "301", 5), pull("2", "400", 3)],
        )
        .unwrap();
        let json = export(&a, "uigf4", None, None).unwrap();
        let b = temp_db("b");
        let s = import(&b, &json).unwrap();
        assert_eq!((s.accounts, s.added), (1, 2));
        assert_eq!(
            import(&b, &json).unwrap().added,
            0,
            "re-import must not duplicate"
        );
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
    fn imports_paimon_moe_uigf_v22_export_and_exports_compatible_v3() {
        // Shape emitted by Paimon.moe's published UIGF converter: a legacy `info` block
        // (`uigf_version`) and a flat list with string count/rank fields.
        let db = temp_db("paimon-v22");
        let paimon = r#"{"info":{"uid":"800000009","lang":"en-us","export_timestamp":1700000000,
            "export_app":"Paimon Wish Converter","export_app_version":"1.0.0","uigf_version":"v2.2"},
            "list":[{"gacha_type":"400","uigf_gacha_type":"301","count":"1",
            "time":"2024-01-05 03:04:05","name":"Jean","item_type":"Character","rank_type":"5","id":"1665723900010000001"}]}"#;
        let imported = import(&db, paimon).unwrap();
        assert_eq!((imported.accounts, imported.added), (1, 1));
        let pulls = db.pulls_for("hk4e", "800000009").unwrap();
        assert_eq!(
            (pulls[0].gacha_type.as_str(), pulls[0].rank_type),
            ("400", 5)
        );
        assert_eq!(pulls[0].name, "Jean");
        let legacy = export(&db, "uigf3", Some(Game::Hk4e), Some("800000009")).unwrap();
        let value: Value = serde_json::from_str(&legacy).unwrap();
        assert_eq!(value["info"]["uigf_version"], "v3.0");
        assert_eq!(value["list"][0]["uigf_gacha_type"], "301");
    }

    #[test]
    fn imports_native_paimon_backup_and_roundtrips_the_wish_collections() {
        let source = r#"{"wish-uid":"800000009","locale":"en-us",
          "wish-counter-character-event":{"total":5,"legendary":2,"rare":1,"pulls":[
            {"type":"weapon","code":"301","id":"cool_steel","time":"2024-01-05 03:04:05","pity":1},
            {"type":"character","code":"400","id":"albedo","time":"2024-01-05 03:05:05","pity":1},
            {"type":"character","code":"301","id":"amber","time":"2024-01-05 03:06:05","pity":10},
            {"type":"weapon","code":"301","id":"cool_steel","time":"2024-01-05 03:07:05","pity":1},
            {"type":"character","code":"301","id":"albedo","time":"2024-01-05 03:07:05","pity":1}]}}"#;
        let from = temp_db("native-paimon-a");
        let imported = import(&from, source).unwrap();
        assert_eq!(
            (imported.accounts, imported.added, imported.skipped),
            (1, 5, 0)
        );
        let rows = from.pulls_for("hk4e", "800000009").unwrap();
        assert_eq!(
            rows.iter().map(|p| p.rank_type).collect::<Vec<_>>(),
            vec![3, 5, 4, 3, 5],
            "pulls sharing a timestamp keep the order of the backup"
        );
        assert_eq!(rows[1].name, "Albedo");
        assert_eq!(import(&from, source).unwrap().added, 0);

        let exported: Value = serde_json::from_str(
            &export(&from, "paimon", Some(Game::Hk4e), Some("800000009")).unwrap(),
        )
        .unwrap();
        assert_eq!(exported["wish-uid"], "800000009");
        assert_eq!(
            exported["wish-counter-character-event"]["pulls"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(
            exported["wish-counter-character-event"]["pulls"][1]["code"],
            "400"
        );
        assert_eq!(exported["wish-counter-character-event"]["legendary"], 2);

        let to = temp_db("native-paimon-b");
        assert_eq!(import(&to, &exported.to_string()).unwrap().added, 5);
        assert_eq!(to.pulls_for("hk4e", "800000009").unwrap().len(), 5);
    }

    #[test]
    fn native_paimon_import_and_live_refresh_do_not_duplicate_same_pull() {
        let db = temp_db("paimon-live-refresh-dedupe");
        let source = r#"{"wish-uid":"800000009","locale":"en-us",
          "wish-counter-character-event":{"pulls":[
            {"type":"character","code":"301","id":"jean","time":"2024-01-05 03:04:05","pity":78}]}}"#;
        assert_eq!(import(&db, source).unwrap().added, 1);

        let mut refreshed = pull("1665723900010000001", "301", 5);
        refreshed.uid = "800000009".into();
        refreshed.item_id.clear(); // The live HoYoverse record omits this field.
        refreshed.time = "2024-01-05 03:04:05".into();
        assert_eq!(db.merge("hk4e", "800000009", 8, &[refreshed]).unwrap(), 0);
        assert_eq!(db.pulls_for("hk4e", "800000009").unwrap().len(), 1);
    }

    #[test]
    fn paimon_export_maps_english_api_item_names_to_native_keys() {
        let db = temp_db("paimon-api-name-export");
        let mut api_pull = pull("987654321", "301", 5);
        api_pull.item_id = "10000003".into();
        db.merge("hk4e", "800000001", 8, &[api_pull]).unwrap();
        let exported: Value = serde_json::from_str(
            &export(&db, "paimon", Some(Game::Hk4e), Some("800000001")).unwrap(),
        )
        .unwrap();
        let row = &exported["wish-counter-character-event"]["pulls"][0];
        assert_eq!(row["id"], "jean");
        assert_eq!(row["type"], "character");
    }

    #[test]
    fn native_paimon_import_reports_unknown_items_instead_of_inventing_rarity() {
        let db = temp_db("native-paimon-unknown");
        let source = r#"{"wish-uid":"800000010","wish-counter-standard":{"pulls":[
          {"type":"character","code":"200","id":"future_unknown_item","time":"2024-01-05 03:04:05","pity":1}]}}"#;
        let result = import(&db, source).unwrap();
        assert_eq!((result.added, result.skipped), (0, 1));
        assert!(db.pulls_for("hk4e", "800000010").unwrap().is_empty());
    }

    #[test]
    fn live_catalog_maps_new_paimon_keys_for_import_and_export() {
        let catalog = Catalog::parse_and_validate(br#"{"schema_version":1,"catalog_version":2,"generated_at":"2026-10-24T00:00:00Z","items":[{"game":"hk4e","id":"10000077","name":"Future Blade","aliases":[],"rarity":5,"kind":"weapon","traits":{},"paimon_key":"future_blade"}]}"#).unwrap();
        let db = temp_db("paimon-live-catalog");
        let source = r#"{"wish-uid":"800000011","wish-counter-standard":{"pulls":[{"type":"weapon","code":"200","id":"future_blade","time":"2024-01-05 03:04:05","pity":1}]}}"#;
        assert_eq!(import_with_catalog(&db, source, &catalog).unwrap().added, 1);
        let encoded: Value = serde_json::from_str(
            &export_with_catalog(&db, "paimon", Some(Game::Hk4e), Some("800000011"), &catalog)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            encoded["wish-counter-standard"]["pulls"][0]["id"],
            "future_blade"
        );
    }

    #[test]
    fn native_paimon_catalog_covers_missing_weapon_keys_from_real_backup() {
        // These keys were found in the user-provided Paimon.moe backup. Keep a small,
        // identifier-only fixture here so future catalog edits don't silently drop them.
        let keys = [
            ("sharpshooters_oath", 3, "weapon"),
            ("dragons_bane", 4, "weapon"),
            ("lions_roar", 4, "weapon"),
            ("flower-wreathed_feathers", 4, "weapon"),
            ("moonweavers_dawn", 4, "weapon"),
            ("amos_bow", 5, "weapon"),
            ("primordial_jade_winged-spear", 5, "weapon"),
            ("the_dockhands_assistant", 4, "weapon"),
            ("astral_vultures_crimson_plumage", 5, "weapon"),
            ("cranes_echoing_call", 5, "weapon"),
            ("kaguras_verity", 5, "weapon"),
            ("key_of_khaj-nisut", 5, "weapon"),
            ("mouuns_moon", 4, "weapon"),
            ("nocturnes_curtain_call", 5, "weapon"),
            ("sacrificers_staff", 4, "weapon"),
            ("xiphos_moonlight", 4, "weapon"),
        ];
        for (id, rarity, kind) in keys {
            assert_eq!(
                crate::paimon_items::lookup(id).map(|(_, r, k)| (r, k)),
                Some((rarity, kind)),
                "{id}"
            );
        }
    }

    #[test]
    fn uigf4_preserves_zenless_channel_types_and_item_data() {
        let db = temp_db("nap-roundtrip");
        let source = r#"{"info":{"export_timestamp":1700000000,"export_app":"fixture","export_app_version":"1","version":"v4.2"},
          "nap":[{"uid":"1000000012","timezone":8,"lang":"en-us","list":[
            {"gacha_id":"2001","gacha_type":"2","item_id":"1181","count":"1","time":"2024-01-05 03:04:05","name":"Grace","item_type":"Agent","rank_type":"4","id":"1234567890123456789"},
            {"gacha_id":"3001","gacha_type":"5","item_id":"2001","count":"1","time":"2024-01-05 03:05:05","name":"Bangboo","item_type":"Bangboo","rank_type":"3","id":"1234567890123456790"}]}]}"#;
        assert_eq!(import(&db, source).unwrap().added, 2);
        let encoded: Value = serde_json::from_str(
            &export(&db, "uigf4", Some(Game::Nap), Some("1000000012")).unwrap(),
        )
        .unwrap();
        let rows = encoded["nap"][0]["list"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["gacha_type"], "2");
        assert_eq!(rows[1]["gacha_type"], "5");
        assert_eq!(rows[0]["id"], "1234567890123456789");
        assert_eq!(
            (rows[0]["rank_type"].as_str(), rows[1]["rank_type"].as_str()),
            (Some("4"), Some("3"))
        );
    }

    #[test]
    fn time_strings_are_normalised() {
        assert_eq!(normalize_time("2024-1-5 3:04:05"), "2024-01-05 03:04:05");
        assert_eq!(
            normalize_time("2024-01-05T03:04:05Z"),
            "2024-01-05 03:04:05"
        );
        assert_eq!(
            normalize_time("2024-01-05 03:04:05.123"),
            "2024-01-05 03:04:05"
        );
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
        assert_eq!(
            (s.accounts, s.added, s.skipped),
            (1, 1, 1),
            "a row without rank_type cannot be used for pity"
        );
        let rows = db.pulls_for("hkrpg", "700000005").unwrap();
        assert_eq!(rows[0].time, "2024-01-05 03:04:05");
        assert_eq!(db.list_accounts().unwrap()[0].timezone, 1);
    }

    #[test]
    fn uigf4_export_only_contains_games_with_data_and_valid_types() {
        let a = temp_db("shape");
        a.merge(
            "hkrpg",
            "100000001",
            8,
            &[Pull {
                game: "hkrpg".into(),
                uid: "100000001".into(),
                gacha_type: "11".into(),
                lang: "en".into(),
                ..pull("5", "11", 4)
            }],
        )
        .unwrap();
        let v: Value = serde_json::from_str(&export(&a, "uigf4", None, None).unwrap()).unwrap();
        assert!(v.get("hk4e").is_none() && v.get("nap").is_none());
        let acc = &v["hkrpg"][0];
        assert!(acc["uid"].is_u64() && acc["timezone"].is_i64());
        assert_eq!(acc["lang"], "en-us");
        assert!(
            v["info"]["export_timestamp"].is_i64()
                && v["info"]["version"].as_str().unwrap().starts_with("v4.")
        );
        assert!(acc["list"][0]["gacha_id"].is_string() && acc["list"][0]["id"].is_string());
    }
}
