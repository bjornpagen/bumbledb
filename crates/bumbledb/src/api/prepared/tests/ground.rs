use super::*;
use crate::ir::FoldOp;

use crate::ir::normalize::Role;
use crate::plan::ground::with_grounding_disabled;
use bumbledb_theory::schema::{RelationDescriptor, Side, StatementDescriptor};

/// Posting(id, account, amount) with the declared id key; Account(id, name)
/// with the declared id key; Posting(account) <= Account(id) — statement 2
/// after the two declared keys (the deleted fresh auto-keys' positions,
/// preserved as declared statements).
fn ground_descriptor() -> SchemaDescriptor {
    let key = |relation: u32| StatementDescriptor::Functionality {
        relation: RelationId(relation),
        projection: Box::new([FieldId(0)]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
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
                    },
                    FieldDescriptor {
                        name: "name".into(),
                        value_type: ValueType::String,
                    },
                ],
            },
        ],
        statements: vec![
            key(0),
            key(1),
            StatementDescriptor::Containment {
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
            },
        ],
    }
}

fn ground_fix() -> Fix {
    let accounts: Vec<Vec<Value>> = [(1u64, "cash"), (2, "fees"), (3, "rent")]
        .into_iter()
        .map(|(id, name)| vec![Value::U64(id), Value::String(name.into())])
        .collect();
    let postings: Vec<Vec<Value>> = [
        (1u64, 1u64, 10i64),
        (2, 1, 10),
        (3, 1, -5),
        (4, 2, 40),
        (5, 2, 25),
        (6, 3, 7),
    ]
    .into_iter()
    .map(|(id, account, amount)| vec![Value::U64(id), Value::U64(account), Value::I64(amount)])
    .collect();
    Fix::heap(
        ground_descriptor(),
        &[(RelationId(0), postings), (RelationId(1), accounts)],
    )
}

fn walk_atoms() -> Vec<Atom> {
    vec![
        Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        },
        Atom {
            source: crate::ir::AtomSource::Edb(RelationId(1)),
            bindings: vec![(FieldId(0), Term::Var(VarId(1)))],
        },
    ]
}

fn plan_roles<S>(prepared: &PreparedQuery<S>, rule: usize) -> Vec<Role> {
    let PreparedRule::FreeJoin(rule) = &prepared.pipeline.main_rules()[rule] else {
        panic!("a two-atom query plans as Free Join");
    };
    rule.plan
        .occurrences()
        .iter()
        .map(|o| o.role.clone())
        .collect()
}

fn answers(buffer: &Answers) -> Vec<Vec<AnswerValue<'_>>> {
    let mut answers: Vec<Vec<AnswerValue<'_>>> = (0..buffer.len())
        .map(|answer| {
            (0..buffer.arity)
                .map(|column| buffer.get(answer, column))
                .collect::<Vec<_>>()
        })
        .collect();
    answers.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    answers
}

/// Grading(id, kind — 0 = Det) with the declared id key (statement 0, the
/// deleted auto-key's position); Det(grading u64, rate i64) with the
/// declared key Det(grading) -> Det (statement 1) and the discriminated-
/// union pair `Grading(id | kind == 0) == Det(grading)` written as its two
/// containments (statements 2 and 3).
fn du_descriptor() -> SchemaDescriptor {
    let side = |relation: u32, field: u16, selection: &[(u16, crate::ir::Value)]| Side {
        relation: RelationId(relation),
        projection: Box::new([FieldId(field)]),
        selection: selection
            .iter()
            .map(|(f, v)| {
                (
                    FieldId(*f),
                    bumbledb_theory::schema::LiteralSet::One(v.clone()),
                )
            })
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
                    },
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: ValueType::U64,
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
                    },
                    FieldDescriptor {
                        name: "rate".into(),
                        value_type: ValueType::I64,
                    },
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: RelationId(0),
                projection: Box::new([FieldId(0)]),
            },
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
}

fn du_fix() -> Fix {
    let gradings: Vec<Vec<Value>> = [(1u64, 0u64), (2, 0), (3, 1)]
        .into_iter()
        .map(|(id, kind)| vec![Value::U64(id), Value::U64(kind)])
        .collect();
    let dets: Vec<Vec<Value>> = [(1u64, 25i64), (2, 40)]
        .into_iter()
        .map(|(grading, rate)| vec![Value::U64(grading), Value::I64(rate)])
        .collect();
    Fix::heap(
        du_descriptor(),
        &[(RelationId(0), gradings), (RelationId(1), dets)],
    )
}

