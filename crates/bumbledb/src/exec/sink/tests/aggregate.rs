use super::*;
use crate::error::{Error, FindIndex};
use crate::ir::FoldOp;

#[test]
fn float_sum_and_mean_share_one_exact_total() {
    use crate::exec::run::{Bindings, LeafBatch, Sink as _};
    use bumbledb_theory::F64;
    let finds = [
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Sum,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Mean,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink = AggregateSink::new(&finds, 2);
    let bindings = Bindings::new(2);
    let keys = [
        0,
        F64::from(1e16).to_order_key(),
        1,
        F64::from(1.0).to_order_key(),
        2,
        F64::from(-1e16).to_order_key(),
        3,
        F64::from(99.0).to_order_key(),
    ];
    for _ in 0..2 {
        sink.emit_batch(&LeafBatch {
            keys: &keys,
            arity: 2,
            survivors: &[0, 2, 1],
            key_slots: &[0, 1],
            bindings: &bindings,
        });
    }
    assert_eq!(
        sink.float_accs.len(),
        1,
        "one exact sum/count for two output operators"
    );
    assert_eq!(
        sink.into_answers().unwrap(),
        vec![vec![
            F64::from(1.0).to_order_key(),
            F64::from_bits(0x3fd5_5555_5555_5555).to_order_key(),
            3
        ]]
    );

    let mut sink = AggregateSink::new(&finds, 2);
    let mut bindings = Bindings::new(2);
    bindings.set(1, F64::from(3.0).to_order_key());
    sink.emit_batch(&LeafBatch {
        keys: &[0, 1, 2],
        arity: 1,
        survivors: &[0, 1, 2],
        key_slots: &[0],
        bindings: &bindings,
    });
    assert_eq!(
        sink.into_answers().unwrap(),
        vec![vec![
            F64::from(9.0).to_order_key(),
            F64::from(3.0).to_order_key(),
            3
        ]]
    );
}

#[test]
fn written_union_does_not_share_float_inputs_that_alias_in_only_one_rule() {
    use crate::exec::run::{Bindings, Sink as _};
    use bumbledb_theory::F64;
    let finds = |mean_slot| {
        [
            FindSpec::Agg(AggSpec::Float {
                op: FoldOp::Sum,
                slot: 0,
            }),
            FindSpec::Agg(AggSpec::Float {
                op: FoldOp::Mean,
                slot: mean_slot,
            }),
        ]
    };
    let mut sink = AggregateSink::for_union(&finds(0), 2, 0);
    let mut bindings = Bindings::new(2);
    bindings.set(0, F64::from(1.0).to_order_key());
    bindings.set(1, F64::from(10.0).to_order_key());
    sink.emit(&bindings);
    sink.aim(&finds(1), 2, &[]);
    bindings.set(0, F64::from(2.0).to_order_key());
    bindings.set(1, F64::from(20.0).to_order_key());
    sink.emit(&bindings);
    assert_eq!(sink.float_accs.len(), 2);
    assert_eq!(
        sink.into_answers().unwrap(),
        vec![vec![
            F64::from(3.0).to_order_key(),
            F64::from(10.5).to_order_key()
        ]]
    );
}

#[test]
fn cardinality_failure_precedes_finalization_and_reset_clears_the_failure() {
    use crate::exec::run::{Bindings, Sink as _};
    use bumbledb_theory::F64;
    for value in [F64::ZERO, F64::NAN, F64::INFINITY, F64::NEG_INFINITY] {
        let mut sink = AggregateSink::new(
            [
                FindSpec::Agg(AggSpec::Float {
                    op: FoldOp::Sum,
                    slot: 0,
                }),
                FindSpec::Agg(AggSpec::Count),
            ],
            2,
        );
        let mut bindings = Bindings::new(2);
        bindings.set(0, value.to_order_key());
        sink.emit(&bindings);
        sink.group_counts[0] = u64::MAX; // synthetic boundary; no impossible allocation
        bindings.set(1, 1);
        sink.emit(&bindings);
        let mut emitted = 0;
        assert_eq!(
            sink.finalize_into(&mut Vec::new(), |_| {
                emitted += 1;
                Ok(())
            }),
            Err(Error::Overflow(crate::error::OverflowKind::Cardinality))
        );
        assert_eq!(emitted, 0);
        sink.reset();
        sink.emit(&bindings);
        assert_eq!(
            sink.into_answers().unwrap(),
            vec![vec![value.to_order_key(), 1]]
        );
    }
}

fn sorted_aggregate_rows(sink: &mut AggregateSink) -> Vec<Vec<u64>> {
    let mut rows = Vec::new();
    sink.finalize_into(&mut Vec::new(), |row| {
        rows.push(row.to_vec());
        Ok(())
    })
    .expect("in range");
    rows.sort_unstable();
    rows
}

#[test]
fn shared_batch_reductions_follow_selection_and_layout_changes() {
    use crate::exec::run::{Bindings, LeafBatch, Sink as _};
    use bumbledb_theory::F64;

    let fold = |op, slot| {
        FindSpec::Agg(AggSpec::Fold {
            op,
            slot,
            signed: slot == 1,
        })
    };
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        fold(FoldOp::Min, 1),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Sum,
            slot: 2,
        }),
        fold(FoldOp::Max, 1),
        fold(FoldOp::Sum, 1),
        fold(FoldOp::Min, 1),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Mean,
            slot: 2,
        }),
        fold(FoldOp::Max, 2),
        FindSpec::Agg(AggSpec::Count),
    ];
    let f = |x| F64::from(x).to_order_key();
    let keys = [
        i64_to_word(-7),
        f(1e16),
        i64_to_word(999),
        f(99.0),
        i64_to_word(-3),
        f(1.0),
        i64_to_word(5),
        f(-1e16),
    ];
    {
        let mut sink = AggregateSink::new(&finds, 3);
        let mut reference = AggregateSink::new(&finds, 3);
        sink.begin(Some(crate::api::db::test_operation()));
        let mut feed = |key_slots: &[usize], keys: &[u64], survivors: &[u32], outer: [u64; 3]| {
            feed_batch_and_reference(
                &mut sink,
                &mut reference,
                key_slots,
                keys,
                survivors,
                &outer,
            );
        };
        // Endpoints alone falsely call this dense: row 1 must stay excluded.
        feed(&[1, 2], &keys, &[0, 3, 2], [0, 0, 0]);
        feed(&[1, 2], &keys, &[0, 3, 2], [0, 0, 0]); // dedup and cached layout
        feed(&[1, 2], &keys, &[2, 1, 0], [1, 0, 0]); // descending selection
        feed(
            &[2, 1],
            &[f(8.0), i64_to_word(-4), f(7.0), i64_to_word(9)],
            &[0, 1],
            [0, 0, 0],
        );
        feed(
            &[0, 1, 2],
            &[2, i64_to_word(11), f(2.0), 3, i64_to_word(-12), f(3.0)],
            &[0, 1],
            [0, 0, 0],
        );
        feed(&[], &[], &[0], [4, i64_to_word(-1), f(6.0)]);
        assert_eq!(
            sorted_aggregate_rows(&mut sink),
            sorted_aggregate_rows(&mut reference)
        );
        sink.reset();
        let reversed: Vec<_> = finds.iter().rev().cloned().collect();
        sink.aim(&reversed, 3, &[]);
        reference = AggregateSink::new(&reversed, 3);
        let mut bindings = Bindings::new(3);
        for (slot, word) in [0, i64_to_word(13), f(2.0)].into_iter().enumerate() {
            bindings.set(slot, word);
        }
        reference.emit(&bindings);
        sink.emit_batch(&LeafBatch {
            keys: &[],
            arity: 0,
            survivors: &[0],
            key_slots: &[],
            bindings: &bindings,
        });
        assert_eq!(
            sorted_aggregate_rows(&mut sink),
            sorted_aggregate_rows(&mut reference)
        );
    }
}

