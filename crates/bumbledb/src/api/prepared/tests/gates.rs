//! Authored discriminators D07–D12 / D25 for the L05 query machine.
//! Each gate is a consumer of the production
//! execute/delivery path — not a `type_name` / `size_of` / fn-ref claim.

use super::*;
use crate::ir::{
    Atom, AtomSource, FindTerm, HeadTerm, Interior, InteriorId, ParamId, Query, Rule, Term, Value,
    VarId,
};
use crate::work::{WorkContext, WorkError};
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor,
};

/// Cancellation of an explicit execution context stops the production path.
#[test]
fn cancelled_execution_context_refuses_execute() {
    let rows = &[(1, 3, "a", 10), (2, 3, "b", 25), (3, 7, "c", 40)];
    let store = posting_store("d07-tiny-units", rows);
    let mut prepared = store.prepare(&by_account_query()).expect("prepare");
    let work = WorkContext::new();
    work.cancel();
    let refused = store.db.read(crate::api::db::test_operation(), |instance| {
        prepared.execute_collect_with_work(instance, &work, &[BindValue::U64(3), BindValue::I64(0)])
    });
    assert!(
        refused.is_err(),
        "cancelled work cannot complete execution, got {refused:?}"
    );
}

#[test]
fn allocating_collect_obeys_frame_cancellation_and_retries_cleanly() {
    let store = posting_store("collect-result-cap", &[(1, 3, "a", 10), (2, 3, "b", 25)]);
    let ample = WorkContext::new();
    let snapshot = store.db.snapshot(&ample).unwrap();
    let mut prepared = snapshot.frame(&ample).prepare(&by_account_query()).unwrap();
    let tiny = WorkContext::new();
    tiny.cancel();
    let error = snapshot
        .frame(&tiny)
        .execute_collect(&mut prepared, &[BindValue::U64(3), BindValue::I64(0)])
        .expect_err("cancelled collection cannot publish rows");
    assert!(matches!(
        error,
        crate::Error::Store(error) if matches!(
            *error,
            crate::storage::store::StoreError::Work(WorkError::Cancelled)
        )
    ));
    let rows = snapshot
        .frame(&ample)
        .execute_collect(&mut prepared, &[BindValue::U64(3), BindValue::I64(0)])
        .unwrap();
    assert_eq!(rows.len(), 2);
}

/// Reusable join pools are query-owned, not owned by the output answers.
#[test]
fn completed_execution_keeps_query_pools_until_explicit_release() {
    use bumbledb_theory::schema::ValueType;
    const METRIC: RelationId = RelationId(0);
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
    let store = StoreFix::store("d08-retain-capacity", descriptor);
    let facts: Vec<Vec<Value>> = (0..256u64)
        .map(|i| {
            vec![
                Value::U64(i),
                Value::U64(i % 4),
                Value::I64(i.cast_signed()),
            ]
        })
        .collect();
    store.insert_dyn(METRIC, &facts);
    // Self-join on id forces a COLT level map; no Functionality key
    // so this is not a key-probe.
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(1)), FindTerm::Var(VarId(2))],
        atoms: vec![
            Atom {
                source: AtomSource::Edb(METRIC),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(1), Term::Var(VarId(1))),
                ],
            },
            Atom {
                source: AtomSource::Edb(METRIC),
                bindings: vec![
                    (FieldId(0), Term::Var(VarId(0))),
                    (FieldId(2), Term::Var(VarId(2))),
                ],
            },
        ],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = store.prepare(&query).expect("prepare");
    let work = WorkContext::new();
    store
        .db
        .read(crate::api::db::test_operation(), |instance| {
            let out = prepared.execute_collect_with_work(instance, &work, &[] as &[BindValue])?;
            assert_eq!(out.len(), 256, "one pair per metric id");
            let pool_bytes = |prepared: &PreparedQuery<T>| {
                prepared
                    .pipeline
                    .main_rules()
                    .iter()
                    .map(|rule| match rule {
                        PreparedRule::FreeJoin(rule) => rule
                            .memo
                            .colts
                            .iter()
                            .map(Colt::retained_bytes)
                            .sum::<usize>(),
                        PreparedRule::KeyProbe(_) => 0,
                    })
                    .sum::<usize>()
            };
            let retained = pool_bytes(&prepared);
            assert!(retained > 0);
            drop(out);
            assert_eq!(
                pool_bytes(&prepared),
                retained,
                "answers do not own query pools"
            );
            let before = crate::alloc_counter::snapshot().window;
            prepared.release_memory();
            let after = crate::alloc_counter::snapshot().window;
            assert_eq!(pool_bytes(&prepared), 0);
            #[cfg(feature = "alloc-counter")]
            assert!(after.dealloc_bytes - before.dealloc_bytes >= retained as u64);
            let _ = (before, after);
            let again = prepared.execute_collect_with_work(instance, &work, &[] as &[BindValue])?;
            assert_eq!(again.len(), 256);
            Ok(())
        })
        .expect("execute");
    drop(prepared);
}

