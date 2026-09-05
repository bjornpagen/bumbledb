//! P12 gate coverage: ported safety properties of DELETED 0.x APIs (the G00
//! rule — "the original regression should fail against the audited behavior;
//! if the old API is deleted, retain a baseline reproduction plus successor
//! behavioral tests proving the dangerous capability no longer exists").
//! Baselines live in `audit/`; each test cites its audit row and states the
//! successor property it proves over the LANDED surfaces.
//!
//! Verification: `NotRun` (F2 authors, does not execute).

mod lane_support;

use std::sync::Arc;

use bumbledb::schema::{
    FieldDescriptor, RelationDescriptor, RelationId, SchemaDescriptor, StatementDescriptor,
    ValueType,
};
use bumbledb::{ChangeSet, Db, ExecutionPolicy, FieldId, Id128, Value};

use bumbledb_log::history::command::{Command, CommandMetadata};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseIdentity, ReceiptEpoch, RequestId, TerminalOutcome,
};
use bumbledb_log::store::mem::{Behavior, MemStore, Op};
use bumbledb_log::writer::{HostedHistory, LocalHistory, LogError, ResolveOutcome, SubmitOutcome};

use lane_support::{LIMITS, op, temp_dir, work};

/// Two relations; `Account(id)` carries a Functionality key, `Audit(entry)`
/// does not — the REP-020 fixture.
fn two_relation_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "Audit".into(),
                fields: vec![FieldDescriptor {
                    name: "entry".into(),
                    value_type: ValueType::U64,
                }],
                extension: None,
            },
            RelationDescriptor {
                name: "Account".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                    },
                    FieldDescriptor {
                        name: "balance".into(),
                        value_type: ValueType::U64,
                    },
                ],
                extension: None,
            },
        ],
        statements: vec![StatementDescriptor::Functionality {
            relation: RelationId(1),
            projection: Box::new([FieldId(0)]),
        }],
    }
}

/// `Note(owner: id128, body: string)` for the entity-identity port.
fn id128_schema() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Note".into(),
            fields: vec![
                FieldDescriptor {
                    name: "owner".into(),
                    value_type: ValueType::Id128,
                },
                FieldDescriptor {
                    name: "body".into(),
                    value_type: ValueType::String,
                },
            ],
            extension: None,
        }],
        statements: vec![],
    }
}

fn create_history(
    tag: &str,
    schema: SchemaDescriptor,
) -> (Arc<Db<SchemaDescriptor>>, LocalHistory<SchemaDescriptor>) {
    let dir = temp_dir(tag).join("db");
    let db = Arc::new(
        Db::create(&dir, schema, work())
            .expect("create store")
            .expect("empty store admits"),
    );
    let history = LocalHistory::create(
        Arc::clone(&db),
        bumbledb_log::history::DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        bumbledb_log::history::IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        op(0xc3),
        LIMITS,
        &work(),
    )
    .expect("local history creates");
    (db, history)
}

