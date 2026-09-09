//! Borrowed aggregate inputs preserve distinctness, exact folds, and scratch reuse.
use super::*;
use crate::exec::run::{Bindings, Flow, LeafBatch, Sink as _};

#[test]
fn borrowed_batch_refusal_restores_routes_after_reset_and_release() {
    let plan = borrowed_plan(false);
    let group = plan.slot_of(VarId(1));
    let amount = plan.slot_of(VarId(2));
    let finds = [
        var_spec(&plan, 1),
        agg_spec(&plan, FoldOp::Sum, 2, true),
        FindSpec::Agg(AggSpec::Count),
    ];
    let mut sink =
        AggregateSink::new_distinct(&finds, plan.slot_count(), plan.distinct_witness().unwrap());
    let mut bindings = Bindings::new(plan.slot_count());
    bindings.set(plan.slot_of(VarId(0)), 77);
    let feed = |sink: &mut AggregateSink, words: &[u64], slots: &[usize]| {
        sink.emit_batch(&LeafBatch {
            keys: words,
            arity: slots.len(),
            survivors: &[0],
            key_slots: slots,
            bindings: &bindings,
        })
    };
    assert_eq!(
        feed(&mut sink, &[0, i64_to_word(3)], &[group, amount]),
        Flow::Continue
    );
    let routing = (
        sink.cached_leaf_words.as_ptr(),
        sink.cached_leaf_words.capacity(),
    );
    let words = sink.cached_leaf_words.clone();
    sink.group_counts[0] = u64::MAX;
    assert_eq!(
        feed(&mut sink, &[0, i64_to_word(4)], &[group, amount]),
        Flow::Error
    );
    assert_eq!(
        (
            sink.cached_leaf_words.as_ptr(),
            sink.cached_leaf_words.capacity()
        ),
        routing
    );
    assert_eq!(sink.cached_leaf_words, words);
    assert_eq!(sink.binding_scratch.capacity(), 0);
    let mut published = 0;
    let result = sink.finalize_into(&mut Vec::new(), |_| {
        published += 1;
        Ok(())
    });
    assert!(matches!(
        result,
        Err(crate::Error::Overflow(crate::OverflowKind::Cardinality))
    ));
    assert_eq!(published, 0);

    sink.reset();
    assert_eq!(
        feed(&mut sink, &[i64_to_word(9), 1], &[amount, group]),
        Flow::Continue
    );
    let mut rows = Vec::new();
    sink.finalize_into(&mut Vec::new(), |row| {
        rows.push(row.to_vec());
        Ok(())
    })
    .unwrap();
    assert_eq!(rows, [vec![1, i64_to_word(9), 1]]);
    sink.release_memory();
    assert_eq!(sink.cached_leaf_words.capacity(), 0);
    assert_eq!(sink.binding_scratch.capacity(), 0);

    sink.reset();
    bindings.set(group, 2);
    bindings.set(amount, i64_to_word(11));
    assert_eq!(
        sink.emit_batch(&LeafBatch {
            keys: &[],
            arity: 0,
            survivors: &[0],
            key_slots: &[],
            bindings: &bindings,
        }),
        Flow::Continue
    );
    assert_eq!(sink.cached_leaf_words.len(), plan.slot_count());
    assert!(sink.cached_leaf_words.iter().all(Option::is_none));
    assert_eq!(sink.binding_scratch.capacity(), 0);
    assert_eq!(sink.into_answers().unwrap(), [vec![2, i64_to_word(11), 1]]);
}

fn borrowed_plan(physical: bool) -> ValidatedPlan {
    let schema = schema();
    let vars = if physical {
        vec![(1, 1), (2, 2)]
    } else {
        vec![(0, 0), (1, 1), (2, 2)]
    };
    let normalized = normalized(&schema, vec![occurrence(0, POSTING, &vars)], vec![]);
    planned(&schema, &normalized, &[0], &[1])
}

#[test]
fn witnessed_rows_do_not_stage_bindings_and_reuse_warm_storage() {
    for physical in [false, true] {
        let plan = borrowed_plan(physical);
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
                        assert_eq!(
                            sink.binding_scratch.len(),
                            if physical { plan.slot_count() } else { 0 }
                        );
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
                        assert_borrowed_answers(&mut sink, batch_len);
                    }
                    if physical {
                        assert_unwitnessed_reuse(sink, &keys, &key_slots, &bindings);
                    }
                }
            }
        }
    }
}

fn assert_borrowed_answers(sink: &mut AggregateSink, batch_len: usize) {
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

fn assert_unwitnessed_reuse(
    mut sink: AggregateSink,
    keys: &[u64],
    key_slots: &[usize],
    bindings: &Bindings,
) {
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
            keys,
            arity: 2,
            survivors: &[0, 0],
            key_slots,
            bindings,
        }),
        Flow::Continue
    );
    assert_eq!(sink.binding_scratch.len(), bindings.slot_count());
    assert_eq!(
        (
            sink.binding_scratch.as_ptr(),
            sink.binding_scratch.capacity()
        ),
        staging
    );
    assert_eq!(sink.into_answers().unwrap(), [vec![0, i64_to_word(1), 1]]);
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

