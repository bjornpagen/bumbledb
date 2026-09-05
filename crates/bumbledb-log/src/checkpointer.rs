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
use crate::history::locator::ChainVisitor;
use crate::history::receipt::RECEIPT_KEY_PREFIX;
use crate::history::{DecisionStamp, FrameError, HeadRevision};
use crate::manifest::{HeadError, HeadRecord, RecoveryRoot, TailPolicy, decode_head, encode_head};
use crate::store::{
    BackendError, ConditionalOutcome, HeadVersion, ObjectError, ObjectKind, ObjectRef,
    ObservedError, ReceiveLimits, ReceivedHead, ReceivingStore, TransportContext, get_verified,
    head_key, put_verified, read_head_bounded,
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
    // Reserve measured headroom: start maintenance at three quarters, and
    // always once a single admission remains — a tiny envelope (max_count 2)
    // must warn before the cliff, not jump from Ok to MaintenanceRequired.
    if count.saturating_add(1) >= policy.max_count
        || count.saturating_mul(4) >= policy.max_count.saturating_mul(3)
        || recovery.tail_bytes.saturating_mul(4) >= policy.max_bytes.saturating_mul(3)
    {
        return Headroom::StartCheckpoint;
    }
    Headroom::Ok
}

fn write_failure<E: Into<CheckpointError>>(error: WriteError<E>) -> CheckpointError {
    match error {
        WriteError::Sink(error) => error.into(),
        WriteError::RecordTooLarge { .. } => CheckpointError::Frame(FrameError::LimitExceeded),
        WriteError::OutOfOrder => CheckpointError::Corruption("stream section order"),
    }
}

fn store_failure(error: bumbledb::store::StoreError) -> CheckpointError {
    CheckpointError::HostSeal(error.into())
}

/// Export one coherent logical stream from ONE owned store snapshot into
/// `sink`: the store's canonical logical export of the facts (relation
/// ascending, then tuple fingerprint, then full canonical bytes within a
/// collision bucket — a deterministic function of the logical state, in
/// bounded memory; `OwnedSnapshot::export`), then keyed system records
/// (`m` history, then `r` receipts, ascending — excluding rows at or below
/// `retired_filter`). The control projection, every fact and every system
/// record derive from the same committed transaction.
///
/// Bounded by construction: no whole-relation buffering and no in-RAM sort
/// (audit-log #5; STORE-01/PERF-004). Closed-extension relations are schema
/// constants, never stored, and so never exported.
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
    let snapshot = db.integration_store().snapshot(work).map_err(store_failure)?;
    let control = snapshot
        .attachment()
        .map_err(store_failure)?
        .ok_or(CheckpointError::NotInitialized)?;
    let authority = decode_control(control, policy.head_cap).map_err(CheckpointError::Frame)?;
    let mut writer = StreamWriter::new(sink, policy.chunk_bytes, policy.stream);
    // Facts: the store's bounded canonical logical export from this exact
    // snapshot. Writer refusals are smuggled out around the storage error
    // channel; the poison `Cancelled` below never escapes.
    let mut sink_error: Option<WriteError<K::Error>> = None;
    let exported = snapshot.export(work, &mut |relation, row| {
        match writer.fact(relation.0, row) {
            Ok(()) => Ok(()),
            Err(error) => {
                sink_error = Some(error);
                Err(bumbledb::store::StoreError::Work(WorkError::Cancelled))
            }
        }
    });
    if let Some(error) = sink_error.take() {
        return Err(write_failure(error));
    }
    exported.map_err(store_failure)?;
    // System records in ascending key order: 'm' (0x6d) < 'r' (0x72).
    for prefix in [HISTORY_KEY_PREFIX, RECEIPT_KEY_PREFIX] {
        let scanned: Result<(), bumbledb::store::StoreError> =
            snapshot.host_scan(&[prefix], work, &mut |key: &[u8], value: &[u8]| {
                if prefix == RECEIPT_KEY_PREFIX
                    && key.len() >= 9
                    && u64::from_be_bytes(key[1..9].try_into().expect("width")) <= retired_filter
                {
                    return Ok(());
                }
                match writer.system(key, value) {
                    Ok(()) => Ok(()),
                    Err(error) => {
                        sink_error = Some(error);
                        Err(bumbledb::store::StoreError::Work(WorkError::Cancelled))
                    }
                }
            });
        if let Some(error) = sink_error.take() {
            return Err(write_failure(error));
        }
        scanned.map_err(store_failure)?;
    }
    let summary = writer.finish().map_err(write_failure)?;
    Ok(Captured { authority, summary })
}

