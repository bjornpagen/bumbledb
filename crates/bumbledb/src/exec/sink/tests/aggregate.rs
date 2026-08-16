use super::*;
use crate::error::Error;
use crate::ir::FoldOp;

/// The constant-group fast path — one group
/// probe per run (memoized across batches), gather folds for every
/// op — is value-identical to the per-row seen path at every batch
/// size, on the stats shape (group key bound above the leaf).
#[test]
fn constant_group_batches_fold_once_per_run() {
    let dir = TempDir::new("sink-constant-group");
    let schema = schema();
    // 8 accounts x 300 postings: each account's leaf subtree spans
    // several batches at size 128 — the run memo holds probes at 8.
    let mut postings = Vec::new();
    let mut id = 0u64;
    for account in 0..8u64 {
        for i in 0..300i64 {
            postings.push((id, account, i - 150));
            id += 1;
        }
    }
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    // Hand-factored GJ plan: n0 binds the account, n1 the
    // (id, amount) suffix — the stats shape, where the leaf's group
    // key is outer.
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
    // The fast path (elided) vs the per-row seen path, across sizes.
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
        // Per account: Sum = -150, Count = 300, Min = -150, Max = 149.
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

/// The dedup-then-gather arm — duplicate full bindings
/// collapse before the fold, identically at every batch size, with
/// the group probe still hoisted.
#[test]
fn dedup_constant_group_collapses_duplicates_before_folding() {
    let dir = TempDir::new("sink-dedup-constant");
    let schema = schema();
    // Fresh ids exist in storage but the query does not bind them:
    // (account, amount) bindings collapse. Account 1 holds amounts
    // {5, 5, 7} -> {5, 7}; account 2 holds {5, 5, 5} -> {5}.
    let postings = vec![
        (1u64, 1u64, 5i64),
        (2, 1, 5),
        (3, 1, 7),
        (4, 2, 5),
        (5, 2, 5),
        (6, 2, 5),
    ];
    let views = views_of(&dir, &schema, &postings, &[]);
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
        // distinct_bindings = false: the dedup arm is mandatory.
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

/// Pack's finalize sort orders claims by the START word alone — the
/// sweep's one precondition. Equal-start claims coalesce identically
/// whatever their end order, so adversarial emit orders (longer end
/// first, dominated claim last, detached segment interleaved, a ray)
/// pack to the same maximal segments.
#[test]
fn pack_finalize_orders_claims_by_start_word_alone() {
    use crate::exec::run::{Bindings, Sink as _};
    // Slots: 0 = group, 1..3 = the claim interval's word pair.
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
        (1, 10, 20),       // equal start, longer end FIRST
        (1, 10, 15),       // ... shorter end second
        (1, 5, 12),        // overlaps [10, 20] from the left
        (2, 10, u64::MAX), // a packed ray is a ray
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

/// The count-only dedup arm — no fold input reads the batch keys, so
/// the seen-set pass counts first sights instead of collecting a
/// survivor list — is value-identical to the gather regime: duplicate
/// full bindings collapse, outer-sourced folds stay value × count, at
/// every batch size (the r6 seen-set lane's per-insert constant).
#[test]
fn count_only_dedup_folds_without_survivor_collection() {
    let dir = TempDir::new("sink-dedup-count-only");
    let schema = schema();
    // Fresh ids exist in storage but the query does not bind them:
    // (account, amount) bindings collapse. Account 1 holds amounts
    // {5, 5, 7} -> {5, 7}; account 2 holds {5, 5, 5} -> {5}.
    let postings = vec![
        (1u64, 1u64, 5i64),
        (2, 1, 5),
        (3, 1, 7),
        (4, 2, 5),
        (5, 2, 5),
        (6, 2, 5),
    ];
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
        vec![],
    );
    let plan = two_node_plan(&schema, &normalized, &[0], &[1], &[0, 1]);
    // Count (nullary) + Sum over the GROUP slot (outer at the leaf):
    // no fold input is a leaf key word, so the count-only arm fires.
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
        // distinct_bindings = false: the dedup arm is mandatory.
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
        // Account 1: 2 distinct bindings — Count 2, Sum(account) 1×2.
        // Account 2: 1 distinct binding — Count 1, Sum(account) 2×1.
        assert_eq!(rows, vec![vec![1, 2, 2], vec![2, 1, 2]], "batch {batch}");
    }
}

/// An aggregate over a slot bound above the leaf folds as
/// value x count (i128/u128 — identical to count additions),
/// including the deterministic finalize-time overflow.
#[test]
fn constant_over_slot_folds_value_times_count() {
    let dir = TempDir::new("sink-constant-over");
    let schema = schema();
    // Sum(account) grouped by account: the over-slot is the group
    // slot itself — outer at the leaf. Account big enough that
    // value x count overflows u64 (caught at finalize) for one
    // group, stays in range for the other.
    let big = u64::MAX / 2;
    let mut postings = vec![];
    for id in 0..5u64 {
        postings.push((id, big, 1i64));
    }
    for id in 5..8u64 {
        postings.push((id, 7u64, 1i64));
    }
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = two_node_plan(&schema, &normalized, &[1], &[0, 2], &[0, 1, 2]);
    let finds =
        |plan: &ValidatedPlan| vec![var_spec(plan, 1), agg_spec(plan, FoldOp::Sum, 1, false)];
    // Overflow parity: the batch path and the per-row path yield the
    // same typed error (big x 5 > u64::MAX).
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
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: 1 })
            ),
            "{err:?}"
        );
    }
    // Value parity in range: drop the big account.
    let dir2 = TempDir::new("sink-constant-over-ok");
    let views = views_of(&dir2, &schema, &postings[5..], &[]);
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

