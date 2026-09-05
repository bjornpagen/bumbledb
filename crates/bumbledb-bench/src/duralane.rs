//! The durability axis, post-ENG-008: exactly one point, **durable**.
//!
//! The old two-point axis paired `Db::create_nosync` (`MDB_NOSYNC`) with
//! SQLite `synchronous=OFF` so neither twin ever crossed a sync boundary.
//! The successor deleted the no-sync constructor surface entirely (ENG-008:
//! benchmark-only weakening must never be a production capability), and the
//! bench cannot honestly weaken the engine from outside the store — so the
//! ours-side NOSYNC lane is dropped, and with it its SQLite OFF twin (an
//! unpaired OFF mirror would compare a syncing engine against a non-syncing
//! one, which chapter 40's fairness rule forbids). Durable pairs `Db::create`
//! (LMDB issues `F_FULLFSYNC` unconditionally on macOS) with SQLite WAL
//! `synchronous=FULL fullfsync=ON`; a misconfigured twin fails before
//! flattering anyone.
use rusqlite::Connection;

use crate::storemode::StoreMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLane {
    Durable,
}

pub const ALL: [DurabilityLane; 1] = [DurabilityLane::Durable];

impl DurabilityLane {
    #[must_use]
    pub fn store_mode(self) -> StoreMode {
        match self {
            Self::Durable => StoreMode::Durable,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
        }
    }

    #[must_use]
    pub fn sqlite_sync_label(self) -> &'static str {
        match self {
            Self::Durable => "wal+synchronous=FULL+fullfsync=ON",
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
        }
        pragma(conn, "wal_autocheckpoint", 0)?;
        crate::sqlite_run::mmap_whole_file(conn)?;
        Ok(())
    }

    /// The parity readback: a misconfigured twin fails before flattering
    /// anyone.
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
