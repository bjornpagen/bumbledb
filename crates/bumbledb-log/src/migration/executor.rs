//! The one native migration runner for `LocalHistory` authorities (C11).
//!
//! One operation plans the whole pending suffix against ONE frozen source,
//! builds ONE final staged target through the core checked builder, verifies
//! complete admission with the core judge, publishes genesis + inherited
//! history + one `Applied` record together under `Frozen/AwaitingCutover`,
//! and returns a durable `ReadyToSwitch`. Activation is explicit and
//! one-time. Abort durably fences the target BEFORE thawing the matching
//! source. Every entrypoint takes the caller's stable `OperationId`, fixed
//! before dispatch; resume never substitutes plan bytes; ordinary open
//! (`LocalHistory::open`) never initializes or migrates and `status` writes
//! nothing.

use std::path::Path;

use bumbledb::integration::HostRecordChange;
use bumbledb::scalar::ScalarEvaluator;
use bumbledb::schema::{SchemaDescriptor, ValidateDescriptor as _};
use bumbledb::schema::RelationId;
use bumbledb::{Admission, ChangeSet, Db, Violations, WorkContext, WorkError};
use crate::recovery::{begin_staged, StagedPopulation};

use crate::history::authority::{
    Access, ActivateOutcome, Activation, ActivationCause, DeleteOutcome, DeletedReason,
    FreezeIntent, FreezeOutcome, HeadAuthority, Lifecycle, decode_control, encode_control,
};
use crate::history::command::Limits;
use crate::history::decision::{GenesisProvenance, GenesisRecord, genesis_stamp};
use crate::history::{
    AccessMode, DatabaseId, DatabaseIdentity, DecisionDigest, FrameError, IncarnationId,
    OperationId, SchemaId,
};
use crate::writer::{LocalHistory, LogError};

use super::compile::{CompileError, CompiledPlan, compile};
use super::history::{
    Applied, AppliedSource, AppliedStep, HistoryError, HistoryRecord, encode_record, history_key,
    system_digest, verify_chain,
};
use super::lock::{NamespaceError, TargetNamespace};
use super::manifest::{Manifest, ManifestError, bind_plans, plan_set_digest, verify_manifest};
use super::plan::{Plan, PlanError};
use super::state::{MigrationState, StateError};

/// The executor's complete typed refusal/failure roster.
#[derive(Debug)]
pub enum MigrationError {
    Manifest(ManifestError),
    History(HistoryError),
    Plan(PlanError),
    Compile(CompileError),
    State(StateError),
    Namespace(NamespaceError),
    Log(LogError),
    Frame(FrameError),
    Work(WorkError),
    Core(bumbledb::Error),
    /// The core admission machinery rejected the final built state — the
    /// complete violation set, never target activation.
    AdmissionRejected(Violations),
    /// The source is frozen by a DIFFERENT operation.
    SourceFrozenByOther {
        operation: OperationId,
    },
    /// The frozen operation matches but the plan set does not: different
    /// plan bytes cannot take over a frozen operation by reusing a label.
    PlanSetMismatch,
    /// The source database's schema is not the pending suffix's source.
    SourceSchemaMismatch,
    /// This operation was durably cancelled; it permanently reports Aborted
    /// and can never resume target creation or activation.
    Aborted {
        operation: OperationId,
    },
    /// Activation already won this operation; automatic abort/thaw refuses.
    ActivationWon,
    /// The target namespace holds evidence of a DIFFERENT operation/plan.
    TargetConflict,
    /// Same operation/source/plan bytes, conflicting completed output.
    OutputMismatch,
    /// The activation reference does not name the published target's
    /// recorded evidence (wrong/stale ref).
    StaleActivationRef,
    /// The planned target incarnation must be a new lineage.
    IncarnationReused,
    /// The requested suffix is empty while pending work exists, or does not
    /// start at the applied prefix.
    WrongSuffix {
        applied: u64,
    },
    /// The hosted data plane's checkpoint capture/upload/fetch failed
    /// (target checkpoint publication or reuse verification).
    Checkpoint(crate::checkpointer::CheckpointError),
    /// Hydrating a published hosted target's recovery objects for reuse
    /// verification failed.
    Hydration(crate::recovery::RecoveryError),
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "migration: {self:?}")
    }
}

impl std::error::Error for MigrationError {}

macro_rules! from_error {
    ($variant:ident, $source:ty) => {
        impl From<$source> for MigrationError {
            fn from(error: $source) -> Self {
                Self::$variant(error)
            }
        }
    };
}

from_error!(Manifest, ManifestError);
from_error!(History, HistoryError);
from_error!(Plan, PlanError);
from_error!(Compile, CompileError);
from_error!(State, StateError);
from_error!(Namespace, NamespaceError);
from_error!(Frame, FrameError);
from_error!(Work, WorkError);
from_error!(Core, bumbledb::Error);
from_error!(Checkpoint, crate::checkpointer::CheckpointError);
from_error!(Hydration, crate::recovery::RecoveryError);

impl From<LogError> for MigrationError {
    fn from(error: LogError) -> Self {
        match error {
            // Bounded work/deadline/cancellation is ONE typed resource
            // refusal no matter which layer charged it: the SDK maps
            // `MigrationError::Work` to the exact core reason, while a
            // nested `Log(Work)` would be respelled as migration drift.
            LogError::Work(work) => Self::Work(work),
            other => Self::Log(other),
        }
    }
}

impl From<bumbledb::integration::IntegrationError> for MigrationError {
    fn from(error: bumbledb::integration::IntegrationError) -> Self {
        Self::from(LogError::from(error))
    }
}

impl From<crate::history::authority::AuthorityError> for MigrationError {
    fn from(error: crate::history::authority::AuthorityError) -> Self {
        Self::from(LogError::from(error))
    }
}

/// One pending step's inputs: the canonical plan and the ACTUAL target
/// schema snapshot it produces (`meta/NNNN.schema.json`). The plan's
/// recorded schema ids must fingerprint these descriptors exactly.
pub struct StepInput {
    pub plan: Plan,
    pub to_descriptor: SchemaDescriptor,
}

/// One migration request: the stable operation ref exists BEFORE dispatch
/// and the planned final target incarnation is fixed with it.
pub struct SuffixRequest<'a> {
    pub operation: OperationId,
    pub manifest: &'a Manifest,
    /// The source schema snapshot at the applied prefix.
    pub source_descriptor: SchemaDescriptor,
    /// The contiguous pending steps, starting exactly at the applied prefix.
    pub steps: &'a [StepInput],
    pub target_database: DatabaseId,
    pub target_incarnation: IncarnationId,
}

