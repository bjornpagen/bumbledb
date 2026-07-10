//! The chase's result-equality pins (`plan/chase.rs`): the eliminated
//! plan and the chase-disabled plan execute the same query to identical
//! result sets, projection and aggregate sinks both — the module doc's
//! bit-identical claim, exercised end to end.

use super::*;

use crate::exec::dispatch::ExecPlan;
use crate::ir::normalize::Role;
use crate::ir::AggOp;
use crate::plan::chase::with_chase_disabled;
use crate::schema::{RelationDescriptor, Side, StatementDescriptor};

/// Posting(id serial, account u64, amount i64); Account(id serial,
/// name str); Posting(account) <= Account(id) — statement 2 after the
/// two serial auto-keys.
fn chase_schema() -> Schema {
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
                        name: "name".into(),
                        value_type: ValueType::String,
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

/// Commits accounts and postings in one transaction (the containment is
/// judged on the final state, so the cluster inserts together).
fn populate(env: &Environment, schema: &Schema) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, name) in [(1u64, "cash"), (2, "fees"), (3, "rent")] {
        let name_id = delta.intern_str(&view, name).expect("intern");
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(id), ValueRef::String(name_id)],
            schema.relation(RelationId(1)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(1), &bytes).expect("insert");
    }
    // Duplicate (account, amount) pairs on purpose: the aggregate fold
    // must count both bindings (distinct posting ids), eliminated or
    // not.
    for (id, account, amount) in [
        (1u64, 1u64, 10i64),
        (2, 1, 10),
        (3, 1, -5),
        (4, 2, 40),
        (5, 2, 25),
        (6, 3, 7),
    ] {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(id),
                ValueRef::U64(account),
                ValueRef::I64(amount),
            ],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// The existence-walk atoms: Posting(id = pid, account = x, amount = m),
/// Account(id = x).
fn walk_atoms() -> Vec<Atom> {
    vec![
        Atom {
            relation: RelationId(0),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        },
        Atom {
            relation: RelationId(1),
            bindings: vec![(FieldId(0), Term::Var(VarId(1)))],
        },
    ]
}

/// The prepared plan's roles — asserting the marks so neither side of
/// the differential is vacuously equal.
fn plan_roles(prepared: &PreparedQuery<'_, ()>) -> Vec<Role> {
    let ExecPlan::FreeJoin(plan) = &prepared.plan else {
        panic!("a two-atom query plans as Free Join");
    };
    plan.occurrences().iter().map(|o| o.role).collect()
}

fn rows(buffer: &ResultBuffer) -> Vec<Vec<ResultValue<'_>>> {
    let mut rows: Vec<Vec<ResultValue<'_>>> = (0..buffer.len())
        .map(|row| {
            (0..2)
                .map(|column| buffer.get(row, column))
                .collect::<Vec<_>>()
        })
        .collect();
    rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    rows
}

/// Grading(id serial, kind enum{Det, Custom}); Det(grading u64, rate
/// i64) with the declared key Det(grading) -> Det (statement 1 after
/// Grading's auto-key 0) and the discriminated-union pair
/// `Grading(id | kind == Det) == Det(grading)` written as its two
/// containments (statements 2 and 3).
fn du_schema() -> Schema {
    let kind = ValueType::Enum {
        variants: ["Det", "Custom"].iter().map(|v| Box::from(*v)).collect(),
    };
    let side = |relation: u32, field: u16, selection: &[(u16, crate::ir::Value)]| Side {
        relation: RelationId(relation),
        projection: Box::new([FieldId(field)]),
        selection: selection
            .iter()
            .map(|(f, v)| (FieldId(*f), v.clone()))
            .collect(),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Grading".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: kind,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                name: "Det".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "grading".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "rate".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(1),
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Containment {
                source: side(0, 0, &[(1, Value::Enum(0))]),
                target: side(1, 0, &[]),
            },
            StatementDescriptor::Containment {
                source: side(1, 0, &[]),
                target: side(0, 0, &[(1, Value::Enum(0))]),
            },
        ],
    }
    .validate()
    .expect("valid fixture")
}

