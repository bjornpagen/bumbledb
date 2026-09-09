use super::*;

fn elided(prepared: &PreparedQuery<T>) -> bool {
    matches!(&prepared.sink, EitherSink::Projection(sink) if sink.output_hashing_is_elided())
}

fn id_rule() -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: AtomSource::Edb(POSTING),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![],
    }
}

fn instrument_filter_schema() -> SchemaDescriptor {
    let relation = |name: &str, fields: &[(&str, ValueType)]| RelationDescriptor {
        name: name.into(),
        fields: fields
            .iter()
            .map(|(name, value_type)| FieldDescriptor {
                name: (*name).into(),
                value_type: *value_type,
            })
            .collect(),
        extension: None,
    };
    SchemaDescriptor {
        relations: vec![
            relation(
                "Posting",
                &[
                    ("id", ValueType::U64),
                    ("amount", ValueType::I64),
                    ("instrument", ValueType::U64),
                ],
            ),
            relation(
                "Instrument",
                &[("id", ValueType::U64), ("symbol", ValueType::String)],
            ),
        ],
        statements: [RelationId(0), RelationId(1)]
            .into_iter()
            .map(
                |relation| crate::schema::StatementDescriptor::Functionality {
                    relation,
                    projection: Box::new([FieldId(0)]),
                },
            )
            .collect(),
    }
}

fn instrument_filter_rule() -> Rule {
    Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            },
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(2))),
                    (FieldId(1), Term::Param(crate::ir::ParamId(0))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    }
}

#[test]
fn hidden_instrument_key_string_filter_matches_forced_hash_in_order() {
    // Registered benchmark `string`: output Posting(id, amount), joining
    // through a hidden instrument ID to a parameterized symbol filter.
    let fix = StoreFix::store("projection-hidden-instrument", instrument_filter_schema());
    let postings = [(0, 7, 10), (1, 7, 11), (2, 8, 10), (3, 9, 12), (4, 9, 99)];
    fix.insert_dyn(
        RelationId(0),
        &postings
            .iter()
            .map(|&(id, amount, instrument)| {
                vec![Value::U64(id), Value::I64(amount), Value::U64(instrument)]
            })
            .collect::<Vec<_>>(),
    );
    // Symbols are not unique. The absent FK target is permitted: no
    // containment law is necessary for the at-most-one-fact argument.
    let instruments = [(10, "common"), (11, "common"), (12, "other")];
    fix.insert_dyn(
        RelationId(1),
        &instruments
            .iter()
            .map(|&(id, symbol)| vec![Value::U64(id), Value::String(symbol.into())])
            .collect::<Vec<_>>(),
    );
    let base = instrument_filter_rule();
    let pairs = |answers: &Answers| -> Vec<(u64, i64)> {
        (0..answers.len())
            .map(|row| match (answers.get(row, 0), answers.get(row, 1)) {
                (AnswerValue::U64(id), AnswerValue::I64(amount)) => (id, amount),
                other => panic!("expected posting ID/amount, got {other:?}"),
            })
            .collect()
    };
    for (reverse, fallback) in [(false, false), (true, false), (true, true)] {
        let mut rule = base.clone();
        if reverse {
            rule.atoms.reverse();
        }
        let query = Query::single(rule);
        let mut append = fix.prepare(&query).unwrap();
        assert!(
            elided(&append),
            "the hidden FK key closes from the output posting ID"
        );
        let mut hashed = fix.prepare(&query).unwrap();
        let rule = &hashed.pipeline.main_rules()[0];
        hashed.sink =
            EitherSink::Projection(ProjectionSink::from_finds(rule.finds(), rule.slot_count()));
        for prepared in [&mut append, &mut hashed] {
            prepared.force_cursor_fallback(fallback);
        }
        for symbol in ["common", "missing", "other", "common"] {
            let params = [BindValue::Str(symbol)];
            let actual = pairs(&fix.execute(&mut append, &params).unwrap());
            let control = pairs(&fix.execute(&mut hashed, &params).unwrap());
            assert_eq!(actual, control, "first-emission order, including reuse");
            let expected: Vec<_> = postings
                .iter()
                .filter(|(_, _, instrument)| {
                    instruments
                        .iter()
                        .any(|(id, name)| id == instrument && *name == symbol)
                })
                .map(|&(id, amount, _)| (id, amount))
                .collect();
            let mut sorted = actual;
            sorted.sort_unstable();
            assert_eq!(
                sorted, expected,
                "independent join and symbol-filter denotation: reverse={reverse}, fallback={fallback}, symbol={symbol}"
            );
        }
    }
}