fn float_plan() -> ValidatedPlan {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "FloatRow".into(),
            fields: [
                ("id", ValueType::U64),
                ("group", ValueType::U64),
                ("value", ValueType::F64),
            ]
            .into_iter()
            .map(|(name, value_type)| FieldDescriptor {
                name: name.into(),
                value_type,
            })
            .collect(),
        }],
        statements: vec![],
    }
    .validate()
    .unwrap();
    let normalized = normalized(
        &schema,
        vec![occurrence(0, RelationId(0), &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    planned(&schema, &normalized, &[0], &[1])
}

#[test]
fn borrowed_float_aliases_are_exact_across_layouts_and_partition_flushes() {
    let plan = float_plan();
    let group = plan.slot_of(VarId(1));
    let value = plan.slot_of(VarId(2));
    let f = |value| crate::F64::from(value).to_order_key();
    let finds = [
        var_spec(&plan, 1),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Sum,
            slot: value,
        }),
        FindSpec::Agg(AggSpec::Float {
            op: FoldOp::Mean,
            slot: value,
        }),
        FindSpec::Agg(AggSpec::Count),
    ];
    for flush in [false, true] {
        let mut sink = AggregateSink::new_distinct(
            &finds,
            plan.slot_count(),
            plan.distinct_witness().unwrap(),
        );
        sink.begin(Some(crate::api::db::test_operation()));
        assert_eq!(sink.binding_scratch.capacity(), 0);
        sink.binding_scratch.fill(u64::MAX);
        let mut bindings = Bindings::new(plan.slot_count());
        bindings.set(plan.slot_of(VarId(0)), 0);
        assert_eq!(
            sink.emit_batch(&LeafBatch {
                keys: &[f(1e16), 7],
                arity: 2,
                survivors: &[0],
                key_slots: &[value, group],
                bindings: &bindings,
            }),
            Flow::Continue
        );
        if flush {
            sink.force_spill().unwrap();
        }
        assert_eq!(
            sink.emit_batch(&LeafBatch {
                keys: &[7, f(1.0), 7, f(-1e16)],
                arity: 2,
                survivors: &[1, 0],
                key_slots: &[group, value],
                bindings: &bindings,
            }),
            Flow::Continue
        );
        assert_eq!(sink.float_accs.len(), 1, "Sum and Mean share one primary");
        assert!(sink.binding_scratch.iter().all(|&word| word == u64::MAX));
        assert_eq!(
            sink.into_answers().unwrap(),
            [vec![
                7,
                f(1.0),
                crate::F64::from_bits(0x3fd5_5555_5555_5555).to_order_key(),
                3,
            ]]
        );
    }
}

#[test]
fn borrowed_pack_reads_both_endpoints_across_layouts_and_partition_flushes() {
    let schema = schema();
    let normalized = normalized(
        &schema,
        vec![occurrence(0, PAYROLL, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    let group = plan.slot_of(VarId(1));
    let start = plan.slot_of(VarId(2));
    let finds = [var_spec(&plan, 1), FindSpec::Pack { slot: start }];
    for flush in [false, true] {
        let mut sink = AggregateSink::new_distinct(
            &finds,
            plan.slot_count(),
            plan.distinct_witness().unwrap(),
        );
        sink.begin(Some(crate::api::db::test_operation()));
        assert_eq!(sink.binding_scratch.capacity(), 0);
        sink.binding_scratch.fill(u64::MAX);
        let mut bindings = Bindings::new(plan.slot_count());
        bindings.set(plan.slot_of(VarId(0)), 0);
        assert_eq!(
            sink.emit_batch(&LeafBatch {
                keys: &[i64_to_word(9), 7, i64_to_word(3)],
                arity: 3,
                survivors: &[0],
                key_slots: &[start + 1, group, start],
                bindings: &bindings,
            }),
            Flow::Continue
        );
        if flush {
            sink.force_spill().unwrap();
        }
        assert_eq!(
            sink.emit_batch(&LeafBatch {
                keys: &[7, i64_to_word(1), i64_to_word(5)],
                arity: 3,
                survivors: &[0],
                key_slots: &[group, start, start + 1],
                bindings: &bindings,
            }),
            Flow::Continue
        );
        assert!(sink.binding_scratch.iter().all(|&word| word == u64::MAX));
        assert_eq!(
            sink.into_answers().unwrap(),
            [vec![7, i64_to_word(1), i64_to_word(9)]]
        );
    }
}
