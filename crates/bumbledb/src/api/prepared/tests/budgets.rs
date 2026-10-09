//! Ordinary allocation, explicit cancellation, and completed-result ownership.
//! Cursor execution is a representation alternative. Completed-result paging
//! does not make execution streaming.
use super::*;
use crate::ir::FoldOp;

use bumbledb_theory::schema::ValueType as TheoryValueType;

const METRIC: RelationId = RelationId(0);

/// Text-free input isolates join and result ownership.
fn metric_descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Metric".into(),
            fields: vec![
                FieldDescriptor {
                    name: "id".into(),
                    value_type: TheoryValueType::U64,
                },
                FieldDescriptor {
                    name: "bucket".into(),
                    value_type: TheoryValueType::U64,
                },
                FieldDescriptor {
                    name: "amount".into(),
                    value_type: TheoryValueType::I64,
                },
            ],
        }],
        statements: vec![],
    }
}

fn metric_store(name: &'static str, rows: u64) -> StoreFix {
    let fix = StoreFix::store(name, metric_descriptor());
    let facts: Vec<Vec<Value>> = (0..rows)
        .map(|i| {
            vec![
                Value::U64(i),
                Value::U64(i % 3),
                Value::I64(i.cast_signed() - 64),
            ]
        })
        .collect();
    fix.insert_dyn(METRIC, &facts);
    fix
}

/// A self-join on id forces a sibling level map over every key.
fn self_join() -> Query {
    Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: crate::ir::AtomSource::Edb(METRIC),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: crate::ir::AtomSource::Edb(METRIC),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    })
}

fn bucket_amounts(answers: &Answers) -> Vec<(u64, i64)> {
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
}

/// A self-join preserves answers and can be reused after release.
#[test]
fn self_joins_preserve_answers_after_memory_release() {
    let fix = metric_store("ordinary-self-join", 640);
    let query = self_join();
    let mut expected: Vec<_> = (0..640i64)
        .map(|id| (u64::try_from(id).unwrap() % 3, id - 64))
        .collect();
    expected.sort_unstable();
    let mut prepared = fix.prepare(&query).unwrap();
    for _ in 0..3 {
        let out = fix.execute(&mut prepared, &[] as &[BindValue]).unwrap();
        prepared.release_memory();
        assert_eq!(bucket_amounts(&out), expected);
    }
}

fn interior_amounts(answers: &Answers) -> Vec<(u64, i64)> {
    bucket_amounts(answers)
}

/// Projection interiors deduplicate exactly and reset cleanly for reuse.
#[test]
fn projection_interior_deduplicates_and_reuses() {
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 10),
        (3, 7, "c", 25),
        (4, 7, "d", 25),
        (5, 9, "e", -5),
        (6, 9, "f", 40),
        (7, 9, "g", 40),
    ];
    let fix = postings(rows);
    // Interior: distinct (account, amount) pairs; main reads the stage.
    let query = Query {
        interiors: vec![Interior {
            rules: vec![
                crate::ir::ProjectionRule {
                    finds: vec![VarId(0), VarId(1)],
                    atoms: vec![Atom {
                        source: AtomSource::Edb(POSTING),
                        bindings: vec![
                            (FieldId(1), Term::Var(VarId(0))),
                            (FieldId(3), Term::Var(VarId(1))),
                        ],
                    }],
                    negated: vec![],
                    conditions: vec![],
                }
                .to_rule(),
            ],
        }],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    };

    let mut resident = fix.prepare(&query).expect("prepare");
    let expected = interior_amounts(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert_eq!(expected.len(), 4, "distinct (account, amount) pairs");

    let again = interior_amounts(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("re-execute"),
    );
    assert_eq!(again, expected);
}