/// The durable activation reference `ReadyToSwitch` carries: it binds the
/// operation, plan set, final target identity and its genesis digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivationRef {
    pub operation: OperationId,
    pub plan_set_digest: [u8; 32],
    pub target: DatabaseIdentity,
    pub target_genesis: DecisionDigest,
}

/// Read-only status. Drift/ahead/corruption surface as typed errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationStatus {
    UpToDate {
        applied: u64,
    },
    Pending {
        applied: u64,
        pending: u64,
    },
    /// The source is frozen. `target_present`/`target_cancelled` are
    /// namespace observations, not activation permission.
    Frozen {
        operation: OperationId,
        intent: FreezeIntent,
        applied: u64,
        target_present: bool,
        target_cancelled: bool,
    },
}

/// The migrate outcome: complete (`ReadyToSwitch`), evidence of a prior
/// activation, or nothing pending. Refusals are errors.
#[expect(
    clippy::large_enum_variant,
    reason = "ts/crate log_wire/admin.rs matches these variant shapes; \
              ReadyToSwitch carries the fixed-size activation reference by \
              value on one frame"
)]
#[derive(Debug)]
pub enum MigrateOutcome {
    UpToDate {
        applied: u64,
    },
    /// The final target is published, verified and STILL FROZEN. Activation
    /// is a separate explicit call with this reference.
    ReadyToSwitch {
        activation_ref: ActivationRef,
        applied: Applied,
    },
    AlreadyActivated {
        activation: Activation,
        access: AccessMode,
    },
}

/// Activation evidence: the recorded one-time marker plus the target's
/// CURRENT access mode. A matching retry never thaws a later freeze or
/// revives a deleted authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivateReport {
    pub activation: Activation,
    pub access: AccessMode,
}

/// How the target was durably fenced during abort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFence {
    /// No genesis existed; a pre-genesis tombstone now fences delayed
    /// genesis/installation.
    TombstonePreGenesis,
    /// The published unactivated target's control is now terminally Deleted.
    TargetDeleted,
    /// Matching terminal evidence already existed (idempotent retry).
    AlreadyFenced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbortReport {
    pub fence: TargetFence,
    /// Whether THIS call thawed the source (false on an evidence-only retry
    /// after the source was already thawed).
    pub thawed: bool,
}

/// One abort request. The target descriptor is required so a published
/// target's control can be read and terminally cancelled.
pub struct AbortRequest<'a> {
    pub operation: OperationId,
    pub plan_set_digest: [u8; 32],
    pub target_database: DatabaseId,
    pub target_incarnation: IncarnationId,
    pub target_schema: SchemaId,
    pub target_descriptor: &'a SchemaDescriptor,
}

/// The local migration runner over one opened source authority.
pub struct LocalMigration<'h, S> {
    source: &'h LocalHistory<S>,
    /// Owned so the runner cannot dangle a caller's temporary path.
    targets_root: std::path::PathBuf,
    limits: Limits,
}

impl<'h, S> LocalMigration<'h, S> {
    #[must_use]
    pub fn new(source: &'h LocalHistory<S>, targets_root: &Path, limits: Limits) -> Self {
        Self {
            source,
            targets_root: targets_root.to_path_buf(),
            limits,
        }
    }

    /// Read-only status against the verified manifest. Never writes, never
    /// initializes, never migrates.
    /// # Errors
    /// Manifest/chain drift, ahead databases and corruption are typed.
    pub fn status(
        &self,
        manifest: &Manifest,
        work: &WorkContext,
    ) -> Result<MigrationStatus, MigrationError> {
        work.checkpoint()?;
        verify_manifest(manifest, self.limits.envelope_bytes)?;
        let authority = self.source.authority()?;
        let chain = read_chain(self.source.db(), self.limits.envelope_bytes)?;
        let applied = verify_chain(&chain, manifest, self.limits.envelope_bytes)?;
        let live = authority
            .live()
            .map_err(|_| MigrationError::Log(LogError::DatabaseDeleted))?;
        if let Access::Frozen { operation, intent } = live.access {
            let (target_present, target_cancelled) = match intent {
                FreezeIntent::Migration { target, .. } => {
                    let namespace = TargetNamespace::new(&self.targets_root, target)?;
                    (
                        namespace.target_exists(),
                        namespace
                            .read_tombstone(self.limits.envelope_bytes)?
                            .is_some(),
                    )
                }
                FreezeIntent::Erasure => (false, false),
            };
            return Ok(MigrationStatus::Frozen {
                operation,
                intent,
                applied,
                target_present,
                target_cancelled,
            });
        }
        let total = manifest.entries.len() as u64;
        if applied == total {
            Ok(MigrationStatus::UpToDate { applied })
        } else {
            Ok(MigrationStatus::Pending {
                applied,
                pending: total - applied,
            })
        }
    }

