//! Versioned envelope framing only. Application changes, declared result
//! bytes and rejection evidence remain borrowed core-owned byte spans until
//! the core decoder verifies them. No decoded value in this module is an
//! executable command or trusted receipt.

use bumbledb::{ChangeError, ChangeSet, Schema, WorkContext, WorkError};

use super::frame::{
    self, Reader, begin_frame, check_limit, frame_len, put_id, put_identity, put_span, put_state,
    put_u64,
};
use super::{
    ChangeSummary, CommandDigest, CommandId, CommandRef, CommandResult, Condition,
    DatabaseIdentity, DecisionStamp, StateStamp,
};

pub use super::frame::FrameError;

pub const FAMILY: &[u8] = b"bumbledb.command.v1\0";
pub const LAYOUT: u16 = 1;
const COMMAND: u8 = 1;
const RECEIPT: u8 = 2;
const COMMAND_DIGEST_DOMAIN: &str = "bumbledb.command.v1/command-digest";
const HASH_QUANTUM: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    Frame(FrameError),
    Core(ChangeError),
    SchemaMismatch,
    Work(WorkError),
}

impl From<FrameError> for CommandError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<ChangeError> for CommandError {
    fn from(error: ChangeError) -> Self {
        Self::Core(error)
    }
}

impl From<WorkError> for CommandError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

/// One immutable named request retaining the core's *same* checked payload.
/// No callback, log row representation, mutable source, or full envelope copy.
/// This proves canonical command grammar, not state admission or publication.
#[derive(Debug, Clone)]
pub struct Command {
    metadata: CommandMetadata,
    changes: ChangeSet,
    result: CommandResult,
    reference: CommandRef,
}

impl Command {
    /// Retain already checked core ownership and bind every command field,
    /// including the bounded declared result metadata. Hashing checks the
    /// shared work allowance at bounded byte intervals.
    /// # Errors
    /// Refuses mismatched schema/witness, frame limits or exhausted work.
    pub fn seal(
        metadata: CommandMetadata,
        changes: ChangeSet,
        result: CommandResult,
        limits: Limits,
        work: &WorkContext,
    ) -> Result<Self, CommandError> {
        work.checkpoint()?;
        if metadata.identity.schema_id != changes.schema() {
            return Err(CommandError::SchemaMismatch);
        }
        command_size(metadata, changes.as_bytes(), result.as_bytes(), limits)?;
        let mut hasher = blake3::Hasher::new_derive_key(COMMAND_DIGEST_DOMAIN);
        command_parts(metadata, changes.as_bytes(), result.as_bytes(), |part| {
            for chunk in part.chunks(HASH_QUANTUM) {
                work.step(1)?;
                hasher.update(chunk);
            }
            Ok::<_, WorkError>(())
        })?;
        let reference = CommandRef {
            identity: metadata.identity,
            id: metadata.id,
            digest: CommandDigest::from_bytes(*hasher.finalize().as_bytes()),
        };
        Ok(Self {
            metadata,
            changes,
            result,
            reference,
        })
    }

    /// The verified boundary invokes the core's one strict change decoder.
    /// Framing alone never upgrades arbitrary bytes into an executable delta.
    /// # Errors
    /// Refuses malformed/noncanonical/foreign data and bounded resource failures.
    pub fn parse(
        schema: &Schema,
        bytes: &[u8],
        limits: Limits,
        work: &WorkContext,
    ) -> Result<Self, CommandError> {
        work.checkpoint()?;
        let (metadata, core_changes, result) = parse_command_fields(bytes, limits)?;
        if metadata.identity.schema_id != bumbledb::schema::fingerprint::fingerprint(schema) {
            return Err(CommandError::SchemaMismatch);
        }
        work.input((bytes.len() - core_changes.len()) as u64)?;
        let changes = ChangeSet::parse(schema, core_changes, work)?;
        Self::seal(
            metadata,
            changes,
            CommandResult::from_canonical_bytes(Box::from(result)),
            limits,
            work,
        )
    }

    #[must_use]
    pub const fn metadata(&self) -> CommandMetadata {
        self.metadata
    }

    #[must_use]
    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    #[must_use]
    pub fn result(&self) -> &CommandResult {
        &self.result
    }

    #[must_use]
    pub const fn command_ref(&self) -> CommandRef {
        self.reference
    }

