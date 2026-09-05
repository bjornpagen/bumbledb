//! Retained receipt rows: the log system table materialized transactionally
//! beside application facts.
//!
//! A row is keyed by `CommandId` inside the core's opaque host-record space
//! and stores the bounded receipt envelope (`command` family, receipt kind).
//! Rows are written in the same LMDB transaction as their facts and head
//! attachment, retained across checkpoints, filtered only when the retirement
//! frontier advances atomically, and looked up before any new admission.

use super::command::{
    FrameError, Limits, ReceiptMetadata, UnverifiedOutcome, UnverifiedReceiptEnvelope,
    decode_receipt, encode_receipt,
};
use super::{
    ChangeSummary, CommandId, CommandRef, CommandResult, RejectionEvidence, TerminalOutcome,
    TerminalReceipt,
};

/// Host-record key prefix for receipt rows. Keys order by epoch then request
/// bytes, so one retired epoch is one contiguous key range.
pub const RECEIPT_KEY_PREFIX: u8 = b'r';
pub const RECEIPT_KEY_LEN: usize = 25;

#[must_use]
pub fn receipt_key(id: CommandId) -> [u8; RECEIPT_KEY_LEN] {
    let mut key = [0; RECEIPT_KEY_LEN];
    key[0] = RECEIPT_KEY_PREFIX;
    key[1..9].copy_from_slice(&id.receipt_epoch.get().to_be_bytes());
    key[9..].copy_from_slice(id.request_id.as_core().as_bytes());
    key
}

/// Errors converting a stored row into an owned receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptRowError {
    Frame(FrameError),
    /// The stored row's command identity disagrees with its key or scope.
    ForeignRow,
}

impl From<FrameError> for ReceiptRowError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

/// Encode one durable receipt row from an owned terminal receipt.
/// # Errors
/// Refuses oversized frames and invalid receipt grammar.
pub fn encode_receipt_row(
    receipt: &TerminalReceipt,
    limits: Limits,
) -> Result<Vec<u8>, FrameError> {
    let outcome = match &receipt.outcome {
        TerminalOutcome::Committed { changed, result } => UnverifiedOutcome::Committed {
            changed: *changed,
            result: result.as_bytes(),
        },
        TerminalOutcome::NoChange { result } => UnverifiedOutcome::NoChange {
            result: result.as_bytes(),
        },
        TerminalOutcome::PreconditionFailed { expected, observed } => {
            UnverifiedOutcome::PreconditionFailed {
                expected: *expected,
                observed: *observed,
            }
        }
        TerminalOutcome::InvariantRejected { evidence } => UnverifiedOutcome::InvariantRejected {
            core_evidence: evidence.as_bytes(),
        },
    };
    encode_receipt(
        UnverifiedReceiptEnvelope {
            metadata: ReceiptMetadata {
                command: receipt.command,
                decision_at: receipt.decision_at,
                state_at: receipt.state_at,
            },
            outcome,
        },
        limits,
    )
}

/// Decode one stored row into an owned receipt, checking that the row's
/// recorded command matches the identity scope and the key it was fetched
/// under. A wrong-scope row is a foreign/corrupt row, never a receipt.
/// # Errors
/// Refuses malformed rows and identity/key mismatches.
pub fn decode_receipt_row(
    expected: CommandRef,
    bytes: &[u8],
    limits: Limits,
) -> Result<TerminalReceipt, ReceiptRowError> {
    let envelope = decode_receipt(bytes, limits)?;
    let metadata = envelope.metadata;
    if metadata.command.identity != expected.identity || metadata.command.id != expected.id {
        return Err(ReceiptRowError::ForeignRow);
    }
    Ok(owned_receipt(envelope))
}

/// Decode a row when only the storage key is known (retention walks and
/// inspection), still verifying the key binding.
/// # Errors
/// Refuses malformed rows and key mismatches.
pub fn decode_receipt_row_at(
    key: CommandId,
    bytes: &[u8],
    limits: Limits,
) -> Result<TerminalReceipt, ReceiptRowError> {
    let envelope = decode_receipt(bytes, limits)?;
    if envelope.metadata.command.id != key {
        return Err(ReceiptRowError::ForeignRow);
    }
    Ok(owned_receipt(envelope))
}

fn owned_receipt(envelope: UnverifiedReceiptEnvelope<'_>) -> TerminalReceipt {
    let metadata = envelope.metadata;
    let outcome = match envelope.outcome {
        UnverifiedOutcome::Committed { changed, result } => TerminalOutcome::Committed {
            changed,
            result: CommandResult::from_canonical_bytes(Box::from(result)),
        },
        UnverifiedOutcome::NoChange { result } => TerminalOutcome::NoChange {
            result: CommandResult::from_canonical_bytes(Box::from(result)),
        },
        UnverifiedOutcome::PreconditionFailed { expected, observed } => {
            TerminalOutcome::PreconditionFailed { expected, observed }
        }
        UnverifiedOutcome::InvariantRejected { core_evidence } => {
            TerminalOutcome::InvariantRejected {
                // Framing already refused empty evidence.
                evidence: RejectionEvidence::from_canonical_bytes(Box::from(core_evidence))
                    .expect("framed evidence is nonempty"),
            }
        }
    };
    TerminalReceipt {
        command: metadata.command,
        decision_at: metadata.decision_at,
        state_at: metadata.state_at,
        outcome,
    }
}

