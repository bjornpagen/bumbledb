use super::*;
use crate::error::Error;

/// PRD 02 (docs/perf/): the constant-group fast path — one group
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
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        residuals: vec![],
    };
    // Hand-factored GJ plan: n0 binds the account, n1 the
    // (id, amount) suffix — the stats shape, where the leaf's group
    // key is outer.
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(1)],
                }],
            },
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(0), VarId(2)],
                }],
            },
        ],
    };
    let sink_vars: BTreeSet<VarId> = [VarId(0), VarId(1), VarId(2)].into();
    let plan =
        validate(&plan, &normalized, &schema, vec![0; 2], &sink_vars).expect("valid plan");
    let finds = |plan: &ValidatedPlan| {
        vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(1)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
            FindSpec::Agg {
                op: AggOp::Count,
                over_slot: None,
                signed: false,
            },
            FindSpec::Agg {
                op: AggOp::Min,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
            FindSpec::Agg {
                op: AggOp::Max,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
        ]
    };
    // The fast path (elided) vs the per-row seen path, across sizes.
    let mut reference: Option<Vec<Vec<u64>>> = None;
    for (batch, distinct) in [(1usize, true), (7, true), (128, true), (128, false)] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), distinct);
        Executor::with_batch_size(&plan, batch).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
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

/// PRD 02: the dedup-then-gather arm — duplicate full bindings
/// collapse before the fold, identically at every batch size, with
/// the group probe still hoisted.
#[test]
fn dedup_constant_group_collapses_duplicates_before_folding() {
    let dir = TempDir::new("sink-dedup-constant");
    let schema = schema();
    // Serials exist in storage but the query does not bind them:
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
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
        residuals: vec![],
    };
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(0)],
                }],
            },
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(1)],
                }],
            },
        ],
    };
    let sink_vars: BTreeSet<VarId> = [VarId(0), VarId(1)].into();
    let plan =
        validate(&plan, &normalized, &schema, vec![0; 2], &sink_vars).expect("valid plan");
    let finds = |plan: &ValidatedPlan| {
        vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(0)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(1))),
                signed: true,
            },
            FindSpec::Agg {
                op: AggOp::Count,
                over_slot: None,
                signed: false,
            },
        ]
    };
    for batch in [1usize, 2, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        // distinct_bindings = false: the dedup arm is mandatory.
        let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), false);
        Executor::with_batch_size(&plan, batch).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
        let mut rows = sink.into_rows().expect("in range");
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![1, i64_to_word(12), 2], vec![2, i64_to_word(5), 1],],
            "batch {batch}"
        );
    }
}

/// PRD 02: an aggregate over a slot bound above the leaf folds as
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
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        residuals: vec![],
    };
    let plan = crate::plan::fj::FjPlan {
        nodes: vec![
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(1)],
                }],
            },
            crate::plan::fj::Node {
                subatoms: vec![crate::plan::fj::Subatom {
                    occ: OccId(0),
                    vars: vec![VarId(0), VarId(2)],
                }],
            },
        ],
    };
    let sink_vars: BTreeSet<VarId> = [VarId(0), VarId(1), VarId(2)].into();
    let plan =
        validate(&plan, &normalized, &schema, vec![0; 2], &sink_vars).expect("valid plan");
    let finds = |plan: &ValidatedPlan| {
        vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(1)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(1))),
                signed: false,
            },
        ]
    };
    // Overflow parity: the batch path and the per-row path yield the
    // same typed error (big x 5 > u64::MAX).
    for distinct in [true, false] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), distinct);
        Executor::with_batch_size(&plan, 128).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
        let err = sink.into_rows().unwrap_err();
        assert!(matches!(err, Error::Overflow { find: 1 }), "{err:?}");
    }
    // Value parity in range: drop the big account.
    let dir2 = TempDir::new("sink-constant-over-ok");
    let views = views_of(&dir2, &schema, &postings[5..], &[]);
    for distinct in [true, false] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), distinct);
        Executor::with_batch_size(&plan, 128).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
        let rows = sink.into_rows().expect("in range");
        assert_eq!(rows, vec![vec![7, 21]], "distinct {distinct}");
    }
}

/// PRD 01 (docs/perf/): the aggregate leaf batch folds bit-identically
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
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0], &[1]);
    let finds = |plan: &ValidatedPlan| {
        vec![
            FindSpec::Var {
                slot: plan.slot_of(VarId(1)),
            },
            FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(plan.slot_of(VarId(2))),
                signed: true,
            },
            FindSpec::Agg {
                op: AggOp::Count,
                over_slot: None,
                signed: false,
            },
        ]
    };
    for batch in [1usize, 2, 7, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), true);
        Executor::with_batch_size(&plan, batch).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
        // Account 8's Sum overflows: the error is deterministic and
        // carries the find index, at every batch size.
        let err = sink.into_rows().unwrap_err();
        assert!(
            matches!(err, Error::Overflow { find: 1 }),
            "batch {batch}: {err:?}"
        );
    }
    // Remove the overflowing account: values identical at every size.
    let dir2 = TempDir::new("sink-batch-boundary-ok");
    let views = views_of(&dir2, &schema, &postings[..4], &[]);
    let mut reference: Option<Vec<Vec<u64>>> = None;
    for batch in [1usize, 2, 7, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
        let mut sink = AggregateSink::new(finds(&plan), plan.slots().len(), true);
        Executor::with_batch_size(&plan, batch).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut crate::exec::run::NoopCounters,
        );
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
