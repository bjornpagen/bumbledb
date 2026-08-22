use super::*;

use crate::ir::ParamId;
use crate::storage::dict;
use bumbledb_theory::schema::{IntervalElement, StatementDescriptor};

#[test]
fn key_probe_fast_lane_hits_misses_and_type_errors() {
    let dir = TempDir::new("prepared-key_probe-lane");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41), (2, 8, "memo-b", 42)]);

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
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new(&schema);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
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

    prepared
        .execute(&txn, &cache, &[BindValue::U64(2)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(8));
    assert_eq!(out.get(0, 1), AnswerValue::String("memo-b"));
    assert_eq!(out.get(0, 2), AnswerValue::I64(42));

    prepared
        .execute(&txn, &cache, &[BindValue::U64(999)], &mut out)
        .expect("miss is empty, not an error");
    assert_eq!(out.len(), 0);
    // Param-type error: typed, before any probe.
    let err = prepared
        .execute(&txn, &cache, &[BindValue::Bool(true)], &mut out)
        .expect_err("type mismatch");
    assert!(matches!(err, Error::ParamTypeMismatch { .. }), "{err:?}");
}

#[test]
fn a_key_probe_prepare_and_execute_build_no_image() {
    let dir = TempDir::new("prepared-key_probe-statsfree");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(1, 7, "memo-a", 41)]);
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
    let txn = env.read_txn().expect("txn");
    let cache = crate::image::cache::ImageCache::new(&schema);
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepares");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "the fast lane classified"
    );
    let mut out = Answers::new();
    prepared
        .execute(&txn, &cache, &[BindValue::U64(1)], &mut out)
        .expect("hit");
    assert_eq!(out.len(), 1);
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "a key-probe execution must not build images (and so never walks stats)"
    );
}

#[test]
fn key_probe_queries_flow_through_the_same_surface() {
    let dir = TempDir::new("prepared-key_probe");
    let schema = schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_postings(&env, &schema, &[(5, 7, "found", 42)]);
    let cache = ImageCache::new(&schema);

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
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "plain-variable key_probe takes the fast lane"
    );
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::I64(42));

    let (answers, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert_eq!(answers.len(), 1);
    assert!(report.contains("query:"), "{report}");
}

fn booking_schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Booking".into(),
            fields: vec![
                FieldDescriptor {
                    name: "room".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "label".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0), FieldId(1)]),
        }],
    }
    .validate()
    .expect("valid fixture")
}

fn insert_bookings(env: &Environment, schema: &Schema, rows: &[(u64, (u64, u64), u64)]) {
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(schema);
    for (room, (start, end), label) in rows {
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(*room),
                ValueRef::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(*start, *end).expect("nonempty interval"),
                ),
                ValueRef::U64(*label),
            ],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
    }
    drop(view);
    commit(delta, env).expect("commit").expect("admitted");
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
fn pointwise_key_point_lookup_uses_key_probe_and_is_image_free() {
    let dir = TempDir::new("prepared-key_probe-pointwise");
    let schema = booking_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_bookings(
        &env,
        &schema,
        &[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = booking_query(Term::Literal(Value::IntervalU64(
        bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
    )));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "pointwise key lookup takes the fast lane"
    );

    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(100));
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "post-commit cold: the key-probe path builds no image"
    );

    let near = booking_query(Term::Literal(Value::IntervalU64(
        bumbledb_theory::Interval::<u64>::new(5, 11).expect("nonempty interval"),
    )));
    let mut near = prepare(&txn, &cache, &schema, &near).expect("prepare");
    let answers = near
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(answers.len(), 0);
    #[cfg(feature = "trace")]
    assert_eq!(cache.resident(), (0, 0));
}