    /// Execute the pending suffix: freeze durably, build one staged final
    /// target, publish it Frozen/AwaitingCutover, return `ReadyToSwitch`.
    /// Interruption at any point leaves a resumable durable state; calling
    /// again with the SAME operation and plan bytes resumes or reuses.
    /// # Errors
    /// The complete typed roster; a failure after freeze leaves the source
    /// frozen (no timer thaws it).
    #[expect(
        clippy::too_many_lines,
        reason = "one bounded resume-or-build migration pipeline"
    )]
    pub fn migrate(
        &self,
        request: &SuffixRequest<'_>,
        work: &WorkContext,
    ) -> Result<MigrateOutcome, MigrationError> {
        work.checkpoint()?;
        let cap = self.limits.envelope_bytes;
        verify_manifest(request.manifest, cap)?;
        let source_identity = self.source.identity();
        if request.target_incarnation == source_identity.incarnation_id {
            return Err(MigrationError::IncarnationReused);
        }

        // The applied prefix decides the exact pending suffix.
        let chain = read_chain(self.source.db(), cap)?;
        let applied = verify_chain(&chain, request.manifest, cap)?;
        let total = request.manifest.entries.len() as u64;
        if request.steps.is_empty() {
            if applied == total
                && !matches!(
                    self.source.authority()?.live()?.access,
                    Access::Frozen { .. }
                )
            {
                return Ok(MigrateOutcome::UpToDate { applied });
            }
            return Err(MigrationError::WrongSuffix { applied });
        }
        let first =
            usize::try_from(applied).map_err(|_| MigrationError::WrongSuffix { applied })?;
        let plans: Vec<&Plan> = request.steps.iter().map(|step| &step.plan).collect();
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

        // Resolve the target namespace BEFORE freezing: a durable
        // cancellation permanently reports Aborted, and retrying a
        // cancelled operation must not re-freeze a thawed source.
        let namespace = TargetNamespace::new(&self.targets_root, request.target_incarnation)?;
        let refuse_tombstone = |tombstone: HeadAuthority| match tombstone.lifecycle {
            Lifecycle::Deleted { operation, .. } if operation == request.operation => {
                MigrationError::Aborted { operation }
            }
            _ => MigrationError::TargetConflict,
        };
        if let Some(tombstone) = namespace.read_tombstone(cap)? {
            return Err(refuse_tombstone(tombstone));
        }
        // Likewise a published target that is already terminal or already
        // activated is pure recorded evidence: report it without freezing
        // (a matching retry never thaws OR re-freezes anything).
        let last_descriptor = &request.steps.last().expect("nonempty suffix").to_descriptor;
        if let Some(outcome) = published_terminal_evidence(
            &namespace,
            request.operation,
            target_identity,
            last_descriptor,
            cap,
        )? {
            return Ok(outcome);
        }

        // Freeze durably under this operation, or verify the existing
        // freeze is EXACTLY this operation and plan set.
        let intent = FreezeIntent::Migration {
            plan_set_digest: psd,
            target: request.target_incarnation,
        };
        self.freeze_source(request.operation, intent, work)?;

        // Re-check after the freeze: an abort that raced between the check
        // and the freeze already holds its durable fence, so this attempt
        // stops here (frozen and safely resumable — the abort retry thaws;
        // the locked no-overwrite install would refuse the tombstone anyway).
        if let Some(tombstone) = namespace.read_tombstone(cap)? {
            return Err(refuse_tombstone(tombstone));
        }
        if namespace.target_exists() {
            return self.reuse_published_target(
                &namespace,
                request,
                &chain,
                psd,
                target_identity,
                last_descriptor,
                work,
            );
        }

        // Capture the exact frozen source and execute the ordered suffix.
        let state = capture_source(self.source.db(), work)?;
        let (state, _) = execute_steps(state, &compiled, work)?;
        let target_digest = state.digest()?;

        // One Applied record for the whole executed suffix.
        let source_position = self
            .source
            .authority()?
            .position()
            .ok_or(MigrationError::Log(LogError::DatabaseDeleted))?;
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
            initial_system_digest: system_digest(&target_chain, cap)?,
            provenance: GenesisProvenance::Migration {
                source_database: source_identity.database_id,
                source_incarnation: source_identity.incarnation_id,
                plan_set_digest: psd,
            },
        };
        let activation_ref = self.build_and_install(
            &namespace,
            request.operation,
            psd,
            genesis,
            &target_chain,
            &state,
            last_descriptor,
            work,
        )?;
        Ok(MigrateOutcome::ReadyToSwitch {
            activation_ref,
            applied: applied_record,
        })
    }

    /// Durable abort: fence the target FIRST (terminal control deletion or
    /// pre-genesis tombstone under the stable namespace lock), THEN thaw
    /// the matching frozen source. Uncertainty never authorizes thaw; a
    /// won activation refuses; a cancelled operation stays Aborted forever.
    /// # Errors
    /// `ActivationWon`, operation mismatches and storage failures.
    pub fn abort(
        &self,
        request: &AbortRequest<'_>,
        work: &WorkContext,
    ) -> Result<AbortReport, MigrationError> {
        work.checkpoint()?;
        let source_identity = self.source.identity();
        let authority = self.source.authority()?;
        let live = authority.live()?;
        let held = match live.access {
            Access::Frozen { operation, intent } => {
                if operation != request.operation {
                    return Err(MigrationError::SourceFrozenByOther { operation });
                }
                match intent {
                    FreezeIntent::Migration {
                        plan_set_digest,
                        target,
                    } if plan_set_digest == request.plan_set_digest
                        && target == request.target_incarnation =>
                    {
                        true
                    }
                    _ => return Err(MigrationError::PlanSetMismatch),
                }
            }
            Access::Active => false,
        };

        let reason = DeletedReason::MigrationAborted {
            source_database: source_identity.database_id,
            source_incarnation: source_identity.incarnation_id,
            plan_set_digest: request.plan_set_digest,
        };
        let planned_identity = DatabaseIdentity {
            database_id: request.target_database,
            incarnation_id: request.target_incarnation,
            schema_id: request.target_schema,
        };
        let fence = fence_target(
            &self.targets_root,
            planned_identity,
            request.operation,
            reason,
            request.target_descriptor,
            self.limits,
            work,
        )?;

        // Only after the irreversible fence is known durable may the
        // matching frozen source thaw. The transition re-verifies the held
        // operation inside the exclusive session.
        let thawed = if held {
            with_authority(self.source.db(), self.limits, work, |authority| {
                let live = authority.live().map_err(LogError::from)?;
                match live.access {
                    Access::Frozen { operation, .. } if operation == request.operation => {
                        let thawed = authority.thaw(request.operation).map_err(LogError::from)?;
                        Ok(Transition::Commit(thawed, true))
                    }
                    _ => Ok(Transition::Keep(false)),
                }
            })?
        } else {
            // Evidence-only retry: the fence exists and the source is
            // already thawed (or was never frozen by this operation).
            false
        };
        Ok(AbortReport { fence, thawed })
    }

    fn freeze_source(
        &self,
        operation: OperationId,
        intent: FreezeIntent,
        work: &WorkContext,
    ) -> Result<(), MigrationError> {
        // The exclusive writer session is held from the authority read
        // through the control commit: freeze waits behind an admitted
        // transaction, prevents further ones, and cannot lose an update.
        with_authority(self.source.db(), self.limits, work, |authority| {
            match authority.freeze(operation, intent) {
                Ok(FreezeOutcome::Frozen(frozen)) => Ok(Transition::Commit(frozen, ())),
                Ok(FreezeOutcome::AlreadyFrozen { .. }) => Ok(Transition::Keep(())),
                Err(crate::history::authority::AuthorityError::OperationMismatch { held }) => {
                    // The SAME operation with different plan bytes/target is
                    // a plan-set takeover attempt, not a foreign freeze.
                    Err(if held == operation {
                        MigrationError::PlanSetMismatch
                    } else {
                        MigrationError::SourceFrozenByOther { operation: held }
                    })
                }
                Err(error) => Err(MigrationError::from(LogError::from(error))),
            }
        })
    }

    /// Resume: a complete published target for this exact operation and
    /// plan set is verified and reused, never regenerated.
    #[expect(
        clippy::too_many_arguments,
        reason = "one private resume path; grouping would only rename the coupling"
    )]
    fn reuse_published_target(
        &self,
        namespace: &TargetNamespace,
        request: &SuffixRequest<'_>,
        source_chain: &[HistoryRecord],
        psd: [u8; 32],
        target_identity: DatabaseIdentity,
        last_descriptor: &SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<MigrateOutcome, MigrationError> {
        let cap = self.limits.envelope_bytes;
        let target_db: Db<SchemaDescriptor> =
            Db::open(&namespace.target_dir(), last_descriptor.clone())?;
        let control = read_attachment(&target_db)?.ok_or(MigrationError::TargetConflict)?;
        let authority = decode_control(&control, cap).map_err(LogError::from)?;
        if authority.identity != target_identity {
            return Err(MigrationError::TargetConflict);
        }
        if let Activation::Activated { operation, .. } = authority.activation {
            if operation == request.operation {
                let access = match &authority.lifecycle {
                    Lifecycle::Live(live) => live.access.mode(),
                    Lifecycle::Deleted { .. } => AccessMode::Deleted,
                };
                return Ok(MigrateOutcome::AlreadyActivated {
                    activation: authority.activation,
                    access,
                });
            }
            return Err(MigrationError::TargetConflict);
        }
        match &authority.lifecycle {
            Lifecycle::Deleted { operation, .. } => {
                return Err(if *operation == request.operation {
                    MigrationError::Aborted {
                        operation: *operation,
                    }
                } else {
                    MigrationError::TargetConflict
                });
            }
            Lifecycle::Live(live) => match live.access {
                Access::Frozen { operation, intent } => {
                    if operation != request.operation {
                        return Err(MigrationError::TargetConflict);
                    }
                    match intent {
                        FreezeIntent::Migration {
                            plan_set_digest,
                            target,
                        } if plan_set_digest == psd && target == request.target_incarnation => {}
                        _ => return Err(MigrationError::PlanSetMismatch),
                    }
                }
                Access::Active => return Err(MigrationError::TargetConflict),
            },
        }
        // Verify the recorded output: chain extension plus the actual
        // canonical state digest. Same operation/source/plan with
        // conflicting completed output refuses (never overwrite).
        let target_chain = read_chain(&target_db, cap)?;
        if target_chain.len() != source_chain.len() + 1 {
            return Err(MigrationError::OutputMismatch);
        }
        let Some(HistoryRecord::Applied(applied_record)) = target_chain.last() else {
            return Err(MigrationError::OutputMismatch);
        };
        if applied_record.operation != request.operation || applied_record.plan_set_digest != psd {
            return Err(MigrationError::OutputMismatch);
        }
        let schema = last_descriptor
            .clone()
            .validate()
            .map_err(bumbledb::Error::from)?;
        let mut recomputed: Option<[u8; 32]> = None;
        {
            let mut captured = None;
            target_db.read(|read| {
                captured = Some(MigrationState::from_source(read, &schema, work));
                Ok(())
            })?;
            if let Some(state) = captured {
                recomputed = Some(state?.digest()?);
            }
        }
        if recomputed != Some(applied_record.target_digest) {
            return Err(MigrationError::OutputMismatch);
        }
        let live = authority.live()?;
        if live.decision.seq != 0 {
            return Err(MigrationError::OutputMismatch);
        }
        Ok(MigrateOutcome::ReadyToSwitch {
            activation_ref: ActivationRef {
                operation: request.operation,
                plan_set_digest: psd,
                target: target_identity,
                target_genesis: live.decision.hash,
            },
            applied: applied_record.clone(),
        })
    }

    /// Build the final target in private staging and durably install it
    /// under the stable namespace lock (no-overwrite, tombstone-refusing).
    #[expect(
        clippy::too_many_arguments,
        reason = "one private build path shared by migrate and initialize"
    )]
    fn build_and_install(
        &self,
        namespace: &TargetNamespace,
        operation: OperationId,
        psd: [u8; 32],
        genesis: GenesisRecord,
        target_chain: &[HistoryRecord],
        state: &MigrationState,
        descriptor: &SchemaDescriptor,
        work: &WorkContext,
    ) -> Result<ActivationRef, MigrationError> {
        build_and_install(
            namespace,
            operation,
            psd,
            genesis,
            target_chain,
            state,
            descriptor,
            self.limits,
            work,
        )
    }
}

