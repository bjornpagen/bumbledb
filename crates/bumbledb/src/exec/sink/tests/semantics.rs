use super::*;
use crate::error::{Error, FindIndex};
use crate::exec::run::{Bindings, Flow, Sink};
use crate::ir::FoldOp;

#[test]
fn sum_distinguishes_bound_fresh_ids_and_collapses_unbound_ones() {
    let dir = TempDir::new("sink-footgun");
    let schema = schema();

    let postings = vec![(1u64, 7u64, 100i64), (2, 7, 100)];
    let views = views_of(&dir, &schema, &postings, &[]);

    let normalized_bound = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)])],
        vec![],
    );
    let plan = planned(&schema, &normalized_bound, &[0], &[1]);
    let finds = vec![var_spec(&plan, 1), agg_spec(&plan, FoldOp::Sum, 2, true)];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");
    assert_eq!(rows, vec![vec![7, i64_to_word(200)]]);

    let normalized_unbound = normalized(
        &schema,
        vec![occurrence(0, POSTING, &[(1, 0), (2, 1)])],
        vec![],
    );
    let plan = planned(&schema, &normalized_unbound, &[0], &[0]);
    let finds = vec![var_spec(&plan, 0), agg_spec(&plan, FoldOp::Sum, 1, true)];
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

    let normalized = normalized(
        &schema,
        vec![
            occurrence(0, POSTING, &[(0, 0), (1, 1), (2, 2)]),
            occurrence(1, TAG, &[(0, 0), (1, 3)]),
        ],
        vec![],
    );
    let plan = planned(&schema, &normalized, &[0, 1], &[1]);
    let finds = vec![var_spec(&plan, 1), agg_spec(&plan, FoldOp::Sum, 2, true)];
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
    let finds =
        |plan: &ValidatedPlan| vec![var_spec(plan, 1), agg_spec(plan, FoldOp::Sum, 2, true)];

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
        agg_spec(&plan, FoldOp::Sum, 1, true),
        FindSpec::Agg(AggSpec::Count),
    ];
    let rows = run_aggregate(&plan, &views[..1], finds).expect("rows");

    assert!(rows.is_empty());
}

#[test]
fn sum_is_order_independent_near_the_boundary() {

    let sum_find = FindSpec::Agg(AggSpec::Fold {
        op: FoldOp::Sum,
        slot: 0,
        width: 1,
        signed: true,
    });
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
                Error::Overflow(crate::error::OverflowKind::Aggregate { find: FindIndex(0) })
            ),
            "{err:?}"
        );
    }
}

#[test]
fn min_and_max_honor_logical_i64_order_across_the_sign_boundary() {
    let mut sink = AggregateSink::new(
        vec![
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Min,
                slot: 0,
                width: 1,
                signed: true,
            }),
            FindSpec::Agg(AggSpec::Fold {
                op: FoldOp::Max,
                slot: 0,
                width: 1,
                signed: true,
            }),
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
