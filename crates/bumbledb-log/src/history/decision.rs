//! Immutable decision and genesis record framing.
//!
//! One decision object binds identity, sequence, exact parent, the concrete
//! canonical command envelope and the terminal outcome evidence. The decision
//! never contains its own digest; the sealed receipt and head attachment carry
//! that digest afterwards. Genesis is an explicit admitted initial snapshot
//! whose sequence-zero sentinel hashes a versioned record binding identity,
//! schema, canonical initial digests and provenance — never one universal zero
//! hash. Preimages are acyclic: a genesis record excludes its own stamp and
//! the manifest hash that will carry it.
//!
//! Framing establishes grammar. It does not admit facts, verify recorded
//! judgment against a state, or grant authority. Physical bytes remain
//! provisional until the F3 format freeze (C12).

use super::command::{
    self, FrameError, Limits, UnverifiedOutcome, outcome_len, put_outcome, read_outcome,
};
use super::frame::{
    Reader, begin_frame, check_limit, frame_len, put_identity, put_stamp, put_state, put_u64,
};
use super::{
    CommandRef, DatabaseIdentity, DecisionDigest, DecisionStamp, IncarnationId, StateStamp,
};

pub const FAMILY: &[u8] = b"bumbledb.decision.v1\0";
pub const LAYOUT: u16 = 1;
const DECISION: u8 = 1;
const GENESIS: u8 = 2;
const DECISION_DIGEST_DOMAIN: &str = "bumbledb.decision.v1/decision-digest";
const GENESIS_DIGEST_DOMAIN: &str = "bumbledb.decision.v1/genesis-digest";

/// The fields a writer supplies when framing one decision. The canonical
/// command bytes are the exact sealed command envelope; the outcome is the
/// judged terminal evidence.
#[derive(Debug, Clone, Copy)]
pub struct DecisionParts<'a> {
    pub identity: DatabaseIdentity,
    pub seq: u64,
    pub parent: DecisionStamp,
    pub before_state: StateStamp,
    pub after_state: StateStamp,
    pub canonical_command: &'a [u8],
    pub outcome: UnverifiedOutcome<'a>,
}

/// Successfully framed and internally consistent, **not** verified against
/// any state, chain position, or storage digest.
#[derive(Debug, PartialEq, Eq)]
pub struct UnverifiedDecisionEnvelope<'a> {
    pub identity: DatabaseIdentity,
    pub seq: u64,
    pub parent: DecisionStamp,
    pub before_state: StateStamp,
    pub after_state: StateStamp,
    /// The embedded exact command envelope bytes (command family framing).
    pub canonical_command: &'a [u8],
    /// The embedded command's identity/digest, re-derived from those bytes.
    pub command: CommandRef,
    pub outcome: UnverifiedOutcome<'a>,
    digest: DecisionDigest,
}

impl UnverifiedDecisionEnvelope<'_> {
    /// The domain-separated digest of exactly these bytes: the value a stamp,
    /// receipt or head attachment carries for this decision.
    #[must_use]
    pub const fn digest(&self) -> DecisionDigest {
        self.digest
    }

    #[must_use]
    pub const fn stamp(&self) -> DecisionStamp {
        DecisionStamp {
            seq: self.seq,
            hash: self.digest,
        }
    }
}

/// # Errors
/// Refuses inconsistent stamps/outcomes, oversized frames and allocation.
pub fn encode_decision(parts: DecisionParts<'_>, limits: Limits) -> Result<Vec<u8>, FrameError> {
    validate_parts(&parts)?;
    check_limit(parts.canonical_command.len(), limits.envelope_bytes)?;
    let len = frame_len(
        FAMILY.len(),
        &[
            64,
            8,
            40,
            24,
            24,
            8,
            parts.canonical_command.len(),
            outcome_len(parts.outcome, limits)?,
        ],
    )?;
    let mut out = begin_frame(FAMILY, LAYOUT, DECISION, len, limits.envelope_bytes)?;
    put_identity(&mut out, parts.identity);
    put_u64(&mut out, parts.seq);
    put_stamp(&mut out, parts.parent);
    put_state(&mut out, parts.before_state);
    put_state(&mut out, parts.after_state);
    put_u64(&mut out, parts.canonical_command.len() as u64);
    out.extend_from_slice(parts.canonical_command);
    put_outcome(&mut out, parts.outcome)?;
    debug_assert_eq!(out.len(), len);
    Ok(out)
}

/// The digest carried by stamps/receipts for exactly these decision bytes.
#[must_use]
pub fn decision_digest(bytes: &[u8]) -> DecisionDigest {
    DecisionDigest::from_bytes(blake3::derive_key(DECISION_DIGEST_DOMAIN, bytes))
}