/// Explicit activation of a published frozen target. Verifies the reference
/// against recorded evidence and atomically transitions the target to
/// Active with its one-time activation marker in the SAME control commit.
/// A matching retry returns the recorded evidence plus the CURRENT access
/// mode without mutating anything.
/// # Errors
/// Wrong/stale references, cancelled targets and storage failures.
pub fn activate_target(
    targets_root: &Path,
    reference: &ActivationRef,
    target_descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<ActivateReport, MigrationError> {
    work.checkpoint()?;
    let cap = limits.envelope_bytes;
    let namespace = TargetNamespace::new(targets_root, reference.target.incarnation_id)?;
    // Activation and abort exclude each other on the SAME stable namespace
    // lock, so a paused activation can never race a fence across handles.
    let lock = namespace.lock()?;
    if let Some(tombstone) = namespace.read_tombstone(cap)? {
        drop(lock);
        return Err(match tombstone.lifecycle {
            Lifecycle::Deleted { operation, .. } if operation == reference.operation => {
                MigrationError::Aborted { operation }
            }
            _ => MigrationError::TargetConflict,
        });
    }
    if !namespace.target_exists() {
        drop(lock);
        return Err(MigrationError::StaleActivationRef);
    }
    let target_db: Db<SchemaDescriptor> =
        Db::open(&namespace.target_dir(), target_descriptor.clone())?;
    let report = with_authority(&target_db, limits, work, |authority| {
        if authority.identity != reference.target {
            return Err(MigrationError::StaleActivationRef);
        }
        // The genesis binding: a wrong/stale reference refuses before any
        // transition is attempted.
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
            Ok(ActivateOutcome::Activated(activated)) => {
                let report = ActivateReport {
                    activation: activated.activation,
                    access: AccessMode::Active,
                };
                Ok(Transition::Commit(activated, report))
            }
            Ok(ActivateOutcome::AlreadyActivated { activation, access }) => {
                Ok(Transition::Keep(ActivateReport { activation, access }))
            }
            Err(crate::history::authority::AuthorityError::Deleted) => {
                Err(MigrationError::Aborted {
                    operation: reference.operation,
                })
            }
            Err(crate::history::authority::AuthorityError::OperationMismatch { .. }) => {
                Err(MigrationError::TargetConflict)
            }
            Err(crate::history::authority::AuthorityError::ActivationEvidenceMismatch) => {
                Err(MigrationError::StaleActivationRef)
            }
            Err(error) => Err(MigrationError::from(LogError::from(error))),
        }
    })?;
    // Durable readable copy of the one-time activation, beside the tombstone
    // in the stable namespace: recorded evidence for probes that run while a
    // live owner later holds the activated store open. The control commit
    // above is the authority; a crash before this write heals on the next
    // matching activate retry (the Keep path re-records it).
    let recorded =
        read_attachment(&target_db)?.ok_or(MigrationError::Log(LogError::NotInitialized))?;
    let recorded = decode_control(&recorded, cap).map_err(LogError::from)?;
    if matches!(recorded.activation, Activation::Activated { .. }) {
        namespace.record_activation(&lock, &recorded, cap)?;
    }
    drop(lock);
    Ok(report)
}

