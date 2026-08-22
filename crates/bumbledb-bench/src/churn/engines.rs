use std::path::Path;

use bumbledb::{Db, RelationId, Value};

use crate::corpus_gen::{self, GenConfig, Sizes};
use crate::schema::{AccountId, InstrumentId, JournalEntryId, Ledger, Posting, PostingId, ids};

use super::ops::PostingBody;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqliteSync {
    Full,

    Nosync,
}

impl SqliteSync {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Nosync => "nosync",
        }
    }
}

pub struct OursLane {
    pub db: Db<Ledger>,

    pub last_minted: u64,
}

impl std::fmt::Debug for OursLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OursLane")
            .field("last_minted", &self.last_minted)
            .finish_non_exhaustive()
    }
}

/// # Errors
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
        db.write(|tx| {
            tx.insert_dyn(rel, corpus_gen::relation_rows(r#gen, rel))
                .map(bumbledb::MutationReport::changed)
        })
        .map_err(|e| format!("churn load (relation {}): {e:?}", rel.0))?
        .unwrap();
    }
    Ok(OursLane {
        db,
        last_minted: Sizes::of(r#gen.scale).postings - 1,
    })
}

/// # Errors
/// # Panics
/// Only on programmer-invariant violations (WAL refused; corpus values
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

/// Applies one cycle to ours in ONE `db.write`: every removal deletes (a no-op
/// delete aborts the whole transaction inside the closure — the
/// `writebench::posting_swap` in-closure sentinel-abort precedent, so a cycle
/// is delete-bearing by contract and a refusal commits nothing), then every
/// body mints a fresh id and inserts.
/// # Errors
/// # Panics
/// On a broken monotone-burn invariant: the minted ids must be strictly
pub fn apply_ours(
    lane: &mut OursLane,
    removals: &[Posting],
    bodies: &[PostingBody],
) -> Result<Vec<Posting>, String> {
    let added = lane
        .db
        .write(|tx| {
            for removal in removals {
                if tx.delete([removal])?.changed() == 0 {
                    return Err(bumbledb::Error::from(std::io::Error::other(
                        "the churn cycle must be delete-bearing: a removal target was absent",
                    )));
                }
            }
            let mut added = Vec::with_capacity(bodies.len());
            for body in bodies {
                let id: PostingId = tx.reserve(1)?.start().expect("nonempty");
                let posting = Posting {
                    id,
                    entry: JournalEntryId(body.entry),
                    account: AccountId(body.account),
                    instrument: InstrumentId(body.instrument),
                    amount: body.amount,
                    at: body.at,
                };
                tx.insert([&posting])?;
                added.push(posting);
            }
            Ok(added)
        })
        .map_err(|e| format!("churn cycle: {e:?}"))?
        .unwrap()
        .value;
    // The monotone-burn invariant, loud: strictly ascending mints, the

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

pub const POSTING_DELETE: &str = "DELETE FROM \"Posting\" WHERE \"id\" = ?1";

/// granularity as ours: every removed id deletes exactly one row (else the
/// refusal names the id — the mirror must be delete-bearing too), then every
/// added posting inserts through the normative mapping. A refusal rolls the
/// transaction back whole.
/// # Errors
/// # Panics
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

/// The transaction body of [`apply_sqlite`] — split out so a refusal can roll
/// back after every cached statement is dropped.
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
