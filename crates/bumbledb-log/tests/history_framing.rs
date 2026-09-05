//! The successor envelope boundary, not executable command/durability proof.

use bumbledb::Id128;
use bumbledb_log::history::command::{
    CommandMetadata, FAMILY, FrameError, LAYOUT, Limits, ReceiptMetadata, UnverifiedOutcome,
    UnverifiedReceiptEnvelope, decode_command, decode_receipt, encode_command, encode_receipt,
};
use bumbledb_log::history::{
    ChangeSummary, CommandDigest, CommandId, Condition, DatabaseId, DatabaseIdentity,
    DecisionDigest, DecisionStamp, EmptyResult, IncarnationId, ReceiptEpoch, RequestId, SchemaId,
    StateStamp,
};

const LIMITS: Limits = Limits {
    envelope_bytes: 1024,
    change_bytes: 256,
    evidence_bytes: 256,
};
const HEADER: usize = FAMILY.len() + 3;

fn metadata() -> CommandMetadata {
    CommandMetadata {
        identity: DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([0x11; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([0x22; 16])),
            schema_id: SchemaId([0x33; 32]),
        },
        id: CommandId {
            receipt_epoch: ReceiptEpoch::new(7).unwrap(),
            request_id: RequestId::from_core(Id128::from_bytes([0x44; 16])),
        },
        condition: Condition::Unconditional,
    }
}

fn state(revision: u64) -> StateStamp {
    StateStamp {
        incarnation: metadata().identity.incarnation_id,
        data_revision: revision,
    }
}

fn encoded_command() -> Vec<u8> {
    // Deliberately arbitrary bytes: successful framing is NOT core admission.
    encode_command(metadata(), &[0xde, 0xad, 0xbe, 0xef], LIMITS).unwrap()
}

fn receipt_metadata() -> ReceiptMetadata {
    let bytes = encoded_command();
    ReceiptMetadata {
        command: decode_command(&bytes, LIMITS).unwrap().command_ref(),
        decision_at: DecisionStamp {
            seq: 9,
            hash: DecisionDigest::from_bytes([0x55; 32]),
        },
        state_at: state(4),
    }
}

