//! Hosted migration cutover transitions: the same authority values over one
//! never-reused HEAD per database, through the C07 [`ConditionalStore`]
//! verbs (P05 owns the actual S3/filesystem adapters).
//!
//! Remote HEAD bodies are the composed [`crate::manifest::HeadRecord`]
//! frames (C08) — exactly the grammar `writer::hosted` publishes: P04's
//! authority control projection embedded verbatim inside P05's retention
//! fields (object epoch, recovery root, named roots, GC state). Every read
//! here decodes the full record ([`manifest::decode_head`]); every authority
//! transition (freeze/thaw/activate/delete) is re-composed onto the exact
//! parent record with [`HeadRecord::with_control`], so retention fields are
//! preserved across a cutover and a tombstone drops only the active recovery
//! root. The target genesis publishes [`manifest::genesis_head_body`] and the
//! pre-genesis cancellation fence publishes a composed
//! [`HeadRecord::cancelled_before_genesis`] tombstone — a bare control frame
//! at any head is corruption-class evidence, never silently accepted.
//!
//! The three-way conditional grammar is the whole point: `Indeterminate`
//! (lost response) is resolved by re-reading recorded evidence, and when it
//! cannot be resolved the operation reports [`HostedOutcome::Unknown`] —
//! the source STAYS frozen, abort never thaws on uncertainty, and a retry
//! under the same operation resumes from durable state. Hosted abort races
//! delayed genesis through conditional CREATE of the exact cancellation
//! tombstone; activation and cancellation race through conditional REPLACE
//! of the same head, so exactly one wins and the loser reads the winner.
//!
//! [`HostedMigration`] and [`initialize`] are the COMPLETE hosted workflow
//! (the F3 finding-D closure of the recorded "hosted data plane pending C08"
//! boundary): S3-class storage IS the hosted authority, so one migration/
//! initialization publishes — besides the authority transitions above — the
//! target's durable recovery objects: ONE streamed verified checkpoint
//! (state chunks + manifest, staged under the target's open object epoch)
//! carrying both the migrated canonical state and the authoritative
//! migration-history records, named by the genesis head's recovery root. A
//! fresh host hydrates the migrated incarnation from the store alone
//! ([`crate::recovery::open_hosted`]); resume verifies a published target
//! from those exact durable objects instead of trusting a directory.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bumbledb::schema::{SchemaDescriptor, ValidateDescriptor as _};
use bumbledb::{Db, WorkContext};

use crate::checkpointer::{CheckpointError, CheckpointPolicy, upload_snapshot};
use crate::codec;
use crate::history::authority::{
    Access, ActivateOutcome, Activation, ActivationCause, DeleteOutcome, DeletedReason,
    FreezeIntent, FreezeOutcome, HeadAuthority, Lifecycle, decode_control, encode_control,
};
use crate::history::command::Limits;
use crate::history::decision::{GenesisProvenance, GenesisRecord, genesis_stamp};
use crate::history::{AccessMode, DatabaseIdentity, OperationId};
use crate::manifest::{self, GcPhase, HeadRecord, RecoveryRoot};
use crate::recovery::{begin_staged, fetch_charged_chunks, import_stream, settlement_failed};
use crate::store::{
    BackendError, ObjectError, ObservedError, ReceiveLimits, ReceivedHead, ReceivingStore,
    TransportContext, get_verified, read_head_bounded,
};
use crate::writer::{HostedHistory, LogError};
use crate::writer::verbs::{ConditionalOutcome, HeadVersion};

use super::executor::{
    AbortReport, ActivateReport, ActivationRef, MigrateOutcome, MigrationError, MigrationStatus,
    StagedTarget, SuffixRequest, TargetFence, applied_steps, build_staged, capture_source,
    compile_suffix, execute_steps, read_attachment, read_chain,
};
use super::history::{Applied, AppliedSource, HistoryRecord, system_digest, verify_chain};
use super::lock::TargetNamespace;
use super::manifest::{Manifest, bind_plans, plan_set_digest, verify_manifest};
use super::state::MigrationState;

/// Bounded CAS re-reads before this invocation stops claiming certainty.
const ATTEMPTS: usize = 4;

/// Certainty of one hosted admin transition. `Unknown` is never rewritten
/// into success or failure; the retained operation/reference resolves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedOutcome<T> {
    Completed(T),
    /// Dispatched with no definite outcome. Nothing was thawed, nothing may
    /// be assumed; resolve by re-invoking with the same stable operation.
    Unknown,
}

/// The hosted cutover driver for one migration: source and target prefixes
/// name their HEAD objects in the shared store. `target_object_epoch` is the
/// initial open object epoch recorded in a target genesis or pre-genesis
/// tombstone head (the same role as `HostedHistory::create`'s parameter);
/// the source's epoch is always read from its own composed head.
pub struct HostedCutover<'a, C> {
    store: &'a C,
    source_prefix: &'a str,
    target_prefix: &'a str,
    target_object_epoch: u64,
    limits: Limits,
}

fn head_key(prefix: &str) -> String {
    format!("{prefix}/HEAD")
}