fn feed_batch_and_reference(
    sink: &mut AggregateSink,
    reference: &mut AggregateSink,
    key_slots: &[usize],
    keys: &[u64],
    survivors: &[u32],
    outer: &[u64],
) {
    use crate::exec::run::{Bindings, LeafBatch, LeafSource, Sink as _};
    let mut bindings = Bindings::new(outer.len());
    for (slot, &word) in outer.iter().enumerate() {
        bindings.set(slot, word);
    }
    let batch = LeafBatch {
        keys,
        arity: key_slots.len(),
        key_slots,
        survivors,
        bindings: &bindings,
    };
    for &entry in survivors {
        let mut row = Bindings::new(outer.len());
        for slot in 0..outer.len() {
            row.set(
                slot,
                match batch.source_of(slot) {
                    LeafSource::Outer => bindings.get(slot),
                    LeafSource::Key(word) => batch.key(entry, word),
                },
            );
        }
        reference.emit(&row);
    }
    sink.emit_batch(&batch);
}

#[test]
fn union_reaim_invalidates_fold_sources_even_with_the_same_leaf_layout() {
    use crate::exec::run::{Bindings, LeafBatch, Sink as _};
    let finds = |min_slot, max_slot| {
        [
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Min,
                slot: min_slot,
                signed: false,
            }),
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Max,
                slot: max_slot,
                signed: false,
            }),
        ]
    };
    let bindings = Bindings::new(2);
    let mut sink = AggregateSink::for_union(&finds(0, 1), 2, 0);
    sink.emit_batch(&LeafBatch {
        keys: &[100, 1],
        arity: 2,
        survivors: &[0],
        key_slots: &[0, 1],
        bindings: &bindings,
    });
    sink.aim(&finds(1, 0), 2, &[]);
    sink.emit_batch(&LeafBatch {
        keys: &[200, 2],
        arity: 2,
        survivors: &[0],
        key_slots: &[0, 1],
        bindings: &bindings,
    });
    assert_eq!(sink.into_answers().unwrap(), vec![vec![2, 200]]);
}

