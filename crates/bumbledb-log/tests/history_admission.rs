//! Actual production guard/counter tests. No claim of atomic LMDB publication.

use bumbledb::Id128;
use bumbledb_log::history::admission::{AdmissionView, Refusal, Resolution, Submission};
use bumbledb_log::history::{
    AccessMode, CommandDigest, CommandId, CommandRef, CommandResult, Condition, CounterExhausted,
    DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, HeadRevision, HistoryPosition,
    IncarnationId, PolicyError, ReceiptEpoch, ReceiptPolicy, RequestId, SchemaId, StateStamp,
    TerminalOutcome, TerminalReceipt,
};

fn view() -> AdmissionView {
    let incarnation = IncarnationId::from_core(Id128::from_bytes([2; 16]));
    AdmissionView {
        identity: DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: incarnation,
            schema_id: SchemaId([3; 32]),
        },
        access: AccessMode::Active,
        receipts: ReceiptPolicy::INITIAL,
        decision: DecisionStamp {
            seq: 3,
            hash: DecisionDigest::from_bytes([4; 32]),
        },
        state: StateStamp {
            incarnation,
            data_revision: 2,
        },
    }
}

fn command() -> CommandRef {
    CommandRef {
        identity: view().identity,
        id: CommandId {
            receipt_epoch: ReceiptEpoch::INITIAL,
            request_id: RequestId::from_core(Id128::from_bytes([5; 16])),
        },
        digest: CommandDigest::from_bytes([6; 32]),
    }
}

fn receipt() -> TerminalReceipt {
    TerminalReceipt {
        command: command(),
        decision_at: view().decision,
        state_at: view().state,
        outcome: TerminalOutcome::NoChange {
            result: CommandResult::empty(),
        },
    }
}

#[test]
fn retained_receipt_precedes_freeze_closed_epoch_and_stale_witness() {
    let receipt = receipt();
    let mut captured = view();
    captured.access = AccessMode::Frozen;
    captured.receipts = captured
        .receipts
        .rotate(ReceiptEpoch::new(2).unwrap())
        .unwrap();
    let stale = Condition::ExactState(StateStamp {
        data_revision: 0,
        ..captured.state
    });
    assert_eq!(
        captured.resolve(command(), Some(&receipt)),
        Ok(Resolution::Found(&receipt))
    );
    assert_eq!(
        captured.submit(command(), stale, Some(&receipt)),
        Ok(Submission::AlreadyDecided(&receipt))
    );
    assert_eq!(
        captured.submit(command(), stale, None),
        Err(Refusal::CommandEpochClosed)
    );
    let mut conflict = command();
    conflict.digest = CommandDigest::from_bytes([8; 32]);
    assert_eq!(
        captured.submit(conflict, stale, Some(&receipt)),
        Err(Refusal::CommandIdentityConflict)
    );
    captured.receipts = captured.receipts.retire(1).unwrap();
    assert_eq!(
        captured.resolve(command(), Some(&receipt)),
        Err(Refusal::ReceiptExpiredUnknown)
    );
    captured.access = AccessMode::Deleted;
    assert_eq!(
        captured.resolve(command(), Some(&receipt)),
        Err(Refusal::DatabaseDeleted)
    );
    conflict.identity.database_id = DatabaseId::from_core(Id128::from_bytes([9; 16]));
    assert_eq!(
        captured.resolve(conflict, Some(&receipt)),
        Err(Refusal::IdentityMismatch)
    );
}

#[test]
fn open_absence_is_frontier_scoped_and_exact_state_precedes_semantic_evaluation() {
    let mut captured = view();
    assert_eq!(
        captured.resolve(command(), None),
        Ok(Resolution::NotRecordedAt {
            decision_at: captured.decision
        })
    );
    assert_eq!(
        captured.submit(command(), Condition::Unconditional, None),
        Ok(Submission::Evaluate)
    );
    assert_eq!(
        captured.submit(command(), Condition::ExactState(captured.state), None),
        Ok(Submission::Evaluate)
    );
    let expected = StateStamp {
        data_revision: 1,
        ..captured.state
    };
    assert_eq!(
        captured.submit(command(), Condition::ExactState(expected), None),
        Ok(Submission::PreconditionFailed {
            expected,
            observed: captured.state
        })
    );
    captured.access = AccessMode::Frozen;
    assert!(matches!(
        captured.resolve(command(), None),
        Ok(Resolution::NotRecordedAt { .. })
    ));
    assert_eq!(
        captured.submit(command(), Condition::ExactState(expected), None),
        Err(Refusal::DatabaseFrozen)
    );
    let mut future = command();
    future.id.receipt_epoch = ReceiptEpoch::new(2).unwrap();
    assert_eq!(
        captured.resolve(future, None),
        Err(Refusal::CommandEpochNotOpen {
            open: ReceiptEpoch::INITIAL,
            requested: future.id.receipt_epoch
        })
    );
}

