use super::*;
use crate::api::prepared::ParamArg;
use crate::error::AtomIndex;
use crate::ir::ParamId;

fn by_account_set_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn by_account_scalar_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Param(ParamId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

fn id_amount_answers(buffer: &Answers) -> Vec<(u64, i64)> {
    let mut answers: Vec<(u64, i64)> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::U64(id) = buffer.get(answer, 0) else {
                panic!("column 0 is a u64 id");
            };
            let AnswerValue::I64(amount) = buffer.get(answer, 1) else {
                panic!("column 1 is an i64 amount");
            };
            (id, amount)
        })
        .collect();
    answers.sort_unstable();
    answers
}

#[test]
fn in_family_equals_the_union_of_per_element_executions() {
    let rows: Vec<(u64, u64, String, i64)> = (0..600u64)
        .map(|i| {
            let amount = i64::try_from(i).expect("small") * 3 - 100;
            (i, i % 250, format!("m{}", i % 5), amount)
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    let fix = postings(&borrowed);

    let mut set_query = fix.prepare(&by_account_set_query()).expect("prepare");
    let mut scalar_query = fix.prepare(&by_account_scalar_query()).expect("prepare");

    for size in [0usize, 1, 2, 200] {
        let elements: Vec<Value> = (0..size)
            .map(|k| Value::U64(u64::try_from(k).expect("small") * 7))
            .collect();
        let got = fix
            .execute(&mut set_query, &[ParamArg::Set(&elements)])
            .expect("set execution");

        let mut union: Vec<(u64, i64)> = Vec::new();
        for element in &elements {
            let Value::U64(account) = element else {
                unreachable!("the elements are U64 accounts")
            };
            let per = fix
                .execute(&mut scalar_query, &[BindValue::U64(*account)])
                .expect("scalar execution");
            union.extend(id_amount_answers(&per));
        }
        union.sort_unstable();
        union.dedup();
        assert_eq!(id_amount_answers(&got), union, "size {size}");
        if size == 0 {
            assert!(got.is_empty(), "the empty set matches nothing");
        }
    }

    let dup = [Value::U64(7), Value::U64(7), Value::U64(7)];
    let once = [Value::U64(7)];
    let got_dup = fix
        .execute(&mut set_query, &[ParamArg::Set(&dup)])
        .expect("execute");
    let got_once = fix
        .execute(&mut set_query, &[ParamArg::Set(&once)])
        .expect("execute");
    assert_eq!(id_amount_answers(&got_dup), id_amount_answers(&got_once));

    let err = fix
        .execute(&mut set_query, &[BindValue::U64(7)])
        .unwrap_err();
    assert!(matches!(err, Error::ParamSetExpected { param } if param.0 == 0));
}

#[test]
fn profile_binds_param_sets_exactly_as_execute_args() {
    let rows: Vec<(u64, u64, String, i64)> = (0..600u64)
        .map(|i| {
            let amount = i64::try_from(i).expect("small") * 3 - 100;
            (i, i % 250, format!("m{}", i % 5), amount)
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    let fix = postings(&borrowed);

    let mut prepared = fix.prepare(&by_account_set_query()).expect("prepare");
    let elements = [Value::U64(7), Value::U64(14)];
    let executed = fix
        .execute(&mut prepared, &[ParamArg::Set(&elements)])
        .expect("execute");
    assert!(!executed.is_empty(), "the fixture selects rows");
}

#[test]
fn out_of_vocabulary_string_elements_contribute_nothing() {
    let fix = postings(&[
        (1, 7, "rent", -1200),
        (2, 7, "salary", 5000),
        (3, 8, "rent", -900),
    ]);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::ParamSet(ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");

    let string = |text: &str| Value::String(text.into());
    let with_ghost = [string("rent"), string("ghost")];
    let rent_only = [string("rent")];
    let all_ghost = [string("ghost"), string("phantom")];

    let got = fix
        .execute(&mut prepared, &[ParamArg::Set(&with_ghost)])
        .expect("execute");
    let control = fix
        .execute(&mut prepared, &[ParamArg::Set(&rent_only)])
        .expect("execute");
    assert_eq!(amounts_of(&got), amounts_of(&control));
    assert_eq!(amounts_of(&got), vec![-1200, -900]);

    let empty = fix
        .execute(&mut prepared, &[ParamArg::Set(&all_ghost)])
        .expect("execute");
    assert!(empty.is_empty(), "an all-unstored set matches nothing");
}

fn interval_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Payroll".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: bumbledb_theory::schema::IntervalElement::U64,
                        },
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Event".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "emp".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "at".into(),
                        value_type: ValueType::U64,
                    },
                ],
            },
        ],
        statements: vec![],
    }
}

