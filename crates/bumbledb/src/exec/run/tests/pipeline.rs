use super::*;
use crate::ir::WordCmp;

#[test]
fn pipelined_executor_matches_oracle() {
    let _dir = TempDir::new("run-pipeline-equiv");
    let schema = schema(3);

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
            vec![FilterPredicate::FieldsCompare {
                left: OperandAddr::from(VarId(0)),
                right: OperandAddr::from(VarId(3)),
                op: WordCmp::Ne,
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
            assert!(
                matches!(executor.drive, super::super::Drive::Pipeline(_)),
                "pipeline dispatched"
            );
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
        fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
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
    assert!(matches!(executor.drive, super::super::Drive::Pipeline(_)));
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

    for scratch in &executor.scratch {
        assert!(
            scratch.pending_bindings.capacity()
                <= 2 * BATCH * plan.slot_count() + plan.slot_count()
        );
    }
}

#[test]
fn zero_binding_gate_yields_one_entry_not_the_relation() {
    #[derive(Default)]
    struct EmitCount {
        emits: u64,
    }
    impl Counters for EmitCount {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
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

    // executor must never enumerate (emits stay at the join's own 300).
    let r: Vec<(u64, u64)> = (0..300u64).map(|i| (i, i % 7)).collect();
    let t: Vec<(u64, u64)> = (0..7u64).map(|i| (i, i + 100)).collect();
    let gate: Vec<(u64, u64)> = (0..500u64).map(|i| (i, i)).collect();
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[]), 
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

    for order in [[0u16, 1, 2], [0u16, 2, 1]] {
        let plan = planned_with_sinks(&normalized, &schema, &order, &sinks);
        for present in [true, false] {
            let gate_rows = if present { gate.clone() } else { Vec::new() };
            let dir = TempDir::new(&format!("run-gate-{}-{}", order[2], usize::from(present)));
            let views = views_of(&dir, &schema, &[r.clone(), gate_rows, t.clone()]);
            let mut executor = Executor::new(&plan);
            assert!(
                matches!(executor.drive, super::super::Drive::Pipeline(_)),
                "three nodes pipeline"
            );
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

#[test]
fn deep_nodes_accumulate_full_batches_across_pump_returns() {

    #[derive(Default)]
    struct MaxPass {
        current: usize,
        max: usize,
    }
    impl Counters for MaxPass {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, node: usize, _: usize, _: bool) {
            if node == 2 {
                self.current += 1;
            }
        }
        fn residual(&mut self, _: usize, _: bool) {}
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
        fn phase_start(&mut self, node: usize, phase: JoinPhase) {
            if node == 2 && phase == JoinPhase::Probe {
                self.current = 0;
            }
        }
        fn phase_end(&mut self, node: usize, phase: JoinPhase) {
            if node == 2 && phase == JoinPhase::Probe {
                self.max = self.max.max(self.current);
            }
        }
    }

    let dir = TempDir::new("run-deep-accumulation");
    let schema = schema(4);

    let r0: Vec<(u64, u64)> = (0..2048).map(|i| (i, i)).collect();
    let r1: Vec<(u64, u64)> = (0..2048).filter(|i| i % 2 == 0).map(|i| (i, i)).collect();
    let r2: Vec<(u64, u64)> = (0..2048).filter(|i| i % 4 == 0).map(|i| (i, i)).collect();
    let r3: Vec<(u64, u64)> = (0..2048).map(|i| (i, i % 5)).collect();
    let views = views_of(&dir, &schema, &[r0, r1, r2, r3]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 2), (1, 3)]),
            occurrence(3, 3, &[(0, 3), (1, 4)]),
        ],
        vec![],
    );
    let sinks = all_vars(&normalized);
    let plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2, 3], &sinks);
    assert_eq!(plan.nodes().len(), 4, "one node per occurrence");
    let expected: BTreeSet<Vec<u64>> = (0..2048u64)
        .filter(|i| i % 4 == 0)
        .map(|i| vec![i, i, i, i, i % 5])
        .collect();
    let mut executor = Executor::new(&plan);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = MaxPass::default();
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");
    let got: BTreeSet<Vec<u64>> = sink
        .rows
        .iter()
        .map(|row| {
            (0..5u16)
                .map(|v| row[plan.slot_of(VarId(v))])
                .collect::<Vec<u64>>()
        })
        .collect();
    assert_eq!(got, expected, "the 4-chain joins correctly");
    assert_eq!(
        counters.max, BATCH,
        "node 2 accumulated a full cross-pump batch (premature drains cap it at 64)"
    );
}