impl<'a, C: ReceivingStore> HostedCutover<'a, C>
where
    C::Error: BackendError + ObservedError,
{
    pub fn new(
        store: &'a C,
        source_prefix: &'a str,
        target_prefix: &'a str,
        target_object_epoch: u64,
        limits: Limits,
    ) -> Self {
        Self {
            store,
            source_prefix,
            target_prefix,
            target_object_epoch,
            limits,
        }
    }

    /// Read and decode one composed head. A malformed body — a bare control
    /// projection included — is corruption-class at this boundary.
    fn read_head(&self, prefix: &str) -> Result<Option<(HeadVersion, HeadRecord)>, MigrationError> {
        read_head_record(
            self.store,
            prefix,
            self.limits.envelope_bytes,
            None,
        )
    }

    /// Durably freeze the source under this operation via head CAS.
    /// # Errors
    /// Foreign freezes, deleted sources and backend failures are typed.
    pub fn freeze_source(
        &self,
        operation: OperationId,
        intent: FreezeIntent,
    ) -> Result<HostedOutcome<()>, MigrationError> {
        for _ in 0..ATTEMPTS {
            let Some((version, record)) = self.read_head(self.source_prefix)? else {
                return Err(MigrationError::Log(LogError::NotInitialized));
            };
            match record.control.freeze(operation, intent) {
                Ok(FreezeOutcome::AlreadyFrozen { .. }) => {
                    return Ok(HostedOutcome::Completed(()));
                }
                Ok(FreezeOutcome::Frozen(frozen)) => {
                    match self.replace(self.source_prefix, &version, &record, &frozen)? {
                        ConditionalOutcome::Published { .. } => {
                            return Ok(HostedOutcome::Completed(()));
                        }
                        // Another writer moved the head: re-read and
                        // re-evaluate against the winner.
                        ConditionalOutcome::PreconditionFailed
                        | ConditionalOutcome::Indeterminate => {}
                    }
                }
                Err(crate::history::authority::AuthorityError::OperationMismatch { held }) => {
                    return Err(MigrationError::SourceFrozenByOther { operation: held });
                }
                Err(error) => return Err(MigrationError::from(LogError::from(error))),
            }
        }
        // The last dispatch may still win; certainty comes from re-reading
        // under the same operation, never from assuming a timeout failed.
        Ok(HostedOutcome::Unknown)
    }

    /// Publish the final target genesis head once, by conditional CREATE of
    /// the composed genesis record (genesis recovery root, empty named roots,
    /// idle GC, the configured initial object epoch).
    /// A durable cancellation tombstone wins this race permanently.
    /// # Errors
    /// A recorded foreign head/cancellation is a typed refusal.
    pub fn publish_target_genesis(
        &self,
        frozen_target: &HeadAuthority,
        operation: OperationId,
    ) -> Result<HostedOutcome<()>, MigrationError> {
        let body = manifest::genesis_head_body(
            frozen_target,
            self.target_object_epoch,
            self.limits.envelope_bytes,
        )
        .map_err(LogError::from)?;
        match self
            .store
            .create_head(&head_key(self.target_prefix), &body)
            .map_err(|_| MigrationError::Log(LogError::Backend))?
        {
            ConditionalOutcome::Published { .. } => Ok(HostedOutcome::Completed(())),
            ConditionalOutcome::PreconditionFailed | ConditionalOutcome::Indeterminate => {
                // Resolve by reading what actually exists. The recorded
                // CONTROL is the evidence; retention fields are the data
                // plane's and do not decide this race.
                match self.read_head(self.target_prefix)? {
                    Some((_, existing)) if existing.control == *frozen_target => {
                        Ok(HostedOutcome::Completed(()))
                    }
                    Some((_, existing)) => {
                        Err(target_evidence_refusal(&existing.control, operation))
                    }
                    None => Ok(HostedOutcome::Unknown),
                }
            }
        }
    }

    /// Explicit hosted activation via target head CAS; matching retries
    /// return recorded evidence with the CURRENT access mode.
    /// # Errors
    /// Wrong/stale references and cancelled targets are typed refusals.
    pub fn activate(
        &self,
        reference: &ActivationRef,
    ) -> Result<HostedOutcome<ActivateReport>, MigrationError> {
        for _ in 0..ATTEMPTS {
            let Some((version, record)) = self.read_head(self.target_prefix)? else {
                return Err(MigrationError::StaleActivationRef);
            };
            let authority = record.control;
            if authority.identity != reference.target {
                return Err(MigrationError::StaleActivationRef);
            }
            if let Lifecycle::Live(live) = &authority.lifecycle
                && authority.activation == Activation::NotActivated
                && (live.decision.seq != 0 || live.decision.hash != reference.target_genesis)
            {
                return Err(MigrationError::StaleActivationRef);
            }
            let cause = ActivationCause::Migration {
                plan_set_digest: reference.plan_set_digest,
            };
            match authority.activate(reference.operation, reference.target_genesis, cause) {
                Ok(ActivateOutcome::AlreadyActivated { activation, access }) => {
                    return Ok(HostedOutcome::Completed(ActivateReport {
                        activation,
                        access,
                    }));
                }
                Ok(ActivateOutcome::Activated(activated)) => {
                    match self.replace(self.target_prefix, &version, &record, &activated)? {
                        ConditionalOutcome::Published { .. } => {
                            return Ok(HostedOutcome::Completed(ActivateReport {
                                activation: activated.activation,
                                access: AccessMode::Active,
                            }));
                        }
                        ConditionalOutcome::PreconditionFailed
                        | ConditionalOutcome::Indeterminate => {}
                    }
                }
                Err(crate::history::authority::AuthorityError::Deleted) => {
                    return Err(MigrationError::Aborted {
                        operation: reference.operation,
                    });
                }
                Err(crate::history::authority::AuthorityError::ActivationEvidenceMismatch) => {
                    return Err(MigrationError::StaleActivationRef);
                }
                Err(crate::history::authority::AuthorityError::OperationMismatch { .. }) => {
                    return Err(MigrationError::TargetConflict);
                }
                Err(error) => return Err(MigrationError::from(LogError::from(error))),
            }
        }
        Ok(HostedOutcome::Unknown)
    }

    /// Durably fence the planned target BEFORE any thaw: terminal deletion
    /// of a published unactivated matching target (the composed successor
    /// drops the active recovery root, preserving named roots and barrier
    /// progress), or conditional CREATE of the exact composed pre-genesis
    /// cancellation tombstone when the target head is absent. An uncertain
    /// cancellation returns `Unknown` — never thaw.
    /// # Errors
    /// `ActivationWon` and conflicting evidence are typed refusals.
    pub fn fence_target(
        &self,
        planned_identity: DatabaseIdentity,
        operation: OperationId,
        reason: DeletedReason,
    ) -> Result<HostedOutcome<TargetFence>, MigrationError> {
        for _ in 0..ATTEMPTS {
            match self.read_head(self.target_prefix)? {
                None => {
                    let tombstone = HeadAuthority::cancelled_before_genesis(
                        planned_identity,
                        operation,
                        reason,
                    );
                    let record =
                        HeadRecord::cancelled_before_genesis(tombstone, self.target_object_epoch);
                    let body = manifest::encode_head(&record, self.limits.envelope_bytes)
                        .map_err(LogError::from)?;
                    match self
                        .store
                        .create_head(&head_key(self.target_prefix), &body)
                        .map_err(|_| MigrationError::Log(LogError::Backend))?
                    {
                        ConditionalOutcome::Published { .. } => {
                            return Ok(HostedOutcome::Completed(TargetFence::TombstonePreGenesis));
                        }
                        // A head appeared (delayed genesis won the create,
                        // or our own earlier create landed): re-read.
                        ConditionalOutcome::PreconditionFailed
                        | ConditionalOutcome::Indeterminate => {}
                    }
                }
                Some((version, record)) => {
                    let authority = record.control;
                    if authority.identity != planned_identity {
                        return Err(MigrationError::TargetConflict);
                    }
                    match authority.delete(operation, reason) {
                        Ok(DeleteOutcome::AlreadyDeleted { .. }) => {
                            return Ok(HostedOutcome::Completed(TargetFence::AlreadyFenced));
                        }
                        Ok(DeleteOutcome::Deleted(deleted)) => {
                            match self.replace(self.target_prefix, &version, &record, &deleted)? {
                                ConditionalOutcome::Published { .. } => {
                                    return Ok(HostedOutcome::Completed(
                                        TargetFence::TargetDeleted,
                                    ));
                                }
                                // Activation may have won the CAS: re-read
                                // and re-judge against the actual winner.
                                ConditionalOutcome::PreconditionFailed
                                | ConditionalOutcome::Indeterminate => {}
                            }
                        }
                        Err(
                            crate::history::authority::AuthorityError::ActivationEvidenceMismatch,
                        ) => {
                            return Err(MigrationError::ActivationWon);
                        }
                        Err(crate::history::authority::AuthorityError::OperationMismatch {
                            ..
                        }) => {
                            return Err(MigrationError::TargetConflict);
                        }
                        Err(error) => return Err(MigrationError::from(LogError::from(error))),
                    }
                }
            }
        }
        Ok(HostedOutcome::Unknown)
    }

    /// Thaw the matching frozen source AFTER a completed target fence.
    /// # Errors
    /// Foreign operations and backend failures are typed.
    pub fn thaw_source(
        &self,
        operation: OperationId,
    ) -> Result<HostedOutcome<bool>, MigrationError> {
        for _ in 0..ATTEMPTS {
            let Some((version, record)) = self.read_head(self.source_prefix)? else {
                return Err(MigrationError::Log(LogError::NotInitialized));
            };
            let live = record.control.live().map_err(LogError::from)?;
            match live.access {
                Access::Active => return Ok(HostedOutcome::Completed(false)),
                Access::Frozen {
                    operation: held, ..
                } => {
                    if held != operation {
                        return Err(MigrationError::SourceFrozenByOther { operation: held });
                    }
                    let thawed = record.control.thaw(operation).map_err(LogError::from)?;
                    match self.replace(self.source_prefix, &version, &record, &thawed)? {
                        ConditionalOutcome::Published { .. } => {
                            return Ok(HostedOutcome::Completed(true));
                        }
                        ConditionalOutcome::PreconditionFailed
                        | ConditionalOutcome::Indeterminate => {}
                    }
                }
            }
        }
        Ok(HostedOutcome::Unknown)
    }

    /// Complete hosted abort: fence the target durably FIRST, then thaw the
    /// matching source. `Unknown` at either stage stops with the source
    /// frozen; a retry under the same operation resumes from evidence.
    /// # Errors
    /// `ActivationWon` and conflicting evidence refuse automatic abort.
    pub fn abort(
        &self,
        planned_identity: DatabaseIdentity,
        operation: OperationId,
        reason: DeletedReason,
    ) -> Result<HostedOutcome<super::executor::AbortReport>, MigrationError> {
        let fence = match self.fence_target(planned_identity, operation, reason)? {
            HostedOutcome::Completed(fence) => fence,
            // An uncertain target cancellation never authorizes thaw.
            HostedOutcome::Unknown => return Ok(HostedOutcome::Unknown),
        };
        match self.thaw_source(operation)? {
            HostedOutcome::Completed(thawed) => {
                Ok(HostedOutcome::Completed(super::executor::AbortReport {
                    fence,
                    thawed,
                }))
            }
            HostedOutcome::Unknown => Ok(HostedOutcome::Unknown),
        }
    }

    /// Compose the successor head from the EXACT parent record — the
    /// transitioned control swapped in, every retention field preserved (a
    /// tombstone drops only the active recovery root) — and conditionally
    /// replace it.
    fn replace(
        &self,
        prefix: &str,
        expected: &HeadVersion,
        parent: &HeadRecord,
        next: &HeadAuthority,
    ) -> Result<ConditionalOutcome, MigrationError> {
        let successor = parent.with_control(*next);
        let body = manifest::encode_head(&successor, self.limits.envelope_bytes)
            .map_err(LogError::from)?;
        self.store
            .replace_head(&head_key(prefix), expected, &body)
            .map_err(|_| MigrationError::Log(LogError::Backend))
    }
}

