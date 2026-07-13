use super::*;
use crate::error::Error;

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
            agg_spec(plan, FoldOp::Sum, Some(2), true),
            agg_spec(plan, FoldOp::Count, None, false),
            agg_spec(plan, FoldOp::Min, Some(2), true),
            agg_spec(plan, FoldOp::Max, Some(2), true),
        ]
    };
    // The fast path (elided) vs the per-row seen path, across sizes.
    let mut reference: Option<Vec<Vec<u64>>> = None;
    for (batch, distinct) in [(1usize, true), (7, true), (128, true), (128, false)] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count(), distinct);
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
        let mut rows = sink.into_rows().expect("in range");
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
            agg_spec(plan, FoldOp::Sum, Some(1), true),
            agg_spec(plan, FoldOp::Count, None, false),
        ]
    };
    for batch in [1usize, 2, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        // distinct_bindings = false: the dedup arm is mandatory.
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count(), false);
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_rows().expect("in range");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![1, i64_to_word(12), 2], vec![2, i64_to_word(5), 1],],
            "batch {batch}"
        );
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
    let finds = |plan: &ValidatedPlan| {
        vec![
            var_spec(plan, 1),
            agg_spec(plan, FoldOp::Sum, Some(1), false),
        ]
    };
    // Overflow parity: the batch path and the per-row path yield the
    // same typed error (big x 5 > u64::MAX).
    for distinct in [true, false] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count(), distinct);
        Executor::with_batch_size(&plan, 128)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let err = sink.into_rows().unwrap_err();
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
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count(), distinct);
        Executor::with_batch_size(&plan, 128)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let rows = sink.into_rows().expect("in range");
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
            agg_spec(plan, FoldOp::Sum, Some(2), true),
            agg_spec(plan, FoldOp::Count, None, false),
        ]
    };
    for batch in [1usize, 2, 7, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count(), true);
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
        let err = sink.into_rows().unwrap_err();
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
        let mut sink = AggregateSink::new(finds(&plan), plan.slot_count(), true);
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_rows().expect("in range");
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

/// PRD 18: `CountDistinct` collapses multiplicities per group — 3
/// postings, 2 distinct amounts ⇒ 2 — with per-group scoping (the
/// other group's identical amount counts separately), identically at
/// every batch size and in both dedup regimes.
#[test]
fn count_distinct_collapses_multiplicities_per_group() {
    let dir = TempDir::new("sink-count-distinct");
    let schema = schema();
    // Account 1: amounts {5, 5, 7} ⇒ 2 distinct; account 2: {5} ⇒ 1
    // (5 also appears in account 1 — scoping is per group).
    let postings = vec![(1u64, 1u64, 5i64), (2, 1, 5), (3, 1, 7), (4, 2, 5)];
    let views = views_of(&dir, &schema, &postings, &[]);
    // Fresh ids bound: every fact is a distinct binding.
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    let finds = |plan: &ValidatedPlan| {
        vec![
            var_spec(plan, 1),
            agg_spec(plan, FoldOp::CountDistinct, Some(2), true),
        ]
    };
    for batch in [1usize, 2, 128] {
        for distinct in [true, false] {
            let mut colts = colts_for(&plan, &views);
            let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
            let mut sink = AggregateSink::new(finds(&plan), plan.slot_count(), distinct);
            Executor::with_batch_size(&plan, batch)
                .execute(
                    &plan,
                    &mut colts,
                    &mut bindings,
                    &mut sink,
                    &mut crate::exec::run::NoopCounters,
                )
                .expect("execute");
            let mut rows = sink.into_rows().expect("rows");
            rows.sort_unstable();
            assert_eq!(
                rows,
                vec![vec![1, 2], vec![2, 1]],
                "batch {batch} distinct {distinct}"
            );
        }
    }
}

