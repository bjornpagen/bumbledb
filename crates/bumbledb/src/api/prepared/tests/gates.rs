//! Authored discriminators D07–D12 / D25 for the L05 query machine.
//! Verification: NotRun. Each gate is a consumer of the production
//! execute/delivery path — not a `type_name` / `size_of` / fn-ref claim.

use super::*;
use crate::api::prepared::source::UNBOUNDED_POLICY;
use crate::ir::{
    Atom, AtomSource, FindTerm, HeadTerm, Interior, InteriorId, ParamId, Query, Rule, Term, Value,
    VarId,
};
use crate::work::Resource;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, RelationDescriptor, RelationId, SchemaDescriptor,
};

/// D07: a 1-unit execute ledger refuses on the production path. There is
/// no ordinary MAX/year twin (`UNBOUNDED_POLICY` is `#[cfg(test)]` only).
#[test]
fn d07_tiny_work_units_refuse_execute() {
    let rows = &[(1, 3, "a", 10), (2, 3, "b", 25), (3, 7, "c", 40)];
    let store = posting_store("d07-tiny-units", rows);
    let mut prepared = store.prepare(&by_account_query()).expect("prepare");
    let work = crate::work::ExecutionPolicy {
        work_units: 1,
        ..UNBOUNDED_POLICY
    }
    .start()
    .expect("policy");
    let refused = store.db.read(|instance| {
        prepared.execute_collect_with_work(
            instance,
            &work,
            &[BindValue::U64(3), BindValue::I64(0)],
        )
    });
    assert!(
        refused.is_err(),
        "one work unit cannot complete a charged execute, got {refused:?}"
    );
}

/// D08: a production join that actually grew COLT/working capacity
/// keeps that charge after the operation returns. Dropping the answers
/// does not refund it; dropping the prepared owner does.
/// `used(WorkUnits) > 0` is not this gate.
#[test]
fn d08_successful_execute_retains_work_charges() {
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
    let work = UNBOUNDED_POLICY.start().expect("work");
    store
        .db
        .read(|instance| {
            let out = prepared.execute_collect_with_work(instance, &work, &[] as &[BindValue])?;
            assert_eq!(out.len(), 256, "one pair per metric id");
            let charged = work.used(Resource::WorkingBytes);
            assert!(
                charged > 0,
                "join growth retained working capacity, not a WorkUnits stand-in"
            );
            drop(out);
            assert_eq!(
                work.used(Resource::WorkingBytes),
                charged,
                "dropping the answers does not refund the retained pool"
            );
            Ok(())
        })
        .expect("execute");
    drop(prepared);
    assert_eq!(
        work.used(Resource::WorkingBytes),
        0,
        "the prepared owner held the retained COLT/working charge"
    );
}

/// D09: aggregate interior → join + bound negation, RAM below
/// intermediate cardinality. Spilled and resident answers agree; peak
/// working stay bounded; a tiny nonempty run stays resident.
/// `d09_spill_opens_via_exhausted` stays as the intern-spill sibling.
#[test]
fn d09_derived_pipeline_spills_with_bounded_peak() {
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
    let work_resident = UNBOUNDED_POLICY.start().expect("work");
    let got_resident = store
        .db
        .read(|instance| {
            resident.execute_collect_with_work(instance, &work_resident, &[] as &[BindValue])
        })
        .expect("resident");
    assert_eq!(pairs(&got_resident), expected);
    assert_eq!(
        work_resident.used(Resource::ScratchBytes),
        0,
        "tiny nonempty stages stay resident"
    );
    assert!(
        !resident.used_nonresident_text(),
        "resident path does not open scratch text"
    );

    let mut spilled = store.prepare(&query).expect("prepare");
    spilled.set_sink_ram(0);
    let work_spill = UNBOUNDED_POLICY.start().expect("work");
    let got_spill = store
        .db
        .read(|instance| {
            spilled.execute_collect_with_work(instance, &work_spill, &[] as &[BindValue])
        })
        .expect("spilled derived pipeline");
    assert_eq!(
        pairs(&got_spill),
        expected,
        "aggregate→join+negation agrees after scratch"
    );
    assert!(
        work_spill.used(Resource::ScratchBytes) > 0,
        "RAM below intermediate cardinality opened scratch"
    );
    assert!(
        work_spill.used(Resource::WorkingBytes) < 1 << 20,
        "peak working stays bounded — no whole-image resurrection"
    );
}