    /// The exact canonical envelope bytes whose digest is `command_ref`.
    /// # Errors
    /// Refuses oversized frames and allocation failure.
    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, FrameError> {
        encode_command_with_result(
            self.metadata,
            self.changes.as_bytes(),
            self.result.as_bytes(),
            limits,
        )
    }
}

/// Explicit resident-memory limits for this first borrowed/in-memory packet.
/// These are not a requirement that future spillable core changes reside in RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub envelope_bytes: usize,
    pub change_bytes: usize,
    pub evidence_bytes: usize,
    pub result_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMetadata {
    pub identity: DatabaseIdentity,
    pub id: CommandId,
    pub condition: Condition,
}

/// Successfully framed, **not core-admitted**. Hashing arbitrary bytes does
/// not make them a canonical checked delta. Only the core can establish that.
#[derive(Debug, PartialEq, Eq)]
pub struct UnverifiedCommandEnvelope<'a> {
    pub metadata: CommandMetadata,
    pub core_changes: &'a [u8],
    pub result: &'a [u8],
    digest: CommandDigest,
}

impl UnverifiedCommandEnvelope<'_> {
    /// Copied identity for these exact bytes, not proof of executable meaning.
    #[must_use]
    pub const fn command_ref(&self) -> CommandRef {
        CommandRef {
            identity: self.metadata.identity,
            id: self.metadata.id,
            digest: self.digest,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptMetadata {
    pub command: CommandRef,
    pub decision_at: DecisionStamp,
    pub state_at: StateStamp,
}

/// Outcome framing does not verify core evidence or recorded semantic judgment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnverifiedOutcome<'a> {
    Committed {
        changed: ChangeSummary,
        result: &'a [u8],
    },
    NoChange {
        result: &'a [u8],
    },
    PreconditionFailed {
        expected: StateStamp,
        observed: StateStamp,
    },
    InvariantRejected {
        core_evidence: &'a [u8],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnverifiedReceiptEnvelope<'a> {
    pub metadata: ReceiptMetadata,
    pub outcome: UnverifiedOutcome<'a>,
}

/// Encode borrowed core bytes with no log-level scalar/fact interpretation.
/// The declared result is empty here; [`encode_command_with_result`] frames a
/// nonempty bounded canonical result.
/// # Errors
/// Refuses mismatched witness identity, oversized frames and allocation failure.
pub fn encode_command(
    metadata: CommandMetadata,
    core_changes: &[u8],
    limits: Limits,
) -> Result<Vec<u8>, FrameError> {
    encode_command_with_result(metadata, core_changes, &[], limits)
}

/// # Errors
/// Refuses mismatched witness identity, oversized frames and allocation failure.
pub fn encode_command_with_result(
    metadata: CommandMetadata,
    core_changes: &[u8],
    result: &[u8],
    limits: Limits,
) -> Result<Vec<u8>, FrameError> {
    let len = command_size(metadata, core_changes, result, limits)?;
    let mut out = frame::allocate_frame(len, limits.envelope_bytes)?;
    command_parts(metadata, core_changes, result, |part| {
        out.extend_from_slice(part);
        Ok::<_, FrameError>(())
    })?;
    debug_assert_eq!(out.len(), len);
    Ok(out)
}

fn command_size(
    metadata: CommandMetadata,
    core_changes: &[u8],
    result: &[u8],
    limits: Limits,
) -> Result<usize, FrameError> {
    validate_condition(metadata.identity, metadata.condition)?;
    check_limit(core_changes.len(), limits.change_bytes)?;
    check_limit(result.len(), limits.result_bytes)?;
    let condition_len = if matches!(metadata.condition, Condition::Unconditional) {
        1
    } else {
        25
    };
    let len = frame_len(
        FAMILY.len(),
        &[88, condition_len, 8, core_changes.len(), 8, result.len()],
    )?;
    check_limit(len, limits.envelope_bytes)?;
    Ok(len)
}

// One framing definition for the bounded encoder and zero-copy command hash.
fn command_parts<E>(
    metadata: CommandMetadata,
    core_changes: &[u8],
    result: &[u8],
    mut part: impl FnMut(&[u8]) -> Result<(), E>,
) -> Result<(), E> {
    part(FAMILY)?;
    part(&LAYOUT.to_be_bytes())?;
    part(&[COMMAND])?;
    part(metadata.identity.database_id.as_core().as_bytes())?;
    part(metadata.identity.incarnation_id.as_core().as_bytes())?;
    part(&metadata.identity.schema_id.0)?;
    part(&metadata.id.receipt_epoch.get().to_be_bytes())?;
    part(metadata.id.request_id.as_core().as_bytes())?;
    match metadata.condition {
        Condition::Unconditional => part(&[0])?,
        Condition::ExactState(state) => {
            part(&[1])?;
            part(state.incarnation.as_core().as_bytes())?;
            part(&state.data_revision.to_be_bytes())?;
        }
    }
    part(&(core_changes.len() as u64).to_be_bytes())?;
    part(core_changes)?;
    part(&(result.len() as u64).to_be_bytes())?;
    part(result)
}

/// Borrow an exactly framed command and compute its domain-separated binding.
/// # Errors
/// Refuses malformed/oversized framing, trailing bytes and oversized results.
pub fn decode_command(
    bytes: &[u8],
    limits: Limits,
) -> Result<UnverifiedCommandEnvelope<'_>, FrameError> {
    let (metadata, core_changes, result) = parse_command_fields(bytes, limits)?;
    let digest = CommandDigest::from_bytes(blake3::derive_key(COMMAND_DIGEST_DOMAIN, bytes));
    Ok(UnverifiedCommandEnvelope {
        metadata,
        core_changes,
        result,
        digest,
    })
}

type CommandFields<'a> = (CommandMetadata, &'a [u8], &'a [u8]);

fn parse_command_fields(bytes: &[u8], limits: Limits) -> Result<CommandFields<'_>, FrameError> {
    let mut input = Reader::begin(bytes, FAMILY, LAYOUT, COMMAND, limits.envelope_bytes)?;
    let identity = input.identity()?;
    let id = input.id()?;
    let condition = match input.tag()? {
        (_, 0) => Condition::Unconditional,
        (_, 1) => Condition::ExactState(input.state()?),
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    validate_condition(identity, condition)?;
    let core_changes = input.span(limits.change_bytes)?;
    let result = input.span(limits.result_bytes)?;
    input.end()?;
    Ok((
        CommandMetadata {
            identity,
            id,
            condition,
        },
        core_changes,
        result,
    ))
}

/// Encode outcome metadata and opaque core rejection evidence. This cannot
/// interpret core `Violations`; canonical evidence bytes are core-owned.
/// # Errors
/// Refuses invalid metadata, empty evidence, oversized frames and allocation.
pub fn encode_receipt(
    receipt: UnverifiedReceiptEnvelope<'_>,
    limits: Limits,
) -> Result<Vec<u8>, FrameError> {
    validate_receipt(receipt)?;
    let outcome_len = outcome_len(receipt.outcome, limits)?;
    let len = frame_len(FAMILY.len(), &[120, 40, 24, outcome_len])?;
    let mut out = begin_frame(FAMILY, LAYOUT, RECEIPT, len, limits.envelope_bytes)?;
    let metadata = receipt.metadata;
    put_identity(&mut out, metadata.command.identity);
    put_id(&mut out, metadata.command.id);
    out.extend_from_slice(metadata.command.digest.as_bytes());
    put_u64(&mut out, metadata.decision_at.seq);
    out.extend_from_slice(metadata.decision_at.hash.as_bytes());
    put_state(&mut out, metadata.state_at);
    put_outcome(&mut out, receipt.outcome)?;
    debug_assert_eq!(out.len(), len);
    Ok(out)
}

/// The framed byte length of one outcome section, checked against limits.
pub(crate) fn outcome_len(
    outcome: UnverifiedOutcome<'_>,
    limits: Limits,
) -> Result<usize, FrameError> {
    Ok(match outcome {
        UnverifiedOutcome::Committed { result, .. } => {
            check_limit(result.len(), limits.result_bytes)?;
            result
                .len()
                .checked_add(25)
                .ok_or(FrameError::LengthOverflow)?
        }
        UnverifiedOutcome::NoChange { result } => {
            check_limit(result.len(), limits.result_bytes)?;
            result
                .len()
                .checked_add(9)
                .ok_or(FrameError::LengthOverflow)?
        }
        UnverifiedOutcome::PreconditionFailed { .. } => 49,
        UnverifiedOutcome::InvariantRejected { core_evidence } => {
            check_limit(core_evidence.len(), limits.evidence_bytes)?;
            core_evidence
                .len()
                .checked_add(9)
                .ok_or(FrameError::LengthOverflow)?
        }
    })
}

pub(crate) fn put_outcome(
    out: &mut Vec<u8>,
    outcome: UnverifiedOutcome<'_>,
) -> Result<(), FrameError> {
    match outcome {
        UnverifiedOutcome::Committed { changed, result } => {
            out.push(0);
            put_u64(out, changed.added());
            put_u64(out, changed.removed());
            put_span(out, result)?;
        }
        UnverifiedOutcome::NoChange { result } => {
            out.push(1);
            put_span(out, result)?;
        }
        UnverifiedOutcome::PreconditionFailed { expected, observed } => {
            out.push(2);
            put_state(out, expected);
            put_state(out, observed);
        }
        UnverifiedOutcome::InvariantRejected { core_evidence } => {
            out.push(3);
            put_span(out, core_evidence)?;
        }
    }
    Ok(())
}

pub(crate) fn read_outcome<'a>(
    input: &mut Reader<'a>,
    limits: Limits,
) -> Result<UnverifiedOutcome<'a>, FrameError> {
    Ok(match input.tag()? {
        (_, 0) => {
            let changed = ChangeSummary::new(input.u64()?, input.u64()?)
                .ok_or(FrameError::EmptyChangeSummary)?;
            UnverifiedOutcome::Committed {
                changed,
                result: input.span(limits.result_bytes)?,
            }
        }
        (_, 1) => UnverifiedOutcome::NoChange {
            result: input.span(limits.result_bytes)?,
        },
        (_, 2) => UnverifiedOutcome::PreconditionFailed {
            expected: input.state()?,
            observed: input.state()?,
        },
        (_, 3) => UnverifiedOutcome::InvariantRejected {
            core_evidence: input.span(limits.evidence_bytes)?,
        },
        (at, got) => return Err(FrameError::Tag { at, got }),
    })
}

