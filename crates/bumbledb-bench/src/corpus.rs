//! Corpus loading (docs/architecture/60-validation.md): one generator stream, two stores,
//! identical contents — the precondition for every verify and every
//! timing run.

use std::path::Path;
use std::time::{Duration, Instant};

use bumbledb::{Db, RelationId, Value};
use rusqlite::Connection;

use crate::gen::{relation_rows, GenConfig, Sizes};
use crate::schema::{ids, schema, Ledger};
use crate::sqlmap;

/// One load's outcome.
#[derive(Debug, Clone, Copy)]
pub struct LoadStats {
    pub facts: u64,
    pub wall: Duration,
    pub facts_per_sec: f64,
}

fn load_stats(facts: u64, wall: Duration) -> LoadStats {
    #[allow(clippy::cast_precision_loss)] // reporting, not arithmetic
    let facts_per_sec = facts as f64 / wall.as_secs_f64().max(f64::EPSILON);
    LoadStats {
        facts,
        wall,
        facts_per_sec,
    }
}

/// Loads the corpus into a bumbledb store, relation by relation in the
/// containment-safe declaration order, through the ordinary `bulk_load` path.
///
/// # Errors
///
/// Engine errors from `bulk_load` (dropping the committed count into the
/// message — a corpus load has no resume story; regenerate).
pub fn load_bumbledb(db: &Db<Ledger>, cfg: GenConfig) -> Result<LoadStats, bumbledb::Error> {
    let start = Instant::now();
    let mut facts = 0u64;
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        facts += db.bulk_load(rel, relation_rows(cfg, rel))?;
    }
    Ok(load_stats(facts, start.elapsed()))
}

/// The `SQLite` session PRAGMAs for loading and benching, with fairness
/// rationale (docs/architecture/60-validation.md):
/// - `journal_mode=WAL`: `SQLite`'s best self for a read-heavy profile.
/// - `synchronous=FULL`: the durability level `00-product.md` pins for
///   the comparison (both engines pay the fsync bill).
/// - `fullfsync=ON` + `checkpoint_fullfsync=ON`: under
///   `synchronous=FULL` both engines must flush **to media**. LMDB does
///   unconditionally on macOS (`lmdb-master-sys` `mdb.c:171`:
///   `MDB_FDATASYNC(fd)` = `fcntl(fd, F_FULLFSYNC)` under `__APPLE__`);
///   `SQLite`'s default `fullfsync=OFF` issues a plain `fsync(2)`
///   instead, which macOS does not propagate through the drive cache
///   (the amalgamation's `unixSync`: `F_FULLFSYNC` only `if(fullSync)`).
///   Without parity the write comparison flatters `SQLite` ~40x while
///   claiming the same durability. No-ops off macOS.
/// - `cache_size=-262144` (256 MiB): generous page cache — the corpus
///   should be memory-resident on both sides, as it is for bumbledb.
/// - `temp_store=MEMORY`: no accidental disk temp files.
///
/// # Errors
///
/// `SQLite` errors verbatim.
///
/// # Panics
///
/// If WAL refuses to engage — the fairness protocol is unconditional.
pub fn configure_sqlite(conn: &Connection) -> rusqlite::Result<()> {
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    assert_eq!(mode.to_lowercase(), "wal", "WAL must engage");
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "fullfsync", "ON")?;
    conn.pragma_update(None, "checkpoint_fullfsync", "ON")?;
    conn.pragma_update(None, "cache_size", -262_144)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    Ok(())
}

/// Loads one row stream into a `SQLite` table: prepared-statement inserts
/// in transactions of 4096 rows (mirroring the engine's bulk chunk),
/// interval fields split through the normative mapping. The one insert
/// loop every `SQLite` mirror shares — the ledger corpus, the verify
/// target corpus, and the scenario worlds.
///
/// # Errors
///
/// `SQLite` errors verbatim.
///
/// # Panics
///
/// Only on a row value breaking the mapping axiom (a programmer error).
pub fn insert_rows(
    conn: &Connection,
    relation: &bumbledb::schema::Relation,
    rows: impl Iterator<Item = Vec<Value>>,
) -> rusqlite::Result<u64> {
    let insert = sqlmap::insert_sql(relation);
    let mut facts = 0u64;
    let mut rows = rows.peekable();
    while rows.peek().is_some() {
        conn.execute_batch("BEGIN IMMEDIATE")?;
        {
            let mut stmt = conn.prepare_cached(&insert)?;
            for row in rows.by_ref().take(4096) {
                stmt.execute(rusqlite::params_from_iter(sqlmap::to_sql_row(&row)))?;
                facts += 1;
            }
        }
        conn.execute_batch("COMMIT")?;
    }
    Ok(facts)
}