/// Durably fence one planned target against activation AND delayed genesis:
/// terminal control deletion for a published target, or the pre-genesis
/// tombstone under the same stable namespace lock genesis installation
/// uses. Returns only after the fence is durable. Never touches a source.
/// # Errors
/// `ActivationWon` when activation already won; conflicts are typed.
pub fn fence_target(
    targets_root: &Path,
    planned_identity: DatabaseIdentity,
    operation: OperationId,
    reason: DeletedReason,
    target_descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<TargetFence, MigrationError> {
    work.checkpoint()?;
    let cap = limits.envelope_bytes;
    let namespace = TargetNamespace::new(targets_root, planned_identity.incarnation_id)?;
    let lock = namespace.lock()?;
    if let Some(existing) = namespace.read_tombstone(cap)? {
        return match existing.lifecycle {
            Lifecycle::Deleted {
                operation: held, ..
            } if held == operation => Ok(TargetFence::AlreadyFenced),
            _ => Err(MigrationError::TargetConflict),
        };
    }
    if namespace.target_exists() {
        let target_db: Db<SchemaDescriptor> =
            match Db::open(&namespace.target_dir(), target_descriptor.clone()) {
                Ok(db) => db,
                // A live owner holds the published target open. Recorded
                // activation evidence means activation already won this
                // namespace (a served target is never automatically
                // aborted); without it, the typed open refusal stands.
                Err(error) if store_locked(&error) => {
                    if let Some(recorded) = namespace.read_activation(cap)? {
                        return Err(if recorded.identity == planned_identity {
                            MigrationError::ActivationWon
                        } else {
                            MigrationError::TargetConflict
                        });
                    }
                    return Err(error.into());
                }
                Err(error) => return Err(error.into()),
            };
        return with_authority(&target_db, limits, work, |authority| {
            if authority.identity != planned_identity {
                return Err(MigrationError::TargetConflict);
            }
            match authority.delete(operation, reason) {
                Ok(DeleteOutcome::Deleted(deleted)) => {
                    Ok(Transition::Commit(deleted, TargetFence::TargetDeleted))
                }
                Ok(DeleteOutcome::AlreadyDeleted { .. }) => {
                    Ok(Transition::Keep(TargetFence::AlreadyFenced))
                }
                Err(crate::history::authority::AuthorityError::ActivationEvidenceMismatch) => {
                    Err(MigrationError::ActivationWon)
                }
                Err(crate::history::authority::AuthorityError::OperationMismatch { .. }) => {
                    Err(MigrationError::TargetConflict)
                }
                Err(error) => Err(MigrationError::from(LogError::from(error))),
            }
        });
    }
    // Absent target: the durable pre-genesis cancellation, installed and
    // fsynced under the lock BEFORE any genesis can exist. A paused
    // installer wakes to find its no-overwrite install refused.
    let tombstone = HeadAuthority::cancelled_before_genesis(planned_identity, operation, reason);
    namespace.install_tombstone(&lock, &tombstone, cap)?;
    drop(lock);
    Ok(TargetFence::TombstonePreGenesis)
}

/// Explicit initialization: execute the generated chain from its declared
/// EMPTY base (including canonical seeds) into a brand-new incarnation.
/// The result is a published Frozen/AwaitingCutover target with one Applied
/// record covering the executed chain; activation is the same explicit
/// step. Creating an empty latest-schema database without running seeds is
/// not an operation this module offers.
/// # Errors
/// The complete typed roster; nothing is retried implicitly.
pub fn initialize(
    targets_root: &Path,
    request: &SuffixRequest<'_>,
    limits: Limits,
    work: &WorkContext,
) -> Result<MigrateOutcome, MigrationError> {
    work.checkpoint()?;
    let cap = limits.envelope_bytes;
    verify_manifest(request.manifest, cap)?;
    if request.steps.is_empty() {
        return Err(MigrationError::WrongSuffix { applied: 0 });
    }
    let plans: Vec<&Plan> = request.steps.iter().map(|step| &step.plan).collect();
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
    let namespace = TargetNamespace::new(targets_root, request.target_incarnation)?;
    if let Some(tombstone) = namespace.read_tombstone(cap)? {
        return Err(match tombstone.lifecycle {
            Lifecycle::Deleted { operation, .. } if operation == request.operation => {
                MigrationError::Aborted { operation }
            }
            _ => MigrationError::TargetConflict,
        });
    }
    let last_descriptor = &request.steps.last().expect("nonempty").to_descriptor;
    if namespace.target_exists() {
        // Reuse-or-refuse follows the same rules as migration resume.
        let runnerless = read_published_ready(
            &namespace,
            request.operation,
            psd,
            target_identity,
            last_descriptor,
            limits,
            work,
        )?;
        return Ok(runnerless);
    }
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
        initial_system_digest: system_digest(&target_chain, cap)?,
        provenance: GenesisProvenance::Create,
    };
    let activation_ref = build_and_install(
        &namespace,
        request.operation,
        psd,
        genesis,
        &target_chain,
        &state,
        last_descriptor,
        limits,
        work,
    )?;
    Ok(MigrateOutcome::ReadyToSwitch {
        activation_ref,
        applied: applied_record,
    })
}