fn target_evidence_refusal(existing: &HeadAuthority, operation: OperationId) -> MigrationError {
    match &existing.lifecycle {
        Lifecycle::Deleted {
            operation: held, ..
        } => {
            if *held == operation {
                MigrationError::Aborted { operation: *held }
            } else {
                MigrationError::TargetConflict
            }
        }
        Lifecycle::Live(_) => MigrationError::TargetConflict,
    }
}

// ---------------------------------------------------------------------------
// The complete hosted workflow: generated migration execution and
// initialization with the hosted data plane (C08/C11). S3-class storage IS
// the hosted authority: a `ReadyToSwitch` target exists hosted-side as its
// composed genesis head whose recovery root names ONE uploaded verified
// checkpoint (chunks + streamed manifest, staged under the target's open
// object epoch) carrying the complete migrated state AND the authoritative
// migration-history records, so a fresh host hydrates the migrated
// incarnation from the store alone (`recovery::open_hosted`).
// ---------------------------------------------------------------------------

/// Read and decode one composed head; a malformed body is corruption-class.
fn read_head_record<C>(
    store: &C,
    prefix: &str,
    cap: usize,
    work: Option<&WorkContext>,
) -> Result<Option<(HeadVersion, HeadRecord)>, MigrationError>
where
    C: ReceivingStore,
    C::Error: BackendError + ObservedError,
{
    if let Some(work) = work {
        work.checkpoint().map_err(|error| MigrationError::Log(error.into()))?;
    }
    match read_head_bounded(
        store,
        &head_key(prefix),
        TransportContext {
            work,
            receive: ReceiveLimits::capped(cap as u64),
        },
    )
    .map_err(map_head_object_error)?
    {
        ReceivedHead::Absent => Ok(None),
        ReceivedHead::Present { version, body } => {
            let record = manifest::decode_head(body.as_bytes(), cap).map_err(LogError::from)?;
            Ok(Some((version, record)))
        }
    }
}