/// Decode one decision envelope, validating internal consistency including
/// the nested command envelope's identity and digest binding.
/// # Errors
/// Refuses malformed frames, nested command grammar errors and every
/// stamp/outcome inconsistency.
pub fn decode_decision(
    bytes: &[u8],
    limits: Limits,
) -> Result<UnverifiedDecisionEnvelope<'_>, FrameError> {
    let mut input = Reader::begin(bytes, FAMILY, LAYOUT, DECISION, limits.envelope_bytes)?;
    let identity = input.identity()?;
    let seq = input.u64()?;
    let parent = input.stamp()?;
    let before_state = input.state()?;
    let after_state = input.state()?;
    let canonical_command = input.span(limits.envelope_bytes)?;
    let outcome = read_outcome(&mut input, limits)?;
    input.end()?;
    let nested = command::decode_command(canonical_command, limits)?;
    let command = nested.command_ref();
    let envelope = UnverifiedDecisionEnvelope {
        identity,
        seq,
        parent,
        before_state,
        after_state,
        canonical_command,
        command,
        outcome,
        digest: decision_digest(bytes),
    };
    validate_envelope(&envelope)?;
    Ok(envelope)
}

fn validate_parts(parts: &DecisionParts<'_>) -> Result<(), FrameError> {
    validate_stamps(
        parts.identity,
        parts.seq,
        parts.parent,
        parts.before_state,
        parts.after_state,
        parts.outcome,
    )
}

fn validate_envelope(envelope: &UnverifiedDecisionEnvelope<'_>) -> Result<(), FrameError> {
    if envelope.command.identity != envelope.identity {
        return Err(FrameError::StateIdentityMismatch);
    }
    validate_stamps(
        envelope.identity,
        envelope.seq,
        envelope.parent,
        envelope.before_state,
        envelope.after_state,
        envelope.outcome,
    )
}

fn validate_stamps(
    identity: DatabaseIdentity,
    seq: u64,
    parent: DecisionStamp,
    before: StateStamp,
    after: StateStamp,
    outcome: UnverifiedOutcome<'_>,
) -> Result<(), FrameError> {
    if seq == 0 || parent.seq.checked_add(1) != Some(seq) {
        return Err(FrameError::InvalidSequence);
    }
    if before.incarnation != identity.incarnation_id || after.incarnation != identity.incarnation_id
    {
        return Err(FrameError::StateIdentityMismatch);
    }
    let facts_changed = matches!(outcome, UnverifiedOutcome::Committed { .. });
    let expected_after = if facts_changed {
        before
            .data_revision
            .checked_add(1)
            .ok_or(FrameError::LengthOverflow)?
    } else {
        before.data_revision
    };
    if after.data_revision != expected_after || before.data_revision > parent.seq {
        return Err(FrameError::InvalidTerminalStamp);
    }
    if let UnverifiedOutcome::PreconditionFailed { expected, observed } = outcome {
        if expected.incarnation != identity.incarnation_id {
            return Err(FrameError::StateIdentityMismatch);
        }
        if expected == observed || observed != before {
            return Err(FrameError::InvalidPreconditionEvidence);
        }
    }
    if let UnverifiedOutcome::InvariantRejected { core_evidence: [] } = outcome {
        return Err(FrameError::EmptyEvidence);
    }
    Ok(())
}

/// Verify one chain step: the envelope claims exactly this parent and the
/// next sequence. This authenticates continuity, not retention of every
/// historical parent object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    WrongParent {
        claimed: DecisionStamp,
        actual: DecisionStamp,
    },
    WrongSequence {
        claimed: u64,
        expected: u64,
    },
}

/// # Errors
/// Refuses a decision that does not extend exactly this parent.
pub fn verify_step(
    parent: DecisionStamp,
    envelope: &UnverifiedDecisionEnvelope<'_>,
) -> Result<(), ChainError> {
    if envelope.parent != parent {
        return Err(ChainError::WrongParent {
            claimed: envelope.parent,
            actual: parent,
        });
    }
    let expected = parent.seq.saturating_add(1);
    if envelope.seq != expected {
        return Err(ChainError::WrongSequence {
            claimed: envelope.seq,
            expected,
        });
    }
    Ok(())
}

/// Provenance of an admitted initial state. A provenance citation is
/// historical metadata, not a retention promise for an external origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenesisProvenance {
    Create,
    Restore {
        source_evidence: [u8; 32],
    },
    Migration {
        source_database: super::DatabaseId,
        source_incarnation: IncarnationId,
        plan_set_digest: [u8; 32],
    },
}

/// The versioned genesis preimage: identity/schema, canonical initial
/// digests and provenance. It excludes its own stamp and any manifest hash
/// that will later carry that stamp (acyclic by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisRecord {
    pub identity: DatabaseIdentity,
    pub initial_application_digest: [u8; 32],
    pub initial_system_digest: [u8; 32],
    pub provenance: GenesisProvenance,
}

