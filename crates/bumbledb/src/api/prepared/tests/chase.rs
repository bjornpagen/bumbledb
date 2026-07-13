//! The chase's result-equality pins (`plan/chase.rs`): the eliminated
//! plan and the chase-disabled plan execute the same query to identical
//! result sets, projection and aggregate sinks both — the module doc's
//! bit-identical claim, exercised end to end.

use super::*;

use crate::ir::AggOp;
use crate::ir::normalize::Role;
use crate::plan::chase::with_chase_disabled;
use crate::schema::{RelationDescriptor, Side, StatementDescriptor};

/// Posting(id fresh, account u64, amount i64); Account(id fresh,
/// name str); Posting(account) <= Account(id) — statement 2 after the
/// two fresh auto-keys.
fn chase_schema() -> Schema {
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

/// One prepared rule's roles — asserting the marks so neither side of
/// the differential is vacuously equal.
fn plan_roles(prepared: &PreparedQuery<'_, ()>, rule: usize) -> Vec<Role> {
    let PreparedRule::FreeJoin(rule) = &prepared.program.rules()[rule] else {
        panic!("a two-atom query plans as Free Join");
    };
    rule.plan.occurrences().iter().map(|o| o.role).collect()
}

fn rows(buffer: &ResultBuffer) -> Vec<Vec<ResultValue<'_>>> {
    let mut rows: Vec<Vec<ResultValue<'_>>> = (0..buffer.len())
        .map(|row| {
            (0..buffer.arity)
                .map(|column| buffer.get(row, column))
                .collect::<Vec<_>>()
        })
        .collect();
    rows.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    rows
}

