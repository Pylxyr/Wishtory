//! Local multi-account SQLite storage (our own file in the app-data folder).

use crate::{
    error::AppError,
    models::{is_rerun_gacha_type, Account, Pull},
};
use rusqlite::{params, Connection, Transaction};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PullSignature {
    time: String,
    gacha_type: String,
    item: String,
    rank_type: u8,
}

fn pull_signature(p: &Pull) -> Option<PullSignature> {
    let item = if !p.name.trim().is_empty() {
        p.name
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    } else if !p.item_id.trim().is_empty() {
        p.item_id.trim().to_lowercase()
    } else {
        return None;
    };
    Some(PullSignature {
        time: p.time.trim().to_string(),
        gacha_type: p.gacha_type.trim().to_string(),
        item,
        rank_type: p.rank_type,
    })
}

const SCHEMA: &str = "PRAGMA journal_mode = WAL;
     PRAGMA busy_timeout = 5000;
     CREATE TABLE IF NOT EXISTS accounts (
        game TEXT NOT NULL, uid TEXT NOT NULL, nickname TEXT NOT NULL DEFAULT '',
        timezone INTEGER NOT NULL DEFAULT 8, PRIMARY KEY (game, uid));
     CREATE TABLE IF NOT EXISTS pulls (
        id TEXT NOT NULL, game TEXT NOT NULL, uid TEXT NOT NULL, gacha_type TEXT NOT NULL,
        gacha_id TEXT NOT NULL DEFAULT '', item_id TEXT NOT NULL DEFAULT '', name TEXT NOT NULL DEFAULT '',
        item_type TEXT NOT NULL DEFAULT '', rank_type INTEGER NOT NULL, time TEXT NOT NULL,
        lang TEXT NOT NULL DEFAULT '', count INTEGER NOT NULL DEFAULT 1,
        PRIMARY KEY (game, uid, id));
     CREATE INDEX IF NOT EXISTS idx_pulls_acc ON pulls (game, uid, gacha_type);
     CREATE INDEX IF NOT EXISTS idx_pulls_time ON pulls (game, uid, time);
     CREATE TABLE IF NOT EXISTS pull_banners (
        game TEXT NOT NULL, uid TEXT NOT NULL, pull_id TEXT NOT NULL, banner_id TEXT NOT NULL,
        PRIMARY KEY (game, uid, pull_id));";

type Migration = fn(&Db) -> Result<(), AppError>;

/// Data migrations, in order. Each entry brings the database up to `user_version` = its first
/// number; entries at or below the stored version are skipped. To add one, append a line with
/// the next number. The table definitions above are idempotent and are not versioned.
const MIGRATIONS: &[(u32, Migration)] =
    &[(2, migrate_dedupe_paimon), (5, migrate_reresolve_banners)];

fn migrate_dedupe_paimon(db: &Db) -> Result<(), AppError> {
    db.deduplicate_legacy_paimon_overlaps().map(|_| ())
}

/// Launch-day windows and Zenless Rescreening moved in banner list 4: resolve stored banners again.
fn migrate_reresolve_banners(db: &Db) -> Result<(), AppError> {
    db.clear_pull_banners()
}

/// One account's worth of pulls for an all-or-nothing multi-account merge.
pub struct AccountPulls {
    pub game: String,
    pub uid: String,
    pub tz: i32,
    /// `false` when `tz` is only a guess derived from the UID.
    pub tz_known: bool,
    pub pulls: Vec<Pull>,
}

/// Cheap to clone; every clone shares the same connection.
#[derive(Clone)]
pub struct Db(Arc<Mutex<Connection>>);