const PAYROLL: RelationId = RelationId(0);
const EVENT: RelationId = RelationId(1);

fn interval_fix() -> Fix {
    let payroll: Vec<Vec<Value>> = [(1u64, 10u64, 20u64), (2, 30, 40)]
        .into_iter()
        .map(|(emp, start, end)| {
            vec![
                Value::U64(emp),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval"),
                ),
            ]
        })
        .collect();
    let events: Vec<Vec<Value>> = [(1u64, 9u64), (1, 10), (1, 19), (1, 20), (2, 35), (3, 15)]
        .into_iter()
        .map(|(emp, at)| vec![Value::U64(emp), Value::U64(at)])
        .collect();
    Fix::heap(
        interval_descriptor(),
        &[(PAYROLL, payroll), (EVENT, events)],
    )
}

fn u64_pairs(buffer: &Answers) -> Vec<(u64, u64)> {
    let mut answers: Vec<(u64, u64)> = (0..buffer.len())
        .map(|answer| {
            let AnswerValue::U64(a) = buffer.get(answer, 0) else {
                panic!("column 0 is u64");
            };
            let AnswerValue::U64(b) = buffer.get(answer, 1) else {
                panic!("column 1 is u64");
            };
            (a, b)
        })
        .collect();
    answers.sort_unstable();
    answers
}

#[test]
fn membership_point_var_join_end_to_end() {
    let fix = interval_fix();

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(PAYROLL),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(EVENT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let got = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(
        u64_pairs(&got),
        vec![(1, 10), (1, 19), (2, 35)],
        "start inclusive, end exclusive, per employee"
    );
}

#[test]
fn set_membership_matches_any_element() {
    let fix = interval_fix();

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let emps = |buffer: &Answers| {
        let mut out: Vec<u64> = (0..buffer.len())
            .map(|answer| {
                let AnswerValue::U64(emp) = buffer.get(answer, 0) else {
                    panic!("column 0 is u64");
                };
                emp
            })
            .collect();
        out.sort_unstable();
        out
    };
    let run = |fix: &Fix, prepared: &mut PreparedQuery<T>, points: &[u64]| {
        let values: Vec<Value> = points.iter().map(|p| Value::U64(*p)).collect();
        let got = fix
            .execute(prepared, &[ParamArg::Set(&values)])
            .expect("execute");
        emps(&got)
    };
    assert_eq!(run(&fix, &mut prepared, &[10]), vec![1], "start is in");
    assert_eq!(
        run(&fix, &mut prepared, &[20]),
        Vec::<u64>::new(),
        "end is out"
    );
    assert_eq!(
        run(&fix, &mut prepared, &[19, 39]),
        vec![1, 2],
        "any element"
    );
    assert_eq!(run(&fix, &mut prepared, &[25]), Vec::<u64>::new(), "gap");
    assert_eq!(
        run(&fix, &mut prepared, &[]),
        Vec::<u64>::new(),
        "empty set"
    );
}

fn ray_fix() -> Fix {
    Fix::heap(
        interval_descriptor(),
        &[(
            PAYROLL,
            vec![vec![
                Value::U64(1),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(10, u64::MAX).expect("nonempty interval"),
                ),
            ]],
        )],
    )
}