#[test]
fn the_du_fixture_introspection_pins_the_eliminated_line() {
    let fix = du_fix();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Literal(Value::U64(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 2, "the two Det rates");
}

#[test]
fn eliminated_and_disabled_executions_agree_on_both_sinks() {
    let fix = ground_fix();

    let projection = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
        atoms: walk_atoms(),
        negated: vec![],
        conditions: vec![],
    });

    let aggregate = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(1)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(2),
            },
        ],
        atoms: walk_atoms(),
        negated: vec![],
        conditions: vec![],
    });

    for query in [&projection, &aggregate] {
        let mut grounded = fix.prepare(query).expect("prepare");
        assert_eq!(
            plan_roles(&grounded, 0),
            vec![
                Role::Positive,
                Role::Eliminated(bumbledb_theory::schema::StatementId(2))
            ],
            "the walk shape eliminates the Account occurrence"
        );
        let mut disabled = with_grounding_disabled(|| fix.prepare(query)).expect("prepare");
        assert_eq!(
            plan_roles(&disabled, 0),
            vec![Role::Positive, Role::Positive],
            "the off switch keeps both occurrences joining"
        );
        let with_grounding = fix
            .execute(&mut grounded, &[] as &[BindValue])
            .expect("execute");
        let without = fix
            .execute(&mut disabled, &[] as &[BindValue])
            .expect("execute");
        assert_eq!(
            answers(&with_grounding),
            answers(&without),
            "elimination is result-identical"
        );
        assert!(!with_grounding.is_empty(), "the fixture produces rows");
    }
}

/// `A(id fresh, b_ref u64)`; `B(id fresh, c_ref u64)`; `C(id fresh)`; `A(b_ref)
/// <= B(id)` (statement 3 after the three fresh auto-keys), `B(c_ref) <= C(id)`
/// (statement 4) — the `A<=B<=C` chain fixture (the plan-level twin lives in
/// `plan/ground/tests.rs: chain_schema`).
fn chain_descriptor() -> SchemaDescriptor {
    let containment = |source: u32, target: u32| StatementDescriptor::Containment {
        source: Side {
            relation: RelationId(source),
            projection: Box::new([FieldId(1)]),
            selection: Box::new([]),
        },
        target: Side {
            relation: RelationId(target),
            projection: Box::new([FieldId(0)]),
            selection: Box::new([]),
        },
    };
    let key = |relation: u32| StatementDescriptor::Functionality {
        relation: RelationId(relation),
        projection: Box::new([FieldId(0)]),
    };
    let plain = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "A".into(),
                fields: vec![plain("id"), plain("b_ref")],
            },
            RelationDescriptor {
                extension: None,
                name: "B".into(),
                fields: vec![plain("id"), plain("c_ref")],
            },
            RelationDescriptor {
                extension: None,
                name: "C".into(),
                fields: vec![plain("id")],
            },
        ],
        statements: vec![key(0), key(1), key(2), containment(0, 1), containment(1, 2)],
    }
}

