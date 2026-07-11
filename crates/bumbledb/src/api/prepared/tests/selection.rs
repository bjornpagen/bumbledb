use super::*;

/// The differential pin for the selection cutover (docs/architecture/40-execution.md):
/// rotating Eq params across many executions, every result compared
/// against a nested-loop filter over the inserted rows.
#[test]
fn selection_params_rotate_differentially() {
    let dir = TempDir::new("prepared-select-diff");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // Seeded rows over 8 memo values, amounts distinct per row.
    let mut state = 0xDEAD_BEEF_u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 33
    };
    let rows: Vec<(u64, u64, String, i64)> = (0..200)
        .map(|id| {
            let memo = format!("m{}", next() % 8);
            let amount = i64::try_from(id).expect("fits") * 3 - 100;
            (id, next() % 5, memo, amount)
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    insert_postings(&env, &schema, &borrowed);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_memo_query()).expect("prepare");
    for cycle in 0..3 {
        for m in 0..8 {
            let memo = format!("m{m}");
            let out = prepared
                .execute_collect(&txn, &cache, &memo_param(&memo))
                .expect("execute");
            let mut expected: Vec<i64> = rows
                .iter()
                .filter(|(_, _, row_memo, _)| *row_memo == memo)
                .map(|(_, _, _, amount)| *amount)
                .collect();
            expected.sort_unstable();
            expected.dedup();
            assert_eq!(
                amounts_of(&out),
                expected,
                "cycle {cycle}, memo {memo} diverges from the nested loop"
            );
        }
    }
    // The never-interned miss stays the empty set.
    let out = prepared
        .execute_collect(&txn, &cache, &memo_param("never-stored"))
        .expect("execute");
    assert!(out.is_empty());
}

/// Counters pin (docs/architecture/40-execution.md): a selection's work is O(selected),
/// never O(relation).
#[test]
fn selection_work_is_o_selected() {
    let dir = TempDir::new("prepared-select-counters");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    // 20 rows, exactly 4 carrying the hot memo, distinct amounts.
    let rows: Vec<(u64, u64, String, i64)> = (0..20)
        .map(|id| {
            let memo = if id % 5 == 0 {
                "hot".to_owned()
            } else {
                format!("cold-{id}")
            };
            (id, id % 3, memo, i64::try_from(id).expect("fits") * 7)
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    insert_postings(&env, &schema, &borrowed);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_memo_query()).expect("prepare");
    let (out, stats) = prepared
        .profile(&txn, &cache, &memo_param("hot"))
        .expect("profile");
    assert_eq!(out.len(), 4);
    let drawn: u64 = stats.rules[0].nodes.iter().map(|n| n.batch_entries).sum();
    assert_eq!(drawn, 4, "work is O(selected), not O(relation): {stats:?}");
}

/// The scan is dead (docs/architecture/40-execution.md): rotating Eq params build the view
/// once per generation; every later execution memo-hits and probes.
#[cfg(feature = "trace")]
#[test]
fn selection_params_rotate_without_view_rebuilds() {
    use crate::obs;

    let dir = TempDir::new("prepared-select-trace");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(
        &env,
        &schema,
        &[
            (1, 0, "m0", 10),
            (2, 0, "m1", 20),
            (3, 0, "m2", 30),
            (4, 0, "m0", 40),
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &by_memo_query()).expect("prepare");

    let mut view_builds = 0;
    let mut memo_hits = 0;
    for _cycle in 0..3 {
        for m in ["m0", "m1", "m2"] {
            obs::start_capture();
            let out = prepared
                .execute_collect(&txn, &cache, &memo_param(m))
                .expect("execute");
            let events = obs::finish_capture();
            assert!(!out.is_empty());
            view_builds += events
                .iter()
                .filter(|e| e.name == obs::names::VIEW_BUILD)
                .count();
            memo_hits += events
                .iter()
                .filter(|e| e.name == obs::names::VIEW_MEMO_HIT)
                .count();
            let probe = events
                .iter()
                .find(|e| e.name == obs::names::SELECT_PROBE)
                .expect("every execution probes");
            assert_eq!(probe.a1, 1, "present keys hit");
        }
    }
    assert_eq!(view_builds, 1, "one view build per generation");
    assert_eq!(memo_hits, 8, "every later execution memo-hits");

    // A never-interned param short-circuits at resolve: no view work,
    // no probe, no join — the empty set.
    obs::start_capture();
    let out = prepared
        .execute_collect(&txn, &cache, &memo_param("never-stored"))
        .expect("execute");
    let events = obs::finish_capture();
    assert!(out.is_empty());
    let names: Vec<&str> = events.iter().map(|e| e.name).collect();
    assert!(!names.contains(&obs::names::VIEW_BUILD), "{names:?}");
    assert!(!names.contains(&obs::names::SELECT_PROBE), "{names:?}");
    assert!(!names.contains(&obs::names::JOIN), "{names:?}");
}