// ---------------------------------------------------------------------------
// Shared private machinery.
// ---------------------------------------------------------------------------

pub(super) fn compile_suffix(
    source_descriptor: &SchemaDescriptor,
    steps: &[StepInput],
) -> Result<Vec<CompiledPlan>, MigrationError> {
    let mut compiled = Vec::with_capacity(steps.len());
    let mut from = source_descriptor;
    for step in steps {
        compiled.push(compile(&step.plan, from, &step.to_descriptor)?);
        from = &step.to_descriptor;
    }
    Ok(compiled)
}

pub(super) fn applied_steps(manifest: &Manifest, first: usize, count: usize) -> Vec<AppliedStep> {
    manifest.entries[first..first + count]
        .iter()
        .map(|entry| AppliedStep {
            sequence: entry.sequence,
            label: entry.label.clone(),
            from_schema: entry.from_schema,
            to_schema: entry.to_schema,
            plan_digest: entry.plan_digest,
        })
        .collect()
}

/// Execute the compiled ordered steps over one starting state. One numeric
/// operation owns the evaluator for the whole suffix and drops before any
/// I/O follows.
pub(super) fn execute_steps(
    state: MigrationState,
    compiled: &[CompiledPlan],
    work: &WorkContext,
) -> Result<(MigrationState, usize), MigrationError> {
    let evaluator = ScalarEvaluator::new().map_err(|error| {
        MigrationError::State(StateError::Scalar {
            relation: bumbledb::RelationId(0),
            error,
        })
    })?;
    let mut current = state;
    let mut executed = 0;
    for plan in compiled {
        current = current.apply(plan, &evaluator, work)?;
        executed += 1;
    }
    drop(evaluator);
    Ok((current, executed))
}

pub(super) fn capture_source<S>(
    db: &Db<S>,
    work: &WorkContext,
) -> Result<MigrationState, MigrationError> {
    let schema = db.schema();
    let mut captured: Option<Result<MigrationState, StateError>> = None;
    db.read(|read| {
        captured = Some(MigrationState::from_source(read, schema, work));
        Ok(())
    })?;
    Ok(captured.ok_or(MigrationError::Log(LogError::Corruption))??)
}

pub(super) fn read_attachment<S>(db: &Db<S>) -> Result<Option<Vec<u8>>, MigrationError> {
    let mut owned = None;
    db.read(|read| {
        owned = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })?;
    Ok(owned)
}

/// Pre-freeze evidence read of an already-published target: a terminal or
/// already-activated target is recorded evidence that must be reported
/// WITHOUT freezing (or re-freezing) any source. A live, not-yet-activated
/// target returns `None` — its full `ReadyToSwitch` reuse verification runs
/// under the held freeze. A published directory without a decodable control
/// is a conflict, never something to freeze over.
/// Whether a core open refusal is the store's live-owner exclusion.
fn store_locked(error: &bumbledb::Error) -> bool {
    matches!(
        error,
        bumbledb::Error::Store(inner)
            if matches!(**inner, bumbledb::store::StoreError::StoreLocked { .. })
    )
}

/// Resolve recorded activation evidence for a target whose store is held
/// open by a live owner (the activated database being served): the durable
/// namespace activation marker, written in `activate_target`'s commit path,
/// is the readable copy of the one-time activation. A locked target WITHOUT
/// matching recorded evidence stays the original typed open refusal — never
/// guessed into an outcome.
fn locked_target_evidence(
    namespace: &TargetNamespace,
    operation: OperationId,
    target_identity: DatabaseIdentity,
    cap: usize,
    locked: bumbledb::Error,
) -> Result<MigrateOutcome, MigrationError> {
    if let Some(recorded) = namespace.read_activation(cap)? {
        if recorded.identity != target_identity {
            return Err(MigrationError::TargetConflict);
        }
        if let Activation::Activated {
            operation: held, ..
        } = recorded.activation
        {
            if held == operation {
                // The store is live-owned, so the CURRENT mode is not
                // readable; the recorded-at-activation mode is the durable
                // evidence (a live owner implies a live materialization).
                let access = match &recorded.lifecycle {
                    Lifecycle::Live(live) => live.access.mode(),
                    Lifecycle::Deleted { .. } => AccessMode::Deleted,
                };
                return Ok(MigrateOutcome::AlreadyActivated {
                    activation: recorded.activation,
                    access,
                });
            }
            return Err(MigrationError::TargetConflict);
        }
    }
    Err(MigrationError::Core(locked))
}

fn published_terminal_evidence(
    namespace: &TargetNamespace,
    operation: OperationId,
    target_identity: DatabaseIdentity,
    descriptor: &SchemaDescriptor,
    cap: usize,
) -> Result<Option<MigrateOutcome>, MigrationError> {
    if !namespace.target_exists() {
        return Ok(None);
    }
    let target_db: Db<SchemaDescriptor> =
        match Db::open(&namespace.target_dir(), descriptor.clone()) {
            Ok(db) => db,
            Err(error) if store_locked(&error) => {
                return locked_target_evidence(namespace, operation, target_identity, cap, error)
                    .map(Some);
            }
            Err(error) => return Err(error.into()),
        };
    let control = read_attachment(&target_db)?.ok_or(MigrationError::TargetConflict)?;
    let authority = decode_control(&control, cap).map_err(LogError::from)?;
    if authority.identity != target_identity {
        return Err(MigrationError::TargetConflict);
    }
    if let Activation::Activated {
        operation: held, ..
    } = authority.activation
    {
        if held == operation {
            let access = match &authority.lifecycle {
                Lifecycle::Live(live) => live.access.mode(),
                Lifecycle::Deleted { .. } => AccessMode::Deleted,
            };
            return Ok(Some(MigrateOutcome::AlreadyActivated {
                activation: authority.activation,
                access,
            }));
        }
        return Err(MigrationError::TargetConflict);
    }
    if let Lifecycle::Deleted {
        operation: held, ..
    } = authority.lifecycle
    {
        return Err(if held == operation {
            MigrationError::Aborted { operation: held }
        } else {
            MigrationError::TargetConflict
        });
    }
    Ok(None)
}