#[test]
fn a_membership_bound_single_atom_query_stays_free_join() {
    let dir = TempDir::new("prepared-key_probe-membership");
    let schema = booking_schema();
    let env = Environment::create(dir.path(), &schema).expect("create");
    insert_bookings(
        &env,
        &schema,
        &[(1, (5, 10), 100), (1, (20, 30), 200), (2, (5, 10), 300)],
    );
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let query = booking_query(Term::Literal(Value::U64(7)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.pipeline.main_rules(), [PreparedRule::FreeJoin(_)]),
        "membership binding is not a key cover"
    );

    let answers = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(answers.len(), 1);
    assert_eq!(answers.get(0, 0), AnswerValue::U64(100));

    let query = booking_query(Term::Literal(Value::U64(25)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(200));
    let query = booking_query(Term::Literal(Value::U64(15)));
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 0);
}

#[test]
fn full_fact_membership_lookup_with_an_interval_field_is_image_free() {
    let dir = TempDir::new("prepared-key_probe-m-interval");

    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Stay".into(),
            fields: vec![
                FieldDescriptor {
                    name: "owner".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let mut bytes = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(2),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
            ),
        ],
        schema.relation(RelationId(0)).layout(),
        &mut bytes,
    );
    delta.insert(&view, RelationId(0), &bytes).expect("insert");
    drop(view);
    commit(delta, &env).expect("commit").expect("admitted");

    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

    let count_stay = |span: (u64, u64)| {
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
    };
    let mut prepared = prepare(&txn, &cache, &schema, &count_stay((5, 10))).expect("prepare");
    assert!(matches!(
        prepared.pipeline.main_rules(),
        [PreparedRule::KeyProbe(_)]
    ));
    let (_, report) = prepared.introspect(&txn, &cache, &[]).expect("introspect");
    assert!(report.contains("query:"), "{report}");

    let out = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(1));
    #[cfg(feature = "trace")]
    assert_eq!(
        cache.resident(),
        (0, 0),
        "post-commit cold: the M path builds no image"
    );

    let mut absent = prepare(&txn, &cache, &schema, &count_stay((5, 11))).expect("prepare");
    let out = absent
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 0);
}

#[test]
fn execute_and_profile_agree_on_an_aggregate_key_probe() {
    let dir = TempDir::new("prepared-key_probe-agg-parity");
    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Stay".into(),
            fields: vec![
                FieldDescriptor {
                    name: "owner".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let mut bytes = Vec::new();
    encode_fact(
        &[
            ValueRef::U64(2),
            ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
            ),
        ],
        schema.relation(RelationId(0)).layout(),
        &mut bytes,
    );
    delta.insert(&view, RelationId(0), &bytes).expect("insert");
    drop(view);
    commit(delta, &env).expect("commit").expect("admitted");
    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");
    let query = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(2))),
                (
                    FieldId(1),
                    Term::Literal(Value::IntervalU64(
                        bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
                    )),
                ),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(
            &prepared.pipeline,
            PreparedPipeline::Cq { rules, .. }
                if matches!(rules.as_slice(), [PreparedRule::KeyProbe(_)])
        ),
        "aggregate key probe keeps the sink"
    );
    let executed = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(executed.len(), 1);
}

