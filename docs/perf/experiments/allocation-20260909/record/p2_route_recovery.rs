// Pending regression for sink/tests/borrowed_rows.rs. Not mounted or run.
// Do not change the comparison-2 frozen source until its session is terminal.
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
    let mut sink = AggregateSink::new_distinct(
        &finds,
        plan.slot_count(),
        plan.distinct_witness().unwrap(),
    );
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
    assert_eq!(feed(&mut sink, &[0, i64_to_word(3)], &[group, amount]), Flow::Continue);
    let routing = (sink.cached_leaf_words.as_ptr(), sink.cached_leaf_words.capacity());
    let words = sink.cached_leaf_words.clone();
    sink.group_counts[0] = u64::MAX;
    assert_eq!(feed(&mut sink, &[0, i64_to_word(4)], &[group, amount]), Flow::Error);
    assert_eq!((sink.cached_leaf_words.as_ptr(), sink.cached_leaf_words.capacity()), routing);
    assert_eq!(sink.cached_leaf_words, words);
    assert_eq!(sink.binding_scratch.capacity(), 0);
    let mut published = 0;
    let result = sink.finalize_into(&mut Vec::new(), |_| {
        published += 1;
        Ok(())
    });
    assert!(matches!(result, Err(crate::Error::Overflow(crate::OverflowKind::Cardinality))));
    assert_eq!(published, 0);

    sink.reset();
    assert_eq!(feed(&mut sink, &[i64_to_word(9), 1], &[amount, group]), Flow::Continue);
    let mut rows = Vec::new();
    sink.finalize_into(&mut Vec::new(), |row| {
        rows.push(row.to_vec());
        Ok(())
    }).unwrap();
    assert_eq!(rows, [vec![1, i64_to_word(9), 1]]);
    sink.release_memory();
    assert_eq!(sink.cached_leaf_words.capacity(), 0);
    assert_eq!(sink.binding_scratch.capacity(), 0);

    sink.reset();
    bindings.set(group, 2);
    bindings.set(amount, i64_to_word(11));
    assert_eq!(sink.emit_batch(&LeafBatch {
        keys: &[],
        arity: 0,
        survivors: &[0],
        key_slots: &[],
        bindings: &bindings,
    }), Flow::Continue);
    assert_eq!(sink.cached_leaf_words.len(), plan.slot_count());
    assert!(sink.cached_leaf_words.iter().all(Option::is_none));
    assert_eq!(sink.binding_scratch.capacity(), 0);
    assert_eq!(sink.into_answers().unwrap(), [vec![2, i64_to_word(11), 1]]);
}
