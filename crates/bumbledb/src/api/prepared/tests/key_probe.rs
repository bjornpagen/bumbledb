use super::*;

use crate::exec::dispatch::{KeyProbeKind, key_probe_row};
use crate::image::canon::RowWords;
use crate::image::intern::InternerHandle;
use crate::ir::ParamId;
use crate::work::{GenerationHandle, GenerationState, WorkError};
use bumbledb_theory::schema::{IntervalElement, StatementDescriptor};

fn text_membership_query() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(1))),
                (FieldId(1), Term::Literal(Value::U64(7))),
                (FieldId(2), Term::Param(ParamId(0))),
                (FieldId(3), Term::Literal(Value::I64(41))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn cancelled_text_resolution_is_not_a_membership_or_indexed_key_miss() {
    for indexed in [false, true] {
        let mut schema = descriptor();
        schema.statements.clear();
        if indexed {
            schema.statements.push(StatementDescriptor::Functionality {
                relation: POSTING,
                projection: Box::new([FieldId(2)]),
            });
        }
        let fix = StoreFix::store("prepared-scratch-key-refusal", schema);
        fix.insert_dyn(POSTING, &posting_rows(&[(1, 7, "alpha", 41)]));
        let query = text_membership_query();
        let prepared = fix.prepare(&query).expect("prepare real probe");
        let [PreparedRule::KeyProbe(rule)] = prepared.pipeline.main_rules() else {
            panic!("fully bound query must use key dispatch");
        };
        assert_eq!(
            matches!(rule.plan.kind, KeyProbeKind::Uniqueness { .. }),
            indexed
        );
        let pin = fix.db.owned_read().unwrap();
        {
            let work = crate::api::db::test_operation();
            let frame = pin.frame(&work);
            if let KeyProbeKind::Uniqueness { projection, .. } = &rule.plan.kind {
                assert!(matches!(
                    frame
                        .snapshot()
                        .projection(*projection)
                        .unwrap()
                        .compiled()
                        .encoding,
                    crate::schema::KeyEncoding::FingerprintBucket
                ));
            }
            let source = super::super::source::QuerySource::store(frame.snapshot(), &work);
            let generation = GenerationHandle::new(GenerationState::new(
                crate::image::CacheGeneration::initial(),
            ));
            let interner = InternerHandle::new(&generation, &work);
            let params = [Const::Text(interner.intern("alpha").unwrap())];
            let mut row = RowWords::new(&[
                ValueType::U64,
                ValueType::U64,
                ValueType::String,
                ValueType::I64,
            ]);
            let mut key = crate::image::view::ResolvedWords::default();
            let mut probe = || {
                key_probe_row(
                    &rule.plan,
                    &source,
                    fix.db.schema(),
                    &interner,
                    &params,
                    &mut row,
                    &mut key,
                )
            };
            assert!(probe().expect("live membership/key hit"));
            assert!(probe().expect("warm membership/key hit"));
            let visited = source.visit_count();
            work.cancel();
            let expected = WorkError::Cancelled;
            let error = probe().expect_err("cancelled probe must not become a nonmatch");
            assert!(matches!(error, Error::Store(store) if matches!(
                *store, crate::storage::store::StoreError::Work(ref error) if *error == expected
            )));
            assert_eq!(
                source.visit_count(),
                visited,
                "refusal precedes any source row visit"
            );
        }
    }
}

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
    assert!(
        !prepared.no_text_probe,
        "a row containing text needs its resolver"
    );
    let PreparedPipeline::PointProbe { rule, finds } = &prepared.pipeline else {
        panic!("plain-variable key_probe takes the fast lane");
    };
    assert_eq!(finds.len(), 3, "one find column per projected variable");
    assert_eq!(
        rule.plan.vars.len(),
        3,
        "PointProbe stores KeyProbeRule, not a tagged PreparedRule"
    );
    // The PointProbe shape delivers directly, without a distinct-result sink.
    let mut out = Answers::new();

    fix.execute_into(&mut prepared, &[BindValue::U64(2)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(8));
    assert_eq!(out.get(0, 1), AnswerValue::String("memo-b"));
    assert_eq!(out.get(0, 2), AnswerValue::I64(42));
    assert!(prepared.text_generation.is_some());
    let PreparedPipeline::PointProbe { rule, .. } = &prepared.pipeline else {
        unreachable!("point probe");
    };
    let row_address = rule.row.span_words(FieldId(0)).as_ptr();

    fix.execute_into(&mut prepared, &[BindValue::U64(999)], &mut out)
        .expect("miss is empty, not an error");
    assert_eq!(out.len(), 0);
    fix.execute_into(&mut prepared, &[BindValue::U64(1)], &mut out)
        .expect("hit after miss overwrites the retained row");
    assert_eq!(out.get(0, 1), AnswerValue::String("memo-a"));
    let PreparedPipeline::PointProbe { rule, .. } = &prepared.pipeline else {
        unreachable!("point probe");
    };
    assert_eq!(
        row_address,
        rule.row.span_words(FieldId(0)).as_ptr(),
        "prepared point probes retain their row allocation"
    );
    // Param-type error: typed, before any probe.
    let err = fix
        .execute_into(&mut prepared, &[BindValue::Bool(true)], &mut out)
        .expect_err("type mismatch");
    assert!(matches!(err, Error::ParamTypeMismatch { .. }), "{err:?}");
}

#[test]
fn cancelled_missing_scalar_probe_refuses_and_clears_reused_answers() {
    let fix = posting_store("prepared-cancelled-point-miss", &[(1, 7, "memo", 41)]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let pin = fix.db.owned_read().unwrap();
    let active = crate::api::db::test_operation();
    let mut prepared = pin.prepare(&query, &active).unwrap();
    assert!(
        !prepared.no_text_probe,
        "unprojected memo text still gets decoded"
    );
    assert!(matches!(
        prepared.pipeline,
        PreparedPipeline::PointProbe { .. }
    ));
    let mut out = Answers::new();
    pin.frame(&active)
        .execute(&mut prepared, &[BindValue::U64(1)], &mut out)
        .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::I64(41));
    // Admit the snapshot before cancelling the operation, so a snapshot
    // gate poll cannot mask a missing poll in the empty determinant seek.
    let cancelled = crate::api::db::test_operation();
    cancelled.cancel();
    for _ in 0..2 {
        let context = cancelled.clone();
        let expected = WorkError::Cancelled;
        let error = pin
            .frame(&context)
            .execute(&mut prepared, &[BindValue::U64(999)], &mut out)
            .expect_err("a stopped point miss must not publish success");
        assert!(matches!(error, Error::Store(store) if matches!(
            *store, crate::storage::store::StoreError::Work(ref error) if *error == expected
        )));
        assert_eq!(
            out.len(),
            0,
            "the previous hit cannot survive a failed operation"
        );
        pin.frame(&active)
            .execute(&mut prepared, &[BindValue::U64(1)], &mut out)
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 0), AnswerValue::I64(41));
    }
    let fresh_work = crate::work::WorkContext::new();
    pin.frame(&fresh_work)
        .execute(&mut prepared, &[BindValue::U64(999)], &mut out)
        .unwrap();
    assert_eq!(out.len(), 0);
    assert_eq!(fresh_work.checkpoint(), Ok(()));
}

