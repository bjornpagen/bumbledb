//! Coherent streamed checkpoints with a bounded validated suffix rebase
//! (C08; REP-014, PERF-004, STORE-03/04/07).
//!
//! One owned core read snapshot supplies facts, retained system rows, the
//! captured control projection and every digest — never "copy now, ask
//! generation later". The source keeps accepting commits while the snapshot
//! exports. A moved head afterwards causes a bounded head rebase plus tail
//! validation, not a re-export, and never a quiet-window restart.
//!
//! Let the captured certificate describe decision S, the current head T with
//! current recovery base B: publication requires `B ≤ S ≤ T` on one lineage,
//! preserves exactly the decisions `(S, T]` in the new recovery root, keeps
//! all roots/receipt policy/mode/GC state from the exact current head, and
//! discards the candidate when another checkpoint already passed S (the
//! recovery base never moves backwards). If the object epoch moved during
//! export, staged chunks are restaged under the current epoch — relabeling a
//! manifest without restoring its child objects is insufficient.

use bumbledb::integration::HostSealError;
use bumbledb::{Db, WorkContext, WorkError};

use crate::codec::{
    self, CheckpointManifest, ChunkSink, StreamLimits, StreamSummary, StreamWriter, WriteError,
};
use crate::history::authority::{HeadAuthority, decode_control};
use crate::history::command::Limits;
use crate::history::decision::{self};
use crate::history::receipt::RECEIPT_KEY_PREFIX;
use crate::history::{DecisionStamp, FrameError, HeadRevision};
use crate::manifest::{HeadError, HeadRecord, RecoveryRoot, TailPolicy, decode_head, encode_head};
use crate::store::{
    BackendError, ConditionalOutcome, ConditionalStore, HeadRead, HeadVersion, ObjectError,
    ObjectKind, ObjectRef, fetch_decision, get_verified, head_key, put_verified,
};

/// The migration-history host-record key prefix (C08/C11 coordination:
/// P09 stores its authoritative applied/baseline evidence under keys
/// beginning with this byte so checkpoints and digests carry it).
pub const HISTORY_KEY_PREFIX: u8 = b'm';

/// Deployment-qualified checkpoint policy. The chunk size and envelope are
/// measured policy, not correctness constants (chapter 21).
#[derive(Debug, Clone, Copy)]
pub struct CheckpointPolicy {
    pub chunk_bytes: usize,
    pub stream: StreamLimits,
    /// Frame cap for control/head bodies.
    pub head_cap: usize,
    pub tail: TailPolicy,
    pub rebase_attempts: u32,
    /// Bounded probe window / walk budget while validating the suffix.
    pub suffix_budget: u64,
}

impl CheckpointPolicy {
    pub const DEFAULT: Self = Self {
        chunk_bytes: codec::CHUNK_TARGET,
        stream: StreamLimits::DEFAULT,
        head_cap: 1024 * 1024,
        tail: TailPolicy::UNBOUNDED,
        rebase_attempts: 16,
        suffix_budget: 65_536,
    };
}

#[derive(Debug)]
pub enum CheckpointError {
    Object(ObjectError),
    Frame(FrameError),
    Head(HeadError),
    Storage(bumbledb::Error),
    HostSeal(HostSealError),
    Work(WorkError),
    /// The local materialization carries no authority attachment.
    NotInitialized,
    /// The head is a terminal tombstone.
    Deleted,
    /// The captured snapshot or walked chain disagrees with the head lineage.
    Corruption(&'static str),
    /// Bounded rebase attempts were exhausted by contention.
    RebaseExhausted,
    /// The publication CAS outcome could not be established; the uploaded
    /// candidate objects remain orphans for a later collection.
    Unresolved,
}

impl From<ObjectError> for CheckpointError {
    fn from(error: ObjectError) -> Self {
        Self::Object(error)
    }
}

impl From<FrameError> for CheckpointError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<HeadError> for CheckpointError {
    fn from(error: HeadError) -> Self {
        Self::Head(error)
    }
}

impl From<bumbledb::Error> for CheckpointError {
    fn from(error: bumbledb::Error) -> Self {
        Self::Storage(error)
    }
}

impl From<WorkError> for CheckpointError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<HostSealError> for CheckpointError {
    fn from(error: HostSealError) -> Self {
        Self::HostSeal(error)
    }
}

#[derive(Debug)]
pub enum CheckpointOutcome {
    Published {
        manifest: ObjectRef,
        base: DecisionStamp,
        tip: DecisionStamp,
        head_revision: HeadRevision,
    },
    /// Another checkpoint already passed the captured decision; the recovery
    /// base never moves backwards. The uploaded candidate becomes orphan
    /// objects for a later collection.
    Discarded { current_base_seq: u64 },
}

/// One coherent capture: the authority projection and stream summary all
/// read from a single committed LMDB snapshot.
pub struct Captured {
    pub authority: HeadAuthority,
    pub summary: StreamSummary,
}

/// Admission headroom against the configured tail envelope: the writer-side
/// backpressure consult (C08).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Headroom {
    Ok,
    /// Start checkpointing: measured headroom is nearly consumed.
    StartCheckpoint,
    /// Admission would exceed the envelope; refuse with backpressure until a
    /// checkpoint advances. Includes no-op/rejection decisions.
    MaintenanceRequired,
}