/// # Errors
/// Refuses oversized frames and allocation failure.
pub fn encode_genesis(record: &GenesisRecord, cap: usize) -> Result<Vec<u8>, FrameError> {
    let provenance_len = match record.provenance {
        GenesisProvenance::Create => 1,
        GenesisProvenance::Restore { .. } => 33,
        GenesisProvenance::Migration { .. } => 65,
    };
    let len = frame_len(FAMILY.len(), &[64, 64, provenance_len])?;
    let mut out = begin_frame(FAMILY, LAYOUT, GENESIS, len, cap)?;
    put_identity(&mut out, record.identity);
    out.extend_from_slice(&record.initial_application_digest);
    out.extend_from_slice(&record.initial_system_digest);
    match record.provenance {
        GenesisProvenance::Create => out.push(0),
        GenesisProvenance::Restore { source_evidence } => {
            out.push(1);
            out.extend_from_slice(&source_evidence);
        }
        GenesisProvenance::Migration {
            source_database,
            source_incarnation,
            plan_set_digest,
        } => {
            out.push(2);
            out.extend_from_slice(source_database.as_core().as_bytes());
            out.extend_from_slice(source_incarnation.as_core().as_bytes());
            out.extend_from_slice(&plan_set_digest);
        }
    }
    debug_assert_eq!(out.len(), len);
    Ok(out)
}

/// # Errors
/// Refuses malformed genesis frames.
pub fn decode_genesis(bytes: &[u8], cap: usize) -> Result<GenesisRecord, FrameError> {
    let mut input = Reader::begin(bytes, FAMILY, LAYOUT, GENESIS, cap)?;
    let identity = input.identity()?;
    let initial_application_digest = input.array()?;
    let initial_system_digest = input.array()?;
    let provenance = match input.tag()? {
        (_, 0) => GenesisProvenance::Create,
        (_, 1) => GenesisProvenance::Restore {
            source_evidence: input.array()?,
        },
        (_, 2) => GenesisProvenance::Migration {
            source_database: super::DatabaseId::from_core(bumbledb::Id128::from_bytes(
                input.array()?,
            )),
            source_incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(
                input.array()?,
            )),
            plan_set_digest: input.array()?,
        },
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    input.end()?;
    Ok(GenesisRecord {
        identity,
        initial_application_digest,
        initial_system_digest,
        provenance,
    })
}

/// The sequence-zero genesis stamp for a record: hash the versioned record
/// bytes under the genesis domain. Distinct identities, schemas, initial
/// digests and provenance always yield distinct sentinels.
/// # Errors
/// Refuses oversized frames and allocation failure.
pub fn genesis_stamp(record: &GenesisRecord, cap: usize) -> Result<DecisionStamp, FrameError> {
    let bytes = encode_genesis(record, cap)?;
    Ok(DecisionStamp {
        seq: 0,
        hash: DecisionDigest::from_bytes(blake3::derive_key(GENESIS_DIGEST_DOMAIN, &bytes)),
    })
}

/// Canonical digests of the **empty** initial state, used by ordinary blank
/// creation. These are exactly the canonical empty export projection owned by
/// the checkpoint stream codec (C08/C12): genesis sentinels, blank hydration
/// checks and the empty export all name one value per domain. P05's forcing
/// test (`lane_checkpoint.rs::blank_initial_digests_equal_the_empty_export_projection`)
/// pins the equality; the earlier `bumbledb.decision.v1/empty-*` domains were
/// a recorded C12 defect and are gone.
#[must_use]
pub fn blank_initial_digests() -> ([u8; 32], [u8; 32]) {
    (
        crate::codec::empty_application_digest(),
        crate::codec::empty_system_digest(),
    )
}

#[cfg(test)]
mod tests {
    use bumbledb::Id128;

    use super::super::Condition;
    use super::super::{
        ChangeSummary, CommandId, DatabaseId, ReceiptEpoch, RequestId, SchemaId,
        command::{CommandMetadata, encode_command},
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

    fn state(revision: u64) -> StateStamp {
        StateStamp {
            incarnation: identity().incarnation_id,
            data_revision: revision,
        }
    }

    fn command_bytes() -> Vec<u8> {
        encode_command(
            CommandMetadata {
                identity: identity(),
                id: CommandId {
                    receipt_epoch: ReceiptEpoch::INITIAL,
                    request_id: RequestId::from_core(Id128::from_bytes([4; 16])),
                },
                condition: Condition::Unconditional,
            },
            &[0xaa, 0xbb],
            LIMITS,
        )
        .unwrap()
    }

    fn parts(command: &[u8]) -> DecisionParts<'_> {
        DecisionParts {
            identity: identity(),
            seq: 3,
            parent: DecisionStamp {
                seq: 2,
                hash: DecisionDigest::from_bytes([7; 32]),
            },
            before_state: state(1),
            after_state: state(2),
            canonical_command: command,
            outcome: UnverifiedOutcome::Committed {
                changed: ChangeSummary::new(1, 0).unwrap(),
                result: &[],
            },
        }
    }