impl Db {
    pub fn open(path: &Path) -> Result<Db, AppError> {
        let conn = Connection::open(path)?;
        let user_version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        conn.execute_batch(SCHEMA)?;
        let db = Db(Arc::new(Mutex::new(conn)));
        for (version, migrate) in MIGRATIONS {
            if user_version < *version {
                migrate(&db)?;
                db.conn()
                    .execute_batch(&format!("PRAGMA user_version = {version};"))?;
            }
        }
        Ok(db)
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs blocking database (and file) work off the async runtime's worker threads.
    pub async fn run<T, F>(&self, work: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&Db) -> Result<T, AppError> + Send + 'static,
    {
        let db = self.clone();
        tauri::async_runtime::spawn_blocking(move || work(&db))
            .await
            .map_err(|e| AppError::Io(format!("A background task failed: {e}")))?
    }

    /// Remove duplicate rows written by the old Paimon importer and a later HoYoverse refresh.
    /// The importer marked its IDs with `pmoe-`; keep those richer rows and remove only the
    /// matching number of non-Paimon rows. Same-source repeated items remain intact.
    fn deduplicate_legacy_paimon_overlaps(&self) -> Result<usize, AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let rows = {
            let mut query = tx.prepare(&format!(
                "SELECT {} FROM pulls WHERE game = 'hk4e' ORDER BY time, id",
                Pull::COLUMNS
            ))?;
            let mapped = query.query_map([], Pull::from_row)?;
            mapped.collect::<Result<Vec<_>, _>>()?
        };
        let mut groups = HashMap::<(String, PullSignature), (Vec<String>, Vec<String>)>::new();
        for pull in rows {
            let Some(signature) = pull_signature(&pull) else {
                continue;
            };
            let group = groups.entry((pull.uid.clone(), signature)).or_default();
            if pull.id.starts_with("pmoe-") {
                group.0.push(pull.id);
            } else {
                group.1.push(pull.id);
            }
        }
        let mut removed = 0;
        for ((uid, _), (paimon_ids, other_ids)) in groups {
            for id in other_ids.iter().take(paimon_ids.len()) {
                removed += tx.execute(
                    "DELETE FROM pulls WHERE game='hk4e' AND uid=?1 AND id=?2",
                    params![uid, id],
                )?;
            }
        }
        tx.commit()?;
        Ok(removed)
    }

    /// Upserts the account and merges pulls by unique id. Existing rows are kept, but blank
    /// descriptive fields (name, item type, ...) are filled in from the new copy, so a sparse
    /// import is completed by a later fetch. Returns the number of NEW rows.
    pub fn merge(&self, game: &str, uid: &str, tz: i32, pulls: &[Pull]) -> Result<usize, AppError> {
        self.merge_one(game, uid, tz, true, pulls)
    }

