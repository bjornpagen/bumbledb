use super::*;
use crate::ir::FoldOp;

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
fn forced_sink_spill_preserves_the_answer_set() {
    // Q-DISK (distinct-state half): zero RAM allowance forces the main
    // sink's seen-set/result rows into the scratch tier from row one; the
    // published set is unchanged.
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "b", 25),
        (4, 7, "d", 40),
    ];
    let store = posting_store("prepared-forced-spill", rows);
    let query = by_account_query();

    let mut spilled = store.prepare(&query).expect("prepare");
    spilled.set_sink_ram(0);
    let got = store
        .execute(&mut spilled, &[BindValue::U64(3), BindValue::I64(0)])
        .expect("spilled execute");
    assert_eq!(answers_of(&got), vec![("a".into(), 10), ("b".into(), 25)]);

    // Success → success reuse on the same spilled plan (Q-ATOMIC shape).
    let got = store
        .execute(&mut spilled, &[BindValue::U64(7), BindValue::I64(0)])
        .expect("spilled re-execute");
    assert_eq!(answers_of(&got), vec![("b".into(), 25), ("d".into(), 40)]);
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
        .read(|instance| {
            prepared.execute_complete(instance, &[BindValue::U64(3), BindValue::I64(0)])
        })
        .expect("sealed result");
    assert_eq!(sealed.len(), 5);
    let mut cursor = sealed.into_cursor(2);
    let mut rows_seen = Vec::new();
    let mut terminal_pages = 0;
    while let Some(page) = cursor.next_page().expect("page") {
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

/// Chapter 12 §6 composition: a resident working-byte refusal (image slab
/// charge) licenses exactly ONE restart through the complete cursor
/// fallback — which succeeds under the same ledger because cursors and the
/// scratch-backed sink state do not need the resident slabs. A ledger too
/// small for either path surfaces the typed exhaustion instead of looping.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one end-to-end exhaustion-restart scenario"
)]
fn working_byte_exhaustion_restarts_once_into_the_fallback() {
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

    let bounded = |working_bytes: u64| {
        crate::work::ExecutionPolicy {
            input_bytes: u64::MAX,
            working_bytes,
            scratch_bytes: u64::MAX,
            result_bytes: u64::MAX,
            rows: u64::MAX,
            work_units: u64::MAX,
            timeout: std::time::Duration::from_secs(3600),
        }
        .start()
        .expect("bounded ledger")
    };

    // 24 KiB: far below the ~100 KiB resident slab charge, ample for the
    // fallback's cursor scan — ONE recorded restart, identical answers.
    let mut restarted = fix.prepare(&query).expect("prepare");
    fix.db
        .read(|instance| {
            let work = bounded(24 << 10);
            let source =
                crate::api::prepared::source::QuerySource::store(instance.snapshot(), &work);
            let mut out = Answers::new();
            restarted.execute_source(&source, &[] as &[BindValue], &mut out)?;
            assert_eq!(render(&out), expected, "the restarted path is the query");
            Ok(())
        })
        .expect("restarted execute");

    // The restart composes with the sink spill, boundedly: working refusal
    // → the ONE restart → the fallback's forced sink spill hits a scratch
    // ledger refusal → the TYPED scratch exhaustion surfaces (a scratch
    // refusal never licenses another restart) and no partial answer
    // publishes (Q-ATOMIC).
    let mut refused = fix.prepare(&query).expect("prepare");
    refused.set_sink_ram(0);
    fix.db
        .read(|instance| {
            let work = crate::work::ExecutionPolicy {
                input_bytes: u64::MAX,
                working_bytes: 24 << 10,
                scratch_bytes: 1,
                result_bytes: u64::MAX,
                rows: u64::MAX,
                work_units: u64::MAX,
                timeout: std::time::Duration::from_secs(3600),
            }
            .start()
            .expect("bounded ledger");
            let source =
                crate::api::prepared::source::QuerySource::store(instance.snapshot(), &work);
            let mut out = Answers::new();
            let result = refused.execute_source(&source, &[] as &[BindValue], &mut out);
            assert!(
                matches!(
                    &result,
                    Err(crate::error::Error::Store(store)) if matches!(
                        **store,
                        crate::storage::store::StoreError::Work(
                            crate::work::WorkError::Exhausted {
                                resource: crate::work::Resource::ScratchBytes,
                                ..
                            }
                        )
                    )
                ),
                "typed scratch exhaustion after the one restart, got {result:?}"
            );
            assert_eq!(out.len(), 0, "no partial answer publishes");
            Ok(())
        })
        .expect("refused execute");
}