#[must_use]
pub fn admission_headroom(recovery: &RecoveryRoot, policy: &TailPolicy) -> Headroom {
    let count = recovery.tail_count();
    if count >= policy.max_count || recovery.tail_bytes >= policy.max_bytes {
        return Headroom::MaintenanceRequired;
    }
    // Reserve measured headroom: start maintenance at three quarters.
    if count.saturating_mul(4) >= policy.max_count.saturating_mul(3)
        || recovery.tail_bytes.saturating_mul(4) >= policy.max_bytes.saturating_mul(3)
    {
        return Headroom::StartCheckpoint;
    }
    Headroom::Ok
}

enum CaptureFailure<E> {
    Sink(E),
    Frame(FrameError),
    Host(HostSealError),
    Work(WorkError),
    NotInitialized,
    RecordTooLarge,
    OutOfOrder,
}

impl<E> From<WriteError<E>> for CaptureFailure<E> {
    fn from(error: WriteError<E>) -> Self {
        match error {
            WriteError::Sink(error) => Self::Sink(error),
            WriteError::RecordTooLarge { .. } => Self::RecordTooLarge,
            WriteError::OutOfOrder => Self::OutOfOrder,
        }
    }
}

/// Export one coherent logical stream from the transitional core store into
/// `sink`: canonical facts (relation ascending, canonical bytes ascending
/// within a relation), then keyed system records (`m` history, then `r`
/// receipts, ascending — excluding rows at or below `retired_filter`).
///
/// System-record enumeration reads through the landed P02R core seam
/// `ReadInstance::integration_host_scan` (requested by
/// implementation/packets/P05.md; also used by `admin::retired_row_keys`).
///
/// Closed-extension relations are schema constants, re-established by the
/// schema on import, and are not exported as stored facts.
///
/// # Errors
/// Sink, storage, frame and work refusals; nothing partial is returned.
pub fn capture_into<S, K: ChunkSink>(
    db: &Db<S>,
    sink: &mut K,
    retired_filter: u64,
    policy: &CheckpointPolicy,
    work: &WorkContext,
) -> Result<Captured, CheckpointError>
where
    K::Error: Into<CheckpointError>,
{
    let schema = db.schema();
    let mut failure: Option<CaptureFailure<K::Error>> = None;
    let mut captured: Option<Captured> = None;
    let storage = db.read(|read| {
        let mut run = || -> Result<Captured, CaptureFailure<K::Error>> {
            let control = read
                .integration_host_attachment()
                .map_err(|error| CaptureFailure::Host(HostSealError::Storage(error)))?
                .ok_or(CaptureFailure::NotInitialized)?;
            let authority =
                decode_control(control, policy.head_cap).map_err(CaptureFailure::Frame)?;
            let mut writer = StreamWriter::new(sink, policy.chunk_bytes, policy.stream);
            for (index, relation) in schema.relations().iter().enumerate() {
                if relation.body().closed_rows().is_some() {
                    continue;
                }
                let relation_id = bumbledb::schema::RelationId(
                    u32::try_from(index).map_err(|_| CaptureFailure::RecordTooLarge)?,
                );
                // Canonical order requires a per-relation sort: the
                // transitional scan yields storage order. Charged before
                // growth; the successor store's snapshot export supplies
                // bounded canonical order natively (recorded cost boundary).
                let mut rows: Vec<Vec<u8>> = Vec::new();
                for entry in read
                    .scan(relation_id)
                    .map_err(|error| CaptureFailure::Host(HostSealError::Storage(error)))?
                {
                    let values = entry
                        .map_err(|error| CaptureFailure::Host(HostSealError::Storage(error)))?;
                    let row =
                        bumbledb::canonical::CanonicalRow::encode(relation.fields(), &values, work)
                            .map_err(|_| CaptureFailure::RecordTooLarge)?;
                    work.step(1).map_err(CaptureFailure::Work)?;
                    rows.push(row.as_bytes().to_vec());
                }
                rows.sort_unstable();
                for row in rows {
                    writer.fact(relation_id.0, &row)?;
                }
            }
            // System records in ascending key order: 'm' (0x6d) < 'r' (0x72).
            for prefix in [HISTORY_KEY_PREFIX, RECEIPT_KEY_PREFIX] {
                let mut sink_error: Option<WriteError<K::Error>> = None;
                read.integration_host_scan(&[prefix], &mut |key: &[u8], value: &[u8]| {
                    if prefix == RECEIPT_KEY_PREFIX
                        && key.len() >= 9
                        && u64::from_be_bytes(key[1..9].try_into().expect("width"))
                            <= retired_filter
                    {
                        return Ok(());
                    }
                    if let Err(error) = writer.system(key, value) {
                        sink_error = Some(error);
                        return Err(HostSealError::Work(WorkError::Cancelled));
                    }
                    Ok(())
                })
                .or_else(|error| {
                    if sink_error.is_some() {
                        Ok(())
                    } else {
                        Err(CaptureFailure::Host(error))
                    }
                })?;
                if let Some(error) = sink_error {
                    return Err(error.into());
                }
            }
            let summary = writer.finish()?;
            Ok(Captured { authority, summary })
        };
        match run() {
            Ok(value) => captured = Some(value),
            Err(error) => failure = Some(error),
        }
        Ok(())
    });
    if let Some(failure) = failure {
        return Err(match failure {
            CaptureFailure::Sink(error) => error.into(),
            CaptureFailure::Frame(error) => CheckpointError::Frame(error),
            CaptureFailure::Host(error) => CheckpointError::HostSeal(error),
            CaptureFailure::Work(error) => CheckpointError::Work(error),
            CaptureFailure::NotInitialized => CheckpointError::NotInitialized,
            CaptureFailure::RecordTooLarge => CheckpointError::Frame(FrameError::LimitExceeded),
            CaptureFailure::OutOfOrder => CheckpointError::Corruption("stream section order"),
        });
    }
    storage?;
    captured.ok_or(CheckpointError::Corruption("capture returned nothing"))
}

