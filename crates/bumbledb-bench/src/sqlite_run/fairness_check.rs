use rusqlite::Connection;

use crate::schema::schema;
use crate::sqlmap;

use super::FairnessCheck;

impl FairnessCheck {
    /// as a checked invariant, finding 074 — LMDB maps the whole store

    /// # Errors
    pub fn run(conn: &Connection) -> Result<(), String> {
        let mut expected = sqlmap::expected_indexes(schema());
        expected.extend(crate::families::expected_indexes());
        Self::run_with(conn, &expected)
    }

    /// # Errors
    pub fn run_calendar(conn: &Connection) -> Result<(), String> {
        let mut expected = sqlmap::expected_indexes(crate::calendar::schema());
        expected.extend(crate::calendar::families::expected_indexes());
        Self::run_with(conn, &expected)
    }

    fn run_with(conn: &Connection, expected: &[(String, String)]) -> Result<(), String> {
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

        // every macOS commit; SQLite must too, or the write comparison

        for pragma in ["fullfsync", "checkpoint_fullfsync"] {
            let on: i64 = conn
                .query_row(&format!("PRAGMA {pragma}"), [], |row| row.get(0))
                .map_err(|e| format!("{pragma}: {e}"))?;
            if on != 1 {
                return Err(format!("fairness: {pragma} is OFF — flush to media"));
            }
        }
        if let Some(path) = conn.path().filter(|p| !p.is_empty()) {
            let file_bytes = std::fs::metadata(path)
                .map_err(|e| format!("fairness: stat {path}: {e}"))?
                .len();
            let mmap: i64 = conn
                .query_row("PRAGMA mmap_size", [], |row| row.get(0))
                .map_err(|e| format!("mmap_size: {e}"))?;
            if u64::try_from(mmap).unwrap_or(0) < file_bytes {
                return Err(format!(
                    "fairness: mmap_size {mmap} < the {file_bytes}-byte file —                      the memory-residency parity claim is broken"
                ));
            }
        }
        for (table, index) in expected {
            let mut stmt = conn
                .prepare(&format!("PRAGMA index_list(\"{table}\")"))
                .map_err(|e| format!("index_list: {e}"))?;
            let present = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("index_list: {e}"))?
                .filter_map(std::result::Result::ok)
                .any(|name| name == *index);
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
