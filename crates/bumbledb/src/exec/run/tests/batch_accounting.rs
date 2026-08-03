use super::*;

/// The pipeline's gather/assembly work is phase-attributed (Gap B):
/// under `PhaseTimers`, `pump`'s cover iteration and `probe_pass`'s
/// batch assembly land in `jp_gather` windows instead of vanishing
/// between the timed phases — a deep plan's formerly unattributed
/// half. Zero-cost off stands untouched: the window calls are
/// `Counters` defaults, compiled to nothing under `NoopCounters`.
#[cfg(feature = "trace")]
#[test]
fn pump_gather_windows_are_attributed() {
    let dir = TempDir::new("run-gather-phase");
    let schema = schema(3);
    let r: Vec<(u64, u64)> = (0..64u64).map(|i| (i, i % 8)).collect();
    let s: Vec<(u64, u64)> = (0..8u64)
        .flat_map(|y| (0..8u64).map(move |j| (y, y * 8 + j)))
        .collect();
    let t: Vec<(u64, u64)> = (0..64u64).map(|z| (z, z)).collect();
    let views = views_of(&dir, &schema, &[r, s, t]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
        ],
        vec![],
    );
    let sinks = all_vars(&normalized);
    let plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
    let mut executor = Executor::new(&plan);
    assert!(executor.pipe.is_some(), "pipeline dispatched");
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    crate::obs::start_capture();
    let mut timers = PhaseTimers::new();
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut timers)
        .expect("execute");
    timers.flush();
    let events = crate::obs::finish_capture();
    assert!(!sink.rows.is_empty());
    // Both pumped nodes attribute gather windows: the virtual root's
    // pass (node 0) and the middle node's (node 1).
    for name in ["jp_gather_n0", "jp_gather_n1"] {
        let event = events
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("{name} attributed"));
        assert!(event.a1 > 0, "{name} counts its windows");
    }
}
