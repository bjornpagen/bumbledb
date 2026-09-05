use super::*;

use crate::ir::ParamId;
use bumbledb_theory::schema::{IntervalElement, StatementDescriptor};

#[test]
fn key_probe_fast_lane_hits_misses_and_type_errors() {
    let fix = postings(&[(1, 7, "memo-a", 41), (2, 8, "memo-b", 42)]);

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepares");
    let PreparedPipeline::PointProbe { rule, finds } = &prepared.pipeline else {
        panic!("plain-variable key_probe takes the fast lane");
    };
    assert_eq!(finds.len(), 3, "one find column per projected variable");
    assert_eq!(
        rule.plan.vars.len(),
        3,
        "PointProbe stores KeyProbeRule, not a tagged PreparedRule"
    );
    let mut out = Answers::new();

    fix.execute_into(&mut prepared, &[BindValue::U64(2)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(8));
    assert_eq!(out.get(0, 1), AnswerValue::String("memo-b"));
    assert_eq!(out.get(0, 2), AnswerValue::I64(42));

    fix.execute_into(&mut prepared, &[BindValue::U64(999)], &mut out)
        .expect("miss is empty, not an error");
    assert_eq!(out.len(), 0);
    // Param-type error: typed, before any probe.
    let err = fix
        .execute_into(&mut prepared, &[BindValue::Bool(true)], &mut out)
        .expect_err("type mismatch");
    assert!(matches!(err, Error::ParamTypeMismatch { .. }), "{err:?}");
}

#[cfg(feature = "trace")]
#[test]
fn a_key_probe_prepare_and_execute_build_no_image() {
    use crate::obs;

    let fix = posting_store("prepared-keyprobe-noimage", &[(1, 7, "memo-a", 41)]);
    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Param(crate::ir::ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepares");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "the fast lane classified"
    );
    let mut out = Answers::new();
    obs::start_capture();
    fix.execute_into(&mut prepared, &[BindValue::U64(1)], &mut out)
        .expect("hit");
    let events = obs::finish_capture();
    assert_eq!(out.len(), 1);
    assert!(
        !events.iter().any(|e| e.point() == obs::names::IMAGE_BUILD),
        "a key-probe execution must not build images"
    );
}

#[test]
fn key_probe_queries_flow_through_the_same_surface() {
    let fix = postings(&[(5, 7, "found", 42)]);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(5))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "plain-variable key_probe takes the fast lane"
    );
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::I64(42));
}

fn booking_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Booking".into(),
            fields: vec![
                FieldDescriptor {
                    name: "room".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                },
                FieldDescriptor {
                    name: "label".into(),
                    value_type: ValueType::U64,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0), FieldId(1)]),
        }],
    }
}

fn bookings(rows: &[(u64, (u64, u64), u64)]) -> Fix {
    let facts: Vec<Vec<Value>> = rows
        .iter()
        .map(|(room, (start, end), label)| {
            vec![
                Value::U64(*room),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(*start, *end).expect("nonempty interval"),
                ),
                Value::U64(*label),
            ]
        })
        .collect();
    Fix::heap(booking_descriptor(), &[(RelationId(0), facts)])
}

fn booking_query(span_term: Term) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(1))),
                (FieldId(1), span_term),
                (FieldId(2), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn pointwise_key_point_lookup_uses_key_probe() {
    let fix = bookings(&[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)]);
    let query = booking_query(Term::Literal(Value::IntervalU64(
        bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
    )));
    let mut prepared = fix.prepare(&query).expect("prepare");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "pointwise key lookup takes the fast lane"
    );

    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(100));

    let near = booking_query(Term::Literal(Value::IntervalU64(
        bumbledb_theory::Interval::<u64>::new(5, 11).expect("nonempty interval"),
    )));
    let mut near = fix.prepare(&near).expect("prepare");
    let answers = fix
        .execute(&mut near, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(answers.len(), 0);
}

