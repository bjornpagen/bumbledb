//! Borrowed aggregate inputs preserve distinctness, exact folds, and scratch reuse.
use super::*;
use crate::exec::run::{Bindings, Flow, LeafBatch, Sink as _};

#[test]
fn witnessed_rows_do_not_stage_bindings_and_reuse_warm_storage() {
    let schema = schema();
    for physical in [false, true] {
        let vars = if physical {
            vec![(1, 1), (2, 2)]
        } else {
            vec![(0, 0), (1, 1), (2, 2)]
        };
        let normalized = normalized(&schema, vec![occurrence(0, POSTING, &vars)], vec![]);
        let plan = planned(&schema, &normalized, &[0], &[1]);
        let group = plan.slot_of(VarId(1));
        let amount = plan.slot_of(VarId(2));
        let finds = [
            var_spec(&plan, 1),
            agg_spec(&plan, FoldOp::Sum, 2, true),
            FindSpec::Agg(AggSpec::Count),
        ];
        for dense in [false, true] {
            for batch_len in [0, 1, 3] {
                for batched in [false, true] {
                    let radixes: &[u16] = if dense { &[2] } else { &[] };
                    let mut sink = if physical {
                        AggregateSink::with_capacity_hint(&finds, plan.slot_count(), 0, radixes)
                    } else {
                        AggregateSink::without_seen_set(
                            &finds,
                            plan.slot_count(),
                            plan.distinct_witness().expect("complete row is bound"),
                            0,
                            radixes,
                        )
                    };
                    let mut bindings = Bindings::new(plan.slot_count());
                    let keys = [
                        i64_to_word(1),
                        0,
                        i64_to_word(99),
                        1,
                        i64_to_word(2),
                        1,
                        i64_to_word(3),
                        0,
                    ];
                    let key_slots = [amount, group];
                    let selection = [3, 0, 2];
                    let survivors = &selection[..batch_len];
                    for warm in [false, true] {
                        sink.reset();
                        if physical {
                            assert!(
                                sink.set_physical_distinct(plan.scalar_set_traversal())
                                    .is_some()
                            );
                        }
                        sink.binding_scratch.fill(u64::MAX);
                        if !physical {
                            bindings.set(plan.slot_of(VarId(0)), 77);
                        }
                        let before = crate::alloc_counter::snapshot().window;
                        if batched {
                            assert_eq!(
                                sink.emit_batch(&LeafBatch {
                                    keys: &keys,
                                    arity: 2,
                                    survivors,
                                    key_slots: &key_slots,
                                    bindings: &bindings,
                                }),
                                Flow::Continue
                            );
                        } else {
                            for &entry in survivors {
                                bindings.set(amount, keys[entry as usize * 2]);
                                bindings.set(group, keys[entry as usize * 2 + 1]);
                                assert_eq!(sink.emit(&bindings), Flow::Continue);
                            }
                        }
                        let after = crate::alloc_counter::snapshot().window;
                        #[cfg(feature = "alloc-counter")]
                        if warm {
                            assert_eq!(
                                after, before,
                                "no fresh allocation or freeing in a warm fold"
                            );
                        }
                        let _ = (warm, before, after);
                        assert!(
                            sink.binding_scratch.iter().all(|&word| word == u64::MAX),
                            "witnessed folds must not stage unused binding words"
                        );
                        let mut rows = Vec::new();
                        sink.finalize_into(&mut Vec::new(), |row| {
                            rows.push(row.to_vec());
                            Ok(())
                        })
                        .unwrap();
                        rows.sort_unstable();
                        let expected = match batch_len {
                            0 => vec![],
                            1 => vec![vec![0, i64_to_word(3), 1]],
                            3 => vec![vec![0, i64_to_word(4), 2], vec![1, i64_to_word(2), 1]],
                            _ => unreachable!(),
                        };
                        assert_eq!(rows, expected);
                    }
                    if physical {
                        sink.reset();
                        assert!(
                            sink.physical_distinct.is_none(),
                            "reset removes the physical witness"
                        );
                        let staging = (
                            sink.binding_scratch.as_ptr(),
                            sink.binding_scratch.capacity(),
                        );
                        assert_eq!(
                            sink.emit_batch(&LeafBatch {
                                keys: &keys,
                                arity: 2,
                                survivors: &[0, 0],
                                key_slots: &key_slots,
                                bindings: &bindings,
                            }),
                            Flow::Continue
                        );
                        assert_eq!(sink.binding_scratch.len(), plan.slot_count());
                        assert_eq!(
                            (
                                sink.binding_scratch.as_ptr(),
                                sink.binding_scratch.capacity()
                            ),
                            staging
                        );
                        assert_eq!(sink.into_answers().unwrap(), [vec![0, i64_to_word(1), 1]]);
                    }
                }
            }
        }
    }
}