/// Aggregate interiors fold exactly.
#[test]
fn aggregate_interior_folds_per_group() {
    let rows: &[(u64, u64, &str, i64)] = &[
        (1, 3, "a", 10),
        (2, 3, "b", 25),
        (3, 7, "c", 25),
        (4, 7, "d", 40),
        (5, 9, "e", -5),
        (6, 9, "f", 40),
    ];
    let fix = postings(rows);
    // Interior: per-account exact Sum of amounts; main reads the stage.
    let query = Query {
        interiors: vec![Interior {
            rules: vec![Rule {
                finds: vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: FoldOp::Sum,
                        over: VarId(1),
                    },
                ],
                atoms: vec![Atom {
                    source: AtomSource::Edb(POSTING),
                    bindings: vec![
                        (FieldId(1), Term::Var(VarId(0))),
                        (FieldId(3), Term::Var(VarId(1))),
                    ],
                }],
                negated: vec![],
                conditions: vec![],
            }],
        }],
        head: vec![HeadTerm::Var, HeadTerm::Var],
        rules: vec![Rule {
            finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
            atoms: vec![Atom {
                source: AtomSource::Interior(InteriorId(0)),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            }],
            negated: vec![],
            conditions: vec![],
        }],
        rec: None,
    };

    let mut resident = fix.prepare(&query).expect("prepare");
    let expected = interior_amounts(
        &fix.execute(&mut resident, &[] as &[BindValue])
            .expect("resident"),
    );
    assert_eq!(
        expected,
        vec![(3, 35), (7, 65), (9, 35)],
        "per-account sums"
    );
}

/// Borrowed result visits allocate no second Answers carrier. Cancellation
/// stops the visitor immediately, does not mutate the sealed owner, and does
/// not poison a fresh delivery context or a reused preparation.
#[test]
fn completed_results_support_cancellable_borrowed_delivery_without_copying() {
    use crate::work::WorkContext;
    let fix = metric_store("borrowed-result-delivery", 4096);
    let query = self_join();
    let mut prepared = fix.prepare(&query).unwrap();
    let complete = fix
        .db
        .read(WorkContext::new(), |instance| {
            prepared.execute_complete(instance, &[] as &[BindValue])
        })
        .unwrap();
    assert_eq!(complete.len(), 4096);
    prepared.release_memory();

    let cancelled = WorkContext::new();
    let mut visits = 0;
    let error = complete
        .visit_rows(&cancelled, |_| {
            visits += 1;
            if visits == 257 {
                cancelled.cancel();
            }
            Ok(())
        })
        .unwrap_err();
    assert!(error.is_cancelled());
    assert_eq!(visits, 257);
    assert_eq!(complete.len(), 4096);

    let work = WorkContext::new();
    let mut seen = vec![false; 4096];
    let before = crate::alloc_counter::snapshot().window;
    complete
        .visit_rows(&work, |row| {
            let mut values = row.values();
            let Some(AnswerValue::U64(bucket)) = values.next() else {
                panic!("bucket")
            };
            let Some(AnswerValue::I64(amount)) = values.next() else {
                panic!("amount")
            };
            assert!(values.next().is_none());
            let id = usize::try_from(amount + 64).unwrap();
            assert_eq!(bucket, id as u64 % 3);
            assert!(!seen[id]);
            seen[id] = true;
            Ok(())
        })
        .unwrap();
    let after = crate::alloc_counter::snapshot().window;
    assert!(seen.into_iter().all(|visited| visited));
    assert_eq!(
        after.allocs, before.allocs,
        "borrowed delivery allocates no result copy"
    );
    let _ = (before, after);
    let before_move = crate::alloc_counter::snapshot().window;
    let answers = complete.into_answers();
    let after_move = crate::alloc_counter::snapshot().window;
    assert_eq!(
        after_move.allocs, before_move.allocs,
        "consuming the owner only moves storage"
    );
    let _ = (before_move, after_move);
    assert_eq!(
        bucket_amounts(&answers),
        bucket_amounts(&fix.execute(&mut prepared, &[] as &[BindValue]).unwrap())
    );
}