#[test]
fn uuid_keyed_join_with_negative_guard_preserves_order_across_sink_tiers() {
    let schema = SchemaDescriptor {
        relations: ["Left", "Right", "Blocked"]
            .into_iter()
            .map(|name| RelationDescriptor {
                name: name.into(),
                fields: vec![FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::Uuid,
                }],
                extension: None,
            })
            .collect(),
        statements: [RelationId(0), RelationId(1)]
            .into_iter()
            .map(
                |relation| crate::schema::StatementDescriptor::Functionality {
                    relation,
                    projection: Box::new([FieldId(0)]),
                },
            )
            .collect(),
    };
    let fix = StoreFix::store("unique-uuid-join", schema);
    // Two IDs share the high word; two share the low word. Neither half
    // alone is a valid key, and both words must survive append and spill.
    let ids = [
        crate::Uuid::from_bytes([0; 16]),
        crate::Uuid::from_bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
        crate::Uuid::from_bytes([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        crate::Uuid::from_bytes([2; 16]),
    ];
    let facts: Vec<_> = ids.iter().map(|id| vec![Value::Uuid(*id)]).collect();
    fix.insert_dyn(RelationId(0), &facts);
    fix.insert_dyn(RelationId(1), &facts);
    fix.insert_dyn(RelationId(2), &[facts[3].clone()]);
    let atom = |relation| Atom {
        source: AtomSource::Edb(relation),
        bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
    };
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![atom(RelationId(0)), atom(RelationId(1))],
        negated: vec![atom(RelationId(2))],
        conditions: vec![],
    });
    for fallback in [false, true] {
        let mut append = fix.prepare(&query).unwrap();
        assert!(elided(&append));
        let mut hashed = fix.prepare(&query).unwrap();
        let rule = &hashed.pipeline.main_rules()[0];
        hashed.sink =
            EitherSink::Projection(ProjectionSink::from_finds(rule.finds(), rule.slot_count()));
        for prepared in [&mut append, &mut hashed] {
            prepared.force_cursor_fallback(fallback);
        }
        // Reuse both sinks, including after delivery.
        for _ in 0..2 {
            let actual = fix.execute(&mut append, &[] as &[BindValue]).unwrap();
            let control = fix.execute(&mut hashed, &[] as &[BindValue]).unwrap();
            assert_eq!(actual.len(), 3);
            assert_eq!(actual.len(), control.len());
            for row in 0..actual.len() {
                assert_eq!(actual.get(row, 0), control.get(row, 0));
            }
            for id in &ids[..3] {
                assert!((0..actual.len()).any(|row| actual.get(row, 0) == AnswerValue::Uuid(*id)));
            }
        }
    }
}

#[test]
fn computed_heads_and_derived_occurrences_do_not_install_projection_witnesses() {
    let fix = posting_store(
        "unique-projection-exclusions",
        &[(1, 0, "", 0), (2, 0, "", 0)],
    );
    let mut computed_rule = id_rule();
    computed_rule.finds = vec![FindTerm::Compute(crate::ScalarExpr::Literal(Value::U64(0)))];
    let mut computed = fix.prepare(&Query::single(computed_rule)).unwrap();
    assert!(!elided(&computed));
    let answers = fix.execute(&mut computed, &[] as &[BindValue]).unwrap();
    assert_eq!(
        answers.len(),
        1,
        "two bindings collapse to one computed row"
    );
    assert_eq!(answers.get(0, 0), AnswerValue::U64(0));
    for _ in 0..3 {
        computed.release_memory();
        let after = fix.execute(&mut computed, &[] as &[BindValue]).unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after.get(0, 0), AnswerValue::U64(0));
        assert!(!elided(&computed));
    }

    let mut outer = id_rule();
    outer.atoms[0].source = AtomSource::Interior(InteriorId(0));
    let query = Query {
        interiors: vec![Interior {
            rules: vec![id_rule()],
        }],
        head: vec![HeadTerm::Var],
        rules: vec![outer],
        rec: None,
    };
    let mut derived = fix.prepare(&query).unwrap();
    assert!(
        !elided(&derived),
        "EDB key proofs do not certify derived occurrences"
    );
    let answers = fix.execute(&mut derived, &[] as &[BindValue]).unwrap();
    assert_eq!(answers.len(), 2);
    for id in [1, 2] {
        assert!((0..answers.len()).any(|row| answers.get(row, 0) == AnswerValue::U64(id)));
    }
}