#[test]
#[cfg_attr(miri, ignore)]
fn constant_group_batches_fold_once_per_run() {
    let schema = schema();

    let mut postings = Vec::new();
    let mut id = 0u64;
    for account in 0..8u64 {
        for i in 0..300i64 {
            postings.push((id, account, i - 150));
            id += 1;
        }
    }
    let views = views_of(&schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );

    let plan = two_node_plan(&schema, &normalized, &[1], &[0, 2], &[0, 1, 2]);
    let finds = |plan: &ValidatedPlan| {
        vec![
            var_spec(plan, 1),
            agg_spec(plan, FoldOp::Sum, 2, true),
            FindSpec::Agg(AggSpec::Count),
            agg_spec(plan, FoldOp::Min, 2, true),
            agg_spec(plan, FoldOp::Max, 2, true),
            // Nonadjacent aliases share reductions, not result positions.
            agg_spec(plan, FoldOp::Sum, 0, false),
            agg_spec(plan, FoldOp::Max, 2, true),
            agg_spec(plan, FoldOp::Sum, 2, true),
            agg_spec(plan, FoldOp::Min, 0, false),
            agg_spec(plan, FoldOp::Min, 2, true),
            // Outer slots stay constant across each scanned suffix.
            agg_spec(plan, FoldOp::Max, 1, false),
        ]
    };

    let mut reference: Option<Vec<Vec<u64>>> = None;
    for (batch, distinct) in [(1usize, true), (7, true), (128, true), (128, false)] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = aggregate_sink(&plan, finds(&plan), distinct);
        sink.begin(Some(crate::api::db::test_operation()));
        let mut execute = |sink: &mut AggregateSink| {
            Executor::with_batch_size(&plan, batch)
                .execute(
                    &plan,
                    &mut colts,
                    &mut bindings,
                    sink,
                    &mut crate::exec::run::NoopCounters,
                )
                .expect("execute");
        };
        execute(&mut sink);
        if distinct && batch == 128 {
            // One sum/extrema kernel per column, one probe per group.
            assert_eq!(sink.fold_inputs.len(), 4);
            assert_eq!(sink.group_probes, 8);
        }
        let rows = sorted_aggregate_rows(&mut sink);

        let capacity = sink.fold_inputs.capacity();
        sink.reset();
        let mut reversed = finds(&plan);
        reversed.reverse();
        sink.aim(&reversed, plan.slot_count(), &[]);
        execute(&mut sink);
        assert_eq!(sink.fold_inputs.capacity(), capacity);
        let mut expected: Vec<Vec<_>> = rows
            .iter()
            .map(|row| row.iter().copied().rev().collect())
            .collect();
        expected.sort_unstable();
        let rerun = sorted_aggregate_rows(&mut sink);
        assert_eq!(rerun, expected, "aliases survive aim and reset");

        assert_eq!(rows.len(), 8, "batch {batch} distinct {distinct}");
        assert_eq!(
            rows[0],
            vec![
                0,
                i64_to_word(-150),
                300,
                i64_to_word(-150),
                i64_to_word(149),
                44850,
                i64_to_word(149),
                i64_to_word(-150),
                0,
                i64_to_word(-150),
                0,
            ],
            "batch {batch} distinct {distinct}"
        );
        match &reference {
            None => reference = Some(rows),
            Some(r) => assert_eq!(*r, rows, "batch {batch} distinct {distinct}"),
        }
    }
}