#[test]
fn a_membership_bound_single_atom_query_stays_free_join() {
    let fix = bookings(&[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)]);

    let query = booking_query(Term::Literal(Value::U64(7)));
    let mut prepared = fix.prepare(&query).expect("prepare");
    assert!(
        matches!(prepared.pipeline.main_rules(), [PreparedRule::FreeJoin(_)]),
        "membership binding is not a key cover"
    );

    let answers = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(answers.len(), 1);
    assert_eq!(answers.get(0, 0), AnswerValue::U64(100));

    let query = booking_query(Term::Literal(Value::U64(25)));
    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(200));
    let query = booking_query(Term::Literal(Value::U64(15)));
    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 0);
}

fn stay_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Stay".into(),
            fields: vec![
                FieldDescriptor {
                    name: "owner".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                },
            ],
        }],
        statements: vec![],
    }
}

fn one_stay() -> Fix {
    Fix::heap(
        stay_descriptor(),
        &[(
            RelationId(0),
            vec![vec![
                Value::U64(2),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
                ),
            ]],
        )],
    )
}

fn count_stay(span: (u64, u64)) -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(2))),
                (
                    FieldId(1),
                    Term::Literal(Value::IntervalU64(
                        bumbledb_theory::Interval::<u64>::new(span.0, span.1)
                            .expect("nonempty interval"),
                    )),
                ),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn full_fact_membership_lookup_with_an_interval_field_is_exact() {
    let fix = one_stay();
    let mut prepared = fix.prepare(&count_stay((5, 10))).expect("prepare");
    assert!(matches!(
        prepared.pipeline.main_rules(),
        [PreparedRule::KeyProbe(_)]
    ));

    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(1));

    let mut absent = fix.prepare(&count_stay((5, 11))).expect("prepare");
    let out = fix
        .execute(&mut absent, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 0);
}

#[test]
fn full_fact_membership_agrees_between_store_and_heap() {
    // The store path answers membership through the exact fingerprint
    // bucket (HASH-02); the heap path binary-searches canonical bytes.
    let store = StoreFix::store("prepared-keyprobe-membership", stay_descriptor());
    store.insert_dyn(
        RelationId(0),
        &[vec![
            Value::U64(2),
            Value::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
            ),
        ]],
    );
    let heap = one_stay();
    for (span, expected) in [((5, 10), 1usize), ((5, 11), 0), ((6, 10), 0)] {
        let mut on_store = store.prepare(&count_stay(span)).expect("prepare");
        let mut on_heap = heap.prepare(&count_stay(span)).expect("prepare");
        let store_out = store
            .execute(&mut on_store, &[] as &[BindValue])
            .expect("store execute");
        let heap_out = heap
            .execute(&mut on_heap, &[] as &[BindValue])
            .expect("heap execute");
        assert_eq!(store_out.len(), expected, "span {span:?}");
        assert_eq!(heap_out.len(), expected, "span {span:?}");
    }
}

#[test]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "the bare `generation` method path defeats the read closure's HRTB inference"
)]
fn an_unstored_text_param_on_the_fast_path_is_empty_not_an_error() {
    let descriptor = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Doc".into(),
            fields: vec![
                FieldDescriptor {
                    name: "name".into(),
                    value_type: ValueType::String,
                },
                FieldDescriptor {
                    name: "val".into(),
                    value_type: ValueType::U64,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    };
    let docs = StoreFix::store("prepared-keyprobe-doc", descriptor);
    docs.insert_dyn(
        RelationId(0),
        &[vec![Value::String("alice".into()), Value::U64(7)]],
    );

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = docs.prepare(&query).expect("prepare");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "plain-variable key_probe takes the fast lane"
    );

    let generation_before = docs
        .db
        .read(|instance| instance.generation())
        .expect("generation");
    let out = docs
        .execute(&mut prepared, &[BindValue::Str("ghost")])
        .expect("an unstored text is empty, not an error");
    assert_eq!(out.len(), 0);
    let generation_after = docs
        .db
        .read(|instance| instance.generation())
        .expect("generation");
    assert_eq!(
        generation_before, generation_after,
        "the read path never writes (interner tokens are execution-scoped)"
    );

    let out = docs
        .execute(&mut prepared, &[BindValue::Str("alice")])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(7));
}
