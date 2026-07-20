//! The temporal smoke tests: tiny corpora, zero timing. The gate test
//! runs the full uncapped multiset oracle for every family; the ray
//! test asserts the corpus law's consequence (past the horizon the
//! answer set IS the ray set); the mixed-mask test asserts both planted
//! witness arms answer. Determinism is covered by the registry-wide
//! `scenario_rows_are_deterministic`.

use std::collections::{BTreeMap, BTreeSet};

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{AnswerValue, Answers, Db, Interval, Query, Value};

use crate::families::bind_values;

use super::corpus::{SMOKE, TP_BASE, TP_HORIZON};

/// A scratch store loaded with the SMOKE corpus.
fn smoke_store(name: &str) -> (Db<SchemaDescriptor>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = Db::create(&dir, bumbledb::Theory::descriptor(super::Temporal)).expect("create");
    for (rel, rows) in super::corpus::rows_smoke(7) {
        db.bulk_load_dyn(rel, rows).expect("bulk load");
    }
    (db, dir)
}

/// Executes one two-variable finds query and collects its `(u64, u64)`
/// answer rows.
fn run_pairs(db: &Db<SchemaDescriptor>, query: &Query, params: &[Value]) -> Vec<(u64, u64)> {
    let mut prepared = db.prepare(query).expect("prepare");
    let mut buffer = Answers::new();
    db.read(|snap| snap.execute(&mut prepared, &bind_values(params), &mut buffer))
        .expect("execute");
    let cell = |row: usize, col: usize| match buffer.get(row, col) {
        AnswerValue::U64(v) => v,
        other => panic!("a u64 find column, got {other:?}"),
    };
    (0..buffer.len())
        .map(|row| (cell(row, 0), cell(row, 1)))
        .collect()
}

/// The recomputed SMOKE span rows as `id → interval` — the corpus fn is
/// pure in the seed, so the map is the store's ground truth.
fn spans_by_id() -> BTreeMap<u64, Interval<i64>> {
    super::corpus::spans(7, &SMOKE)
        .iter()
        .map(|row| match (&row[0], &row[2]) {
            (Value::U64(id), Value::IntervalI64(iv)) => (*id, *iv),
            other => panic!("a span row is (id, key, span, weight), got {other:?}"),
        })
        .collect()
}

/// The tier-0 oracle gate: every temporal family × param set at SMOKE
/// scale must produce value-identical multisets on both engines — the
/// FULL uncapped gate (`gate_scenario`), correctness only, no timing.
#[test]
fn temporal_smoke_gate_agrees_on_every_family() {
    let dir = std::env::temp_dir().join("bumbledb-temporal-smoke-gate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    crate::scenarios::gate_scenario(&dir, &super::scenario_smoke(), 7)
        .expect("every temporal family agrees with SQLite at smoke scale");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The corpus law's consequence, asserted: at a post-horizon instant
/// the stabbing query's answer set is EXACTLY the ray rows — every
/// bounded span has ended (ends strictly inside the horizon by
/// construction) and every ray covers the instant (all ray starts sit
/// below the horizon end). No ray predicate exists anywhere in t4; the
/// separation is the coordinates'.
#[test]
fn ray_stab_answers_only_rays_at_smoke() {
    let (db, dir) = smoke_store("bumbledb-temporal-ray-stab");
    let ray_ids: BTreeSet<u64> = spans_by_id()
        .into_iter()
        .filter_map(|(id, iv)| iv.is_ray().then_some(id))
        .collect();
    assert!(ray_ids.len() >= 2, "at least the two planted rays exist");
    let answers = run_pairs(
        &db,
        &super::stab(),
        &[Value::I64(TP_BASE + TP_HORIZON + 1_000)],
    );
    assert!(!answers.is_empty(), "the planted rays answer");
    assert_eq!(
        answers.len(),
        ray_ids.len(),
        "every ray starts before the horizon end, so all rays cover the instant"
    );
    for (_key, id) in &answers {
        assert!(ray_ids.contains(id), "a non-ray answered past the horizon");
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The planted witnesses answer: t3 on key 1 is non-empty, at least one
/// answered pair's spans (recomputed from the corpus rows by id) abut
/// exactly (MEETS) and at least one nests strictly (DURING) — both mask
/// arms are asserted, not hoped.
#[test]
fn planted_meets_and_during_answer_at_smoke() {
    let (db, dir) = smoke_store("bumbledb-temporal-mixed-mask");
    let by_id = spans_by_id();
    let answers = run_pairs(&db, &super::mixed_mask(), &[Value::U64(1)]);
    assert!(!answers.is_empty(), "the planted witnesses answer on key 1");
    let mut meets = false;
    let mut during = false;
    for (a, b) in &answers {
        let l = by_id[a];
        let r = by_id[b];
        if l.end() == r.start() {
            meets = true;
        }
        if r.start() < l.start() && l.end() < r.end() {
            during = true;
        }
    }
    assert!(meets, "at least one answered pair abuts exactly (MEETS)");
    assert!(during, "at least one answered pair nests strictly (DURING)");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