fn map_head_object_error(error: ObjectError) -> MigrationError {
    MigrationError::Log(match error {
        ObjectError::Backend(_) | ObjectError::Unverified { .. } => LogError::Backend,
        ObjectError::Missing { .. }
        | ObjectError::WrongLength { .. }
        | ObjectError::WrongDigest { .. }
        | ObjectError::ImmutableConflict { .. }
        | ObjectError::Frame(_) => LogError::Corruption,
    })
}

/// Conditionally CREATE one composed target head; a lost response resolves
/// by re-reading the recorded evidence (the CONTROL decides the race —
/// retention fields are the data plane's own).
fn create_target_record<C>(
    store: &C,
    prefix: &str,
    record: &HeadRecord,
    operation: OperationId,
    cap: usize,
    work: Option<&WorkContext>,
) -> Result<HostedOutcome<()>, MigrationError>
where
    C: ReceivingStore,
    C::Error: BackendError + ObservedError,
{
    let body = manifest::encode_head(record, cap).map_err(LogError::from)?;
    match store
        .create_head(&head_key(prefix), &body)
        .map_err(|_| MigrationError::Log(LogError::Backend))?
    {
        ConditionalOutcome::Published { .. } => Ok(HostedOutcome::Completed(())),
        ConditionalOutcome::PreconditionFailed | ConditionalOutcome::Indeterminate => {
            match read_head_record(store, prefix, cap, work)? {
                Some((_, existing)) if existing.control == record.control => {
                    Ok(HostedOutcome::Completed(()))
                }
                Some((_, existing)) => Err(target_evidence_refusal(&existing.control, operation)),
                None => Ok(HostedOutcome::Unknown),
            }
        }
    }
}

/// What the target head durably records for one operation/plan set.
enum TargetProbe {
    /// No target head exists yet.
    Absent,
    /// Recorded terminal evidence: this operation already activated.
    AlreadyActivated(MigrateOutcome),
    /// A live, frozen, not-yet-activated target under EXACTLY this
    /// operation and plan set: verified reuse runs against it.
    Reusable(HeadRecord),
}

fn probe_target<C>(
    store: &C,
    prefix: &str,
    operation: OperationId,
    psd: [u8; 32],
    target_identity: DatabaseIdentity,
    cap: usize,
    work: Option<&WorkContext>,
) -> Result<TargetProbe, MigrationError>
where
    C: ReceivingStore,
    C::Error: BackendError + ObservedError,
{
    let Some((_, record)) = read_head_record(store, prefix, cap, work)? else {
        return Ok(TargetProbe::Absent);
    };
    let control = record.control;
    if control.identity != target_identity {
        return Err(MigrationError::TargetConflict);
    }
    if let Activation::Activated {
        operation: held, ..
    } = control.activation
    {
        if held == operation {
            let access = match &control.lifecycle {
                Lifecycle::Live(live) => live.access.mode(),
                Lifecycle::Deleted { .. } => AccessMode::Deleted,
            };
            return Ok(TargetProbe::AlreadyActivated(
                MigrateOutcome::AlreadyActivated {
                    activation: control.activation,
                    access,
                },
            ));
        }
        return Err(MigrationError::TargetConflict);
    }
    match &control.lifecycle {
        Lifecycle::Deleted {
            operation: held, ..
        } => Err(if *held == operation {
            MigrationError::Aborted { operation: *held }
        } else {
            MigrationError::TargetConflict
        }),
        Lifecycle::Live(live) => match live.access {
            Access::Frozen {
                operation: held,
                intent,
            } => {
                if held != operation {
                    return Err(MigrationError::TargetConflict);
                }
                match intent {
                    FreezeIntent::Migration {
                        plan_set_digest,
                        target,
                    } if plan_set_digest == psd && target == target_identity.incarnation_id => {
                        Ok(TargetProbe::Reusable(record))
                    }
                    _ => Err(MigrationError::PlanSetMismatch),
                }
            }
            Access::Active => Err(MigrationError::TargetConflict),
        },
    }
}

/// What a published target must extend and re-derive to be OUR exact output.
struct ExpectedTarget<'x> {
    operation: OperationId,
    psd: [u8; 32],
    identity: DatabaseIdentity,
    descriptor: &'x SchemaDescriptor,
    provenance: GenesisProvenance,
    /// The exact history records the target chain must extend (the source
    /// chain for a migration; empty for initialization).
    prior_chain: &'x [HistoryRecord],
    /// The generated manifest and the applied position the whole chain must
    /// flatten to.
    plans: &'x Manifest,
    expected_applied: u64,
}