struct UploadSink<'a, B> {
    backend: &'a B,
    prefix: &'a str,
    epoch: u64,
}

impl<B: ReceivingStore> ChunkSink for UploadSink<'_, B>
where
    B::Error: BackendError + ObservedError,
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
/// `tip` to `base`, counting bytes and the oldest epoch touched. The initial
/// tip ObjectRef is preserved (C6).
struct Suffix {
    tail_bytes: u64,
    epoch_floor: u64,
    tip_object: Option<ObjectRef>,
}

struct SuffixVisitor {
    tail_bytes: u64,
    epoch_floor: u64,
    limits: Limits,
}

impl ChainVisitor for SuffixVisitor {
    type Error = ObjectError;

    fn visit(
        &mut self,
        _stamp: DecisionStamp,
        bytes: &[u8],
        reference: ObjectRef,
    ) -> Result<bool, ObjectError> {
        self.tail_bytes += bytes.len() as u64;
        self.epoch_floor = self.epoch_floor.min(reference.epoch);
        if let Ok(envelope) = decision::decode_decision(bytes, self.limits)
            && let Some(parent) = envelope.parent_object
        {
            self.epoch_floor = self.epoch_floor.min(parent.epoch);
        }
        Ok(true)
    }
}

fn map_suffix_walk(error: ObjectError) -> CheckpointError {
    match error {
        ObjectError::Frame(_)
        | ObjectError::Missing { .. }
        | ObjectError::WrongDigest { .. } => CheckpointError::Corruption(
            "suffix walk did not reach the captured base",
        ),
        ObjectError::Backend(error) => CheckpointError::Object(ObjectError::Backend(error)),
    }
}

#[expect(clippy::too_many_arguments, reason = "one bounded suffix walk")]
fn validate_suffix<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    base: DecisionStamp,
    tip: DecisionStamp,
    tip_object: Option<ObjectRef>,
    limits: Limits,
    budget: u64,
    work: &WorkContext,
) -> Result<Suffix, CheckpointError>
where
    B::Error: BackendError + ObservedError,
{
    if tip == base {
        return Ok(Suffix {
            tail_bytes: 0,
            epoch_floor: u64::MAX,
            tip_object: None,
        });
    }
    crate::history::locator::validate_tip_locator(tip, tip_object)
        .map_err(|_| CheckpointError::Corruption("tip locator missing or invalid"))?;
    let starting = tip_object.expect("tip locator validated");
    work.checkpoint()?;
    let mut walk_budget = budget;
    let mut visitor = SuffixVisitor {
        tail_bytes: 0,
        epoch_floor: starting.epoch,
        limits,
    };
    crate::history::locator::walk_decision_chain(
        backend,
        prefix,
        tip,
        base,
        Some(starting),
        limits,
        &mut walk_budget,
        work,
        &mut visitor,
    )
    .map_err(map_suffix_walk)?;
    Ok(Suffix {
        tail_bytes: visitor.tail_bytes,
        epoch_floor: visitor.epoch_floor,
        tip_object: Some(starting),
    })
}

