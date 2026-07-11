//! The `Pack` criteria (20-query-ir § aggregation) at the API boundary:
//! the coalescing fold end to end — validate → plan → execute → result
//! buffer. Relation-shaped: one result row per (group, maximal segment);
//! adjacency merges, duplicates collapse in the sweep, a packed ray is a
//! ray; `Pack` groups by the non-aggregated head vars exactly as `Sum`
//! does; and a multi-rule head folds the union (the spanning seen-set
//! keys (group, claim) pairs).

use super::*;
use crate::ir::{AggOp, ParamId};
use crate::schema::{Generation, IntervalElement};

/// Busy(id fresh u64, person u64, cap u64, slot interval<u64>);
/// Shift(id fresh u64, person u64, slot interval<i64>).
fn pack_schema() -> Schema {
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
                name: "Busy".into(),
                fields: vec![
                    fresh_id(),
                    field("person", ValueType::U64),
                    field("cap", ValueType::U64),
                    field(
                        "slot",
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
                    field("person", ValueType::U64),
                    field(
                        "slot",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                ],
            },
        ],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture")
}

const BUSY: RelationId = RelationId(0);
const SHIFT: RelationId = RelationId(1);

fn insert_busy(env: &Environment, schema: &Schema, rows: &[(u64, u64, u64, (u64, u64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, person, cap, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*person),
                ValueRef::U64(*cap),
                ValueRef::IntervalU64(*start, *end),
            ],
            schema.relation(BUSY).layout(),
            &mut bytes,
        );
        delta.insert(&view, BUSY, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

fn insert_shifts(env: &Environment, schema: &Schema, rows: &[(u64, u64, (i64, i64))]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, person, (start, end)) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*id),
                ValueRef::U64(*person),
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

/// Q(person, Pack(slot)) :- Busy(person, slot).
fn pack_query() -> Query {
    Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    })
}

fn packed_u64_rows(out: &ResultBuffer) -> Vec<(u64, u64, u64)> {
    let mut rows: Vec<(u64, u64, u64)> = (0..out.len())
        .map(|row| match (out.get(row, 0), out.get(row, 1)) {
            (ResultValue::U64(person), ResultValue::IntervalU64(iv)) => {
                (person, iv.start(), iv.end())
            }
            other => panic!("(u64, interval<u64>) row: {other:?}"),
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// Overlap, containment, adjacency, and duplicate claims fold into
/// maximal segments per group — one result row each, groups scoped.
#[test]
fn pack_coalesces_overlap_adjacency_and_duplicates_per_group() {
    let dir = TempDir::new("pack-coalesce");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_busy(
        &env,
        &schema,
        &[
            (1, 10, 0, (1, 3)),  // overlaps the next
            (2, 10, 0, (2, 5)),  // meets the next (adjacency merges)
            (3, 10, 0, (5, 7)),  //
            (4, 10, 0, (2, 4)),  // nested inside [1, 7)
            (5, 10, 0, (9, 10)), // the gap: a second segment
            (6, 20, 0, (4, 6)),  // duplicate claims, distinct fresh ids —
            (7, 20, 0, (4, 6)),  //   they collapse in the sweep
        ],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &pack_query()).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        packed_u64_rows(&out),
        vec![(10, 1, 7), (10, 9, 10), (20, 4, 6)]
    );
}

/// A ray absorbs everything after its start and the packed ray is a ray
/// (`end == MAX` is the frontier no later claim exceeds) — the I64
/// element type, negative spans included.
#[test]
fn pack_absorbs_rays_over_i64_spans() {
    let dir = TempDir::new("pack-rays");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_shifts(
        &env,
        &schema,
        &[
            (1, 10, (-5, -2)),
            (2, 10, (-2, 4)),       // adjacency across zero
            (3, 10, (3, i64::MAX)), // the ray [3, ∞)
            (4, 10, (100, 200)),    // absorbed by the ray
            (5, 20, (-10, -9)),     // bounded group stays bounded
        ],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
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
    let mut rows: Vec<(u64, i64, i64)> = (0..out.len())
        .map(|row| match (out.get(row, 0), out.get(row, 1)) {
            (ResultValue::U64(person), ResultValue::IntervalI64(iv)) => {
                (person, iv.start(), iv.end())
            }
            other => panic!("(u64, interval<i64>) row: {other:?}"),
        })
        .collect();
    rows.sort_unstable();
    assert_eq!(rows, vec![(10, -5, i64::MAX), (20, -10, -9)]);
}

/// The group-interaction criterion: `Pack` groups by the non-aggregated
/// head vars exactly as `Sum` does — one fixture, both queries, the same
/// group-key set.
#[test]
fn pack_groups_exactly_as_sum_does() {
    let dir = TempDir::new("pack-groups-as-sum");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_busy(
        &env,
        &schema,
        &[
            (1, 10, 4, (1, 3)),
            (2, 10, 6, (7, 9)),
            (3, 20, 5, (2, 4)),
            (4, 30, 1, (2, 4)),
        ],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let sum_query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(2)),
            },
        ],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    });
    let mut sum = prepare(&txn, &cache, &schema, &sum_query).expect("prepare");
    let sum_out = sum.execute_collect(&txn, &cache, &[]).expect("execute");
    let mut sum_groups: Vec<u64> = (0..sum_out.len())
        .map(|row| match sum_out.get(row, 0) {
            ResultValue::U64(person) => person,
            other => panic!("u64 group key: {other:?}"),
        })
        .collect();
    sum_groups.sort_unstable();

    let mut pack = prepare(&txn, &cache, &schema, &pack_query()).expect("prepare");
    let pack_out = pack.execute_collect(&txn, &cache, &[]).expect("execute");
    let mut pack_groups: Vec<u64> = packed_u64_rows(&pack_out)
        .into_iter()
        .map(|(person, _, _)| person)
        .collect();
    pack_groups.dedup();
    assert_eq!(pack_groups, sum_groups);
    // Every group's claims are disjoint here, so Pack is one row per
    // claim — the group scoping is the assertion above.
    assert_eq!(pack_out.len(), 4);
}

/// Q(person, Pack(slot)) :- cap ≥ ?0 ∪ cap ≤ ?1 — the multi-rule head
/// folds the union: claims derived by both rules dedup through the
/// spanning seen-set (keyed on (group, claim)), then coalesce.
#[test]
fn multi_rule_pack_folds_the_union() {
    let dir = TempDir::new("pack-union");
    let schema = pack_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_busy(
        &env,
        &schema,
        &[
            (1, 10, 1, (1, 3)), // low-cap rule only
            (2, 10, 5, (5, 6)), // both rules — one fold, not two
            (3, 10, 9, (6, 8)), // high-cap rule only (adjacency merges)
        ],
    );
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let rule = |op: CmpOp, param: u16| Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Pack,
                over: Some(VarId(1)),
            },
        ],
        atoms: vec![Atom {
            relation: BUSY,
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(2))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![PredicateTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(2)),
            rhs: Term::Param(ParamId(param)),
        })],
    };
    let query = Query {
        head: vec![
            crate::ir::HeadTerm::Var,
            crate::ir::HeadTerm::Aggregate(crate::ir::HeadOp::Pack),
        ],
        rules: vec![rule(CmpOp::Ge, 0), rule(CmpOp::Le, 1)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::U64(5), BindValue::U64(5)])
        .expect("execute");
    // The union is all three claims: [1,3) stands alone; [5,6) — derived
    // by BOTH rules, folded once — meets [6,8).
    assert_eq!(packed_u64_rows(&out), vec![(10, 1, 3), (10, 5, 8)]);
}
