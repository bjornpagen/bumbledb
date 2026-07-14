use super::*;
use crate::error::Error;
use crate::exec::run::{Bindings, Flow, Sink};

#[test]
fn sum_distinguishes_bound_fresh_ids_and_collapses_unbound_ones() {
    let dir = TempDir::new("sink-footgun");
    let schema = schema();
    // Two postings of amount 100 to account 7.
    let postings = vec![(1u64, 7u64, 100i64), (2, 7, 100)];
    let views = views_of(&dir, &schema, &postings, &[]);

    // Fresh ids bound: two distinct bindings -> Sum = 200.
    let normalized_bound = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized_bound, &[0], &[1]);
    let finds = vec![
        var_spec(&plan, 1),
        agg_spec(&plan, FoldOp::Sum, Some(2), true),
    ];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
    assert_eq!(rows, vec![vec![7, i64_to_word(200)]]);

    // Fresh ids unbound: the two facts collapse to one binding -> 100.
    // This documents the set-semantics footgun deliberately.
    let normalized_unbound = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
        vec![],
    );
    let plan = planned(&schema, &normalized_unbound, &[0], &[0]);
    let finds = vec![
        var_spec(&plan, 0),
        agg_spec(&plan, FoldOp::Sum, Some(1), true),
    ];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
    assert_eq!(rows, vec![vec![7, i64_to_word(100)]]);
}

#[test]
fn joining_a_three_tag_relation_triples_the_sum() {
    let dir = TempDir::new("sink-tag-triple");
    let schema = schema();
    let postings = vec![(1u64, 7u64, 100i64)];
    let tags = vec![(1u64, 10u64), (1, 11), (1, 12)];
    let views = views_of(&dir, &schema, &postings, &tags);
    // Sum(amount) by account joined with tags: the 3 tag bindings
    // multiply the binding set — exactly the documented footgun.
    let normalized = normalized(
        &schema,
        vec![
            occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)]),
            occurrence(1, TAG, &[(0, 0), (1, 3)]),
        ],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0, 1], &[1]);
    let finds = vec![
        var_spec(&plan, 1),
        agg_spec(&plan, FoldOp::Sum, Some(2), true),
    ];
    let rows = run_aggregate(&plan, &views, finds).expect("rows");
    assert_eq!(rows, vec![vec![7, i64_to_word(300)]]);
}

#[test]
fn witnessed_elision_matches_the_seen_set_path() {
    let dir = TempDir::new("sink-elision");
    let schema = schema();
    let postings = vec![(1u64, 7u64, 10i64), (2, 7, 20), (3, 8, 30)];
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[1]);
    assert!(plan.distinct_witness().is_some(), "fresh ids are bound");
    let finds = |plan: &ValidatedPlan| {
        vec![
            var_spec(plan, 1),
            agg_spec(plan, FoldOp::Sum, Some(2), true),
        ]
    };

    // Elided path (as the plan proves) vs forced seen-set path.
    let mut colts = colts_for(&plan, &views);
    let mut bindings = crate::exec::run::Bindings::new(plan.slot_count());
    let mut elided = AggregateSink::new_distinct(
        finds(&plan),
        plan.slot_count(),
        plan.distinct_witness()
            .expect("fresh ids prove distinctness"),
    );
    Executor::new(&plan)
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut elided,
            &mut crate::exec::run::NoopCounters,
        )
        .expect("execute");
    let mut colts = colts_for(&plan, &views);
    let mut checked = AggregateSink::new(finds(&plan), plan.slot_count());
    Executor::new(&plan)
        .execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut checked,
            &mut crate::exec::run::NoopCounters,
        )
        .expect("execute");
    let mut a = elided.into_answers().expect("rows");
    let mut b = checked.into_answers().expect("rows");
    a.sort_unstable();
    b.sort_unstable();
    assert_eq!(a, b);
    assert_eq!(a.len(), 2);
}

