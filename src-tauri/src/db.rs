//! Local multi-account SQLite storage (our own file in the app-data folder).

use crate::{error::AppError, models::{Account, Pull}};
use rusqlite::{params, Connection};
use std::{path::Path, sync::{Mutex, MutexGuard}};

pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> Result<Db, AppError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA user_version = 1;
             CREATE TABLE IF NOT EXISTS accounts (
                game TEXT NOT NULL, uid TEXT NOT NULL, nickname TEXT NOT NULL DEFAULT '',
                timezone INTEGER NOT NULL DEFAULT 8, PRIMARY KEY (game, uid));
             CREATE TABLE IF NOT EXISTS pulls (
                id TEXT NOT NULL, game TEXT NOT NULL, uid TEXT NOT NULL, gacha_type TEXT NOT NULL,
                gacha_id TEXT NOT NULL DEFAULT '', item_id TEXT NOT NULL DEFAULT '', name TEXT NOT NULL DEFAULT '',
                item_type TEXT NOT NULL DEFAULT '', rank_type INTEGER NOT NULL, time TEXT NOT NULL,
                lang TEXT NOT NULL DEFAULT '', count INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (game, uid, id));
             CREATE INDEX IF NOT EXISTS idx_pulls_acc ON pulls (game, uid, gacha_type);",
        )?;
        Ok(Db(Mutex::new(conn)))
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Upserts the account and merges pulls by unique id. Existing rows are kept, but blank
    /// descriptive fields (name, item type, ...) are filled in from the new copy, so a sparse
    /// import is completed by a later fetch. Returns the number of NEW rows.
    pub fn merge(&self, game: &str, uid: &str, tz: i32, pulls: &[Pull]) -> Result<usize, AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO accounts (game, uid, timezone) VALUES (?1, ?2, ?3)
             ON CONFLICT(game, uid) DO UPDATE SET timezone = excluded.timezone",
            params![game, uid, tz],
        )?;
        let mut added = 0;
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
                let n = ins.execute(params![
                    p.id, game, uid, p.gacha_type, p.gacha_id, p.item_id, p.name, p.item_type, p.rank_type, p.time, p.lang, p.count
                ])?;
                if n == 0 {
                    fill.execute(params![game, uid, p.id, p.gacha_id, p.item_id, p.name, p.item_type, p.lang])?;
                }
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
            Ok(Account { game: r.get(0)?, uid: r.get(1)?, nickname: r.get(2)?, timezone: r.get(3)?, pull_count: r.get(4)? })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn delete_account(&self, game: &str, uid: &str) -> Result<(), AppError> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM pulls WHERE game = ?1 AND uid = ?2", params![game, uid])?;
        tx.execute("DELETE FROM accounts WHERE game = ?1 AND uid = ?2", params![game, uid])?;
        tx.commit()?;
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
                id: r.get(0)?, game: r.get(1)?, uid: r.get(2)?, gacha_type: r.get(3)?, gacha_id: r.get(4)?,
                item_id: r.get(5)?, name: r.get(6)?, item_type: r.get(7)?, rank_type: r.get(8)?,
                time: r.get(9)?, lang: r.get(10)?, count: r.get(11)?,
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
            id: id.into(), game: "hk4e".into(), uid: "1".into(), gacha_type: "301".into(), gacha_id: String::new(),
            item_id: String::new(), name: name.into(), item_type: String::new(), rank_type: 4,
            time: "2024-01-01 00:00:00".into(), lang: String::new(), count: 1,
        }
    }

    #[test]
    fn merge_is_idempotent_and_fills_blank_fields() {
        let path = std::env::temp_dir().join(format!("wishtory-db-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path).unwrap();
        assert_eq!(db.merge("hk4e", "1", 8, &[pull("10", ""), pull("11", "Keep")]).unwrap(), 2);
        // Same ids again: nothing new, but the blank name of id 10 gets filled and "Keep" stays.
        assert_eq!(db.merge("hk4e", "1", 8, &[pull("10", "Filled"), pull("11", "Other")]).unwrap(), 0);
        let rows = db.pulls_for("hk4e", "1").unwrap();
        assert_eq!(rows.iter().find(|p| p.id == "10").unwrap().name, "Filled");
        assert_eq!(rows.iter().find(|p| p.id == "11").unwrap().name, "Keep");
        // Accounts are kept apart.
        assert_eq!(db.merge("hk4e", "2", 8, &[pull("10", "X")]).unwrap(), 1);
        assert_eq!(db.list_accounts().unwrap().len(), 2);
        db.delete_account("hk4e", "1").unwrap();
        assert!(db.pulls_for("hk4e", "1").unwrap().is_empty());
        assert_eq!(db.pulls_for("hk4e", "2").unwrap().len(), 1);
    }
}
