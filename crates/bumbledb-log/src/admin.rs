//! Administrative authority transitions over both backends (C08).
//!
//! Every operation uses its existing operation/root/barrier identity and the
//! completed/not-started/outcome-unknown discipline: a hosted CAS whose
//! response is lost resolves by re-reading the head and recognizing the
//! transition **evidence** (the P04 transitions are evidence-idempotent), a
//! local transition is one LMDB transaction. No generic admin journal exists
//! and no maintenance manufactures a command receipt.
//!
//! Receipt retirement is the one compound transition: hosted retirement
//! advances atomically with a checkpoint that no longer promises the retired
//! rows ([`crate::checkpointer::CheckpointKind::RetireReceipts`]); the local
//! row deletion follows the durable frontier, never precedes it.
//! LocalHistory retires in one transaction — control and exactly the retired
//! rows together.

use bumbledb::integration::{
    AttachmentChange, HostChanges, HostRecordChange, HostSealError, IntegrationError,
};
use bumbledb::{Db, WorkContext, WorkError};

use crate::checkpointer::read_live_head;
use crate::history::authority::{
    ActivateOutcome, ActivationCause, AuthorityError, DeleteOutcome, DeletedReason, FreezeIntent,
    FreezeOutcome, HeadAuthority, decode_control, encode_control,
};
use crate::history::receipt::RECEIPT_KEY_PREFIX;
use crate::history::{
    DecisionDigest, FrameError, HeadRevision, OperationId, ReceiptEpoch, StateStamp,
};
use crate::manifest::{HeadError, HeadRecord, NamedRoot, RootKind, RootPolicy, encode_head};
use crate::store::{
    BackendError, ConditionalOutcome, ConditionalStore, ObjectError, backend as backend_error,
    head_key,
};

#[derive(Debug)]
pub enum AdminError {
    Object(ObjectError),
    Frame(FrameError),
    Head(HeadError),
    Authority(AuthorityError),
    Storage(bumbledb::Error),
    Host(HostSealError),
    Work(WorkError),
    /// The materialization/head is not initialized; open never initializes.
    NotInitialized,
    /// Bounded CAS attempts exhausted by contention; nothing is claimed.
    CasExhausted,
    Corruption(&'static str),
    Checkpoint(crate::checkpointer::CheckpointError),
}

impl From<ObjectError> for AdminError {
    fn from(error: ObjectError) -> Self {
        Self::Object(error)
    }
}
impl From<FrameError> for AdminError {
    fn from(error: FrameError) -> Self {
        Self::Frame(error)
    }
}
impl From<HeadError> for AdminError {
    fn from(error: HeadError) -> Self {
        Self::Head(error)
    }
}
impl From<AuthorityError> for AdminError {
    fn from(error: AuthorityError) -> Self {
        Self::Authority(error)
    }
}
impl From<bumbledb::Error> for AdminError {
    fn from(error: bumbledb::Error) -> Self {
        Self::Storage(error)
    }
}
impl From<HostSealError> for AdminError {
    fn from(error: HostSealError) -> Self {
        Self::Host(error)
    }
}
impl From<WorkError> for AdminError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}
impl From<crate::checkpointer::CheckpointError> for AdminError {
    fn from(error: crate::checkpointer::CheckpointError) -> Self {
        Self::Checkpoint(error)
    }
}
impl From<IntegrationError> for AdminError {
    fn from(error: IntegrationError) -> Self {
        match error {
            IntegrationError::Core(error) => Self::Storage(error),
            IntegrationError::Host(error) => Self::Host(error),
            IntegrationError::Work(error) => Self::Work(error),
            IntegrationError::Changes(_)
            | IntegrationError::ForeignSchema
            | IntegrationError::ReentrantWriter => Self::Corruption("integration misuse"),
        }
    }
}

const CAS_ATTEMPTS: u32 = 16;