fn seal_with(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    build: impl FnOnce(&mut bumbledb::ChangeSetBuilder<'_>),
) -> Command {
    let mut draft = ChangeSet::builder(db.schema(), work());
    build(&mut draft);
    let changes = draft.finish().expect("draft finishes");
    Command::seal(
        CommandMetadata {
            identity,
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .expect("command seals")
}

fn rows_in(db: &Db<SchemaDescriptor>, relation: RelationId) -> Vec<Vec<Value>> {
    let mut rows = Vec::new();
    db.read(work(), |read| {
        for row in read.scan(relation)? {
            rows.push(row?);
        }
        Ok(())
    })
    .expect("scan reads");
    rows
}

/// REP-020 baseline (audit/10, split commit discarded earlier successful
/// receipts): the `commit_split` API is DELETED; the successor property is
/// that ONE multi-relation tenant command is all-or-none — a violation in
/// the second relation leaves zero facts from the first and one durable
/// rejection receipt that retries return unchanged (PROTO-10, G09).
#[test]
fn rep020_a_multi_relation_command_is_all_or_none_with_one_receipt() {
    let (db, history) = create_history("rep020", two_relation_schema());
    let command = seal_with(&db, history.identity(), 0x01, |draft| {
        // A lawful audit row plus TWO rows conflicting on Account's key.
        draft
            .insert(RelationId(0), &[Value::U64(777)])
            .expect("insert");
        draft
            .insert(RelationId(1), &[Value::U64(1), Value::U64(100)])
            .expect("insert");
        draft
            .insert(RelationId(1), &[Value::U64(1), Value::U64(200)])
            .expect("insert");
    });
    let receipt = match history.submit(&command, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("a judged rejection is a decision: {other:?}"),
    };
    assert!(
        matches!(receipt.outcome, TerminalOutcome::InvariantRejected { .. }),
        "the key conflict rejects the WHOLE command"
    );
    assert!(
        rows_in(&db, RelationId(0)).is_empty(),
        "no successful prefix survives"
    );
    assert!(rows_in(&db, RelationId(1)).is_empty());
    // The rejection is durable and stable: the retry returns the retained
    // receipt, and no partial state appears on the way.
    let retry = match history.submit(&command, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("retry resolves the retained receipt: {other:?}"),
    };
    assert_eq!(retry.decision_at, receipt.decision_at);
    assert!(rows_in(&db, RelationId(0)).is_empty());
}

/// SDK-001 baseline (audit/30, the next live commit overwrote unresolved
/// Pending state): the successor property is that a failed/unknown attempt
/// followed by ANOTHER command on the same live handle preserves the first
/// command's resolution evidence — here the first attempt's CAS was dropped
/// (never applied), a second command decides, and the first still resolves
/// to a definite typed observation, never an invented outcome (PROTO-17,
/// G07/G09).
#[test]
fn sdk001_a_later_command_preserves_the_earlier_attempts_resolution() {
    let dir = temp_dir("sdk001").join("db");
    let db = Arc::new(
        Db::create(&dir, two_relation_schema(), work())
            .expect("create store")
            .expect("empty store admits"),
    );
    let store = MemStore::new();
    // Drop the first decision CAS: dispatched, never applied, reported
    // indeterminate — the audited machine lost this evidence.
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateDropped);
    let history = HostedHistory::create(
        Arc::clone(&db),
        store,
        "t".to_string(),
        0,
        bumbledb_log::history::DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        bumbledb_log::history::IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        op(0xc3),
        LIMITS,
        &work(),
    )
    // Bound the attempt budget to one so the dropped CAS surfaces instead of
    // being retried into success — the certainty arm under test.
    .expect("hosted history creates")
    .with_attempts(1);

    let first = seal_with(&db, history.identity(), 0x11, |draft| {
        draft
            .insert(RelationId(0), &[Value::U64(1)])
            .expect("insert");
    });
    let first_ref = first.command_ref();
    match history.submit(&first, &work()) {
        // An honest internal resolution (Decided) is also lawful; then the
        // property below still holds trivially.
        SubmitOutcome::OutcomeUnknown { .. } | SubmitOutcome::Decided { .. } => {}
        SubmitOutcome::NotSubmitted { error, .. } => {
            panic!("a dispatched attempt is never rewritten to NotSubmitted: {error:?}")
        }
    }

    // The next command on the SAME live handle.
    let second = seal_with(&db, history.identity(), 0x12, |draft| {
        draft
            .insert(RelationId(0), &[Value::U64(2)])
            .expect("insert");
    });
    match history.submit(&second, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("the second command decides: {other:?}"),
    }

    // The first command's evidence was NOT overwritten: it resolves to a
    // definite typed observation (its CAS was dropped, so NotRecordedAt at
    // the current tip — an observation, never proof of nonpublication, and
    // never a fabricated receipt).
    match history.resolve(first_ref, &work()).expect("resolve runs") {
        ResolveOutcome::NotRecordedAt { decision_at } => {
            assert!(decision_at.seq >= 1, "resolved against the current tip");
        }
        ResolveOutcome::Found(receipt) => {
            // If the machine internally retried and published, the outcome
            // must be the exact retained decision.
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("the earlier attempt resolves to typed evidence: {other:?}"),
    }
}

/// REP-004/ENG-004 baseline (audit/10 and audit/20: the database entity
/// allocator granted equal counter ranges / reissued escaped IDs): the
/// allocator and every `FreshRef` surface are DELETED. Successor property:
/// entity identity is application-owned Id128 bytes sealed INSIDE the
/// command; retries return the retained receipt and the persisted bytes are
/// exactly the application's, with no issuance authority anywhere
/// (PROTO-11, E-NO-RESERVE, G09).
#[test]
fn rep004_entity_bytes_are_application_owned_and_retry_stable() {
    let (db, history) = create_history("rep004", id128_schema());
    let owner = Id128::from_bytes([
        0x0f, 0x1e, 0x2d, 0x3c, 0x4b, 0x5a, 0x69, 0x78, 0x87, 0x96, 0xa5, 0xb4, 0xc3, 0xd2, 0xe1,
        0xf0,
    ]);
    let command = seal_with(&db, history.identity(), 0x21, |draft| {
        draft
            .insert(
                RelationId(0),
                &[Value::Id128(owner), Value::String("mine".into())],
            )
            .expect("insert");
    });
    let receipt = match history.submit(&command, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("decides: {other:?}"),
    };
    assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
    // The retry re-seals the identical bytes and returns the SAME receipt —
    // no range, counter or reservation participates.
    let retry = match history.submit(&command, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("retry resolves: {other:?}"),
    };
    assert_eq!(retry.decision_at, receipt.decision_at);
    let rows = rows_in(&db, RelationId(0));
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].first(),
        Some(&Value::Id128(owner)),
        "the persisted identity is byte-for-byte the application's"
    );
    // Duplicate identity is ordinary schema law, not an issuance conflict: a
    // second command reusing the same Id128 in a keyless relation admits.
    let duplicate = seal_with(&db, history.identity(), 0x22, |draft| {
        draft
            .insert(
                RelationId(0),
                &[Value::Id128(owner), Value::String("again".into())],
            )
            .expect("insert");
    });
    match history.submit(&duplicate, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("duplicate id follows ordinary schema law: {other:?}"),
    }
}

/// ENG-007 baseline (audit/20: infrastructure failure was flattened into a
/// semantic rejection by the fresh-ID burn machine): the burn machine is
/// DELETED. Successor property: resource exhaustion is a typed operational
/// refusal (`NotSubmitted(Work)`) that never masquerades as a durable
/// `InvariantRejected`, and the identical command decides once real budget
/// arrives (G03/G06/G09 boundary).
#[test]
fn eng007_exhaustion_is_operational_not_a_semantic_rejection() {
    let (db, history) = create_history("eng007", two_relation_schema());
    let command = seal_with(&db, history.identity(), 0x31, |draft| {
        draft
            .insert(RelationId(0), &[Value::U64(9)])
            .expect("insert");
    });
    let starved = ExecutionPolicy {
        input_bytes: 1,
        working_bytes: 1,
        scratch_bytes: 1,
        result_bytes: 1,
        rows: 1,
        work_units: 1,
        timeout: std::time::Duration::from_secs(60),
    }
    .start()
    .expect("starved budget starts");
    match history.submit(&command, &starved) {
        SubmitOutcome::NotSubmitted { error, .. } => {
            assert!(
                matches!(
                    error,
                    LogError::Work(_) | LogError::Core(_) | LogError::Storage(_)
                ),
                "exhaustion is typed and operational: {error:?}"
            );
        }
        SubmitOutcome::Decided { receipt, .. } => {
            panic!("a starved budget cannot decide: {:?}", receipt.outcome)
        }
        SubmitOutcome::OutcomeUnknown { error, .. } => {
            panic!("local submission is definite: {error:?}")
        }
    }
    // No fact and no receipt appeared; the same command then decides.
    assert!(rows_in(&db, RelationId(0)).is_empty());
    match history.submit(&command, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("the refusal was recoverable: {other:?}"),
    }
}

/// REP-001/REP-007 baseline (audit/10: an old writer recreated a retired
/// slot; scratch recovery deleted a reachable checkpoint): braid slots and
/// scratch deletion authority are DELETED. Successor property: a stale
/// writer's pre-staged decision object never enters live history — it is an
/// unreferenced orphan the epoch collector removes, while every protected
/// dependency survives (GC-01/GC-02 shape over the composed hosted layout).
#[test]
fn rep001_a_stale_writers_staging_is_an_orphan_never_history() {
    use bumbledb_log::checkpointer::read_live_head;
    use bumbledb_log::gc::{GcPolicy, run_collection};
    use bumbledb_log::store::{
        ObjectKind, ReceiveLimits, TransportContext, get_verified, put_verified,
    };

    let store = MemStore::new();
    let mut mirror = lane_support::Mirror::create("rep001", &store, "t");
    let identity = mirror.identity;
    mirror.submit(&lane_support::insert_user(mirror.db(), identity, 0x01, 10));
    mirror.submit(&lane_support::insert_user(mirror.db(), identity, 0x02, 20));

    // The "old writer": bytes staged under the current epoch, never
    // referenced by any published head (its CAS lost).
    let old_epoch = mirror.head().object_epoch;
    let orphan = put_verified(
        &store,
        "t",
        old_epoch,
        ObjectKind::Decision,
        b"stale-decision",
    )
    .expect("stale staging");

    let policy = GcPolicy {
        head_cap: lane_support::HEAD_CAP,
        ..GcPolicy::DEFAULT
    };
    let report = run_collection(&store, "t", op(0x77), LIMITS, &policy, &work())
        .expect("collection converges");
    assert!(report.finished);
    assert!(
        get_verified(
            &store,
            "t",
            &orphan,
            TransportContext::new(&work(), ReceiveLimits::exact(orphan.length)),
        )
        .is_err(),
        "the stale writer's staging is collected, never adopted"
    );
    // Every protected decision the recovery root names is still verifiable.
    let (head, _) = read_live_head(&store, "t", lane_support::HEAD_CAP).expect("head reads");
    let recovery = head.recovery.expect("recovery root");
    assert_eq!(recovery.tip.seq, 2, "the retained history is intact");
}