/// PRD 18: the elision fixture at the sink observables — a fresh-keyed
/// plan proves distinct bindings, the sink skips the binding seen-set
/// (`seen_elided`), and `CountDistinct`'s value sets still dedup
/// (`distinct_values_held` < bindings folded). The two sets are
/// different sets; elision never touches the value sets.
#[test]
fn elision_skips_the_seen_set_but_never_the_value_sets() {
    let dir = TempDir::new("sink-elision-count-distinct");
    let schema = schema();
    let postings = vec![(1u64, 7u64, 10i64), (2, 7, 10), (3, 7, 25)];
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    assert!(plan.distinct_bindings(), "fresh ids are bound");
    let finds = vec![
        var_spec(&plan, 1),
        agg_spec(&plan, FoldOp::CountDistinct, Some(2), true),
    ];
    let mut colts = colts_for(&plan, &views);
    let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
    let mut sink = AggregateSink::new(finds, plan.slot_count(), plan.distinct_bindings());
    Executor::new(&plan)
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        )
        .expect("execute");
    assert!(sink.seen_elided(), "the plan proved distinct bindings");
    assert_eq!(
        sink.distinct_values_held(),
        2,
        "3 bindings folded, 2 distinct amounts held — the value set dedups"
    );
    let rows = sink.into_rows().expect("rows");
    assert_eq!(rows, vec![vec![7, 2]]);
}

/// PRD 18: `ArgMax` restricts each group to the bindings attaining the
/// key's maximum — latest posting per account by fresh id — and
/// `ArgMin` mirrors it; a global (no group key) Arg works.
#[test]
fn arg_restriction_picks_the_extreme_binding_per_group() {
    let dir = TempDir::new("sink-arg-extreme");
    let schema = schema();
    // Account 1: ids 1..3 (latest = 3, amount 7); account 2: ids 4..5
    // (latest = 5, amount -1; earliest = 4, amount 9).
    let postings = vec![
        (1u64, 1u64, 5i64),
        (2, 1, 5),
        (3, 1, 7),
        (4, 2, 9),
        (5, 2, -1),
    ];
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    for batch in [1usize, 2, 128] {
        // ArgMax(key = id) carrying the amount, by account.
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let finds = vec![var_spec(&plan, 1), arg_spec(&plan, 2, 0, true)];
        let mut sink = AggregateSink::new(finds, plan.slot_count(), plan.distinct_bindings());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_rows().expect("rows");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![1, i64_to_word(7)], vec![2, i64_to_word(-1)]],
            "batch {batch}: single winner per group (fresh keys cannot tie)"
        );

        // ArgMin mirror.
        let mut colts = colts_for(&plan, &views);
        let finds = vec![var_spec(&plan, 1), arg_spec(&plan, 2, 0, false)];
        let mut sink = AggregateSink::new(finds, plan.slot_count(), plan.distinct_bindings());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_rows().expect("rows");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![1, i64_to_word(5)], vec![2, i64_to_word(9)]],
            "batch {batch}: ArgMin mirrors"
        );

        // Global group: one row for the whole input.
        let mut colts = colts_for(&plan, &views);
        let finds = vec![arg_spec(&plan, 2, 0, true)];
        let mut sink = AggregateSink::new(finds, plan.slot_count(), plan.distinct_bindings());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let rows = sink.into_rows().expect("rows");
        assert_eq!(
            rows,
            vec![vec![i64_to_word(-1)]],
            "batch {batch}: id 5 is globally latest"
        );
    }
}

