//! Versioned envelope framing only. Application changes and rejection evidence
//! remain borrowed core-owned byte spans until the core decoder verifies them.
//! No decoded value in this module is an executable command or trusted receipt.

use bumbledb::{ChangeError, ChangeSet, Id128, Schema, WorkContext, WorkError};

use super::{
    ChangeSummary, CommandDigest, CommandId, CommandRef, Condition, DatabaseId, DatabaseIdentity,
    DecisionDigest, DecisionStamp, EmptyResult, IncarnationId, ReceiptEpoch, RequestId, SchemaId,
    StateStamp,
};

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
    reference: CommandRef,
}

impl Command {
    /// Retain already checked core ownership and bind every command field.
    /// Hashing checks the shared work allowance at bounded byte intervals.
    /// # Errors
    /// Refuses mismatched schema/witness, frame limits or exhausted work.
    pub fn seal(
        metadata: CommandMetadata,
        changes: ChangeSet,
        limits: Limits,
        work: &WorkContext,
    ) -> Result<Self, CommandError> {
        work.checkpoint()?;
        if metadata.identity.schema_id != changes.schema() {
            return Err(CommandError::SchemaMismatch);
        }
        command_size(metadata, changes.as_bytes(), limits)?;
        let mut hasher = blake3::Hasher::new_derive_key(COMMAND_DIGEST_DOMAIN);
        command_parts(metadata, changes.as_bytes(), |part| {
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
        let (metadata, core_changes) = parse_command_fields(bytes, limits)?;
        if metadata.identity.schema_id != bumbledb::schema::fingerprint::fingerprint(schema) {
            return Err(CommandError::SchemaMismatch);
        }
        work.input((bytes.len() - core_changes.len()) as u64)?;
        let changes = ChangeSet::parse(schema, core_changes, work)?;
        Self::seal(metadata, changes, limits, work)
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
    pub const fn command_ref(&self) -> CommandRef {
        self.reference
    }
}

/// Explicit resident-memory limits for this first borrowed/in-memory packet.
/// These are not a requirement that future spillable core changes reside in RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub envelope_bytes: usize,
    pub change_bytes: usize,
    pub evidence_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    LimitExceeded,
    LengthOverflow,
    Allocation,
    Truncated { at: usize },
    Family,
    Layout { got: u16 },
    Kind { got: u8 },
    Tag { at: usize, got: u8 },
    InvalidEpoch,
    StateIdentityMismatch,
    NonemptyResultUnsupported,
    EmptyChangeSummary,
    EmptyEvidence,
    InvalidTerminalStamp,
    InvalidPreconditionEvidence,
    TrailingBytes { at: usize },
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
    pub result: EmptyResult,
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
        result: EmptyResult,
    },
    NoChange {
        result: EmptyResult,
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
/// # Errors
/// Refuses mismatched witness identity, oversized frames and allocation failure.
pub fn encode_command(
    metadata: CommandMetadata,
    core_changes: &[u8],
    limits: Limits,
) -> Result<Vec<u8>, FrameError> {
    let len = command_size(metadata, core_changes, limits)?;
    let mut out = allocate_frame(len, limits)?;
    command_parts(metadata, core_changes, |part| {
        out.extend_from_slice(part);
        Ok::<_, FrameError>(())
    })?;
    debug_assert_eq!(out.len(), len);
    Ok(out)
}

fn command_size(
    metadata: CommandMetadata,
    core_changes: &[u8],
    limits: Limits,
) -> Result<usize, FrameError> {
    validate_condition(metadata.identity, metadata.condition)?;
    check_limit(core_changes.len(), limits.change_bytes)?;
    let condition_len = if matches!(metadata.condition, Condition::Unconditional) {
        1
    } else {
        25
    };
    let len = frame_len(&[88, condition_len, 8, core_changes.len(), 8])?;
    check_limit(len, limits.envelope_bytes)?;
    Ok(len)
}

// One framing definition for the bounded encoder and zero-copy command hash.
fn command_parts<E>(
    metadata: CommandMetadata,
    core_changes: &[u8],
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
    part(&0_u64.to_be_bytes()) // Only empty result metadata in this packet.
}

/// Borrow an exactly framed command and compute its domain-separated binding.
/// # Errors
/// Refuses malformed/oversized framing, trailing bytes and unsupported results.
pub fn decode_command(
    bytes: &[u8],
    limits: Limits,
) -> Result<UnverifiedCommandEnvelope<'_>, FrameError> {
    let (metadata, core_changes) = parse_command_fields(bytes, limits)?;
    let digest = CommandDigest::from_bytes(blake3::derive_key(COMMAND_DIGEST_DOMAIN, bytes));
    Ok(UnverifiedCommandEnvelope {
        metadata,
        core_changes,
        result: EmptyResult,
        digest,
    })
}

fn parse_command_fields(
    bytes: &[u8],
    limits: Limits,
) -> Result<(CommandMetadata, &[u8]), FrameError> {
    let mut input = Reader::begin(bytes, COMMAND, limits)?;
    let identity = input.identity()?;
    let id = input.id()?;
    let condition = match input.tag()? {
        (_, 0) => Condition::Unconditional,
        (_, 1) => Condition::ExactState(input.state()?),
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
    validate_condition(identity, condition)?;
    let core_changes = input.span(limits.change_bytes)?;
    input.empty_result()?;
    input.end()?;
    Ok((
        CommandMetadata {
            identity,
            id,
            condition,
        },
        core_changes,
    ))
}

/// Encode outcome metadata and opaque core rejection evidence. This cannot
/// serialize `Violations` until the core owns its canonical evidence codec.
/// # Errors
/// Refuses invalid metadata, empty evidence, oversized frames and allocation.
pub fn encode_receipt(
    receipt: UnverifiedReceiptEnvelope<'_>,
    limits: Limits,
) -> Result<Vec<u8>, FrameError> {
    validate_receipt(receipt)?;
    let outcome_len = match receipt.outcome {
        UnverifiedOutcome::Committed { .. } => 25,
        UnverifiedOutcome::NoChange { .. } => 9,
        UnverifiedOutcome::PreconditionFailed { .. } => 49,
        UnverifiedOutcome::InvariantRejected { core_evidence } => {
            check_limit(core_evidence.len(), limits.evidence_bytes)?;
            core_evidence
                .len()
                .checked_add(9)
                .ok_or(FrameError::LengthOverflow)?
        }
    };
    let len = frame_len(&[120, 40, 24, outcome_len])?;
    let mut out = begin(RECEIPT, len, limits)?;
    let metadata = receipt.metadata;
    put_identity(&mut out, metadata.command.identity);
    put_id(&mut out, metadata.command.id);
    out.extend_from_slice(metadata.command.digest.as_bytes());
    put_u64(&mut out, metadata.decision_at.seq);
    out.extend_from_slice(metadata.decision_at.hash.as_bytes());
    put_state(&mut out, metadata.state_at);
    match receipt.outcome {
        UnverifiedOutcome::Committed { changed, .. } => {
            out.push(0);
            put_u64(&mut out, changed.added());
            put_u64(&mut out, changed.removed());
            put_u64(&mut out, 0);
        }
        UnverifiedOutcome::NoChange { .. } => {
            out.push(1);
            put_u64(&mut out, 0);
        }
        UnverifiedOutcome::PreconditionFailed { expected, observed } => {
            out.push(2);
            put_state(&mut out, expected);
            put_state(&mut out, observed);
        }
        UnverifiedOutcome::InvariantRejected { core_evidence } => {
            out.push(3);
            put_span(&mut out, core_evidence)?;
        }
    }
    debug_assert_eq!(out.len(), len);
    Ok(out)
}

/// Decode outcome spans without admitting a durable receipt or core evidence.
/// # Errors
/// Refuses malformed/oversized framing, trailing bytes and unsupported results.
pub fn decode_receipt(
    bytes: &[u8],
    limits: Limits,
) -> Result<UnverifiedReceiptEnvelope<'_>, FrameError> {
    let mut input = Reader::begin(bytes, RECEIPT, limits)?;
    let identity = input.identity()?;
    let id = input.id()?;
    let digest = CommandDigest::from_bytes(input.array()?);
    let decision_at = DecisionStamp {
        seq: input.u64()?,
        hash: DecisionDigest::from_bytes(input.array()?),
    };
    let state_at = input.state()?;
    let outcome = match input.tag()? {
        (_, 0) => {
            let changed = ChangeSummary::new(input.u64()?, input.u64()?)
                .ok_or(FrameError::EmptyChangeSummary)?;
            input.empty_result()?;
            UnverifiedOutcome::Committed {
                changed,
                result: EmptyResult,
            }
        }
        (_, 1) => {
            input.empty_result()?;
            UnverifiedOutcome::NoChange {
                result: EmptyResult,
            }
        }
        (_, 2) => UnverifiedOutcome::PreconditionFailed {
            expected: input.state()?,
            observed: input.state()?,
        },
        (_, 3) => UnverifiedOutcome::InvariantRejected {
            core_evidence: input.span(limits.evidence_bytes)?,
        },
        (at, got) => return Err(FrameError::Tag { at, got }),
    };
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

fn validate_condition(identity: DatabaseIdentity, condition: Condition) -> Result<(), FrameError> {
    if let Condition::ExactState(state) = condition
        && state.incarnation != identity.incarnation_id
    {
        return Err(FrameError::StateIdentityMismatch);
    }
    Ok(())
}

fn validate_receipt(receipt: UnverifiedReceiptEnvelope<'_>) -> Result<(), FrameError> {
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

fn check_limit(length: usize, cap: usize) -> Result<(), FrameError> {
    if length > cap {
        Err(FrameError::LimitExceeded)
    } else {
        Ok(())
    }
}

fn frame_len(parts: &[usize]) -> Result<usize, FrameError> {
    parts.iter().try_fold(FAMILY.len() + 3, |sum, part| {
        sum.checked_add(*part).ok_or(FrameError::LengthOverflow)
    })
}

fn begin(kind: u8, len: usize, limits: Limits) -> Result<Vec<u8>, FrameError> {
    let mut out = allocate_frame(len, limits)?;
    out.extend_from_slice(FAMILY);
    out.extend_from_slice(&LAYOUT.to_be_bytes());
    out.push(kind);
    Ok(out)
}

fn allocate_frame(len: usize, limits: Limits) -> Result<Vec<u8>, FrameError> {
    check_limit(len, limits.envelope_bytes)?;
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| FrameError::Allocation)?;
    Ok(out)
}

fn put_identity(out: &mut Vec<u8>, identity: DatabaseIdentity) {
    out.extend_from_slice(identity.database_id.as_core().as_bytes());
    out.extend_from_slice(identity.incarnation_id.as_core().as_bytes());
    out.extend_from_slice(&identity.schema_id.0);
}

fn put_id(out: &mut Vec<u8>, id: CommandId) {
    put_u64(out, id.receipt_epoch.get());
    out.extend_from_slice(id.request_id.as_core().as_bytes());
}

fn put_state(out: &mut Vec<u8>, state: StateStamp) {
    out.extend_from_slice(state.incarnation.as_core().as_bytes());
    put_u64(out, state.data_revision);
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_span(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), FrameError> {
    put_u64(
        out,
        u64::try_from(bytes.len()).map_err(|_| FrameError::LengthOverflow)?,
    );
    out.extend_from_slice(bytes);
    Ok(())
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn begin(bytes: &'a [u8], kind: u8, limits: Limits) -> Result<Self, FrameError> {
        check_limit(bytes.len(), limits.envelope_bytes)?;
        let mut input = Self { bytes, at: 0 };
        if input.take(FAMILY.len())? != FAMILY {
            return Err(FrameError::Family);
        }
        let version = u16::from_be_bytes(input.array()?);
        if version != LAYOUT {
            return Err(FrameError::Layout { got: version });
        }
        let (_, got) = input.tag()?;
        if got != kind {
            return Err(FrameError::Kind { got });
        }
        Ok(input)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], FrameError> {
        let end = self.at.checked_add(len).ok_or(FrameError::LengthOverflow)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(FrameError::Truncated { at: self.at })?;
        self.at = end;
        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], FrameError> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    fn u64(&mut self) -> Result<u64, FrameError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn tag(&mut self) -> Result<(usize, u8), FrameError> {
        let at = self.at;
        Ok((at, self.array::<1>()?[0]))
    }

    fn identity(&mut self) -> Result<DatabaseIdentity, FrameError> {
        Ok(DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes(self.array()?)),
            incarnation_id: IncarnationId::from_core(Id128::from_bytes(self.array()?)),
            schema_id: SchemaId(self.array()?),
        })
    }

    fn id(&mut self) -> Result<CommandId, FrameError> {
        Ok(CommandId {
            receipt_epoch: ReceiptEpoch::new(self.u64()?).ok_or(FrameError::InvalidEpoch)?,
            request_id: RequestId::from_core(Id128::from_bytes(self.array()?)),
        })
    }

    fn state(&mut self) -> Result<StateStamp, FrameError> {
        Ok(StateStamp {
            incarnation: IncarnationId::from_core(Id128::from_bytes(self.array()?)),
            data_revision: self.u64()?,
        })
    }

    fn span(&mut self, cap: usize) -> Result<&'a [u8], FrameError> {
        let len = usize::try_from(self.u64()?).map_err(|_| FrameError::LengthOverflow)?;
        check_limit(len, cap)?;
        self.take(len)
    }

    fn empty_result(&mut self) -> Result<(), FrameError> {
        if self.u64()? == 0 {
            Ok(())
        } else {
            Err(FrameError::NonemptyResultUnsupported)
        }
    }

    fn end(self) -> Result<(), FrameError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(FrameError::TrailingBytes { at: self.at })
        }
    }
}