/// What a transition step decided against the exact observed head.
#[expect(
    clippy::large_enum_variant,
    reason = "one Step lives on the CAS driver's frame for one attempt and \
              is consumed immediately; boxing the proposed head would spend \
              an allocation per attempt to shrink a transient"
)]
enum Step<T> {
    /// Recorded evidence already satisfies the request; no CAS.
    Evidence(T),
    /// Publish this successor head; on success return the value.
    Publish(HeadRecord, T),
}

/// The one hosted maintenance driver: read the exact head, apply an
/// evidence-idempotent transition, CAS. A lost response self-resolves on the
/// next read because the transition recognizes its own recorded evidence.
fn hosted_transition<B: ConditionalStore, T>(
    backend: &B,
    prefix: &str,
    cap: usize,
    work: &WorkContext,
    mut step: impl FnMut(&HeadRecord) -> Result<Step<T>, AdminError>,
) -> Result<T, AdminError>
where
    B::Error: BackendError,
{
    for _ in 0..CAS_ATTEMPTS {
        work.checkpoint()?;
        let (current, version) = read_live_head(backend, prefix, cap)?;
        match step(&current)? {
            Step::Evidence(value) => return Ok(value),
            Step::Publish(proposed, value) => {
                let body = encode_head(&proposed, cap)?;
                match backend
                    .replace_head(&head_key(prefix), &version, &body)
                    .map_err(backend_error)
                    .map_err(AdminError::Object)?
                {
                    ConditionalOutcome::Published { .. } => return Ok(value),
                    ConditionalOutcome::PreconditionFailed | ConditionalOutcome::Indeterminate => {
                        // Re-read; evidence recognition resolves a lost
                        // response, a competing writer forces a fresh step.
                    }
                }
            }
        }
    }
    Err(AdminError::CasExhausted)
}

/// Rotate the open receipt epoch (maintenance CAS). Strictly advances; a
/// delayed writer using the previous head cannot publish after this barrier.
///
/// # Errors
/// Backward rotation, frozen/deleted authorities and contention refuse.
pub fn rotate_receipts_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    next: ReceiptEpoch,
    cap: usize,
    work: &WorkContext,
) -> Result<HeadRevision, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        let live = current.control.live()?;
        if live.receipts.open_epoch() >= next {
            // Evidence: rotation already at/past the requested epoch.
            return Ok(Step::Evidence(current.control.revision));
        }
        let control = current.control.rotate_receipts(next)?;
        let revision = control.revision;
        Ok(Step::Publish(current.with_control(control), revision))
    })
}

/// Durable freeze under a named operation (maintenance CAS). Reads and
/// retained receipt lookup continue; only new command admission stops; no
/// timer thaws it.
///
/// # Errors
/// A different operation's freeze refuses; matching retries are evidence.
pub fn freeze_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    operation: OperationId,
    intent: FreezeIntent,
    cap: usize,
    work: &WorkContext,
) -> Result<FreezeOutcome, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        match current.control.freeze(operation, intent)? {
            FreezeOutcome::AlreadyFrozen { operation, intent } => {
                Ok(Step::Evidence(FreezeOutcome::AlreadyFrozen {
                    operation,
                    intent,
                }))
            }
            FreezeOutcome::Frozen(control) => Ok(Step::Publish(
                current.with_control(control),
                FreezeOutcome::Frozen(control),
            )),
        }
    })
}

/// Thaw the matching frozen operation. For a migration abort the caller must
/// already hold the durable target fence; an uncertain cancellation never
/// authorizes thaw (that ordering is the runner's obligation, enforced by
/// P09's workflow over this operation).
///
/// # Errors
/// Mismatched operations and active authorities refuse.
pub fn thaw_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    operation: OperationId,
    cap: usize,
    work: &WorkContext,
) -> Result<HeadRevision, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        match current.control.thaw(operation) {
            Ok(control) => {
                let revision = control.revision;
                Ok(Step::Publish(current.with_control(control), revision))
            }
            // Already thawed by our earlier lost-response attempt: the
            // operation no longer holds a freeze — evidence, not failure.
            Err(AuthorityError::NotFrozen) => Ok(Step::Evidence(current.control.revision)),
            Err(error) => Err(error.into()),
        }
    })
}