struct UploadSink<'a, B> {
    backend: &'a B,
    prefix: &'a str,
    epoch: u64,
}

impl<B: ConditionalStore> ChunkSink for UploadSink<'_, B>
where
    B::Error: BackendError,
{
    type Error = CheckpointError;

    fn chunk(&mut self, bytes: &[u8]) -> Result<ObjectRef, CheckpointError> {
        Ok(put_verified(
            self.backend,
            self.prefix,
            self.epoch,
            ObjectKind::Chunk,
            bytes,
        )?)
    }
}

/// The suffix walk: verify the exact decisions `(base, tip]` chain back from
/// `tip` to `base`, counting bytes and the oldest epoch touched.
struct Suffix {
    tail_bytes: u64,
    epoch_floor: u64,
}

#[expect(clippy::too_many_arguments, reason = "one bounded suffix walk")]
fn validate_suffix<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    base: DecisionStamp,
    tip: DecisionStamp,
    epoch_floor: u64,
    epoch_ceiling: u64,
    limits: Limits,
    budget: u64,
    work: &WorkContext,
) -> Result<Suffix, CheckpointError>
where
    B::Error: BackendError,
{
    let mut cursor = tip;
    let mut tail_bytes = 0u64;
    let mut floor = epoch_ceiling;
    let mut steps = 0u64;
    while cursor != base {
        work.checkpoint()?;
        steps += 1;
        if steps > budget || cursor.seq == 0 || cursor.seq <= base.seq {
            return Err(CheckpointError::Corruption(
                "suffix walk did not reach the captured base",
            ));
        }
        let (epoch, bytes) =
            fetch_decision(backend, prefix, epoch_floor, epoch_ceiling, &cursor.hash)?;
        let envelope = codec_decision(&bytes, limits)?;
        if envelope.stamp() != cursor {
            return Err(CheckpointError::Corruption("decision digest mismatch"));
        }
        tail_bytes = tail_bytes.saturating_add(bytes.len() as u64);
        floor = floor.min(epoch);
        cursor = envelope.parent;
    }
    Ok(Suffix {
        tail_bytes,
        epoch_floor: floor,
    })
}

