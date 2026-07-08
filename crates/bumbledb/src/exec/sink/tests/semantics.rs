use super::*;
use crate::error::Error;
use crate::exec::run::{Bindings, Flow, Sink};

#[test]
fn sum_distinguishes_bound_serials_and_collapses_unbound_ones() {
    let dir = TempDir::new("sink-footgun");
    let schema = schema();
    // Two postings of amount 100 to account 7.
    let postings = vec![(1u64, 7u64, 100i64), (2, 7, 100)];
    let views = views_of(&dir, &schema, &postings, &[]);

    // Serials bound: two distinct bindings -> Sum = 200.
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0], &[1]);
    let finds = vec![
        FindSpec::Var {
            slot: plan.slot_of(VarId(1)),
        },
        FindSpec::Agg {
            op: AggOp::Sum,
            over_slot: Some(plan.slot_of(VarId(2))),
            signed: true,
        },
    ];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
    assert_eq!(rows, vec![vec![7, i64_to_word(200)]]);

    // Serials unbound: the two facts collapse to one binding -> 100.
    // This documents the set-semantics footgun deliberately.
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0], &[0]);
    let finds = vec![
        FindSpec::Var {
            slot: plan.slot_of(VarId(0)),
        },
        FindSpec::Agg {
            op: AggOp::Sum,
            over_slot: Some(plan.slot_of(VarId(1))),
            signed: true,
        },
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
    let normalized = NormalizedQuery {
        occurrences: vec![
            occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)]),
            occurrence(1, TAG, &[(0, 0), (1, 3)]),
        ],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0, 1], &[1]);
    let finds = vec![
        FindSpec::Var {
            slot: plan.slot_of(VarId(1)),
        },
        FindSpec::Agg {
            op: AggOp::Sum,
            over_slot: Some(plan.slot_of(VarId(2))),
            signed: true,
        },
    ];
    let rows = run_aggregate(&plan, &views, finds).expect("rows");
    assert_eq!(rows, vec![vec![7, i64_to_word(300)]]);
}

#[test]
fn distinct_flag_elision_matches_the_seen_set_path() {
    let dir = TempDir::new("sink-elision");
    let schema = schema();
    let postings = vec![(1u64, 7u64, 10i64), (2, 7, 20), (3, 8, 30)];
    let views = views_of(&dir, &schema, &postings, &[]);
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0], &[1]);
    assert!(plan.distinct_bindings(), "serials are bound");
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
        ]
    };

    // Elided path (as the plan proves) vs forced seen-set path.
    let mut colts = colts_for(&plan, &views);
    let mut bindings = crate::exec::run::Bindings::new(plan.slots().len());
    let mut elided = AggregateSink::new(finds(&plan), plan.slots().len(), true);
    Executor::new(&plan).execute(
        &plan,
        &mut colts,
        &mut bindings,
        &mut elided,
        &mut crate::exec::run::NoopCounters,
    );
    let mut colts = colts_for(&plan, &views);
    let mut checked = AggregateSink::new(finds(&plan), plan.slots().len(), false);
    Executor::new(&plan).execute(
        &plan,
        &mut colts,
        &mut bindings,
        &mut checked,
        &mut crate::exec::run::NoopCounters,
    );
    let mut a = elided.into_rows().expect("rows");
    let mut b = checked.into_rows().expect("rows");
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
    let normalized = NormalizedQuery {
        occurrences: vec![occurrence(0, POSTING, &[(0, 0), (2, 1)])],
        residuals: vec![],
    };
    let plan = planned(&normalized, &schema, &[0], &[]);
    let finds = vec![
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
    ];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
    // The empty set — not a [NULL] or [0] row (documented divergence
    // from SQL's ungrouped-aggregate behavior).
    assert!(rows.is_empty());
}

#[test]
fn sum_is_order_independent_near_the_boundary() {
    // {i64::MAX, 1, -2} sums to MAX-1 under any fold order thanks to
    // i128 accumulation; {MAX, 1} overflows deterministically.
    for order in [[0usize, 1, 2], [2, 1, 0], [1, 2, 0]] {
        let values = [i64::MAX, 1, -2];
        let mut sink = AggregateSink::new(
            vec![FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(0),
                signed: true,
            }],
            1,
            true,
        );
        let mut bindings = Bindings::new(1);
        bindings.reset();
        for idx in order {
            bindings.set(0, i64_to_word(values[idx]));
            assert_eq!(sink.emit(&bindings), Flow::Continue);
        }
        let rows = sink.into_rows().expect("in range");
        assert_eq!(rows, vec![vec![i64_to_word(i64::MAX - 1)]]);
    }
    for order in [[0usize, 1], [1, 0]] {
        let values = [i64::MAX, 1];
        let mut sink = AggregateSink::new(
            vec![FindSpec::Agg {
                op: AggOp::Sum,
                over_slot: Some(0),
                signed: true,
            }],
            1,
            true,
        );
        let mut bindings = Bindings::new(1);
        bindings.reset();
        for idx in order {
            bindings.set(0, i64_to_word(values[idx]));
            sink.emit(&bindings);
        }
        let err = sink.into_rows().unwrap_err();
        assert!(matches!(err, Error::Overflow { find: 0 }), "{err:?}");
    }
}

#[test]
fn min_and_max_honor_logical_i64_order_across_the_sign_boundary() {
    let mut sink = AggregateSink::new(
        vec![
            FindSpec::Agg {
                op: AggOp::Min,
                over_slot: Some(0),
                signed: true,
            },
            FindSpec::Agg {
                op: AggOp::Max,
                over_slot: Some(0),
                signed: true,
            },
        ],
        1,
        true,
    );
    let mut bindings = Bindings::new(1);
    bindings.reset();
    for v in [-5i64, 3, -100, 42, 0] {
        bindings.set(0, i64_to_word(v));
        sink.emit(&bindings);
    }
    let rows = sink.into_rows().expect("rows");
    assert_eq!(rows, vec![vec![i64_to_word(-100), i64_to_word(42)]]);
}
