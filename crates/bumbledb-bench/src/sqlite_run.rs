//! The `SQLite` runner and the fairness contract (docs/benchmarks/16):
//! `SQLite` measured under exactly the engine's protocol, with the
//! fairness rules encoded as assertions — a benchmark nobody can dismiss
//! as a strawman.
//!
//! Symmetry argument for the timed path: bumbledb materializes every row
//! into a `ResultBuffer`; the `SQLite` side does typed `get_ref` reads on
//! every column of every row (a full drain — no lazy-cursor discounts).
//! Both engines touch every value; decoding into `compare::Owned` is
//! verify's job, never the timed path's.

use std::path::Path;

use bumbledb::schema::ValueType;
use bumbledb::{ParamId, Value};
use rusqlite::Connection;

use crate::gen::{GenConfig, Rng, Sizes};
use crate::harness::{self, Measurement};
use crate::schema::{schema, PostingId};
use crate::translate::Translated;
use crate::writebench::{non_posting_relations, seeded_posting, write_protocol};
use crate::{corpus, sqlmap};

/// A family's statement, prepared exactly once and reused across every
/// warmup and sample (mirroring `PreparedQuery`). This is the **only**
/// construction site for timed `SQLite` statements — statement reuse is
/// asserted by type: no re-prepare path exists.
pub struct PreparedFamily<'c> {
    stmt: rusqlite::Statement<'c>,
    param_order: Vec<ParamId>,
    result_types: Vec<ValueType>,
}

impl<'c> PreparedFamily<'c> {
    /// Prepares the translated SQL once against the bench connection.
    ///
    /// # Errors
    ///
    /// `SQLite` errors, stringified.
    pub fn new(
        conn: &'c Connection,
        translated: &Translated,
        result_types: Vec<ValueType>,
    ) -> Result<Self, String> {
        Ok(Self {
            stmt: conn
                .prepare(&translated.sql)
                .map_err(|e| format!("prepare: {e}"))?,
            param_order: translated.params.clone(),
            result_types,
        })
    }
}

/// One timed sample: bind via the normative mapping, drain ALL rows with
/// typed reads on every column, return the row count (the harness's
/// black-box/work contract).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn sample(family: &mut PreparedFamily<'_>, params: &[Value]) -> Result<u64, String> {
    let bound: Vec<rusqlite::types::Value> = family
        .param_order
        .iter()
        .map(|p| sqlmap::to_sql_value(&params[usize::from(p.0)]))
        .collect();
    let mut rows = family
        .stmt
        .query(rusqlite::params_from_iter(bound))
        .map_err(|e| format!("query: {e}"))?;
    let mut count = 0u64;
    while let Some(row) = rows.next().map_err(|e| format!("step: {e}"))? {
        for (column, ty) in family.result_types.iter().enumerate() {
            let value = row.get_ref(column).map_err(|e| format!("read: {e}"))?;
            match ty {
                ValueType::Bool | ValueType::Enum { .. } | ValueType::U64 | ValueType::I64 => {
                    std::hint::black_box(value.as_i64().map_err(|e| format!("i64: {e}"))?);
                }
                ValueType::String => {
                    std::hint::black_box(value.as_str().map_err(|e| format!("str: {e}"))?);
                }
                ValueType::Bytes => {
                    std::hint::black_box(value.as_blob().map_err(|e| format!("blob: {e}"))?);
                }
            }
        }
        count += 1;
    }
    Ok(count)
}

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

/// The fairness contract as code — run before measuring, so a
/// misconfigured oracle fails the run instead of flattering the engine.
pub struct FairnessCheck;

