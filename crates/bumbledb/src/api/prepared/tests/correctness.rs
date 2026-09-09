use super::*;
use crate::ir::FoldOp;

fn id_amount_pairs(answers: &Answers) -> Vec<(u64, i64)> {
    (0..answers.len())
        .map(|row| match (answers.get(row, 0), answers.get(row, 1)) {
            (AnswerValue::U64(id), AnswerValue::I64(amount)) => (id, amount),
            other => panic!("expected id/amount, got {other:?}"),
        })
        .collect()
}

fn output_hashing_is_elided(prepared: &PreparedQuery<T>) -> bool {
    match &prepared.sink {
        EitherSink::Projection(sink) => sink.output_hashing_is_elided(),
        _ => false,
    }
}

fn fallback_lookup_schema(lookup_type: ValueType, keyed_lookup: bool) -> SchemaDescriptor {
    use bumbledb_theory::schema::StatementDescriptor;

    let mut schema = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "lookup".into(),
                        value_type: lookup_type,
                    },
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Gate".into(),
                fields: vec![FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                }],
            },
        ],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    };
    if keyed_lookup {
        schema.statements.push(StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(1)]),
        });
    }
    schema
}

fn fallback_lookup_query(selected_field: FieldId, gate_first: bool) -> Query {
    let variable_field = FieldId(1 - selected_field.0);
    let mut atoms = vec![
        Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (selected_field, Term::Param(crate::ir::ParamId(0))),
                (variable_field, Term::Var(VarId(0))),
            ],
        },
        Atom {
            source: AtomSource::Edb(RelationId(1)),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        },
    ];
    if gate_first {
        atoms.reverse();
    }
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms,
        negated: vec![],
        conditions: vec![],
    })
}

#[test]
fn keyed_fallback_uses_selection_fields_and_preserves_typed_keys() {
    let cases = [
        (
            ValueType::Bool,
            [Value::Bool(false), Value::Bool(true)],
            [BindValue::Bool(false), BindValue::Bool(true)],
        ),
        (
            ValueType::I64,
            [Value::I64(-7), Value::I64(9)],
            [BindValue::I64(-7), BindValue::I64(9)],
        ),
        (
            ValueType::F64,
            [
                Value::F64(crate::F64::from(-7.5)),
                Value::F64(crate::F64::from(9.25)),
            ],
            [
                BindValue::F64(crate::F64::from(-7.5)),
                BindValue::F64(crate::F64::from(9.25)),
            ],
        ),
        (
            ValueType::U64,
            [Value::U64(7), Value::U64(9)],
            [BindValue::U64(7), BindValue::U64(9)],
        ),
        (
            ValueType::String,
            [
                Value::String("first".into()),
                Value::String("second".into()),
            ],
            [BindValue::Str("first"), BindValue::Str("second")],
        ),
        (
            ValueType::FixedBytes { len: 4 },
            [
                Value::FixedBytes(Box::new([1, 2, 3, 4])),
                Value::FixedBytes(Box::new([5, 6, 7, 8])),
            ],
            [
                BindValue::FixedBytes(&[1, 2, 3, 4]),
                BindValue::FixedBytes(&[5, 6, 7, 8]),
            ],
        ),
    ];
    for (lookup_type, values, params) in cases {
        // The selection is field 1, while the join variable is field 0.
        // String tokens and inline bytes are not scalar U64 lookup keys.
        for keyed_lookup in [false, true] {
            let store = StoreFix::store(
                "fallback-typed-lookup",
                fallback_lookup_schema(lookup_type, keyed_lookup),
            );
            store.insert_dyn(
                RelationId(0),
                &[
                    vec![Value::U64(101), values[0].clone()],
                    vec![Value::U64(202), values[1].clone()],
                ],
            );
            store.insert_dyn(
                RelationId(1),
                &[vec![Value::U64(101)], vec![Value::U64(202)]],
            );
            let mut prepared = store
                .prepare(&fallback_lookup_query(FieldId(1), false))
                .unwrap();
            prepared.force_cursor_fallback(true);
            for (index, expected) in [(1, 202), (0, 101), (1, 202)] {
                let answers = store.execute(&mut prepared, &[params[index]]).unwrap();
                assert_eq!(answers.len(), 1, "{lookup_type:?}, keyed={keyed_lookup}");
                assert_eq!(answers.get(0, 0), AnswerValue::U64(expected));
            }
        }
    }
}