#[test]
fn borrowed_row_union_keys_keep_repeated_spans_across_reaim() {
    let finds = |group, value| {
        [
            FindSpec::Var {
                slot: group,
                width: 2,
            },
            FindSpec::Var {
                slot: group,
                width: 2,
            },
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Sum,
                slot: value,
                width: 1,
                signed: false,
            }),
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Sum,
                slot: value,
                width: 1,
                signed: false,
            }),
            FindSpec::Agg(AggSpec::Count),
        ]
    };
    let mut sink = AggregateSink::for_union(&finds(0, 2), 3, 0);
    assert_eq!(
        sink.union_scratch.len(),
        6,
        "union key can be wider than the binding"
    );
    let bindings = Bindings::new(3);
    let feed = |sink: &mut AggregateSink, keys: &[u64]| {
        sink.emit_batch(&LeafBatch {
            keys,
            arity: 3,
            survivors: &[0, 0],
            key_slots: &[0, 1, 2],
            bindings: &bindings,
        })
    };
    assert_eq!(feed(&mut sink, &[10, 11, 5]), Flow::Continue);
    sink.aim(&finds(1, 0), 3, &[]);
    assert_eq!(feed(&mut sink, &[7, 10, 11]), Flow::Continue);
    assert_eq!(
        sink.into_answers().unwrap(),
        [vec![10, 11, 10, 11, 12, 12, 2]]
    );

    let mut count = AggregateSink::for_union(&[FindSpec::Agg(AggSpec::Count)], 3, 0);
    assert_eq!(feed(&mut count, &[1, 2, 3]), Flow::Continue);
    assert_eq!(feed(&mut count, &[4, 5, 6]), Flow::Continue);
    assert!(count.union_scratch.is_empty());
    assert_eq!(
        count.into_answers().unwrap(),
        [vec![1]],
        "zero-word keys still deduplicate"
    );
}

#[test]
fn borrowed_row_refusal_restores_staging_before_reset_and_release() {
    let mut sink = AggregateSink::new([FindSpec::Agg(AggSpec::Count)], 1);
    let mut bindings = Bindings::new(1);
    bindings.set(0, 1);
    assert_eq!(sink.emit(&bindings), Flow::Continue);
    sink.group_counts[0] = u64::MAX;
    let staging = (
        sink.binding_scratch.as_ptr(),
        sink.binding_scratch.capacity(),
    );
    bindings.set(0, 2);
    assert_eq!(sink.emit(&bindings), Flow::Error);
    assert_eq!(
        (
            sink.binding_scratch.as_ptr(),
            sink.binding_scratch.capacity()
        ),
        staging
    );
    assert_eq!(sink.binding_scratch.len(), 1);
    sink.reset();
    assert_eq!(sink.emit(&bindings), Flow::Continue);
    sink.release_memory();
    assert_eq!(sink.binding_scratch.capacity(), 0);
    sink.reset();
    assert_eq!(sink.emit(&bindings), Flow::Continue);
    assert_eq!(sink.into_answers().unwrap(), [vec![1]]);
}
