use super::*;

#[test]
fn selection_params_rotate_differentially() {
    let mut state = 0xDEAD_BEEF_u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 33
    };
    let rows: Vec<(u64, u64, String, i64)> = (0..200)
        .map(|id| {
            let memo = format!("m{}", next() % 8);
            let amount = i64::try_from(id).expect("fits") * 3 - 100;
            (id, next() % 5, memo, amount)
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    let fix = postings(&borrowed);
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");
    for cycle in 0..3 {
        for m in 0..8 {
            let memo = format!("m{m}");
            let out = fix
                .execute(&mut prepared, &memo_param(&memo))
                .expect("execute");
            let mut expected: Vec<i64> = rows
                .iter()
                .filter(|(_, _, row_memo, _)| *row_memo == memo)
                .map(|(_, _, _, amount)| *amount)
                .collect();
            expected.sort_unstable();
            expected.dedup();
            assert_eq!(
                amounts_of(&out),
                expected,
                "cycle {cycle}, memo {memo} diverges from the nested loop"
            );
        }
    }

    let out = fix
        .execute(&mut prepared, &memo_param("never-stored"))
        .expect("execute");
    assert!(out.is_empty());
}

#[test]
fn selection_work_is_o_selected() {
    let rows: Vec<(u64, u64, String, i64)> = (0..20)
        .map(|id| {
            let memo = if id % 5 == 0 {
                "hot".to_owned()
            } else {
                format!("cold-{id}")
            };
            (id, id % 3, memo, i64::try_from(id).expect("fits") * 7)
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    let fix = postings(&borrowed);
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[ParamArg::Scalar(BindValue::Str("hot"))])
        .expect("execute");
    assert_eq!(out.len(), 4);
}

#[test]
fn selection_params_rotate_without_view_rebuilds() {
    // Epoch memoization is a store behavior: heap ticks rebuild per
    // execution by design, so this suite runs over one committed store.
    let fix = posting_store(
        "prepared-selection-memo",
        &[
            (1, 0, "m0", 10),
            (2, 0, "m1", 20),
            (3, 0, "m2", 30),
            (4, 0, "m0", 40),
        ],
    );
    let mut prepared = fix.prepare(&by_memo_query()).expect("prepare");

    let mut initial_binding = None;
    for _cycle in 0..3 {
        for text in ["m0", "m1", "m2", "never-stored"] {
            let out = fix
                .execute(&mut prepared, &memo_param(text))
                .expect("execute");
            assert_eq!(
                out.len(),
                match text {
                    "m0" => 2,
                    "never-stored" => 0,
                    _ => 1,
                }
            );
            let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
                panic!("free join fixture")
            };
            let Binding::Bound(bound) = &rule.memo.occs[0].active else {
                panic!("executed binding")
            };
            let state = (bound.epoch, bound.last_used);
            assert_eq!(
                state,
                *initial_binding.get_or_insert(state),
                "selection changes reuse the original binding"
            );
            assert!(rule.memo.occs[0].parked.iter().all(Option::is_none));
        }
    }
}

fn literal_comparison(relation: RelationId, field: FieldId, op: CmpOp, literal: Value) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(relation),
            bindings: vec![(field, Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Literal(literal),
        })],
    })
}

const MIXED_OPS: [CmpOp; 6] = [
    CmpOp::Lt,
    CmpOp::Le,
    CmpOp::Gt,
    CmpOp::Ge,
    CmpOp::Eq,
    CmpOp::Ne,
];

/// Host IEEE comparison: the oracle for exact mixed comparisons.
#[expect(clippy::float_cmp, reason = "exact IEEE comparison is the oracle")]
fn ieee_holds(op: CmpOp, a: f64, b: f64) -> bool {
    match op {
        CmpOp::Lt => a < b,
        CmpOp::Le => a <= b,
        CmpOp::Gt => a > b,
        CmpOp::Ge => a >= b,
        CmpOp::Eq => a == b,
        CmpOp::Ne => a != b,
        CmpOp::Allen { .. } | CmpOp::PointIn => unreachable!("scalar operators"),
    }
}

