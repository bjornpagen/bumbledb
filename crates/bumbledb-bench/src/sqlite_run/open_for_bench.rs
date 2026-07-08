use std::path::Path;

use rusqlite::Connection;

use crate::corpus;

/// Opens a loaded oracle for a timing run. Every pragma's fairness
/// rationale:
/// - `journal_mode=WAL` (asserted): `SQLite`'s best self for the
///   read-heavy profile.
/// - `synchronous=FULL`: pinned by `00-product.md` — both engines pay
///   the same fsync bill.
/// - `cache_size=-262144` (256 MiB): the corpus should be
///   memory-resident, as it is for bumbledb.
/// - `mmap_size=1 GiB`: zero-copy page reads — the analogue of the
///   engine's LMDB mapping.
/// - `wal_autocheckpoint=0` plus one `wal_checkpoint(TRUNCATE)` now:
///   no checkpoint I/O lands inside a measured window.
///
/// No cache pre-warming beyond that: warmups are the warm-up,
/// identically to ours.
///
/// # Errors
///
/// `SQLite` errors verbatim.
///
/// # Panics
///
/// If WAL refuses to engage — the fairness protocol is unconditional.
pub fn open_for_bench(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    corpus::configure_sqlite(&conn)?;
    conn.pragma_update(None, "mmap_size", 1_073_741_824_i64)?;
    conn.pragma_update(None, "wal_autocheckpoint", 0)?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    Ok(conn)
}