/// One-time activation through the target's actual authority. A matching
/// retry returns the recorded evidence and current access mode without
/// mutating; it never thaws a later freeze or revives a deleted authority.
///
/// # Errors
/// Foreign operations and conflicting evidence refuse.
pub fn activate_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    operation: OperationId,
    target_genesis: DecisionDigest,
    cause: ActivationCause,
    cap: usize,
    work: &WorkContext,
) -> Result<ActivateOutcome, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        match current.control.activate(operation, target_genesis, cause)? {
            ActivateOutcome::AlreadyActivated { activation, access } => {
                Ok(Step::Evidence(ActivateOutcome::AlreadyActivated {
                    activation,
                    access,
                }))
            }
            ActivateOutcome::Activated(control) => Ok(Step::Publish(
                current.with_control(control),
                ActivateOutcome::Activated(control),
            )),
        }
    })
}

/// Terminal tombstone (erasure or migration abort). Drops the active
/// recovery root, preserves identity/revision continuity, explicit named
/// roots, the object epoch and any running barrier's progress. If activation
/// already won a matching migration operation, cancellation refuses.
///
/// # Errors
/// Conflicting recorded tombstones and won activations refuse.
pub fn tombstone_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    operation: OperationId,
    reason: DeletedReason,
    cap: usize,
    work: &WorkContext,
) -> Result<DeleteOutcome, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        match current.control.delete(operation, reason)? {
            DeleteOutcome::AlreadyDeleted { operation, reason } => {
                Ok(Step::Evidence(DeleteOutcome::AlreadyDeleted {
                    operation,
                    reason,
                }))
            }
            DeleteOutcome::Deleted(control) => Ok(Step::Publish(
                current.with_control(control),
                DeleteOutcome::Deleted(control),
            )),
        }
    })
}

/// Cancel an unpublished migration target by conditionally creating its
/// terminal tombstone head **instead of** genesis. A delayed conditional
/// genesis creation then loses. Returns whether this call created it, found
/// the matching tombstone (evidence) or found conflicting target evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelOutcome {
    Fenced,
    AlreadyFenced,
    /// The target namespace holds a live/foreign head; automatic abort must
    /// refuse — activation may have won.
    TargetExists,
}

/// # Errors
/// Backend failures and frame refusals; an unresolved create returns
/// `CasExhausted` rather than a claim.
#[expect(clippy::too_many_arguments, reason = "one bounded cancel transition")]
pub fn cancel_target_before_genesis<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    planned: crate::history::DatabaseIdentity,
    operation: OperationId,
    reason: DeletedReason,
    object_epoch: u64,
    cap: usize,
    work: &WorkContext,
) -> Result<CancelOutcome, AdminError>
where
    B::Error: BackendError,
{
    let control = HeadAuthority::cancelled_before_genesis(planned, operation, reason);
    let record = HeadRecord::cancelled_before_genesis(control, object_epoch);
    let body = encode_head(&record, cap)?;
    for _ in 0..CAS_ATTEMPTS {
        work.checkpoint()?;
        match backend
            .create_head(&head_key(prefix), &body)
            .map_err(backend_error)
            .map_err(AdminError::Object)?
        {
            ConditionalOutcome::Published { .. } => return Ok(CancelOutcome::Fenced),
            ConditionalOutcome::PreconditionFailed => {
                let (existing, _) = read_live_head(backend, prefix, cap)?;
                return match existing.control.delete(operation, reason) {
                    Ok(DeleteOutcome::AlreadyDeleted { .. }) => Ok(CancelOutcome::AlreadyFenced),
                    _ => Ok(CancelOutcome::TargetExists),
                };
            }
            ConditionalOutcome::Indeterminate => {
                // Resolve by reading: our exact tombstone is evidence.
                if let Ok((existing, _)) = read_live_head(backend, prefix, cap) {
                    return match existing.control.delete(operation, reason) {
                        Ok(DeleteOutcome::AlreadyDeleted { .. }) => {
                            Ok(CancelOutcome::AlreadyFenced)
                        }
                        _ => Ok(CancelOutcome::TargetExists),
                    };
                }
                // Still absent or unreadable: retry the fence.
            }
        }
    }
    Err(AdminError::CasExhausted)
}