impl FairnessCheck {
    /// Asserts the session and store shape: WAL on, `synchronous=FULL`,
    /// `fullfsync`/`checkpoint_fullfsync` ON (flush-to-media parity with
    /// LMDB's macOS commits — docs/perf/08), every expected index
    /// present (the [`sqlmap::expected_indexes`] registry against
    /// `PRAGMA index_list`), and `ANALYZE` statistics populated. Statement reuse needs no runtime check —
    /// [`PreparedFamily`] owns the only construction site by type.
    ///
    /// # Errors
    ///
    /// A message naming the first failed rule.
    pub fn run(conn: &Connection) -> Result<(), String> {
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
        // Durability parity (docs/perf/08): LMDB flushes to media on
        // every macOS commit; SQLite must too, or the write comparison
        // is a lie told at the same synchronous level.
        for pragma in ["fullfsync", "checkpoint_fullfsync"] {
            let on: i64 = conn
                .query_row(&format!("PRAGMA {pragma}"), [], |row| row.get(0))
                .map_err(|e| format!("{pragma}: {e}"))?;
            if on != 1 {
                return Err(format!("fairness: {pragma} is OFF — flush to media"));
            }
        }
        for (table, index) in sqlmap::expected_indexes(schema()) {
            let mut stmt = conn
                .prepare(&format!("PRAGMA index_list(\"{table}\")"))
                .map_err(|e| format!("index_list: {e}"))?;
            let present = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("index_list: {e}"))?
                .filter_map(std::result::Result::ok)
                .any(|name| name == index);
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

/// The `SQLite` posting insert, mirroring the corpus loader's shape.
const POSTING_INSERT: &str = "INSERT INTO \"Posting\" VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

fn sqlite_posting_params(rng: &mut Rng, sizes: &Sizes, id: u64) -> [rusqlite::types::Value; 8] {
    use rusqlite::types::Value as Sql;
    let posting = seeded_posting(rng, sizes, PostingId(id));
    [
        Sql::Integer(i64::try_from(id).expect("axiom")),
        Sql::Integer(i64::try_from(posting.transfer.0).expect("axiom")),
        Sql::Integer(i64::try_from(posting.account.0).expect("axiom")),
        Sql::Integer(i64::try_from(posting.instrument.0).expect("axiom")),
        Sql::Integer(posting.amount),
        Sql::Integer(posting.at),
        Sql::Text(posting.memo),
        Sql::Integer(i64::from(posting.reconciled)),
    ]
}

fn next_posting_id(conn: &Connection) -> Result<u64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(\"id\"), -1) + 1 FROM \"Posting\"",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|next| u64::try_from(next).expect("dense ids"))
    .map_err(|e| format!("next id: {e}"))
}

/// `commit_single` on `SQLite`: one sample = one bound `INSERT` on a
/// reused prepared statement inside `BEGIN IMMEDIATE … COMMIT` (the WAL +
/// `synchronous=FULL` session — the same fsync bill the engine pays).
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn commit_single(conn: &Connection, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0001);
    let mut next = next_posting_id(conn)?;
    harness::measure(write_protocol("commit_single"), || {
        let mut run = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            conn.prepare_cached(POSTING_INSERT)?
                .execute(sqlite_posting_params(&mut rng, &sizes, next))?;
            conn.execute_batch("COMMIT")
        };
        run().map_err(|e| format!("commit_single sqlite: {e}"))?;
        next += 1;
        Ok(1)
    })
}

/// `commit_batch` on `SQLite`: one sample = 512 bound executions in one
/// transaction.
///
/// # Errors
///
/// `SQLite` errors, stringified.
pub fn commit_batch(conn: &Connection, cfg: GenConfig) -> Result<Measurement, String> {
    let sizes = Sizes::of(cfg.scale);
    let mut rng = Rng::new(cfg.seed ^ 0x0115_0002);
    let mut next = next_posting_id(conn)?;
    harness::measure(write_protocol("commit_batch"), || {
        let mut run = || -> rusqlite::Result<()> {
            conn.execute_batch("BEGIN IMMEDIATE")?;
            {
                let mut stmt = conn.prepare_cached(POSTING_INSERT)?;
                for _ in 0..512 {
                    stmt.execute(sqlite_posting_params(&mut rng, &sizes, next))?;
                    next += 1;
                }
            }
            conn.execute_batch("COMMIT")
        };
        run().map_err(|e| format!("commit_batch sqlite: {e}"))?;
        Ok(512)
    })
}