#[test]
fn keyed_interval_membership_keeps_hashing_for_hidden_points_through_real_normalization() {
    let schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Span".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "during".into(),
                        value_type: ValueType::Interval {
                            element: bumbledb_theory::schema::IntervalElement::I64,
                        },
                    },
                ],
                extension: None,
            },
            RelationDescriptor {
                name: "Point".into(),
                fields: vec![FieldDescriptor {
                    name: "at".into(),
                    value_type: ValueType::I64,
                }],
                extension: None,
            },
        ],
        statements: vec![crate::schema::StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    };
    let fix = StoreFix::store("projection-hidden-membership-points", schema);
    fix.insert_dyn(
        RelationId(0),
        &[vec![
            Value::U64(7),
            Value::IntervalI64(bumbledb_theory::Interval::<i64>::new(0, 10).unwrap()),
        ]],
    );
    fix.insert_dyn(RelationId(1), &[vec![Value::I64(1)], vec![Value::I64(2)]]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(RelationId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: AtomSource::Edb(RelationId(1)),
                bindings: vec![(FieldId(0), Term::Var(VarId(1)))],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let validated = crate::ir::validate::validate(fix.db.schema(), &query).unwrap();
    let normalized = crate::ir::normalize::normalize_rules(fix.db.schema(), &[], validated.rules());
    assert_eq!(
        normalized[0].occurrences[0].vars,
        vec![(FieldId(0), VarId(0))]
    );
    assert_eq!(
        normalized[0].occurrences[0].point_vars,
        vec![(FieldId(1), VarId(1), false)]
    );
    for fallback in [false, true] {
        let mut prepared = fix.prepare(&query).unwrap();
        assert!(
            !elided(&prepared),
            "a determined interval does not determine its membership point"
        );
        prepared.force_cursor_fallback(fallback);
        let answers = fix.execute(&mut prepared, &[] as &[BindValue]).unwrap();
        assert_eq!(
            answers.len(),
            1,
            "both matching points project the same span ID"
        );
        assert_eq!(answers.get(0, 0), AnswerValue::U64(7));
    }
}

#[test]
fn unique_projection_refusals_publish_no_partial_rows_and_allow_reuse() {
    let rows: Vec<_> = (0..513).map(|id| (id, 0, "", 0)).collect();
    let fix = posting_store("unique-projection-refusals", &rows);
    let mut prepared = fix.prepare(&Query::single(id_rule())).unwrap();
    assert!(elided(&prepared));
    let mut out = fix.execute(&mut prepared, &[] as &[BindValue]).unwrap();
    assert_eq!(out.len(), rows.len());
    for _ in 0..2 {
        fix.db
            .read(crate::api::db::test_operation(), |instance| {
                let work = crate::work::WorkContext::new();
                work.cancel();
                let source =
                    crate::api::prepared::source::QuerySource::store(instance.snapshot(), &work);
                let result = prepared.execute_source(&source, &[] as &[BindValue], &mut out);
                let expected = matches!(&result, Err(Error::Store(store)) if matches!(
                    &**store,
                    crate::storage::store::StoreError::Work(crate::work::WorkError::Cancelled)
                ));
                assert!(expected, "unexpected execution result: {result:?}");
                assert!(
                    out.is_empty(),
                    "neither stale nor partial answers may escape refusal"
                );
                Ok(())
            })
            .unwrap();
        fix.execute_into(&mut prepared, &[] as &[BindValue], &mut out)
            .unwrap();
        assert_eq!(out.len(), rows.len(), "the sink resets after refusal");
        let mut actual: Vec<_> = (0..out.len())
            .map(|row| match out.get(row, 0) {
                AnswerValue::U64(id) => id,
                other => panic!("expected id, got {other:?}"),
            })
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, (0..513).collect::<Vec<_>>());
    }
}
