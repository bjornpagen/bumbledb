use super::*;

#[test]
fn projection_scan_filters_residuals_like_the_oracle() {
    let schema = schema();

    let postings: Vec<(u64, u64, i64)> = (0..60)
        .map(|i| (i, i % 5, i64::try_from(i * 7 % 23).expect("small")))
        .collect();
    let views = views_of(&schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![
            occurrence(0, POSTING, &[(1, 0), (2, 1)]),
            occurrence(1, POSTING, &[(1, 0), (2, 2)]),
        ],
        vec![FilterPredicate::FieldsCompare {
            left: OperandAddr::from(VarId(1)),
            right: OperandAddr::from(VarId(2)),
            op: crate::ir::WordCmp::Lt,
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

#[test]
fn pinned_leaf_skips_preserve_d2() {
    let schema = schema();

    let postings: Vec<(u64, u64, i64)> = (0..40)
        .map(|i| (i, i % 4, i64::try_from(i).expect("small")))
        .collect();
    let tags: Vec<(u64, u64)> = (0..40).map(|i| (i, 900 + i)).collect();
    let views = views_of(&schema, &postings, &tags);

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
    let schema = schema();

    let postings = vec![(1u64, 7u64, 100i64)];
    let tags: Vec<(u64, u64)> = (0..50).map(|t| (1, t)).collect();
    let views = views_of(&schema, &postings, &tags);

    let normalized = normalized(
        &schema,
        vec![
            occurrence(0, POSTING, &[(0, 0), (1, 1)]),
            occurrence(1, TAG, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );

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

#[test]
fn interval_projection_carries_both_slot_words() {
    let schema = schema();
    let rows = vec![
        (1u64, 10u64, (5i64, 9i64)),
        (2, 10, (-3, 4)),
        (3, 11, (5, 9)),
    ];
    let views = payroll_views_of(&schema, &rows);

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