/// The dedup-then-gather arm — duplicate full bindings collapse before the
/// fold, identically at every batch size, with the group probe still hoisted.
#[test]
fn dedup_constant_group_collapses_duplicates_before_folding() {
    let schema = schema();

    let postings = vec![
        (1u64, 1u64, 5i64),
        (2, 1, 5),
        (3, 1, 7),
        (4, 2, 5),
        (5, 2, 5),
        (6, 2, 5),
    ];
    let views = views_of(&schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
        vec![],
    );
    let plan = two_node_plan(&schema, &normalized, &[0], &[1], &[0, 1]);
    let finds = |plan: &ValidatedPlan| {
        vec![
            var_spec(plan, 0),
            agg_spec(plan, FoldOp::Sum, 1, true),
            FindSpec::Agg(AggSpec::Count),
        ]
    };
    for batch in [1usize, 2, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());

        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_answers().expect("in range");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![1, i64_to_word(12), 2], vec![2, i64_to_word(5), 1],],
            "batch {batch}"
        );
    }
}

#[test]
fn pack_finalize_orders_claims_by_start_word_alone() {
    use crate::exec::run::{Bindings, Sink as _};

    let mut sink = AggregateSink::new(
        vec![
            FindSpec::Var { slot: 0, width: 1 },
            FindSpec::Pack { slot: 1 },
        ],
        3,
    );
    let mut bindings = Bindings::new(3);
    for (group, start, end) in [
        (1u64, 30u64, 40u64),
        (1, 10, 20),
        (1, 10, 15),
        (1, 5, 12),
        (2, 10, u64::MAX),
        (2, 1, 2),
    ] {
        bindings.set(0, group);
        bindings.set(1, start);
        bindings.set(2, end);
        sink.emit(&bindings);
    }
    let mut rows = sink.into_answers().expect("in range");
    rows.sort_unstable();
    assert_eq!(
        rows,
        vec![
            vec![1, 5, 20],
            vec![1, 30, 40],
            vec![2, 1, 2],
            vec![2, 10, u64::MAX],
        ]
    );
}

#[test]
fn count_only_dedup_folds_without_survivor_collection() {
    let schema = schema();

    let postings = vec![
        (1u64, 1u64, 5i64),
        (2, 1, 5),
        (3, 1, 7),
        (4, 2, 5),
        (5, 2, 5),
        (6, 2, 5),
    ];
    let views = views_of(&schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
        vec![],
    );
    let plan = two_node_plan(&schema, &normalized, &[0], &[1], &[0, 1]);

    let finds = |plan: &ValidatedPlan| {
        vec![
            var_spec(plan, 0),
            FindSpec::Agg(AggSpec::Count),
            agg_spec(plan, FoldOp::Sum, 0, false),
        ]
    };
    for batch in [1usize, 2, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());

        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_answers().expect("in range");
        rows.sort_unstable();

        assert_eq!(rows, vec![vec![1, 2, 2], vec![2, 1, 2]], "batch {batch}");
    }
}

