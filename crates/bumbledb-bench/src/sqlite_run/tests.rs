use super::*;
use crate::gen::{GenConfig, Scale};
use crate::translate::translate;
use crate::writebench::non_posting_relations;
use crate::{corpus, families, gen, sqlmap};
use rusqlite::Connection;

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
    let translated = translate(&(family.query)(), crate::schema::schema(), &[]).expect("translate");
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
                rusqlite::params_from_iter(bind_params(&translated.params, params)),
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
        translate(&(point.query)(), crate::schema::schema(), &[]).expect("translate");
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

    // Clearing fullfsync fails the contract by name (docs/architecture/50-validation.md).
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