fn codec_decision(
    bytes: &[u8],
    limits: Limits,
) -> Result<decision::UnverifiedDecisionEnvelope<'_>, CheckpointError> {
    decision::decode_decision(bytes, limits).map_err(CheckpointError::Frame)
}

/// The head-control transition a rebase installs beside the new recovery
/// root: plain maintenance for an ordinary checkpoint, or receipt
/// retirement (which must advance atomically with the checkpoint that no
/// longer promises the retired rows).
#[derive(Debug, Clone, Copy)]
pub enum CheckpointKind {
    Ordinary,
    RetireReceipts { through: u64 },
}

/// Publish one streamed checkpoint of the local materialization onto the
/// hosted head. The export runs once; only the bounded head rebase repeats
/// under contention. Continuous writes never force a re-export.
///
/// # Errors
/// Typed backend/frame/storage refusals; `RebaseExhausted` under unbounded
/// contention; `Unresolved` when a dispatched CAS could not be proven.
#[expect(
    clippy::too_many_lines,
    reason = "one bounded publish pipeline: capture, stage, CAS, rebase"
)]
pub fn publish_checkpoint<S, B: ConditionalStore>(
    db: &Db<S>,
    backend: &B,
    prefix: &str,
    command_limits: Limits,
    kind: CheckpointKind,
    policy: &CheckpointPolicy,
    work: &WorkContext,
) -> Result<CheckpointOutcome, CheckpointError>
where
    B::Error: BackendError,
{
    // 1. The exact parent head names the staging epoch and lineage.
    let (parent, _) = read_live_head(backend, prefix, policy.head_cap)?;
    let mut staged_epoch = parent.object_epoch;
    let retired_filter = match kind {
        CheckpointKind::Ordinary => parent
            .control
            .live()
            .map_err(|_| CheckpointError::Deleted)?
            .receipts
            .retired_through(),
        CheckpointKind::RetireReceipts { through } => through,
    };

    // 2. One coherent capture, streamed straight into verified chunk uploads.
    let mut sink = UploadSink {
        backend,
        prefix,
        epoch: staged_epoch,
    };
    let captured = capture_into(db, &mut sink, retired_filter, policy, work)?;
    if captured.authority.identity != parent.control.identity {
        return Err(CheckpointError::Corruption("captured foreign identity"));
    }
    let capture_position = captured
        .authority
        .position()
        .ok_or(CheckpointError::Deleted)?;
    let base = capture_position.decision;

    // 3. The streamed manifest.
    let mut chunks = captured.summary.chunks.clone();
    let mut manifest = CheckpointManifest {
        identity: captured.authority.identity,
        decision: base,
        state: capture_position.state,
        control_at_capture: captured.authority,
        application_digest: captured.summary.application_digest,
        system_digest: captured.summary.system_digest,
        stream_digest: captured.summary.stream_digest,
        total_bytes: captured.summary.total_bytes,
        rows: captured.summary.rows,
        system_records: captured.summary.system_records,
        chunks: chunks.clone(),
    };
    let mut manifest_ref = put_verified(
        backend,
        prefix,
        staged_epoch,
        ObjectKind::Checkpoint,
        &codec::encode_manifest(&manifest, policy.stream)?,
    )?;

    // 4. Bounded rebase against the moving head.
    for _ in 0..policy.rebase_attempts {
        work.checkpoint()?;
        let (current, version) = read_live_head(backend, prefix, policy.head_cap)?;
        if current.control.identity != manifest.identity {
            return Err(CheckpointError::Corruption("head lineage changed"));
        }
        let live = current
            .control
            .live()
            .map_err(|_| CheckpointError::Deleted)?;
        let current_recovery = current.recovery.ok_or(CheckpointError::Corruption(
            "live head without recovery root",
        ))?;
        // The recovery base never moves backwards; equality causes no
        // pointless republication.
        if current_recovery.checkpoint.is_some() && base.seq <= current_recovery.base.seq {
            return Ok(CheckpointOutcome::Discarded {
                current_base_seq: current_recovery.base.seq,
            });
        }
        if base.seq > live.decision.seq {
            return Err(CheckpointError::Corruption(
                "captured snapshot is ahead of the head",
            ));
        }
        // Validate exactly the retained suffix (S, T].
        let suffix = validate_suffix(
            backend,
            prefix,
            base,
            live.decision,
            current_recovery.epoch_floor,
            current.object_epoch,
            command_limits,
            policy.suffix_budget,
            work,
        )?;
        // New dependencies staged under a now-closed epoch must be restaged
        // under the current epoch (chapter 21).
        if staged_epoch != current.object_epoch {
            let new_epoch = current.object_epoch;
            let mut restaged = Vec::with_capacity(chunks.len());
            for old in &chunks {
                let bytes = get_verified(backend, prefix, old)?;
                restaged.push(put_verified(
                    backend,
                    prefix,
                    new_epoch,
                    ObjectKind::Chunk,
                    &bytes,
                )?);
            }
            chunks = restaged;
            manifest.chunks.clone_from(&chunks);
            manifest_ref = put_verified(
                backend,
                prefix,
                new_epoch,
                ObjectKind::Checkpoint,
                &codec::encode_manifest(&manifest, policy.stream)?,
            )?;
            staged_epoch = new_epoch;
        }
        let recovery = RecoveryRoot {
            checkpoint: Some(manifest_ref),
            base,
            tip: live.decision,
            tail_bytes: suffix.tail_bytes,
            epoch_floor: if live.decision == base {
                current.object_epoch
            } else {
                suffix.epoch_floor
            },
        };
        let control = match kind {
            CheckpointKind::Ordinary => current.control.maintained().map_err(HeadError::from)?,
            CheckpointKind::RetireReceipts { through } => current
                .control
                .retire_receipts(through)
                .map_err(HeadError::from)?,
        };
        let proposed = HeadRecord {
            control,
            recovery: Some(recovery),
            roots: current.roots.clone(),
            gc: current.gc.clone(),
            object_epoch: current.object_epoch,
        };
        let body = encode_head(&proposed, policy.head_cap)?;
        match backend
            .replace_head(&head_key(prefix), &version, &body)
            .map_err(crate::store::backend)
            .map_err(CheckpointError::Object)?
        {
            ConditionalOutcome::Published { .. } => {
                return Ok(CheckpointOutcome::Published {
                    manifest: manifest_ref,
                    base,
                    tip: live.decision,
                    head_revision: proposed.control.revision,
                });
            }
            ConditionalOutcome::PreconditionFailed => {}
            ConditionalOutcome::Indeterminate => {
                // Resolve by reading the head: the successful atomic
                // replacement is the linearization point, so our exact
                // manifest in the current recovery root proves publication.
                let (observed, _) = read_live_head(backend, prefix, policy.head_cap)?;
                match observed.recovery.and_then(|root| root.checkpoint) {
                    Some(reference) if reference == manifest_ref => {
                        return Ok(CheckpointOutcome::Published {
                            manifest: manifest_ref,
                            base,
                            tip: live.decision,
                            head_revision: observed.control.revision,
                        });
                    }
                    _ if observed.control.revision == current.control.revision => {
                        // Nothing landed; retry against the same head.
                    }
                    _ => return Err(CheckpointError::Unresolved),
                }
            }
        }
    }
    Err(CheckpointError::RebaseExhausted)
}

/// Read and decode the current composed head. `NotInitialized` for a missing
/// head — reading never initializes.
///
/// # Errors
/// Backend and frame refusals.
pub fn read_live_head<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    cap: usize,
) -> Result<(HeadRecord, HeadVersion), CheckpointError>
where
    B::Error: BackendError,
{
    match backend
        .read_head(&head_key(prefix))
        .map_err(crate::store::backend)
        .map_err(CheckpointError::Object)?
    {
        HeadRead::Present { version, body } => {
            let record = decode_head(&body, cap)?;
            Ok((record, version))
        }
        HeadRead::Absent => Err(CheckpointError::NotInitialized),
    }
}
