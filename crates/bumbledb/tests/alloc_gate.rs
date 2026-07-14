//! The allocation gate (docs/architecture/40-execution.md): the doc's protocol as a contract of warm
//! prepared-query execution through the public surface — single-threaded
//! harness (one test function, its own binary), two measured windows.
//! **Steady state:** N=8 warmups over a fixed param set, then M=8 measured
//! runs asserting **zero** allocator hits, arena growth included, result
//! buffer caller-provided. **High-water:** a parameter sequence of strictly
//! increasing selectivity, asserting allocations occur only on executions
//! that set a new intermediate high-water and that any repeat of a
//! previously-seen parameter is silent ([`escalation_gate`]).
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
use bumbledb::ir::{
    AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Rule, Term, Value, VarId,
};
use bumbledb::schema::{
    FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor, ValueType,
};
use bumbledb::{BindValue, ConditionTree, Db, PreparedQuery, ResultBuffer, Snapshot};

mod common;

/// Posting(id fresh, account u64, amount i64, memo str) +
/// Account(id fresh, holder u64) +
/// Busy(id fresh, person u64, slot interval<u64>), with
/// `Posting(account) <= Account(id)`.
fn schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
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
                extension: None,
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "holder".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Busy".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "person".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "slot".into(),
                        value_type: ValueType::Interval {
                            element: bumbledb::schema::IntervalElement::U64,
                        },
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
}

const POSTING: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const BUSY: RelationId = RelationId(2);

// The borrowed-struct gate's typed schema (PRD 22): a str-bearing
// relation whose generated struct borrows its memo (`&'a str`).
bumbledb::schema! {
    pub GateLedger;
    relation GateItem {
        id: u64 as GateItemId, fresh,
        memo: str,
    }
}

/// The high-water window's escalation ladder: per rung, one account that
/// is the sole account of its holder, with this many postings — so each
/// escalation parameter (holders 5..10, accounts 20..25) binds a strictly
/// hotter key and every rung's join intermediates strictly dominate the
/// last's.
const LADDER: [u64; 5] = [6, 24, 72, 240, 660];

fn populate(db: &Db<SchemaDescriptor>) {
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
        // The Pack fixture: per person, overlapping, adjacent, nested,
        // duplicate, and ray-bearing claims — the warm coalescing fold's
        // group lists, sort, and sweep all inside the measured window.
        for id in 0..120u64 {
            let person = id % 6;
            let start = (id * 7) % 40;
            let end = if id % 5 == 4 {
                u64::MAX // the ray
            } else {
                start + 1 + id % 9
            };
            tx.insert_dyn(
                BUSY,
                &[
                    Value::U64(id),
                    Value::U64(person),
                    Value::IntervalU64(
                        bumbledb::Interval::<u64>::new(start, end).expect("nonempty interval"),
                    ),
                ],
            )?;
        }
        let mut account = 20u64;
        let mut holder = 5u64;
        let mut id = 500u64;
        for count in LADDER {
            tx.insert_dyn(ACCOUNT, &[Value::U64(account), Value::U64(holder)])?;
            for _ in 0..count {
                tx.insert_dyn(
                    POSTING,
                    &[
                        Value::U64(id),
                        Value::U64(account),
                        Value::I64((id.cast_signed() % 100) - 50),
                        Value::String(format!("memo-{}", id % 4).into_bytes().into()),
                    ],
                )?;
                id += 1;
            }
            account += 1;
            holder += 1;
        }
        Ok(())
    })
    .expect("populate");
}

/// Q(holder, amount) :- Posting(account = a, amount), Account(id = a,
/// holder), amount >= ?0 — the join shape.
fn join_query() -> Query {
    Query::single(Rule {
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
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    })
}

/// Q(holder, Sum(amount)) :- ... — the aggregate shape.
fn aggregate_query() -> Query {
    Query::single(Rule {
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
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    })
}

/// Q(holder, memo) :- Posting(account = a, memo), Account(id = a, holder),
/// memo != "memo-0" — string results through the byte heap, a PendingIntern
/// literal under Ne, and a projection narrower than the join (the D2
/// SkipSuffix path live on every duplicate (holder, memo) pair).
fn string_query() -> Query {
    Query::single(Rule {
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
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(3)),
            rhs: Term::Literal(Value::String(Box::from(&b"memo-0"[..]))),
        })],
    })
}

