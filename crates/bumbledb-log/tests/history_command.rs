//! Actual core `ChangeSet` ownership/decoder through the successor log envelope.
//! This is canonical command qualification, not a complete history authority.

use std::time::Duration;

use bumbledb::schema::{
    FieldDescriptor, Generation, RelationDescriptor, SchemaDescriptor, ValidateDescriptor as _,
    ValueType,
};
use bumbledb::work::Resource;
use bumbledb::{
    ChangeError, ChangeSet, ExecutionPolicy, Id128, RelationId, Schema, Value, WorkContext,
    WorkError,
};
use bumbledb_log::history::command::{
    Command, CommandError, CommandMetadata, FrameError, Limits, decode_command, encode_command,
};
use bumbledb_log::history::{
    CommandId, Condition, DatabaseId, DatabaseIdentity, IncarnationId, ReceiptEpoch, RequestId,
    StateStamp,
};

const LIMITS: Limits = Limits {
    envelope_bytes: 100_000,
    change_bytes: 90_000,
    evidence_bytes: 1000,
};

fn schema(name: &str, value_type: ValueType) -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: name.into(),
            fields: vec![FieldDescriptor {
                name: "value".into(),
                value_type,
                generation: Generation::None,
            }],
            extension: None,
        }],
        statements: vec![],
    }
    .validate()
    .unwrap()
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 10_000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(30),
    }
}

fn work() -> WorkContext {
    policy().start().unwrap()
}

fn metadata(schema: &Schema) -> CommandMetadata {
    CommandMetadata {
        identity: DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes([2; 16])),
            schema_id: bumbledb::schema::fingerprint::fingerprint(schema),
        },
        id: CommandId {
            receipt_epoch: ReceiptEpoch::INITIAL,
            request_id: RequestId::from_core(Id128::from_bytes([3; 16])),
        },
        condition: Condition::Unconditional,
    }
}

fn numbers(schema: &Schema, context: &WorkContext, reverse: bool) -> ChangeSet {
    let mut draft = ChangeSet::builder(schema, context.clone());
    for number in if reverse { [3, 2, 1] } else { [1, 2, 3] } {
        draft.delete(RelationId(0), &[Value::U64(number)]).unwrap();
        draft.insert(RelationId(0), &[Value::U64(number)]).unwrap();
        draft.insert(RelationId(0), &[Value::U64(number)]).unwrap();
    }
    draft.finish().unwrap()
}

#[test]
fn sealed_command_retains_exact_core_allocation_and_live_memory_charge() {
    let schema = schema("Number", ValueType::U64);
    let source_work = work();
    let changes = numbers(&schema, &source_work, false);
    let native_bytes = changes.as_bytes().as_ptr();
    let charged = source_work.used(Resource::WorkingBytes);
    assert!(charged >= changes.as_bytes().len() as u64);
    let seal_work = work();
    let command = Command::seal(metadata(&schema), changes.clone(), LIMITS, &seal_work).unwrap();
    assert_eq!(command.changes().as_bytes().as_ptr(), native_bytes);
    assert_eq!(
        source_work.used(Resource::WorkingBytes),
        charged,
        "no new change-body copy"
    );
    assert_eq!(
        seal_work.used(Resource::WorkingBytes),
        0,
        "hashing needs no resident envelope clone"
    );
    drop(changes);
    assert_eq!(source_work.used(Resource::WorkingBytes), charged);
    let retry = command.clone();
    drop(command);
    assert_eq!(retry.changes().as_bytes().as_ptr(), native_bytes);
    drop(retry);
    assert_eq!(source_work.used(Resource::WorkingBytes), 0);
}

#[test]
fn equal_normalized_intents_hash_identically_and_owned_parse_uses_core_decoder() {
    let schema = schema("Number", ValueType::U64);
    let left = Command::seal(
        metadata(&schema),
        numbers(&schema, &work(), false),
        LIMITS,
        &work(),
    )
    .unwrap();
    let right = Command::seal(
        metadata(&schema),
        numbers(&schema, &work(), true),
        LIMITS,
        &work(),
    )
    .unwrap();
    assert_eq!(left.command_ref(), right.command_ref());
    assert_eq!(
        left.changes().len(),
        3,
        "same-command additions win once per fact"
    );
    let mut wire = encode_command(left.metadata(), left.changes().as_bytes(), LIMITS).unwrap();
    let decoded = Command::parse(&schema, &wire, LIMITS, &work()).unwrap();
    assert_eq!(decoded.command_ref(), left.command_ref());
    assert_eq!(
        decoded.command_ref(),
        decode_command(&wire, LIMITS).unwrap().command_ref()
    );
    assert_eq!(decoded.changes().as_bytes(), left.changes().as_bytes());
    wire.fill(0);
    assert_eq!(
        decoded.changes().as_bytes(),
        left.changes().as_bytes(),
        "parse owns immutable core bytes"
    );
    let mut conditional = metadata(&schema);
    conditional.condition = Condition::ExactState(StateStamp {
        incarnation: conditional.identity.incarnation_id,
        data_revision: 0,
    });
    let conditional = Command::seal(conditional, left.changes().clone(), LIMITS, &work()).unwrap();
    assert_ne!(conditional.command_ref().digest, left.command_ref().digest);
}