/// Verify one published, frozen, unactivated hosted target completely from
/// its durable recovery objects: fetch and digest-verify the checkpoint,
/// hydrate it into private scratch through the core's judged admission,
/// verify the migration-history extension and the recomputed canonical state
/// digest, then re-derive the genesis stamp from that evidence — the head's
/// recorded genesis decision must equal it exactly. Same operation/source/
/// plan with conflicting completed output refuses (`OutputMismatch`), never
/// overwrite; tampered objects refuse as corruption-class evidence.
#[expect(
    clippy::too_many_lines,
    clippy::too_many_arguments,
    reason = "one bounded verified-reuse pipeline over the durable evidence"
)]
fn verify_published_target<C>(
    store: &C,
    target_prefix: &str,
    record: &HeadRecord,
    expected: &ExpectedTarget<'_>,
    scratch_root: &Path,
    limits: Limits,
    policy: &CheckpointPolicy,
    work: &WorkContext,
) -> Result<MigrateOutcome, MigrationError>
where
    C: ReceivingStore,
    C::Error: BackendError + ObservedError,
{
    let cap = limits.envelope_bytes;
    let live = record.control.live().map_err(LogError::from)?;
    if live.decision.seq != 0 {
        return Err(MigrationError::OutputMismatch);
    }
    // A migrated hosted target's data plane is its checkpoint: a live head
    // without one has no reconstructible state — corruption-class.
    let recovery = record.recovery.ok_or(MigrationError::Log(LogError::Corruption))?;
    let checkpoint = recovery
        .checkpoint
        .ok_or(MigrationError::Log(LogError::Corruption))?;
    let charged = get_verified(
        store,
        target_prefix,
        &checkpoint,
        TransportContext::new(work, ReceiveLimits::exact(checkpoint.length)),
    )
    .map_err(|error| MigrationError::Checkpoint(CheckpointError::Object(error)))?;
    let ckpt = codec::decode_manifest(charged.as_bytes(), policy.stream)
        .map_err(MigrationError::Frame)?;
    drop(charged.into_owner());
    if ckpt.identity != expected.identity {
        return Err(MigrationError::TargetConflict);
    }
    if ckpt.decision != live.decision
        || ckpt
            .control_at_capture
            .position()
            .is_none_or(|position| position.decision != live.decision)
    {
        return Err(MigrationError::OutputMismatch);
    }

    // Stream, digest-verify and hydrate the complete checkpoint state in
    // bounded batches — never a whole-database RAM materialization.
    let scratch = TargetNamespace::new(scratch_root, expected.identity.incarnation_id)?
        .fresh_staging();
    if let Some(parent) = scratch.parent() {
        std::fs::create_dir_all(parent).map_err(super::lock::NamespaceError::Io)?;
    }
    let staged = begin_staged(&scratch, expected.descriptor.clone(), work)
        .map_err(MigrationError::Hydration)?;
    let owners = fetch_charged_chunks(store, target_prefix, &ckpt.chunks, work)
        .map_err(MigrationError::Hydration)?;
    let mut keep = |_key: &[u8], _value: &[u8]| true;
    import_stream(
        &staged,
        &ckpt,
        owners
            .iter()
            .map(|charged| Ok::<_, crate::recovery::RecoveryError>(charged.as_bytes())),
        &mut keep,
        None,
        policy.stream,
        work,
    )
    .map_err(MigrationError::Hydration)?;
    for charged in owners {
        drop(charged.into_owner());
    }

    let control_bytes = encode_control(&ckpt.control_at_capture, cap).map_err(LogError::from)?;
    staged
        .write_host(&[], Some(&control_bytes), work)
        .map_err(MigrationError::Hydration)?;
    let dest = staged.destination().to_path_buf();
    staged
        .complete_install(work)
        .map_err(MigrationError::Hydration)?;
    let scratch_db = match bumbledb::Db::open(&dest, expected.descriptor.clone(), work.clone()) {
        Ok(db) => std::sync::Arc::new(db),
        Err(error) => {
            return Err(MigrationError::Hydration(settlement_failed(
                dest,
                bumbledb::store::StoreError::from(std::io::Error::other(error)),
            )));
        }
    };

    // The authoritative migration history rides the checkpoint's 'm' system rows.
    let chain = read_chain(&scratch_db, cap)?;
    if chain.len() != expected.prior_chain.len() + 1
        || chain[..expected.prior_chain.len()] != *expected.prior_chain
    {
        return Err(MigrationError::OutputMismatch);
    }
    let Some(HistoryRecord::Applied(applied)) = chain.last() else {
        return Err(MigrationError::OutputMismatch);
    };
    if applied.operation != expected.operation || applied.plan_set_digest != expected.psd {
        return Err(MigrationError::OutputMismatch);
    }
    if verify_chain(&chain, expected.plans, cap)? != expected.expected_applied {
        return Err(MigrationError::OutputMismatch);
    }

    let verify = || -> Result<MigrateOutcome, MigrationError> {
        let schema = expected
            .descriptor
            .clone()
            .validate()
            .map_err(bumbledb::Error::from)?;
        let mut captured = None;
        scratch_db.read(|read| {
            captured = Some(MigrationState::from_source(read, &schema, work));
            Ok(())
        })?;
        let recomputed = captured
            .ok_or(MigrationError::Log(LogError::Corruption))??
            .digest()?;
        if recomputed != applied.target_digest {
            return Err(MigrationError::OutputMismatch);
        }
        // Re-derive the genesis from the verified evidence: the recorded
        // head decision must be exactly this stamp, binding state digest,
        // history extension and provenance in one hash.
        let genesis = GenesisRecord {
            identity: expected.identity,
            initial_application_digest: applied.target_digest,
            initial_system_digest: system_digest(&chain, cap).map_err(LogError::from)?,
            provenance: expected.provenance,
        };
        let stamp = genesis_stamp(&genesis, cap).map_err(LogError::from)?;
        if stamp != live.decision {
            return Err(MigrationError::OutputMismatch);
        }
        Ok(MigrateOutcome::ReadyToSwitch {
            activation_ref: ActivationRef {
                operation: expected.operation,
                plan_set_digest: expected.psd,
                target: expected.identity,
                target_genesis: live.decision.hash,
            },
            applied: applied.clone(),
        })
    };
    let outcome = verify();
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

/// Build the ONE judged staged target in private scratch, upload its
/// complete verified checkpoint under the target's open object epoch, and
/// publish the composed genesis head once by conditional create — the
/// recovery root names the checkpoint, so the migrated state and its history
/// metadata are reconstructible from the store alone. A durable cancellation
/// tombstone wins the create race permanently; `Unknown` leaves everything
/// resumable under the same operation. Staging is always removed as scratch.
#[expect(
    clippy::too_many_arguments,
    reason = "one private build/publish path shared by migrate and initialize"
)]
fn build_and_publish<C>(
    store: &C,
    target_prefix: &str,
    target_object_epoch: u64,
    scratch_root: &Path,
    limits: Limits,
    policy: &CheckpointPolicy,
    operation: OperationId,
    psd: [u8; 32],
    genesis: GenesisRecord,
    target_chain: &[HistoryRecord],
    state: &MigrationState,
    descriptor: &SchemaDescriptor,
    work: &WorkContext,
) -> Result<HostedOutcome<ActivationRef>, MigrationError>
where
    C: ReceivingStore,
    C::Error: BackendError + ObservedError,
{
    let cap = limits.envelope_bytes;
    let target_identity = genesis.identity;
    let staging =
        TargetNamespace::new(scratch_root, target_identity.incarnation_id)?.fresh_staging();
    let publish = |staged: StagedTarget| -> Result<HostedOutcome<ActivationRef>, MigrationError> {
        let retired = staged
            .frozen
            .live()
            .map_err(LogError::from)?
            .receipts
            .retired_through();
        let (ckpt, manifest_ref) = upload_snapshot(
            &staged.db,
            store,
            target_prefix,
            target_object_epoch,
            retired,
            policy,
            work,
        )?;
        if ckpt.decision != staged.genesis {
            return Err(MigrationError::Log(LogError::Corruption));
        }
        drop(staged.db);
        let record = HeadRecord {
            control: staged.frozen,
            object_epoch: target_object_epoch,
            recovery: Some(RecoveryRoot::checkpoint_only(
                Some(manifest_ref),
                staged.genesis,
                0,
                target_object_epoch,
            )),
            roots: Vec::new(),
            gc: GcPhase::Idle,
        };
        match create_target_record(store, target_prefix, &record, operation, cap, Some(work))? {
            HostedOutcome::Completed(()) => Ok(HostedOutcome::Completed(ActivationRef {
                operation,
                plan_set_digest: psd,
                target: target_identity,
                target_genesis: staged.genesis.hash,
            })),
            HostedOutcome::Unknown => Ok(HostedOutcome::Unknown),
        }
    };
    let staged = build_staged(
        &staging, operation, psd, genesis, target_chain, state, descriptor, limits, work,
    );
    let outcome = staged.and_then(publish);
    // Staging is private scratch: never adopted by name, always removed.
    let _ = std::fs::remove_dir_all(&staging);
    outcome
}