/// Read the complete migration history chain from one coherent snapshot.
pub(super) fn read_chain<S>(db: &Db<S>, cap: usize) -> Result<Vec<HistoryRecord>, MigrationError> {
    let mut rows: Vec<Vec<u8>> = Vec::new();
    let mut host_error = None;
    db.read(|read| {
        let mut index = 0u64;
        loop {
            match read.integration_host_record(&history_key(index)) {
                Ok(Some(bytes)) => rows.push(bytes.to_vec()),
                Ok(None) => break,
                Err(error) => {
                    host_error = Some(error);
                    break;
                }
            }
            index += 1;
        }
        Ok(())
    })?;
    if let Some(error) = host_error {
        return Err(MigrationError::from(LogError::from(error)));
    }
    rows.iter()
        .map(|bytes| super::history::decode_record(bytes, cap).map_err(MigrationError::from))
        .collect()
}

/// One serialized read-transition-commit over a database's control
/// attachment: the exclusive writer session is held from the read through
/// the commit, so no concurrent submission or transition can be lost.
#[expect(
    clippy::large_enum_variant,
    reason = "one Transition is consumed on the frame that built it; \
              HeadAuthority is a fixed-size Copy control frame"
)]
enum Transition<T> {
    Commit(HeadAuthority, T),
    Keep(T),
}

fn with_authority<S, T>(
    db: &Db<S>,
    limits: Limits,
    work: &WorkContext,
    transition: impl FnOnce(HeadAuthority) -> Result<Transition<T>, MigrationError>,
) -> Result<T, MigrationError> {
    let mut session = db.integration_writer(work)?;
    let control = read_attachment(db)?.ok_or(MigrationError::Log(LogError::NotInitialized))?;
    let authority = decode_control(&control, limits.envelope_bytes).map_err(LogError::from)?;
    match transition(authority)? {
        Transition::Keep(value) => Ok(value),
        Transition::Commit(next, value) => {
            let bytes = encode_control(&next, limits.envelope_bytes).map_err(LogError::from)?;
            let empty = ChangeSet::builder(db.schema(), work.clone())
                .finish()
                .map_err(|error| MigrationError::Log(LogError::Core(error.into())))?;
            let prepared = match session.prepare(&empty)? {
                Admission::Accepted(prepared) => prepared,
                Admission::Rejected(_) => return Err(MigrationError::Log(LogError::Corruption)),
            };
            let sealed = prepared.seal(HostChanges {
                records: &[],
                attachment: AttachmentChange::Put(&bytes),
            })?;
            sealed.commit()?;
            Ok(value)
        }
    }
}