/// The chained elimination executed end to end (the empirical arm of
/// `lean/Bumbledb/Exec/Rewrites.lean: Query.chained_elimination_sound`): on the
/// `A<=B<=C` chain, `B` falls with `A` as its pairing source and `C` falls with
/// the already-eliminated `B` as its source — the plan keeps one occurrence —
/// and the execution's answers are identical to the grounding-disabled
/// three-way join's.
#[test]
fn a_chained_elimination_executes_result_identical_to_the_disabled_plan() {
    let c_rows: Vec<Vec<Value>> = [1u64, 2]
        .into_iter()
        .map(|id| vec![Value::U64(id)])
        .collect();
    let b_rows: Vec<Vec<Value>> = [(10u64, 1u64), (11, 2)]
        .into_iter()
        .map(|(id, c_ref)| vec![Value::U64(id), Value::U64(c_ref)])
        .collect();
    let a_rows: Vec<Vec<Value>> = [(100u64, 10u64), (101, 11), (102, 10)]
        .into_iter()
        .map(|(id, b_ref)| vec![Value::U64(id), Value::U64(b_ref)])
        .collect();
    let fix = Fix::heap(
        chain_descriptor(),
        &[
            (RelationId(0), a_rows),
            (RelationId(1), b_rows),
            (RelationId(2), c_rows),
        ],
    );

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(1))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(2)),
                bindings: vec![(FieldId(0), Term::Var(VarId(2)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut grounded = fix.prepare(&query).expect("prepare");
    assert_eq!(
        plan_roles(&grounded, 0),
        vec![
            Role::Positive,
            Role::Eliminated(bumbledb_theory::schema::StatementId(3)),
            Role::Eliminated(bumbledb_theory::schema::StatementId(4)),
        ],
        "the chain eliminates both targets, each mark carrying its own containment"
    );
    let mut disabled = with_grounding_disabled(|| fix.prepare(&query)).expect("prepare");
    assert_eq!(
        plan_roles(&disabled, 0),
        vec![Role::Positive, Role::Positive, Role::Positive],
        "the off switch keeps all three occurrences joining"
    );
    let with_grounding = fix
        .execute(&mut grounded, &[] as &[BindValue])
        .expect("execute");
    let without = fix
        .execute(&mut disabled, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        answers(&with_grounding),
        answers(&without),
        "the chained elimination is result-identical"
    );
    assert_eq!(with_grounding.len(), 3, "every A row survives the walk");
}

#[test]
fn per_rule_elimination_marks_one_rule_only() {
    let fix = ground_fix();

    let rule = |name_filter: bool| {
        let mut atoms = walk_atoms();
        if name_filter {
            atoms[1]
                .bindings
                .push((FieldId(1), Term::Literal(Value::String(Box::from("cash")))));
        }
        Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(2))],
            atoms,
            negated: vec![],
            conditions: vec![],
        }
    };
    let query = Query {
        interiors: vec![],
        head: rule(false).head(),
        rules: vec![rule(false), rule(true)],
        rec: None,
    };
    let mut prepared = fix.prepare(&query).expect("prepare");
    assert_eq!(
        prepared.pipeline.main_rules().len(),
        2,
        "differing bodies never subsume"
    );
    assert_eq!(
        plan_roles(&prepared, 0),
        vec![
            Role::Positive,
            Role::Eliminated(bumbledb_theory::schema::StatementId(2))
        ],
        "the unfiltered walk eliminates its Account occurrence"
    );
    assert_eq!(
        plan_roles(&prepared, 1),
        vec![Role::Positive, Role::Positive],
        "the filtered rule keeps its Account occurrence — no cross-rule state"
    );
    let mut disabled = with_grounding_disabled(|| fix.prepare(&query)).expect("prepare");
    assert_eq!(
        plan_roles(&disabled, 0),
        vec![Role::Positive, Role::Positive],
        "the off switch keeps every occurrence joining"
    );
    let with_grounding = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let without = fix
        .execute(&mut disabled, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        answers(&with_grounding),
        answers(&without),
        "per-rule elimination is result-identical"
    );
    assert!(!with_grounding.is_empty(), "the fixture produces rows");
}

#[test]
fn dnf_residue_subsumption_deletes_the_filtered_rule() {
    let fix = du_fix();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Or(vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Gt,
                lhs: Term::Var(VarId(1)),
                rhs: Term::Literal(Value::I64(30)),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Eq,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Literal(Value::U64(0)),
            }),
        ])],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    assert_eq!(
        prepared.pipeline.main_rules().len(),
        1,
        "the subsumed disjunct is deleted"
    );
    assert_eq!(
        plan_roles(&prepared, 0),
        vec![
            Role::Positive,
            Role::Eliminated(bumbledb_theory::schema::StatementId(3))
        ],
        "the survivor still carries its own elimination mark"
    );

    let results = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(results.len(), 2, "the two Det rates");

    let mut disabled = with_grounding_disabled(|| fix.prepare(&query)).expect("prepare");
    assert_eq!(
        disabled.pipeline.main_rules().len(),
        2,
        "the off switch covers both passes: no elimination, no deletion"
    );
    let with_passes = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let without = fix
        .execute(&mut disabled, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        answers(&with_passes),
        answers(&without),
        "subsumption is result-identical"
    );
    assert!(!with_passes.is_empty(), "the fixture produces rows");
}