/// Q(holder, Min(amount), Max(amount)) :- ... — the Min/Max aggregate shape.
fn minmax_query() -> Query {
    Query::single(Rule {
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
        conditions: vec![],
    })
}

/// Q(amount) :- Posting(memo == "memo-1", amount) — the param-free
/// str-literal selection (the literal latch, PRD 09): the first
/// (sanctioned) execution resolves and latches the literal into the
/// plan template; every measured execution rides the fully-latched
/// fast path — the latch wrote a fixed-size word into an existing
/// slot, so the steady-state zero stays zero.
fn latch_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (
                    FieldId(3),
                    Term::Literal(Value::String(Box::from(&b"memo-1"[..]))),
                ),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(amount) :- Posting(memo = ?0, amount) — the selection shape
/// (docs/architecture/40-execution.md): a rotating Eq param on a non-key field probes the
/// COLT's selection level; after the rotation's first cycle forces every
/// probed subtrie, further rotation must not touch the allocator.
fn selection_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(3), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(memo, amount) :- Posting(account = ?0, memo, amount) — string
/// results across rotating params (docs/architecture/40-execution.md): the finalize memo and
/// the buffer byte heap must both sit at their high-water after warmup.
fn string_rotation_query() -> Query {
    Query::single(Rule {
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
        conditions: vec![],
    })
}

/// Q(memo, amount) :- Posting(account = a, memo, amount),
/// Account(id = a, holder = ?0) — the high-water escalation shape
/// (docs/architecture/40-execution.md): the holder param is an Eq
/// selection level (one view, probed per execution — no per-param view
/// churn), and each ladder holder joins strictly more postings than the
/// last, so intermediates — pending buffers, sink dedup, result staging —
/// escalate with the parameter.
fn escalation_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: POSTING,
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: ACCOUNT,
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(holder, amount) :- account-side rule ∪ posting-side rule — the
/// multi-rule union shape (docs/architecture/40-execution.md § the rule
/// loop): two overlapping rules over the same head, one shared param
/// reaching both, one sink whose seen-set spans the rules. The overlap
/// (both rules admit mid-range amounts) keeps the spanning seen-set's
/// absorption live inside the measured window.
fn union_rules_query() -> Query {
    let rule = |op: CmpOp| Rule {
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
        conditions: vec![ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    Query {
        head: vec![bumbledb::HeadTerm::Var, bumbledb::HeadTerm::Var],
        rules: vec![rule(CmpOp::Ge), rule(CmpOp::Le)],
    }
}

/// Q(holder, Sum(amount), Count) :- the same two rules — the multi-rule
/// aggregate shape: the union regime's always-spanning head-projection
/// seen-set must reach its high-water and stay silent.
fn union_aggregate_query() -> Query {
    let rule = |op: CmpOp| Rule {
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
        conditions: vec![ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(ParamId(0)),
        })],
    };
    Query {
        head: vec![
            bumbledb::HeadTerm::Var,
            bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Sum),
            bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Count),
        ],
        rules: vec![rule(CmpOp::Ge), rule(CmpOp::Le)],
    }
}

/// Q(person, Pack(slot)) :- Busy(person, slot) — the coalescing-fold
/// shape (docs/architecture/40-execution.md § set semantics): warm
/// executions exercise the pooled per-group claim lists, the finalize
/// sort (`sort_unstable`, in-place), and the sweep's emit continuation —
/// all covered by the zero-allocation window as retained high-water
/// scratch.
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
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// Q(amount) :- Posting(id = ?0, amount) — the key-probe shape.
fn key_probe_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            relation: POSTING,
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