    /// Merges several accounts in ONE transaction: either everything is stored or nothing is.
    /// An account whose `tz_known` is false only gets its (guessed) time zone when it is new;
    /// an existing account keeps the zone it has. Returns the number of NEW rows.
    pub fn merge_many(&self, blocks: &[AccountPulls]) -> Result<usize, AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let mut added = 0;
        for b in blocks {
            added += merge_into(&tx, &b.game, &b.uid, b.tz, b.tz_known, &b.pulls)?;
        }
        tx.commit()?;
        Ok(added)
    }

    fn merge_one(
        &self,
        game: &str,
        uid: &str,
        tz: i32,
        tz_known: bool,
        pulls: &[Pull],
    ) -> Result<usize, AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let added = merge_into(&tx, game, uid, tz, tz_known, pulls)?;
        tx.commit()?;
        Ok(added)
    }

    pub fn list_accounts(&self) -> Result<Vec<Account>, AppError> {
        let conn = self.conn();
        let mut st = conn.prepare(
            "SELECT a.game, a.uid, a.nickname, a.timezone,
                    (SELECT COUNT(*) FROM pulls p WHERE p.game = a.game AND p.uid = a.uid)
             FROM accounts a ORDER BY a.game, a.uid",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Account {
                game: r.get(0)?,
                uid: r.get(1)?,
                nickname: r.get(2)?,
                timezone: r.get(3)?,
                pull_count: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn delete_account(&self, game: &str, uid: &str) -> Result<(), AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM pulls WHERE game = ?1 AND uid = ?2",
            params![game, uid],
        )?;
        tx.execute(
            "DELETE FROM pull_banners WHERE game = ?1 AND uid = ?2",
            params![game, uid],
        )?;
        tx.execute(
            "DELETE FROM accounts WHERE game = ?1 AND uid = ?2",
            params![game, uid],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Permanently remove all saved pulls and their associated accounts.
    pub fn clear_all_history(&self) -> Result<usize, AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let deleted = tx.execute("DELETE FROM pulls", [])?;
        tx.execute("DELETE FROM pull_banners", [])?;
        tx.execute("DELETE FROM accounts", [])?;
        tx.commit()?;
        Ok(deleted)
    }

    /// Ids of the stored pulls of one account that a re-download could not improve: rows with a
    /// name, item type and language. A sparse import (a file with blanks) therefore keeps being
    /// downloaded until a fetch has filled it in.
    pub fn complete_pull_ids_for(
        &self,
        game: &str,
        uid: &str,
    ) -> Result<HashSet<String>, AppError> {
        let conn = self.conn();
        let mut st = conn.prepare(
            "SELECT id FROM pulls WHERE game = ?1 AND uid = ?2
               AND name <> '' AND item_type <> '' AND lang <> ''",
        )?;
        let rows = st.query_map(params![game, uid], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Pulls of one account that have no stored banner yet, oldest → newest.
    pub fn pulls_without_banner(&self, game: &str, uid: &str) -> Result<Vec<Pull>, AppError> {
        let conn = self.conn();
        let mut st = conn.prepare(&format!(
            "SELECT {} FROM pulls p
             WHERE p.game = ?1 AND p.uid = ?2
               AND NOT EXISTS (SELECT 1 FROM pull_banners b
                               WHERE b.game = p.game AND b.uid = p.uid AND b.pull_id = p.id)
             ORDER BY p.time ASC, CAST(p.id AS INTEGER) ASC",
            Pull::COLUMNS
        ))?;
        let rows = st.query_map(params![game, uid], Pull::from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Stores `(pull id, banner id)` pairs for one account, replacing any earlier value.
    pub fn set_pull_banners(
        &self,
        game: &str,
        uid: &str,
        rows: &[(String, String)],
    ) -> Result<(), AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        {
            let mut insert = tx.prepare(
                "INSERT OR REPLACE INTO pull_banners (game, uid, pull_id, banner_id)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (pull_id, banner_id) in rows {
                insert.execute(params![game, uid, pull_id, banner_id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Pull id → banner id for one account.
    pub fn pull_banners_for(
        &self,
        game: &str,
        uid: &str,
    ) -> Result<HashMap<String, String>, AppError> {
        let conn = self.conn();
        let mut st = conn
            .prepare("SELECT pull_id, banner_id FROM pull_banners WHERE game = ?1 AND uid = ?2")?;
        let rows = st.query_map(params![game, uid], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Forget every stored banner so they are resolved again against a newer banner list.
    pub fn clear_pull_banners(&self) -> Result<(), AppError> {
        self.conn().execute("DELETE FROM pull_banners", [])?;
        Ok(())
    }

    /// Oldest → newest.
    pub fn pulls_for(&self, game: &str, uid: &str) -> Result<Vec<Pull>, AppError> {
        let conn = self.conn();
        let mut st = conn.prepare(&format!(
            "SELECT {} FROM pulls WHERE game = ?1 AND uid = ?2
             ORDER BY time ASC, CAST(id AS INTEGER) ASC",
            Pull::COLUMNS
        ))?;
        let rows = st.query_map(params![game, uid], Pull::from_row)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

/// Stored rows that an incoming pull without a known id may be the same wish as, grouped by
/// content and handed out once each.
struct CrossSourceCandidates(HashMap<PullSignature, VecDeque<String>>);

impl CrossSourceCandidates {
    /// Paimon.moe and HoYoverse assign different record IDs to the same wish, so a pull whose
    /// id is unknown may still be a stored row under another id. Only stored rows whose id is
    /// NOT part of the incoming batch qualify: a row the batch names by id is already matched
    /// by that id, and pairing a genuinely new pull with it would silently drop the new pull.
    fn load(
        tx: &Transaction,
        game: &str,
        uid: &str,
        fresh: &[&Pull],
        incoming_ids: &HashSet<&str>,
    ) -> Result<Self, AppError> {
        let times = fresh
            .iter()
            .filter(|p| pull_signature(p).is_some())
            .map(|p| p.time.trim());
        let (Some(lo), Some(hi)) = (times.clone().min(), times.max()) else {
            return Ok(Self(HashMap::new()));
        };
        let mut by_signature = HashMap::<PullSignature, VecDeque<String>>::new();
        let mut query = tx.prepare(&format!(
            "SELECT {} FROM pulls WHERE game = ?1 AND uid = ?2 AND time >= ?3 AND time <= ?4
             ORDER BY time, CAST(id AS INTEGER), id",
            Pull::COLUMNS
        ))?;
        let rows = query.query_map(params![game, uid, lo, hi], Pull::from_row)?;
        for row in rows {
            let row = row?;
            if incoming_ids.contains(row.id.as_str()) {
                continue;
            }
            if let Some(signature) = pull_signature(&row) {
                by_signature.entry(signature).or_default().push_back(row.id);
            }
        }
        Ok(Self(by_signature))
    }

    fn take(&mut self, signature: &PullSignature) -> Option<String> {
        self.0.get_mut(signature)?.pop_front()
    }
}

fn merge_into(
    tx: &Transaction,
    game: &str,
    uid: &str,
    tz: i32,
    tz_known: bool,
    pulls: &[Pull],
) -> Result<usize, AppError> {
    tx.execute(
        if tz_known {
            "INSERT INTO accounts (game, uid, timezone) VALUES (?1, ?2, ?3)
             ON CONFLICT(game, uid) DO UPDATE SET timezone = excluded.timezone"
        } else {
            "INSERT INTO accounts (game, uid, timezone) VALUES (?1, ?2, ?3)
             ON CONFLICT(game, uid) DO NOTHING"
        },
        params![game, uid, tz],
    )?;
    let mut existing_ids = HashSet::<String>::new();
    {
        let mut query = tx.prepare("SELECT id FROM pulls WHERE game = ?1 AND uid = ?2")?;
        for id in query.query_map(params![game, uid], |r| r.get::<_, String>(0))? {
            existing_ids.insert(id?);
        }
    }
    let incoming_ids: HashSet<&str> = pulls.iter().map(|p| p.id.as_str()).collect();
    let fresh: Vec<&Pull> = pulls
        .iter()
        .filter(|p| !existing_ids.contains(&p.id))
        .collect();
    let mut candidates = CrossSourceCandidates::load(tx, game, uid, &fresh, &incoming_ids)?;

    let mut added = 0;
    let mut ins = tx.prepare(
        "INSERT OR IGNORE INTO pulls (id, game, uid, gacha_type, gacha_id, item_id, name, item_type, rank_type, time, lang, count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    let mut fill = tx.prepare(
        "UPDATE pulls SET
            gacha_id  = CASE WHEN gacha_id  = '' THEN ?4 ELSE gacha_id  END,
            item_id   = CASE WHEN item_id   = '' THEN ?5 ELSE item_id   END,
            name      = CASE WHEN name      = '' THEN ?6 ELSE name      END,
            item_type = CASE WHEN item_type = '' THEN ?7 ELSE item_type END,
            lang      = CASE WHEN lang      = '' THEN ?8 ELSE lang      END
         WHERE game = ?1 AND uid = ?2 AND id = ?3
           AND (gacha_id = '' OR item_id = '' OR name = '' OR item_type = '' OR lang = '')",
    )?;
    let mut fill_row = |id: &str, p: &Pull| {
        fill.execute(params![
            game,
            uid,
            id,
            p.gacha_id,
            p.item_id,
            p.name,
            p.item_type,
            p.lang
        ])
    };
    // A record read from a rerun channel is that channel's, even when it is already stored as the
    // ordinary channel's (an import from another tool, which only knows 2 / 3): the live history
    // is the authority, and the banner it was matched to is worked out again.
    let mut reclassify = tx.prepare(
        "UPDATE pulls SET gacha_type = ?4 WHERE game = ?1 AND uid = ?2 AND id = ?3 AND gacha_type <> ?4",
    )?;
    let mut forget_banner =
        tx.prepare("DELETE FROM pull_banners WHERE game = ?1 AND uid = ?2 AND pull_id = ?3")?;
    for p in pulls {
        if existing_ids.contains(&p.id) {
            fill_row(&p.id, p)?;
            if is_rerun_gacha_type(game, &p.gacha_type)
                && reclassify.execute(params![game, uid, p.id, p.gacha_type])? > 0
            {
                forget_banner.execute(params![game, uid, p.id])?;
            }
            continue;
        }
        if let Some(stored_id) = pull_signature(p).and_then(|s| candidates.take(&s)) {
            fill_row(&stored_id, p)?;
            continue;
        }
        let n = ins.execute(params![
            p.id,
            game,
            uid,
            p.gacha_type,
            p.gacha_id,
            p.item_id,
            p.name,
            p.item_type,
            p.rank_type,
            p.time,
            p.lang,
            p.count
        ])?;
        if n == 0 {
            fill_row(&p.id, p)?;
        }
        existing_ids.insert(p.id.clone());
        added += n;
    }
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pull(id: &str, name: &str) -> Pull {
        Pull {
            id: id.into(),
            game: "hk4e".into(),
            uid: "1".into(),
            gacha_type: "301".into(),
            gacha_id: String::new(),
            item_id: String::new(),
            name: name.into(),
            item_type: String::new(),
            rank_type: 4,
            time: "2024-01-01 00:00:00".into(),
            lang: String::new(),
            count: 1,
        }
    }

    #[test]
    fn merge_is_idempotent_and_fills_blank_fields() {
        let path = std::env::temp_dir().join(format!("wishtory-db-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        assert_eq!(
            db.merge("hk4e", "1", 8, &[pull("10", ""), pull("11", "Keep")])
                .unwrap(),
            2
        );
        // Same ids again: nothing new, but the blank name of id 10 gets filled and "Keep" stays.
        assert_eq!(
            db.merge("hk4e", "1", 8, &[pull("10", "Filled"), pull("11", "Other")])
                .unwrap(),
            0
        );
        let rows = db.pulls_for("hk4e", "1").unwrap();
        assert_eq!(rows.iter().find(|p| p.id == "10").unwrap().name, "Filled");
        assert_eq!(rows.iter().find(|p| p.id == "11").unwrap().name, "Keep");
        // Accounts are kept apart.
        assert_eq!(db.merge("hk4e", "2", 8, &[pull("10", "X")]).unwrap(), 1);
        assert_eq!(db.list_accounts().unwrap().len(), 2);
        // A guessed time zone never replaces a stored one; a known one does.
        let tz_of = |db: &Db| {
            db.list_accounts()
                .unwrap()
                .iter()
                .find(|a| a.uid == "2")
                .unwrap()
                .timezone
        };
        db.merge_many(&[AccountPulls {
            game: "hk4e".into(),
            uid: "2".into(),
            tz: -5,
            tz_known: false,
            pulls: vec![],
        }])
        .unwrap();
        assert_eq!(tz_of(&db), 8);
        db.merge("hk4e", "2", 1, &[]).unwrap();
        assert_eq!(tz_of(&db), 1);
        db.delete_account("hk4e", "1").unwrap();
        assert!(db.pulls_for("hk4e", "1").unwrap().is_empty());
        assert_eq!(db.pulls_for("hk4e", "2").unwrap().len(), 1);
    }

    #[test]
    fn open_removes_only_cross_source_paimon_overlaps_and_preserves_multiplicity() {
        let path =
            std::env::temp_dir().join(format!("wishtory-dedupe-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        let mut first = pull("pmoe-1-1", "Jean");
        let mut second = pull("pmoe-1-2", "Jean");
        first.rank_type = 5;
        second.rank_type = 5;
        db.merge("hk4e", "1", 8, &[first, second]).unwrap();
        {
            let conn = db.conn();
            conn.execute(
                "INSERT INTO pulls (id,game,uid,gacha_type,item_id,name,item_type,rank_type,time,lang,count)
                 VALUES ('123456789','hk4e','1','301','','Jean','Character',5,'2024-01-01 00:00:00','en-us',1)",
                [],
            ).unwrap();
            conn.execute_batch("PRAGMA user_version = 1;").unwrap();
        }
        drop(db);

        let reopened = Db::open(&path).unwrap();
        let rows = reopened.pulls_for("hk4e", "1").unwrap();
        assert_eq!(
            rows.len(),
            2,
            "keep both source pulls, remove only the overlapping refresh copy"
        );
        assert!(rows.iter().all(|row| row.id.starts_with("pmoe-")));
    }

    #[test]
    fn clear_all_history_removes_pulls_and_accounts_across_games() {
        let path =
            std::env::temp_dir().join(format!("wishtory-clear-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        for (game, uid) in [("hk4e", "1"), ("hkrpg", "2"), ("nap", "3")] {
            let mut row = pull(uid, "Jean");
            row.game = game.into();
            row.uid = uid.into();
            db.merge(game, uid, 8, &[row]).unwrap();
        }

        assert_eq!(db.clear_all_history().unwrap(), 3);
        assert!(db.list_accounts().unwrap().is_empty());
        assert!(db.pulls_for("hk4e", "1").unwrap().is_empty());
        assert!(db.pulls_for("hkrpg", "2").unwrap().is_empty());
        assert!(db.pulls_for("nap", "3").unwrap().is_empty());
    }

    #[test]
    fn pull_banners_are_stored_per_account_and_removed_with_it() {
        let path =
            std::env::temp_dir().join(format!("wishtory-db-banners-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        db.merge("hk4e", "1", 8, &[pull("a", "x"), pull("b", "y")])
            .unwrap();
        assert_eq!(db.pulls_without_banner("hk4e", "1").unwrap().len(), 2);
        db.set_pull_banners("hk4e", "1", &[("a".into(), "banner-1".into())])
            .unwrap();
        let missing = db.pulls_without_banner("hk4e", "1").unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id, "b");
        let stored = db.pull_banners_for("hk4e", "1").unwrap();
        assert_eq!(stored.get("a").map(String::as_str), Some("banner-1"));
        // A later value replaces the earlier one.
        db.set_pull_banners("hk4e", "1", &[("a".into(), "banner-2".into())])
            .unwrap();
        assert_eq!(
            db.pull_banners_for("hk4e", "1")
                .unwrap()
                .get("a")
                .map(String::as_str),
            Some("banner-2")
        );
        db.clear_pull_banners().unwrap();
        assert_eq!(db.pulls_without_banner("hk4e", "1").unwrap().len(), 2);
        db.set_pull_banners("hk4e", "1", &[("a".into(), "banner-1".into())])
            .unwrap();
        db.delete_account("hk4e", "1").unwrap();
        assert!(db.pull_banners_for("hk4e", "1").unwrap().is_empty());
        drop(db);
        let _ = std::fs::remove_file(&path);
    }

    fn stamped(id: &str, name: &str, time: &str) -> Pull {
        let mut p = pull(id, name);
        p.time = time.into();
        p
    }

    fn temp_db(tag: &str) -> (Db, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("wishtory-{tag}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        (Db::open(&path).unwrap(), path)
    }

    #[test]
    fn new_pull_with_a_stored_pulls_content_is_not_dropped() {
        // Regression: the incoming batch lists newest first, the stored rows oldest first. A
        // genuinely new pull that shares time/item/rank with a stored one used to be paired
        // with that stored row and silently discarded.
        let (db, path) = temp_db("same-signature");
        assert_eq!(db.merge("hk4e", "1", 8, &[pull("5", "Twin")]).unwrap(), 1);
        assert_eq!(
            db.merge("hk4e", "1", 8, &[pull("6", "Twin"), pull("5", "Twin")])
                .unwrap(),
            1
        );
        let ids: Vec<_> = db
            .pulls_for("hk4e", "1")
            .unwrap()
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, ["5", "6"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cross_source_copies_pair_once_per_stored_row() {
        // Stored under Paimon.moe-style ids; the same wishes now arrive with HoYoverse ids.
        let (db, path) = temp_db("cross-source");
        let t = "2024-03-01 10:00:00";
        db.merge(
            "hk4e",
            "1",
            8,
            &[
                stamped("1000000000000000001", "Twin", t),
                stamped("1000000000000000002", "Twin", t),
            ],
        )
        .unwrap();
        let real = |a: &str, b: &str| [stamped(a, "Twin", t), stamped(b, "Twin", t)];
        assert_eq!(
            db.merge(
                "hk4e",
                "1",
                8,
                &real("1700000000000000002", "1700000000000000001")
            )
            .unwrap(),
            0,
            "both copies are already stored under the other source's ids"
        );
        // A third identical wish is new, and repeating the merge changes nothing.
        let three = [
            stamped("1700000000000000003", "Twin", t),
            stamped("1700000000000000002", "Twin", t),
            stamped("1700000000000000001", "Twin", t),
        ];
        assert_eq!(db.merge("hk4e", "1", 8, &three).unwrap(), 1);
        assert_eq!(db.merge("hk4e", "1", 8, &three).unwrap(), 0);
        assert_eq!(db.pulls_for("hk4e", "1").unwrap().len(), 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn merge_many_is_all_or_nothing() {
        let (db, path) = temp_db("merge-many");
        let block = |game: &str, uid: &str, id: &str| AccountPulls {
            game: game.into(),
            uid: uid.into(),
            tz: 8,
            tz_known: true,
            pulls: vec![{
                let mut p = pull(id, "x");
                p.game = game.into();
                p.uid = uid.into();
                p
            }],
        };
        assert_eq!(
            db.merge_many(&[block("hk4e", "1", "a"), block("hkrpg", "2", "b")])
                .unwrap(),
            2
        );
        assert_eq!(db.list_accounts().unwrap().len(), 2);
        // A failure in a later block must leave earlier blocks of the batch unsaved.
        db.conn()
            .execute_batch("CREATE TRIGGER fail_c BEFORE INSERT ON pulls WHEN NEW.id = 'c' BEGIN SELECT RAISE(ABORT, 'nope'); END;")
            .unwrap();
        let err = db.merge_many(&[block("nap", "3", "ok"), block("nap", "4", "c")]);
        assert!(err.is_err());
        assert_eq!(
            db.list_accounts().unwrap().len(),
            2,
            "nothing from the failed batch is kept"
        );
        assert!(db.pulls_for("nap", "3").unwrap().is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn migrations_run_once_and_stamp_the_version() {
        let (db, path) = temp_db("migrate");
        let version = |db: &Db| -> u32 {
            db.conn()
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(version(&db), MIGRATIONS.last().unwrap().0);
        db.merge("hk4e", "1", 8, &[pull("a", "x")]).unwrap();
        db.set_pull_banners("hk4e", "1", &[("a".into(), "b1".into())])
            .unwrap();
        drop(db);
        let reopened = Db::open(&path).unwrap();
        assert_eq!(
            reopened.pull_banners_for("hk4e", "1").unwrap().len(),
            1,
            "an up-to-date database must not re-run migrations"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn only_fully_described_pulls_count_as_complete() {
        let (db, path) = temp_db("complete-ids");
        let described = |id: &str, name: &str| {
            let mut p = pull(id, name);
            p.item_type = "Character".into();
            p.lang = "en".into();
            p
        };
        let mut no_name = described("no-name", "");
        no_name.name = String::new();
        let mut no_type = described("no-type", "Jean");
        no_type.item_type = String::new();
        let mut no_lang = described("no-lang", "Jean");
        no_lang.lang = String::new();
        db.merge(
            "hk4e",
            "1",
            8,
            &[described("whole", "Jean"), no_name, no_type, no_lang],
        )
        .unwrap();
        let ids = db.complete_pull_ids_for("hk4e", "1").unwrap();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains("whole"));
        assert!(db
            .complete_pull_ids_for("hk4e", "other")
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_rerun_channel_read_reclassifies_a_pull_stored_as_the_ordinary_channel() {
        let (db, path) = temp_db("rerun-reclassify");
        let stored = |id: &str, gacha_type: &str| {
            let mut p = pull(id, "Alice");
            p.game = "nap".into();
            p.gacha_type = gacha_type.into();
            p
        };
        // Imported from a tool that only knows the types 2 / 3, so it is stored as Exclusive.
        db.merge("nap", "1", 8, &[stored("a", "2"), stored("b", "2")])
            .unwrap();
        db.set_pull_banners(
            "nap",
            "1",
            &[
                ("a".into(), "banner-x".into()),
                ("b".into(), "banner-y".into()),
            ],
        )
        .unwrap();
        // The live fetch reads "a" from the Rescreening channel (and "b" again as Exclusive).
        assert_eq!(
            db.merge("nap", "1", 8, &[stored("a", "102"), stored("b", "2")])
                .unwrap(),
            0
        );
        let types: Vec<_> = db
            .pulls_for("nap", "1")
            .unwrap()
            .into_iter()
            .map(|p| (p.id, p.gacha_type))
            .collect();
        assert_eq!(
            types,
            [("a".into(), "102".into()), ("b".into(), "2".into())]
        );
        let banners = db.pull_banners_for("nap", "1").unwrap();
        assert!(!banners.contains_key("a"), "its banner is matched again");
        assert_eq!(banners.get("b").map(String::as_str), Some("banner-y"));
        // Only the rerun channels override: Genshin's 400 stays 400 when 301 is read again.
        let mut genshin = pull("g", "x");
        genshin.gacha_type = "400".into();
        db.merge("hk4e", "1", 8, &[genshin.clone()]).unwrap();
        genshin.gacha_type = "301".into();
        db.merge("hk4e", "1", 8, &[genshin]).unwrap();
        assert_eq!(db.pulls_for("hk4e", "1").unwrap()[0].gacha_type, "400");
        let _ = std::fs::remove_file(&path);
    }
}
