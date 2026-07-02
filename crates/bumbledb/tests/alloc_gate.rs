//! The allocation gate (PRD 26): the doc's protocol as a unit-level
//! contract of `PreparedQuery::execute` — single-threaded harness (one
//! test function, its own binary), N=8 warmups over a fixed param set,
//! then M=8 measured runs asserting **zero** allocator hits, arena growth
//! included, result buffer caller-provided.
//!
//! Run with `cargo test --features alloc-counter --test alloc_gate`.

#![cfg(feature = "alloc-counter")]

use bumbledb::alloc_counter;
use bumbledb::api::prepared::{prepare, PreparedQuery, ResultBuffer};
use bumbledb::encoding::{encode_fact, ValueRef};
use bumbledb::image::cache::ImageCache;
use bumbledb::ir::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};
use bumbledb::schema::{
    ConstraintDescriptor, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId,
    Schema, SchemaDescriptor, ValueType,
};
use bumbledb::storage::commit::commit;
use bumbledb::storage::delta::WriteDelta;
use bumbledb::storage::env::Environment;

/// Posting(id serial, account u64, amount i64) +
/// Account(id serial, holder u64).
fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![ConstraintDescriptor::ForeignKey {
                    name: "posting_account".into(),
                    fields: Box::new([FieldId(1)]),
                    target_relation: RelationId(1),
                    target_constraint: bumbledb::schema::ConstraintId(0),
                }],
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "holder".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

const POSTING: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);

fn populate(env: &Environment, schema: &Schema) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for account in 0..20u64 {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(account), ValueRef::U64(account % 5)],
            schema.relation(ACCOUNT).layout(),
            &mut bytes,
        );
        delta.insert(&view, ACCOUNT, &bytes).expect("insert");
    }
    for id in 0..500u64 {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(id),
                ValueRef::U64(id % 20),
                ValueRef::I64((id.cast_signed() % 100) - 50),
            ],
            schema.relation(POSTING).layout(),
            &mut bytes,
        );
        delta.insert(&view, POSTING, &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// Q(holder, amount) :- Posting(account = a, amount), Account(id = a,
/// holder), amount >= ?0 — the join shape.
fn join_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: ACCOUNT,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        }],
    }
}

/// Q(holder, Sum(amount)) :- ... — the aggregate shape.
fn aggregate_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Count,
                over: None,
            },
        ],
        atoms: vec![
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(3))),
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: ACCOUNT,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Var(VarId(0))),
                ],
            },
        ],
        predicates: vec![Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        }],
    }
}

/// Q(amount) :- Posting(id = ?0, amount) — the guard-probe shape.
fn guard_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        predicates: vec![],
    }
}

/// The gate protocol for one prepared query and its fixed param set.
fn gate(
    label: &str,
    prepared: &mut PreparedQuery<'_>,
    txn: &bumbledb::storage::env::ReadTxn<'_>,
    cache: &ImageCache,
    param_set: &[Vec<Value>],
) {
    let mut out = ResultBuffer::new();
    // N = 8 warmup runs over the fixed param set.
    for _ in 0..8 {
        for params in param_set {
            prepared.execute(txn, cache, params, &mut out).expect(label);
        }
    }
    // M = 8 measured runs: zero allocations, arena growth included.
    alloc_counter::reset();
    for _ in 0..8 {
        for params in param_set {
            prepared.execute(txn, cache, params, &mut out).expect(label);
        }
    }
    assert_eq!(
        alloc_counter::count(),
        0,
        "{label}: a warm execution allocated"
    );
    assert!(!out.is_empty(), "{label}: the fixture produced rows");
}

/// One test function: the gate binary is single-threaded by construction.
#[test]
fn zero_warm_allocation_gate() {
    let dir = std::env::temp_dir().join("bumbledb-alloc-gate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("test dir");
    let schema = schema();
    let env = Environment::create(&dir, &schema).expect("create");
    populate(&env, &schema);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    let join_params = vec![
        vec![Value::I64(-10)],
        vec![Value::I64(0)],
        vec![Value::I64(25)],
    ];
    // The miss (9999) runs first so the last measured execution leaves rows.
    let guard_params = vec![
        vec![Value::U64(9999)],
        vec![Value::U64(5)],
        vec![Value::U64(499)],
    ];

    // The three shapes, across batch sizes (1 = the degenerate scalar).
    for batch in [1usize, 2, 64, 128] {
        let mut join = prepare(&txn, &schema, &join_query()).expect("prepare");
        join.set_batch_size(batch);
        gate(
            &format!("join/batch{batch}"),
            &mut join,
            &txn,
            &cache,
            &join_params,
        );

        let mut aggregate = prepare(&txn, &schema, &aggregate_query()).expect("prepare");
        aggregate.set_batch_size(batch);
        gate(
            &format!("aggregate/batch{batch}"),
            &mut aggregate,
            &txn,
            &cache,
            &join_params,
        );
    }
    let mut guard = prepare(&txn, &schema, &guard_query()).expect("prepare");
    gate("guard", &mut guard, &txn, &cache, &guard_params);

    // Warmup convergence: allocation is finite — by the third warmup round
    // a run allocates nothing.
    let mut fresh = prepare(&txn, &schema, &join_query()).expect("prepare");
    let mut out = ResultBuffer::new();
    let mut per_round = Vec::new();
    for _ in 0..3 {
        alloc_counter::reset();
        for params in &join_params {
            fresh
                .execute(&txn, &cache, params, &mut out)
                .expect("execute");
        }
        per_round.push(alloc_counter::count());
    }
    assert_eq!(
        per_round[2], 0,
        "third warmup round must be silent: {per_round:?}"
    );

    drop(txn);
    drop(env);
    let _ = std::fs::remove_dir_all(&dir);
}