fn membership_literal_query(point: u64) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Literal(Value::U64(point))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn membership_of_the_last_point_in_a_ray_is_true_and_the_ceiling_rejects() {
    let fix = ray_fix();

    let mut prepared = fix
        .prepare(&membership_literal_query(u64::MAX - 1))
        .expect("prepare");
    let got = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(got.len(), 1, "MAX-1 is a point of [10, \u{221e})");

    let Err(err) = fix.prepare(&membership_literal_query(u64::MAX)) else {
        panic!("the ceiling is not a point");
    };
    assert!(
        matches!(
            err,
            Error::Validation(crate::error::ValidationError::PointLiteralAtCeiling {
                atom: AtomIndex(0),
                field: FieldId(1),
            })
        ),
        "got {err:?}"
    );
}

#[test]
fn point_param_at_the_ceiling_is_a_bind_error() {
    let fix = ray_fix();

    let scalar_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(PAYROLL),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(EVENT),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Param(ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&scalar_query).expect("prepare");
    let err = fix
        .execute(&mut prepared, &[ParamArg::Scalar(BindValue::U64(u64::MAX))])
        .expect_err("the ceiling is not a point");
    assert!(
        matches!(err, Error::PointParamAtCeiling { param: ParamId(0) }),
        "got {err:?}"
    );

    let set_query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&set_query).expect("prepare");
    let ceiling = [Value::U64(u64::MAX)];
    let err = fix
        .execute(&mut prepared, &[ParamArg::Set(&ceiling)])
        .expect_err("the ceiling is not a point");
    assert!(
        matches!(err, Error::PointParamAtCeiling { param: ParamId(0) }),
        "got {err:?}"
    );
    let last_point = [Value::U64(u64::MAX - 1)];
    let got = fix
        .execute(&mut prepared, &[ParamArg::Set(&last_point)])
        .expect("execute");
    assert_eq!(got.len(), 1, "MAX-1 is a point of [10, \u{221e})");
}

fn block_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Posting".into(),
                fields: vec![
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
                name: "Block".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "account".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "kind".into(),
                        value_type: ValueType::U64,
                    },
                ],
            },
        ],
        statements: vec![],
    }
}

#[test]
fn negated_set_bindings_reject_under_any_element() {
    let posting_rows: Vec<Vec<Value>> = [(7u64, 70i64), (8, 80), (9, 90)]
        .into_iter()
        .map(|(account, amount)| vec![Value::U64(account), Value::I64(amount)])
        .collect();
    let block_rows: Vec<Vec<Value>> = [(7u64, 1u64), (8, 5)]
        .into_iter()
        .map(|(account, kind)| vec![Value::U64(account), Value::U64(kind)])
        .collect();
    let fix = Fix::heap(
        block_descriptor(),
        &[(RelationId(0), posting_rows), (RelationId(1), block_rows)],
    );

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(1)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::ParamSet(ParamId(0))),
            ],
        }],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let run = |fix: &Fix, prepared: &mut PreparedQuery<T>, kinds: &[u64]| {
        let values: Vec<Value> = kinds.iter().map(|k| Value::U64(*k)).collect();
        let got = fix
            .execute(prepared, &[ParamArg::Set(&values)])
            .expect("execute");
        amounts_of(&got)
    };

    assert_eq!(run(&fix, &mut prepared, &[1, 2]), vec![80, 90]);

    assert_eq!(run(&fix, &mut prepared, &[5]), vec![70, 90]);

    assert_eq!(run(&fix, &mut prepared, &[1, 5]), vec![90]);

    assert_eq!(run(&fix, &mut prepared, &[3, 4]), vec![70, 80, 90]);

    assert_eq!(run(&fix, &mut prepared, &[]), vec![70, 80, 90]);
}
