//! `LocalHistory` over a real core LMDB store: atomic facts/receipt/head
//! attachment, durable no-op/rejection/precondition receipts, retry
//! deduplication and the exact-state (ABA) witness. Maps to PROTO-02/07/08/09/
//! 10/17 and CONC-02/03. Verification: `NotRun` (F1 authors, does not execute).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bumbledb::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
use bumbledb::{ChangeSet, Db, ExecutionPolicy, Id128, RelationId, Value, WorkContext};

use bumbledb_log::history::command::{Command, CommandMetadata, Limits};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity, IncarnationId, OperationId,
    ReceiptEpoch, RequestId, TerminalOutcome,
};
use bumbledb_log::writer::{LocalHealth, LocalHistory, ResolveOutcome, SubmitOutcome};

const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 10_000,
    result_bytes: 1_000,
};

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-log-wl-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// One relation `User(id)`. Rejection shapes (key/capacity) are covered by the
/// independent model tests; this integration lane exercises the durable
/// commit/no-change/precondition receipts and retry deduplication.
fn theory() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "User".into(),
            fields: vec![FieldDescriptor {
                name: "id".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![],
    }
}

fn fresh_db(tag: &str) -> Arc<Db<SchemaDescriptor>> {
    let dir = temp_dir(tag).join("db");
    Arc::new(
        Db::create(&dir, theory())
            .expect("create store")
            .expect("empty store admits"),
    )
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 1_000_000,
        result_bytes: 1_000_000,
        rows: 100_000,
        work_units: 10_000_000,
        timeout: Duration::from_secs(60),
    }
}

fn work() -> WorkContext {
    policy().start().unwrap()
}

fn identity(db: &Db<SchemaDescriptor>) -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([0xa1; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([0xb2; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(db.schema()),
    }
}

fn command(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    condition: Condition,
    build: impl FnOnce(&mut bumbledb::ChangeSetBuilder<'_>),
) -> Command {
    let mut draft = ChangeSet::builder(db.schema(), work());
    build(&mut draft);
    let changes = draft.finish().unwrap();
    let metadata = CommandMetadata {
        identity,
        id: CommandId {
            receipt_epoch: ReceiptEpoch::INITIAL,
            request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
        },
        condition,
    };
    Command::seal(metadata, changes, CommandResult::empty(), LIMITS, &work()).unwrap()
}

fn open(
    tag: &str,
) -> (
    Arc<Db<SchemaDescriptor>>,
    LocalHistory<SchemaDescriptor>,
    DatabaseIdentity,
) {
    let db = fresh_db(tag);
    let identity = identity(&db);
    let history = LocalHistory::create(
        Arc::clone(&db),
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes([0xc3; 16])),
        LIMITS,
        &work(),
    )
    .unwrap();
    (db, history, identity)
}

#[test]
fn commit_no_change_and_retry_are_durable_stable_receipts() {
    let (_db, history, identity) = open("outcomes");
    let insert = command(
        history.db(),
        identity,
        1,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(7)]).unwrap();
        },
    );
    // First submit: a committed decision.
    let committed = match history.submit(&insert, &work()) {
        SubmitOutcome::Decided {
            receipt,
            local_health,
        } => {
            assert!(matches!(local_health, LocalHealth::Ready { .. }));
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
            receipt
        }
        other => panic!("expected committed, got {other:?}"),
    };

    // Retry the identical command: same stable receipt, not a second commit.
    match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert_eq!(
                receipt, committed,
                "retry returns the same terminal receipt"
            );
        }
        other => panic!("expected decided retry, got {other:?}"),
    }

    // A different command inserting the same fact nets no change (idempotent).
    let again = command(
        history.db(),
        identity,
        2,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(7)]).unwrap();
        },
    );
    match history.submit(&again, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::NoChange { .. }));
        }
        other => panic!("expected no-change, got {other:?}"),
    }

    // A distinct new fact is a fresh committed decision.
    let more = command(
        history.db(),
        identity,
        3,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(9)]).unwrap();
        },
    );
    match history.submit(&more, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert!(matches!(receipt.outcome, TerminalOutcome::Committed { .. }));
        }
        other => panic!("expected committed, got {other:?}"),
    }
}

#[test]
fn exact_state_witness_detects_intervening_change_and_aba() {
    let (_db, history, identity) = open("exact");
    // Capture the initial state stamp.
    let initial = history.authority().unwrap().position().unwrap().state;
    // A committed change moves the state stamp.
    let insert = command(
        history.db(),
        identity,
        1,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(1)]).unwrap();
        },
    );
    let after = match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt.state_at,
        other => panic!("{other:?}"),
    };
    assert_ne!(
        initial, after,
        "a committed change advances the state stamp"
    );

    // An exact-state command against the STALE initial stamp fails precondition.
    let stale = command(
        history.db(),
        identity,
        2,
        Condition::ExactState(initial),
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(2)]).unwrap();
        },
    );
    match history.submit(&stale, &work()) {
        SubmitOutcome::Decided { receipt, .. } => match receipt.outcome {
            TerminalOutcome::PreconditionFailed { expected, observed } => {
                assert_eq!(expected, initial);
                assert_ne!(observed, initial);
            }
            other => panic!("expected precondition-failed, got {other:?}"),
        },
        other => panic!("{other:?}"),
    }

    // An exact-state command against the CURRENT stamp commits.
    let fresh = command(
        history.db(),
        identity,
        3,
        Condition::ExactState(after),
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(3)]).unwrap();
        },
    );
    assert!(matches!(
        history.submit(&fresh, &work()),
        SubmitOutcome::Decided {
            receipt: bumbledb_log::history::TerminalReceipt {
                outcome: TerminalOutcome::Committed { .. },
                ..
            },
            ..
        }
    ));
}