/// Loads one relation's generator stream into the `SQLite` mirror.
///
/// # Errors
///
/// `SQLite` errors verbatim.
///
/// # Panics
///
/// Only on a corpus value breaking the mapping axiom (a programmer
/// error).
pub fn load_sqlite_relation(
    conn: &Connection,
    cfg: GenConfig,
    rel: RelationId,
) -> rusqlite::Result<u64> {
    insert_rows(conn, schema().relation(rel), relation_rows(cfg, rel))
}

/// Creates, configures, and loads the `SQLite` mirror: DDL from the schema
/// descriptors, every relation via [`load_sqlite_relation`], then
/// `ANALYZE` and a truncating WAL checkpoint.
///
/// # Errors
///
/// `SQLite` errors verbatim.
///
/// # Panics
///
/// Only on programmer-invariant violations (WAL refused; corpus values
/// breaking the mapping axiom).
pub fn load_sqlite(path: &Path, cfg: GenConfig) -> rusqlite::Result<(Connection, LoadStats)> {
    let conn = Connection::open(path)?;
    configure_sqlite(&conn)?;
    for statement in sqlmap::ddl(schema()) {
        conn.execute(&statement, [])?;
    }

    let start = Instant::now();
    let mut facts = 0u64;
    for rel in 0..ids::RELATIONS {
        facts += load_sqlite_relation(&conn, cfg, RelationId(rel))?;
    }
    conn.execute_batch("ANALYZE")?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    Ok((conn, load_stats(facts, start.elapsed())))
}

/// Cross-store equality: per-relation counts, then a seeded sample of
/// facts fetched from `SQLite` by fresh id and compared value-for-value
/// against the generator (which both stores loaded from).
///
/// # Panics
///
/// On any inequality — this is test/verify support, not a soft check.
pub fn assert_loaded_equal(db: &Db<Ledger>, conn: &Connection, cfg: GenConfig) {
    let schema = schema();
    let sizes = Sizes::of(cfg.scale);
    for rel in 0..ids::RELATIONS {
        let rel = RelationId(rel);
        let name = schema.relation(rel).name();
        let ours = db
            .read(|snap| Ok(snap.scan(rel)?.count()))
            .expect("scan counts");
        let theirs: u64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(ours as u64, theirs, "row counts diverge for {name}");
        assert_eq!(ours as u64, sizes.rows(rel), "generator count for {name}");
    }

    // 100 seeded sample postings, fetched from SQLite by id, compared to
    // the generator's row (bumbledb equality to the generator is already
    // covered transitively by counts + set semantics + the verify layer).
    let mut rng = crate::gen::Rng::new(cfg.seed ^ 0xA5A5);
    for _ in 0..100 {
        let i = rng.range(sizes.postings);
        let expected = crate::gen::row(&cfg, &sizes, ids::POSTING, i);
        let relation = schema.relation(ids::POSTING);
        let got: Vec<Value> = conn
            .query_row(
                "SELECT * FROM \"Posting\" WHERE \"id\" = ?1",
                [i64::try_from(i).expect("axiom")],
                |row| {
                    let mut values = Vec::new();
                    for (idx, field) in relation.fields().iter().enumerate() {
                        let raw: rusqlite::types::Value = row.get(idx)?;
                        values.push(
                            sqlmap::from_sql_value(&raw, &field.value_type)
                                .expect("mapped value decodes"),
                        );
                    }
                    Ok(values)
                },
            )
            .expect("sample fetch");
        assert_eq!(got, expected, "posting {i} diverges");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::Scale;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// Both loads at S scale, then the cross-store equality sweep — the
    /// passing gate in one test (S is the test scale by design).
    #[test]
    fn both_stores_load_the_same_corpus() {
        let dir = scratch("corpus-load");
        let cfg = GenConfig {
            seed: 1,
            scale: Scale::S,
        };
        let db = Db::create(&dir.join("db"), Ledger).expect("create");
        let ours = load_bumbledb(&db, cfg).expect("bumbledb load");
        let (conn, theirs) = load_sqlite(&dir.join("oracle.sqlite"), cfg).expect("sqlite load");
        assert_eq!(ours.facts, theirs.facts);
        assert!(ours.facts_per_sec > 0.0 && theirs.facts_per_sec > 0.0);
        assert_loaded_equal(&db, &conn, cfg);

        // PRAGMA verification: the fairness settings actually engaged.
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("pragma");
        assert_eq!(mode.to_lowercase(), "wal");
        let sync: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .expect("pragma");
        assert_eq!(sync, 2, "FULL");

        drop((db, conn));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