#[test]
fn well_framed_arbitrary_bytes_cannot_become_a_verified_command() {
    let schema = schema("Number", ValueType::U64);
    let bytes = encode_command(metadata(&schema), b"not a checked delta", LIMITS).unwrap();
    assert!(decode_command(&bytes, LIMITS).is_ok());
    assert!(matches!(
        Command::parse(&schema, &bytes, LIMITS, &work()),
        Err(CommandError::Core(ChangeError::Truncated))
    ));
    let core = numbers(&schema, &work(), false);
    let mut malformed = core.as_bytes().to_vec();
    malformed[0] ^= 1;
    let bytes = encode_command(metadata(&schema), &malformed, LIMITS).unwrap();
    assert!(matches!(
        Command::parse(&schema, &bytes, LIMITS, &work()),
        Err(CommandError::Core(ChangeError::WrongFamily))
    ));
    for end in 0..core.as_bytes().len() {
        let bytes = encode_command(metadata(&schema), &core.as_bytes()[..end], LIMITS).unwrap();
        assert!(
            matches!(
                Command::parse(&schema, &bytes, LIMITS, &work()),
                Err(CommandError::Core(_))
            ),
            "core prefix {end}"
        );
    }
}

#[test]
fn complete_schema_identity_limits_and_work_are_enforced_before_sealing() {
    let schema = schema("Number", ValueType::U64);
    let other = self::schema("OtherNumber", ValueType::U64);
    let changes = numbers(&schema, &work(), false);
    assert!(matches!(
        Command::seal(metadata(&other), changes.clone(), LIMITS, &work()),
        Err(CommandError::SchemaMismatch)
    ));
    let wire = encode_command(metadata(&schema), changes.as_bytes(), LIMITS).unwrap();
    assert!(matches!(
        Command::parse(&other, &wire, LIMITS, &work()),
        Err(CommandError::SchemaMismatch)
    ));
    let tiny = Limits {
        change_bytes: 0,
        ..LIMITS
    };
    assert!(matches!(
        Command::seal(metadata(&schema), changes.clone(), tiny, &work()),
        Err(CommandError::Frame(FrameError::LimitExceeded))
    ));
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        Command::seal(metadata(&schema), changes.clone(), LIMITS, &cancelled),
        Err(CommandError::Work(WorkError::Cancelled))
    ));
    let none = ExecutionPolicy {
        work_units: 0,
        ..policy()
    }
    .start()
    .unwrap();
    assert!(matches!(
        Command::seal(metadata(&schema), changes, LIMITS, &none),
        Err(CommandError::Work(WorkError::Exhausted {
            resource: Resource::WorkUnits,
            ..
        }))
    ));
    let input = ExecutionPolicy {
        input_bytes: 0,
        ..policy()
    }
    .start()
    .unwrap();
    assert!(matches!(
        Command::parse(&schema, &wire, LIMITS, &input),
        Err(CommandError::Work(WorkError::Exhausted {
            resource: Resource::InputBytes,
            ..
        }))
    ));
}

#[test]
fn large_core_payload_hash_is_charged_in_bounded_chunks_without_a_copy() {
    let schema = schema("Text", ValueType::String);
    let mut draft = ChangeSet::builder(&schema, work());
    draft
        .insert(RelationId(0), &[Value::String("x".repeat(20_000).into())])
        .unwrap();
    let changes = draft.finish().unwrap();
    let limited = ExecutionPolicy {
        work_units: 12,
        ..policy()
    }
    .start()
    .unwrap();
    assert!(matches!(
        Command::seal(metadata(&schema), changes.clone(), LIMITS, &limited),
        Err(CommandError::Work(WorkError::Exhausted {
            resource: Resource::WorkUnits,
            ..
        }))
    ));
    let context = work();
    let command = Command::seal(metadata(&schema), changes, LIMITS, &context).unwrap();
    assert!(context.used(Resource::WorkUnits) >= 16);
    let bytes = encode_command(command.metadata(), command.changes().as_bytes(), LIMITS).unwrap();
    assert_eq!(
        command.command_ref(),
        decode_command(&bytes, LIMITS).unwrap().command_ref()
    );
}