#[test]
fn intern_miss_param_on_the_fast_path_is_empty_not_an_error() {
    let dir = TempDir::new("prepared-key_probe-intern-miss");

    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Doc".into(),
            fields: vec![
                FieldDescriptor {
                    name: "name".into(),
                    value_type: ValueType::String,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "val".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid fixture");
    let env = Environment::create(dir.path(), &schema).expect("create");
    let view = env.read_txn().expect("txn");
    let mut delta = WriteDelta::new(&schema);
    let name_id = delta.intern_str(&view, "alice").expect("intern");
    let mut bytes = Vec::new();
    encode_fact(
        &[ValueRef::String(name_id), ValueRef::U64(7)],
        schema.relation(RelationId(0)).layout(),
        &mut bytes,
    );
    delta.insert(&view, RelationId(0), &bytes).expect("insert");
    drop(view);
    commit(delta, &env).expect("commit").expect("admitted");

    let cache = ImageCache::new(&schema);
    let txn = env.read_txn().expect("txn");

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
    let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
    assert!(
        matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "plain-variable key_probe takes the fast lane"
    );

    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::Str("ghost")])
        .expect("an intern miss is empty, not an error");
    assert_eq!(out.len(), 0);
    assert_eq!(
        dict::lookup_str(&txn, "ghost").expect("lookup"),
        None,
        "the read path never interns"
    );

    let out = prepared
        .execute_collect(&txn, &cache, &[BindValue::Str("alice")])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(7));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one fixture, two corrupt shapes: the schema, the healthy hit, and both convictions read as one story"
)]
fn a_corrupt_fixed_width_start_through_the_key_probe_is_corruption_not_a_panic() {
    use crate::error::CorruptionError;
    use crate::storage::keys;
    use crate::storage::read;

    let schema = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Slot".into(),
            fields: vec![
                FieldDescriptor {
                    name: "room".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "span".into(),
                    value_type: ValueType::FixedInterval {
                        element: IntervalElement::U64,
                        width: 5,
                    },
                    generation: Generation::None,
                },
                FieldDescriptor {
                    name: "label".into(),
                    value_type: ValueType::U64,
                    generation: Generation::None,
                },
            ],
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    }
    .validate()
    .expect("valid fixture");
    let dir = TempDir::new("prepared-key_probe-fixed-corrupt");
    let env = Environment::create(dir.path(), &schema).expect("create");
    {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let mut bytes = Vec::new();
        encode_fact(
            &[
                ValueRef::U64(1),
                ValueRef::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(5, 10).expect("nonempty interval"),
                ),
                ValueRef::U64(100),
            ],
            schema.relation(RelationId(0)).layout(),
            &mut bytes,
        );
        delta.insert(&view, RelationId(0), &bytes).expect("insert");
        drop(view);
        commit(delta, &env).expect("commit").expect("admitted");
    }

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Literal(Value::U64(1))),
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let cache = ImageCache::new(&schema);
    {
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        assert!(
            matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
            "plain-variable key_probe takes the fast lane"
        );
        let out = prepared
            .execute_collect(&txn, &cache, &[] as &[BindValue])
            .expect("hit");
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(0, 1), AnswerValue::U64(100));
    }

    let layout = schema.relation(RelationId(0)).layout();
    let offset = layout.field_offset(1);
    let victim = {
        let txn = env.read_txn().expect("txn");
        read::scan(&txn, &schema, RelationId(0))
            .expect("scan")
            .map(|e| e.expect("ok").0)
            .next()
            .expect("nonempty")
    };
    let healthy = {
        let txn = env.read_txn().expect("txn");
        read::fetch(&txn, &schema, RelationId(0), victim)
            .expect("fetch")
            .bytes()
            .to_vec()
    };

    for corrupt_start in [u64::MAX - 5, u64::MAX] {
        let mut corrupt = healthy.clone();
        corrupt[offset..offset + 8].copy_from_slice(&corrupt_start.to_be_bytes());
        {
            let mut wtxn = env.write_txn().expect("txn");
            let key = keys::fact_key(RelationId(0), victim);
            env.data().put(wtxn.raw_mut(), &key, &corrupt).expect("put");
            wtxn.commit().expect("commit");
        }
        let txn = env.read_txn().expect("txn");
        let mut prepared = prepare(&txn, &cache, &schema, &query).expect("prepare");
        let err = prepared
            .execute_collect(&txn, &cache, &[] as &[BindValue])
            .expect_err("corrupt stored start convicts");
        assert!(
            matches!(
                err,
                Error::Corruption(CorruptionError::InvalidFixedIntervalStart(bytes))
                    if bytes == corrupt_start.to_be_bytes()
            ),
            "{err:?}"
        );
    }

    let scan = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Var(VarId(1)),
            FindTerm::Var(VarId(2)),
        ],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
                (FieldId(2), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let txn = env.read_txn().expect("txn");
    let mut prepared = prepare(&txn, &cache, &schema, &scan).expect("prepare");
    assert!(
        !matches!(prepared.pipeline, PreparedPipeline::PointProbe { .. }),
        "the all-vars scan must not take the key-probe lane"
    );
    let err = prepared
        .execute_collect(&txn, &cache, &[] as &[BindValue])
        .expect_err("the image build convicts the corrupt start");
    assert!(
        matches!(
            err,
            Error::Corruption(CorruptionError::InvalidFixedIntervalStart(_))
        ),
        "{err:?}"
    );
}