#[test]
fn a_key_probe_prepare_and_execute_build_no_image() {
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
    assert_eq!(prepared.cache.image_count(), 0);
    fix.execute_into(&mut prepared, &[BindValue::U64(1)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    assert_eq!(
        prepared.cache.image_count(),
        0,
        "point probes build no images"
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

#[test]
fn text_free_probe_uses_shared_execution_without_acquiring_a_resolver() {
    let mut descriptor = stay_descriptor();
    descriptor.relations[0].fields.push(FieldDescriptor {
        name: "amount".into(),
        value_type: ValueType::U64,
    });
    descriptor
        .statements
        .push(StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::from([FieldId(0)]),
        });
    let store = StoreFix::store("prepared-text-free-probe", descriptor.clone());
    let facts = vec![vec![
        Value::U64(2),
        Value::IntervalU64(bumbledb_theory::Interval::new(5, 10).unwrap()),
        Value::U64(7),
    ]];
    store.insert_dyn(RelationId(0), &facts);
    let heap = Fix::heap(descriptor, &[(RelationId(0), facts)]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Gt,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::U64(3)),
        })],
    });
    let mut prepared = store.prepare(&query).unwrap();
    let mut oracle = heap.prepare(&query).unwrap();
    assert!(prepared.no_text_probe && oracle.no_text_probe);
    let PreparedPipeline::PointProbe { rule, .. } = &prepared.pipeline else {
        unreachable!()
    };
    assert!(
        !rule.plan.remaining_filters.is_empty(),
        "shared scalar predicates are exercised"
    );
    // Even an unused declared String parameter disallows this capability.
    for extra in [
        ParamSpec::Scalar {
            ty: ValueType::String,
            point: false,
        },
        ParamSpec::Set {
            elem: ValueType::String,
            point: false,
        },
    ] {
        let mut params = prepared.params.clone();
        params.push(extra);
        assert!(!super::super::build::seal_no_text_probe(
            &prepared.pipeline,
            &prepared.schema,
            &params
        ));
    }
    for owner in [2, 3, 2] {
        let args = [BindValue::U64(owner)];
        let actual = store.execute(&mut prepared, &args).unwrap();
        let expected = heap.execute(&mut oracle, &args).unwrap();
        assert_eq!(actual.len(), usize::from(owner == 2));
        assert_eq!(actual.len(), expected.len());
        if owner == 2 {
            assert_eq!(actual.get(0, 0), expected.get(0, 0));
        }
        assert!(prepared.text_generation.is_none() && oracle.text_generation.is_none());
        prepared.release_memory();
    }
    let pin = store.db.owned_read().unwrap();
    let cancelled = crate::api::db::test_operation();
    cancelled.cancel();
    let mut out = Answers::new();
    for owner in [2, 3] {
        assert!(
            pin.frame(&cancelled)
                .execute(&mut prepared, &[BindValue::U64(owner)], &mut out)
                .is_err()
        );
        assert!(out.is_empty());
    }
    assert!(
        store
            .execute(&mut prepared, &[BindValue::Str("wrong")])
            .is_err()
    );
    assert!(prepared.text_generation.is_none());
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
fn pointwise_key_point_lookup_preserves_exact_interval_equality() {
    let fix = bookings(&[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)]);
    let query = booking_query(Term::Literal(Value::IntervalU64(
        bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
    )));
    let mut prepared = fix.prepare(&query).expect("prepare");
    assert!(
        matches!(prepared.pipeline.main_rules(), [PreparedRule::FreeJoin(_)]),
        "pointwise functionality does not prove full-row uniqueness"
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
        !prepared.no_text_probe,
        "String parameters require the ordinary pinned namespace"
    );
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "plain-variable key_probe takes the fast lane"
    );

    let generation_before = docs
        .db
        .read(crate::api::db::test_operation(), |instance| {
            Ok(instance.generation())
        })
        .expect("generation");
    let out = docs
        .execute(&mut prepared, &[BindValue::Str("ghost")])
        .expect("an unstored text is empty, not an error");
    assert_eq!(out.len(), 0);
    prepared.release_memory();
    let rebound = docs
        .execute(&mut prepared, &[BindValue::Str("alice")])
        .unwrap();
    assert_eq!(rebound.get(0, 0), AnswerValue::U64(7));
    assert!(prepared.text_generation.is_some());
    let generation_after = docs
        .db
        .read(crate::api::db::test_operation(), |instance| {
            Ok(instance.generation())
        })
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
