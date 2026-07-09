//! The allocation gate (docs/architecture/30-execution.md): the doc's protocol as a contract of warm
//! prepared-query execution through the public surface — single-threaded
//! harness (one test function, its own binary), N=8 warmups over a fixed
//! param set, then M=8 measured runs asserting **zero** allocator hits,
//! arena growth included, result buffer caller-provided.
//!
//! Run with `cargo test --features alloc-counter --test alloc_gate`.
//!
//! INVARIANT: this binary holds exactly ONE test function, and check.sh
//! runs it with `--test-threads=1` (belt and suspenders). The counting
//! allocator is process-global — a second test running concurrently
//! would count its allocations into the measured window and turn the
//! gate flaky. Add new gate scenarios *inside* the one test, never as
//! sibling `#[test]`s.

#![cfg(feature = "alloc-counter")]

use bumbledb::alloc_counter;
use bumbledb::ir::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema,
    SchemaDescriptor, Side, StatementDescriptor, ValueType,
};
use bumbledb::{Db, PreparedQuery, ResultBuffer, Snapshot};

/// Posting(id serial, account u64, amount i64, memo str) +
/// Account(id serial, holder u64), with
/// `Posting(account) <= Account(id)`.
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
                    FieldDescriptor {
                        name: "memo".into(),
                        value_type: ValueType::String,
                        generation: Generation::None,
                    },
                ],
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
            },
        ],
        statements: vec![StatementDescriptor::Containment {
            source: Side {
                relation: RelationId(0),
                projection: Box::new([FieldId(1)]),
                selection: Box::new([]),
            },
            target: Side {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
                selection: Box::new([]),
            },
        }],
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
                    Value::String(format!("memo-{}", id % 4).into_bytes().into()),
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
        negated: vec![],
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
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        }],
    }
}

/// Q(holder, memo) :- Posting(account = a, memo), Account(id = a, holder),
/// memo != "memo-0" — string results through the byte heap, a PendingIntern
/// literal under Ne, and a projection narrower than the join (the D2
/// SkipSuffix path live on every duplicate (holder, memo) pair).
fn string_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(3))],
        atoms: vec![
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(3))),
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
        negated: vec![],
        predicates: vec![Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(3)),
            rhs: Term::Literal(Value::String(Box::from(&b"memo-0"[..]))),
        }],
    }
}

/// Q(holder, Min(amount), Max(amount)) :- ... — the Min/Max aggregate shape.
fn minmax_query() -> Query {
    Query {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: AggOp::Min,
                over: Some(VarId(1)),
            },
            FindTerm::Aggregate {
                op: AggOp::Max,
                over: Some(VarId(1)),
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
        negated: vec![],
        predicates: vec![],
    }
}

/// Q(amount) :- Posting(memo = ?0, amount) — the selection shape
/// (docs/architecture/30-execution.md): a rotating Eq param on a non-unique field probes the
/// COLT's selection level; after the rotation's first cycle forces every
/// probed subtrie, further rotation must not touch the allocator.
fn selection_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(3), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
    }
}

/// Q(memo, amount) :- Posting(account = ?0, memo, amount) — string
/// results across rotating params (docs/architecture/30-execution.md): the finalize memo and
/// the buffer byte heap must both sit at their high-water after warmup.
fn string_rotation_query() -> Query {
    Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(1), Term::Param(ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        predicates: vec![],
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
        negated: vec![],
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
    assert_eq!(
        alloc_counter::dealloc_count(),
        0,
        "{label}: a warm execution freed retained capacity"
    );
    let bytes = alloc_counter::snapshot();
    assert_eq!(
        (bytes.alloc_bytes, bytes.dealloc_bytes),
        (0, 0),
        "{label}: warm byte totals must be zero too"
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

    // Four rotating residual windows: exactly the view memo's capacity
    // (docs/architecture/30-execution.md) — steady-state rotation must stay allocation-free.
    let join_params = vec![
        vec![Value::I64(-10)],
        vec![Value::I64(0)],
        vec![Value::I64(25)],
        vec![Value::I64(40)],
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
        // String columns, PendingIntern-under-Ne, and the narrow
        // projection (SkipSuffix live); Min/Max aggregates. Params are
        // empty: these gate the literal-resolution and byte-heap paths.
        let no_params = vec![vec![]];
        let mut strings = db.prepare(&string_query())?;
        gate("string", &mut strings, snap, &no_params);
        let mut minmax = db.prepare(&minmax_query())?;
        gate("minmax", &mut minmax, snap, &no_params);

        let mut guard = db.prepare(&guard_query())?;
        gate("guard", &mut guard, snap, &guard_params);

        // The selection shape (docs/architecture/30-execution.md): four rotating Eq params on
        // a non-unique string field — the gate's warmups cover two full
        // rotation cycles, so every probed subtrie is forced and the
        // measured rotations must not touch the allocator.
        let selection_params: Vec<Vec<Value>> = (0..4)
            .map(|m| vec![Value::String(format!("memo-{m}").into_bytes().into())])
            .collect();
        let mut selection = db.prepare(&selection_query())?;
        gate("selection", &mut selection, snap, &selection_params);

        // String projections across rotating params (docs/architecture/30-execution.md): the
        // intern-resolution memo joins the zero-alloc steady state.
        let account_params: Vec<Vec<Value>> = (0..4).map(|a| vec![Value::U64(a)]).collect();
        let mut string_rotation = db.prepare(&string_rotation_query())?;
        gate(
            "string-rotation",
            &mut string_rotation,
            snap,
            &account_params,
        );

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