/// PRD 18: ties are set-honest. Equal keys with different carries keep
/// every attaining row; equal keys whose bindings project EQUAL rows
/// collapse to one (row-level dedup — the answer is a set); and the
/// key variable itself may be carried (key-also-projected).
#[test]
fn arg_ties_keep_every_attaining_row_as_a_set() {
    let dir = TempDir::new("sink-arg-ties");
    let schema = schema();
    // Account 1: two postings tie at amount 9 (ids 2 and 3) — the
    // amount-keyed ArgMax carrying the id yields BOTH; carrying the
    // amount itself (equal rows) yields ONE.
    let postings = vec![(1u64, 1u64, 5i64), (2, 1, 9), (3, 1, 9), (4, 2, 3)];
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    for batch in [1usize, 2, 128] {
        // Different carries: both attaining rows.
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let finds = vec![var_spec(&plan, 1), arg_spec(&plan, 0, 2, true)];
        let mut sink = AggregateSink::new(finds, plan.slot_count(), plan.distinct_bindings());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_rows().expect("rows");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![1, 2], vec![1, 3], vec![2, 4]],
            "batch {batch}: a tie yields every attaining row"
        );

        // Equal projected rows: the tie collapses to one row —
        // key-also-projected (the carry IS the key variable).
        let mut colts = colts_for(&plan, &views);
        let finds = vec![var_spec(&plan, 1), arg_spec(&plan, 2, 2, true)];
        let mut sink = AggregateSink::new(finds, plan.slot_count(), plan.distinct_bindings());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_rows().expect("rows");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![1, i64_to_word(9)], vec![2, i64_to_word(3)]],
            "batch {batch}: equal-row ties collapse (set-honest)"
        );

        // Multi-carry coherence: carrying (id, amount) together — the
        // tied rows are (2, 9) and (3, 9), each projected whole from
        // one surviving binding (restrict-then-project).
        let mut colts = colts_for(&plan, &views);
        let finds = vec![
            var_spec(&plan, 1),
            arg_spec(&plan, 0, 2, true),
            arg_spec(&plan, 2, 2, true),
        ];
        let mut sink = AggregateSink::new(finds, plan.slot_count(), plan.distinct_bindings());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let mut rows = sink.into_rows().expect("rows");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![
                vec![1, 2, i64_to_word(9)],
                vec![1, 3, i64_to_word(9)],
                vec![2, 4, i64_to_word(3)],
            ],
            "batch {batch}: multi-carry rows are coherent per binding"
        );
    }
}

/// PRD 18: `CountDistinct` over intervals counts by VALUE identity
/// (both slot words hashed as one key) — equal intervals collapse,
/// overlapping-but-unequal intervals do not; and an interval variable
/// carries whole (two words) through an Arg restriction.
#[test]
fn count_distinct_and_arg_carry_treat_intervals_by_value() {
    let dir = TempDir::new("sink-interval-values");
    let schema = schema();
    // Emp 10: [5,9) twice (equal — one value) and [6,9) (overlaps
    // [5,9) but is a different value). Emp 11: [5,9) — per-group
    // scoping again.
    let rows = vec![
        (1u64, 10u64, (5i64, 9i64)),
        (2, 10, (5, 9)),
        (3, 10, (6, 9)),
        (4, 11, (5, 9)),
    ];
    let views = payroll_views_of(&dir, &schema, &rows);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, PAYROLL, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    for distinct in [true, false] {
        let finds = vec![
            var_spec(&plan, 1),
            agg_spec(&plan, FoldOp::CountDistinct, Some(2), false),
        ];
        let rows = run_aggregate_distinct(&plan, &views, finds, distinct).expect("rows");
        assert_eq!(
            rows,
            vec![vec![10, 2], vec![11, 1]],
            "distinct {distinct}: value identity, not overlap"
        );
    }

    // ArgMax(key = id) carrying the interval: the latest fact's two
    // words arrive intact in the emitted row.
    let finds = vec![var_spec(&plan, 1), arg_spec(&plan, 2, 0, true)];
    let got = run_aggregate(&plan, &views, finds).expect("rows");
    assert_eq!(
        got,
        vec![
            vec![10, i64_to_word(6), i64_to_word(9)],
            vec![11, i64_to_word(5), i64_to_word(9)],
        ],
        "interval carries span two words"
    );
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
        let finds = vec![
            var_spec(&plan, 2),
            agg_spec(&plan, FoldOp::Count, None, false),
        ];
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
            FindSpec::Agg {
                op: FoldOp::Sum,
                over_slot: Some(x),
                over_width: 1,
                signed: false,
            },
            FindSpec::Agg {
                op: FoldOp::Count,
                over_slot: None,
                over_width: 1,
                signed: false,
            },
        ]
    };
    let mut sink = AggregateSink::with_capacity_hint(&spec(0, 1), 2, false, true, 0);
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
    sink.aim(&spec(2, 0), 3);
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

    let rows = sink.into_rows().expect("in range");
    assert_eq!(
        rows,
        vec![vec![7, 650, 3]],
        "Sum folds {{100, 250, 300}} once each; Count counts the union"
    );
}