/// Read a published target expected to be `ReadyToSwitch` for exactly this
/// operation/plan set (initialization resume). Refuses everything else.
fn read_published_ready(
    namespace: &TargetNamespace,
    operation: OperationId,
    psd: [u8; 32],
    target_identity: DatabaseIdentity,
    descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<MigrateOutcome, MigrationError> {
    let cap = limits.envelope_bytes;
    let target_db: Db<SchemaDescriptor> =
        match Db::open(&namespace.target_dir(), descriptor.clone()) {
            Ok(db) => db,
            // A live owner holds the published target open (the activated
            // database being served): recorded activation evidence resolves
            // the rerun; anything else stays the typed open refusal.
            Err(error) if store_locked(&error) => {
                return locked_target_evidence(namespace, operation, target_identity, cap, error);
            }
            Err(error) => return Err(error.into()),
        };
    let control = read_attachment(&target_db)?.ok_or(MigrationError::TargetConflict)?;
    let authority = decode_control(&control, cap).map_err(LogError::from)?;
    if authority.identity != target_identity {
        return Err(MigrationError::TargetConflict);
    }
    if let Activation::Activated {
        operation: held, ..
    } = authority.activation
    {
        if held == operation {
            let access = match &authority.lifecycle {
                Lifecycle::Live(live) => live.access.mode(),
                Lifecycle::Deleted { .. } => AccessMode::Deleted,
            };
            return Ok(MigrateOutcome::AlreadyActivated {
                activation: authority.activation,
                access,
            });
        }
        return Err(MigrationError::TargetConflict);
    }
    let live = authority.live()?;
    match live.access {
        Access::Frozen {
            operation: held,
            intent:
                FreezeIntent::Migration {
                    plan_set_digest,
                    target,
                },
        } if held == operation
            && plan_set_digest == psd
            && target == target_identity.incarnation_id => {}
        _ => return Err(MigrationError::TargetConflict),
    }
    let chain = read_chain(&target_db, cap)?;
    let Some(HistoryRecord::Applied(applied_record)) = chain.last() else {
        return Err(MigrationError::OutputMismatch);
    };
    if applied_record.operation != operation || applied_record.plan_set_digest != psd {
        return Err(MigrationError::OutputMismatch);
    }
    let schema = descriptor
        .clone()
        .validate()
        .map_err(bumbledb::Error::from)?;
    let mut captured = None;
    target_db.read(|read| {
        captured = Some(MigrationState::from_source(read, &schema, work));
        Ok(())
    })?;
    let recomputed = captured
        .ok_or(MigrationError::Log(LogError::Corruption))??
        .digest()?;
    if recomputed != applied_record.target_digest || live.decision.seq != 0 {
        return Err(MigrationError::OutputMismatch);
    }
    Ok(MigrateOutcome::ReadyToSwitch {
        activation_ref: ActivationRef {
            operation,
            plan_set_digest: psd,
            target: target_identity,
            target_genesis: live.decision.hash,
        },
        applied: applied_record.clone(),
    })
}

/// One completely built staged target: the open staged database (private
/// scratch until published), its frozen genesis control and genesis stamp.
pub(super) struct StagedTarget {
    pub(super) db: Db<SchemaDescriptor>,
    pub(super) frozen: HeadAuthority,
    pub(super) genesis: crate::history::DecisionStamp,
}

/// Build the staged target database at `staging` through the core checked
/// builder: complete judged admission of every relation's final rows, then
/// genesis control + the complete history chain committed in one host
/// transaction. The result is private scratch; publication (a locked local
/// install, or the hosted checkpoint + genesis-head data plane) is the
/// caller's separate durable step.
#[expect(
    clippy::too_many_arguments,
    reason = "one private build path shared by the local and hosted runners"
)]
pub(super) fn build_staged(
    staging: &Path,
    operation: OperationId,
    psd: [u8; 32],
    genesis: GenesisRecord,
    target_chain: &[HistoryRecord],
    state: &MigrationState,
    descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<StagedTarget, MigrationError> {
    let cap = limits.envelope_bytes;
    let target_identity = genesis.identity;
    let stamp = genesis_stamp(&genesis, cap).map_err(LogError::from)?;
    let authority = HeadAuthority::genesis(target_identity, stamp, Activation::NotActivated)
        .map_err(LogError::from)?;
    let frozen = match authority
        .freeze(
            operation,
            FreezeIntent::Migration {
                plan_set_digest: psd,
                target: target_identity.incarnation_id,
            },
        )
        .map_err(LogError::from)?
    {
        FreezeOutcome::Frozen(frozen) => frozen,
        FreezeOutcome::AlreadyFrozen { .. } => unreachable!("a fresh genesis is active"),
    };
    let control = encode_control(&frozen, cap).map_err(LogError::from)?;

    // Stream compiled final sets into private unready staging; complete
    // admission once. No InstanceBuilder / ready-path population.
    let schema = descriptor
        .clone()
        .validate()
        .map_err(bumbledb::Error::from)?;
    if let Some(parent) = staging.parent() {
        std::fs::create_dir_all(parent).map_err(NamespaceError::Io)?;
    }
    let staged = begin_staged(staging, descriptor.clone(), work)?;
    populate_staged(&staged, state, &schema, work)?;
    let mut records: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(target_chain.len());
    for (index, record) in target_chain.iter().enumerate() {
        records.push((
            history_key(index as u64).to_vec(),
            encode_record(record, cap).map_err(LogError::from)?,
        ));
    }
    let host: Vec<HostRecordChange<'_>> = records
        .iter()
        .map(|(key, value)| HostRecordChange::Put {
            key: key.as_slice(),
            value: value.as_slice(),
        })
        .collect();
    staged.write_host(&host, Some(&control), work)?;
    let dest = staged.destination().to_path_buf();
    staged.complete_install(work)?;
    let staged_db = Db::open(&dest, descriptor.clone(), work.clone())?;
    Ok(StagedTarget {
        db: staged_db,
        frozen,
        genesis: stamp,
    })
}

const STAGED_BATCH_BYTES: usize = 16 * 1024 * 1024;

fn populate_staged(
    staged: &StagedPopulation,
    state: &MigrationState,
    schema: &bumbledb::schema::Schema,
    work: &WorkContext,
) -> Result<(), MigrationError> {
    for (index, relation) in schema.relations().iter().enumerate() {
        if relation.body().closed_rows().is_some() {
            continue;
        }
        work.checkpoint()?;
        let id = RelationId(u32::try_from(index).expect("validated relation count"));
        let fields = relation.fields();
        let mut pending: Vec<Vec<bumbledb::Value>> = Vec::new();
        let mut batch_bytes = 0usize;
        let mut flush_error = None;
        state.visit_rows(id, fields, work, &mut |values| {
            pending.push(values.to_vec());
            batch_bytes = batch_bytes.saturating_add(64 + 48 * values.len());
            if batch_bytes >= STAGED_BATCH_BYTES {
                if let Err(error) = flush_staged_batch(staged, schema, id, &pending, work) {
                    flush_error = Some(error);
                    return Ok(false);
                }
                pending.clear();
                batch_bytes = 0;
            }
            Ok(true)
        })?;
        if let Some(error) = flush_error {
            return Err(error);
        }
        if !pending.is_empty() {
            flush_staged_batch(staged, schema, id, &pending, work)?;
        }
    }
    Ok(())
}

fn flush_staged_batch(
    staged: &StagedPopulation,
    schema: &bumbledb::schema::Schema,
    id: RelationId,
    pending: &[Vec<bumbledb::Value>],
    work: &WorkContext,
) -> Result<(), MigrationError> {
    let mut builder = ChangeSet::builder(schema, work.clone());
    for values in pending {
        builder
            .insert(id, values)
            .map_err(|error| MigrationError::Log(LogError::Core(error.into())))?;
    }
    let changes = builder
        .finish()
        .map_err(|error| MigrationError::Log(LogError::Core(error.into())))?;
    if changes.is_empty() {
        return Ok(());
    }
    staged.apply_unjudged(&changes, work)?;
    Ok(())
}

/// Build the staged target database through the core checked builder, write
/// genesis control + the complete history chain in one host transaction,
/// close it, and durably install it into the stable namespace under the
/// kernel lock (no-overwrite; the tombstone always wins a race).
#[expect(
    clippy::too_many_arguments,
    reason = "one private build path shared by migrate and initialize"
)]
fn build_and_install(
    namespace: &TargetNamespace,
    operation: OperationId,
    psd: [u8; 32],
    genesis: GenesisRecord,
    target_chain: &[HistoryRecord],
    state: &MigrationState,
    descriptor: &SchemaDescriptor,
    limits: Limits,
    work: &WorkContext,
) -> Result<ActivationRef, MigrationError> {
    let cap = limits.envelope_bytes;
    let target_identity = genesis.identity;
    let staging = namespace.fresh_staging();
    let staged = build_staged(
        &staging,
        operation,
        psd,
        genesis,
        target_chain,
        state,
        descriptor,
        limits,
        work,
    )?;
    let stamp = staged.genesis;
    drop(staged.db);

    // Durable publication: the no-overwrite install under the stable lock.
    // An abort that fenced this namespace while we built wins here; the
    // staging directory is deliberately removed as private scratch.
    let lock = namespace.lock()?;
    match namespace.install_target(&lock, &staging, cap) {
        Ok(()) => {}
        Err(error) => {
            drop(lock);
            let _ = std::fs::remove_dir_all(&staging);
            return Err(match error {
                NamespaceError::ForeignTombstone => match namespace.read_tombstone(cap) {
                    Ok(Some(tombstone)) => match tombstone.lifecycle {
                        Lifecycle::Deleted {
                            operation: held, ..
                        } if held == operation => MigrationError::Aborted { operation: held },
                        _ => MigrationError::TargetConflict,
                    },
                    _ => MigrationError::TargetConflict,
                },
                NamespaceError::TargetExists => MigrationError::OutputMismatch,
                other => MigrationError::Namespace(other),
            });
        }
    }
    drop(lock);
    Ok(ActivationRef {
        operation,
        plan_set_digest: psd,
        target: target_identity,
        target_genesis: stamp.hash,
    })
}
