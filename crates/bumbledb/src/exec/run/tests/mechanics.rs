use super::*;

#[test]
fn scalar_set_traversal_deduplicates_middle_terminals_and_never_scans_raw_leaves() {
    #[derive(Default)]
    struct RawRows(Vec<Vec<u64>>);
    impl Sink for RawRows {
        fn may_use_distinct_traversal() -> bool {
            true
        }

        fn emit(&mut self, bindings: &Bindings) -> Flow {
            self.0.push(
                (0..bindings.slot_count())
                    .map(|slot| bindings.get(slot))
                    .collect(),
            );
            Flow::Continue
        }
        fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow {
            for &entry in batch.survivors {
                self.0.push(
                    (0..batch.bindings.slot_count())
                        .map(|slot| match batch.source_of(slot) {
                            LeafSource::Key(word) => batch.key(entry, word),
                            LeafSource::Outer => batch.bindings.get(slot),
                        })
                        .collect(),
                );
            }
            Flow::Continue
        }
        fn begin_scan(&mut self, _: &LeafScan<'_>) -> ScanOffer {
            panic!("physical distinctness must never offer raw source multiplicity")
        }
    }
    let schema = schema(2);
    let normalized = normalized(
        vec![occurrence(0, 0, &[(0, 0)]), occurrence(1, 1, &[(0, 1)])],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    assert_eq!(
        plan.nodes().len(),
        2,
        "first occurrence terminates before the leaf"
    );
    assert!(
        plan.distinct_witness().is_none(),
        "hidden fields distinguish stored facts"
    );
    let witness = plan.scalar_set_traversal().expect("scalar physical proof");
    let images = views_of(
        &schema,
        &[
            vec![(7, 1), (7, 2), (8, 3)],
            vec![(11, 1), (11, 2), (12, 3)],
        ],
    );
    let mut colts = colts_for(&plan, &images);
    let mut executor = Executor::new(&plan);
    executor.set_physical_distinct(Some(witness));
    let gate = plan.nodes()[0]
        .subatoms
        .iter()
        .position(|sub| sub.occ == OccId(1))
        .expect("S has an empty-key gate before binding its variable");
    assert!(plan.nodes()[0].subatoms[gate].vars.is_empty());
    assert!(!executor.scratch[0].children[gate].is_empty());
    let mut bindings = Bindings::new(plan.slot_count());
    for _ in 0..2 {
        let mut sink = RawRows::default();
        executor
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut NoopCounters,
            )
            .unwrap();
        sink.0.sort_unstable();
        assert_eq!(
            sink.0,
            vec![vec![7, 11], vec![7, 12], vec![8, 11], vec![8, 12]]
        );
        for (node, scratch) in plan.nodes().iter().zip(&executor.scratch) {
            assert!(
                node.covers
                    .iter()
                    .all(|&cover| scratch.children[usize::from(cover)].is_empty()),
                "terminating covers need no child storage; the S() gate still continues"
            );
        }
    }
}
use crate::ir::WordCmp;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one differential scenario switches covers with live and dead children"
)]
fn reused_executor_follows_reordered_dynamic_covers() {
    for pipeline in [false, true] {
        let neg_id = 2 + u16::from(pipeline);
        let schema = schema(usize::from(neg_id) + 1);
        let mut occurrences = vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 1)]),
        ];
        let mut subatoms = vec![
            crate::plan::fj::Subatom {
                occ: OccId(0),
                vars: vec![VarId(0), VarId(1)],
            },
            crate::plan::fj::Subatom {
                occ: OccId(1),
                vars: vec![VarId(1), VarId(0)],
            },
        ];
        if pipeline {
            occurrences.push(occurrence(2, 2, &[(0, 1), (1, 2)]));
            subatoms.push(crate::plan::fj::Subatom {
                occ: OccId(2),
                vars: vec![VarId(1)],
            });
        }
        occurrences.push(negated(neg_id, u32::from(neg_id), &[(0, 0), (1, 1)]));
        let mut normalized = normalized(
            occurrences,
            vec![FilterPredicate::FieldsCompare {
                left: OperandAddr::from(VarId(0)),
                right: OperandAddr::from(VarId(1)),
                op: WordCmp::Ne,
            }],
        );
        normalized
            .word_residuals
            .push(FilterPredicate::FieldsCompare {
                left: OperandAddr::var_word(VarId(0), 0),
                right: OperandAddr::var_word(VarId(1), 0),
                op: WordCmp::Lt,
            });
        let mut nodes = vec![crate::plan::fj::Node {
            estimate: 0,
            subatoms,
        }];
        if pipeline {
            nodes.push(crate::plan::fj::Node {
                estimate: 0,
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(2),
                    vars: vec![VarId(2)],
                }],
            });
        }
        let plan = validate(
            &crate::plan::fj::FjPlan { nodes },
            &normalized,
            &schema,
            &all_vars(&normalized),
        )
        .unwrap();
        let mut executor = Executor::with_batch_size(&plan, 2);
        let mut bindings = Bindings::new(plan.slot_count());
        let payload = |i: u64| if i.is_multiple_of(2) { i - 1 } else { i + 10 };
        for (r_len, s_len) in [(3, 5), (5, 3), (3, 5)] {
            let mut data: Vec<_> = [r_len, s_len]
                .map(|len| (1..=len).map(|i| (i, payload(i))).collect())
                .into();
            if pipeline {
                data.push((1..=5).map(|i| (payload(i), i + 100)).collect());
            }
            data.push(vec![(1, payload(1))]);
            let views = views_of(&schema, &data);
            let mut colts = colts_for(&plan, &views);
            colts[0].force_root().unwrap();
            colts[1].force_root().unwrap();
            let expected: BTreeSet<Vec<u64>> = (1..=r_len.min(s_len))
                .filter(|&i| i < payload(i) && i != 1)
                .map(|i| {
                    let mut row = vec![0; plan.slot_count()];
                    row[plan.slot_of(VarId(0))] = i;
                    row[plan.slot_of(VarId(1))] = payload(i);
                    if pipeline {
                        row[plan.slot_of(VarId(2))] = i + 100;
                    }
                    row
                })
                .collect();
            for _ in 0..2 {
                let sentinel = Cursor::Row(u32::MAX);
                for scratch in &mut executor.scratch {
                    for children in &mut scratch.children {
                        children.fill(sentinel);
                    }
                }
                let mut sink = CollectSink::default();
                let mut counters = RecordingCounters::default();
                executor
                    .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
                    .unwrap();
                let chosen = usize::from(s_len < r_len);
                assert_eq!(counters.cover_choices[0], (0, chosen, true));
                assert_eq!(executor.scratch[0].source_cover, Some(chosen));
                assert_eq!(sink.rows, expected, "pipeline {pipeline}, cover {chosen}");
                for children in &executor.scratch[0].children[..2] {
                    assert!(children.is_empty());
                }
                if pipeline {
                    assert!(
                        executor.scratch[0].children[2]
                            .iter()
                            .any(|&c| c != sentinel),
                        "the continuing sibling must retain its real cursors"
                    );
                    assert!(executor.scratch[1].children.iter().all(Vec::is_empty));
                }
            }
        }
    }
}