#[test]
fn heap_keyed_fallback_does_not_stop_after_a_rejected_first_row() {
    let heap = Fix::heap(
        fallback_lookup_schema(ValueType::U64, false),
        &[
            (
                RelationId(0),
                vec![
                    vec![Value::U64(1), Value::U64(10)],
                    vec![Value::U64(2), Value::U64(20)],
                ],
            ),
            (RelationId(1), vec![vec![Value::U64(20)]]),
        ],
    );
    // Gate binds the last atom's only variable. A successful callback does
    // not imply a matched row: the heap key visitor first sees rejected id 1.
    let mut prepared = heap
        .prepare(&fallback_lookup_query(FieldId(0), true))
        .unwrap();
    prepared.force_cursor_fallback(true);
    for (id, expected_len) in [(2, 1), (1, 0), (2, 1)] {
        let answers = heap.execute(&mut prepared, &[BindValue::U64(id)]).unwrap();
        assert_eq!(answers.len(), expected_len);
        if expected_len != 0 {
            assert_eq!(answers.get(0, 0), AnswerValue::U64(20));
        }
    }
}

fn keyed_range_query(project_id: bool) -> Query {
    Query::single(Rule {
        finds: if project_id {
            vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))]
        } else {
            vec![FindTerm::Var(VarId(1))]
        },
        atoms: vec![Atom {
            source: AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
                (FieldId(1), Term::Var(VarId(2))),
            ],
        }],
        negated: vec![],
        conditions: vec![
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Ge,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Param(crate::ir::ParamId(0)),
            }),
            ConditionTree::Leaf(Comparison {
                op: CmpOp::Lt,
                lhs: Term::Var(VarId(2)),
                rhs: Term::Param(crate::ir::ParamId(1)),
            }),
        ],
    })
}

#[test]
fn store_keyed_range_installs_append_and_matches_forced_hash_control() {
    // Same shape as the benchmark: output(id, amount), range predicate on
    // an unprojected scalar. The fixture calls that scalar account, not at.
    let rows: Vec<_> = (0..513u64)
        .map(|id| (id, id % 11, "", i64::try_from(id % 7).unwrap()))
        .collect();
    let store = posting_store("prepared-proved-output", &rows);
    let query = keyed_range_query(true);
    for fallback in [false, true] {
        let mut append = store.prepare(&query).unwrap();
        assert!(
            output_hashing_is_elided(&append),
            "prove and activate the actual store pipeline"
        );
        let mut hashed = store.prepare(&query).unwrap();
        let rule = &hashed.pipeline.main_rules()[0];
        hashed.sink =
            EitherSink::Projection(ProjectionSink::from_finds(rule.finds(), rule.slot_count()));
        assert!(!output_hashing_is_elided(&hashed));
        for prepared in [&mut append, &mut hashed] {
            prepared.force_cursor_fallback(fallback);
        }
        for (lower, upper) in [(0u64, 3u64), (4, 8), (9, 11), (0, 11), (0, 3)] {
            let params = [BindValue::U64(lower), BindValue::U64(upper)];
            let actual = id_amount_pairs(&store.execute(&mut append, &params).unwrap());
            let control = id_amount_pairs(&store.execute(&mut hashed, &params).unwrap());
            assert_eq!(actual, control, "same first-emission order, not just a set");
            let mut sorted = actual;
            sorted.sort_unstable();
            let expected: Vec<_> = rows
                .iter()
                .filter(|row| row.1 >= lower && row.1 < upper)
                .map(|row| (row.0, row.3))
                .collect();
            assert_eq!(sorted, expected, "independent literal range denotation");
        }
    }
}