/// D09: intern spill matches `BeyondMemory` and L05 opens scratch only
/// through `ResidentTextExhausted::open_nonresident`. `on_work` sees a
/// prior execute charge (not a twin at zero). Intern and scratch tokens
/// do not alias; [`TextEq::tokens_equal`] unifies meaning. Verification:
/// NotRun.
#[test]
fn d09_spill_opens_via_exhausted() {
    use crate::exec::scratch::capability::{ScratchCapability, ScratchPolicy};
    use crate::image::{
        intern::InternerHandle, is_resident_token, is_scratch_token, NonresidentTextStore,
        ResidentAdmit, TextEq,
    };
    use crate::work::{CacheLedger, CachePolicy, GenerationHandle, GenerationState};

    let work = UNBOUNDED_POLICY.start().expect("work");
    work.step(5).expect("prior execute charge");
    let charged = work.used(Resource::WorkUnits);
    assert!(charged > 0, "execute ledger is already running");
    let cap = ScratchCapability::on_work(&work, ScratchPolicy::from_work(&work)).expect("on_work");
    assert_eq!(
        cap.work().used(Resource::WorkUnits),
        charged,
        "on_work sees the prior execute charge, not a twin at zero"
    );

    let fat = GenerationHandle::new(GenerationState::new(
        crate::image::CacheGeneration::initial(),
        CacheLedger::unbounded(),
    ));
    let intern_tok = match InternerHandle::new(&fat, &work)
        .intern_or_spill("shared")
        .expect("fat intern")
    {
        ResidentAdmit::Ready(tok) => tok,
        ResidentAdmit::BeyondMemory(_) => panic!("unbounded cache must admit"),
    };
    assert_eq!(intern_tok, 0);
    assert!(is_resident_token(intern_tok));

    let tiny = GenerationHandle::new(GenerationState::new(
        crate::image::CacheGeneration::initial(),
        CacheLedger::new(CachePolicy { cache_bytes: 8 }),
    ));
    let admitted = InternerHandle::new(&tiny, &work)
        .intern_or_spill("a-text-that-cannot-fit-eight-cache-bytes")
        .expect("unbounded work");
    let ResidentAdmit::BeyondMemory(exhausted) = admitted else {
        panic!("tiny cache intern must spill");
    };
    let scratch_before = work.used(Resource::ScratchBytes);
    let mut store = PreparedQuery::<T>::open_from_exhausted(&exhausted, &work)
        .expect("exhausted.open_nonresident");
    let scratch_tok = store.intern("shared", &work).expect("scratch intern");
    assert!(
        work.used(Resource::ScratchBytes) > scratch_before,
        "open_nonresident via on_work charges the execute ledger"
    );
    let epoch = store.epoch();
    assert!(
        store.live(scratch_tok),
        "dense id belongs to this store instance"
    );
    assert!(
        store.text_eq().accepts_stamp(epoch),
        "TextEq stamps memos with store.epoch(), not a packed tag"
    );
    assert!(is_scratch_token(scratch_tok));
    assert!(NonresidentTextStore::owns_token(scratch_tok));
    assert_ne!(
        intern_tok, scratch_tok,
        "intern and scratch tokens cannot alias"
    );
    assert!(
        TextEq::bind(&fat, Some(&store)).tokens_equal(scratch_tok, intern_tok),
        "TextEq unifies intern and scratch; raw words stay unequal"
    );
    assert!(
        !fat.tokens_equal(intern_tok, &fat, scratch_tok),
        "GenerationHandle::tokens_equal refuses a scratch-tagged id"
    );
    let mut out = Vec::new();
    assert!(store.resolve(scratch_tok, &mut out).expect("resolve"));
    assert_eq!(out, b"shared");
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

/// D12: `preview_page` copies under admitted overlap; `commit` is the
/// only advance. Resource abort retries the same row. `abort` discards
/// ticket-local pending — a fresh ticket cannot commit that preview.
#[test]
fn d12_preview_does_not_advance_until_commit() {
    let work = UNBOUNDED_POLICY.start().expect("work");
    let mut answers = Answers::new();
    answers.begin(1);
    answers.push_value(&AnswerValue::String("alpha"));
    answers.push_value(&AnswerValue::String("beta"));
    let sealed = crate::api::prepared::result::CompleteResult::seal(
        answers,
        crate::api::prepared::result::ResultIdentity {
            source: crate::api::prepared::source::PinnedSource::Heap,
            generation: None,
        },
        &work,
        usize::MAX,
    )
    .expect("seal");
    let mut cursor = sealed.into_cursor(8);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let preview = ticket
        .preview_page(&work, 64)
        .expect("preview")
        .expect("nonempty");
    assert_eq!(preview.len(), 2);
    assert_eq!(cursor.debug_next_row(), 0, "preview leaves next_row");
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
    let facts = [
        (1u64, 1u64, 10u64, 20u64),
        (2, 1, 0, 15),
        (3, 2, 4, 6),
    ]
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

/// D25: `into_cursor(page_rows)` is a real cap (not hardcoded 1). Two
/// rows that fit alone but not together are two pages; commit is the
/// only advance. Verification: NotRun.
#[test]
fn d25_into_cursor_page_cap_commits_once() {
    let rows = &[
        (1, 3, "aaaaaaaa", 10),
        (2, 3, "bbbbbbbb", 25),
        (3, 3, "c", 30),
    ];
    let store = posting_store("d25-page-cap", rows);
    let mut prepared = store.prepare(&by_account_query()).expect("prepare");
    let sealed = store
        .db
        .read(|instance| {
            prepared.execute_complete(instance, &[BindValue::U64(3), BindValue::I64(0)])
        })
        .expect("complete");
    assert_eq!(sealed.len(), 3);
    let work = UNBOUNDED_POLICY.start().expect("work");
    let mut cursor = sealed.into_cursor(8);
    let mut ticket = DeliveryTicket::open(&mut cursor);
    let preview = ticket
        .preview_page(&work, 32)
        .expect("row1 fits alone")
        .expect("nonempty");
    assert_eq!(preview.len(), 1, "row2 jointly exceeds 32 encoded bytes");
    assert_eq!(cursor.debug_next_row(), 0);
    assert!(
        ticket.preview_charged_bytes() > 0,
        "preview reserved before copy"
    );
    let adopted = ticket.adopt().expect("adopt");
    let charge = work
        .reserve(
            crate::work::ByteKind::Result,
            crate::api::prepared::result::logical_bytes_for_test(&adopted),
        )
        .expect("register QueuedOutput stand-in");
    drop(charge);
    ticket.commit();
    assert_eq!(cursor.debug_next_row(), 1);

    let mut ticket = DeliveryTicket::open(&mut cursor);
    ticket.preview_page(&work, 32).expect("row2 preview");
    ticket.abort();
    assert_eq!(cursor.debug_next_row(), 1, "abort retries the same row");

    let retry = cursor
        .next_page_with_work(&work, 32)
        .expect("retry")
        .expect("row2");
    assert_eq!(retry.rows.len(), 1);
}