/// The aggregate leaf batch folds bit-identically
/// to the scalar degenerate case at every batch size, including the
/// deterministic-overflow class at the i64 boundary.
#[test]
fn aggregate_leaf_batches_match_the_scalar_fold_at_the_boundary() {
    let dir = TempDir::new("sink-batch-boundary");
    let schema = schema();
    // Account 7 sums to exactly i64::MAX (in range); account 8
    // overflows deterministically.
    let postings = vec![
        (1u64, 7u64, i64::MAX),
        (2, 7, 1),
        (3, 7, -2),
        (4, 7, 1),
        (5, 8, i64::MAX),
        (6, 8, 1),
    ];
    let views = views_of(&dir, &schema, &postings, &[]);
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
        // Account 8's Sum overflows: the error is deterministic and
        // carries the find index, at every batch size.
        let err = sink.into_answers().unwrap_err();
        assert!(
            matches!(
                err,
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: 1 })
            ),
            "batch {batch}: {err:?}"
        );
    }
    // Remove the overflowing account: values identical at every size.
    let dir2 = TempDir::new("sink-batch-boundary-ok");
    let views = views_of(&dir2, &schema, &postings[..4], &[]);
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

/// PRD 18 slot plumbing: an interval variable as a GROUP key — the
/// group map keys on both words, so equal-start/different-end
/// intervals land in different groups.
#[test]
fn interval_group_keys_span_both_words() {
    let dir = TempDir::new("sink-interval-group");
    let schema = schema();
    // [5,9) x2, [5,7) x1: grouping by the interval yields counts 2, 1.
    let rows = vec![
        (1u64, 10u64, (5i64, 9i64)),
        (2, 11, (5, 9)),
        (3, 12, (5, 7)),
    ];
    let views = payroll_views_of(&dir, &schema, &rows);
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

/// The union regime's dedup key is HEAD-shaped, never rule-slot-shaped
/// (docs/architecture/40-execution.md § the rule loop): two rules with
/// DIFFERENT binding-slot layouts emitting equal head projections fold
/// once — a key over the full slot array could never absorb across
/// layouts, which is exactly why the representation is the head
/// projection.
#[test]
fn the_union_seen_set_keys_head_projections_across_rule_layouts() {
    use crate::exec::run::{Bindings, Sink};

    // Head: (group var, Sum(x), Count). Rule A: group at slot 0, x at
    // slot 1 (two slots). Rule B: x at slot 0, an unrelated existential
    // at slot 1, group at slot 2 (three slots).
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
    sink.reset(); // once per execution, never per rule

    // Rule A: (g = 7, x = 100) and (g = 7, x = 250).
    let mut bindings = Bindings::new(2);
    for x in [100u64, 250] {
        bindings.reset();
        bindings.set(0, 7);
        bindings.set(1, x);
        sink.emit(&bindings);
    }
    assert_eq!(sink.distinct_seen(), Some(2), "rule A seeds the union");

    // Rule B, re-aimed to its own layout: re-derives (g = 7, x = 100)
    // at different slots (absorbed) and adds (g = 7, x = 300).
    // Hand-written provenance: the head-projection key never reads the
    // shared-slot spans.
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

/// The DNF-derived union regime re-keys on the SHARED SLOT ARRAYS
/// (ruled 2026-07-23, R2): the disjuncts of one written rule share one
/// variable scope, so the `VarId`-ordered spans read the same binding
/// tuple through each clone's own layout — a cross-disjunct
/// re-derivation is absorbed, while distinct full bindings projecting
/// to EQUAL head rows all fold (the head-projection key would eat
/// them; the or-transparency law forbids exactly that —
/// `lean/Bumbledb/Exec/Dedup.lean: dnf_rekey_transparent`).
#[test]
fn the_dnf_union_seen_set_keys_shared_slot_arrays_across_clone_layouts() {
    use crate::exec::run::{Bindings, Sink};

    // Head: (g, Sum(x)) over the scope {g = v0, x = v1, e = v2} — e is
    // an existential outside the head. Clone A lays out g@0, x@1, e@2;
    // clone B lays out e@0, x@1, g@2 (the DP is free to reorder nodes,
    // never the shared scope).
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
    sink.reset(); // once per execution, never per rule

    // Clone A: (g = 7, x = 100, e = 5).
    let mut bindings = Bindings::new(3);
    bindings.reset();
    bindings.set(0, 7);
    bindings.set(1, 100);
    bindings.set(2, 5);
    sink.emit(&bindings);
    assert_eq!(sink.distinct_seen(), Some(1), "clone A seeds the union");

    // Clone B, re-aimed with ITS spans: the same full binding
    // (7, 100, 5) is absorbed across layouts, and (7, 100, 6) — equal
    // head row, distinct existential — folds.
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

/// The dense group table (finding 049) is answer-identical to the
/// open-domain map over the same emissions: two schema-proven radixes
/// (2 × 3 ordinals), every group inhabited out of order, ties of the
/// mixed-radix arithmetic to the map's hash keys checked value by
/// value.
#[test]
fn dense_group_tables_match_the_hashed_map_word_for_word() {
    use crate::exec::run::{Bindings, Sink};

    // Head: (a, b, Sum(x), Count) — a ∈ 0..2, b ∈ 0..3, x open.
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

    // Every (a, b) group, emitted out of ordinal order with duplicates
    // (the seen-set absorbs the repeat identically in both regimes).
    let mut bindings = Bindings::new(3);
    for (a, b, x) in [
        (1u64, 2u64, 10u64),
        (0, 0, 1),
        (1, 0, 5),
        (0, 2, 7),
        (1, 2, 30),
        (0, 1, 2),
        (1, 1, 4),
        (1, 2, 10), // the duplicate full binding
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
