use super::*;

#[test]
fn selection_params_rotate_differentially() {
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
    let fix = postings(&borrowed);
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");
    for cycle in 0..3 {
        for m in 0..8 {
            let memo = format!("m{m}");
            let out = fix
                .execute(&mut prepared, &memo_param(&memo))
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

    let out = fix
        .execute(&mut prepared, &memo_param("never-stored"))
        .expect("execute");
    assert!(out.is_empty());
}

#[test]
fn selection_work_is_o_selected() {
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
    let fix = postings(&borrowed);
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[ParamArg::Scalar(BindValue::Str("hot"))])
        .expect("execute");
    assert_eq!(out.len(), 4);
}

#[cfg(feature = "trace")]
#[test]
fn selection_params_rotate_without_view_rebuilds() {
    use crate::obs;

    // Epoch memoization is a store behavior: heap ticks rebuild per
    // execution by design, so this suite runs over one committed store.
    let fix = posting_store(
        "prepared-selection-memo",
        &[
            (1, 0, "m0", 10),
            (2, 0, "m1", 20),
            (3, 0, "m2", 30),
            (4, 0, "m0", 40),
        ],
    );
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");

    let mut view_builds = 0;
    let mut memo_hits = 0;
    for _cycle in 0..3 {
        for m in ["m0", "m1", "m2"] {
            obs::start_capture();
            let out = fix.execute(&mut prepared, &memo_param(m)).expect("execute");
            let events = obs::finish_capture();
            assert!(!out.is_empty());
            view_builds += events
                .iter()
                .filter(|e| e.point() == obs::names::VIEW_BUILD)
                .count();
            memo_hits += events
                .iter()
                .filter(|e| e.point() == obs::names::VIEW_MEMO_HIT)
                .count();
            let probe = events
                .iter()
                .find(|e| e.point() == obs::names::SELECT_PROBE)
                .expect("every execution probes");
            assert_eq!(probe.a1(), 1, "present keys hit");
        }
    }
    assert_eq!(view_builds, 1, "one view build per generation");
    assert_eq!(memo_hits, 8, "every later execution memo-hits");

    // A never-stored text now binds to a fresh interner token (interning
    // never misses); the selection probe runs, misses, and the join never
    // starts — the empty verdict is a probe miss, not a bind short-circuit.
    obs::start_capture();
    let out = fix
        .execute(&mut prepared, &memo_param("never-stored"))
        .expect("execute");
    let events = obs::finish_capture();
    assert!(out.is_empty());
    let names: Vec<obs::TracePoint> = events.iter().map(|e| e.point()).collect();
    assert!(!names.contains(&obs::names::VIEW_BUILD), "{names:?}");
    let probe = events
        .iter()
        .find(|e| e.point() == obs::names::SELECT_PROBE)
        .expect("the probe runs against the fresh token");
    assert_eq!(probe.a1(), 0, "absent keys miss");
    assert!(!names.contains(&obs::names::JOIN), "{names:?}");
}
