use super::*;

/// The pipelined executor matches the nested-loop oracle bit for bit
/// across both all-variable and projected shapes, across batch
/// sizes that stress fill boundaries (pending exactly at, one under,
/// and far over the batch), multi-batch expansions with resume
/// tokens, empty covers, and duplicate-heavy skew.
#[test]
fn pipelined_executor_matches_oracle() {
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

/// Counter-proven batching: a triangle-shaped plan
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

/// The zero-binding nonemptiness gate (a participating atom with no
/// variables) asks one question — "does the relation hold any fact?" —
/// yet the executor used to ENUMERATE the gate relation once per
/// pending entry: every position yields the same empty key row, so a
/// gate of |G| rows multiplied the join's work by |G| for zero
/// distinguishable bindings. Under a projection sink D2's first-emit
/// skip hid it; under an aggregate sink (never skips, gate forces the
/// dedup seen-set) it was the S-scale crucible hang — verify random
/// case 19, `MIN ... GROUP BY` over a star join × a bare `PostingTag`
/// atom, |join| × |`PostingTag`| ≈ 10¹⁰ folds. The collapse: a
/// zero-arity cover yields at most one entry — in `pump` (middle node)
/// and `run_node` (leaf) both — and an empty gate still kills every
/// binding.
#[test]
fn zero_binding_gate_yields_one_entry_not_the_relation() {
    #[derive(Default)]
    struct EmitCount {
        emits: u64,
    }
    impl Counters for EmitCount {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {
            self.emits += 1;
        }
        fn skip(&mut self, _: usize) {}
    }

    let schema = schema(3);
    // R0 ⋈ R2 on y: 300 answers; the gate holds 500 facts the
    // executor must never enumerate (emits stay at the join's own 300).
    let r: Vec<(u64, u64)> = (0..300u64).map(|i| (i, i % 7)).collect();
    let t: Vec<(u64, u64)> = (0..7u64).map(|i| (i, i + 100)).collect();
    let gate: Vec<(u64, u64)> = (0..500u64).map(|i| (i, i)).collect();
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[]), // the gate: no bindings at all
            occurrence(2, 2, &[(0, 1), (1, 2)]),
        ],
        vec![],
    );
    let sinks = all_vars(&normalized);
    let expected: BTreeSet<Vec<u64>> = r
        .iter()
        .map(|(x, y)| {
            let (_, z) = t.iter().find(|(ty, _)| ty == y).expect("dense join key");
            vec![*x, *y, *z]
        })
        .collect();
    // Both placements: the gate as a middle pipeline node (`pump`'s
    // collapse) and as the leaf (`run_node`'s collapse).
    for order in [[0u16, 1, 2], [0u16, 2, 1]] {
        let plan = planned_with_sinks(&normalized, &schema, &order, &sinks);
        for present in [true, false] {
            let gate_rows = if present { gate.clone() } else { Vec::new() };
            let dir = TempDir::new(&format!("run-gate-{}-{}", order[2], usize::from(present)));
            let views = views_of(&dir, &schema, &[r.clone(), gate_rows, t.clone()]);
            let mut executor = Executor::new(&plan);
            assert!(executor.pipe.is_some(), "three nodes pipeline");
            let mut colts = colts_for(&plan, &views);
            let mut bindings = Bindings::new(plan.slot_count());
            let mut sink = CollectSink::default();
            let mut counters = EmitCount::default();
            executor
                .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
                .expect("execute");
            let got: BTreeSet<Vec<u64>> = sink
                .rows
                .iter()
                .map(|row| {
                    (0..3u16)
                        .map(|v| row[plan.slot_of(VarId(v))])
                        .collect::<Vec<u64>>()
                })
                .collect();
            if present {
                assert_eq!(got, expected, "order {order:?}");
                assert_eq!(
                    counters.emits,
                    expected.len() as u64,
                    "order {order:?}: the gate is never enumerated"
                );
            } else {
                assert!(
                    got.is_empty(),
                    "order {order:?}: an empty gate kills the rule"
                );
                assert_eq!(counters.emits, 0, "order {order:?}");
            }
        }
    }
}