/// The hosted migration runner: the durable workflow of chapter 22 over one
/// hosted source (its S3-class store and caught-up local materialization)
/// and one planned hosted target prefix.
///
/// `db` is the local materialization of the hosted source (the directory
/// `recovery::open_hosted` owns); `scratch_root` holds private staging
/// builds and reuse-verification scratch — never a published namespace.
/// `target_object_epoch` is the initial open object epoch of the target
/// incarnation: the checkpoint and its chunks are staged under it and it is
/// recorded in the genesis (or pre-genesis tombstone) head.
pub struct HostedMigration<'a, S, C> {
    db: &'a Arc<Db<S>>,
    store: &'a C,
    source_prefix: &'a str,
    target_prefix: &'a str,
    target_object_epoch: u64,
    scratch_root: PathBuf,
    limits: Limits,
    policy: CheckpointPolicy,
}

impl<'a, S, C> HostedMigration<'a, S, C>
where
    C: ReceivingStore,
    C::Error: BackendError + ObservedError,
{
    #[expect(
        clippy::too_many_arguments,
        reason = "one explicit construction naming every durable coordinate"
    )]
    #[must_use]
    pub fn new(
        db: &'a Arc<Db<S>>,
        store: &'a C,
        source_prefix: &'a str,
        target_prefix: &'a str,
        target_object_epoch: u64,
        scratch_root: &Path,
        limits: Limits,
        policy: CheckpointPolicy,
    ) -> Self {
        Self {
            db,
            store,
            source_prefix,
            target_prefix,
            target_object_epoch,
            scratch_root: scratch_root.to_path_buf(),
            limits,
            policy,
        }
    }

    fn cutover(&self) -> HostedCutover<'_, C> {
        HostedCutover::new(
            self.store,
            self.source_prefix,
            self.target_prefix,
            self.target_object_epoch,
            self.limits,
        )
    }

    fn cap(&self) -> usize {
        self.limits.envelope_bytes
    }

    fn source_head(
        &self,
        work: Option<&WorkContext>,
    ) -> Result<(HeadVersion, HeadRecord), MigrationError> {
        read_head_record(self.store, self.source_prefix, self.cap(), work)?
            .ok_or(MigrationError::Log(LogError::NotInitialized))
    }

    /// Read-only status against the verified manifest and the durable source
    /// and target heads. Never writes, never initializes, never migrates.
    ///
    /// # Errors
    /// Manifest/chain drift, tombstoned sources and corruption are typed.
    pub fn status(
        &self,
        plans: &Manifest,
        work: &WorkContext,
    ) -> Result<MigrationStatus, MigrationError> {
        work.checkpoint()?;
        let cap = self.cap();
        verify_manifest(plans, cap)?;
        let (_, source) = self.source_head(Some(work))?;
        let chain = read_chain(self.db.as_ref(), cap)?;
        let applied = verify_chain(&chain, plans, cap)?;
        let live = source
            .control
            .live()
            .map_err(|_| MigrationError::Log(LogError::DatabaseDeleted))?;
        if let Access::Frozen { operation, intent } = live.access {
            // Namespace observations come from the durable target head.
            let (target_present, target_cancelled) =
                match read_head_record(self.store, self.target_prefix, cap, Some(work))? {
                    None => (false, false),
                    Some((_, record)) => match record.control.lifecycle {
                        Lifecycle::Live(_) => (true, false),
                        Lifecycle::Deleted { .. } => (false, true),
                    },
                };
            return Ok(MigrationStatus::Frozen {
                operation,
                intent,
                applied,
                target_present,
                target_cancelled,
            });
        }
        let total = plans.entries.len() as u64;
        if applied == total {
            Ok(MigrationStatus::UpToDate { applied })
        } else {
            Ok(MigrationStatus::Pending {
                applied,
                pending: total - applied,
            })
        }
    }

    /// Execute the pending suffix against the hosted source: durable freeze
    /// (composed-head CAS), catch the local materialization up to the frozen
    /// tip, execute the ordered plans, build ONE judged staged target,
    /// upload its complete verified checkpoint (state + history metadata)
    /// under the target's open object epoch, publish the composed genesis
    /// head once by conditional create, and return a durable
    /// `ReadyToSwitch`. The source STAYS frozen; activation is explicit.
    /// `Unknown` means a dispatched step has no proven outcome — nothing is
    /// thawed and the same operation resumes from durable evidence.
    ///
    /// # Errors
    /// The complete typed roster; a failure after freeze leaves the source
    /// frozen (no timer thaws it).
    #[expect(
        clippy::too_many_lines,
        reason = "one bounded resume-or-build hosted migration pipeline"
    )]
    pub fn migrate(
        &self,
        request: &SuffixRequest<'_>,
        work: &WorkContext,
    ) -> Result<HostedOutcome<MigrateOutcome>, MigrationError> {
        work.checkpoint()?;
        let cap = self.cap();
        verify_manifest(request.manifest, cap)?;

        // The durable source head names the source identity and lineage.
        let (_, source) = self.source_head(Some(work))?;
        let source_identity = source.control.identity;
        if request.target_incarnation == source_identity.incarnation_id {
            return Err(MigrationError::IncarnationReused);
        }
        // The local materialization must BE this database before its chain
        // or facts mean anything.
        let local_control =
            read_attachment(self.db.as_ref())?.ok_or(MigrationError::Log(LogError::NotInitialized))?;
        let local_authority = decode_control(&local_control, cap).map_err(LogError::from)?;
        if local_authority.identity != source_identity {
            return Err(MigrationError::Log(LogError::Identity));
        }

        // The applied prefix decides the exact pending suffix (history
        // records are fixed at genesis; catch-up never changes them).
        let chain = read_chain(self.db.as_ref(), cap)?;
        let applied = verify_chain(&chain, request.manifest, cap)?;
        let total = request.manifest.entries.len() as u64;
        if request.steps.is_empty() {
            let live = source.control.live().map_err(LogError::from)?;
            if applied == total && matches!(live.access, Access::Active) {
                return Ok(HostedOutcome::Completed(MigrateOutcome::UpToDate {
                    applied,
                }));
            }
            return Err(MigrationError::WrongSuffix { applied });
        }
        let first =
            usize::try_from(applied).map_err(|_| MigrationError::WrongSuffix { applied })?;
        let plans: Vec<&super::plan::Plan> =
            request.steps.iter().map(|step| &step.plan).collect();
        bind_plans(request.manifest, first, &plans, cap)?;
        let psd = plan_set_digest(request.manifest, first, request.steps.len(), cap)?;

        // Compile the whole suffix before freezing anything.
        let compiled = compile_suffix(&request.source_descriptor, request.steps)?;
        if compiled[0].from_id != source_identity.schema_id {
            return Err(MigrationError::SourceSchemaMismatch);
        }
        let final_schema = compiled.last().expect("nonempty suffix").to_id;
        let target_identity = DatabaseIdentity {
            database_id: request.target_database,
            incarnation_id: request.target_incarnation,
            schema_id: final_schema,
        };
        let last_descriptor = &request.steps.last().expect("nonempty suffix").to_descriptor;
        let expected = ExpectedTarget {
            operation: request.operation,
            psd,
            identity: target_identity,
            descriptor: last_descriptor,
            provenance: GenesisProvenance::Migration {
                source_database: source_identity.database_id,
                source_incarnation: source_identity.incarnation_id,
                plan_set_digest: psd,
            },
            prior_chain: &chain,
            plans: request.manifest,
            expected_applied: applied + request.steps.len() as u64,
        };

        // Resolve recorded target evidence BEFORE freezing: retrying a
        // settled (cancelled/activated) operation never re-freezes anything.
        match probe_target(
            self.store,
            self.target_prefix,
            request.operation,
            psd,
            target_identity,
            cap,
            Some(work),
        )? {
            TargetProbe::AlreadyActivated(outcome) => {
                return Ok(HostedOutcome::Completed(outcome));
            }
            // A live matching target is verified under the held freeze below.
            TargetProbe::Absent | TargetProbe::Reusable(_) => {}
        }

        // Durable freeze under this operation's stable identity.
        let intent = FreezeIntent::Migration {
            plan_set_digest: psd,
            target: request.target_incarnation,
        };
        match self.cutover().freeze_source(request.operation, intent) {
            Ok(HostedOutcome::Completed(())) => {}
            Ok(HostedOutcome::Unknown) => return Ok(HostedOutcome::Unknown),
            // The SAME operation with different plan bytes/target is a
            // plan-set takeover attempt, not a foreign freeze.
            Err(MigrationError::SourceFrozenByOther { operation })
                if operation == request.operation =>
            {
                return Err(MigrationError::PlanSetMismatch);
            }
            Err(error) => return Err(error),
        }

        // Post-freeze evidence: an abort that raced the freeze already holds
        // its durable fence; a published matching target is verified reuse.
        match probe_target(
            self.store,
            self.target_prefix,
            request.operation,
            psd,
            target_identity,
            cap,
            Some(work),
        )? {
            TargetProbe::AlreadyActivated(outcome) => {
                return Ok(HostedOutcome::Completed(outcome));
            }
            TargetProbe::Reusable(record) => {
                return verify_published_target(
                    self.store,
                    self.target_prefix,
                    &record,
                    &expected,
                    &self.scratch_root,
                    self.limits,
                    &self.policy,
                    work,
                )
                .map(HostedOutcome::Completed);
            }
            TargetProbe::Absent => {}
        }

        // Catch the local materialization up to the FROZEN tip through the
        // one landed hosted read-side machine, then capture the final source
        // in one coherent read.
        let history = HostedHistory::open(
            Arc::clone(self.db),
            self.store,
            self.source_prefix.to_string(),
            self.limits,
            work,
        )
        .map_err(MigrationError::from)?;
        let reached = history.catch_up(work).map_err(MigrationError::from)?;
        drop(history);
        let (_, frozen_source) = self.source_head(Some(work))?;
        let source_position = frozen_source
            .control
            .position()
            .ok_or(MigrationError::Log(LogError::DatabaseDeleted))?;
        if reached != source_position.decision {
            return Err(MigrationError::Log(LogError::Backend));
        }
        let state = capture_source(self.db.as_ref(), work)?;
        let (state, _) = execute_steps(state, &compiled, work)?;
        let target_digest = state.digest()?;

        // One Applied record for the whole executed suffix.
        let applied_record = Applied {
            operation: request.operation,
            plan_set_digest: psd,
            source: AppliedSource::Database {
                database: source_identity.database_id,
                incarnation: source_identity.incarnation_id,
                schema: source_identity.schema_id,
                decision: source_position.decision,
                state: source_position.state,
            },
            target_incarnation: request.target_incarnation,
            target_schema: final_schema,
            target_digest,
            steps: applied_steps(request.manifest, first, request.steps.len()),
        };
        let mut target_chain = chain;
        target_chain.push(HistoryRecord::Applied(applied_record.clone()));
        let genesis = GenesisRecord {
            identity: target_identity,
            initial_application_digest: target_digest,
            initial_system_digest: system_digest(&target_chain, cap).map_err(LogError::from)?,
            provenance: GenesisProvenance::Migration {
                source_database: source_identity.database_id,
                source_incarnation: source_identity.incarnation_id,
                plan_set_digest: psd,
            },
        };
        let published = build_and_publish(
            self.store,
            self.target_prefix,
            self.target_object_epoch,
            &self.scratch_root,
            self.limits,
            &self.policy,
            request.operation,
            psd,
            genesis,
            &target_chain,
            &state,
            last_descriptor,
            work,
        )?;
        Ok(match published {
            HostedOutcome::Completed(activation_ref) => {
                HostedOutcome::Completed(MigrateOutcome::ReadyToSwitch {
                    activation_ref,
                    applied: applied_record,
                })
            }
            HostedOutcome::Unknown => HostedOutcome::Unknown,
        })
    }

    /// Explicit hosted activation via the target head CAS; matching retries
    /// return recorded evidence with the CURRENT access mode.
    ///
    /// # Errors
    /// Wrong/stale references and cancelled targets are typed refusals.
    pub fn activate(
        &self,
        reference: &ActivationRef,
    ) -> Result<HostedOutcome<ActivateReport>, MigrationError> {
        self.cutover().activate(reference)
    }

    /// Complete hosted abort: durably fence the planned target FIRST
    /// (terminal deletion, or the composed pre-genesis cancellation
    /// tombstone), THEN thaw the matching frozen source. `Unknown` at either
    /// stage stops with the source frozen; a retry under the same operation
    /// resumes from durable evidence.
    ///
    /// # Errors
    /// `ActivationWon`, foreign operations/plan sets and conflicting
    /// evidence refuse automatic abort/thaw.
    pub fn abort(
        &self,
        planned_target: DatabaseIdentity,
        operation: OperationId,
        psd: [u8; 32],
    ) -> Result<HostedOutcome<AbortReport>, MigrationError> {
        let (_, source) = self.source_head(None)?;
        let source_identity = source.control.identity;
        // A frozen source binds the abort to the exact operation and plan
        // set; a foreign freeze or different plan bytes refuse before any
        // fence is attempted.
        if let Lifecycle::Live(live) = &source.control.lifecycle
            && let Access::Frozen {
                operation: held,
                intent,
            } = live.access
        {
            if held != operation {
                return Err(MigrationError::SourceFrozenByOther { operation: held });
            }
            match intent {
                FreezeIntent::Migration {
                    plan_set_digest,
                    target,
                } if plan_set_digest == psd && target == planned_target.incarnation_id => {}
                _ => return Err(MigrationError::PlanSetMismatch),
            }
        }
        let reason = DeletedReason::MigrationAborted {
            source_database: source_identity.database_id,
            source_incarnation: source_identity.incarnation_id,
            plan_set_digest: psd,
        };
        self.cutover().abort(planned_target, operation, reason)
    }
}