/// `bulk` on `SQLite`: pre-seeded throwaway files (the corpus minus
/// postings, built before any timing), the full posting stream timed in
/// 4096-row transactions per sample.
///
/// # Errors
///
/// `SQLite` errors, stringified.
///
/// # Panics
///
/// On scratch I/O failures.
pub fn bulk(cfg: GenConfig, scratch: &Path) -> Result<Measurement, String> {
    use std::cell::RefCell;
    let proto = write_protocol("bulk");
    let mut pending = std::collections::VecDeque::new();
    for sample in 0..proto.warmups + proto.samples {
        let path = scratch.join(format!("bulk-oracle-{sample}.sqlite"));
        let conn = Connection::open(&path).map_err(|e| format!("open: {e}"))?;
        corpus::configure_sqlite(&conn).map_err(|e| format!("configure: {e}"))?;
        for statement in sqlmap::ddl(schema()) {
            conn.execute(&statement, [])
                .map_err(|e| format!("ddl: {e}"))?;
        }
        for rel in non_posting_relations() {
            corpus::load_sqlite_relation(&conn, cfg, rel).map_err(|e| format!("seed: {e}"))?;
        }
        pending.push_back(conn);
    }
    let pending = RefCell::new(pending);
    let done = RefCell::new(Vec::new());
    harness::measure(proto, || {
        let conn = pending.borrow_mut().pop_front().expect("pre-seeded store");
        let facts = corpus::load_sqlite_relation(&conn, cfg, crate::schema::ids::POSTING)
            .map_err(|e| format!("bulk sqlite: {e}"))?;
        done.borrow_mut().push(conn);
        Ok(facts)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::Scale;
    use crate::translate::translate;
    use crate::{families, gen};

    const CFG: GenConfig = GenConfig {
        seed: 1,
        scale: Scale::S,
    };

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bumbledb-bench-sqlite-run-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// One loaded S oracle covers the read-side criteria: the fairness
    /// contract passes, then fails by name when an index is dropped;
    /// sample drains (count == COUNT(*) cross-check); re-binding across
    /// param sets changes counts; one `PreparedFamily` runs 100 samples.
    #[test]
    fn fairness_and_the_prepared_sample_contract() {
        let dir = scratch("read");
        let path = dir.join("oracle.sqlite");
        let (conn, _) = corpus::load_sqlite(&path, CFG).expect("load");
        drop(conn);
        let conn = open_for_bench(&path).expect("open for bench");
        FairnessCheck::run(&conn).expect("fairness holds on a loaded corpus");

        // The range family: window params make counts differ per set.
        let family = families::all()
            .iter()
            .find(|f| f.name == "range")
            .expect("registered");
        let translated = translate(&(family.query)(), crate::schema::schema()).expect("translate");
        let types: Vec<ValueType> = {
            let db_dir = dir.join("types-db");
            let db = bumbledb::Db::create(&db_dir, crate::schema::schema()).expect("create");
            let prepared = db.prepare(&(family.query)()).expect("prepare");
            prepared.column_types().cloned().collect()
        };
        let mut prepared = PreparedFamily::new(&conn, &translated, types).expect("prepare once");

        let sets = (family.params)(&CFG);
        let mut counts = Vec::new();
        for params in &sets {
            let count = sample(&mut prepared, params).expect("sample");
            // Drain cross-check: the count matches COUNT(*) over the
            // same SQL and binding.
            let expected: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM ({})", translated.sql),
                    rusqlite::params_from_iter(
                        translated
                            .params
                            .iter()
                            .map(|p| sqlmap::to_sql_value(&params[usize::from(p.0)])),
                    ),
                    |row| row.get(0),
                )
                .expect("count");
            assert_eq!(count, u64::try_from(expected).expect("non-negative"));
            counts.push(count);
        }
        assert!(
            counts.iter().all(|c| *c == counts[0] && *c > 0),
            "the ~2% windows select uniformly by construction: {counts:?}"
        );

        // Re-binding across param sets changes counts: the point family's
        // three hits return one row each, the miss returns none.
        let point = families::all()
            .iter()
            .find(|f| f.name == "point")
            .expect("registered");
        let point_translated =
            translate(&(point.query)(), crate::schema::schema()).expect("translate");
        let point_types: Vec<ValueType> = {
            let db =
                bumbledb::Db::open(&dir.join("types-db"), crate::schema::schema()).expect("reopen");
            let prepared = db.prepare(&(point.query)()).expect("prepare");
            prepared.column_types().cloned().collect()
        };
        let mut point_prepared =
            PreparedFamily::new(&conn, &point_translated, point_types).expect("prepare once");
        let point_counts: Vec<u64> = (point.params)(&CFG)
            .iter()
            .map(|params| sample(&mut point_prepared, params).expect("sample"))
            .collect();
        assert_eq!(point_counts, vec![1, 1, 1, 0], "hits then the miss");
        drop(point_prepared);

        // Prepared-once discipline: 100 samples on the same statement.
        for round in 0..100 {
            let set = &sets[round % sets.len()];
            sample(&mut prepared, set).expect("reused statement");
        }

        // Clearing fullfsync fails the contract by name (docs/perf/08).
        conn.pragma_update(None, "fullfsync", "OFF")
            .expect("pragma");
        let err = FairnessCheck::run(&conn).expect_err("must fail");
        assert!(err.contains("fullfsync"), "{err}");
        conn.pragma_update(None, "fullfsync", "ON").expect("pragma");

        // Drop an index: the contract fails naming it.
        conn.execute("DROP INDEX \"idx_posting_memo\"", [])
            .expect("drop");
        let err = FairnessCheck::run(&conn).expect_err("must fail");
        assert!(err.contains("idx_posting_memo"), "{err}");
        drop(prepared);
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The write mirrors run their full protocols with directionally sane
    /// results (a 512-row transaction outlasts a 1-row one).
    #[test]
    fn write_mirrors_run_with_sane_direction() {
        let dir = scratch("write");
        let conn = Connection::open(dir.join("oracle.sqlite")).expect("open");
        corpus::configure_sqlite(&conn).expect("configure");
        for statement in sqlmap::ddl(crate::schema::schema()) {
            conn.execute(&statement, []).expect("ddl");
        }
        for rel in non_posting_relations() {
            corpus::load_sqlite_relation(&conn, CFG, rel).expect("seed");
        }
        let single = commit_single(&conn, CFG).expect("commit_single");
        assert!(single.stats.min > 0);
        assert_eq!(single.work, 64);
        let batch = commit_batch(&conn, CFG).expect("commit_batch");
        assert_eq!(batch.work, 512 * 32);
        assert!(
            batch.stats.p50 > single.stats.p50,
            "512 rows outlast 1: batch {} vs single {}",
            batch.stats.p50,
            single.stats.p50
        );
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM \"Posting\"", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 64 + 8 + 512 * (32 + 4), "warmups included");
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The bulk mirror reports positive throughput over its protocol.
    #[test]
    fn bulk_mirror_reports_positive_throughput() {
        let dir = scratch("bulk");
        let m = bulk(CFG, &dir).expect("bulk");
        assert_eq!(m.work, gen::Sizes::of(CFG.scale).postings * 8);
        assert!(m.stats.min > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
