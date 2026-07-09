use rusqlite::Connection;

use crate::schema::schema;
use crate::sqlmap;

use super::FairnessCheck;

impl FairnessCheck {
    /// Asserts the session and store shape: WAL on, `synchronous=FULL`,
    /// `fullfsync`/`checkpoint_fullfsync` ON (flush-to-media parity with
    /// LMDB's macOS commits — docs/architecture/60-validation.md), every
    /// expected index present (the statement-derived
    /// [`sqlmap::expected_indexes`] registry PLUS the family-owned
    /// composites, `crate::families::expected_indexes`, against
    /// `PRAGMA index_list`), and `ANALYZE` statistics populated.
    /// Statement reuse needs no runtime check — [`PreparedFamily`] owns
    /// the only construction site by type.
    ///
    /// # Errors
    ///
    /// A message naming the first failed rule.
    pub fn run(conn: &Connection) -> Result<(), String> {
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(|e| format!("journal_mode: {e}"))?;
        if mode.to_lowercase() != "wal" {
            return Err(format!("fairness: journal_mode is {mode}, not wal"));
        }
        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(|e| format!("synchronous: {e}"))?;
        if synchronous != 2 {
            return Err(format!(
                "fairness: synchronous is {synchronous}, not FULL (2)"
            ));
        }
        // Durability parity (docs/architecture/50-validation.md): LMDB flushes to media on
        // every macOS commit; SQLite must too, or the write comparison
        // is a lie told at the same synchronous level.
        for pragma in ["fullfsync", "checkpoint_fullfsync"] {
            let on: i64 = conn
                .query_row(&format!("PRAGMA {pragma}"), [], |row| row.get(0))
                .map_err(|e| format!("{pragma}: {e}"))?;
            if on != 1 {
                return Err(format!("fairness: {pragma} is OFF — flush to media"));
            }
        }
        let mut expected = sqlmap::expected_indexes(schema());
        expected.extend(crate::families::expected_indexes());
        for (table, index) in expected {
            let mut stmt = conn
                .prepare(&format!("PRAGMA index_list(\"{table}\")"))
                .map_err(|e| format!("index_list: {e}"))?;
            let present = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("index_list: {e}"))?
                .filter_map(std::result::Result::ok)
                .any(|name| name == index);
            if !present {
                return Err(format!("fairness: index {index} missing on {table}"));
            }
        }
        let analyzed: i64 = conn
            .query_row("SELECT COUNT(*) FROM sqlite_stat1", [], |row| row.get(0))
            .map_err(|_| "fairness: ANALYZE never ran (no sqlite_stat1)".to_owned())?;
        if analyzed == 0 {
            return Err("fairness: sqlite_stat1 is empty — ANALYZE never ran".to_owned());
        }
        Ok(())
    }
}