/// Grading(id fresh, kind u64 — 0 = Det); Det(grading u64, rate
/// i64) with the declared key Det(grading) -> Det (statement 1 after
/// Grading's auto-key 0) and the discriminated-union pair
/// `Grading(id | kind == 0) == Det(grading)` written as its two
/// containments (statements 2 and 3).
fn du_schema() -> Schema {
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
                extension: None,
                name: "Grading".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Fresh,
                    },
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
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
                source: side(0, 0, &[(1, Value::U64(0))]),
                target: side(1, 0, &[]),
            },
            StatementDescriptor::Containment {
                source: side(1, 0, &[]),
                target: side(0, 0, &[(1, Value::U64(0))]),
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
    for (id, kind) in [(1u64, 0u64), (2, 0), (3, 1)] {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(id), ValueRef::U64(kind)],
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

/// The EXPLAIN golden on the DU fixture
/// (`docs/architecture/40-execution.md` § the chase):
/// the one-sided walk `Q(rate) :- Det(grading = g, rate),
/// Grading(id = g, kind == 0)` reports the header's elimination with
/// the licensing statement rendered in the `schema!` notation — the
/// mirrored pair renders `==` once — and the structured stats carry the
/// same mark as data.
#[test]
fn the_du_fixture_explain_pins_the_eliminated_line() {
    let dir = TempDir::new("chase-du-golden");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    populate_du(&env, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
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
                    (FieldId(1), Term::Literal(Value::U64(0))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");

    let (rows, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
    assert_eq!(rows.len(), 2, "the two Det rates");
    assert!(
        report.contains("eliminated: Grading via Grading(id | kind == 0) == Det(grading)\n"),
        "the golden eliminated line is missing:\n{report}"
    );

    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(
        stats.rules[0].eliminated,
        vec![crate::api::stats::EliminatedOccurrence {
            occurrence: 1,
            relation: "Grading".into(),
            statement: crate::schema::StatementId(3),
            rendered: "Grading(id | kind == 0) == Det(grading)".into(),
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
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    // Projection sink: Q(pid, m) — posting ids keep duplicate amounts
    // distinct, so the set comparison is over real multi-row output.
    let projection = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: walk_atoms(),
        negated: vec![],
        predicates: vec![],
    });
    // Aggregate sink: Q(x, Sum(m)) — pid stays bound (not projected),
    // so the fold domain counts every posting; the eliminated plan must
    // reproduce it without ever touching Account.
    let aggregate = Query::single(Rule {
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
    });

    for query in [&projection, &aggregate] {
        let mut chased = prepare(&txn, &cache, &schema, query).expect("prepare");
        assert_eq!(
            plan_roles(&chased, 0),
            vec![
                Role::Positive,
                Role::Eliminated(crate::schema::StatementId(2))
            ],
            "the walk shape eliminates the Account occurrence"
        );
        let mut disabled =
            with_chase_disabled(|| prepare(&txn, &cache, &schema, query)).expect("prepare");
        assert_eq!(
            plan_roles(&disabled, 0),
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

/// The chase runs per rule, independently: a two-rule union where the
/// walk's Account occurrence is containment-implied in rule 0 but
/// filter-blocked in rule 1 (an extra selection beyond ψ — condition
/// 2), so the mark stays rule-local, no rule subsumes the other, and
/// the off switch changes no results.
#[test]
fn per_rule_elimination_marks_one_rule_only() {
    let dir = TempDir::new("chase-per-rule");
    let schema = chase_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    populate(&env, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    // rule 0: Q(pid, m) :- Posting(pid, x, m), Account(id = x);
    // rule 1: the same walk with Account(name == "cash") — the extra
    // target selection refuses elimination in that rule alone.
    let rule = |name_filter: bool| {
        let mut atoms = walk_atoms();
        if name_filter {
            atoms[1].bindings.push((
                FieldId(1),
                Term::Literal(Value::String(Box::from(&b"cash"[..]))),
            ));
        }
        Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
            atoms,
            negated: vec![],
            predicates: vec![],
        }
    };
    let query = Query {
        head: rule(false).head(),
        rules: vec![rule(false), rule(true)],
    };
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert_eq!(
        prepared.program.rules().len(),
        2,
        "differing bodies never subsume"
    );
    assert_eq!(
        plan_roles(&prepared, 0),
        vec![
            Role::Positive,
            Role::Eliminated(crate::schema::StatementId(2))
        ],
        "the unfiltered walk eliminates its Account occurrence"
    );
    assert_eq!(
        plan_roles(&prepared, 1),
        vec![Role::Positive, Role::Positive],
        "the filtered rule keeps its Account occurrence — no cross-rule state"
    );
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert!(stats.subsumed.is_empty(), "no rule was deleted");

    let mut disabled =
        with_chase_disabled(|| prepare(&txn, &cache, &schema, &query)).expect("prepare");
    assert_eq!(
        plan_roles(&disabled, 0),
        vec![Role::Positive, Role::Positive],
        "the off switch keeps every occurrence joining"
    );
    let with_chase = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let without = disabled
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        rows(&with_chase),
        rows(&without),
        "per-rule elimination is result-identical"
    );
    assert!(!with_chase.is_empty(), "the fixture produces rows");
}

/// The DNF residue: lowering `(rate > 30 ∨ kind == Det)` over the DU
/// walk produces a rule pair where elimination discharges the second
/// disjunct's `kind` filter with the Grading occurrence itself — the
/// filterless rule subsumes the rate-filtered one, the subsumed rule is
/// deleted at prepare, results are identical with the passes off, and
/// EXPLAIN names the deletion with the subsuming rule's index.
#[test]
fn dnf_residue_subsumption_deletes_the_filtered_rule() {
    let dir = TempDir::new("chase-subsume");
    let schema = du_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    populate_du(&env, &schema);
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
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
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        predicates: vec![PredicateTree::Or(vec![
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Gt,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::I64(30)),
            }),
            PredicateTree::Leaf(Comparison {
                op: CmpOp::Eq,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Literal(Value::U64(0)),
            }),
        ])],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert_eq!(
        prepared.program.rules().len(),
        1,
        "the subsumed disjunct is deleted"
    );
    assert_eq!(
        plan_roles(&prepared, 0),
        vec![
            Role::Positive,
            Role::Eliminated(crate::schema::StatementId(3))
        ],
        "the survivor still carries its own elimination mark"
    );

    let (results, report) = prepared.explain(&txn, &cache, &[]).expect("explain");
    assert_eq!(results.len(), 2, "the two Det rates");
    assert!(
        report.contains("subsumed: rule 0 by rule 1\n"),
        "EXPLAIN names the deletion with the subsuming rule's index:\n{report}"
    );
    let (_, stats) = prepared.profile(&txn, &cache, &[]).expect("profile");
    assert_eq!(
        stats.subsumed,
        vec![crate::api::stats::SubsumedRule { rule: 0, by: 1 }],
        "the structured stats carry the record as data"
    );

    let mut disabled =
        with_chase_disabled(|| prepare(&txn, &cache, &schema, &query)).expect("prepare");
    assert_eq!(
        disabled.program.rules().len(),
        2,
        "the off switch covers both passes: no elimination, no deletion"
    );
    let with_passes = prepared
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    let without = disabled
        .execute_collect(&txn, &cache, &[])
        .expect("execute");
    assert_eq!(
        rows(&with_passes),
        rows(&without),
        "subsumption is result-identical"
    );
    assert!(!with_passes.is_empty(), "the fixture produces rows");
}
