//! (validate → plan → execute → result buffer), including interval finds.
use super::*;
use crate::ir::FoldOp;
use bumbledb_theory::schema::IntervalElement;

fn interval_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Payroll".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "emp".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "during".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::I64,
                    },
                },
            ],
        }],
        statements: vec![],
    }
}

const PAYROLL: RelationId = RelationId(0);

fn scalar_set_folds(set_selection: bool) -> Query {
    let mut bindings = vec![(FieldId(3), Term::Var(VarId(1)))];
    if set_selection {
        bindings.push((FieldId(1), Term::ParamSet(crate::ir::ParamId(0))));
    } else {
        bindings.push((FieldId(1), Term::Var(VarId(0))));
    }
    let mut finds = vec![
        FindTerm::Aggregate {
            op: FoldOp::Sum,
            over: VarId(1),
        },
        FindTerm::Count,
    ];
    if !set_selection {
        finds.insert(0, FindTerm::Var(VarId(0)));
    }
    Query::single(Rule {
        finds,
        atoms: vec![Atom {
            source: AtomSource::Edb(POSTING),
            bindings,
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn physical_scalar_folds_drop_hidden_fact_multiplicity_but_fallback_keeps_dedup() {
    let fix = postings(&[
        (1, 10, "a", 5),
        (2, 10, "b", 5),
        (3, 20, "a", 5),
        (4, 20, "b", 7),
    ]);
    let mut prepared = fix.prepare(&scalar_set_folds(false)).unwrap();
    let PreparedRule::FreeJoin(rule) = &prepared.pipeline.main_rules()[0] else {
        panic!("free join")
    };
    assert!(rule.plan.distinct_witness().is_none());
    assert!(rule.plan.scalar_set_traversal().is_some());
    for fallback in [false, true, false] {
        prepared.force_cursor_fallback(fallback);
        let out = fix.execute(&mut prepared, &[] as &[BindValue]).unwrap();
        let mut rows: Vec<_> = (0..out.len())
            .map(|row| {
                let (AnswerValue::U64(group), AnswerValue::I64(sum), AnswerValue::U64(count)) =
                    (out.get(row, 0), out.get(row, 1), out.get(row, 2))
                else {
                    panic!("typed aggregates")
                };
                (group, sum, count)
            })
            .collect();
        rows.sort_unstable();
        assert_eq!(rows, vec![(10, 5, 1), (20, 12, 2)]);
        let EitherSink::Aggregate(sink) = &prepared.sink else {
            panic!("aggregate sink")
        };
        assert!(!sink.seen_elided(), "the semantic witness was never forged");
        assert_eq!(sink.distinct_seen(), Some(if fallback { 3 } else { 0 }));
    }
}

#[test]
fn physical_scalar_folds_deduplicate_across_set_selection_values_and_rebinds() {
    let fix = postings(&[
        (1, 10, "a", 5),
        (2, 10, "b", 5),
        (3, 20, "a", 5),
        (4, 20, "b", 7),
    ]);
    let mut prepared = fix.prepare(&scalar_set_folds(true)).unwrap();
    for fallback in [false, true, false] {
        prepared.force_cursor_fallback(fallback);
        for (elements, sum, count) in [
            (vec![Value::U64(10), Value::U64(20), Value::U64(10)], 12, 2),
            (vec![Value::U64(10)], 5, 1),
        ] {
            let out = fix
                .execute(&mut prepared, &[ParamArg::Set(&elements)])
                .unwrap();
            assert_eq!(out.len(), 1);
            assert!(matches!(out.get(0, 0), AnswerValue::I64(value) if value == sum));
            assert!(matches!(out.get(0, 1), AnswerValue::U64(value) if value == count));
        }
    }
}

fn payroll(rows: &[(u64, u64, (i64, i64))]) -> Fix {
    let facts: Vec<Vec<Value>> = rows
        .iter()
        .map(|(id, emp, (start, end))| {
            vec![
                Value::U64(*id),
                Value::U64(*emp),
                Value::IntervalI64(
                    bumbledb_theory::Interval::<i64>::new(*start, *end).expect("nonempty interval"),
                ),
            ]
        })
        .collect();
    Fix::heap(interval_descriptor(), &[(PAYROLL, facts)])
}

#[test]
fn interval_find_round_trips_through_answers() {
    let stored = [
        (1u64, 10u64, (5i64, 9i64)),
        (2, 10, (-3, 4)),
        (3, 11, (i64::MIN, i64::MAX)),
    ];
    let fix = payroll(&stored);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(PAYROLL),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let types: Vec<ValueType> = prepared
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
        .collect();
    assert_eq!(
        types,
        vec![
            ValueType::U64,
            ValueType::Interval {
                element: IntervalElement::I64
            },
        ],
        "the signature reports the interval type"
    );
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, i64, i64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(emp), AnswerValue::IntervalI64(iv)) => (emp, iv.start(), iv.end()),
            other => panic!("(u64, interval) answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();
    let mut expected: Vec<(u64, i64, i64)> = stored
        .iter()
        .map(|(_, emp, (start, end))| (*emp, *start, *end))
        .collect();
    expected.sort_unstable();
    assert_eq!(answers, expected, "stored bounds round-trip exactly");
}

#[test]
fn a_closed_group_key_takes_the_dense_table() {
    let fix = super::folded::readings(super::folded::READINGS);

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
            FindTerm::Count,
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let EitherSink::Aggregate(sink) = &prepared.sink else {
        panic!("folds build the aggregate sink");
    };
    assert!(
        sink.dense_group_table(),
        "the closed domain (4 rows) proves the dense table"
    );
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, i64, u64)> = (0..out.len())
        .map(|answer| {
            let (AnswerValue::U64(kind), AnswerValue::I64(sum), AnswerValue::U64(count)) =
                (out.get(answer, 0), out.get(answer, 1), out.get(answer, 2))
            else {
                panic!("(u64, i64, u64) rows");
            };
            (kind, sum, count)
        })
        .collect();
    answers.sort_unstable();
    assert_eq!(
        answers,
        vec![(0, 100, 1), (1, 421, 2), (2, 220, 1), (3, 300, 1)],
        "per-kind folds over the dense ordinals"
    );

    let open = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let prepared = fix.prepare(&open).expect("prepare");
    let EitherSink::Aggregate(sink) = &prepared.sink else {
        panic!("folds build the aggregate sink");
    };
    assert!(!sink.dense_group_table(), "open domains keep the map");
}

#[test]
fn fold_split_then_gj_split_composes_on_a_grouped_cyclic_body() {
    let edge_descriptor = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Edge".into(),
            fields: vec![
                FieldDescriptor {
                    name: "src".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "dst".into(),
                    value_type: ValueType::U64,
                },
            ],
        }],
        statements: vec![],
    };
    let edges: Vec<Vec<Value>> = [(1u64, 2u64), (2, 3), (1, 3), (3, 1), (2, 1), (4, 1)]
        .into_iter()
        .map(|(src, dst)| vec![Value::U64(src), Value::U64(dst)])
        .collect();
    let fix = Fix::heap(edge_descriptor, &[(RelationId(0), edges)]);

    let edge = |a: u16, b: u16| Atom {
        source: crate::ir::AtomSource::Edb(RelationId(0)),
        bindings: vec![
            (FieldId(0), Term::Var(VarId(a))),
            (FieldId(1), Term::Var(VarId(b))),
        ],
    };
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(2)), FindTerm::Count],
        atoms: vec![edge(0, 1), edge(1, 2), edge(0, 2)],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let mut answers: Vec<(u64, u64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::U64(z), AnswerValue::U64(n)) => (z, n),
            other => panic!("all-U64 answer: {other:?}"),
        })
        .collect();
    answers.sort_unstable();

    assert_eq!(answers, vec![(1, 1), (3, 2)]);
}
