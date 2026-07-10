use super::*;

/// The pipelined executor — dispatched exactly
/// for skip-free plans with middle nodes — matches the recursive
/// executor and the nested-loop oracle bit for bit, across batch
/// sizes that stress fill boundaries (pending exactly at, one under,
/// and far over the batch), multi-batch expansions with resume
/// tokens, empty covers, and duplicate-heavy skew.
#[test]
fn pipelined_executor_matches_recursive_and_oracle() {
    let _dir = TempDir::new("run-pipeline-equiv");
    let schema = schema(3);
    // Chain shape with heavy fanout at every step; sizes cross the
    // 128 batch on both sides.
    for (n_r, n_s, n_t) in [(127u64, 128, 129), (5, 300, 40), (1, 1, 1), (200, 0, 10)] {
        let r: Vec<(u64, u64)> = (0..n_r).map(|i| (i % 13, i % 7)).collect();
        let s: Vec<(u64, u64)> = (0..n_s).map(|i| (i % 7, i % 11)).collect();
        let t: Vec<(u64, u64)> = (0..n_t).map(|i| (i % 11, i)).collect();
        let mut r = r;
        r.sort_unstable();
        r.dedup();
        let mut s = s;
        s.sort_unstable();
        s.dedup();
        let mut t = t;
        t.sort_unstable();
        t.dedup();
        let dir2 = TempDir::new(&format!("run-pipeline-{n_r}-{n_s}-{n_t}"));
        let views = views_of(&dir2, &schema, &[r.clone(), s.clone(), t.clone()]);
        let normalized = normalized(
            vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
            vec![PlacedComparison {
                op: CmpOp::Ne,
                lhs: VarId(0),
                rhs: VarId(3),
            }],
        );
        let sinks = all_vars(&normalized);
        let pipe_plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
        assert!(pipe_plan.skip_free(), "all-vars projections are skip-free");
        let rec_plan = planned(&normalized, &schema, &[0, 1, 2]);
        assert!(!rec_plan.skip_free());

        let mut expected = BTreeSet::new();
        for (rx, ry) in &r {
            for (sy, sz) in &s {
                for (tz, tw) in &t {
                    if ry == sy && sz == tz && rx != tw {
                        expected.insert(vec![*rx, *ry, *sz, *tw]);
                    }
                }
            }
        }
        for batch in [1usize, 2, 127, 128, 129, 1024] {
            let mut executor = Executor::with_batch_size(&pipe_plan, batch);
            assert!(executor.pipe.is_some(), "pipeline dispatched");
            let mut colts = colts_for(&pipe_plan, &views);
            let mut bindings = Bindings::new(pipe_plan.slot_count());
            let mut sink = CollectSink::default();
            executor
                .execute(
                    &pipe_plan,
                    &mut colts,
                    &mut bindings,
                    &mut sink,
                    &mut NoopCounters,
                )
                .expect("execute");
            let got: BTreeSet<Vec<u64>> = sink
                .rows
                .iter()
                .map(|row| {
                    (0..4u16)
                        .map(|v| row[pipe_plan.slot_of(VarId(v))])
                        .collect::<Vec<u64>>()
                })
                .collect();
            assert_eq!(got, expected, "sizes ({n_r},{n_s},{n_t}) batch {batch}");
        }
    }
}

/// Counter-proven batching: a triangle-shaped skip-free plan
/// whose middle node used to probe once per parent now probes in
/// cross-parent batches with mean length well above the gate.
#[test]
fn pipelined_middle_nodes_probe_in_cross_parent_batches() {
    #[derive(Default)]
    struct ProbeBatches {
        passes: usize,
        probes: usize,
        current: usize,
        node: usize,
    }
    impl Counters for ProbeBatches {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, node: usize, _: usize, _: bool) {
            if node == self.node {
                self.probes += 1;
                self.current += 1;
            }
        }
        fn residual(&mut self, _: usize, _: bool) {}
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
        fn phase_start(&mut self, node: usize, phase: JoinPhase) {
            if node == self.node && phase == JoinPhase::Probe {
                self.current = 0;
            }
        }
        fn phase_end(&mut self, node: usize, phase: JoinPhase) {
            if node == self.node && phase == JoinPhase::Probe && self.current > 0 {
                self.passes += 1;
            }
        }
    }

    let dir = TempDir::new("run-pipeline-batching");
    let schema = schema(3);
    // R fans out 1000 parents; the middle node probes S per parent —
    // fanout 1 each — the exact starvation shape.
    let r: Vec<(u64, u64)> = (0..1000).map(|i| (i % 4, i)).collect();
    let s: Vec<(u64, u64)> = (0..1000).map(|i| (i, i % 5)).collect();
    let t: Vec<(u64, u64)> = (0..5).map(|i| (i, i)).collect();
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
    assert!(plan.skip_free());
    let mut executor = Executor::new(&plan);
    assert!(executor.pipe.is_some());
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = ProbeBatches {
        node: 1,
        ..Default::default()
    };
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");
    assert!(!sink.rows.is_empty());
    assert!(counters.passes > 0);
    let mean = counters.probes / counters.passes;
    assert!(
        mean >= 32,
        "middle-node probes batch across parents: mean {mean} (probes {}, passes {})",
        counters.probes,
        counters.passes
    );

    // The memory bound: pending buffers never exceed two batches.
    for scratch in &executor.scratch {
        assert!(
            scratch.pending_bindings.capacity()
                <= 2 * BATCH * plan.slot_count() + plan.slot_count()
        );
    }
}
