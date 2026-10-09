//! The one durability point every write lane times: the engine's per-commit
//! sync against SQLite WAL with `synchronous=FULL` and `fullfsync`, read back
//! before any timing so a misconfigured twin fails instead of flattering.
use rusqlite::Connection;

/// The pairing rendered into every write report.
pub const DURABILITY: &str = "Db::create (LMDB issues F_FULLFSYNC unconditionally on macOS) vs \
    SQLite WAL synchronous=FULL fullfsync=ON checkpoint_fullfsync=ON, cache_size=-262144, \
    temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — both \
    engines flush to media on every commit";

pub const SQLITE_SYNC: &str = "wal+synchronous=FULL+fullfsync=ON";

/// # Errors
pub fn configure_durable(conn: &Connection) -> Result<(), String> {
    crate::worlds::corpus::configure_sqlite(conn)
        .map_err(|e| format!("configure (durable): {e}"))?;
    conn.pragma_update(None, "wal_autocheckpoint", 0)
        .map_err(|e| format!("pragma wal_autocheckpoint: {e}"))?;
    super::mmap_whole_file(conn)
}

/// # Errors
pub fn assert_durable_parity(conn: &Connection) -> Result<(), String> {
    let journal: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(|e| format!("pragma journal_mode: {e}"))?;
    if journal.to_lowercase() != "wal" {
        return Err(format!(
            "parity: pragma journal_mode: expected wal, found {journal}"
        ));
    }
    for (pragma, expected) in [("synchronous", 2), ("fullfsync", 1)] {
        let found: i64 = conn
            .query_row(&format!("PRAGMA {pragma}"), [], |row| row.get(0))
            .map_err(|e| format!("pragma {pragma}: {e}"))?;
        if found != expected {
            return Err(format!(
                "parity: pragma {pragma}: expected {expected}, found {found}"
            ));
        }
    }
    Ok(())
}