/// A change summary from the core's judged application counts. Zero/zero is
/// `NoChange`; anything else is a committed summary.
#[must_use]
pub fn summarize(added: u64, removed: u64) -> Option<ChangeSummary> {
    ChangeSummary::new(added, removed)
}

/// Whether a retirement frontier covers this key: `epoch <= retired_through`.
/// Retirement deletes exactly these rows in the same authority transaction
/// that advances the frontier — never a later best-effort sweep.
#[must_use]
pub fn retired(id: CommandId, retired_through: u64) -> bool {
    id.receipt_epoch.get() <= retired_through
}

#[cfg(test)]
mod tests {
    use bumbledb::Id128;

    use super::super::{
        DatabaseId, DatabaseIdentity, DecisionDigest, DecisionStamp, IncarnationId, ReceiptEpoch,
        RequestId, SchemaId, StateStamp,
    };
    use super::*;

    const LIMITS: Limits = Limits {
        envelope_bytes: 4096,
        change_bytes: 256,
        evidence_bytes: 256,
        result_bytes: 64,
    };

    fn identity() -> DatabaseIdentity {
        DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([2; 16])),
            schema_id: SchemaId([3; 32]),
        }
    }

    fn command(epoch: u64, request: u8) -> CommandRef {
        CommandRef {
            identity: identity(),
            id: CommandId {
                receipt_epoch: ReceiptEpoch::new(epoch).unwrap(),
                request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
            },
            digest: super::super::CommandDigest::from_bytes([9; 32]),
        }
    }

    fn receipt(outcome: TerminalOutcome) -> TerminalReceipt {
        let facts_changed = matches!(outcome, TerminalOutcome::Committed { .. });
        TerminalReceipt {
            command: command(1, 7),
            decision_at: DecisionStamp {
                seq: 4,
                hash: DecisionDigest::from_bytes([5; 32]),
            },
            state_at: StateStamp {
                incarnation: identity().incarnation_id,
                data_revision: if facts_changed { 2 } else { 1 },
            },
            outcome,
        }
    }

    #[test]
    fn keys_order_by_epoch_then_request_and_bind_the_row() {
        let low = receipt_key(command(1, 7).id);
        let high_epoch = receipt_key(command(2, 0).id);
        let high_request = receipt_key(command(1, 8).id);
        assert!(low < high_epoch);
        assert!(low < high_request);
        assert!(high_request < high_epoch);
        assert!(retired(command(1, 7).id, 1));
        assert!(!retired(command(2, 0).id, 1));
    }

    #[test]
    fn all_four_outcomes_roundtrip_as_owned_rows() {
        let outcomes = [
            TerminalOutcome::Committed {
                changed: ChangeSummary::new(2, 1).unwrap(),
                result: CommandResult::empty(),
            },
            TerminalOutcome::NoChange {
                result: CommandResult::from_canonical_bytes(Box::from([1, 2, 3])),
            },
            TerminalOutcome::PreconditionFailed {
                expected: StateStamp {
                    incarnation: identity().incarnation_id,
                    data_revision: 0,
                },
                observed: StateStamp {
                    incarnation: identity().incarnation_id,
                    data_revision: 1,
                },
            },
            TerminalOutcome::InvariantRejected {
                evidence: RejectionEvidence::from_canonical_bytes(Box::from(*b"core-evidence"))
                    .unwrap(),
            },
        ];
        for outcome in outcomes {
            let row = receipt(outcome);
            let bytes = encode_receipt_row(&row, LIMITS).unwrap();
            assert_eq!(
                decode_receipt_row(row.command, &bytes, LIMITS).unwrap(),
                row
            );
            assert_eq!(
                decode_receipt_row_at(row.command.id, &bytes, LIMITS).unwrap(),
                row
            );
        }
    }

    #[test]
    fn foreign_scope_and_wrong_key_rows_never_become_receipts() {
        let row = receipt(TerminalOutcome::NoChange {
            result: CommandResult::empty(),
        });
        let bytes = encode_receipt_row(&row, LIMITS).unwrap();
        let mut foreign = row.command;
        foreign.identity.incarnation_id = IncarnationId::from_core(Id128::from_bytes([8; 16]));
        assert_eq!(
            decode_receipt_row(foreign, &bytes, LIMITS),
            Err(ReceiptRowError::ForeignRow)
        );
        assert_eq!(
            decode_receipt_row_at(command(1, 8).id, &bytes, LIMITS),
            Err(ReceiptRowError::ForeignRow)
        );
        // Digest conflicts are the admission layer's refusal, not silent
        // acceptance here: the row still decodes under its own recorded digest.
        let mut conflicting = row.command;
        conflicting.digest = super::super::CommandDigest::from_bytes([1; 32]);
        let decoded = decode_receipt_row(conflicting, &bytes, LIMITS).unwrap();
        assert_ne!(decoded.command.digest, conflicting.digest);
    }
}
