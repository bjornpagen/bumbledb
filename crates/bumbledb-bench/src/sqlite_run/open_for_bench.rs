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
/// - `mmap_size` covering the whole file ([`mmap_whole_file`]):
///   zero-copy page reads — the analogue of the engine's LMDB mapping,
///   which maps the whole store unconditionally.
/// - `wal_autocheckpoint=0` plus one `wal_checkpoint(TRUNCATE)` now:
///   no checkpoint I/O lands inside a measured window.
///
/// No cache pre-warming beyond that: warmups are the warm-up,
/// identically to ours.
///
/// # Errors
///
/// `SQLite` errors verbatim; the mmap coverage refusal as a
/// `SQLITE_MISUSE`-free string wrapped into `rusqlite::Error` is
/// avoided by panicking instead — see Panics.
///
/// # Panics
///
/// If WAL refuses to engage, or the effective mmap cannot cover the
/// file — the fairness protocol is unconditional.
pub fn open_for_bench(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    corpus::configure_sqlite(&conn)?;
    conn.pragma_update(None, "wal_autocheckpoint", 0)?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    mmap_whole_file(&conn).expect("the fairness protocol is unconditional");
    Ok(conn)
}

/// The memory-residency parity law (finding 074): the oracle's whole
/// file is mmapped — the size derives from the file (plus WAL-growth
/// headroom, floored at the historical 1 GiB), never a constant a
/// scale bump can outgrow — and the value actually in effect is read
/// back: `SQLite` silently clamps at its compile-time ceiling
/// (`SQLITE_MAX_MMAP_SIZE`), and a clamp below the file size would
/// hand the tail to pread+memcpy through the page cache, a one-sided
/// handicap on exactly the whole-scan families.
///
/// # Errors
///
/// The connection lacking a file path, the stat failing, or the
/// effective mmap falling short of the file — each named.
pub fn mmap_whole_file(conn: &Connection) -> Result<(), String> {
    const FLOOR: u64 = 1 << 30;
    const HEADROOM: u64 = 256 << 20;
    let path = conn
        .path()
        .filter(|p| !p.is_empty())
        .ok_or_else(|| "mmap parity: the connection has no file path".to_owned())?;
    let file_bytes = std::fs::metadata(path)
        .map_err(|e| format!("mmap parity: stat {path}: {e}"))?
        .len();
    let want = i64::try_from((file_bytes + HEADROOM).max(FLOOR))
        .map_err(|_| format!("mmap parity: {file_bytes}-byte file overflows the pragma"))?;
    conn.pragma_update(None, "mmap_size", want)
        .map_err(|e| format!("mmap parity: pragma mmap_size: {e}"))?;
    let effective: i64 = conn
        .query_row("PRAGMA mmap_size", [], |row| row.get(0))
        .map_err(|e| format!("mmap parity: readback: {e}"))?;
    if u64::try_from(effective).unwrap_or(0) < file_bytes {
        return Err(format!(
            "mmap parity: {effective} bytes mapped < the {file_bytes}-byte file —              raise SQLITE_MAX_MMAP_SIZE in the bundled build"
        ));
    }
    Ok(())
}