#[test]
fn hidden_keys_heap_and_overlapping_union_keep_output_hashing() {
    let rows: Vec<_> = (0..33u64)
        .map(|id| (id, id % 11, "", i64::try_from(id % 7).unwrap()))
        .collect();
    let store = posting_store("prepared-output-proof-exclusions", &rows);
    let query = keyed_range_query(true);
    let mut repeats = store.prepare(&keyed_range_query(false)).unwrap();
    assert!(
        !output_hashing_is_elided(&repeats),
        "hidden ids cannot prove amount unique"
    );
    assert_eq!(
        store
            .execute(&mut repeats, &[BindValue::U64(0), BindValue::U64(11)])
            .unwrap()
            .len(),
        7
    );
    let heap = postings(&rows);
    assert!(
        !output_hashing_is_elided(&heap.prepare(&query).unwrap()),
        "heap is deliberately outside the initial optimization scope"
    );

    let mut left = query.rules()[0].clone();
    left.conditions.push(ConditionTree::Leaf(Comparison {
        op: CmpOp::Lt,
        lhs: Term::Var(VarId(2)),
        rhs: Term::Literal(Value::U64(7)),
    }));
    let mut right = query.rules()[0].clone();
    right.conditions.push(ConditionTree::Leaf(Comparison {
        op: CmpOp::Ge,
        lhs: Term::Var(VarId(2)),
        rhs: Term::Literal(Value::U64(5)),
    }));
    let union = Query {
        interiors: vec![],
        rec: None,
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![left, right],
    };
    let mut union = store.prepare(&union).unwrap();
    assert_eq!(
        union.pipeline.main_rules().len(),
        2,
        "neither overlapping arm subsumes the other"
    );
    assert!(!output_hashing_is_elided(&union));
    let mut union_rows = id_amount_pairs(
        &store
            .execute(&mut union, &[BindValue::U64(0), BindValue::U64(11)])
            .unwrap(),
    );
    union_rows.sort_unstable();
    assert_eq!(
        union_rows,
        rows.iter().map(|row| (row.0, row.3)).collect::<Vec<_>>()
    );
}

#[test]
fn u64_ranges_and_cross_atom_residuals_match_nested_loops() {
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "c", 25),
        (4, 7, "d", 40),
        (5, 9, "e", -5),
        (6, 9, "f", 40),
    ];
    let fix = postings(rows);

    let range = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(0), Term::Var(VarId(0))),
                (FieldId(1), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::U64(7)),
        })],
    });
    let mut prepared = fix.prepare(&range).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let mut got: Vec<u64> = (0..out.len())
        .map(|answer| match out.get(answer, 0) {
            AnswerValue::U64(id) => id,
            other => panic!("column 0 is u64: {other:?}"),
        })
        .collect();
    got.sort_unstable();
    let mut expected: Vec<u64> = rows.iter().filter(|r| r.1 >= 7).map(|r| r.0).collect();
    expected.sort_unstable();
    assert_eq!(got, expected, "u64 ordered comparison");

    let spread = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(0))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(2))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Lt,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Var(VarId(1)),
        })],
    });
    let mut prepared = fix.prepare(&spread).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let mut got: Vec<(i64, i64)> = (0..out.len())
        .map(|answer| match (out.get(answer, 0), out.get(answer, 1)) {
            (AnswerValue::I64(x), AnswerValue::I64(y)) => (x, y),
            other => panic!("two i64 columns: {other:?}"),
        })
        .collect();
    got.sort_unstable();
    let mut expected = std::collections::BTreeSet::new();
    for p1 in rows {
        for p2 in rows {
            if p1.1 == p2.1 && p1.3 < p2.3 {
                expected.insert((p1.3, p2.3));
            }
        }
    }
    assert_eq!(
        got,
        expected.into_iter().collect::<Vec<_>>(),
        "cross-atom residual"
    );
}

#[test]
fn aggregates_fold_every_binding_of_existential_suffixes() {
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 7, "a", 10),
        (2, 7, "b", 10),
        (3, 7, "c", 20),
        (4, 8, "z", 5),
    ];
    let fix = postings(rows);

    let query = Query::single(Rule {
        finds: vec![
            FindTerm::Var(VarId(0)),
            FindTerm::Aggregate {
                op: FoldOp::Sum,
                over: VarId(1),
            },
        ],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(3), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });

    let mut bindings = std::collections::BTreeSet::new();
    for p1 in rows {
        for p2 in rows {
            if p1.1 == p2.1 {
                bindings.insert((p1.1, p1.3, p2.2));
            }
        }
    }
    let mut expected = std::collections::BTreeMap::new();
    for (x, y, _) in &bindings {
        *expected.entry(*x).or_insert(0i64) += y;
    }

    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    let mut got: Vec<(u64, i64)> = (0..out.len())
        .map(|answer| {
            let AnswerValue::U64(account) = out.get(answer, 0) else {
                panic!("column 0 is u64");
            };
            let AnswerValue::I64(sum) = out.get(answer, 1) else {
                panic!("column 1 is i64");
            };
            (account, sum)
        })
        .collect();
    got.sort_unstable();
    assert_eq!(got, expected.into_iter().collect::<Vec<_>>());
}