/// Aggregate interior → join + bound negation agrees between the resident
/// executor and the representation fallback; neither requires a quota.
#[test]
fn derived_pipeline_matches_cursor_execution_and_retains_resident_stages() {
    let mut owned: Vec<(u64, u64, String, i64)> = (1..=32u64)
        .map(|id| (id, id, "ok".to_owned(), i64::try_from(id).expect("fits")))
        .collect();
    owned.push((200, 3, "blocked".into(), 1));
    let borrowed: Vec<(u64, u64, &str, i64)> = owned
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    let store = posting_store("d09-derived-pipeline", &borrowed);
    let query = Query {
        interiors: vec![Interior {
            rules: vec![Rule {
                finds: vec![
                    FindTerm::Var(VarId(0)),
                    FindTerm::Aggregate {
                        op: crate::ir::FoldOp::Sum,
                        over: VarId(1),
                    },
                ],
                atoms: vec![Atom {
                    source: AtomSource::Edb(super::POSTING),
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
            negated: vec![Atom {
                source: AtomSource::Edb(super::POSTING),
                bindings: vec![
                    (FieldId(1), Term::Var(VarId(0))),
                    (FieldId(2), Term::Literal(Value::String("blocked".into()))),
                ],
            }],
            conditions: vec![],
        }],
        rec: None,
    };
    let expected: Vec<(u64, i64)> = (1..=32u64)
        .filter(|&account| account != 3)
        .map(|account| (account, i64::try_from(account).expect("fits")))
        .collect();

    let pairs = |answers: &Answers| -> Vec<(u64, i64)> {
        let mut rows: Vec<(u64, i64)> = (0..answers.len())
            .map(|row| {
                let (AnswerValue::U64(account), AnswerValue::I64(sum)) =
                    (answers.get(row, 0), answers.get(row, 1))
                else {
                    panic!("typed (account, sum) answers")
                };
                (account, sum)
            })
            .collect();
        rows.sort_unstable();
        rows
    };

    let mut resident = store.prepare(&query).expect("prepare");
    let work_resident = WorkContext::new();
    let got_resident = store
        .db
        .read(crate::api::db::test_operation(), |instance| {
            resident.execute_collect_with_work(instance, &work_resident, &[] as &[BindValue])
        })
        .expect("resident");
    assert_eq!(pairs(&got_resident), expected);
    assert!(
        resident
            .derived
            .published
            .iter()
            .all(super::super::derived::SealedStage::is_resident)
    );
    let mut spilled = store.prepare(&query).expect("prepare");
    spilled.force_cursor_fallback(true);
    let work_spill = WorkContext::new();
    let got_spill = store
        .db
        .read(crate::api::db::test_operation(), |instance| {
            spilled.execute_collect_with_work(instance, &work_spill, &[] as &[BindValue])
        })
        .expect("cursor derived pipeline");
    assert_eq!(
        pairs(&got_spill),
        expected,
        "aggregate→join+negation agrees through cursor execution"
    );
}

/// Independently minted tokens compare by exact bytes across generations.
#[test]
fn text_token_identity_is_generation_scoped_and_owners_survive_reclamation() {
    use crate::image::{CacheGeneration, intern::InternerHandle};
    use crate::work::{GenerationHandle, GenerationState};
    let work = WorkContext::new();
    let first = GenerationHandle::new(GenerationState::new(CacheGeneration::initial()));
    let second = GenerationHandle::new(GenerationState::new(CacheGeneration::initial()));
    let left = InternerHandle::new(&first, &work).intern("shared").unwrap();
    let different = InternerHandle::new(&second, &work)
        .intern("different")
        .unwrap();
    let right = InternerHandle::new(&second, &work)
        .intern("shared")
        .unwrap();
    assert_eq!(
        left.word, different.word,
        "raw words can alias across generations"
    );
    assert_ne!(left.word, right.word);
    assert!(!first.tokens_equal(left.word, &second, different.word));
    assert!(first.tokens_equal(left.word, &second, right.word));
    first.lock_resolver().reclaim_unowned();
    second.lock_resolver().reclaim_unowned();
    assert!(first.tokens_equal(left.word, &second, right.word));
    assert_eq!(left.text.as_ref(), "shared");
    assert_eq!(right.text.as_ref(), "shared");
}

/// D10: a key-bound query over many unrelated rows visits through the
/// compiled witness, not a full scan.
#[test]
fn d10_key_bound_query_visits_are_bounded() {
    let rows: Vec<(u64, u64, String, i64)> = (0..48)
        .map(|id| {
            (
                id,
                id % 5,
                format!("m{id}"),
                i64::try_from(id).expect("fits"),
            )
        })
        .collect();
    let borrowed: Vec<(u64, u64, &str, i64)> = rows
        .iter()
        .map(|(id, account, memo, amount)| (*id, *account, memo.as_str(), *amount))
        .collect();
    let store = posting_store("d10-key-visits", &borrowed);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: vec![Atom {
            source: AtomSource::Edb(super::POSTING),
            bindings: vec![
                (FieldId(0), Term::Param(ParamId(0))),
                (FieldId(2), Term::Var(VarId(0))),
                (FieldId(3), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = store.prepare(&query).expect("prepare");
    let out = store
        .execute(&mut prepared, &[BindValue::U64(7)])
        .expect("keyed execute");
    assert_eq!(answers_of(&out), vec![("m7".into(), 7)]);
    let visits = prepared.last_visits();
    assert!(
        visits > 0 && visits < borrowed.len(),
        "keyed walk must visit the matching row, not all {} rows (got {visits})",
        borrowed.len()
    );
}

/// D12: `commit` is the only advance. An aborted preview retries the
/// same row. `abort` discards
/// ticket-local pending — a fresh ticket cannot commit that preview.
#[test]
fn d12_preview_does_not_advance_until_commit() {
    let work = WorkContext::new();
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String("alpha"));
    answers.push_value(&AnswerValue::String("beta"));
    let sealed = crate::api::prepared::result::CompleteResult::seal(
        answers,
        crate::api::prepared::result::ResultIdentity {
            source: crate::api::prepared::source::PinnedSource::Heap(
                crate::schema::fingerprint::SchemaFingerprint([0; 32]),
            ),
            generation: None,
        },
        &work,
    )
    .expect("seal");
    let mut cursor = sealed.into_cursor(8);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let preview = ticket
        .preview_page(&work)
        .expect("preview")
        .expect("nonempty");
    assert_eq!(preview.len(), 2);
    ticket.abort();
    assert_eq!(cursor.debug_next_row(), 0, "abort retries the same row");
}

/// D11: Pack output is the logical union, not insertion-token order.
/// Reverse claims `[10,20)` then `[0,15)` become `[0,20)`.
#[test]
fn d11_pack_order_is_logical_not_insertion() {
    use bumbledb_theory::schema::{IntervalElement, ValueType};
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
    };
    let descriptor = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            extension: None,
            name: "Busy".into(),
            fields: vec![
                field("id", ValueType::U64),
                field("person", ValueType::U64),
                field(
                    "slot",
                    ValueType::Interval {
                        element: IntervalElement::U64,
                    },
                ),
            ],
        }],
        statements: vec![],
    };
    let facts = [(1u64, 1u64, 10u64, 20u64), (2, 1, 0, 15), (3, 2, 4, 6)]
        .into_iter()
        .map(|(id, person, start, end)| {
            vec![
                Value::U64(id),
                Value::U64(person),
                Value::IntervalU64(
                    bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty"),
                ),
            ]
        })
        .collect::<Vec<_>>();
    let fix = Fix::heap(descriptor, &[(RelationId(0), facts)]);
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Pack { over: VarId(1) }],
        atoms: vec![Atom {
            source: AtomSource::Edb(RelationId(0)),
            bindings: vec![
                (FieldId(1), Term::Var(VarId(0))),
                (FieldId(2), Term::Var(VarId(1))),
            ],
        }],
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).expect("prepare");
    let out = fix
        .execute(&mut prepared, &[] as &[BindValue])
        .expect("pack execute");
    let mut got: Vec<(u64, u64, u64)> = (0..out.len())
        .map(|row| match (out.get(row, 0), out.get(row, 1)) {
            (AnswerValue::U64(person), AnswerValue::IntervalU64(iv)) => {
                (person, iv.start(), iv.end())
            }
            other => panic!("(u64, interval) {other:?}"),
        })
        .collect();
    got.sort_unstable();
    assert_eq!(
        got,
        vec![(1, 0, 20), (2, 4, 6)],
        "reverse [10,20) then [0,15) unions to [0,20)"
    );
}

/// Page size controls delivery rows, never execution cardinality or bytes.
#[test]
fn into_cursor_page_size_and_ticket_commit_control_advancement() {
    let store = posting_store(
        "page-size",
        &[
            (1, 3, "aaaaaaaa", 10),
            (2, 3, "bbbbbbbb", 25),
            (3, 3, "c", 30),
        ],
    );
    let mut prepared = store.prepare(&by_account_query()).unwrap();
    let sealed = store
        .db
        .read(WorkContext::new(), |instance| {
            prepared.execute_complete(instance, &[BindValue::U64(3), BindValue::I64(0)])
        })
        .unwrap();
    assert_eq!(sealed.len(), 3);
    let work = WorkContext::new();
    let mut cursor = sealed.into_cursor(2);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let preview = ticket.preview_page(&work).unwrap().unwrap();
    assert_eq!(preview.len(), 2, "the configured row count is honored");
    let adopted = ticket.adopt().unwrap();
    let first = answers_of(&adopted);
    ticket.abort();
    assert_eq!(
        cursor.debug_next_row(),
        0,
        "adopting without committing cannot advance"
    );

    let cancelled = WorkContext::new();
    cancelled.cancel();
    assert!(cursor.next_page(&cancelled).is_err());
    assert_eq!(cursor.debug_next_row(), 0);
    let retry = cursor.next_page(&work).unwrap().unwrap();
    assert_eq!(answers_of(&retry.rows), first);
    assert_eq!(cursor.debug_next_row(), 2);
    assert!(!retry.terminal);
    let last = cursor.next_page(&work).unwrap().unwrap();
    assert_eq!(last.rows.len(), 1);
    assert!(last.terminal);
    assert!(cursor.next_page(&work).unwrap().is_none());
}
