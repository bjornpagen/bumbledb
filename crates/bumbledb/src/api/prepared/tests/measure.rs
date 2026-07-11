//! The measure criteria (20-query-ir § the measure) at the API boundary: `Duration` in finds, in
//! `Sum`/`Min`/`Max`, and in comparisons (literal, param, same-atom
//! variable, cross-atom variable), over both element types and the
//! boundary intervals `[x, x+1)` / `[MIN, MAX−1)`; the ray raising the
//! typed `MeasureOfRay` on every evaluation path while the same query
//! guarded by `Allen(DISJOINT` from the ray probe`)` or a bounded-end
//! filter succeeds; and `Sum(Duration)` overflow at the wide-accumulator
//! → u64 boundary as the existing typed overflow error.

use super::*;
use crate::allen::AllenMask;
use crate::ir::{AggOp, MaskTerm, ParamId};
use crate::schema::{Generation, IntervalElement};

/// Session(id fresh u64, tag u64, cap u64, span interval<u64>);
/// Shift(id fresh u64, tag u64, span interval<i64>);
/// Window(id fresh u64, tag u64, cap u64).
fn measure_schema() -> Schema {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    let fresh_id = || FieldDescriptor {
        name: "id".into(),
        value_type: ValueType::U64,
        generation: Generation::Fresh,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Session".into(),
                fields: vec![
                    fresh_id(),
                    field("tag", ValueType::U64),
                    field("cap", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Shift".into(),
                fields: vec![
                    fresh_id(),
                    field("tag", ValueType::U64),
                    field(
                        "span",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Window".into(),
                fields: vec![
                    fresh_id(),
                    field("tag", ValueType::U64),
                    field("cap", ValueType::U64),
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const SESSION: RelationId = RelationId(0);
const SHIFT: RelationId = RelationId(1);
const WINDOW: RelationId = RelationId(2);

fn insert_sessions(env: &Environment, schema: &Schema, rows: &[(u64, u64, u64, (u64, u64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, tag, cap, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*tag),
                ValueRef::U64(*cap),
                ValueRef::IntervalU64(*start, *end),
            ],
            schema.relation(SESSION).layout(),
            &mut bytes,
        );
        delta.insert(&view, SESSION, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

fn insert_shifts(env: &Environment, schema: &Schema, rows: &[(u64, u64, (i64, i64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, tag, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*tag),
                ValueRef::IntervalI64(*start, *end),
            ],
            schema.relation(SHIFT).layout(),
            &mut bytes,
        );
        delta.insert(&view, SHIFT, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

fn insert_windows(env: &Environment, schema: &Schema, rows: &[(u64, u64, u64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, tag, cap) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(*id), ValueRef::U64(*tag), ValueRef::U64(*cap)],
            schema.relation(WINDOW).layout(),
            &mut bytes,
        );
        delta.insert(&view, WINDOW, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

fn u64_rows(out: &ResultBuffer, arity: usize) -> Vec<Vec<u64>> {
    let mut rows: Vec<Vec<u64>> = (0..out.len())
        .map(|row| {
            (0..arity)
                .map(|column| match out.get(row, column) {
                    ResultValue::U64(v) => v,
                    other => panic!("all-U64 row: {other:?}"),
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// Q(tag, Duration(span)) :- Session(tag, span) — the measure in a find
/// position, boundary intervals included: `[x, x+1)` measures 1 and
/// `[MIN, MAX−1)` measures `MAX−1`.
#[test]
fn duration_find_projects_the_measure_u64() {
    let dir = TempDir::new("measure-find-u64");
    let schema = measure_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_sessions(
        &env,
        &schema,
        &[
            (1, 10, 0, (5, 6)),            // [x, x+1): measure 1
            (2, 20, 0, (0, u64::MAX - 1)), // [MIN, MAX−1): measure MAX−1
            (3, 30, 0, (100, 350)),        // measure 250
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        atoms: vec![Atom {
            relation: SESSION,
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        u64_rows(&out, 2),
        vec![vec![10, 1], vec![20, u64::MAX - 1], vec![30, 250]]
    );
}

/// The I64 element type: the encoded-word subtraction equals the true
/// difference (the sign-flip bias cancels) — negative-spanning intervals
/// and the boundary shapes included.
#[test]
fn duration_find_projects_the_measure_i64() {
    let dir = TempDir::new("measure-find-i64");
    let schema = measure_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_shifts(
        &env,
        &schema,
        &[
            (1, 10, (-5, 5)),                  // spans zero: measure 10
            (2, 20, (i64::MIN, i64::MAX - 1)), // [MIN, MAX−1): 2^64 − 2
            (3, 30, (7, 8)),                   // [x, x+1): measure 1
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        atoms: vec![Atom {
            relation: SHIFT,
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        u64_rows(&out, 2),
        vec![vec![10, 10], vec![20, u64::MAX - 1], vec![30, 1]]
    );
}

/// Q(tag, Sum(Duration), Min(Duration), Max(Duration)) :- Session — the
/// measure as the aggregated input of all three folds, grouped.
#[test]
fn sum_min_max_over_the_measure() {
    let dir = TempDir::new("measure-folds");
    let schema = measure_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_sessions(
        &env,
        &schema,
        &[
            (1, 10, 0, (0, 4)),   // 4
            (2, 10, 0, (10, 13)), // 3
            (3, 20, 0, (5, 6)),   // 1
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let over = |op: AggOp| FindTerm::AggregateDuration { op, over: VarId(1) };
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            over(AggOp::Sum),
            over(AggOp::Min),
            over(AggOp::Max),
        ],
        atoms: vec![Atom {
            relation: SESSION,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        u64_rows(&out, 4),
        vec![vec![10, 7, 3, 4], vec![20, 1, 1, 1]]
    );
}

/// Duration comparisons in all four shapes: vs a literal (the fused
/// dense kernel), vs a param, vs a same-atom u64 variable (the same-fact
/// filter), and vs a cross-atom u64 variable (the measure residual).
#[test]
fn duration_comparisons_filter_and_join() {
    let dir = TempDir::new("measure-comparisons");
    let schema = measure_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_sessions(
        &env,
        &schema,
        &[
            (1, 10, 5, (0, 4)),   // measure 4 < cap 5
            (2, 20, 2, (10, 13)), // measure 3 >= cap 2
            (3, 30, 1, (5, 6)),   // measure 1 >= cap 1
        ],
    );
    insert_windows(&env, &schema, &[(1, 10, 2), (2, 20, 4), (3, 30, 1)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let session_atom = Atom {
        relation: SESSION,
        bindings: vec![
            (FieldId(1), Term::Var(VarId(0))),
            (FieldId(2), Term::Var(VarId(2))),
            (FieldId(3), Term::Var(VarId(1))),
        ],
    };
    let run = |query: &Query| -> Vec<Vec<u64>> {
        let mut prepared = prepare(&txn, &cache, &schema, query).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[])
            .expect("execute");
        u64_rows(&out, 1)
    };
    let single = |predicates: Vec<PredicateTree>| {
        Query::single(Rule {
            finds: vec![FindTerm::Var(VarId(0))],
            atoms: vec![session_atom.clone()],
            negated: vec![],
            predicates,
        })
    };

    // Literal, both orientations (the mirrored form flips the operator).
    let literal = single(vec![PredicateTree::Leaf(Comparison {
        op: CmpOp::Gt,
        lhs: Term::Duration(VarId(1)),
        rhs: Term::Literal(Value::U64(2)),
    })]);
    assert_eq!(run(&literal), vec![vec![10], vec![20]]);
    let mirrored = single(vec![PredicateTree::Leaf(Comparison {
        op: CmpOp::Ge,
        lhs: Term::Literal(Value::U64(3)),
        rhs: Term::Duration(VarId(1)),
    })]);
    assert_eq!(run(&mirrored), vec![vec![20], vec![30]]);

    // Param bound at execution.
    let param = single(vec![PredicateTree::Leaf(Comparison {
        op: CmpOp::Lt,
        lhs: Term::Duration(VarId(1)),
        rhs: Term::Param(ParamId(0)),
    })]);
    let mut prepared = prepare(&txn, &cache, &schema, &param).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(4)])
        .expect("execute");
    assert_eq!(u64_rows(&out, 1), vec![vec![20], vec![30]]);

    // Same-atom u64 variable: measure vs the fact's own cap.
    let same_atom = single(vec![PredicateTree::Leaf(Comparison {
        op: CmpOp::Ge,
        lhs: Term::Duration(VarId(1)),
        rhs: Term::Var(VarId(2)),
    })]);
    assert_eq!(run(&same_atom), vec![vec![20], vec![30]]);

    // Cross-atom u64 variable: the measure residual inside the join.
    let cross_atom = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                relation: SESSION,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: WINDOW,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(3))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Duration(VarId(1)),
            rhs: Term::Var(VarId(3)),
        })],
    });
    // Session measures by tag: 10 → 4 (cap 2: pass), 20 → 3 (cap 4:
    // fail), 30 → 1 (cap 1: pass).
    assert_eq!(run(&cross_atom), vec![vec![10], vec![30]]);
}

/// The ray probe `[MAX−1, MAX)`: an interval intersects it iff it covers
/// the point `MAX−1`, i.e. iff `end == MAX` — exactly the rays.
fn ray_guard() -> PredicateTree {
    PredicateTree::Leaf(Comparison {
        op: CmpOp::Allen {
            mask: MaskTerm::Literal(AllenMask::DISJOINT),
        },
        lhs: Term::Var(VarId(1)),
        rhs: Term::Literal(Value::IntervalU64(u64::MAX - 1, u64::MAX)),
    })
}

/// A ray reaching `Duration` raises the typed `MeasureOfRay` — the
/// engine's one runtime type error — on every evaluation path: the find,
/// the fold, the filter, and the cross-atom residual. The same queries
/// guarded by `Allen(DISJOINT` from the ray probe`)` succeed, and so
/// does the bounded-end (`COVERED_BY` a bounded window) form: the
/// filter-order law runs the guard before the subtraction.
#[test]
#[allow(clippy::too_many_lines)] // one fixture, every evaluation path in order
fn a_ray_reaching_duration_raises_and_a_guarded_query_succeeds() {
    let dir = TempDir::new("measure-ray");
    let schema = measure_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_sessions(
        &env,
        &schema,
        &[
            (1, 10, 0, (0, 4)),        // bounded: measure 4
            (2, 20, 0, (7, u64::MAX)), // the ray [7, ∞)
        ],
    );
    insert_windows(&env, &schema, &[(1, 10, 2), (2, 20, 2)]);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let session_atom = Atom {
        relation: SESSION,
        bindings: vec![
            (FieldId(1), Term::Var(VarId(0))),
            (FieldId(3), Term::Var(VarId(1))),
        ],
    };
    let assert_ray = |query: &Query| {
        let mut prepared = prepare(&txn, &cache, &schema, query).expect("prepare");
        match prepared.execute_collect(&txn, &cache, &[]) {
            Err(Error::MeasureOfRay { start, end }) => {
                assert_eq!((start, end), (7, u64::MAX), "the offending interval words");
            }
            other => panic!("expected MeasureOfRay, got {other:?}"),
        }
    };
    let run_guarded = |mut rule: Rule| -> Vec<Vec<u64>> {
        rule.predicates.push(ray_guard());
        let query = Query::single(rule);
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        let out = prepared
            .execute_collect(&txn, &cache, &[])
            .expect("guarded execute");
        u64_rows(&out, out.arity())
    };

    // The find position.
    let find_rule = Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Duration(VarId(1))],
        atoms: vec![session_atom.clone()],
        negated: vec![],
        predicates: vec![],
    };
    assert_ray(&Query::single(find_rule.clone()));
    assert_eq!(run_guarded(find_rule.clone()), vec![vec![10, 4]]);

    // The fold position.
    let fold_rule = Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::AggregateDuration {
                op: AggOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![session_atom.clone()],
        negated: vec![],
        predicates: vec![],
    };
    assert_ray(&Query::single(fold_rule.clone()));
    assert_eq!(run_guarded(fold_rule), vec![vec![10, 4]]);

    // The filter position (measure vs literal).
    let filter_rule = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![session_atom.clone()],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Gt,
            lhs: Term::Duration(VarId(1)),
            rhs: Term::Literal(Value::U64(1)),
        })],
    };
    assert_ray(&Query::single(filter_rule.clone()));
    assert_eq!(run_guarded(filter_rule.clone()), vec![vec![10]]);

    // The bounded-end guard form: span ⊆ [0, 100) bounds the end below
    // the ceiling, so the ray never reaches the subtraction.
    let mut bounded = filter_rule.clone();
    bounded.predicates.push(PredicateTree::Leaf(Comparison {
        op: CmpOp::Allen {
            mask: MaskTerm::Literal(AllenMask::COVERED_BY),
        },
        lhs: Term::Var(VarId(1)),
        rhs: Term::Literal(Value::IntervalU64(0, 100)),
    }));
    let query = Query::single(bounded);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("bounded-end execute");
    assert_eq!(u64_rows(&out, 1), vec![vec![10]]);

    // The cross-atom residual: the ray reaches the subtraction inside
    // the join (the executor's poison path).
    let residual_rule = Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            session_atom.clone(),
            Atom {
                relation: WINDOW,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(3))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Duration(VarId(1)),
            rhs: Term::Var(VarId(3)),
        })],
    };
    assert_ray(&Query::single(residual_rule.clone()));
    assert_eq!(run_guarded(residual_rule), vec![vec![10]]);
}

/// `Sum(Duration)` overflow at the wide-accumulator → u64 boundary: two
/// near-maximal measures exceed `u64::MAX`, and the failure is the
/// existing typed overflow error — the single finalize range check, like
/// every Sum.
#[test]
fn sum_of_durations_overflow_is_the_typed_overflow_error() {
    let dir = TempDir::new("measure-sum-overflow");
    let schema = measure_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_sessions(
        &env,
        &schema,
        &[
            (1, 10, 0, (0, u64::MAX - 1)), // measure MAX−1
            (2, 10, 0, (1, u64::MAX - 1)), // measure MAX−2
        ],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::AggregateDuration {
                op: AggOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            relation: SESSION,
            bindings: vec![
                (FieldId(0), Term::Var(VarId(2))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    match prepared.execute_collect(&txn, &cache, &[]) {
        Err(Error::Overflow(crate::error::OverflowKind::Aggregate { find })) => {
            assert_eq!(find, 1);
        }
        other => panic!("expected the typed overflow error, got {other:?}"),
    }
}