#[test]
fn ne_against_a_never_interned_string_matches_everything() {
    let fix = postings(&[(1, 7, "rent", -1200), (2, 9, "food", -55)]);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::String(Box::from("ghost"))),
        })],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("execute");
    assert_eq!(out.len(), 2, "no stored memo equals a never-stored value");

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(POSTING),
            bindings: vec![
                (FieldId(2), Term::Var(VarId(1))),
                (FieldId(3), Term::Var(VarId(0))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ne,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Param(crate::ir::ParamId(0)),
        })],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[BindValue::Str("ghost")])
        .expect("execute");
    assert_eq!(out.len(), 2);

    let out = fix
        .execute(&mut prepared, &[BindValue::Str("rent")])
        .expect("execute");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::I64(-55));
}

#[test]
fn results_decode_text_tokens_to_original_bytes() {
    let fix = postings(&[(1, 7, "a rather long memo text", 10)]);
    let mut prepared = fix.prepare(&by_account_query()).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("execute");
    assert_eq!(
        out.get(0, 0),
        AnswerValue::String("a rather long memo text")
    );
}

#[test]
fn store_and_heap_sources_agree_on_the_same_rows() {
    // The same query over the same rows through both C05 sources: the
    // committed store (images from canonical rows via the interner) and
    // the admitted heap instance.
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "b", 25),
        (4, 7, "d", 40),
    ];
    let heap = postings(rows);
    let store = posting_store("prepared-store-heap-parity", rows);
    let query = by_account_query();

    let mut on_heap = heap.prepare(&query).expect("prepare heap");
    let heap_out = heap
        .execute(&mut on_heap, &[BindValue::U64(3), BindValue::I64(0)])
        .expect("execute heap");
    let mut on_store = store.prepare(&query).expect("prepare store");
    let store_out = store
        .execute(&mut on_store, &[BindValue::U64(3), BindValue::I64(0)])
        .expect("execute store");
    assert_eq!(answers_of(&heap_out), answers_of(&store_out));
    assert_eq!(
        answers_of(&heap_out),
        vec![("a".into(), 10), ("b".into(), 25)]
    );
}

#[test]
fn a_prepared_query_refuses_a_foreign_source() {
    // Identity is pinned at prepare: a heap-prepared plan cannot run
    // against a store lease, nor a store plan against another store.
    let rows: &[(u64, u64, &str, i64)] = &[(1, 3, "a", 10)];
    let heap = postings(rows);
    let store = posting_store("prepared-foreign-source", rows);
    let query = by_memo_query();

    let mut heap_prepared = heap.prepare(&query).expect("prepare heap");
    let refused = store.execute(&mut heap_prepared, &memo_param("a"));
    assert!(
        matches!(refused, Err(Error::ForeignPreparedQuery)),
        "heap plan on a store lease refuses, got {refused:?}"
    );

    let other = posting_store("prepared-foreign-source-b", rows);
    let mut store_prepared = store.prepare(&query).expect("prepare store");
    let refused = other.execute(&mut store_prepared, &memo_param("a"));
    assert!(
        matches!(refused, Err(Error::ForeignPreparedQuery)),
        "another environment's lease refuses, got {refused:?}"
    );
}