/// Explicit hosted initialization: execute the generated chain from its
/// declared EMPTY base (canonical seeds run exactly once) into a brand-new
/// hosted incarnation — ONE judged staged target, its complete verified
/// checkpoint (state + the one `Applied` history record) uploaded under the
/// initial object epoch, and the composed genesis head published once by
/// conditional create. The result is `ReadyToSwitch`; activation is the same
/// explicit step ([`HostedCutover::activate`] / [`HostedMigration::activate`]).
/// There is NO empty-latest-schema entrypoint that could mark skipped seeds
/// applied; a fresh host later hydrates the incarnation from the store alone.
///
/// # Errors
/// The complete typed roster; nothing is retried implicitly.
#[expect(
    clippy::too_many_arguments,
    reason = "one explicit entrypoint naming every durable coordinate"
)]
pub fn initialize<C>(
    store: &C,
    target_prefix: &str,
    target_object_epoch: u64,
    scratch_root: &Path,
    request: &SuffixRequest<'_>,
    limits: Limits,
    policy: &CheckpointPolicy,
    work: &WorkContext,
) -> Result<HostedOutcome<MigrateOutcome>, MigrationError>
where
    C: ReceivingStore,
    C::Error: BackendError + ObservedError,
{
    work.checkpoint()?;
    let cap = limits.envelope_bytes;
    verify_manifest(request.manifest, cap)?;
    if request.steps.is_empty() {
        return Err(MigrationError::WrongSuffix { applied: 0 });
    }
    let plans: Vec<&super::plan::Plan> = request.steps.iter().map(|step| &step.plan).collect();
    bind_plans(request.manifest, 0, &plans, cap)?;
    let psd = plan_set_digest(request.manifest, 0, request.steps.len(), cap)?;
    let compiled = compile_suffix(&request.source_descriptor, request.steps)?;
    if compiled[0].from_id != request.manifest.base_schema {
        return Err(MigrationError::SourceSchemaMismatch);
    }
    let final_schema = compiled.last().expect("nonempty").to_id;
    let target_identity = DatabaseIdentity {
        database_id: request.target_database,
        incarnation_id: request.target_incarnation,
        schema_id: final_schema,
    };
    let last_descriptor = &request.steps.last().expect("nonempty").to_descriptor;

    // Recorded evidence first: reuse-or-refuse follows the migration rules.
    match probe_target(
        store,
        target_prefix,
        request.operation,
        psd,
        target_identity,
        cap,
        Some(work),
    )? {
        TargetProbe::AlreadyActivated(outcome) => return Ok(HostedOutcome::Completed(outcome)),
        TargetProbe::Reusable(record) => {
            let expected = ExpectedTarget {
                operation: request.operation,
                psd,
                identity: target_identity,
                descriptor: last_descriptor,
                provenance: GenesisProvenance::Create,
                prior_chain: &[],
                plans: request.manifest,
                expected_applied: request.steps.len() as u64,
            };
            return verify_published_target(
                store,
                target_prefix,
                &record,
                &expected,
                scratch_root,
                limits,
                policy,
                work,
            )
            .map(HostedOutcome::Completed);
        }
        TargetProbe::Absent => {}
    }

    // Execute the whole chain from the declared empty base (seeds included).
    let (state, _) = execute_steps(MigrationState::empty(), &compiled, work)?;
    let target_digest = state.digest()?;
    let applied_record = Applied {
        operation: request.operation,
        plan_set_digest: psd,
        source: AppliedSource::EmptyBase {
            base_schema: request.manifest.base_schema,
        },
        target_incarnation: request.target_incarnation,
        target_schema: final_schema,
        target_digest,
        steps: applied_steps(request.manifest, 0, request.steps.len()),
    };
    let target_chain = vec![HistoryRecord::Applied(applied_record.clone())];
    let genesis = GenesisRecord {
        identity: target_identity,
        initial_application_digest: target_digest,
        initial_system_digest: system_digest(&target_chain, cap).map_err(LogError::from)?,
        provenance: GenesisProvenance::Create,
    };
    let published = build_and_publish(
        store,
        target_prefix,
        target_object_epoch,
        scratch_root,
        limits,
        policy,
        request.operation,
        psd,
        genesis,
        &target_chain,
        &state,
        last_descriptor,
        work,
    )?;
    Ok(match published {
        HostedOutcome::Completed(activation_ref) => {
            HostedOutcome::Completed(MigrateOutcome::ReadyToSwitch {
                activation_ref,
                applied: applied_record,
            })
        }
        HostedOutcome::Unknown => HostedOutcome::Unknown,
    })
}
