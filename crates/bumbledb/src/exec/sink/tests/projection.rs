use super::*;

/// The projection scan with leaf residuals (the spread
/// shape) — positions filter through the residual, insert into the
/// seen-set, and match the brute-force pair set exactly.
#[test]
fn projection_scan_filters_residuals_like_the_oracle() {
    let dir = TempDir::new("sink-projection-scan");
    let schema = schema();
    // Pairs within an account: Q(lo, hi) :- Posting(acct, lo),
    // Posting(acct, hi), lo < hi.
    let postings: Vec<(u64, u64, i64)> = (0..60)
        .map(|i| (i, i % 5, i64::try_from(i * 7 % 23).expect("small")))
        .collect();
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![
            occurrence(0, POSTING, &[(1, 0), (2, 1)]),
            occurrence(1, POSTING, &[(1, 0), (2, 2)]),
        ],
        vec![crate::ir::normalize::PlacedComparison {
            op: crate::ir::CmpOp::Lt,
            lhs: VarId(1),
            rhs: VarId(2),
        }],
    );
    let plan = planned(&schema, &normalized, &[0, 1], &[1, 2]);
    let views2 = vec![views[0].clone(), views[0].clone()];
    let mut expected = BTreeSet::new();
    for (_, ka, va) in &postings {
        for (_, kb, vb) in &postings {
            if ka == kb && va < vb {
                expected.insert(vec![i64_to_word(*va), i64_to_word(*vb)]);
            }
        }
    }
    for batch in [1usize, 128] {
        let mut colts = colts_for(&plan, &views2);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(1)), plan.slot_of(VarId(2))]);
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let got: BTreeSet<Vec<u64>> = sink.answers().map(<[u64]>::to_vec).collect();
        assert_eq!(got, expected, "batch {batch}");
    }
}

/// The pinned-leaf elision preserves D2 exactly — a fanout-1
/// leaf that binds nothing sink-relevant skips per parent element,
/// and the parent's absorption still runs, at every batch size.
#[test]
fn pinned_leaf_skips_preserve_d2() {
    let dir = TempDir::new("sink-pinned-d2");
    let schema = schema();
    // One tag per posting: the tag leaf pins to Cursor::Row.
    let postings: Vec<(u64, u64, i64)> = (0..40)
        .map(|i| (i, i % 4, i64::try_from(i).expect("small")))
        .collect();
    let tags: Vec<(u64, u64)> = (0..40).map(|i| (i, 900 + i)).collect();
    let views = views_of(&dir, &schema, &postings, &tags);
    // Q(account) :- Posting(id=p, account=a), PostingTag(posting=p, tag=t).
    let normalized = normalized(
        &schema,
        vec![
            occurrence(0, POSTING, &[(0, 0), (1, 1)]),
            occurrence(1, TAG, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0, 1], &[1]);
    for batch in [1usize, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(1))]);
        let mut counters = SkipCounter::default();
        Executor::with_batch_size(&plan, batch)
            .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
            .expect("execute");
        let mut rows: Vec<Vec<u64>> = sink.answers().map(<[u64]>::to_vec).collect();
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec![vec![0], vec![1], vec![2], vec![3]],
            "batch {batch}"
        );
        assert!(counters.skips > 0, "batch {batch}: pinned leaves skip");
    }
}

#[test]
fn duplicate_witness_projection_dedups_and_skips_suffixes() {
    let dir = TempDir::new("sink-projection-skip");
    let schema = schema();
    // One posting, many tags: projecting only the account, the tag
    // suffix multiplies witnesses without changing the projection.
    // The tag node is the LEAF and is not sink-relevant: at batch
    // size 128 all 50 tags arrive in one leaf batch and the batch
    // emit must stop at the first row (`stop_on_skip`) — the
    // same skip the recursive path signaled per-row.
    let postings = vec![(1u64, 7u64, 100i64)];
    let tags: Vec<(u64, u64)> = (0..50).map(|t| (1, t)).collect();
    let views = views_of(&dir, &schema, &postings, &tags);
    // Q(account) :- Posting(id=p, account=a), PostingTag(posting=p).
    let normalized = normalized(
        &schema,
        vec![
            occurrence(0, POSTING, &[(0, 0), (1, 1)]),
            occurrence(1, TAG, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    // Sink-relevant vars: just the account (var 1).
    let plan = planned(&schema, &normalized, &[0, 1], &[1]);
    for batch in [1usize, 2, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(1))]);
        let mut counters = SkipCounter::default();
        Executor::with_batch_size(&plan, batch)
            .execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters)
            .expect("execute");

        let rows: Vec<Vec<u64>> = sink.answers().map(<[u64]>::to_vec).collect();
        assert_eq!(rows, vec![vec![7]], "batch {batch}");
        assert!(
            counters.skips > 0,
            "batch {batch}: the tag suffix must be skipped after the first witness"
        );
    }
}

/// PRD 18: an interval find flows through the projection sink as its
/// two-slot span — the word-level `slots` expansion — and the emitted
/// word rows carry both bounds of every stored fact.
#[test]
fn interval_projection_carries_both_slot_words() {
    let dir = TempDir::new("sink-projection-interval");
    let schema = schema();
    let rows = vec![
        (1u64, 10u64, (5i64, 9i64)),
        (2, 10, (-3, 4)),
        (3, 11, (5, 9)),
    ];
    let views = payroll_views_of(&dir, &schema, &rows);
    // Q(emp, during) :- Payroll(id, emp, during).
    let normalized = normalized(
        &schema,
        vec![occurrence(0, PAYROLL, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1, 2]);
    assert_eq!(plan.width_of(VarId(2)), 2, "interval vars are two slots");
    let expected: BTreeSet<Vec<u64>> = rows
        .iter()
        .map(|(_, emp, (start, end))| vec![*emp, i64_to_word(*start), i64_to_word(*end)])
        .collect();
    for batch in [1usize, 128] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
        // The word-level expansion make_sink performs in production.
        let during = plan.slot_of(VarId(2));
        let mut sink = ProjectionSink::new(vec![plan.slot_of(VarId(1)), during, during + 1]);
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut crate::exec::run::NoopCounters,
            )
            .expect("execute");
        let got: BTreeSet<Vec<u64>> = sink.answers().map(<[u64]>::to_vec).collect();
        assert_eq!(got, expected, "batch {batch}");
    }
}