#[test]
fn constant_over_slot_folds_value_times_count() {
    let schema = schema();

    let big = u64::MAX / 2;
    let mut postings = vec![];
    for id in 0..5u64 {
        postings.push((id, big, 1i64));
    }
    for id in 5..8u64 {
        postings.push((id, 7u64, 1i64));
    }
    let views = views_of(&schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = two_node_plan(&schema, &normalized, &[1], &[0, 2], &[0, 1, 2]);
    let finds =
        |plan: &ValidatedPlan| vec![var_spec(plan, 1), agg_spec(plan, FoldOp::Sum, 1, false)];

    for distinct in [true, false] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = aggregate_sink(&plan, finds(&plan), distinct);
        Executor::with_batch_size(&plan, 128)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let err = sink.into_answers().unwrap_err();
        assert!(
            matches!(
                err,
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: FindIndex(1) })
            ),
            "{err:?}"
        );
    }
    // Value parity in range: drop the big account.
    let views = views_of(&schema, &postings[5..], &[]);
    for distinct in [true, false] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = aggregate_sink(&plan, finds(&plan), distinct);
        Executor::with_batch_size(&plan, 128)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let rows = sink.into_answers().expect("in range");
        assert_eq!(rows, vec![vec![7, 21]], "distinct {distinct}");
    }
}

#[test]
fn aggregate_leaf_batches_match_the_scalar_fold_at_the_boundary() {
    let schema = schema();

    let postings = vec![
        (1u64, 7u64, i64::MAX),
        (2, 7, 1),
        (3, 7, -2),
        (4, 7, 1),
        (5, 8, i64::MAX),
        (6, 8, 1),
    ];
    let views = views_of(&schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    let finds = |plan: &ValidatedPlan| {
        vec![
            var_spec(plan, 1),
            agg_spec(plan, FoldOp::Sum, 2, true),
            FindSpec::Agg(AggSpec::Count),
        ]
    };
    for batch in [1usize, 2, 7, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = aggregate_sink(&plan, finds(&plan), true);
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");

        let err = sink.into_answers().unwrap_err();
        assert!(
            matches!(
                err,
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: FindIndex(1) })
            ),
            "batch {batch}: {err:?}"
        );
    }

    let views = views_of(&schema, &postings[..4], &[]);
    let mut reference: Option<Vec<Vec<u64>>> = None;
    for batch in [1usize, 2, 7, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = aggregate_sink(&plan, finds(&plan), true);
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_answers().expect("in range");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![7, i64_to_word(i64::MAX), 4]],
            "batch {batch}"
        );
        match &reference {
            None => reference = Some(rows),
            Some(r) => assert_eq!(*r, rows, "batch {batch}"),
        }
    }
}

#[test]
fn interval_group_keys_span_both_words() {
    let schema = schema();

    let rows = vec![
        (1u64, 10u64, (5i64, 9i64)),
        (2, 11, (5, 9)),
        (3, 12, (5, 7)),
    ];
    let views = payroll_views_of(&schema, &rows);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, PAYROLL, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[2]);
    for distinct in [true, false] {
        let finds = vec![var_spec(&plan, 2), FindSpec::Agg(AggSpec::Count)];
        let mut got = run_aggregate_distinct(&plan, &views, finds, distinct).expect("rows");
        got.sort_unstable();
        assert_eq!(
            got,
            vec![
                vec![i64_to_word(5), i64_to_word(7), 1],
                vec![i64_to_word(5), i64_to_word(9), 2],
            ],
            "distinct {distinct}"
        );
    }
}

