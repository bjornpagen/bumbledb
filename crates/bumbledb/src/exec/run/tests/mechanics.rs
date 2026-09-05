use super::*;
use crate::ir::WordCmp;

#[test]
fn dynamic_cover_prefers_the_forced_small_side() {
    let schema = schema(2);

    let r: Vec<(u64, u64)> = (0..500).map(|i| (i % 250, i)).collect();
    let s: Vec<(u64, u64)> = vec![(0, 0), (1, 1)];
    let views = views_of(&schema, &[r, s]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );

    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                estimate: 0,
                subatoms: vec![
                    crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(0)],
                    },
                ],
            },
            crate::plan::fj::Node {
                estimate: 0,
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(1)],
                }],
            },
            crate::plan::fj::Node {
                estimate: 0,
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(1),
                    vars: vec![VarId(2)],
                }],
            },
        ],
    };
    let plan = validate(&plan, &normalized, &schema, &BTreeSet::new()).expect("valid plan");

    let mut colts = colts_for(&plan, &views);
    let s_root = Colt::root();
    colts[1].get(s_root, 0, &[0]);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = RecordingCounters::default();
    Executor::new(&plan)
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");

    let (node, subatom, exact) = counters.cover_choices[0];
    assert_eq!((node, subatom, exact), (0, 1, true));
    assert!(!sink.rows.is_empty());
}

/// Regression for the cover-soundness deviation: a subatom carrying an
/// already-bound variable must never be a runtime-eligible cover.
#[test]
fn covers_never_rebind_an_already_bound_variable() {
    let schema = schema(3);
    let r = vec![(1, 1)];
    let s: Vec<(u64, u64)> = (0..100).map(|z| (1, z)).collect();
    let t = vec![(2, 5)];
    let views = views_of(&schema, &[r, s, t]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            occurrence(2, 2, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1, 2]);

    // The mixed-var subatom T(x, z) must not be listed as a cover of

    for node in plan.nodes() {
        for &cover in &node.covers {
            let vars = &node.subatoms[cover as usize].vars;
            assert_eq!(
                vars.len(),
                node.new_vars.len(),
                "a cover must bind exactly the node's new vars"
            );
        }
    }

    let results = run(&plan, &views);
    assert!(
        results.is_empty(),
        "T binds x=2, R binds x=1: joining them must be empty, got {results:?}"
    );
}

#[test]
fn backtracking_restores_sources_across_sequential_executions() {
    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..20).map(|i| (i % 4, i)).collect();
    let s: Vec<(u64, u64)> = (0..4).map(|i| (i, i * 10)).collect();
    let views = views_of(&schema, &[r, s]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut executor = Executor::new(&plan);

    let mut first = CollectSink::default();
    executor
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut first,
            &mut NoopCounters,
        )
        .expect("execute");
    let mut second = CollectSink::default();
    executor
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut second,
            &mut NoopCounters,
        )
        .expect("execute");
    assert_eq!(first.rows, second.rows);
    assert!(!first.rows.is_empty());
}

#[test]
fn results_are_identical_across_batch_sizes() {
    let schema = schema(3);
    let r: Vec<(u64, u64)> = (0..150).map(|i| (i % 7, i % 11)).collect();
    let s: Vec<(u64, u64)> = (0..90).map(|i| (i % 11, i % 5)).collect();
    let t: Vec<(u64, u64)> = (0..40).map(|i| (i % 5, i)).collect();
    let views = views_of(&schema, &[r, s, t]);
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
    let plan = planned(&normalized, &schema, &[0, 1, 2]);
    let reference = run_batched(&plan, &views, 1);
    assert!(!reference.is_empty());
    for batch in [2usize, 64, 128, 1024] {
        assert_eq!(
            run_batched(&plan, &views, batch),
            reference,
            "batch size {batch} must match the scalar degenerate case"
        );
    }

    let views = views_of(&schema, &[vec![(1, 2)], vec![], vec![(0, 0)]]);
    for batch in [1usize, 2, 64, 128, 256, 1024] {
        assert!(run_batched(&plan, &views, batch).is_empty());
    }
}

#[test]
fn phase_one_hashes_the_whole_batch_before_any_phase_two_probe() {
    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i)).collect();
    let s: Vec<(u64, u64)> = (0..10).map(|i| (i, i * 2)).collect();
    let views = views_of(&schema, &[r, s]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = PhaseOrderCounters::default();
    Executor::with_batch_size(&plan, 128)
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");

    let first_probe = counters
        .events
        .iter()
        .position(|(kind, node, _)| *kind == "probe" && *node == 0)
        .expect("probes happened");
    let hashes_before = counters.events[..first_probe]
        .iter()
        .filter(|(kind, node, _)| *kind == "hash" && *node == 0)
        .count();
    assert_eq!(
        hashes_before, 10,
        "the entire batch is hashed before the first bucket load"
    );
    assert!(!sink.rows.is_empty());
}