/// An integer column against F64 literals matches IEEE comparison of the exact
/// values: fractional bounds round, NaN orders and equals nothing.
#[test]
fn integer_columns_compare_exactly_against_float_literals() {
    let rows: Vec<(u64, u64, &str, i64)> = (-6i64..=6)
        .map(|amount| {
            (
                amount.unsigned_abs() * 2 + u64::from(amount < 0),
                1,
                "m",
                amount,
            )
        })
        .collect();
    let fix = postings(&rows);
    for literal in [
        2.5,
        -2.5,
        3.0,
        -0.0,
        1e-300,
        -1e-300,
        6.0,
        1e19,
        -1e19,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        for op in MIXED_OPS {
            let query = literal_comparison(
                POSTING,
                FieldId(3),
                op,
                Value::F64(crate::F64::from(literal)),
            );
            let mut prepared = fix.prepare(&query).expect("a mixed literal validates");
            let out = fix
                .execute(&mut prepared, &[] as &[BindValue])
                .expect("execute");
            let mut got: Vec<i64> = (0..out.len())
                .map(|row| match out.get(row, 0) {
                    AnswerValue::I64(amount) => amount,
                    other => panic!("i64 answers, got {other:?}"),
                })
                .collect();
            got.sort_unstable();
            let expected: Vec<i64> = rows
                .iter()
                .map(|row| row.3)
                .filter(|amount| ieee_holds(op, *amount as f64, literal))
                .collect();
            assert_eq!(got, expected, "amount {op:?} {literal}");
        }
    }
}

/// An F64 column against integer literals beyond 2^53 compares against the
/// literal's exact value, not its rounded neighbour.
#[test]
fn float_columns_compare_exactly_against_large_integer_literals() {
    let two53 = 9_007_199_254_740_992.0_f64;
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Reading".into(),
            fields: vec![FieldDescriptor {
                name: "value".into(),
                value_type: ValueType::F64,
            }],
        }],
        statements: vec![],
    };
    let values = [two53 - 1.0, two53, two53 + 2.0, -two53, -two53 - 2.0];
    let fix = Fix::heap(
        schema,
        &[(
            RelationId(0),
            values
                .iter()
                .map(|value| vec![Value::F64(crate::F64::from(*value))])
                .collect(),
        )],
    );
    let between = (1i64 << 53) + 1;
    for (literal, exact_cases) in [
        (
            Value::I64(between),
            [
                (CmpOp::Lt, 4),
                (CmpOp::Le, 4),
                (CmpOp::Gt, 1),
                (CmpOp::Ge, 1),
                (CmpOp::Eq, 0),
                (CmpOp::Ne, 5),
            ],
        ),
        (
            Value::I64(-between),
            [
                (CmpOp::Lt, 1),
                (CmpOp::Le, 1),
                (CmpOp::Gt, 4),
                (CmpOp::Ge, 4),
                (CmpOp::Eq, 0),
                (CmpOp::Ne, 5),
            ],
        ),
        (
            Value::U64(1 << 53),
            [
                (CmpOp::Lt, 3),
                (CmpOp::Le, 4),
                (CmpOp::Gt, 1),
                (CmpOp::Ge, 2),
                (CmpOp::Eq, 1),
                (CmpOp::Ne, 4),
            ],
        ),
    ] {
        for (op, count) in exact_cases {
            let query = literal_comparison(RelationId(0), FieldId(0), op, literal.clone());
            let mut prepared = fix.prepare(&query).expect("a mixed literal validates");
            let out = fix
                .execute(&mut prepared, &[] as &[BindValue])
                .expect("execute");
            assert_eq!(out.len(), count, "value {op:?} {literal:?}");
        }
    }
}

#[test]
fn integer_and_float_variables_do_not_compare() {
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Pair".into(),
            fields: vec![
                FieldDescriptor {
                    name: "count".into(),
                    value_type: ValueType::I64,
                },
                FieldDescriptor {
                    name: "ratio".into(),
                    value_type: ValueType::F64,
                },
            ],
        }],
        statements: vec![],
    };
    let fix = Fix::heap(schema, &[]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Var(VarId(1)),
        })],
    });
    let Err(error) = fix.prepare(&query) else {
        panic!("a mixed variable comparison is refused")
    };
    assert!(matches!(
        error,
        Error::Validation(crate::error::ValidationError::Comparison {
            index: 0,
            refusal: crate::ir::validate::error::ComparisonRefusal::MixedNumeric,
        })
    ));
    assert!(error.to_string().contains("toF64Exact"));
}
