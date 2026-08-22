//! The pairing rationale, recorded once: - **Durable** pairs `Db::create` —
//! LMDB on macOS issues sibling: a misconfigured twin fails before flattering
//! anyone.

use rusqlite::Connection;

use crate::storemode::StoreMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLane {
    Durable,
    Nosync,
}

pub const ALL: [DurabilityLane; 2] = [DurabilityLane::Durable, DurabilityLane::Nosync];

impl DurabilityLane {
    #[must_use]
    pub fn store_mode(self) -> StoreMode {
        match self {
            Self::Durable => StoreMode::Durable,
            Self::Nosync => StoreMode::Nosync,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Nosync => "nosync",
        }
    }

    #[must_use]
    pub fn sqlite_sync_label(self) -> &'static str {
        match self {
            Self::Durable => "wal+synchronous=FULL+fullfsync=ON",
            Self::Nosync => "wal+synchronous=OFF",
        }
    }

    #[must_use]
    pub fn describe(self) -> &'static str {
        match self {
            Self::Durable => {
                "Db::create (LMDB issues F_FULLFSYNC unconditionally on macOS) vs SQLite WAL \
                 synchronous=FULL fullfsync=ON checkpoint_fullfsync=ON, cache_size=-262144, \
                 temp_store=MEMORY, whole-file mmap (coverage asserted), wal_autocheckpoint=0 — \
                 both engines flush to media on every commit"
            }
            Self::Nosync => {
                "Db::create_nosync (MDB_NOSYNC: pages and meta pwritten, no sync boundary ever \
                 crossed; durable-shaped store, not a kind) vs SQLite WAL synchronous=OFF \
                 fullfsync=OFF checkpoint_fullfsync=OFF, cache_size=-262144, temp_store=MEMORY, \
                 whole-file mmap (coverage asserted), wal_autocheckpoint=0 — WAL frames written, \
                 never synced (OFF, not NORMAL: NORMAL still syncs at checkpoints, which would \
                 cross-match a lane that never syncs)"
            }
        }
    }

    /// # Errors

    /// # Panics

    /// If WAL refuses to engage — the fairness protocol is

    pub fn configure(self, conn: &Connection) -> Result<(), String> {
        match self {
            Self::Durable => {
                crate::corpus::configure_sqlite(conn)
                    .map_err(|e| format!("configure (durable): {e}"))?;
            }
            Self::Nosync => {
                let mode: String = conn
                    .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
                    .map_err(|e| format!("pragma journal_mode: {e}"))?;
                assert_eq!(mode.to_lowercase(), "wal", "WAL must engage");
                pragma(conn, "synchronous", "OFF")?;
                pragma(conn, "fullfsync", "OFF")?;
                pragma(conn, "checkpoint_fullfsync", "OFF")?;
                pragma(conn, "cache_size", -262_144)?;
                pragma(conn, "temp_store", "MEMORY")?;
            }
        }
        pragma(conn, "wal_autocheckpoint", 0)?;
        crate::sqlite_run::mmap_whole_file(conn)?;
        Ok(())
    }

    /// `Nosync`. A misconfigured twin fails before flattering anyone.

    /// # Errors

    pub fn assert_parity(self, conn: &Connection) -> Result<(), String> {
        let journal: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .map_err(|e| format!("pragma journal_mode: {e}"))?;
        if journal.to_lowercase() != "wal" {
            return Err(format!(
                "parity ({}): pragma journal_mode: expected wal, found {journal}",
                self.label()
            ));
        }
        let expected_sync: i64 = match self {
            Self::Durable => 2,
            Self::Nosync => 0,
        };
        let sync: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .map_err(|e| format!("pragma synchronous: {e}"))?;
        if sync != expected_sync {
            return Err(format!(
                "parity ({}): pragma synchronous: expected {expected_sync}, found {sync}",
                self.label()
            ));
        }
        let expected_fullfsync: i64 = match self {
            Self::Durable => 1,
            Self::Nosync => 0,
        };
        let fullfsync: i64 = conn
            .query_row("PRAGMA fullfsync", [], |row| row.get(0))
            .map_err(|e| format!("pragma fullfsync: {e}"))?;
        if fullfsync != expected_fullfsync {
            return Err(format!(
                "parity ({}): pragma fullfsync: expected {expected_fullfsync}, found {fullfsync}",
                self.label()
            ));
        }
        Ok(())
    }
}

fn pragma(conn: &Connection, name: &str, value: impl rusqlite::ToSql) -> Result<(), String> {
    conn.pragma_update(None, name, value)
        .map_err(|e| format!("pragma {name}: {e}"))
}
