//! Local multi-account SQLite storage (our own file in the app-data folder).

use crate::{
    error::AppError,
    models::{Account, Pull},
};
use rusqlite::{params, Connection};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Mutex, MutexGuard},
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

pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> Result<Db, AppError> {
        let conn = Connection::open(path)?;
        let user_version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
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
             CREATE TABLE IF NOT EXISTS pull_banners (
                game TEXT NOT NULL, uid TEXT NOT NULL, pull_id TEXT NOT NULL, banner_id TEXT NOT NULL,
                PRIMARY KEY (game, uid, pull_id));",
        )?;
        let db = Db(Mutex::new(conn));
        if user_version < 2 {
            db.deduplicate_legacy_paimon_overlaps()?;
            db.conn().execute_batch("PRAGMA user_version = 2;")?;
        }
        Ok(db)
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Remove duplicate rows written by the old Paimon importer and a later HoYoverse refresh.
    /// The importer marked its IDs with `pmoe-`; keep those richer rows and remove only the
    /// matching number of non-Paimon rows. Same-source repeated items remain intact.
    fn deduplicate_legacy_paimon_overlaps(&self) -> Result<usize, AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        let rows = {
            let mut query = tx.prepare(
                "SELECT id, game, uid, gacha_type, gacha_id, item_id, name, item_type, rank_type, time, lang, count
                 FROM pulls WHERE game = 'hk4e' ORDER BY time, id",
            )?;
            let mapped = query.query_map([], |r| {
                Ok(Pull {
                    id: r.get(0)?,
                    game: r.get(1)?,
                    uid: r.get(2)?,
                    gacha_type: r.get(3)?,
                    gacha_id: r.get(4)?,
                    item_id: r.get(5)?,
                    name: r.get(6)?,
                    item_type: r.get(7)?,
                    rank_type: r.get(8)?,
                    time: r.get(9)?,
                    lang: r.get(10)?,
                    count: r.get(11)?,
                })
            })?;
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
        self.merge_with(game, uid, tz, true, pulls)
    }

    /// Like `merge`, but `tz` is only a guess (derived from the UID): it is stored for a new
    /// account and never replaces the time zone an existing account already has.
    pub fn merge_guessed_tz(
        &self,
        game: &str,
        uid: &str,
        tz: i32,
        pulls: &[Pull],
    ) -> Result<usize, AppError> {
        self.merge_with(game, uid, tz, false, pulls)
    }

    fn merge_with(
        &self,
        game: &str,
        uid: &str,
        tz: i32,
        tz_known: bool,
        pulls: &[Pull],
    ) -> Result<usize, AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
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
        let mut added = 0;
        let existing = {
            let mut query = tx.prepare(
                "SELECT id, game, uid, gacha_type, gacha_id, item_id, name, item_type, rank_type, time, lang, count
                 FROM pulls WHERE game = ?1 AND uid = ?2 ORDER BY time, id",
            )?;
            let rows = query.query_map(params![game, uid], |r| {
                Ok(Pull {
                    id: r.get(0)?,
                    game: r.get(1)?,
                    uid: r.get(2)?,
                    gacha_type: r.get(3)?,
                    gacha_id: r.get(4)?,
                    item_id: r.get(5)?,
                    name: r.get(6)?,
                    item_type: r.get(7)?,
                    rank_type: r.get(8)?,
                    time: r.get(9)?,
                    lang: r.get(10)?,
                    count: r.get(11)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut existing_by_signature = HashMap::<PullSignature, Vec<Pull>>::new();
        let mut existing_ids = HashSet::new();
        for pull in existing {
            existing_ids.insert(pull.id.clone());
            if let Some(signature) = pull_signature(&pull) {
                existing_by_signature
                    .entry(signature)
                    .or_default()
                    .push(pull);
            }
        }
        let mut incoming_occurrences = HashMap::<PullSignature, usize>::new();
        {
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
            for p in pulls {
                let signature = pull_signature(p);
                let occurrence = signature.as_ref().map(|key| {
                    let next = incoming_occurrences.entry(key.clone()).or_default();
                    let current = *next;
                    *next += 1;
                    current
                });
                // Paimon.moe and HoYoverse assign different record IDs to the same wish.
                // Pair matching rows by their occurrence within an exact content signature;
                // this prevents cross-source duplicates without collapsing legitimate repeated
                // items from the same timestamp (for example, multiple matching 3★ pulls).
                let content_match = signature.as_ref().and_then(|key| {
                    existing_by_signature
                        .get(key)
                        .and_then(|rows| rows.get(occurrence.unwrap_or(0)))
                });
                let existing_id = if existing_ids.contains(&p.id) {
                    Some(p.id.as_str())
                } else {
                    content_match.map(|old| old.id.as_str())
                };
                if let Some(id) = existing_id {
                    fill.execute(params![
                        game,
                        uid,
                        id,
                        p.gacha_id,
                        p.item_id,
                        p.name,
                        p.item_type,
                        p.lang
                    ])?;
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
                    fill.execute(params![
                        game,
                        uid,
                        p.id,
                        p.gacha_id,
                        p.item_id,
                        p.name,
                        p.item_type,
                        p.lang
                    ])?;
                }
                existing_ids.insert(p.id.clone());
                added += n;
            }
        }
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

    /// Pulls of one account that have no stored banner yet, oldest → newest.
    pub fn pulls_without_banner(&self, game: &str, uid: &str) -> Result<Vec<Pull>, AppError> {
        let conn = self.conn();
        let mut st = conn.prepare(
            "SELECT p.id, p.game, p.uid, p.gacha_type, p.gacha_id, p.item_id, p.name, p.item_type,
                    p.rank_type, p.time, p.lang, p.count
             FROM pulls p
             WHERE p.game = ?1 AND p.uid = ?2
               AND NOT EXISTS (SELECT 1 FROM pull_banners b
                               WHERE b.game = p.game AND b.uid = p.uid AND b.pull_id = p.id)
             ORDER BY p.time ASC, CAST(p.id AS INTEGER) ASC",
        )?;
        let rows = st.query_map(params![game, uid], |r| {
            Ok(Pull {
                id: r.get(0)?,
                game: r.get(1)?,
                uid: r.get(2)?,
                gacha_type: r.get(3)?,
                gacha_id: r.get(4)?,
                item_id: r.get(5)?,
                name: r.get(6)?,
                item_type: r.get(7)?,
                rank_type: r.get(8)?,
                time: r.get(9)?,
                lang: r.get(10)?,
                count: r.get(11)?,
            })
        })?;
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
        let mut st = conn.prepare(
            "SELECT id, game, uid, gacha_type, gacha_id, item_id, name, item_type, rank_type, time, lang, count
             FROM pulls WHERE game = ?1 AND uid = ?2 ORDER BY time ASC, CAST(id AS INTEGER) ASC",
        )?;
        let rows = st.query_map(params![game, uid], |r| {
            Ok(Pull {
                id: r.get(0)?,
                game: r.get(1)?,
                uid: r.get(2)?,
                gacha_type: r.get(3)?,
                gacha_id: r.get(4)?,
                item_id: r.get(5)?,
                name: r.get(6)?,
                item_type: r.get(7)?,
                rank_type: r.get(8)?,
                time: r.get(9)?,
                lang: r.get(10)?,
                count: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
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
        db.merge_guessed_tz("hk4e", "2", -5, &[]).unwrap();
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
}
