//! The churn lane's twin stores and per-cycle appliers: one bumbledb
//! store and N `SQLite` mirrors receiving IDENTICAL logical operations
//! at identical commit granularity — one transaction per cycle on both
//! sides. Delete-bearing **by contract**, not by hope: a removal whose
//! delete is a no-op aborts the whole cycle on ours and refuses on the
//! mirror, so the lane can never silently degrade into insert-only.

use std::path::Path;

use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::{self, GenConfig, Sizes};
use crate::schema::{AccountId, InstrumentId, JournalEntryId, Ledger, Posting, PostingId, ids};

use super::ops::PostingBody;

/// The mirror's sync twin. The parity claim, per lane:
///
/// - [`Full`](Self::Full) is the standard fairness session
///   ([`crate::corpus::configure_sqlite`] — WAL, `synchronous=FULL`,
///   `fullfsync=ON`, `checkpoint_fullfsync=ON`, 256 MiB cache,
///   `temp_store=MEMORY`).
/// - [`Nosync`](Self::Nosync) is the SAME session then
///   `PRAGMA synchronous=OFF` — WAL writes with no sync boundary at
///   commit, the documented matched twin of LMDB's `MDB_NOSYNC`
///   ephemeral kind (both engines' commits reach the OS page cache and
///   never wait on media).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteSync {
    /// The standard fairness session — both engines pay the fsync bill.
    Full,
    /// The fairness session with `synchronous=OFF` — the ephemeral
    /// twin.
    Nosync,
}

impl SqliteSync {
    /// The lane label, as reports print it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Nosync => "nosync",
        }
    }
}

/// The bumbledb side of the churn lane. `last_minted` is the row-id
/// high-water WITNESS: the never-reissue law burns the id space
/// monotonically, and the driver records the burn from its own returned
/// facts — no engine addition is needed.
pub struct OursLane {
    /// The engine store under churn.
    pub db: Db<Ledger>,
    /// The highest posting id minted so far — the monotone-burn
    /// witness.
    pub last_minted: u64,
}

impl std::fmt::Debug for OursLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OursLane")
            .field("last_minted", &self.last_minted)
            .finish_non_exhaustive()
    }
}