#[test]
fn point_source_layouts_follow_reordered_covers_without_retaining_values() {
    let pre = NodePrecompute {
        residual_slots: vec![],
        allen_residual_slots: vec![],
        point_probes: vec![PointProbeSpec {
            occ: 0,
            parts: vec![(0, 1, 2, false), (2, 3, 7, true)],
        }],
        anti_probes: vec![AntiProbeSpec {
            occ: 1,
            form: AntiProbeForm::Gate,
            point_parts: vec![(4, 5, 5, true), (6, 7, 2, false)],
        }],
    };
    let mut scratch = NodeScratch {
        sources: vec![vec![], vec![]],
        point_sources: vec![vec![]],
        anti_sources: vec![vec![]],
        anti_point_sources: vec![vec![]],
        ..NodeScratch::default()
    };
    let slots = [vec![2, 5], vec![5, 2]];
    for (round, cover) in [0, 1, 1, 0].into_iter().enumerate() {
        scratch.prepare_sources(&slots, &pre, cover);
        let values: Vec<_> = (0..8).map(|slot| (round * 100 + slot) as u64).collect();
        let batch: Vec<_> = slots[cover].iter().map(|&slot| values[slot]).collect();
        let resolve = |sources: &[PointSource]| -> Vec<_> {
            sources
                .iter()
                .map(|&(start, end, source, dense)| {
                    let point = match source {
                        Source::Batch(word) => batch[word],
                        Source::Slot(slot) => values[slot],
                    };
                    (start, end, point, dense)
                })
                .collect()
        };
        assert_eq!(
            resolve(&scratch.point_sources[0]),
            vec![(0, 1, values[2], false), (2, 3, values[7], true)]
        );
        assert_eq!(
            resolve(&scratch.anti_point_sources[0]),
            vec![(4, 5, values[5], true), (6, 7, values[2], false)]
        );
    }
}

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