#[test]
fn resolve_returns_the_retained_receipt_and_not_recorded_for_unknown() {
    let (_db, history, identity) = open("resolve");
    let insert = command(
        history.db(),
        identity,
        1,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(5)]).unwrap();
        },
    );
    let receipt = match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("{other:?}"),
    };
    match history.resolve(receipt.command, &work()).unwrap() {
        ResolveOutcome::Found(found) => assert_eq!(found, receipt),
        other => panic!("expected found, got {other:?}"),
    }
    // An unseen command in the open epoch resolves as not-recorded, never a
    // fabricated failure.
    let unseen = CommandId {
        receipt_epoch: ReceiptEpoch::INITIAL,
        request_id: RequestId::from_core(Id128::from_bytes([0x99; 16])),
    };
    let unseen_ref = bumbledb_log::history::CommandRef {
        identity,
        id: unseen,
        digest: bumbledb_log::history::CommandDigest::from_bytes([0; 32]),
    };
    assert!(matches!(
        history.resolve(unseen_ref, &work()).unwrap(),
        ResolveOutcome::NotRecordedAt { .. }
    ));
}

#[test]
fn same_command_id_with_different_bytes_conflicts_after_publication() {
    let (_db, history, identity) = open("conflict");
    let first = command(
        history.db(),
        identity,
        1,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(1)]).unwrap();
        },
    );
    assert!(matches!(
        history.submit(&first, &work()),
        SubmitOutcome::Decided { .. }
    ));
    // Same command ID, different application bytes → identity conflict, never a
    // second execution.
    let forged = command(
        history.db(),
        identity,
        1,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(2)]).unwrap();
        },
    );
    match history.submit(&forged, &work()) {
        SubmitOutcome::NotSubmitted { error, .. } => {
            assert_eq!(
                error,
                bumbledb_log::writer::LogError::CommandIdentityConflict
            );
        }
        other => panic!("expected identity conflict, got {other:?}"),
    }
}

/// The independent history model's Deleted-refuses-before-lookup rule as a
/// PRODUCTION boundary (requested by P11 in implementation/packets/P11.md): a
/// tombstoned authority has no receipt table. The retained receipt row is
/// still physically present in LMDB after the one-transaction tombstone, but
/// no production surface serves it — deletion precedes receipt lookup, unlike
/// Frozen (where retained lookup still works).
#[test]
fn deleted_authority_has_no_receipt_table() {
    let (db, history, identity) = open("deleted");
    let insert = command(
        history.db(),
        identity,
        1,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(11)]).unwrap();
        },
    );
    let receipt = match history.submit(&insert, &work()) {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("{other:?}"),
    };
    // Live: the retained receipt resolves.
    assert!(matches!(
        history.resolve(receipt.command, &work()).unwrap(),
        ResolveOutcome::Found(_)
    ));

    // Production tombstone: the P05 admin transition over the same store.
    match bumbledb_log::admin::tombstone_local(
        &db,
        OperationId::from_core(Id128::from_bytes([0xd4; 16])),
        bumbledb_log::history::authority::DeletedReason::Erasure,
        LIMITS.envelope_bytes,
        &work(),
    )
    .unwrap()
    {
        bumbledb_log::history::authority::DeleteOutcome::Deleted(_) => {}
        other @ bumbledb_log::history::authority::DeleteOutcome::AlreadyDeleted { .. } => {
            panic!("expected a fresh tombstone, got {other:?}")
        }
    }

    // Deletion precedes receipt lookup: the very receipt that resolved above
    // is now unreachable — a typed refusal, never the retained row.
    assert!(matches!(
        history.resolve(receipt.command, &work()),
        Err(bumbledb_log::writer::LogError::DatabaseDeleted)
    ));
    // Retrying the already-decided command is likewise refused without
    // consulting the receipt table: no dedup answer from a tombstone.
    assert!(matches!(
        history.submit(&insert, &work()),
        SubmitOutcome::NotSubmitted {
            error: bumbledb_log::writer::LogError::DatabaseDeleted,
            ..
        }
    ));
    // An unseen command refuses identically: absence and presence of a
    // retained row are indistinguishable through a deleted authority.
    let unseen = command(
        history.db(),
        identity,
        9,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(12)]).unwrap();
        },
    );
    assert!(matches!(
        history.resolve(unseen.command_ref(), &work()),
        Err(bumbledb_log::writer::LogError::DatabaseDeleted)
    ));
}

#[test]
fn foreign_identity_and_uninitialized_open_refuse() {
    let (_db, history, identity) = open("foreign");
    let mut foreign = identity;
    foreign.incarnation_id = IncarnationId::from_core(Id128::from_bytes([0xee; 16]));
    let command = command(
        history.db(),
        foreign,
        1,
        Condition::Unconditional,
        |draft| {
            draft.insert(RelationId(0), &[Value::U64(1)]).unwrap();
        },
    );
    assert!(matches!(
        history.submit(&command, &work()),
        SubmitOutcome::NotSubmitted {
            error: bumbledb_log::writer::LogError::Identity,
            ..
        }
    ));

    // Opening a fresh, uninitialized database never initializes it.
    let empty = fresh_db("uninit");
    assert!(matches!(
        LocalHistory::open(empty, LIMITS),
        Err(bumbledb_log::writer::LogError::NotInitialized)
    ));
}