/// Creates the engine store and loads every writable relation EXCEPT
/// `PostingTag`: its containment targets postings, and a tagged posting
/// could not be churned out without a target-side abort — the churned
/// relation stays reference-free by construction. The bulk load's
/// explicit ids set the fresh high-water, so `last_minted` starts at
/// `postings - 1`.
///
/// # Errors
///
/// Engine errors, stringified with the failing relation named.
pub fn create_ours(
    dir: &Path,
    r#gen: GenConfig,
    mode: crate::storemode::StoreMode,
) -> Result<OursLane, String> {
    let db = mode.create(dir, Ledger)?;
    for rel in (0..ids::RELATIONS)
        .map(RelationId)
        .filter(|rel| *rel != ids::POSTING_TAG)
    {
        db.bulk_load_dyn(rel, corpus_gen::relation_rows(r#gen, rel))
            .map_err(|e| format!("churn load (relation {}): {e:?}", rel.0))?;
    }
    Ok(OursLane {
        db,
        last_minted: Sizes::of(r#gen.scale).postings - 1,
    })
}

/// Creates one `SQLite` mirror: the fairness session (plus the
/// [`SqliteSync::Nosync`] override when asked), the full schema DDL with
/// the closed vocabularies' extension INSERTs, the same relation set as
/// [`create_ours`] (`PostingTag` excluded for the same reason), then
/// `ANALYZE` and a truncating WAL checkpoint.
///
/// # Errors
///
/// `SQLite` errors, stringified with the failing step named.
///
/// # Panics
///
/// Only on programmer-invariant violations (WAL refused; corpus values
/// breaking the mapping axiom).
pub fn create_sqlite(
    path: &Path,
    r#gen: GenConfig,
    sync: SqliteSync,
) -> Result<rusqlite::Connection, String> {
    let conn = rusqlite::Connection::open(path).map_err(|e| format!("churn mirror open: {e}"))?;
    crate::corpus::configure_sqlite(&conn).map_err(|e| format!("churn mirror configure: {e}"))?;
    if sync == SqliteSync::Nosync {
        conn.pragma_update(None, "synchronous", "OFF")
            .map_err(|e| format!("churn mirror nosync pragma: {e}"))?;
    }
    for statement in crate::sqlmap::ddl(crate::schema::schema()) {
        conn.execute(&statement, [])
            .map_err(|e| format!("churn mirror ddl: {e}"))?;
    }
    for statement in crate::sqlmap::extension_ddl(&bumbledb::Theory::descriptor(Ledger)) {
        conn.execute(&statement, [])
            .map_err(|e| format!("churn mirror extension: {e}"))?;
    }
    for rel in (0..ids::RELATIONS)
        .map(RelationId)
        .filter(|rel| *rel != ids::POSTING_TAG)
    {
        crate::corpus::insert_rows(
            &conn,
            crate::schema::schema().relation(rel),
            corpus_gen::relation_rows(r#gen, rel),
        )
        .map_err(|e| format!("churn mirror load (relation {}): {e}", rel.0))?;
    }
    conn.execute_batch("ANALYZE")
        .map_err(|e| format!("churn mirror analyze: {e}"))?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
        .map_err(|e| format!("churn mirror checkpoint: {e}"))?;
    Ok(conn)
}

/// One posting as its dynamic row — the mirror insert's and the model
/// multiset's shared rendering.
#[must_use]
pub fn posting_values(p: &Posting) -> Vec<Value> {
    vec![
        Value::U64(p.id.0),
        Value::U64(p.entry.0),
        Value::U64(p.account.0),
        Value::U64(p.instrument.0),
        Value::I64(p.amount),
        Value::I64(p.at),
    ]
}

/// Applies one cycle to ours in ONE `db.write`: every removal deletes
/// (a no-op delete aborts the whole transaction inside the closure —
/// the `writebench::posting_swap` in-closure sentinel-abort precedent,
/// so a cycle is delete-bearing by contract and a refusal commits
/// nothing), then every body mints a fresh id and inserts. Returns the
/// added postings.
///
/// # Errors
///
/// Engine errors, stringified; a non-delete-bearing cycle, named.
///
/// # Panics
///
/// On a broken monotone-burn invariant: the minted ids must be strictly
/// ascending and the first must exceed `lane.last_minted` (loud —
/// `last_minted` is the never-reissue law's witness).
pub fn apply_ours(
    lane: &mut OursLane,
    removals: &[Posting],
    bodies: &[PostingBody],
) -> Result<Vec<Posting>, String> {
    let added = lane
        .db
        .write(|tx| {
            for removal in removals {
                if !tx.delete(removal)? {
                    // The in-closure sentinel abort: returning `Err`
                    // here drops the delta whole, so nothing below ever
                    // reaches the store.
                    return Err(bumbledb::Error::Io(std::io::Error::other(
                        "the churn cycle must be delete-bearing: a removal target was absent",
                    )));
                }
            }
            let mut added = Vec::with_capacity(bodies.len());
            for body in bodies {
                let id: PostingId = tx.alloc()?;
                let posting = Posting {
                    id,
                    entry: JournalEntryId(body.entry),
                    account: AccountId(body.account),
                    instrument: InstrumentId(body.instrument),
                    amount: body.amount,
                    at: body.at,
                };
                tx.insert(&posting)?;
                added.push(posting);
            }
            Ok(added)
        })
        .map_err(|e| format!("churn cycle: {e:?}"))?;
    // The monotone-burn invariant, loud: strictly ascending mints, the
    // first above the recorded high-water; unchanged when nothing
    // minted.
    let mut watermark = lane.last_minted;
    for posting in &added {
        assert!(
            posting.id.0 > watermark,
            "the monotone-burn invariant broke: minted id {} does not exceed {watermark}",
            posting.id.0
        );
        watermark = posting.id.0;
    }
    lane.last_minted = watermark;
    Ok(added)
}

/// The mirror's posting delete, by id.
pub const POSTING_DELETE: &str = "DELETE FROM \"Posting\" WHERE \"id\" = ?1";

/// Applies one cycle to a mirror in ONE transaction — the same commit
/// granularity as ours: every removed id deletes exactly one row (else
/// the refusal names the id — the mirror must be delete-bearing too),
/// then every added posting inserts through the normative mapping. A
/// refusal rolls the transaction back whole.
///
/// # Errors
///
/// `SQLite` errors, stringified; a delete affecting anything but one
/// row, naming the id.
///
/// # Panics
///
/// Only on a posting value breaking the mapping axiom (a programmer
/// error).
pub fn apply_sqlite(
    conn: &rusqlite::Connection,
    removed: &[Posting],
    added: &[Posting],
) -> Result<(), String> {
    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("churn mirror begin: {e}"))?;
    match apply_sqlite_body(conn, removed, added) {
        Ok(()) => conn
            .execute_batch("COMMIT")
            .map_err(|e| format!("churn mirror commit: {e}")),
        Err(refusal) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(refusal)
        }
    }
}

/// The transaction body of [`apply_sqlite`] — split out so a refusal
/// can roll back after every cached statement is dropped.
fn apply_sqlite_body(
    conn: &rusqlite::Connection,
    removed: &[Posting],
    added: &[Posting],
) -> Result<(), String> {
    let mut delete = conn
        .prepare_cached(POSTING_DELETE)
        .map_err(|e| format!("churn mirror delete prepare: {e}"))?;
    for posting in removed {
        let affected = delete
            .execute([i64::try_from(posting.id.0).expect("the SQLite mapping axiom: u64 < 2^63")])
            .map_err(|e| format!("churn mirror delete: {e}"))?;
        if affected != 1 {
            return Err(format!(
                "the churn mirror must be delete-bearing: deleting posting {} affected \
                 {affected} rows",
                posting.id.0
            ));
        }
    }
    let insert_sql = crate::sqlmap::insert_sql(crate::schema::schema().relation(ids::POSTING));
    let mut insert = conn
        .prepare_cached(&insert_sql)
        .map_err(|e| format!("churn mirror insert prepare: {e}"))?;
    for posting in added {
        insert
            .execute(rusqlite::params_from_iter(crate::sqlmap::to_sql_row(
                &posting_values(posting),
            )))
            .map_err(|e| format!("churn mirror insert: {e}"))?;
    }
    Ok(())
}
