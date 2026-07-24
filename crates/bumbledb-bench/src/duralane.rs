//! The durability-parity lane sum: `DurabilityLane` is the ONLY
//! constructor of both the engine's store mode and the `SQLite` pragma
//! set, so a cross-matched pair (ours durable vs `SQLite` OFF) is
//! unrepresentable — the lane value carries both sides' config.
//!
//! The pairing rationale, recorded once:
//! - **Durable** pairs `Db::create` — LMDB on macOS issues
//!   `F_FULLFSYNC` unconditionally (`lmdb-master-sys` `mdb.c:171`; the
//!   `docs/architecture/60-validation.md` durability-parity clause) —
//!   with `SQLite` WAL `synchronous=FULL` `fullfsync=ON`
//!   `checkpoint_fullfsync=ON`: both engines flush **to media** on
//!   every commit.
//! - **Nosync** pairs `Db::ephemeral` — `MDB_NOSYNC`: pages and meta
//!   are pwritten, no sync boundary is ever crossed — with `SQLite`
//!   WAL `synchronous=OFF` (WAL frames written, never synced). OFF,
//!   not NORMAL, because NORMAL still syncs at WAL checkpoints and
//!   would cross-match a store kind that never syncs at all.
//!
//! Matched pairs only, never cross-matched, by type. The
//! [`DurabilityLane::assert_parity`] readback is the `FairnessCheck`
//! sibling: a misconfigured twin fails before flattering anyone.

use rusqlite::Connection;

use crate::storemode::StoreMode;

/// One durability-matched twin configuration — the closed sum, never a
/// pair of independent switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLane {
    Durable,
    Nosync,
}

/// Both lanes, in report order.
pub const ALL: [DurabilityLane; 2] = [DurabilityLane::Durable, DurabilityLane::Nosync];

impl DurabilityLane {
    /// The engine side of the pair: `Durable` builds with `Db::create`,
    /// `Nosync` with `Db::ephemeral` (`MDB_NOSYNC`).
    #[must_use]
    pub fn store_mode(self) -> StoreMode {
        match self {
            Self::Durable => StoreMode::Durable,
            Self::Nosync => StoreMode::Ephemeral,
        }
    }

    /// The lane's name, as reports and `--lanes` tokens spell it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::Nosync => "nosync",
        }
    }

    /// The `SQLite` twin's documented parity config for this lane.
    #[must_use]
    pub fn sqlite_sync_label(self) -> &'static str {
        match self {
            Self::Durable => "wal+synchronous=FULL+fullfsync=ON",
            Self::Nosync => "wal+synchronous=OFF",
        }
    }

    /// The lane's config prose, rendered verbatim into artifacts.
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
                "Db::ephemeral (MDB_NOSYNC: pages and meta pwritten, no sync boundary ever \
                 crossed) vs SQLite WAL synchronous=OFF fullfsync=OFF checkpoint_fullfsync=OFF, \
                 cache_size=-262144, temp_store=MEMORY, whole-file mmap (coverage asserted), \
                 wal_autocheckpoint=0 — WAL frames written, never synced (OFF, not NORMAL: NORMAL still syncs at \
                 checkpoints, which would cross-match a store kind that never syncs)"
            }
        }
    }

    /// The `SQLite` side of the pair: `Durable` is the standing fairness
    /// config ([`crate::corpus::configure_sqlite`]) plus the bench-open
    /// pragmas (whole-file `mmap_size`, coverage asserted; `wal_autocheckpoint=0` — the
    /// `open_for_bench` set); `Nosync` is the same WAL session with the
    /// sync boundary removed (`synchronous=OFF`, `fullfsync=OFF`,
    /// `checkpoint_fullfsync=OFF`) and the identical cache, temp-store,
    /// and mmap settings.
    ///
    /// # Errors
    ///
    /// `SQLite` errors, stringified with the pragma named.
    ///
    /// # Panics
    ///
    /// If WAL refuses to engage — the fairness protocol is
    /// unconditional (the [`crate::corpus::configure_sqlite`] law).
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

    /// The `FairnessCheck` sibling: reads the session pragmas back and
    /// judges them against this lane — `journal_mode` must be `wal` on
    /// both lanes, `synchronous` must be 2 (FULL) for `Durable` and
    /// 0 (OFF) for `Nosync`, `fullfsync` 1 for `Durable` and 0 for
    /// `Nosync`. A misconfigured twin fails before flattering anyone.
    ///
    /// # Errors
    ///
    /// Any mismatch, naming the pragma, the expected, and the found
    /// value; `SQLite` errors on the readback, stringified.
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
            Self::Durable => 2, // FULL
            Self::Nosync => 0,  // OFF
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

/// One session pragma, with the error naming it.
fn pragma(conn: &Connection, name: &str, value: impl rusqlite::ToSql) -> Result<(), String> {
    conn.pragma_update(None, name, value)
        .map_err(|e| format!("pragma {name}: {e}"))
}