#[test]
fn wrong_schema_lineage_and_corrupted_receipt_do_not_alias() {
    for identity in [
        DatabaseIdentity {
            schema_id: SchemaId([0; 32]),
            ..view().identity
        },
        DatabaseIdentity {
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([0; 16])),
            ..view().identity
        },
    ] {
        assert_eq!(
            view().resolve(
                CommandRef {
                    identity,
                    ..command()
                },
                None
            ),
            Err(Refusal::IdentityMismatch)
        );
    }
    let mut wrong = receipt();
    wrong.command.id.request_id = RequestId::from_core(Id128::from_bytes([1; 16]));
    assert_eq!(
        view().resolve(command(), Some(&wrong)),
        Err(Refusal::InvalidRetainedReceipt)
    );
    wrong = receipt();
    wrong.decision_at.seq = 4;
    assert_eq!(
        view().resolve(command(), Some(&wrong)),
        Err(Refusal::InvalidRetainedReceipt)
    );
    wrong = receipt();
    wrong.state_at.data_revision = 3;
    assert_eq!(
        view().resolve(command(), Some(&wrong)),
        Err(Refusal::InvalidRetainedReceipt)
    );
    wrong = receipt();
    wrong.decision_at.hash = DecisionDigest::from_bytes([0; 32]);
    assert_eq!(
        view().resolve(command(), Some(&wrong)),
        Err(Refusal::InvalidRetainedReceipt)
    );
    let foreign = StateStamp {
        incarnation: IncarnationId::from_core(Id128::from_bytes([8; 16])),
        ..view().state
    };
    assert_eq!(
        view().submit(command(), Condition::ExactState(foreign), None),
        Err(Refusal::StateIdentityMismatch)
    );
}

#[test]
fn policy_is_monotone_and_every_coordinate_has_its_own_exhaustion() {
    assert_eq!(ReceiptEpoch::new(0), None);
    assert_eq!(
        ReceiptPolicy::new(ReceiptEpoch::INITIAL, 1),
        Err(PolicyError::InvalidRetirement)
    );
    let policy = ReceiptPolicy::INITIAL
        .rotate(ReceiptEpoch::new(3).unwrap())
        .unwrap()
        .retire(2)
        .unwrap();
    assert_eq!(policy.retire(1), Err(PolicyError::RetirementMovedBackward));
    assert_eq!(
        policy.rotate(ReceiptEpoch::new(3).unwrap()),
        Err(PolicyError::EpochDidNotAdvance)
    );
    assert_eq!(policy.retire(3), Err(PolicyError::InvalidRetirement));
    let position = HistoryPosition {
        head: HeadRevision(4),
        decision: view().decision,
        state: view().state,
    };
    let hash = DecisionDigest::from_bytes([10; 32]);
    let changed = position.decided(hash, true).unwrap();
    assert_eq!(
        (
            changed.head.0,
            changed.decision.seq,
            changed.state.data_revision
        ),
        (5, 4, 3)
    );
    let no_op = changed.decided(hash, false).unwrap();
    assert_eq!(
        (no_op.head.0, no_op.decision.seq, no_op.state.data_revision),
        (6, 5, 3)
    );
    let maintained = no_op.maintained().unwrap();
    assert_eq!(
        (
            maintained.head.0,
            maintained.decision.seq,
            maintained.state.data_revision
        ),
        (7, 5, 3)
    );
    assert_eq!(
        HistoryPosition {
            head: HeadRevision(u64::MAX),
            ..position
        }
        .decided(hash, false),
        Err(CounterExhausted::HeadRevision)
    );
    assert_eq!(
        HistoryPosition {
            decision: DecisionStamp {
                seq: u64::MAX,
                ..position.decision
            },
            ..position
        }
        .decided(hash, false),
        Err(CounterExhausted::DecisionSequence)
    );
    let full_state = HistoryPosition {
        state: StateStamp {
            data_revision: u64::MAX,
            ..position.state
        },
        ..position
    };
    assert_eq!(
        full_state.decided(hash, true),
        Err(CounterExhausted::DataRevision)
    );
    assert!(full_state.decided(hash, false).is_ok());
    assert_eq!(
        position.head.0, 4,
        "failure calculations never mutate captured state"
    );
}
