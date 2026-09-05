use super::*;
use crate::error::{Error, FindIndex};
use crate::ir::FoldOp;

#[test]
fn float_sum_mean_share_one_exact_lane_and_preserve_compact_integer_state() {
    use crate::exec::run::{Bindings, LeafBatch, Sink as _};
    use bumbledb_theory::F64;
    assert!(std::mem::size_of::<Acc>() <= 32);
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

#[test]
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
        ]
    };

    let mut reference: Option<Vec<Vec<u64>>> = None;
    for (batch, distinct) in [(1usize, true), (7, true), (128, true), (128, false)] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = aggregate_sink(&plan, finds(&plan), distinct);
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        if distinct && batch == 128 {
            assert_eq!(
                sink.group_probes, 8,
                "one probe per group run, memoized across batches"
            );
        }
        let mut rows = sink.into_answers().expect("in range");
        rows.sort_unstable();

        assert_eq!(rows.len(), 8, "batch {batch} distinct {distinct}");
        assert_eq!(
            rows[0],
            vec![
                0,
                i64_to_word(-150),
                300,
                i64_to_word(-150),
                i64_to_word(149)
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
                width: 1,
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

/// The DNF-derived union regime re-keys on the SHARED SLOT ARRAYS (ruled
/// 2026-07-23, R2): the disjuncts of one written rule share one variable scope,
/// so the `VarId`-ordered spans read the same binding tuple through each
/// clone's own layout — a cross-disjunct re-derivation is absorbed, while
/// distinct full bindings projecting to EQUAL head rows all fold (the
/// head-projection key would eat them; the or-transparency law forbids exactly
/// that — `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
#[test]
fn the_dnf_union_seen_set_keys_shared_slot_arrays_across_clone_layouts() {
    use crate::exec::run::{Bindings, Sink};

    let spec = |g: usize, x: usize| {
        vec![
            FindSpec::Var { slot: g, width: 1 },
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Sum,
                slot: x,
                width: 1,
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

/// G05 (grouped-reduction half): the group tables/accumulator banks move
/// onto the charged scratch relation at forced transition points — before
/// the first group (zero allowance creates the scratch tier ahead of row
/// one), during the stream (a tiny allowance flushes mid-groups) and after
/// the first group (an allowance crossed only once several groups exist).
/// Every regime produces identical answer words, INCLUDING exact float
/// bits (`lean/Bumbledb/Float64/Sum.lean` merge laws license the
/// partition merges; the limb bank round-trips bit-for-bit).
#[test]
fn group_state_spill_matches_resident_bits_before_during_and_after_first_group() {
    use crate::exec::run::{Bindings, Sink as _};
    use bumbledb_theory::F64;

    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Sum,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Mean,
            slot: 1,
        }),
        FindSpec::Agg(AggSpec::Fold {
            op: FoldOp::Sum,
            slot: 2,
            width: 1,
            signed: true,
        }),
        FindSpec::Agg(AggSpec::Count),
        FindSpec::Agg(AggSpec::Fold {
            op: FoldOp::Min,
            slot: 2,
            width: 1,
            signed: true,
        }),
    ];
    let feed = |sink: &mut AggregateSink| {
        let mut bindings = Bindings::new(3);
        // Catastrophic-cancellation floats across interleaved groups, so
        // flush partitions cut through every group repeatedly.
        for i in 0..96u64 {
            let group = i % 5;
            bindings.reset();
            bindings.set(0, group);
            bindings.set(
                1,
                F64::from(
                    if i.is_multiple_of(2) { 1e16 } else { -1e16 }
                        + f64::from(u32::try_from(i).expect("96 rows")) * 0.25,
                )
                .to_order_key(),
            );
            bindings.set(2, i64_to_word(i.cast_signed() - 48));
            sink.emit(&bindings);
        }
    };
    let work = crate::api::prepared::source::UNBOUNDED_POLICY
        .start()
        .expect("unbounded ledger");
    let mut resident = AggregateSink::new(finds.clone(), 3);
    feed(&mut resident);
    assert!(!resident.group_state_spilled());
    let mut expected = resident.into_answers().expect("resident");
    expected.sort_unstable();
    assert_eq!(expected.len(), 5, "five groups");

    // Zero / tiny / late allowances: before, during, after the first group.
    for ram_bytes in [0usize, 700, 2000] {
        let mut spilled = AggregateSink::new(finds.clone(), 3);
        spilled.begin(Some(crate::exec::sink::SinkBudget {
            work: work.clone(),
            ram_bytes,
        }));
        feed(&mut spilled);
        assert!(
            spilled.group_state_spilled(),
            "allowance {ram_bytes} forces the transition"
        );
        let mut got = spilled.into_answers().expect("spilled");
        got.sort_unstable();
        assert_eq!(got, expected, "allowance {ram_bytes}: exact word parity");
    }

    // A budget that is never crossed stays resident — the spill is a
    // pressure response, not a mode.
    let mut roomy = AggregateSink::new(finds, 3);
    roomy.begin(Some(crate::exec::sink::SinkBudget {
        work,
        ram_bytes: 1 << 20,
    }));
    feed(&mut roomy);
    assert!(!roomy.group_state_spilled());
    let mut got = roomy.into_answers().expect("roomy");
    got.sort_unstable();
    assert_eq!(got, expected);
}

/// The spilled Pack drain streams maximal segments from the scratch map's
/// key order — same segments as the resident sweep, claims interleaved
/// across groups and out of start order.
#[test]
fn pack_group_spill_streams_the_same_maximal_segments() {
    use crate::exec::run::{Bindings, Sink as _};

    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Pack { slot: 1 },
    ];
    let claims: &[(u64, u64, u64)] = &[
        (1, 30, 40),
        (2, 10, u64::MAX),
        (1, 10, 20),
        (2, 1, 2),
        (1, 10, 15),
        (1, 5, 12),
        (1, 30, 40), // duplicate claim from a distinct binding
        (3, 7, 8),
    ];
    let feed = |sink: &mut AggregateSink| {
        let mut bindings = Bindings::new(4);
        for (i, (group, start, end)) in claims.iter().enumerate() {
            bindings.reset();
            // Slot 3 makes each binding distinct, so the duplicate claim
            // survives dedup and exercises the scratch set's exact
            // insert-if-absent.
            bindings.set(0, *group);
            bindings.set(1, *start);
            bindings.set(2, *end);
            bindings.set(3, i as u64);
            sink.emit(&bindings);
        }
    };
    // Pack claims sit in slots (1, 2): slot 1 carries start, slot 1+1 end.
    let mut resident = AggregateSink::new(finds.clone(), 4);
    feed(&mut resident);
    let mut expected = resident.into_answers().expect("resident");
    expected.sort_unstable();
    assert_eq!(
        expected,
        vec![
            vec![1, 5, 20],
            vec![1, 30, 40],
            vec![2, 1, 2],
            vec![2, 10, u64::MAX],
            vec![3, 7, 8],
        ]
    );

    let work = crate::api::prepared::source::UNBOUNDED_POLICY
        .start()
        .expect("unbounded ledger");
    for ram_bytes in [0usize, 96] {
        let mut spilled = AggregateSink::new(finds.clone(), 4);
        spilled.begin(Some(crate::exec::sink::SinkBudget {
            work: work.clone(),
            ram_bytes,
        }));
        feed(&mut spilled);
        assert!(spilled.group_state_spilled(), "allowance {ram_bytes}");
        let mut got = spilled.into_answers().expect("spilled");
        got.sort_unstable();
        assert_eq!(got, expected, "allowance {ram_bytes}");
    }
}

/// Cardinality stays total across spilled partition merges: a merged group
/// count past `u64::MAX` is the same typed overflow the resident fold
/// raises, and no group publishes (Q-ATOMIC).
#[test]
fn spilled_partition_merge_refuses_cardinality_overflow() {
    use crate::exec::run::{Bindings, Sink as _};

    let finds = vec![
        FindSpec::Var { slot: 0, width: 1 },
        FindSpec::Agg(AggSpec::Count),
    ];
    let work = crate::api::prepared::source::UNBOUNDED_POLICY
        .start()
        .expect("unbounded ledger");
    let mut sink = AggregateSink::new(finds, 2);
    sink.begin(Some(crate::exec::sink::SinkBudget { work, ram_bytes: 0 }));
    let mut bindings = Bindings::new(2);
    bindings.set(0, 7);
    bindings.set(1, 1);
    sink.emit(&bindings); // group 7 folds in RAM
    bindings.set(1, 2);
    sink.emit(&bindings); // entry flush moves count 1 to scratch; folds again
    assert!(sink.group_state_spilled());
    // Synthetic boundary (no impossible allocation): the next flush merges
    // RAM count u64::MAX into the spilled count 1.
    sink.group_counts[0] = u64::MAX;
    bindings.set(1, 3);
    sink.emit(&bindings);
    let mut emitted = 0;
    let refused = sink.finalize_into(&mut Vec::new(), |_| {
        emitted += 1;
        Ok(())
    });
    assert_eq!(
        refused,
        Err(Error::Overflow(crate::error::OverflowKind::Cardinality))
    );
    assert_eq!(emitted, 0, "no partial group published");
    // Failure → success reuse: reset disposes the scratch tier.
    sink.reset();
    assert!(!sink.group_state_spilled());
    bindings.set(0, 9);
    bindings.set(1, 1);
    sink.emit(&bindings);
    assert_eq!(sink.into_answers().unwrap(), vec![vec![9, 1]]);
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
            width: 1,
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