/// Decode outcome spans without admitting a durable receipt or core evidence.
/// # Errors
/// Refuses malformed/oversized framing, trailing bytes and oversized spans.
pub fn decode_receipt(
    bytes: &[u8],
    limits: Limits,
) -> Result<UnverifiedReceiptEnvelope<'_>, FrameError> {
    let mut input = Reader::begin(bytes, FAMILY, LAYOUT, RECEIPT, limits.envelope_bytes)?;
    let identity = input.identity()?;
    let id = input.id()?;
    let digest = CommandDigest::from_bytes(input.array()?);
    let decision_at = input.stamp()?;
    let state_at = input.state()?;
    let outcome = read_outcome(&mut input, limits)?;
    input.end()?;
    let receipt = UnverifiedReceiptEnvelope {
        metadata: ReceiptMetadata {
            command: CommandRef {
                identity,
                id,
                digest,
            },
            decision_at,
            state_at,
        },
        outcome,
    };
    validate_receipt(receipt)?;
    Ok(receipt)
}

pub(crate) fn validate_condition(
    identity: DatabaseIdentity,
    condition: Condition,
) -> Result<(), FrameError> {
    if let Condition::ExactState(state) = condition
        && state.incarnation != identity.incarnation_id
    {
        return Err(FrameError::StateIdentityMismatch);
    }
    Ok(())
}

pub(crate) fn validate_receipt(receipt: UnverifiedReceiptEnvelope<'_>) -> Result<(), FrameError> {
    let metadata = receipt.metadata;
    if metadata.decision_at.seq == 0
        || metadata.state_at.data_revision > metadata.decision_at.seq
        || (matches!(receipt.outcome, UnverifiedOutcome::Committed { .. })
            && metadata.state_at.data_revision == 0)
    {
        return Err(FrameError::InvalidTerminalStamp);
    }
    if metadata.state_at.incarnation != metadata.command.identity.incarnation_id {
        return Err(FrameError::StateIdentityMismatch);
    }
    match receipt.outcome {
        UnverifiedOutcome::PreconditionFailed { expected, observed } => {
            validate_condition(metadata.command.identity, Condition::ExactState(expected))?;
            if expected == observed || observed != metadata.state_at {
                return Err(FrameError::InvalidPreconditionEvidence);
            }
        }
        UnverifiedOutcome::InvariantRejected { core_evidence: [] } => {
            return Err(FrameError::EmptyEvidence);
        }
        _ => {}
    }
    Ok(())
}
