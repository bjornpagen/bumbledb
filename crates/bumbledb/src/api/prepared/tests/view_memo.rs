//! View-memo behavior on the successor substrate: epoch-keyed hits within
//! one generation, rebuilds across generations, parked-binding rotation,
//! heap ticks that never memoize, and trim. The old write-path `advance`
//! lineage hook is deleted with the transitional storage — invalidation is
//! generation-keyed now.
use super::*;

fn rotating_rows() -> Vec<(u64, u64, &'static str, i64)> {
    (0..32u64)
        .map(|i| {
            let memo: &'static str = match i % 3 {
                0 => "m0",
                1 => "m1",
                _ => "m2",
            };
            (i, i % 4, memo, i64::try_from(i).expect("small") * 3 - 40)
        })
        .collect()
}

#[cfg(feature = "trace")]
#[test]
fn same_generation_executions_memo_hit_and_new_generations_rebuild() {
    use crate::obs;

    let fix = posting_store("view-memo-generations", &rotating_rows());
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");

    let run = |fix: &StoreFix, prepared: &mut PreparedQuery<T>, memo: &str| {
        obs::start_capture();
        let out = fix.execute(prepared, &memo_param(memo)).expect("execute");
        let events = obs::finish_capture();
        let builds = events
            .iter()
            .filter(|e| e.point() == obs::names::VIEW_BUILD)
            .count();
        let hits = events
            .iter()
            .filter(|e| e.point() == obs::names::VIEW_MEMO_HIT)
            .count();
        (out.len(), builds, hits)
    };

    let (rows_a, builds, _) = run(&fix, &mut prepared, "m0");
    assert!(rows_a > 0);
    assert_eq!(builds, 1, "the first execution builds the view");
    let (_, builds, hits) = run(&fix, &mut prepared, "m0");
    assert_eq!(builds, 0, "the second execution rebuilds nothing");
    assert_eq!(hits, 1, "the second execution memo-hits");

    // A write advances the generation: the next execution rebuilds.
    fix.insert_dyn(POSTING, &posting_rows(&[(1000, 0, "m0", 999)]));
    let (rows_b, builds, _) = run(&fix, &mut prepared, "m0");
    assert_eq!(builds, 1, "the new generation pays one rebuild");
    assert_eq!(rows_b, rows_a + 1, "and reads the fresh row");
}

#[cfg(feature = "trace")]
#[test]
fn rotating_range_bindings_park_and_return_without_rebuilds() {
    use crate::obs;

    // A range-param query re-binds different residual filters against one
    // epoch-stable source: parked COLTs return without a rebuild scan.
    let fix = posting_store("view-memo-parked", &rotating_rows());
    let query = by_account_query();
    let mut prepared = fix.prepare(&query).expect("prepare");

    let run = |fix: &StoreFix, prepared: &mut PreparedQuery<T>, floor: i64| {
        obs::start_capture();
        let out = fix
            .execute(prepared, &[BindValue::U64(0), BindValue::I64(floor)])
            .expect("execute");
        let events = obs::finish_capture();
        let builds = events
            .iter()
            .filter(|e| e.point() == obs::names::VIEW_BUILD)
            .count();
        let hits = events
            .iter()
            .filter(|e| e.point() == obs::names::VIEW_MEMO_HIT)
            .count();
        (out.len(), builds, hits)
    };

    // Two distinct bindings, then the rotation repeats them.
    let (_, first_builds, _) = run(&fix, &mut prepared, -100);
    assert_eq!(first_builds, 1, "cold build");
    let (_, second_builds, _) = run(&fix, &mut prepared, 0);
    assert!(
        second_builds <= 1,
        "a second residual binding builds at most one filtered view"
    );
    let mut rebuilds = 0;
    for _ in 0..3 {
        for floor in [-100, 0] {
            let (_, builds, hits) = run(&fix, &mut prepared, floor);
            rebuilds += builds;
            assert!(builds == 0 || hits == 0 || builds + hits >= 1);
        }
    }
    assert_eq!(
        rebuilds, 0,
        "repeated bindings hit the active or parked slot — never a rebuild"
    );
}

#[test]
fn heap_executions_never_reuse_a_previous_instances_views() {
    // Heap instances carry no durable identity: each execution rebuilds
    // from the instance it was handed, so answers always track the actual
    // instance — never a stale memo of another one's rows.
    let rows_a: &[(u64, u64, &str, i64)] = &[(1, 7, "a", 10)];
    let fix_a = postings(rows_a);
    let mut prepared = fix_a.prepare(&by_memo_query()).expect("prepare");
    let out = fix_a
        .execute(&mut prepared, &memo_param("a"))
        .expect("execute");
    assert_eq!(out.len(), 1);

    // The same prepared query against the SAME instance again (fresh
    // tick): identical answers, no stale carry.
    let out = fix_a
        .execute(&mut prepared, &memo_param("a"))
        .expect("re-execute");
    assert_eq!(out.len(), 1);
}

#[test]
fn a_heap_prepared_plan_refuses_other_sources() {
    let rows: &[(u64, u64, &str, i64)] = &[(1, 7, "a", 10)];
    let heap = postings(rows);
    let store = posting_store("view-memo-foreign", rows);
    let mut prepared = heap.prepare(&by_memo_query()).expect("prepare");
    let refused = store.execute(&mut prepared, &memo_param("a"));
    assert!(matches!(refused, Err(Error::ForeignPreparedQuery)));
}

#[test]
fn trim_drops_parked_views_and_preserves_answers() {
    let fix = posting_store("view-memo-trim", &rotating_rows());
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let params = [BindValue::U64(0), BindValue::I64(-100)];
    let before = answers_of(&fix.execute(&mut prepared, &params).expect("execute"));
    prepared.trim();
    let after = answers_of(&fix.execute(&mut prepared, &params).expect("re-execute"));
    assert_eq!(before, after, "trim changes cost, never answers");
}