#[test]
fn heap_prepared_witnesses_require_the_same_schema_laws() {
    use bumbledb_theory::schema::{StatementDescriptor, ValueType};

    let keyed = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Item".into(),
            fields: ["id", "payload"]
                .map(|name| FieldDescriptor {
                    name: name.into(),
                    value_type: ValueType::U64,
                })
                .into(),
        }],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(0),
            projection: Box::new([FieldId(0)]),
        }],
    };
    let mut unkeyed = keyed.clone();
    unkeyed.statements.clear();
    let admit = |descriptor: SchemaDescriptor, rows: &[(u64, u64)]| {
        let mut builder =
            InstanceBuilder::new(descriptor, crate::api::db::test_operation()).expect("schema");
        let facts: Vec<_> = rows
            .iter()
            .map(|&(id, payload)| vec![Value::U64(id), Value::U64(payload)])
            .collect();
        builder.load_dyn(RelationId(0), facts.iter()).expect("load");
        builder.admit().expect("admission").expect("lawful rows")
    };
    // Identical Rust typestate and field layout do not imply identical laws.
    let original: OwnedInstance<SchemaDescriptor> = admit(keyed.clone(), &[(1, 10), (2, 20)]);
    let compatible = admit(keyed, &[(7, 30)]);
    let foreign = admit(unkeyed, &[(1, 10), (1, 20)]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Count],
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![(FieldId(0), Term::Var(VarId(0)))],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(0)),
            rhs: Term::Param(crate::ir::ParamId(0)),
        })],
    });
    let params = [BindValue::U64(0)];
    let mut prepared = original.prepare(&query).expect("prepare with key witness");
    let mut out = Answers::new();
    prepared
        .execute_owned(&original, &params, &mut out)
        .expect("original");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(2));

    // Identity is semantic schema identity, not the instance or Arc address.
    prepared
        .execute_owned(&compatible, &params, &mut out)
        .expect("same schema");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(1));

    let refused = prepared.execute_owned(&foreign, &params, &mut out);
    assert!(matches!(refused, Err(Error::ForeignPreparedQuery)));
    assert_eq!(
        out.len(),
        1,
        "identity refusal must not reset caller output"
    );
    assert_eq!(out.get(0, 0), AnswerValue::U64(1));
    let invalid_params: &[BindValue] = &[];
    let refused = prepared.execute_owned(&foreign, invalid_params, &mut out);
    assert!(
        matches!(refused, Err(Error::ForeignPreparedQuery)),
        "foreign schema must refuse before parameter binding"
    );

    // The unkeyed instance itself is valid: its two facts bind one distinct id.
    let mut own = foreign
        .prepare(&query)
        .expect("prepare without a key witness");
    own.execute_owned(&foreign, &params, &mut out)
        .expect("unkeyed execution");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(1));
    prepared
        .execute_owned(&original, &params, &mut out)
        .expect("retry original");
    assert_eq!(out.len(), 1);
    assert_eq!(out.get(0, 0), AnswerValue::U64(2));
}

#[test]
fn forced_cursor_fallback_agrees_with_the_resident_path() {
    // Q-FALLBACK: the complete cursor fallback and the resident Free Join
    // path produce identical sets and identical param/latch behavior.
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "b", 25),
        (4, 7, "d", 40),
        (5, 9, "e", -5),
    ];
    let store = posting_store("prepared-forced-fallback", rows);
    let query = by_account_query();

    let mut resident = store.prepare(&query).expect("prepare");
    let expected = store
        .execute(&mut resident, &[BindValue::U64(3), BindValue::I64(0)])
        .expect("resident execute");

    let mut fallback = store.prepare(&query).expect("prepare");
    fallback.force_cursor_fallback(true);
    let got = store
        .execute(&mut fallback, &[BindValue::U64(3), BindValue::I64(0)])
        .expect("fallback execute");
    assert_eq!(answers_of(&expected), answers_of(&got));

    // Re-binding on the fallback path behaves identically too.
    let got = store
        .execute(&mut fallback, &[BindValue::U64(7), BindValue::I64(30)])
        .expect("fallback execute");
    assert_eq!(answers_of(&got), vec![("d".into(), 40)]);
}

#[test]
fn explicitly_spilled_projection_finalizes_exactly_and_reuses_its_plan() {
    let store = posting_store(
        "prepared-explicit-spill",
        &[
            (1, 3, "a", 10),
            (2, 3, "b", 25),
            (3, 7, "b", 25),
            (4, 7, "d", 40),
        ],
    );
    let mut prepared = store.prepare(&by_account_query()).unwrap();
    for account in [3, 7, 3] {
        let expected = store
            .execute(&mut prepared, &[BindValue::U64(account), BindValue::I64(0)])
            .unwrap();
        let EitherSink::Projection(sink) = &mut prepared.sink else {
            panic!("projection")
        };
        sink.force_spill().unwrap();
        let work = crate::work::WorkContext::new();
        let generation = prepared
            .text_generation
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap();
        let interner = crate::image::intern::InternerHandle::new(&generation, &work);
        let mut actual = Answers::new();
        actual.begin(prepared.signature.columns.len());
        super::super::finalize::finalize(
            &mut prepared.sink,
            &mut prepared.answer_scratch,
            &mut prepared.resolve_memo,
            &interner,
            &prepared.signature.columns,
            &mut actual,
            &work,
        )
        .unwrap();
        assert_eq!(answers_of(&actual), answers_of(&expected));
    }
}