/// A no-op revision-advancing maintenance CAS: fences a still-possible
/// exact-version attempt so `resolve` can race it and then settle. This is
/// the documented administrative/recovery transition, not a writer lease.
///
/// # Errors
/// Tombstones and contention refuse.
pub fn fence_revision_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    cap: usize,
    work: &WorkContext,
) -> Result<HeadRevision, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        let control = current.control.maintained()?;
        let revision = control.revision;
        Ok(Step::Publish(current.with_control(control), revision))
    })
}

/// Register a named restore point / hydration hold against the **exact
/// captured head**: its current recovery closure and stamps. After the head
/// moves, the loop reselects and revalidates against the successor rather
/// than attaching a stale capture (a pin must not resurrect a root already
/// eligible for deletion).
///
/// # Errors
/// Capacity refusals never discard another root; duplicate IDs refuse.
#[expect(clippy::too_many_arguments, reason = "one bounded root registration")]
pub fn add_named_root_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    root_id: OperationId,
    kind: RootKind,
    label: &str,
    operation: OperationId,
    policy: &RootPolicy,
    cap: usize,
    work: &WorkContext,
) -> Result<NamedRoot, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        if let Some(held) = current.roots.iter().find(|held| held.id == root_id) {
            // Evidence: our earlier lost-response registration landed.
            return Ok(Step::Evidence(held.clone()));
        }
        let live = current.control.live()?;
        let recovery = current
            .recovery
            .ok_or(AdminError::Corruption("live head without recovery root"))?;
        let root = NamedRoot {
            id: root_id,
            kind,
            recovery,
            state: live.state,
            label: label.into(),
            operation,
        };
        let proposed = current.add_root(root.clone(), policy)?;
        Ok(Step::Publish(proposed, root))
    })
}

/// Release exactly one named root by ID. Root deletion reports the lost
/// recovery capability (the returned root) before objects are later
/// collected; a stale release cannot remove a different root.
///
/// # Errors
/// Unknown IDs refuse (idempotent release resolves as evidence only when the
/// caller passes `already_released_ok`).
pub fn release_named_root_hosted<B: ConditionalStore>(
    backend: &B,
    prefix: &str,
    root_id: OperationId,
    already_released_ok: bool,
    cap: usize,
    work: &WorkContext,
) -> Result<Option<NamedRoot>, AdminError>
where
    B::Error: BackendError,
{
    hosted_transition(backend, prefix, cap, work, |current| {
        let Some(held) = current
            .roots
            .iter()
            .find(|held| held.id == root_id)
            .cloned()
        else {
            return if already_released_ok {
                Ok(Step::Evidence(None))
            } else {
                Err(AdminError::Head(HeadError::UnknownRoot))
            };
        };
        let proposed = current.release_root(root_id)?;
        Ok(Step::Publish(proposed, Some(held)))
    })
}

// ---------------------------------------------------------------------------
// LocalHistory: every transition is one LMDB transaction.
// ---------------------------------------------------------------------------

/// Read the committed local authority attachment.
///
/// # Errors
/// Uninitialized databases and corruption refuse.
pub fn local_authority<S>(db: &Db<S>, cap: usize) -> Result<HeadAuthority, AdminError> {
    let mut owned = None;
    db.read(|read| {
        owned = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })?;
    let control = owned.ok_or(AdminError::NotInitialized)?;
    Ok(decode_control(&control, cap)?)
}