/// Capture one coherent snapshot of `db` and upload it as a complete
/// verified checkpoint — every chunk plus the streamed manifest — under
/// `epoch` at `prefix`, WITHOUT touching any head. `publish_checkpoint`
/// composes this into a head rebase; the hosted migration data plane (C11)
/// publishes the returned manifest reference inside the target's genesis
/// head recovery root instead, so a migrated incarnation's state and its
/// authoritative migration-history records ('m' system rows) are
/// reconstructible from the store alone.
///
/// # Errors
/// Sink/backend, storage, frame and work refusals; a deleted captured
/// authority refuses (`Deleted`). Uploaded objects from a failed attempt
/// remain orphans for a later collection.
pub fn upload_snapshot<S, B: ReceivingStore>(
    db: &Db<S>,
    backend: &B,
    prefix: &str,
    epoch: u64,
    retired_filter: u64,
    policy: &CheckpointPolicy,
    work: &WorkContext,
) -> Result<(CheckpointManifest, ObjectRef), CheckpointError>
where
    B::Error: BackendError + ObservedError,
{
    let mut sink = UploadSink {
        backend,
        prefix,
        epoch,
    };
    let captured = capture_into(db, &mut sink, retired_filter, policy, work)?;
    let capture_position = captured
        .authority
        .position()
        .ok_or(CheckpointError::Deleted)?;
    let manifest = CheckpointManifest {
        identity: captured.authority.identity,
        decision: capture_position.decision,
        state: capture_position.state,
        control_at_capture: captured.authority,
        application_digest: captured.summary.application_digest,
        system_digest: captured.summary.system_digest,
        stream_digest: captured.summary.stream_digest,
        total_bytes: captured.summary.total_bytes,
        rows: captured.summary.rows,
        system_records: captured.summary.system_records,
        chunks: captured.summary.chunks.clone(),
    };
    let manifest_ref = put_verified(
        backend,
        prefix,
        epoch,
        ObjectKind::Checkpoint,
        &codec::encode_manifest(&manifest, policy.stream)?,
    )?;
    Ok((manifest, manifest_ref))
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
pub fn publish_checkpoint<S, B: ReceivingStore>(
    db: &Db<S>,
    backend: &B,
    prefix: &str,
    command_limits: Limits,
    kind: CheckpointKind,
    policy: &CheckpointPolicy,
    work: &WorkContext,
) -> Result<CheckpointOutcome, CheckpointError>
where
    B::Error: BackendError + ObservedError,
{
    // 1. The exact parent head names the staging epoch and lineage.
    let (parent, _) = read_live_head(backend, prefix, policy.head_cap, work)?;
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

    // 2/3. One coherent capture streamed straight into verified chunk
    // uploads, plus its streamed manifest.
    let (mut manifest, mut manifest_ref) =
        upload_snapshot(db, backend, prefix, staged_epoch, retired_filter, policy, work)?;
    if manifest.identity != parent.control.identity {
        return Err(CheckpointError::Corruption("captured foreign identity"));
    }
    let base = manifest.decision;
    let mut chunks = manifest.chunks.clone();

    // 4. Bounded rebase against the moving head.
    for _ in 0..policy.rebase_attempts {
        work.checkpoint()?;
        let (current, version) = read_live_head(backend, prefix, policy.head_cap, work)?;
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
        // Distinguish ordinary base advancement from same-base receipt-policy
        // replacement (LOG-017): retirement at the same decision is valid.
        if base.seq < current_recovery.base.seq {
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
            current_recovery.tip_object,
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
                let charged = get_verified(
                    backend,
                    prefix,
                    old,
                    TransportContext::new(work, ReceiveLimits::exact(old.length)),
                )?;
                restaged.push(put_verified(
                    backend,
                    prefix,
                    new_epoch,
                    ObjectKind::Chunk,
                    charged.as_bytes(),
                )?);
                drop(charged.into_owner());
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
        let recovery = if live.decision == base {
            RecoveryRoot::checkpoint_only(
                Some(manifest_ref),
                base,
                0,
                current.object_epoch,
            )
        } else {
            let tip_object = suffix.tip_object.ok_or(CheckpointError::Corruption(
                "suffix root missing tip ObjectRef",
            ))?;
            RecoveryRoot::suffix(
                Some(manifest_ref),
                base,
                live.decision,
                tip_object,
                suffix.tail_bytes,
                suffix.epoch_floor,
            )
            .map_err(|_| CheckpointError::Corruption("recovery root locators refused"))?
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
                let (observed, _) = read_live_head(backend, prefix, policy.head_cap, work)?;
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
/// head — reading never initializes. The receive is charged under `work`
/// and decoded from the owner; the reservation is dropped after decode.
///
/// # Errors
/// Backend and frame refusals.
pub fn read_live_head<B: ReceivingStore>(
    backend: &B,
    prefix: &str,
    cap: usize,
    work: &WorkContext,
) -> Result<(HeadRecord, HeadVersion), CheckpointError>
where
    B::Error: BackendError + ObservedError,
{
    work.checkpoint()?;
    match read_head_bounded(
        backend,
        &head_key(prefix),
        TransportContext::new(work, ReceiveLimits::capped(cap as u64)),
    )
    .map_err(CheckpointError::Object)?
    {
        ReceivedHead::Present { version, body } => {
            let record = decode_head(body.as_bytes(), cap)?;
            drop(body);
            Ok((record, version))
        }
        ReceivedHead::Absent => Err(CheckpointError::NotInitialized),
    }
}