/// The gate protocol for one prepared query and its fixed param set.
fn gate(
    label: &str,
    prepared: &mut PreparedQuery<'_, SchemaDescriptor>,
    snap: &Snapshot<'_, SchemaDescriptor>,
    param_set: &[Vec<BindValue<'_>>],
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

/// One measured execution that must not touch the allocator at all —
/// events and bytes, both directions ([`escalation_gate`]'s repeat steps).
fn silent(
    label: &str,
    step: &str,
    prepared: &mut PreparedQuery<'_, SchemaDescriptor>,
    snap: &Snapshot<'_, SchemaDescriptor>,
    params: &[BindValue<'_>],
    out: &mut ResultBuffer,
) {
    alloc_counter::reset();
    snap.execute(prepared, params, out).expect(label);
    let bytes = alloc_counter::snapshot();
    assert_eq!(
        (
            bytes.allocs,
            bytes.deallocs,
            bytes.alloc_bytes,
            bytes.dealloc_bytes
        ),
        (0, 0, 0, 0),
        "{label}: {step} must be allocation-silent"
    );
}

/// The high-water window (docs/architecture/40-execution.md, § CI gate
/// protocol): warm the coldest parameter to its fixpoint, then walk a
/// strictly-hotter parameter sequence asserting (a) allocations occur
/// **only** on executions that set a new intermediate high-water — every
/// repeat of a previously-seen parameter, immediate or later, is silent —
/// and (b) the escalation itself observed at least one growth event (a
/// gate that never sees growth proves nothing).
///
/// Mutation demonstration (the gate is not theater; no test-only
/// injection point lives in the hot path, so the check was done manually
/// during development): a temporary
/// `std::hint::black_box(Vec::<u64>::with_capacity(1));` at the top of
/// `Executor::execute` (`exec/run/execute.rs`) — one heap allocation per
/// execution — made this variant (run first, ahead of the steady-state
/// scenarios) fail at its first repeat step: `escalation: repeat of
/// params[1] right after its high-water run must be allocation-silent`
/// with `(1, 1, 8, 8) != (0, 0, 0, 0)`; in normal order the steady-state
/// gate caught the same mutation at its first measured scenario
/// (`join/batch1: a warm execution allocated: 32 != 0`). Reverting the
/// mutation turned both green again. Observed sensitivity on the real
/// engine: the escalation's growth steps reallocated pools on rungs
/// 24, 72, and 240 (4+4+1 events) and were silent on 660 — the pending
/// buffers had converged at the batch cap, growth permitted but not
/// required on a high-water.
fn escalation_gate(
    label: &str,
    prepared: &mut PreparedQuery<'_, SchemaDescriptor>,
    snap: &Snapshot<'_, SchemaDescriptor>,
    params: &[Vec<BindValue<'_>>],
) {
    let mut out = ResultBuffer::new();
    // Warm the coldest parameter to its fixpoint — first-execution
    // allocations are sanctioned and stay outside the measured window.
    for _ in 0..8 {
        snap.execute(prepared, &params[0], &mut out).expect(label);
    }
    let mut growth_events = 0u64;
    for i in 1..params.len() {
        // A never-seen parameter whose intermediates strictly dominate
        // every prior execution's: a new high-water — the only execution
        // class the contract allows to allocate.
        alloc_counter::reset();
        snap.execute(prepared, &params[i], &mut out).expect(label);
        if alloc_counter::count() > 0 {
            growth_events += 1;
        }
        // The same parameter again: its own high-water now dominates it.
        silent(
            label,
            &format!("repeat of params[{i}] right after its high-water run"),
            prepared,
            snap,
            &params[i],
            &mut out,
        );
        // Every previously-seen parameter sits below the high-water.
        for j in 0..i {
            silent(
                label,
                &format!("repeat of params[{j}] under params[{i}]'s high-water"),
                prepared,
                snap,
                &params[j],
                &mut out,
            );
        }
    }
    // The vacuousness check: an escalation that never grew anything
    // cannot distinguish a correct engine from a gate with no eyes.
    assert!(
        growth_events >= 1,
        "{label}: the escalation observed no growth event — the fixture is vacuous"
    );
    assert!(!out.is_empty(), "{label}: the fixture produced rows");
}

/// One test function: the gate binary is single-threaded by construction.
#[test]
fn zero_warm_allocation_gate() {
    let dir = common::TempDir::new("alloc-gate");
    let db = Db::create(dir.path(), schema()).expect("create");
    populate(&db);

    // Four rotating residual windows: exactly the view memo's capacity
    // (docs/architecture/40-execution.md) — steady-state rotation must stay allocation-free.
    let join_params = vec![
        vec![BindValue::I64(-10)],
        vec![BindValue::I64(0)],
        vec![BindValue::I64(25)],
        vec![BindValue::I64(40)],
    ];
    // The miss (9999) runs first so the last measured execution leaves rows.
    let key_probe_params = vec![
        vec![BindValue::U64(9999)],
        vec![BindValue::U64(5)],
        vec![BindValue::U64(499)],
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

        // The coalescing fold: a warm Pack execution is allocation-free
        // (per-group claim lists pooled by group index, in-place sort,
        // sweep-driven finalize).
        let mut pack = db.prepare(&pack_query())?;
        gate("pack", &mut pack, snap, &no_params);

        let mut key_probe = db.prepare(&key_probe_query())?;
        gate("key_probe", &mut key_probe, snap, &key_probe_params);

        // The rule loop (docs/architecture/40-execution.md § the rule
        // loop): multi-rule prepared queries in the measured window —
        // per-rule sink re-aiming, the shared binding scratch, and the
        // spanning seen-set (projection and the never-elided union
        // aggregate regime) all sit at their high-water after warmup.
        let mut union_rules = db.prepare(&union_rules_query())?;
        gate("union-rules", &mut union_rules, snap, &join_params);
        let mut union_aggregate = db.prepare(&union_aggregate_query())?;
        gate("union-aggregate", &mut union_aggregate, snap, &join_params);

        // The selection shape (docs/architecture/40-execution.md): four rotating Eq params on
        // a non-key string field — the gate's warmups cover two full
        // rotation cycles, so every probed subtrie is forced and the
        // measured rotations must not touch the allocator.
        // Borrowed str payloads at the bind surface (PRD 22): the host
        // owns the strings once; every re-bind borrows them — the gate's
        // zero-allocation assertion now covers the whole bind, boxing
        // included (there is none).
        let memo_texts: Vec<String> = (0..4).map(|m| format!("memo-{m}")).collect();
        let selection_params: Vec<Vec<BindValue<'_>>> = memo_texts
            .iter()
            .map(|text| vec![BindValue::Str(text)])
            .collect();
        let mut selection = db.prepare(&selection_query())?;
        gate("selection", &mut selection, snap, &selection_params);

        // The literal latch (PRD 09): the first warmup crosses the
        // latch; the measured window is the fully-latched fast path.
        let mut latch = db.prepare(&latch_query())?;
        gate("literal-latch", &mut latch, snap, &no_params);

        // String projections across rotating params (docs/architecture/40-execution.md): the
        // intern-resolution memo joins the zero-alloc steady state.
        let account_params: Vec<Vec<BindValue<'_>>> =
            (0..4).map(|a| vec![BindValue::U64(a)]).collect();
        let mut string_rotation = db.prepare(&string_rotation_query())?;
        gate(
            "string-rotation",
            &mut string_rotation,
            snap,
            &account_params,
        );

        // The high-water window (docs/architecture/40-execution.md § CI
        // gate protocol): holders 5..10 bind the ladder accounts —
        // strictly hotter keys, strictly larger intermediates per step.
        let escalation_params: Vec<Vec<BindValue<'_>>> =
            (5..10u64).map(|h| vec![BindValue::U64(h)]).collect();
        let mut escalation = db.prepare(&escalation_query())?;
        escalation_gate("escalation", &mut escalation, snap, &escalation_params);

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

    // Borrowed structs (PRD 22): construct + typed insert and typed get +
    // compare of a str-bearing fact are host-allocation-free. The memo
    // string is committed (and its scratch warmed) before the measured
    // window, so the window holds exactly the host-visible work: encode
    // through borrows, probe, decode to a borrowed view — no `String`
    // per read, no boxing per write. Engine arena/delta copies are
    // sanctioned but absent here by construction (the value is already
    // interned; the fact already present).
    let borrowed_dir = common::TempDir::new("alloc-gate-borrowed");
    let borrowed_db = Db::create(borrowed_dir.path(), GateLedger).expect("create");
    let item = borrowed_db
        .write(|tx| {
            let id: GateItemId = tx.alloc()?;
            tx.insert(&GateItem {
                id,
                memo: "memo-borrowed",
            })?;
            Ok(id)
        })
        .expect("seed");
    borrowed_db
        .write(|tx| {
            // Warm the transaction's encode scratch outside the window.
            tx.insert(&GateItem {
                id: item,
                memo: "memo-borrowed",
            })?;
            alloc_counter::reset();
            let fact = GateItem {
                id: item,
                memo: "memo-borrowed",
            };
            tx.insert(&fact)?;
            let got = tx.get::<GateItem>(item)?.expect("present");
            assert_eq!(got.memo, "memo-borrowed");
            let bytes = alloc_counter::snapshot();
            assert_eq!(
                (
                    bytes.allocs,
                    bytes.deallocs,
                    bytes.alloc_bytes,
                    bytes.dealloc_bytes
                ),
                (0, 0, 0, 0),
                "borrowed-struct insert + get must be host-allocation-free"
            );
            Ok(())
        })
        .expect("borrowed-struct gate");
}