/// One local authority transition committed atomically with optional host
/// row changes. The exclusive writer session serializes against commands.
fn local_transition<S, T>(
    db: &Db<S>,
    cap: usize,
    work: &WorkContext,
    rows: &[HostRecordChange<'_>],
    step: impl FnOnce(&HeadAuthority) -> Result<Step<T>, AdminError>,
) -> Result<T, AdminError> {
    let mut session = db.integration_writer(work)?;
    let current = local_authority(db, cap)?;
    match step(&current)? {
        Step::Evidence(value) => Ok(value),
        Step::Publish(record, value) => {
            let control = encode_control(&record.control, cap)?;
            let empty = bumbledb::ChangeSet::builder(db.schema(), work.clone())
                .finish()
                .map_err(|_| AdminError::Corruption("empty delta refused"))?;
            let prepared = match session.prepare(&empty)? {
                bumbledb::Admission::Accepted(prepared) => prepared,
                bumbledb::Admission::Rejected(_) => {
                    return Err(AdminError::Corruption("empty delta rejected"));
                }
            };
            prepared
                .seal(HostChanges {
                    records: rows,
                    attachment: AttachmentChange::Put(&control),
                })?
                .commit()?;
            Ok(value)
        }
    }
}

fn local_record(control: &HeadAuthority) -> HeadRecord {
    // LocalHistory has no hosted retention fields; the wrapper exists only
    // so local transitions share the hosted step vocabulary.
    HeadRecord {
        control: *control,
        object_epoch: 0,
        recovery: None,
        roots: Vec::new(),
        gc: crate::manifest::GcPhase::Idle,
    }
}

/// Local receipt-epoch rotation: one transaction.
///
/// # Errors
/// Backward rotation and frozen/deleted authorities refuse.
pub fn rotate_receipts_local<S>(
    db: &Db<S>,
    next: ReceiptEpoch,
    cap: usize,
    work: &WorkContext,
) -> Result<(), AdminError> {
    local_transition(db, cap, work, &[], |current| {
        if let Ok(live) = current.live()
            && live.receipts.open_epoch() >= next
        {
            return Ok(Step::Evidence(()));
        }
        Ok(Step::Publish(
            local_record(&current.rotate_receipts(next)?),
            (),
        ))
    })
}

/// The retired receipt-row keys at or below `through`, from one committed
/// read. Uses the recorded `integration_host_scan` seam (P02R).
///
/// # Errors
/// Storage failures refuse.
pub fn retired_row_keys<S>(db: &Db<S>, through: u64) -> Result<Vec<Vec<u8>>, AdminError> {
    let mut keys = Vec::new();
    let mut host_error: Option<HostSealError> = None;
    db.read(|read| {
        if let Err(error) =
            read.integration_host_scan(&[RECEIPT_KEY_PREFIX], &mut |key: &[u8], _value: &[u8]| {
                if key.len() >= 9
                    && u64::from_be_bytes(key[1..9].try_into().expect("width")) <= through
                {
                    keys.push(key.to_vec());
                }
                Ok(())
            })
        {
            host_error = Some(error);
        }
        Ok(())
    })?;
    if let Some(error) = host_error {
        return Err(error.into());
    }
    Ok(keys)
}

/// Local receipt retirement: the frontier advance and exactly the retired
/// rows' deletion commit in ONE transaction — never a later best-effort
/// sweep, and never removal of promised rows before the frontier.
///
/// # Errors
/// Non-monotone retirement and frozen/deleted authorities refuse.
pub fn retire_receipts_local<S>(
    db: &Db<S>,
    through: u64,
    cap: usize,
    work: &WorkContext,
) -> Result<u64, AdminError> {
    let keys = retired_row_keys(db, through)?;
    let deletes: Vec<HostRecordChange<'_>> = keys
        .iter()
        .map(|key| HostRecordChange::Delete { key })
        .collect();
    let removed = keys.len() as u64;
    local_transition(db, cap, work, &deletes, |current| {
        if let Ok(live) = current.live()
            && live.receipts.retired_through() >= through
        {
            return Ok(Step::Evidence(0u64));
        }
        Ok(Step::Publish(
            local_record(&current.retire_receipts(through)?),
            removed,
        ))
    })
}

/// After a hosted retirement checkpoint published, drop the local copies of
/// exactly the retired rows and install the new control — one transaction,
/// strictly after the durable frontier advanced.
///
/// # Errors
/// Storage failures refuse; the durable hosted decision is unaffected.
pub fn apply_hosted_retirement_locally<S>(
    db: &Db<S>,
    new_control: &HeadAuthority,
    through: u64,
    cap: usize,
    work: &WorkContext,
) -> Result<u64, AdminError> {
    let keys = retired_row_keys(db, through)?;
    let deletes: Vec<HostRecordChange<'_>> = keys
        .iter()
        .map(|key| HostRecordChange::Delete { key })
        .collect();
    let removed = keys.len() as u64;
    local_transition(db, cap, work, &deletes, |_| {
        Ok(Step::Publish(local_record(new_control), removed))
    })
}

/// Local freeze/thaw/activate/tombstone: one transaction each; the same
/// evidence semantics as their hosted counterparts.
///
/// # Errors
/// The authority transition's refusals.
pub fn freeze_local<S>(
    db: &Db<S>,
    operation: OperationId,
    intent: FreezeIntent,
    cap: usize,
    work: &WorkContext,
) -> Result<FreezeOutcome, AdminError> {
    local_transition(db, cap, work, &[], |current| {
        match current.freeze(operation, intent)? {
            FreezeOutcome::AlreadyFrozen { operation, intent } => {
                Ok(Step::Evidence(FreezeOutcome::AlreadyFrozen {
                    operation,
                    intent,
                }))
            }
            FreezeOutcome::Frozen(control) => Ok(Step::Publish(
                local_record(&control),
                FreezeOutcome::Frozen(control),
            )),
        }
    })
}

/// # Errors
/// Mismatched operations refuse.
pub fn thaw_local<S>(
    db: &Db<S>,
    operation: OperationId,
    cap: usize,
    work: &WorkContext,
) -> Result<(), AdminError> {
    local_transition(db, cap, work, &[], |current| {
        match current.thaw(operation) {
            Ok(control) => Ok(Step::Publish(local_record(&control), ())),
            Err(AuthorityError::NotFrozen) => Ok(Step::Evidence(())),
            Err(error) => Err(error.into()),
        }
    })
}

/// # Errors
/// Foreign operations and conflicting evidence refuse.
pub fn activate_local<S>(
    db: &Db<S>,
    operation: OperationId,
    target_genesis: DecisionDigest,
    cause: ActivationCause,
    cap: usize,
    work: &WorkContext,
) -> Result<ActivateOutcome, AdminError> {
    local_transition(db, cap, work, &[], |current| {
        match current.activate(operation, target_genesis, cause)? {
            ActivateOutcome::AlreadyActivated { activation, access } => {
                Ok(Step::Evidence(ActivateOutcome::AlreadyActivated {
                    activation,
                    access,
                }))
            }
            ActivateOutcome::Activated(control) => Ok(Step::Publish(
                local_record(&control),
                ActivateOutcome::Activated(control),
            )),
        }
    })
}

/// # Errors
/// Conflicting recorded tombstones and won activations refuse.
pub fn tombstone_local<S>(
    db: &Db<S>,
    operation: OperationId,
    reason: DeletedReason,
    cap: usize,
    work: &WorkContext,
) -> Result<DeleteOutcome, AdminError> {
    local_transition(db, cap, work, &[], |current| {
        match current.delete(operation, reason)? {
            DeleteOutcome::AlreadyDeleted { operation, reason } => {
                Ok(Step::Evidence(DeleteOutcome::AlreadyDeleted {
                    operation,
                    reason,
                }))
            }
            DeleteOutcome::Deleted(control) => Ok(Step::Publish(
                local_record(&control),
                DeleteOutcome::Deleted(control),
            )),
        }
    })
}

/// The captured state stamp of a live local authority — the value local
/// restore points pin.
///
/// # Errors
/// Tombstones refuse.
pub fn local_state<S>(db: &Db<S>, cap: usize) -> Result<StateStamp, AdminError> {
    let authority = local_authority(db, cap)?;
    Ok(authority.live()?.state)
}
