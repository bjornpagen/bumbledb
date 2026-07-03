//! The allocation gate (PRD 26): the doc's protocol as a contract of warm
//! prepared-query execution through the public surface — single-threaded
//! harness (one test function, its own binary), N=8 warmups over a fixed
//! param set, then M=8 measured runs asserting **zero** allocator hits,
//! arena growth included, result buffer caller-provided.
//!
//! Run with `cargo test --features alloc-counter --test alloc_gate`.

#![cfg(feature = "alloc-counter")]

use bumbledb::alloc_counter;
use bumbledb::ir::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};
use bumbledb::schema::{
    ConstraintDescriptor, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId,
    Schema, SchemaDescriptor, ValueType,
};
use bumbledb::{Db, PreparedQuery, ResultBuffer, Snapshot};

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

fn populate(db: &Db<'_>) {
    db.write(|tx| {
        for account in 0..20u64 {
            tx.insert_dyn(ACCOUNT, &[Value::U64(account), Value::U64(account % 5)])?;
        }
        for id in 0..500u64 {
            tx.insert_dyn(
                POSTING,
                &[
                    Value::U64(id),
                    Value::U64(id % 20),
                    Value::I64((id.cast_signed() % 100) - 50),
                ],
            )?;
        }
        Ok(())
    })
    .expect("populate");
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
    snap: &Snapshot<'_>,
    param_set: &[Vec<Value>],
) {
    let mut out = ResultBuffer::new();
    // N = 8 warmup runs over the fixed param set.
    for _ in 0..8 {
        for params in param_set {
            snap.execute(prepared, params, &mut out).expect(label);
        }
    }
    // M = 8 measured runs: zero allocations, arena growth included.
    alloc_counter::reset();
    for _ in 0..8 {
        for params in param_set {
            snap.execute(prepared, params, &mut out).expect(label);
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
    let db = Db::create(&dir, &schema).expect("create");
    populate(&db);

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

    db.read(|snap| {
        // The three shapes, across batch sizes (1 = the degenerate scalar).
        for batch in [1usize, 2, 64, 128] {
            let mut join = db.prepare(&join_query())?;
            join.set_batch_size(batch);
            gate(&format!("join/batch{batch}"), &mut join, snap, &join_params);

            let mut aggregate = db.prepare(&aggregate_query())?;
            aggregate.set_batch_size(batch);
            gate(
                &format!("aggregate/batch{batch}"),
                &mut aggregate,
                snap,
                &join_params,
            );
        }
        let mut guard = db.prepare(&guard_query())?;
        gate("guard", &mut guard, snap, &guard_params);

        // Warmup convergence: allocation is finite — by the third warmup
        // round a run allocates nothing.
        let mut fresh = db.prepare(&join_query())?;
        let mut out = ResultBuffer::new();
        let mut per_round = Vec::new();
        for _ in 0..3 {
            alloc_counter::reset();
            for params in &join_params {
                snap.execute(&mut fresh, params, &mut out)?;
            }
            per_round.push(alloc_counter::count());
        }
        assert_eq!(
            per_round[2], 0,
            "third warmup round must be silent: {per_round:?}"
        );
        Ok(())
    })
    .expect("gate");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
