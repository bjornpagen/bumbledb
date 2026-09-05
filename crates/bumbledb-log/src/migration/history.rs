//! Authoritative migration history records (C11).
//!
//! The generated manifest defines the ordered chain; the DATABASE records
//! what actually happened. One execution of a contiguous pending suffix is
//! ONE `Applied` batch record — never a fictitious published database per
//! plan file. Records are host rows in the authoritative LMDB (committed
//! transactionally with facts, receipts and the head attachment; hosted
//! checkpoints carry them in closure), keyed under `b'm'` — a namespace the
//! receipt-retirement frontier (`b'r'`) can never touch. Flattening the
//! ordered steps of the chain verifies the exact applied manifest prefix.

use crate::history::{
    DatabaseId, DecisionStamp, FrameError, IncarnationId, OperationId, SchemaId, StateStamp,
};

use super::frame::{self, KIND_APPLIED, KIND_BASELINE, Reader, SYSTEM_DIGEST_DOMAIN, keyed_digest};
use super::manifest::{Manifest, ManifestError};
use super::plan::{PlanError, StepLabel};

/// Host-record key prefix for migration history rows. Distinct from the
/// receipt prefix so retirement is structurally unable to erase history.
pub const HISTORY_KEY_PREFIX: u8 = b'm';
pub const HISTORY_KEY_LEN: usize = 9;

/// The chain key of record `index` (0-based, contiguous).
#[must_use]
pub fn history_key(index: u64) -> [u8; HISTORY_KEY_LEN] {
    let mut key = [0; HISTORY_KEY_LEN];
    key[0] = HISTORY_KEY_PREFIX;
    key[1..].copy_from_slice(&index.to_be_bytes());
    key
}

/// One flattened applied step: the identity of one manifest entry that a
/// batch actually executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedStep {
    pub sequence: u64,
    pub label: StepLabel,
    pub from_schema: SchemaId,
    pub to_schema: SchemaId,
    pub plan_digest: [u8; 32],
}

/// What one applied batch executed FROM: the declared empty base (explicit
/// initialization, seeds actually ran) or one captured frozen database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliedSource {
    EmptyBase {
        base_schema: SchemaId,
    },
    Database {
        database: DatabaseId,
        incarnation: IncarnationId,
        schema: SchemaId,
        decision: DecisionStamp,
        state: StateStamp,
    },
}

/// One executed contiguous suffix: original captured source, one final
/// published target, ordered logical steps. Intermediate step schemas are
/// logical boundaries, never incarnations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    pub operation: OperationId,
    pub plan_set_digest: [u8; 32],
    pub source: AppliedSource,
    pub target_incarnation: IncarnationId,
    pub target_schema: SchemaId,
    /// Canonical application-state digest of the final target — excludes
    /// the history record and genesis themselves (acyclic).
    pub target_digest: [u8; 32],
    pub steps: Vec<AppliedStep>,
}

/// Explicit adoption of an already-validated state at a manifest prefix.
/// Visibly different from applying the plans; seeds were NOT executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baseline {
    pub operation: OperationId,
    /// Number of manifest entries this baseline claims (prefix length).
    pub steps_through: u64,
    /// The verified manifest prefix digest at `steps_through`.
    pub validated_prefix: [u8; 32],
    pub target_schema: SchemaId,
    pub target_digest: [u8; 32],
    /// The operator's explicit bounded reason.
    pub reason: Box<str>,
}

/// One chain record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryRecord {
    Applied(Applied),
    Baseline(Baseline),
}

/// Why a stored chain refused. Chain errors are corruption/drift evidence,
/// never permission to guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryError {
    Frame(FrameError),
    Shape(&'static str),
    /// A Baseline record appears anywhere but chain index 0.
    BaselineNotFirst,
    /// The flattened steps do not extend the manifest prefix contiguously.
    NotContiguous {
        at: u64,
    },
    /// A flattened step disagrees with its manifest entry.
    StepMismatch {
        at: u64,
    },
    /// The database chain claims more steps than the manifest records.
    DatabaseAhead {
        recorded: u64,
        manifest: u64,
    },
    /// A baseline's validated prefix does not recompute against the manifest.
    BaselinePrefixMismatch,
    Manifest(ManifestError),
}

impl From<FrameError> for HistoryError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<PlanError> for HistoryError {
    fn from(error: PlanError) -> Self {
        match error {
            PlanError::Json(why) | PlanError::Shape(why) => Self::Shape(why),
            PlanError::Frame(frame) => Self::Frame(frame),
        }
    }
}