#[test]
fn the_union_seen_set_keys_head_projections_across_rule_layouts() {
    use crate::exec::run::{Bindings, Sink};

    let spec = |group: usize, x: usize| {
        vec![
            FindSpec::Var {
                slot: group,
                width: 1,
            },
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Sum,
                slot: x,
                signed: false,
            }),
            FindSpec::Agg(AggSpec::Count),
        ]
    };
    let mut sink = AggregateSink::for_union(&spec(0, 1), 2, 0);
    sink.reset();

    let mut bindings = Bindings::new(2);
    for x in [100u64, 250] {
        bindings.reset();
        bindings.set(0, 7);
        bindings.set(1, x);
        sink.emit(&bindings);
    }
    assert_eq!(sink.distinct_seen(), Some(2), "rule A seeds the union");

    sink.aim(&spec(2, 0), 3, &[]);
    let mut bindings = Bindings::new(3);
    for (x, existential) in [(100u64, 41u64), (300, 42)] {
        bindings.reset();
        bindings.set(0, x);
        bindings.set(1, existential);
        bindings.set(2, 7);
        sink.emit(&bindings);
    }
    assert_eq!(
        sink.distinct_seen(),
        Some(3),
        "the cross-layout duplicate was absorbed by the head-shaped key"
    );

    let rows = sink.into_answers().expect("in range");
    assert_eq!(
        rows,
        vec![vec![7, 650, 3]],
        "Sum folds {{100, 250, 300}} once each; Count counts the union"
    );
}

/// DNF disjuncts share a variable scope. Dedup keys read the same complete
/// binding through each plan layout, absorbing repeated derivations while
/// preserving distinct bindings that project to equal head rows.
#[test]
fn the_dnf_union_seen_set_keys_shared_slot_arrays_across_clone_layouts() {
    use crate::exec::run::{Bindings, Sink};

    let spec = |g: usize, x: usize| {
        vec![
            FindSpec::Var { slot: g, width: 1 },
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Sum,
                slot: x,
                signed: false,
            }),
        ]
    };
    // VarId order: v0 → g's slot, v1 → x's, v2 → e's, per clone.
    let spans_a = [(0, 1), (1, 1), (2, 1)];
    let spans_b = [(2, 1), (1, 1), (0, 1)];
    let mut sink = AggregateSink::for_dnf_union(&spec(0, 1), 3, &spans_a, 0);
    sink.reset();

    let mut bindings = Bindings::new(3);
    bindings.reset();
    bindings.set(0, 7);
    bindings.set(1, 100);
    bindings.set(2, 5);
    sink.emit(&bindings);
    assert_eq!(sink.distinct_seen(), Some(1), "clone A seeds the union");

    sink.aim(&spec(2, 1), 3, &spans_b);
    let mut bindings = Bindings::new(3);
    for existential in [5u64, 6] {
        bindings.reset();
        bindings.set(0, existential);
        bindings.set(1, 100);
        bindings.set(2, 7);
        sink.emit(&bindings);
    }
    assert_eq!(
        sink.distinct_seen(),
        Some(2),
        "the cross-disjunct re-derivation was absorbed; the distinct binding was not"
    );

    let rows = sink.into_answers().expect("in range");
    assert_eq!(
        rows,
        vec![vec![7, 200]],
        "Sum folds the written rule's distinct full bindings: 100 + 100"
    );
}

/// Past the group map's index limit, existing groups still fold but a new
/// group refuses with `Capacity::Groups`, and nothing publishes.
#[test]
fn group_exhaustion_refuses_new_groups_with_capacity_groups() {
    use crate::exec::run::{Bindings, Sink as _};

    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink = AggregateSink::new(finds, 2);
    sink.begin(Some(crate::work::WorkContext::new()));
    let mut bindings = Bindings::new(2);
    bindings.set(0, 7);
    bindings.set(1, 1);
    sink.emit(&bindings);
    let super::super::GroupTable::Hashed(map) = &mut sink.groups else {
        panic!("hashed groups")
    };
    map.assume_full();
    bindings.set(1, 2);
    assert!(
        !sink.emit(&bindings).is_terminal(),
        "an existing group folds"
    );
    assert!(sink.error.is_none());
    bindings.set(0, 8);
    assert!(sink.emit(&bindings).is_terminal(), "a new group refuses");
    let mut emitted = 0;
    let refused = sink.finalize_into(&mut Vec::new(), |_| {
        emitted += 1;
        Ok(())
    });
    assert_eq!(
        refused,
        Err(Error::Capacity(crate::error::Capacity::Groups))
    );
    assert_eq!(emitted, 0, "no partial group published");
}

