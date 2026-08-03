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

/// Zero-yield cover draws are not batches. An entry whose cover holds an
/// exact multiple of the batch size drains at one full batch plus one
/// empty resume draw (the token must be re-presented to learn the entry
/// is exhausted), and `pump` used to count that empty draw — its
/// `run_node` twin breaks before counting — skewing the
/// batches/batch_entries observable ("batching engaged" means
/// batches ≪ entries) low on exact-fit fanouts.
#[test]
fn zero_yield_draws_are_not_batches() {
    #[derive(Default)]
    struct BatchLens {
        batches: u64,
        entries: u64,
        zero_len: u64,
    }
    impl Counters for BatchLens {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, len: usize) {
            self.batches += 1;
            self.entries += u64::try_from(len).expect("batch fits u64");
            self.zero_len += u64::from(len == 0);
        }
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    let dir = TempDir::new("run-batch-accounting");
    let schema = schema(3);
    // Every fanout is an exact multiple of the batch size 4: the root
    // holds 8 R rows (two full draws), and each middle-node entry's S
    // cover holds exactly 4 children — one full draw plus the empty
    // resume the counter must not book.
    let r: Vec<(u64, u64)> = (0..8u64).map(|i| (i, i % 2)).collect();
    let s: Vec<(u64, u64)> = (0..2u64)
        .flat_map(|y| (0..4u64).map(move |j| (y, y * 4 + j)))
        .collect();
    let t: Vec<(u64, u64)> = (0..8u64).map(|z| (z, z)).collect();
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
    let mut executor = Executor::with_batch_size(&plan, 4);
    assert!(executor.pipe.is_some(), "pipeline dispatched");
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = BatchLens::default();
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");
    assert_eq!(sink.rows.len(), 32, "8 parents x 4 children, T total");
    assert!(counters.batches > 0, "the join drew batches");
    assert_eq!(
        counters.zero_len, 0,
        "an exhausted resume draw is not a batch (batches {}, entries {})",
        counters.batches, counters.entries
    );
}
