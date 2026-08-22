use super::*;
use crate::image::view::{Const, FilterPredicate};
use crate::ir::WordCmp;

fn assert_batch_equality(
    plan: &ValidatedPlan,
    views: &[Arc<crate::image::RelationImage>],
    reference: &BTreeSet<Vec<u64>>,
) {
    for batch in [1usize, 2, 7, 64, 256] {
        assert_eq!(
            &run_batched(plan, views, batch),
            reference,
            "batch size {batch} must match"
        );
    }
}

#[test]
fn postings_without_tag_ignores_tag_multiplicity() {
    let dir = TempDir::new("run-anti-multiplicity");
    let schema = schema(2);

    let postings: Vec<(u64, u64)> = (0..10).map(|i| (i, 100 + i)).collect();
    let tags = vec![(1u64, 7u64), (2, 7), (2, 8), (3, 7), (3, 8), (3, 9)];
    let views = views_of(&dir, &schema, &[postings.clone(), tags.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            negated(1, 1, &[(0, 0)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0]);
    let results = run(&plan, &views);

    let mut expected = BTreeSet::new();
    for (p, a) in &postings {
        if !tags.iter().any(|(tp, _)| tp == p) {
            let mut row = vec![0u64; 2];
            row[plan.slot_of(VarId(0))] = *p;
            row[plan.slot_of(VarId(1))] = *a;
            expected.insert(row);
        }
    }
    assert_eq!(results, expected);
    assert_eq!(results.len(), 7, "ids 1, 2, 3 rejected exactly once each");
    assert_batch_equality(&plan, &views, &expected);
}

#[test]
fn negated_atom_with_literal_binding_rejects_only_matching_kind() {
    let dir = TempDir::new("run-anti-literal");
    let schema = schema(2);
    let r: Vec<(u64, u64)> = (0..6).map(|i| (i, i * 10)).collect();

    let s = vec![(1u64, 7u64), (2, 8), (3, 7), (3, 8)];
    let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);

    let mut neg = negated(1, 1, &[(0, 0)]);
    neg.filters = vec![FilterPredicate::Compare {
        field: FieldId(1).into(),
        op: WordCmp::Eq,
        value: Const::Word(7),
    }];
    let normalized = normalized(vec![occurrence(0, 0, &[(0, 0), (1, 1)]), neg], vec![]);
    let plan = planned(&normalized, &schema, &[0]);

    assert!(plan.occurrences()[1].selections.is_empty());
    assert_eq!(plan.occurrences()[1].filters.len(), 1);

    let results = run(&plan, &views);
    let mut expected = BTreeSet::new();
    for (x, a) in &r {
        if !s.iter().any(|(sx, sk)| sx == x && *sk == 7) {
            let mut row = vec![0u64; 2];
            row[plan.slot_of(VarId(0))] = *x;
            row[plan.slot_of(VarId(1))] = *a;
            expected.insert(row);
        }
    }
    assert_eq!(results, expected);
    assert_eq!(results.len(), 4, "x = 2 survives: (2, 8) is not kind 7");
    assert_batch_equality(&plan, &views, &expected);
}

#[test]
fn zero_binding_negated_atom_is_an_emptiness_gate() {
    let schema = schema(2);
    let r = vec![(1u64, 2u64), (3, 4), (5, 6)];
    for (gate_rows, expect_all) in [(vec![(9u64, 9u64)], false), (vec![], true)] {
        let dir = TempDir::new(&format!("run-anti-gate-{expect_all}"));
        let views = views_of(&dir, &schema, &[r.clone(), gate_rows]);

        let normalized = normalized(
            vec![occurrence(0, 0, &[(0, 0), (1, 1)]), negated(1, 1, &[])],
            vec![],
        );
        let plan = planned(&normalized, &schema, &[0]);
        let results = run(&plan, &views);
        let expected: BTreeSet<Vec<u64>> = if expect_all {
            r.iter()
                .map(|(x, a)| {
                    let mut row = vec![0u64; 2];
                    row[plan.slot_of(VarId(0))] = *x;
                    row[plan.slot_of(VarId(1))] = *a;
                    row
                })
                .collect()
        } else {
            BTreeSet::new()
        };
        assert_eq!(results, expected, "gate passthrough = {expect_all}");
        assert_batch_equality(&plan, &views, &expected);
    }
}

/// An anti-probe attached to a middle node of the pipeline: the chain R0(x, y),
/// R1(y, z) with ¬R2(y) rejects at the node that binds y — survivors compact
/// before any deeper probing.
#[test]
fn negation_at_a_middle_node_compacts_before_descending() {
    let dir = TempDir::new("run-anti-middle");
    let schema = schema(3);
    let r: Vec<(u64, u64)> = (0..12).map(|i| (i % 5, i % 4)).collect();
    let s: Vec<(u64, u64)> = (0..10).map(|i| (i % 4, i % 3)).collect();
    let blocked = vec![(1u64, 0u64), (3, 0)];
    let views = views_of(&dir, &schema, &[r.clone(), s.clone(), blocked.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            negated(2, 2, &[(0, 1)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let results = run(&plan, &views);

    let mut expected = BTreeSet::new();
    for (rx, ry) in &r {
        for (sy, sz) in &s {
            if ry == sy && !blocked.iter().any(|(by, _)| by == ry) {
                let mut row = vec![0u64; 3];
                row[plan.slot_of(VarId(0))] = *rx;
                row[plan.slot_of(VarId(1))] = *ry;
                row[plan.slot_of(VarId(2))] = *sz;
                expected.insert(row);
            }
        }
    }
    assert_eq!(results, expected);
    assert!(!expected.is_empty());
    assert_batch_equality(&plan, &views, &expected);
}

#[test]
fn negation_over_variables_bound_at_different_nodes() {
    let dir = TempDir::new("run-anti-split");
    let schema = schema(3);
    let r: Vec<(u64, u64)> = (0..9).map(|i| (i % 3, i % 4)).collect();
    let s: Vec<(u64, u64)> = (0..12).map(|i| (i % 4, i % 5)).collect();
    let blocked = vec![(0u64, 1u64), (2, 3), (1, 0)];
    let views = views_of(&dir, &schema, &[r.clone(), s.clone(), blocked.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 1), (1, 2)]),
            negated(2, 2, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let plan = planned(&normalized, &schema, &[0, 1]);
    let results = run(&plan, &views);

    let mut expected = BTreeSet::new();
    for (rx, ry) in &r {
        for (sy, sz) in &s {
            if ry == sy && !blocked.iter().any(|(bx, bz)| bx == rx && bz == sz) {
                let mut row = vec![0u64; 3];
                row[plan.slot_of(VarId(0))] = *rx;
                row[plan.slot_of(VarId(1))] = *ry;
                row[plan.slot_of(VarId(2))] = *sz;
                expected.insert(row);
            }
        }
    }
    assert_eq!(results, expected);
    assert!(!expected.is_empty());
    assert_batch_equality(&plan, &views, &expected);
}

#[test]
fn negation_under_an_aggregate_excludes_rejected_bindings() {
    use crate::exec::sink::{AggSpec, AggregateSink, FindSpec, FoldOp};

    let dir = TempDir::new("run-anti-aggregate");
    let schema = schema(2);
    let postings: Vec<(u64, u64)> = (0..10).map(|i| (i, 100 + i)).collect();
    let tags = vec![(1u64, 7u64), (2, 7), (2, 8), (3, 7), (3, 8), (3, 9)];
    let views = views_of(&dir, &schema, &[postings.clone(), tags.clone()]);

    let normalized = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            negated(1, 1, &[(0, 0)]),
        ],
        vec![],
    );
    let plan = planned_with_sinks(&normalized, &schema, &[0], &all_vars(&normalized));
    let finds = vec![
        FindSpec::Agg(AggSpec::Fold {
            op: FoldOp::Sum,
            slot: plan.slot_of(VarId(1)),
            width: 1,
            signed: false,
        }),
        FindSpec::Agg(AggSpec::Count),
    ];

    let (mut sum, mut count) = (0u64, 0u64);
    for (p, a) in &postings {
        if !tags.iter().any(|(tp, _)| tp == p) {
            sum += a;
            count += 1;
        }
    }
    assert_eq!(count, 7, "the fixture rejects ids 1, 2, 3");

    for batch in [1usize, 2, 7, 64, 256] {
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slot_count());
        let mut sink = AggregateSink::new(finds.clone(), plan.slot_count());
        Executor::with_batch_size(&plan, batch)
            .execute(
                &plan,
                &mut colts,
                &mut bindings,
                &mut sink,
                &mut NoopCounters,
            )
            .expect("execute");
        let rows = sink.into_answers().expect("in range");
        assert_eq!(rows, vec![vec![sum, count]], "batch size {batch}");
    }
}

#[test]
fn outer_join_idiom_halves_are_complementary() {
    let dir = TempDir::new("run-anti-outer-join");
    let schema = schema(2);
    let a: Vec<(u64, u64)> = (0..16).map(|i| (i % 8, i)).collect();
    let b = vec![(2u64, 20u64), (3, 30), (3, 31), (5, 50)];
    let views = views_of(&dir, &schema, &[a.clone(), b.clone()]);

    let join_half = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            occurrence(1, 1, &[(0, 0), (1, 2)]),
        ],
        vec![],
    );
    let join_plan = planned(&join_half, &schema, &[0, 1]);
    let join_rows = run(&join_plan, &views);

    let absence_half = normalized(
        vec![
            occurrence(0, 0, &[(0, 0), (1, 1)]),
            negated(1, 1, &[(0, 0)]),
        ],
        vec![],
    );
    let absence_plan = planned(&absence_half, &schema, &[0]);
    let absence_rows = run(&absence_plan, &views);

    let mut join_oracle = BTreeSet::new();
    for (ax, ap) in &a {
        for (bx, bq) in &b {
            if ax == bx {
                let mut row = vec![0u64; 3];
                row[join_plan.slot_of(VarId(0))] = *ax;
                row[join_plan.slot_of(VarId(1))] = *ap;
                row[join_plan.slot_of(VarId(2))] = *bq;
                join_oracle.insert(row);
            }
        }
    }
    let mut absence_oracle = BTreeSet::new();
    for (ax, ap) in &a {
        if !b.iter().any(|(bx, _)| bx == ax) {
            let mut row = vec![0u64; 2];
            row[absence_plan.slot_of(VarId(0))] = *ax;
            row[absence_plan.slot_of(VarId(1))] = *ap;
            absence_oracle.insert(row);
        }
    }
    assert_eq!(join_rows, join_oracle);
    assert_eq!(absence_rows, absence_oracle);
    assert_eq!(
        join_rows.len() + absence_rows.len(),
        join_oracle.len() + absence_oracle.len(),
        "the pair partitions the work: |A ⋈ B| + |A − πB|"
    );

    let join_xs: BTreeSet<u64> = join_rows
        .iter()
        .map(|row| row[join_plan.slot_of(VarId(0))])
        .collect();
    let absence_xs: BTreeSet<u64> = absence_rows
        .iter()
        .map(|row| row[absence_plan.slot_of(VarId(0))])
        .collect();
    assert!(join_xs.is_disjoint(&absence_xs));
    let all_xs: BTreeSet<u64> = a.iter().map(|(x, _)| *x).collect();
    let union: BTreeSet<u64> = join_xs.union(&absence_xs).copied().collect();
    assert_eq!(union, all_xs);

    assert_batch_equality(&join_plan, &views, &join_oracle);
    assert_batch_equality(&absence_plan, &views, &absence_oracle);
}