    #[test]
    fn decisions_roundtrip_and_bind_the_nested_command() {
        let command = command_bytes();
        let bytes = encode_decision(parts(&command), LIMITS).unwrap();
        let decoded = decode_decision(&bytes, LIMITS).unwrap();
        assert_eq!(decoded.seq, 3);
        assert_eq!(decoded.canonical_command, command.as_slice());
        assert_eq!(decoded.command.identity, identity());
        assert_eq!(decoded.digest(), decision_digest(&bytes));
        verify_step(parts(&command).parent, &decoded).unwrap();
        assert!(matches!(
            verify_step(
                DecisionStamp {
                    seq: 2,
                    hash: DecisionDigest::from_bytes([8; 32]),
                },
                &decoded,
            ),
            Err(ChainError::WrongParent { .. })
        ));
        for end in 0..bytes.len() {
            assert!(decode_decision(&bytes[..end], LIMITS).is_err(), "{end}");
        }
    }

    #[test]
    fn inconsistent_stamps_and_foreign_nested_commands_refuse() {
        let command = command_bytes();
        let mut wrong_seq = parts(&command);
        wrong_seq.seq = 4;
        assert_eq!(
            encode_decision(wrong_seq, LIMITS),
            Err(FrameError::InvalidSequence)
        );
        let mut wrong_after = parts(&command);
        wrong_after.after_state = state(1);
        assert_eq!(
            encode_decision(wrong_after, LIMITS),
            Err(FrameError::InvalidTerminalStamp)
        );
        let mut no_change_moved = parts(&command);
        no_change_moved.outcome = UnverifiedOutcome::NoChange { result: &[] };
        assert_eq!(
            encode_decision(no_change_moved, LIMITS),
            Err(FrameError::InvalidTerminalStamp)
        );
        // A decision whose embedded command names another database refuses.
        let mut foreign = parts(&command);
        let foreign_command = encode_command(
            CommandMetadata {
                identity: DatabaseIdentity {
                    database_id: DatabaseId::from_core(Id128::from_bytes([9; 16])),
                    ..identity()
                },
                id: CommandId {
                    receipt_epoch: ReceiptEpoch::INITIAL,
                    request_id: RequestId::from_core(Id128::from_bytes([4; 16])),
                },
                condition: Condition::Unconditional,
            },
            &[0xaa],
            LIMITS,
        )
        .unwrap();
        foreign.canonical_command = &foreign_command;
        let bytes = encode_decision(foreign, LIMITS).unwrap();
        assert_eq!(
            decode_decision(&bytes, LIMITS),
            Err(FrameError::StateIdentityMismatch)
        );
    }

    #[test]
    fn genesis_sentinels_bind_identity_digests_and_provenance() {
        let record = GenesisRecord {
            identity: identity(),
            initial_application_digest: blank_initial_digests().0,
            initial_system_digest: blank_initial_digests().1,
            provenance: GenesisProvenance::Create,
        };
        let bytes = encode_genesis(&record, 4096).unwrap();
        assert_eq!(decode_genesis(&bytes, 4096).unwrap(), record);
        let stamp = genesis_stamp(&record, 4096).unwrap();
        assert_eq!(stamp.seq, 0);
        let restored = GenesisRecord {
            provenance: GenesisProvenance::Restore {
                source_evidence: [1; 32],
            },
            ..record
        };
        assert_ne!(
            stamp.hash,
            genesis_stamp(&restored, 4096).unwrap().hash,
            "provenance is part of the sentinel; no universal zero hash"
        );
        let other_identity = GenesisRecord {
            identity: DatabaseIdentity {
                incarnation_id: IncarnationId::from_core(Id128::from_bytes([9; 16])),
                ..identity()
            },
            ..record
        };
        assert_ne!(
            stamp.hash,
            genesis_stamp(&other_identity, 4096).unwrap().hash
        );
        let migration = GenesisRecord {
            provenance: GenesisProvenance::Migration {
                source_database: identity().database_id,
                source_incarnation: identity().incarnation_id,
                plan_set_digest: [5; 32],
            },
            ..record
        };
        let bytes = encode_genesis(&migration, 4096).unwrap();
        assert_eq!(decode_genesis(&bytes, 4096).unwrap(), migration);
    }
}