#[test]
fn execute_complete_seals_only_full_results_and_pages_them() {
    // C05: CompleteResult seals after full evaluation; the consuming
    // cursor delivers every row exactly once with a terminal frame.
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 3, "c", 30),
        (4, 3, "d", 45),
        (5, 3, "e", 50),
    ];
    let store = posting_store("prepared-complete-result", rows);
    let mut prepared = store.prepare(&by_account_query()).expect("prepare");
    let sealed = store
        .db
        .read(crate::api::db::test_operation(), |instance| {
            prepared.execute_complete(instance, &[BindValue::U64(3), BindValue::I64(0)])
        })
        .expect("sealed result");
    assert_eq!(sealed.len(), 5);
    let mut cursor = sealed.into_cursor(2);
    let mut rows_seen = Vec::new();
    let mut terminal_pages = 0;
    while let Some(page) = cursor
        .next_page(&crate::api::db::test_operation())
        .expect("page")
    {
        for answer in 0..page.rows.len() {
            let AnswerValue::String(memo) = page.rows.get(answer, 0) else {
                panic!("column 0 is a string");
            };
            rows_seen.push(memo.to_owned());
        }
        if page.terminal {
            terminal_pages += 1;
        }
    }
    rows_seen.sort();
    assert_eq!(rows_seen, vec!["a", "b", "c", "d", "e"]);
    assert_eq!(terminal_pages, 1, "exactly one terminal frame");
}

/// Selective cursor execution agrees with resident execution; cancellation
/// is an error, never a reason to restart or fabricate an empty answer.
#[test]
fn selective_cursor_execution_is_exact_and_cancellation_does_not_restart() {
    use bumbledb_theory::schema::{
        FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType,
    };
    const METRIC: RelationId = RelationId(0);

    // A text-free relation: the resident image slab charge scales with
    // rows, while the fallback's cursor walk interns nothing.
    let descriptor = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Metric".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "bucket".into(),
                    value_type: ValueType::U64,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: ValueType::I64,
                },
            ],
        }],
        statements: vec![],
    };
    let fix = StoreFix::store("prepared-restart-compose", descriptor);
    let rows: Vec<Vec<Value>> = (0..4096u64)
        .map(|i| {
            vec![
                Value::U64(i),
                Value::U64(i % 3),
                Value::I64(i.cast_signed() - 2048),
            ]
        })
        .collect();
    fix.insert_dyn(METRIC, &rows);

    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: crate::ir::AtomSource::Edb(METRIC),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![ConditionTree::Leaf(Comparison {
            op: CmpOp::Ge,
            lhs: Term::Var(VarId(1)),
            rhs: Term::Literal(Value::I64(2040)),
        })],
    });

    let render = |answers: &Answers| -> Vec<(u64, i64)> {
        let mut rows: Vec<(u64, i64)> = (0..answers.len())
            .map(|row| {
                let (AnswerValue::U64(bucket), AnswerValue::I64(amount)) =
                    (answers.get(row, 0), answers.get(row, 1))
                else {
                    panic!("typed metric answers")
                };
                (bucket, amount)
            })
            .collect();
        rows.sort_unstable();
        rows
    };

    let mut resident = fix.prepare(&query).expect("prepare");
    let expected = render(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert_eq!(expected.len(), 8, "amounts 2040..=2047");

    let mut cursor = fix.prepare(&query).unwrap();
    cursor.force_cursor_fallback(true);
    fix.db.read(crate::work::WorkContext::new(), |instance| {
        let cancelled = crate::work::WorkContext::new();
        cancelled.cancel();
        let stopped = super::super::source::QuerySource::store(instance.snapshot(), &cancelled);
        let active = crate::work::WorkContext::new();
        let source = super::super::source::QuerySource::store(instance.snapshot(), &active);
        let mut out = Answers::new();
        for _ in 0..2 {
            cursor.execute_source(&source, &[] as &[BindValue], &mut out)?;
            assert_eq!(render(&out), expected);
            let result = cursor.execute_source(&stopped, &[] as &[BindValue], &mut out);
            assert!(matches!(result, Err(Error::Store(error))
                if matches!(*error, crate::storage::store::StoreError::Work(crate::work::WorkError::Cancelled))));
            assert!(out.is_empty(), "no stale or partial result");
        }
        Ok(())
    }).unwrap();
}
