use super::*;
use crate::corpus_gen::{GenConfig, Scale};
use crate::translate::translate;
use crate::{corpus, families};
use rusqlite::Connection;

const CFG: GenConfig = GenConfig {
    seed: 1,
    scale: Scale::Tiny,
};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-bench-sqlite-run-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

#[test]
fn fairness_and_the_prepared_sample_contract() {
    let dir = scratch("read");
    let path = dir.join("oracle.sqlite");
    let (conn, _) = corpus::load_sqlite(&path, CFG).expect("load");
    drop(conn);
    let conn = open_for_bench(&path).expect("open for bench");
    FairnessCheck::run(&conn).expect("fairness holds on a loaded corpus");

    let family = families::all()
        .iter()
        .find(|f| f.name == "range")
        .expect("registered");
    let translated = translate(&(family.query)(), crate::schema::schema(), &[]).expect("translate");
    let types: Vec<ValueType> = {
        let db_dir = dir.join("types-db");
        let db = bumbledb::Db::create(&db_dir, crate::schema::Ledger)
            .expect("create")
            .expect("accepted");
        let prepared = db.prepare(&(family.query)()).expect("prepare");
        prepared
            .signature()
            .columns
            .iter()
            .map(|column| *column.ty())
            .collect()
    };
    let mut prepared = PreparedFamily::new(&conn, &translated, types).expect("prepare once");

    let sets = (family.params)(&CFG);
    let mut counts = Vec::new();
    for params in &sets {
        let count = sample_args(&mut prepared, params).expect("sample");

        let expected: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM ({})", translated.sql),
                rusqlite::params_from_iter(bind_args(&translated.params, params)),
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

    let point = families::all()
        .iter()
        .find(|f| f.name == "point")
        .expect("registered");
    let point_translated =
        translate(&(point.query)(), crate::schema::schema(), &[]).expect("translate");
    let point_types: Vec<ValueType> = {
        let db = bumbledb::Db::open(&dir.join("types-db"), crate::schema::Ledger).expect("reopen");
        let prepared = db.prepare(&(point.query)()).expect("prepare");
        prepared
            .signature()
            .columns
            .iter()
            .map(|column| *column.ty())
            .collect()
    };
    let mut point_prepared =
        PreparedFamily::new(&conn, &point_translated, point_types).expect("prepare once");
    let point_counts: Vec<u64> = (point.params)(&CFG)
        .iter()
        .map(|params| sample_args(&mut point_prepared, params).expect("sample"))
        .collect();
    assert_eq!(point_counts, vec![1, 1, 1, 0], "hits then the miss");
    drop(point_prepared);

    for round in 0..100 {
        let set = &sets[round % sets.len()];
        sample_args(&mut prepared, set).expect("reused statement");
    }

    conn.pragma_update(None, "fullfsync", "OFF")
        .expect("pragma");
    let err = FairnessCheck::run(&conn).expect_err("must fail");
    assert!(err.contains("fullfsync"), "{err}");
    conn.pragma_update(None, "fullfsync", "ON").expect("pragma");

    // Drop a family-owned index: the contract fails naming it.
    conn.execute("DROP INDEX \"idx_posting_at\"", [])
        .expect("drop");
    let err = FairnessCheck::run(&conn).expect_err("must fail");
    assert!(err.contains("idx_posting_at"), "{err}");
    drop(prepared);
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cap_trips_on_a_slow_query_and_passes_a_fast_one() {
    let conn = Connection::open_in_memory().expect("open");
    let mut slow = conn
        .prepare(
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM c WHERE x < 1000000000) \
             SELECT COUNT(*) FROM c",
        )
        .expect("prepare");
    let outcome = with_cap(&conn, CapMs(50), || {
        slow.query_row([], |r| r.get::<_, i64>(0))
    })
    .expect("the interrupt is a CapOutcome, not an error");
    assert_eq!(outcome, CapOutcome::Tripped);
    drop(slow);

    let mut fast = conn.prepare("SELECT 1").expect("prepare");
    let outcome = with_cap(&conn, CapMs(10_000), || {
        fast.query_row([], |r| r.get::<_, i64>(0))
    })
    .expect("a fast query under a generous cap");
    assert_eq!(outcome, CapOutcome::Done(1));
}