impl From<ManifestError> for HistoryError {
    fn from(error: ManifestError) -> Self {
        Self::Manifest(error)
    }
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "migration history: {self:?}")
    }
}

impl std::error::Error for HistoryError {}

/// Encode one chain record.
/// # Errors
/// Refuses oversized frames and allocation failure.
pub fn encode_record(record: &HistoryRecord, cap: usize) -> Result<Vec<u8>, FrameError> {
    match record {
        HistoryRecord::Applied(applied) => {
            let mut out = frame::begin(KIND_APPLIED, cap)?;
            out.bytes(applied.operation.as_core().as_bytes())?;
            out.bytes(&applied.plan_set_digest)?;
            match &applied.source {
                AppliedSource::EmptyBase { base_schema } => {
                    out.tag(0)?;
                    out.bytes(&base_schema.0)?;
                }
                AppliedSource::Database {
                    database,
                    incarnation,
                    schema,
                    decision,
                    state,
                } => {
                    out.tag(1)?;
                    out.bytes(database.as_core().as_bytes())?;
                    out.bytes(incarnation.as_core().as_bytes())?;
                    out.bytes(&schema.0)?;
                    out.u64(decision.seq)?;
                    out.bytes(decision.hash.as_bytes())?;
                    out.bytes(state.incarnation.as_core().as_bytes())?;
                    out.u64(state.data_revision)?;
                }
            }
            out.bytes(applied.target_incarnation.as_core().as_bytes())?;
            out.bytes(&applied.target_schema.0)?;
            out.bytes(&applied.target_digest)?;
            out.u64(applied.steps.len() as u64)?;
            for step in &applied.steps {
                out.u64(step.sequence)?;
                out.span(step.label.as_str().as_bytes())?;
                out.bytes(&step.from_schema.0)?;
                out.bytes(&step.to_schema.0)?;
                out.bytes(&step.plan_digest)?;
            }
            Ok(out.finish())
        }
        HistoryRecord::Baseline(baseline) => {
            let mut out = frame::begin(KIND_BASELINE, cap)?;
            out.bytes(baseline.operation.as_core().as_bytes())?;
            out.u64(baseline.steps_through)?;
            out.bytes(&baseline.validated_prefix)?;
            out.bytes(&baseline.target_schema.0)?;
            out.bytes(&baseline.target_digest)?;
            out.span(baseline.reason.as_bytes())?;
            Ok(out.finish())
        }
    }
}

/// Decode one chain record (either kind).
/// # Errors
/// Refuses malformed frames and trailing bytes.
pub fn decode_record(bytes: &[u8], cap: usize) -> Result<HistoryRecord, HistoryError> {
    // Dispatch on the frame kind byte so a malformed Applied frame reports
    // its own refusal instead of a misleading Baseline grammar error.
    let kind_at = frame::FAMILY.len() + 2;
    match bytes.get(kind_at) {
        Some(&KIND_APPLIED) => decode_applied(bytes, cap).map(HistoryRecord::Applied),
        Some(&KIND_BASELINE) => decode_baseline(bytes, cap).map(HistoryRecord::Baseline),
        Some(&got) => Err(HistoryError::Frame(FrameError::Kind { got })),
        None => Err(HistoryError::Frame(FrameError::Truncated { at: kind_at })),
    }
}

fn decode_applied(bytes: &[u8], cap: usize) -> Result<Applied, HistoryError> {
    let mut input = Reader::begin(bytes, KIND_APPLIED, cap)?;
    let operation = OperationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
    let plan_set_digest = input.array()?;
    let source = match input.tag()? {
        (_, 0) => AppliedSource::EmptyBase {
            base_schema: SchemaId(input.array()?),
        },
        (_, 1) => AppliedSource::Database {
            database: DatabaseId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
            incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
            schema: SchemaId(input.array()?),
            decision: DecisionStamp {
                seq: input.u64()?,
                hash: crate::history::DecisionDigest::from_bytes(input.array()?),
            },
            state: StateStamp {
                incarnation: IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?)),
                data_revision: input.u64()?,
            },
        },
        (at, got) => return Err(HistoryError::Frame(FrameError::Tag { at, got })),
    };
    let target_incarnation = IncarnationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
    let target_schema = SchemaId(input.array()?);
    let target_digest = input.array()?;
    let step_count = input.count(112)?;
    let mut steps = Vec::new();
    steps
        .try_reserve_exact(step_count)
        .map_err(|_| HistoryError::Frame(FrameError::Allocation))?;
    for _ in 0..step_count {
        let sequence = input.u64()?;
        let label = StepLabel::new(
            std::str::from_utf8(input.span(cap)?).map_err(|_| HistoryError::Shape("label utf8"))?,
        )?;
        steps.push(AppliedStep {
            sequence,
            label,
            from_schema: SchemaId(input.array()?),
            to_schema: SchemaId(input.array()?),
            plan_digest: input.array()?,
        });
    }
    input.end()?;
    if steps.is_empty() {
        return Err(HistoryError::Shape("applied batch without steps"));
    }
    Ok(Applied {
        operation,
        plan_set_digest,
        source,
        target_incarnation,
        target_schema,
        target_digest,
        steps,
    })
}