/// Commits the DU cluster in one transaction: three gradings (two Det,
/// one Custom) and the two Det rows the pair requires.
fn populate_du(env: &Environment, schema: &Schema) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (id, kind) in [(1u64, 0u8), (2, 0), (3, 1)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(id), ValueRef::Enum(kind)],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    for (grading, rate) in [(1u64, 25i64), (2, 40)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(grading), ValueRef::I64(rate)],
            schema.relation(RelationId(1)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(1), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit");
}

/// The EXPLAIN golden on the DU fixture (docs/prd — the chase surface):
/// the one-sided walk `Q(rate) :- Det(grading = g, rate),
/// Grading(id = g, kind == Det)` reports the header's elimination with
/// the licensing statement rendered in the `schema!` notation — the
/// mirrored pair renders `==` once — and the structured stats carry the
/// same mark as data.
#[test]
fn the_du_fixture_explain_pins_the_eliminated_line() {
    let dir = TempDir::new("chase-du-golden");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    populate_du(&env, &schema);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");
    let query = Query {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                relation: RelationId(1),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                relation: RelationId(0),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Literal(Value::Enum(0))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");

    let (rows, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
    assert_eq!(rows.len(), 2, "the two Det rates");
    assert!(
        report.contains("eliminated: Grading via Grading(id | kind == Det) == Det(grading)\n"),
        "the golden eliminated line is missing:\n{report}"
    );

    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(
        stats.eliminated,
        vec![crate::api::stats::EliminatedOccurrence {
            occurrence: 1,
            relation: "Grading".into(),
            statement: crate::schema::StatementId(3),
            rendered: "Grading(id | kind == Det) == Det(grading)".into(),
        }],
        "the structured stats carry the mark as data"
    );
}

/// Eliminated vs chase-disabled execution: identical result sets under
/// the projection sink and under the aggregate sink.
#[test]
fn eliminated_and_disabled_executions_agree_on_both_sinks() {
    let dir = TempDir::new("chase-differential");
    let schema = chase_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    populate(&env, &schema);
    let cache = ImageCache::new();
    let txn = env.read_txn().expect("txn");

    // Projection sink: Q(pid, m) — posting ids keep duplicate amounts
    // distinct, so the set comparison is over real multi-row output.
    let projection = Query {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: walk_atoms(),
        negated: vec![],
        predicates: vec![],
    };
    // Aggregate sink: Q(x, Sum(m)) — pid stays bound (not projected),
    // so the fold domain counts every posting; the eliminated plan must
    // reproduce it without ever touching Account.
    let aggregate = Query {
        finds: vec![
            FindTerm::Var(VarId(1)),
            FindTerm::Aggregate {
                op: AggOp::Sum,
                over: Some(VarId(2)),
            },
        ],
        atoms: walk_atoms(),
        negated: vec![],
        predicates: vec![],
    };

    for query in [&projection, &aggregate] {
        let mut chased = prepare(&txn, &cache, &schema, query).expect("prepare");
        assert_eq!(
            plan_roles(&chased),
            vec![
                Role::Positive,
                Role::Eliminated(crate::schema::StatementId(2))
            ],
            "the walk shape eliminates the Account occurrence"
        );
        let mut disabled =
            with_chase_disabled(|| prepare(&txn, &cache, &schema, query)).expect("prepare");
        assert_eq!(
            plan_roles(&disabled),
            vec![Role::Positive, Role::Positive],
            "the off switch keeps both occurrences joining"
        );
        let with_chase = chased.execute_collect(&txn, &cache, &[]).expect("execute");
        let without = disabled
            .execute_collect(&txn, &cache, &[])
            .expect("execute");
        assert_eq!(
            rows(&with_chase),
            rows(&without),
            "elimination is result-identical"
        );
        assert!(!with_chase.is_empty(), "the fixture produces rows");
    }
}