#[test]
fn pinned_siblings_probe_without_hashing() {
    let schema = schema(3);

    // so both pin to Cursor::Row after node 0. At node 1 both B(c)

    let a_rows: Vec<(u64, u64)> = vec![(1, 10), (2, 20)];
    let b_rows: Vec<(u64, u64)> = vec![(1, 100), (2, 200)];
    let c_rows: Vec<(u64, u64)> = vec![(10, 100), (20, 200)];
    let views = views_of(&schema, &[a_rows, b_rows, c_rows]);
    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
            occurrence(2, 2, &[(0, 1), (1, 2)]),
        ],
        vec![],
    );

    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                estimate: 0,
                subatoms: vec![
                    crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(0), VarId(1)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(0)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(2),
                        vars: vec![VarId(1)],
                    },
                ],
            },
            crate::plan::fj::Node {
                estimate: 0,
                subatoms: vec![
                    crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(2)],
                    },
                    crate::plan::fj::Subatom {
                        occ: OccId(2),
                        vars: vec![VarId(2)],
                    },
                ],
            },
        ],
    };
    let plan = validate(&plan, &normalized, &schema, &BTreeSet::new()).expect("valid plan");
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = PhaseOrderCounters::default();
    Executor::new(&plan)
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");

    let count = |kind: &str, node: usize, subatom: usize| {
        counters
            .events
            .iter()
            .filter(|(k, n, s)| *k == kind && *n == node && *s == subatom)
            .count()
    };

    assert!(count("hash", 0, 1) > 0, "B's root probe hashes");
    assert!(count("hash", 0, 2) > 0, "C's root probe hashes");

    assert_eq!(count("hash", 1, 1), 0, "pinned probes compute no hash");
    assert_eq!(count("probe", 1, 1), 2, "both entries still probe C");

    assert_eq!(
        sink.rows,
        BTreeSet::from([vec![1, 10, 100], vec![2, 20, 200]])
    );
}

#[test]
fn cover_choice_is_magnitude_first() {
    use KeyCount::{Estimate, Exact};

    assert!(better_cover(Estimate(7), Exact(500)));
    assert!(!better_cover(Exact(500), Estimate(7)));

    assert!(better_cover(Exact(7), Estimate(500)));
    assert!(!better_cover(Estimate(500), Exact(7)));

    assert!(better_cover(Exact(9), Estimate(9)));
    assert!(!better_cover(Estimate(9), Exact(9)));
    assert!(!better_cover(Exact(9), Exact(9)));
    assert!(!better_cover(Estimate(9), Estimate(9)));
}

/// The cost-class ordering: a node's ALU residuals compact the survivor set
/// BEFORE its sibling hash probes, so every residual-killed element is a bucket
/// load never issued.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "both twins pinned in one scenario — clearer kept together"
)]
fn residuals_compact_survivors_before_the_sibling_probes() {
    #[derive(Default)]
    struct Order {
        events: Vec<(&'static str, usize)>,
    }
    impl Counters for Order {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: crate::exec::colt::KeyCount) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, node: usize, _: usize, _: bool) {
            self.events.push(("probe", node));
        }
        fn residual(&mut self, node: usize, _: bool) {
            self.events.push(("residual", node));
        }
        fn anti_probe(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..10)
        .map(|i| if i < 5 { (i, i) } else { (i, i + 1) })
        .collect();
    let s: Vec<(u64, u64)> = (0..10).map(|i| (i, i * 2)).collect();
    let views = views_of(&schema, &[r, s]);
    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(VarId(0)),
            right: OperandAddr::from(VarId(1)),
            op: WordCmp::Ne,
        }],
    );
    let plan = planned(&query, &schema, &[0, 1]);
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = Order::default();
    let mut executor = Executor::with_batch_size(&plan, 128);
    assert!(
        matches!(executor.drive, super::super::Drive::Pipeline(_)),
        "two nodes pipeline"
    );
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");
    assert_eq!(sink.rows.len(), 5, "the Ne survivors join");
    let node0: Vec<&'static str> = counters
        .events
        .iter()
        .filter(|(_, node)| *node == 0)
        .map(|(kind, _)| *kind)
        .collect();
    assert_eq!(
        node0,
        ["residual"; 10]
            .iter()
            .chain(["probe"; 5].iter())
            .copied()
            .collect::<Vec<_>>(),
        "10 residuals compact to 5 before the first bucket load"
    );

    let r2: Vec<(u64, u64)> = (0..10)
        .map(|i| if i < 5 { (i, i) } else { (i, i + 1) })
        .collect();
    let s2: Vec<(u64, u64)> = r2.clone();
    let views = views_of(&schema, &[r2, s2]);
    let query = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 1)]),
        ],
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(VarId(0)),
            right: OperandAddr::from(VarId(1)),
            op: WordCmp::Ne,
        }],
    );
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![crate::plan::fj::Node {
            estimate: 0,
            subatoms: vec![
                crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(0), VarId(1)],
                },
                crate::plan::fj::Subatom {
                    occ: OccId(1),
                    vars: vec![VarId(0), VarId(1)],
                },
            ],
        }],
    };
    let plan = validate(&plan, &query, &schema, &BTreeSet::new()).expect("valid plan");
    let mut colts = colts_for(&plan, &views);
    let mut bindings = Bindings::new(plan.slot_count());
    let mut sink = CollectSink::default();
    let mut counters = Order::default();
    let mut executor = Executor::with_batch_size(&plan, 128);
    assert!(
        matches!(executor.drive, super::super::Drive::Leaf),
        "one factored node runs the leaf pass"
    );
    executor
        .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
        .expect("execute");
    assert_eq!(sink.rows.len(), 5);
    let node0: Vec<&'static str> = counters
        .events
        .iter()
        .filter(|(_, node)| *node == 0)
        .map(|(kind, _)| *kind)
        .collect();
    assert_eq!(
        node0,
        ["residual"; 10]
            .iter()
            .chain(["probe"; 5].iter())
            .copied()
            .collect::<Vec<_>>(),
        "the leaf twin keeps the same order"
    );
}
