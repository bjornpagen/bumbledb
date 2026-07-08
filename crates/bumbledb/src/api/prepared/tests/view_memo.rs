use super::*;

/// The view-memo LRU (docs/architecture/30-execution.md): four rotating residual bindings
/// all memoize; a fifth evicts exactly the least recently used.
#[cfg(feature = "trace")]
#[test]
fn residual_bindings_memoize_under_lru() {
    use crate::obs;

    let dir = TempDir::new("prepared-lru-trace");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[
            (1, 7, "a", 10),
            (2, 7, "b", 20),
            (3, 7, "c", 30),
            (4, 7, "d", 40),
            (5, 7, "e", 50),
            (6, 7, "f", 60),
        ],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let params = |floor: i64| vec![Value::U64(7), Value::I64(floor)];
    let windows = [-100, 15, 25, 35];

    let mut run = |floor: i64| -> (usize, usize, Vec<(String, i64)>) {
        obs::start_capture();
        let out = prepared
            .execute_collect(&txn, &cache, &params(floor))
            .expect("execute");
        let events = obs::finish_capture();
        let builds = events
            .iter()
            .filter(|e| e.name == obs::names::VIEW_BUILD)
            .count();
        let hits = events
            .iter()
            .filter(|e| e.name == obs::names::VIEW_MEMO_HIT)
            .count();
        (builds, hits, rows_of(&out))
    };
    let expected = |floor: i64| -> Vec<(String, i64)> {
        let rows = [
            ("a", 10),
            ("b", 20),
            ("c", 30),
            ("d", 40),
            ("e", 50),
            ("f", 60),
        ];
        let mut expected: Vec<(String, i64)> = rows
            .iter()
            .filter(|(_, amount)| *amount >= floor)
            .map(|(memo, amount)| ((*memo).to_owned(), *amount))
            .collect();
        expected.sort_unstable();
        expected
    };

    // First cycle: every window builds once (differentially checked).
    for floor in windows {
        let (builds, _, rows) = run(floor);
        assert_eq!(builds, 1, "first sight of window {floor} builds");
        assert_eq!(rows, expected(floor));
    }
    // Second cycle: every window hits — active or parked.
    for floor in windows {
        let (builds, hits, rows) = run(floor);
        assert_eq!(builds, 0, "window {floor} memoized");
        assert_eq!(hits, 1);
        assert_eq!(rows, expected(floor));
    }
    // A fifth window evicts the least recently used (floor -100).
    let (builds, _, rows) = run(45);
    assert_eq!(builds, 1, "the fifth binding builds");
    assert_eq!(rows, expected(45));
    // The most recent of the old four still hits...
    let (builds, hits, _) = run(35);
    assert_eq!((builds, hits), (0, 1), "most recent old binding kept");
    // ...and the least recent was the eviction victim.
    let (builds, _, rows) = run(-100);
    assert_eq!(builds, 1, "least recent binding was evicted");
    assert_eq!(rows, expected(-100));
}

/// A generation bump invalidates every memoized binding, and the
/// rebuilt view reflects the new fact.
#[cfg(feature = "trace")]
#[test]
fn a_generation_bump_invalidates_the_memo() {
    use crate::obs;

    let dir = TempDir::new("prepared-lru-generation");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "old", 10)]);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let params = vec![Value::U64(7), Value::I64(0)];
    let out = prepared
        .execute_collect(&txn, &cache, &params)
        .expect("execute");
    assert_eq!(out.len(), 1);
    drop(txn);

    insert_postings(&env, &schema, &[(2, 7, "new", 20)]);
    let txn = env.read_txn().expect("txn");
    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &params)
        .expect("execute");
    let events = obs::finish_capture();
    assert!(
        events.iter().any(|e| e.name == obs::names::VIEW_BUILD),
        "the stale binding rebuilds in place"
    );
    assert_eq!(
        rows_of(&out),
        vec![("new".to_owned(), 20), ("old".to_owned(), 10)],
        "the rebuilt view carries the new fact"
    );
}

/// PRD 03's read-path capture contract (feature `trace`).
#[cfg(feature = "trace")]
#[test]
fn read_path_traces_phases_memo_hits_and_guard() {
    use crate::obs;

    let dir = TempDir::new("prepared-trace-read");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "rent", -1200), (2, 7, "food", -55)]);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let names = |events: &[obs::TraceEvent]| -> Vec<&'static str> {
        events.iter().map(|e| e.name).collect()
    };

    // Prepare: the phase spans, exactly.
    obs::start_capture();
    let mut prepared = prepare(&txn, &cache, &schema, &by_account_query()).expect("prepare");
    let events = obs::finish_capture();
    let got = names(&events);
    for expected in [
        obs::names::VALIDATE,
        obs::names::NORMALIZE,
        obs::names::CLASSIFY,
        obs::names::STATS,
        obs::names::PLAN_DP,
        obs::names::LOWER,
        obs::names::BUILD_COLTS,
        obs::names::PREPARE,
    ] {
        assert!(got.contains(&expected), "missing {expected} in {got:?}");
    }
    // Containment: every phase inside the outer prepare span.
    let outer = events
        .iter()
        .find(|e| e.name == obs::names::PREPARE)
        .expect("outer");
    for e in &events {
        assert!(e.start_ns >= outer.start_ns);
        assert!(e.start_ns + e.dur_ns <= outer.start_ns + outer.dur_ns);
    }

    // First execute: builds views, no memo hits, row count in a0.
    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
        .expect("execute");
    let first = obs::finish_capture();
    assert_eq!(out.len(), 2);
    let first_names = names(&first);
    assert!(
        first_names.contains(&obs::names::VIEW_BUILD),
        "{first_names:?}"
    );
    assert!(!first_names.contains(&obs::names::VIEW_MEMO_HIT));
    let exec = first
        .iter()
        .find(|e| e.name == obs::names::EXECUTE)
        .expect("execute span");
    assert_eq!(exec.a0, 2, "execute a0 carries the row count");

    // Second execute, same snapshot + params: memo hits only.
    obs::start_capture();
    prepared
        .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
        .expect("execute");
    let second = obs::finish_capture();
    let second_names = names(&second);
    assert!(
        second_names.contains(&obs::names::VIEW_MEMO_HIT),
        "{second_names:?}"
    );
    assert!(!second_names.contains(&obs::names::VIEW_BUILD));
    assert!(!second_names.contains(&obs::names::IMAGE_BUILD));

    // A guard-shaped query: guard_probe, never join.
    let guard_query = Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        predicates: vec![],
    };
    let mut guard = prepare(&txn, &cache, &schema, &guard_query).expect("prepare");
    obs::start_capture();
    guard
        .execute_collect(&txn, &cache, &[Value::U64(1)])
        .expect("execute");
    let guard_events = obs::finish_capture();
    let guard_names = names(&guard_events);
    assert!(
        guard_names.contains(&obs::names::GUARD_PROBE),
        "{guard_names:?}"
    );
    assert!(!guard_names.contains(&obs::names::JOIN));
    let probe = guard_events
        .iter()
        .find(|e| e.name == obs::names::GUARD_PROBE)
        .expect("probe");
    assert_eq!(probe.a0, 1, "hit flag");

    // Nothing records without capture.
    prepared
        .execute_collect(&txn, &cache, &[Value::U64(7), Value::I64(-100_000)])
        .expect("execute");
    obs::start_capture();
    assert!(obs::finish_capture().is_empty());
}