fn decode_baseline(bytes: &[u8], cap: usize) -> Result<Baseline, HistoryError> {
    let mut input = Reader::begin(bytes, KIND_BASELINE, cap)?;
    let operation = OperationId::from_core(bumbledb::Id128::from_bytes(input.array()?));
    let steps_through = input.u64()?;
    let validated_prefix = input.array()?;
    let target_schema = SchemaId(input.array()?);
    let target_digest = input.array()?;
    let reason: Box<str> = std::str::from_utf8(input.span(cap)?)
        .map_err(|_| HistoryError::Shape("reason utf8"))?
        .into();
    input.end()?;
    if reason.is_empty() {
        return Err(HistoryError::Shape("baseline without explicit reason"));
    }
    Ok(Baseline {
        operation,
        steps_through,
        validated_prefix,
        target_schema,
        target_digest,
        reason,
    })
}

/// The migration-system digest: the framed chain in order. The migration
/// target genesis binds this as its initial SYSTEM digest, so inherited
/// history plus the new Applied record are part of the genesis sentinel.
/// # Errors
/// Refuses oversized frames and allocation failure.
pub fn system_digest(chain: &[HistoryRecord], cap: usize) -> Result<[u8; 32], FrameError> {
    let mut preimage = Vec::new();
    for record in chain {
        let bytes = encode_record(record, cap)?;
        preimage
            .try_reserve(bytes.len() + 8)
            .map_err(|_| FrameError::Allocation)?;
        preimage.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        preimage.extend_from_slice(&bytes);
    }
    Ok(keyed_digest(SYSTEM_DIGEST_DOMAIN, &preimage))
}

/// The verified flattened position of a chain against a VERIFIED manifest:
/// how many manifest entries are actually applied, with every step checked
/// against its entry and every baseline prefix recomputed.
/// Returns the applied prefix length.
/// # Errors
/// Chain/manifest disagreement with the exact position.
pub fn verify_chain(
    chain: &[HistoryRecord],
    manifest: &Manifest,
    cap: usize,
) -> Result<u64, HistoryError> {
    let manifest_len = manifest.entries.len() as u64;
    let mut position: u64 = 0;
    for (index, record) in chain.iter().enumerate() {
        match record {
            HistoryRecord::Baseline(baseline) => {
                if index != 0 {
                    return Err(HistoryError::BaselineNotFirst);
                }
                if baseline.steps_through > manifest_len {
                    return Err(HistoryError::DatabaseAhead {
                        recorded: baseline.steps_through,
                        manifest: manifest_len,
                    });
                }
                let through = usize::try_from(baseline.steps_through)
                    .map_err(|_| HistoryError::Shape("baseline width"))?;
                let expected = super::manifest::prefix_at(manifest, through, cap)?;
                if expected != baseline.validated_prefix {
                    return Err(HistoryError::BaselinePrefixMismatch);
                }
                position = baseline.steps_through;
            }
            HistoryRecord::Applied(applied) => {
                for step in &applied.steps {
                    if step.sequence != position {
                        return Err(HistoryError::NotContiguous { at: step.sequence });
                    }
                    if position >= manifest_len {
                        return Err(HistoryError::DatabaseAhead {
                            recorded: position + 1,
                            manifest: manifest_len,
                        });
                    }
                    let entry = &manifest.entries[usize::try_from(position)
                        .map_err(|_| HistoryError::Shape("position width"))?];
                    if entry.sequence != step.sequence
                        || entry.label != step.label
                        || entry.from_schema != step.from_schema
                        || entry.to_schema != step.to_schema
                        || entry.plan_digest != step.plan_digest
                    {
                        return Err(HistoryError::StepMismatch { at: step.sequence });
                    }
                    position += 1;
                }
            }
        }
    }
    Ok(position)
}