#[test]
fn global_aggregate_over_empty_input_yields_zero_rows() {
    let dir = TempDir::new("sink-empty-global");
    let schema = schema();
    let views = views_of(&dir, &schema, &[], &[]);
    let normalized = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (2, 1)])],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0], &[]);
    let finds = vec![
        agg_spec(&plan, FoldOp::Sum, Some(1), true),
        agg_spec(&plan, FoldOp::Count, None, false),
    ];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
    // The empty set — not a [NULL] or [0] row (documented divergence
    // from SQL's ungrouped-aggregate behavior).
    assert!(rows.is_empty());

    // Same for the Arg regime: no bindings, no groups, no rows.
    let finds = vec![arg_spec(&plan, 1, 0, true)];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
    assert!(rows.is_empty());
}

#[test]
fn sum_is_order_independent_near_the_boundary() {
    // {i64::MAX, 1, -2} sums to MAX-1 under any fold order thanks to
    // i128 accumulation; {MAX, 1} overflows deterministically.
    let sum_find = FindSpec::Agg {
        op: FoldOp::Sum,
        over_slot: Some(0),
        over_width: 1,
        signed: true,
    };
    for order in [[0usize, 1, 2], [2, 1, 0], [1, 2, 0]] {
        let values = [i64::MAX, 1, -2];
        let mut sink = AggregateSink::new(vec![sum_find], 1);
        let mut bindings = Bindings::new(1);
        bindings.reset();
        for idx in order {
            bindings.set(0, i64_to_word(values[idx]));
            assert_eq!(sink.emit(&bindings), Flow::Continue);
        }
        let rows = sink.into_answers().expect("in range");
        assert_eq!(rows, vec![vec![i64_to_word(i64::MAX - 1)]]);
    }
    for order in [[0usize, 1], [1, 0]] {
        let values = [i64::MAX, 1];
        let mut sink = AggregateSink::new(vec![sum_find], 1);
        let mut bindings = Bindings::new(1);
        bindings.reset();
        for idx in order {
            bindings.set(0, i64_to_word(values[idx]));
            sink.emit(&bindings);
        }
        let err = sink.into_answers().unwrap_err();
        assert!(
            matches!(
                err,
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: 0 })
            ),
            "{err:?}"
        );
    }
}

#[test]
fn min_and_max_honor_logical_i64_order_across_the_sign_boundary() {
    let mut sink = AggregateSink::new(
        vec![
            FindSpec::Agg {
                op: FoldOp::Min,
                over_slot: Some(0),
                over_width: 1,
                signed: true,
            },
            FindSpec::Agg {
                op: FoldOp::Max,
                over_slot: Some(0),
                over_width: 1,
                signed: true,
            },
        ],
        1,
    );
    let mut bindings = Bindings::new(1);
    bindings.reset();
    for v in [-5i64, 3, -100, 42, 0] {
        bindings.set(0, i64_to_word(v));
        sink.emit(&bindings);
    }
    let rows = sink.into_answers().expect("rows");
    assert_eq!(rows, vec![vec![i64_to_word(-100), i64_to_word(42)]]);
}

/// PRD 18: Arg keys compare by encoded word for I64 too — the
/// sign-flipped biased form is order-preserving, so a negative key
/// never beats a positive one under `ArgMax`.
#[test]
fn arg_keys_honor_logical_i64_order_across_the_sign_boundary() {
    // Two slots: slot 0 = the I64 key, slot 1 = a U64 carry.
    let finds = vec![FindSpec::Arg {
        slot: 1,
        width: 1,
        key_slot: 0,
        max: true,
    }];
    let mut sink = AggregateSink::new(finds, 2);
    let mut bindings = Bindings::new(2);
    bindings.reset();
    for (key, carry) in [(-5i64, 10u64), (3, 20), (-100, 30), (0, 40)] {
        bindings.set(0, i64_to_word(key));
        bindings.set(1, carry);
        sink.emit(&bindings);
    }
    let rows = sink.into_answers().expect("rows");
    assert_eq!(rows, vec![vec![20]], "key 3 is the logical maximum");
}