#[test]
fn command_has_explicit_family_kind_and_fixed_width_golden_bytes() {
    let bytes = encoded_command();
    let mut expected = b"bumbledb.command.v1\0\0\x01\x01".to_vec();
    expected.extend([0x11; 16]);
    expected.extend([0x22; 16]);
    expected.extend([0x33; 32]);
    expected.extend([0, 0, 0, 0, 0, 0, 0, 7]);
    expected.extend([0x44; 16]);
    expected.push(0);
    expected.extend([0, 0, 0, 0, 0, 0, 0, 4]);
    expected.extend([0xde, 0xad, 0xbe, 0xef]);
    expected.extend([0; 8]);
    assert_eq!(bytes, expected);
    assert_eq!(LAYOUT, 1);
    let decoded = decode_command(&bytes, LIMITS).unwrap();
    assert_eq!(decoded.metadata, metadata());
    assert_eq!(decoded.core_changes, &[0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(decoded.core_changes.as_ptr(), bytes[HEADER + 97..].as_ptr());
    assert_eq!(
        encode_command(decoded.metadata, decoded.core_changes, LIMITS).unwrap(),
        bytes
    );
    assert_ne!(
        decoded.command_ref().digest.as_bytes(),
        blake3::hash(&bytes).as_bytes()
    );
    assert_eq!(
        decoded.command_ref().digest,
        CommandDigest::from_bytes(blake3::derive_key(
            "bumbledb.command.v1/command-digest",
            &expected
        ))
    );
}

#[test]
fn stable_intent_digest_binds_every_identity_condition_and_application_byte() {
    let original = encoded_command();
    let original_ref = decode_command(&original, LIMITS).unwrap().command_ref();
    assert_eq!(
        original_ref,
        decode_command(&encoded_command(), LIMITS)
            .unwrap()
            .command_ref()
    );
    // Fixed fields: database, incarnation, schema, epoch, request, payload.
    for offset in [
        HEADER,
        HEADER + 16,
        HEADER + 32,
        HEADER + 71,
        HEADER + 72,
        HEADER + 97,
    ] {
        let mut changed = original.clone();
        changed[offset] ^= 1;
        assert_ne!(
            original_ref.digest,
            decode_command(&changed, LIMITS)
                .unwrap()
                .command_ref()
                .digest,
            "offset {offset}"
        );
    }
    let mut exact = metadata();
    exact.condition = Condition::ExactState(state(0));
    let exact_bytes = encode_command(exact, &[0xde, 0xad, 0xbe, 0xef], LIMITS).unwrap();
    let exact_ref = decode_command(&exact_bytes, LIMITS).unwrap().command_ref();
    assert_ne!(original_ref.digest, exact_ref.digest);
    exact.condition = Condition::ExactState(state(1));
    let revised = encode_command(exact, &[0xde, 0xad, 0xbe, 0xef], LIMITS).unwrap();
    assert_ne!(
        exact_ref.digest,
        decode_command(&revised, LIMITS)
            .unwrap()
            .command_ref()
            .digest
    );
}

#[test]
fn every_truncated_command_width_and_wrong_domain_refuses() {
    let mut commands = vec![encoded_command()];
    let mut exact = metadata();
    exact.condition = Condition::ExactState(state(u64::MAX));
    commands.push(encode_command(exact, &[1, 2, 3], LIMITS).unwrap());
    for bytes in commands {
        for end in 0..bytes.len() {
            assert!(
                decode_command(&bytes[..end], LIMITS).is_err(),
                "prefix {end}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            decode_command(&trailing, LIMITS),
            Err(FrameError::TrailingBytes { at: bytes.len() })
        );
    }
    let mut bytes = encoded_command();
    bytes[0] ^= 1;
    assert_eq!(decode_command(&bytes, LIMITS), Err(FrameError::Family));
    bytes = encoded_command();
    bytes[FAMILY.len() + 1] = 2;
    assert_eq!(
        decode_command(&bytes, LIMITS),
        Err(FrameError::Layout { got: 2 })
    );
    bytes = encoded_command();
    bytes[HEADER - 1] = 2;
    assert_eq!(
        decode_command(&bytes, LIMITS),
        Err(FrameError::Kind { got: 2 })
    );
    bytes = encoded_command();
    bytes[HEADER + 88] = 2;
    assert_eq!(
        decode_command(&bytes, LIMITS),
        Err(FrameError::Tag {
            at: HEADER + 88,
            got: 2
        })
    );
    bytes = encoded_command();
    bytes[HEADER + 64..HEADER + 72].fill(0);
    assert_eq!(
        decode_command(&bytes, LIMITS),
        Err(FrameError::InvalidEpoch)
    );
}

#[test]
fn byte_caps_are_checked_before_allocation_and_result_is_not_an_opaque_escape() {
    let bytes = encoded_command();
    let exact = Limits {
        envelope_bytes: bytes.len(),
        change_bytes: 4,
        evidence_bytes: 0,
    };
    assert!(decode_command(&bytes, exact).is_ok());
    assert_eq!(
        encode_command(
            metadata(),
            &[0; 4],
            Limits {
                change_bytes: 3,
                ..exact
            }
        ),
        Err(FrameError::LimitExceeded)
    );
    assert_eq!(
        decode_command(
            &bytes,
            Limits {
                envelope_bytes: bytes.len() - 1,
                ..exact
            }
        ),
        Err(FrameError::LimitExceeded)
    );
    let mut huge = bytes.clone();
    huge[HEADER + 89..HEADER + 97].copy_from_slice(&u64::MAX.to_be_bytes());
    assert_eq!(
        decode_command(&huge, LIMITS),
        Err(FrameError::LimitExceeded)
    );
    let mut nonempty = bytes;
    let last = nonempty.len() - 1;
    nonempty[last] = 1;
    assert_eq!(
        decode_command(&nonempty, LIMITS),
        Err(FrameError::NonemptyResultUnsupported)
    );
    let mut wrong_incarnation = metadata();
    wrong_incarnation.condition = Condition::ExactState(StateStamp {
        incarnation: IncarnationId::from_core(Id128::from_bytes([9; 16])),
        data_revision: 0,
    });
    assert_eq!(
        encode_command(wrong_incarnation, &[], LIMITS),
        Err(FrameError::StateIdentityMismatch)
    );
}

#[test]
fn all_terminal_envelopes_roundtrip_without_claiming_core_evidence_is_verified() {
    let outcomes = [
        UnverifiedOutcome::Committed {
            changed: ChangeSummary::new(2, 1).unwrap(),
            result: EmptyResult,
        },
        UnverifiedOutcome::NoChange {
            result: EmptyResult,
        },
        UnverifiedOutcome::PreconditionFailed {
            expected: state(3),
            observed: state(4),
        },
        UnverifiedOutcome::InvariantRejected {
            core_evidence: b"not-a-verified-core-evidence-record",
        },
    ];
    for outcome in outcomes {
        let receipt = UnverifiedReceiptEnvelope {
            metadata: receipt_metadata(),
            outcome,
        };
        let bytes = encode_receipt(receipt, LIMITS).unwrap();
        assert_eq!(decode_receipt(&bytes, LIMITS).unwrap(), receipt);
        assert_eq!(
            decode_command(&bytes, LIMITS),
            Err(FrameError::Kind { got: 2 })
        );
        for end in 0..bytes.len() {
            assert!(
                decode_receipt(&bytes[..end], LIMITS).is_err(),
                "outcome {outcome:?}, prefix {end}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            decode_receipt(&trailing, LIMITS),
            Err(FrameError::TrailingBytes { at: bytes.len() })
        );
    }
}

#[test]
fn receipt_tags_counts_state_and_evidence_limits_are_strict() {
    let receipt = UnverifiedReceiptEnvelope {
        metadata: receipt_metadata(),
        outcome: UnverifiedOutcome::Committed {
            changed: ChangeSummary::new(1, 0).unwrap(),
            result: EmptyResult,
        },
    };
    let mut bytes = encode_receipt(receipt, LIMITS).unwrap();
    bytes[HEADER + 184] = 4;
    assert_eq!(
        decode_receipt(&bytes, LIMITS),
        Err(FrameError::Tag {
            at: HEADER + 184,
            got: 4
        })
    );
    bytes = encode_receipt(receipt, LIMITS).unwrap();
    bytes[HEADER + 185..HEADER + 201].fill(0);
    assert_eq!(
        decode_receipt(&bytes, LIMITS),
        Err(FrameError::EmptyChangeSummary)
    );
    let mut invalid = receipt;
    invalid.metadata.decision_at.seq = 0;
    assert_eq!(
        encode_receipt(invalid, LIMITS),
        Err(FrameError::InvalidTerminalStamp)
    );
    invalid = receipt;
    invalid.metadata.state_at.data_revision = 10;
    assert_eq!(
        encode_receipt(invalid, LIMITS),
        Err(FrameError::InvalidTerminalStamp)
    );
    invalid.metadata.state_at.data_revision = 0;
    assert_eq!(
        encode_receipt(invalid, LIMITS),
        Err(FrameError::InvalidTerminalStamp)
    );
    invalid = receipt;
    invalid.metadata.state_at.incarnation = IncarnationId::from_core(Id128::from_bytes([6; 16]));
    assert_eq!(
        encode_receipt(invalid, LIMITS),
        Err(FrameError::StateIdentityMismatch)
    );
    for outcome in [
        UnverifiedOutcome::PreconditionFailed {
            expected: state(4),
            observed: state(4),
        },
        UnverifiedOutcome::PreconditionFailed {
            expected: state(2),
            observed: state(3),
        },
    ] {
        assert_eq!(
            encode_receipt(UnverifiedReceiptEnvelope { outcome, ..receipt }, LIMITS),
            Err(FrameError::InvalidPreconditionEvidence)
        );
    }
    let empty = UnverifiedReceiptEnvelope {
        outcome: UnverifiedOutcome::InvariantRejected { core_evidence: &[] },
        ..receipt
    };
    assert_eq!(
        encode_receipt(empty, LIMITS),
        Err(FrameError::EmptyEvidence)
    );
    let evidence = UnverifiedReceiptEnvelope {
        outcome: UnverifiedOutcome::InvariantRejected {
            core_evidence: &[1; 257],
        },
        ..receipt
    };
    assert_eq!(
        encode_receipt(evidence, LIMITS),
        Err(FrameError::LimitExceeded)
    );
}