#[test]
fn dense_group_tables_match_the_hashed_map_word_for_word() {
    use crate::exec::run::{Bindings, Sink};

    let spec = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Var { slot: 1, width: 1 },
        FindSpec::Agg(AggSpec::Fold {
            op: FoldOp::Sum,
            slot: 2,
            signed: false,
        }),
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut dense = AggregateSink::new_dense(&spec, 3, &[2, 3]);
    let mut hashed = AggregateSink::new(&spec, 3);
    assert!(dense.dense_group_table(), "two proven radixes go dense");
    assert!(!hashed.dense_group_table(), "no proof keeps the map");
    dense.reset();
    hashed.reset();

    let mut bindings = Bindings::new(3);
    for (a, b, x) in [
        (1u64, 2u64, 10u64),
        (0, 0, 1),
        (1, 0, 5),
        (0, 2, 7),
        (1, 2, 30),
        (0, 1, 2),
        (1, 1, 4),
        (1, 2, 10),
    ] {
        bindings.reset();
        bindings.set(0, a);
        bindings.set(1, b);
        bindings.set(2, x);
        dense.emit(&bindings);
        hashed.emit(&bindings);
    }
    let mut dense_rows = dense.into_answers().expect("in range");
    let mut hashed_rows = hashed.into_answers().expect("in range");
    dense_rows.sort_unstable();
    hashed_rows.sort_unstable();
    assert_eq!(dense_rows, hashed_rows, "one denotation, two tables");
    assert_eq!(
        dense_rows,
        vec![
            vec![0, 0, 1, 1],
            vec![0, 1, 2, 1],
            vec![0, 2, 7, 1],
            vec![1, 0, 5, 1],
            vec![1, 1, 4, 1],
            vec![1, 2, 40, 2],
        ],
        "mixed-radix ordinals reconstruct every key word"
    );
}

/// F64 MIN and MAX propagate NaN (MAX through NaN's top order key, MIN
/// through its fold as key 0) on the row path, the gathered batch path and
/// the outer-constant path, and agree with each other.
#[test]
fn f64_min_and_max_propagate_nan_on_every_path() {
    use bumbledb_theory::F64;
    let f = |x: f64| F64::from(x).to_order_key();
    let finds = [
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Min,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Max,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Min,
            slot: 2,
        }),
        FindSpec::Agg(AggSpec::Count),
    ];
    let groups: [(u64, &[f64]); 4] = [
        (0, &[3.0, f64::NAN, -1.0]),
        (1, &[2.0, f64::NEG_INFINITY, 5.0]),
        (2, &[f64::NAN]),
        (3, &[0.0, -2.5, f64::INFINITY]),
    ];
    let expected = |values: &[f64], outer: f64| {
        let nan = values.iter().any(|v| v.is_nan());
        let keys: Vec<u64> = values.iter().map(|&v| f(v)).collect();
        let min = if nan {
            f(f64::NAN)
        } else {
            *keys.iter().min().unwrap()
        };
        vec![
            *keys.iter().max().unwrap(),
            min,
            f(outer),
            u64::try_from(values.len()).unwrap(),
        ]
    };
    for outer in [7.5, f64::NAN] {
        let mut batched = AggregateSink::new(&finds, 3);
        let mut rows = AggregateSink::new(&finds, 3);
        for &(group, values) in &groups {
            let keys: Vec<u64> = values.iter().map(|&v| f(v)).collect();
            let survivors: Vec<u32> = (0..u32::try_from(keys.len()).unwrap()).collect();
            feed_batch_and_reference(
                &mut batched,
                &mut rows,
                &[1],
                &keys,
                &survivors,
                &[group, 0, f(outer)],
            );
        }
        for sink in [&mut batched, &mut rows] {
            let got = sorted_aggregate_rows(sink);
            for (row, &(group, values)) in got.iter().zip(&groups) {
                let mut want = vec![group];
                let tail = expected(values, outer);
                want.extend([tail[1], tail[0], tail[2], tail[3]]);
                assert_eq!(row, &want, "group {group} outer {outer}");
            }
        }
    }
}
