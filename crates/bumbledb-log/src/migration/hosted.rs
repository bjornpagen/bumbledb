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

use crate::history::authority::{
    Access, ActivateOutcome, Activation, ActivationCause, DeleteOutcome, DeletedReason,
    FreezeIntent, FreezeOutcome, HeadAuthority, Lifecycle,
};
use crate::history::command::Limits;
use crate::history::{AccessMode, DatabaseIdentity, OperationId};
use crate::manifest::{self, HeadRecord};
use crate::writer::LogError;
use crate::writer::verbs::{ConditionalOutcome, ConditionalStore, HeadRead, HeadVersion};

use super::executor::{ActivateReport, ActivationRef, MigrationError, TargetFence};

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

impl<'a, C: ConditionalStore> HostedCutover<'a, C> {
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
        match self
            .store
            .read_head(&head_key(prefix))
            .map_err(|_| MigrationError::Log(LogError::Backend))?
        {
            HeadRead::Absent => Ok(None),
            HeadRead::Present { version, body } => {
                let record = manifest::decode_head(&body, self.limits.envelope_bytes)
                    .map_err(LogError::from)?;
                Ok(Some((version, record)))
            }
        }
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
                Err(error) => return Err(MigrationError::Log(LogError::from(error))),
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
                Err(error) => return Err(MigrationError::Log(LogError::from(error))),
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
                        Err(error) => return Err(MigrationError::Log(LogError::from(error))),
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
