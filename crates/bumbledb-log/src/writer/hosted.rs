//! `HostedHistory`: one never-reused HEAD over immutable decision objects.
//!
//! The publication machine prepares a private LMDB candidate against the
//! captured head, uploads the one immutable decision object, then conditionally
//! replaces HEAD. The successful atomic replacement is the linearization point;
//! only then does the local LMDB transaction commit. A losing CAS re-evaluates
//! the *same* immutable command against the winner. An unknown CAS/upload never
//! becomes a fabricated success or rejection — it resolves by reading the head
//! and the retained receipt, or returns `OutcomeUnknown` with the retained ref.
//!
//! The remote HEAD body is the composed [`crate::manifest::HeadRecord`] (C08):
//! P04's authority control projection embedded verbatim inside P05's retention
//! fields (recovery root, object epoch, named roots, GC state). This machine
//! reads heads through [`manifest::head-record decoding`](crate::manifest::decode_head)
//! and composes successors through [`crate::manifest::decided_head_body`], so
//! every retention field is preserved across a publication and the configured
//! [`TailPolicy`] envelope backpressures admission (`MaintenanceRequired`).
//! Decision objects are staged under the PARENT head's `object_epoch` (read
//! per attempt, never a constructor-time constant): a decision published after
//! a GC barrier must live in the open epoch, or the epoch's collector would
//! sweep a referenced object (the reference-introduction rule, chapter 21).
//! The local attachment remains the bare control projection.
//!
//! Two bridge-facing seams (P06R2/P04R): [`HostedHistory::catch_up`] is the
//! public read-side verb — advance the local materialization to the current
//! verified tip without submitting, initializing or thawing anything — and
//! [`SubmitOptions`]/[`HostedHistory::submit_with`] are the per-call bounded
//! attempts/backoff override (`with_attempts` consumes `self`; a bridge
//! holding the machine behind an `Arc` needs per-call bounds that can only
//! narrow the machine's own).

use std::sync::Arc;
use std::time::Duration;

use bumbledb::integration::{AttachmentChange, HostChanges};
use bumbledb::{ChangeSet, Db, WorkContext};

use crate::apply::{self, ApplyError};
use crate::checkpointer::{Headroom, admission_headroom};
use crate::history::admission::Submission;
use crate::history::authority::{
    Activation, ActivationCause, HeadAuthority, decode_control, encode_control,
};
use crate::history::command::{Command, Limits};
use crate::history::decision::{self, GenesisProvenance, GenesisRecord};
use crate::history::receipt::{decode_receipt_row, receipt_key};
use crate::history::{
    CommandId, CommandRef, DatabaseId, DatabaseIdentity, DecisionStamp, IncarnationId, OperationId,
    SchemaId, TerminalReceipt,
};
use crate::manifest::{self, HeadRecord, TailPolicy};
use crate::store::{self, BackendError, ObjectError, ObjectKind};

use super::decide::{self, Judged, Plan, RealPrepared};
use super::verbs::{ConditionalOutcome, ConditionalStore, HeadRead, HeadVersion};
use super::{LocalHealth, LogError, ResolveOutcome, SubmitOutcome};

/// Default bounded publication-attempt budget across CAS losses.
pub const DEFAULT_ATTEMPTS: u32 = 16;
/// Default bounded catch-up decisions applied per refresh.
pub const DEFAULT_CATCH_UP: u32 = 4096;
/// The machine's own per-delay backoff bound: a per-call override can never
/// park an owning worker longer than this between contended attempts,
/// whatever the wire requested.
pub const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// Per-call submission bounds — the C09 `SubmitOptions` wire's seam into the
/// machine. `with_attempts` consumes `self`, which a bridge holding the
/// machine behind an `Arc` for its whole lifetime cannot use per call; these
/// options tune ONE `submit_with` invocation and leave the machine untouched.
///
/// Every field is clamped to the machine's own bounds: per-call attempts can
/// only narrow the configured attempt budget (never widen it), and every
/// backoff delay is capped by [`MAX_BACKOFF`]. Backoff applies only between
/// attempts after a definite CAS loss; unknown outcomes resolve immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SubmitOptions {
    /// Bounded publication attempts for THIS call (each CAS loss re-reads and
    /// re-evaluates the same command). `None` uses the machine's configured
    /// budget; `Some(n)` is clamped to `1..=machine attempts`.
    pub attempts: Option<u32>,
    /// Exponential backoff base between contended attempts (`base * 2^n`).
    /// `None`/zero means immediate re-read (the machine default).
    pub backoff_base: Option<Duration>,
    /// The per-delay cap; itself clamped to [`MAX_BACKOFF`].
    pub backoff_cap: Option<Duration>,
}

impl SubmitOptions {
    /// The machine's own defaults: configured attempts, no backoff.
    pub const DEFAULT: Self = Self {
        attempts: None,
        backoff_base: None,
        backoff_cap: None,
    };
}

/// The per-call attempt budget: an override can only narrow the machine's
/// own bound, and at least one attempt is always made.
fn effective_attempts(machine: u32, requested: Option<u32>) -> u32 {
    match requested {
        None => machine,
        Some(requested) => requested.clamp(1, machine.max(1)),
    }
}

/// The bounded delay before retrying after the `lost`-th definite CAS loss;
/// `None` means retry immediately. Never exceeds the per-call cap or the
/// machine's [`MAX_BACKOFF`] bound.
fn backoff_delay(options: SubmitOptions, lost: u32) -> Option<Duration> {
    let base = options.backoff_base?;
    if base.is_zero() {
        return None;
    }
    let cap = options.backoff_cap.unwrap_or(MAX_BACKOFF).min(MAX_BACKOFF);
    let doubled = base.saturating_mul(1u32.checked_shl(lost.min(20)).unwrap_or(u32::MAX));
    let delay = doubled.min(cap);
    if delay.is_zero() { None } else { Some(delay) }
}

pub struct HostedHistory<S, B> {
    db: Arc<Db<S>>,
    backend: B,
    prefix: String,
    identity: DatabaseIdentity,
    limits: Limits,
    /// The configured durable-tail envelope (C08). `UNBOUNDED` until a
    /// deployment qualifies one; the composed-head grammar enforces whatever
    /// is configured on every decided composition, no-ops included.
    tail_policy: TailPolicy,
    attempts: u32,
    catch_up_bound: u32,
}

impl<S, B> HostedHistory<S, B>
where
    B: ConditionalStore,
    B::Error: BackendError,
{
    /// Create a hosted incarnation: install the genesis composed head by
    /// conditional create. The local materialization is initialized to the
    /// same genesis. Refuses an existing head. `object_epoch` is the initial
    /// open object epoch recorded in the genesis head (GC barriers advance it
    /// afterwards; this machine reads the current epoch from the head).
    ///
    /// # Errors
    /// Refuses an existing head, a foreign-schema local database, backend
    /// failures and frame/work limits.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        db: Arc<Db<S>>,
        backend: B,
        prefix: String,
        object_epoch: u64,
        database_id: DatabaseId,
        incarnation_id: IncarnationId,
        operation: OperationId,
        limits: Limits,
        work: &WorkContext,
    ) -> Result<Self, LogError> {
        let schema_id = fingerprint(&db);
        let identity = DatabaseIdentity {
            database_id,
            incarnation_id,
            schema_id,
        };
        let (application, system) = decision::blank_initial_digests();
        let genesis_record = GenesisRecord {
            identity,
            initial_application_digest: application,
            initial_system_digest: system,
            provenance: GenesisProvenance::Create,
        };
        let genesis = decision::genesis_stamp(&genesis_record, limits.envelope_bytes)?;
        let authority = HeadAuthority::genesis(
            identity,
            genesis,
            Activation::Activated {
                operation,
                target_genesis: genesis.hash,
                cause: ActivationCause::Create,
            },
        )?;
        // Local genesis attachment (empty facts).
        write_local_genesis(&db, &authority, limits, work)?;
        let head_key = store::head_key(&prefix);
        let body = manifest::genesis_head_body(&authority, object_epoch, limits.envelope_bytes)?;
        match backend
            .create_head(&head_key, &body)
            .map_err(|_| LogError::Backend)?
        {
            ConditionalOutcome::Published { .. } => {}
            ConditionalOutcome::PreconditionFailed => {
                return Err(LogError::CommandIdentityConflict);
            }
            ConditionalOutcome::Indeterminate => return Err(LogError::Backend),
        }
        Ok(Self {
            db,
            backend,
            prefix,
            identity,
            limits,
            tail_policy: TailPolicy::UNBOUNDED,
            attempts: DEFAULT_ATTEMPTS,
            catch_up_bound: DEFAULT_CATCH_UP,
        })
    }

    /// Open a hosted incarnation by reading its HEAD and catching the local
    /// materialization up to the captured tip. Never initializes on a missing
    /// head; a warm local cache older than the head's checkpoint base is
    /// `MaterializationStale` — recovery hydration (C08), never an empty
    /// fallback. The current object epoch comes from the head itself.
    ///
    /// # Errors
    /// Refuses a missing/foreign head, corruption and backend failures.
    pub fn open(
        db: Arc<Db<S>>,
        backend: B,
        prefix: String,
        limits: Limits,
        work: &WorkContext,
    ) -> Result<Self, LogError> {
        let head_key = store::head_key(&prefix);
        let record = match backend
            .read_head(&head_key)
            .map_err(|_| LogError::Backend)?
        {
            HeadRead::Present { body, .. } => decode_record(&body, limits)?,
            HeadRead::Absent => return Err(LogError::NotInitialized),
        };
        if record.control.identity.schema_id != fingerprint(&db) {
            return Err(LogError::Identity);
        }
        let history = Self {
            db,
            backend,
            prefix,
            identity: record.control.identity,
            limits,
            tail_policy: TailPolicy::UNBOUNDED,
            attempts: DEFAULT_ATTEMPTS,
            catch_up_bound: DEFAULT_CATCH_UP,
        };
        history.catch_up_to(&record, work)?;
        Ok(history)
    }

    #[must_use]
    pub const fn identity(&self) -> DatabaseIdentity {
        self.identity
    }

    #[must_use]
    pub fn db(&self) -> &Db<S> {
        &self.db
    }

    #[must_use]
    pub fn with_attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts.max(1);
        self
    }

    /// Configure the durable-tail envelope this writer enforces (C08). The
    /// values are deployment-qualified policy; the composed head refuses any
    /// decided composition that would exceed them.
    #[must_use]
    pub const fn with_tail_policy(mut self, policy: TailPolicy) -> Self {
        self.tail_policy = policy;
        self
    }

    /// Submit one owned sealed command through the hosted authority, under
    /// the machine's own configured bounds.
    #[must_use]
    pub fn submit(&self, command: &Command, work: &WorkContext) -> SubmitOutcome {
        self.submit_with(command, SubmitOptions::DEFAULT, work)
    }

    /// Submit with per-call bounds (the C09 `SubmitOptions` wire seam,
    /// P04R/P06R2). The options tune THIS call only — the machine is not
    /// consumed or reconfigured — and can never exceed the machine's own
    /// bounds: attempts clamp to `1..=configured attempts`, backoff delays
    /// clamp to [`MAX_BACKOFF`].
    #[must_use]
    pub fn submit_with(
        &self,
        command: &Command,
        options: SubmitOptions,
        work: &WorkContext,
    ) -> SubmitOutcome {
        let reference = command.command_ref();
        match self.try_submit(command, options, work) {
            Ok(outcome) => outcome,
            Err(SubmitFailure::NotSubmitted(error)) => SubmitOutcome::NotSubmitted {
                command: reference,
                error,
            },
            Err(SubmitFailure::Unknown(error)) => SubmitOutcome::OutcomeUnknown {
                command: reference,
                error,
            },
        }
    }

    fn try_submit(
        &self,
        command: &Command,
        options: SubmitOptions,
        work: &WorkContext,
    ) -> Result<SubmitOutcome, SubmitFailure> {
        let reference = command.command_ref();
        if reference.identity != self.identity {
            return Err(SubmitFailure::NotSubmitted(LogError::Identity));
        }
        let attempts = effective_attempts(self.attempts, options.attempts);
        for attempt in 0..attempts {
            work.checkpoint()
                .map_err(|e| SubmitFailure::NotSubmitted(e.into()))?;
            let head_key = store::head_key(&self.prefix);
            let (record, body, version) = match self
                .backend
                .read_head(&head_key)
                .map_err(|_| LogError::Backend)?
            {
                HeadRead::Present { version, body } => {
                    let record = decode_record(&body, self.limits)?;
                    (record, body, version)
                }
                HeadRead::Absent => {
                    return Err(SubmitFailure::NotSubmitted(LogError::NotInitialized));
                }
            };
            // Bring the local materialization to this captured tip before
            // preparing a candidate against it.
            self.catch_up_to(&record, work)
                .map_err(SubmitFailure::NotSubmitted)?;

            let retained = self
                .retained(reference.id, work)
                .map_err(SubmitFailure::NotSubmitted)?;
            let view = record
                .control
                .admission_view()
                .ok_or(SubmitFailure::NotSubmitted(LogError::DatabaseDeleted))?;
            let plan = match view.submit(reference, command.metadata().condition, retained.as_ref())
            {
                Ok(Submission::AlreadyDecided(receipt)) => {
                    return Ok(SubmitOutcome::Decided {
                        receipt: receipt.clone(),
                        local_health: self.local_health(&record.control, work),
                    });
                }
                Ok(Submission::PreconditionFailed { expected, observed }) => {
                    Plan::PreconditionFailed { expected, observed }
                }
                Ok(Submission::Evaluate) => Plan::Evaluate,
                Err(refusal) => return Err(SubmitFailure::NotSubmitted(refusal.into())),
            };

            // Envelope backpressure (C08): a retained receipt above still
            // resolves; NEW decisions — no-ops and rejections included — are
            // refused before any work is dispatched. The composed-head
            // grammar re-enforces this authoritatively at composition time.
            if let Some(recovery) = record.recovery
                && matches!(
                    admission_headroom(&recovery, &self.tail_policy),
                    Headroom::MaintenanceRequired
                )
            {
                return Err(SubmitFailure::NotSubmitted(LogError::MaintenanceRequired {
                    count: recovery.tail_count(),
                    bytes: recovery.tail_bytes,
                }));
            }

            // `attempt_publish` returns with its writer session already
            // dropped, so resolving an unknown CAS here cannot deadlock on the
            // writer lock.
            match self.attempt_publish(command, &record, &body, &version, plan, work)? {
                AttemptResult::Published {
                    receipt,
                    local_health,
                } => {
                    return Ok(SubmitOutcome::Decided {
                        receipt,
                        local_health,
                    });
                }
                // CAS loss: re-read the head and re-evaluate the same command,
                // after the per-call bounded backoff (definite losses only —
                // nothing published, the sealed candidate was dropped).
                AttemptResult::Lost => {
                    if attempt + 1 < attempts
                        && let Some(delay) = backoff_delay(options, attempt)
                    {
                        std::thread::sleep(delay);
                    }
                }
                AttemptResult::Unknown { stamp } => {
                    return match self.resolve_after_unknown(reference, stamp, work)? {
                        AttemptResult::Published {
                            receipt,
                            local_health,
                        } => Ok(SubmitOutcome::Decided {
                            receipt,
                            local_health,
                        }),
                        AttemptResult::Lost | AttemptResult::Unknown { .. } => {
                            Err(SubmitFailure::Unknown(LogError::Backend))
                        }
                    };
                }
            }
        }
        // Bounded attempts exhausted by contention: dispatched work happened,
        // so this is uncertainty, never a fabricated NotSubmitted.
        Err(SubmitFailure::Unknown(LogError::Backend))
    }

    fn attempt_publish(
        &self,
        command: &Command,
        parent: &HeadRecord,
        parent_body: &[u8],
        version: &HeadVersion,
        plan: Plan,
        work: &WorkContext,
    ) -> Result<AttemptResult, SubmitFailure> {
        // Prepare and seal the private candidate on the owning worker. The
        // sealed LMDB transaction is held across the remote attempt and only
        // commits after publication is known.
        let mut session = self
            .db
            .integration_writer(work)
            .map_err(|e| SubmitFailure::NotSubmitted(e.into()))?;
        let schema = self.db.schema();
        let authority = &parent.control;
        let candidate = match plan {
            Plan::PreconditionFailed { expected, observed } => {
                let prepared = decide::prepare_empty(&mut session, schema, work)
                    .map_err(SubmitFailure::NotSubmitted)?;
                decide::seal_candidate(
                    prepared,
                    authority,
                    command,
                    Judged::PreconditionFailed { expected, observed },
                    self.limits,
                )
                .map_err(SubmitFailure::NotSubmitted)?
            }
            Plan::Evaluate => {
                match decide::prepare_real(&mut session, schema, command, self.limits, work)
                    .map_err(SubmitFailure::NotSubmitted)?
                {
                    RealPrepared::Admitted { prepared, judged } => {
                        decide::seal_candidate(prepared, authority, command, judged, self.limits)
                            .map_err(SubmitFailure::NotSubmitted)?
                    }
                    RealPrepared::Rejected { evidence } => {
                        let prepared = decide::prepare_empty(&mut session, schema, work)
                            .map_err(SubmitFailure::NotSubmitted)?;
                        decide::seal_candidate(
                            prepared,
                            authority,
                            command,
                            Judged::InvariantRejected { evidence },
                            self.limits,
                        )
                        .map_err(SubmitFailure::NotSubmitted)?
                    }
                }
            }
        };
        let stamp = candidate.receipt.decision_at;

        // Upload the one immutable decision object under the PARENT head's
        // open object epoch (the GC reference-introduction rule). Immutable
        // content equality absorbs an ambiguous re-PUT; an unresolved upload
        // is a local abort — the candidate was never proven durable, no CAS.
        match store::put_verified(
            &self.backend,
            &self.prefix,
            parent.object_epoch,
            ObjectKind::Decision,
            &candidate.decision_bytes,
        ) {
            Ok(_) => {}
            Err(error) => {
                return Err(SubmitFailure::NotSubmitted(map_object_error(&error)));
            }
        }

        // Compose HEAD' from this exact parent body — control advanced,
        // retention fields preserved, tail accounting grown and the envelope
        // enforced — and conditionally replace it.
        let body = manifest::decided_head_body(
            parent_body,
            &candidate.new_authority,
            candidate.decision_bytes.len() as u64,
            &self.tail_policy,
            self.limits.envelope_bytes,
        )
        .map_err(|error| SubmitFailure::NotSubmitted(error.into()))?;
        let head_key = store::head_key(&self.prefix);
        match self
            .backend
            .replace_head(&head_key, version, &body)
            .map_err(|_| LogError::Backend)?
        {
            ConditionalOutcome::Published { .. } => {
                let receipt = candidate.receipt.clone();
                // Publication known: commit/apply the local transaction. A
                // local commit failure keeps the Published receipt with
                // unavailable local health; it is never a rejection.
                match candidate.sealed.commit() {
                    Ok(_) => Ok(AttemptResult::Published {
                        receipt,
                        local_health: LocalHealth::Ready { at: stamp },
                    }),
                    Err(error) => Ok(AttemptResult::Published {
                        receipt,
                        local_health: LocalHealth::Unavailable {
                            error: error.into(),
                        },
                    }),
                }
            }
            ConditionalOutcome::PreconditionFailed => {
                // Another writer won this head. Abort the private candidate
                // (drop the sealed transaction) and retry the same command.
                drop(candidate);
                Ok(AttemptResult::Lost)
            }
            ConditionalOutcome::Indeterminate => {
                // The CAS outcome is unknown. Abort the private candidate so the
                // writer session releases at return; the caller resolves the
                // uncertainty once the writer lock is free.
                drop(candidate);
                Ok(AttemptResult::Unknown { stamp })
            }
        }
    }

    // After an unknown CAS, read the head and the retained receipt to decide
    // whether this exact decision published. A fresh head at our stamp, or a
    // retained receipt for our command, is publication; otherwise uncertainty.
    fn resolve_after_unknown(
        &self,
        reference: CommandRef,
        stamp: DecisionStamp,
        work: &WorkContext,
    ) -> Result<AttemptResult, SubmitFailure> {
        let head_key = store::head_key(&self.prefix);
        let record = match self
            .backend
            .read_head(&head_key)
            .map_err(|_| LogError::Backend)?
        {
            HeadRead::Present { body, .. } => {
                decode_record(&body, self.limits).map_err(SubmitFailure::Unknown)?
            }
            HeadRead::Absent => return Err(SubmitFailure::Unknown(LogError::Backend)),
        };
        // Catch up so the retained receipt is locally visible if it published.
        self.catch_up_to(&record, work)
            .map_err(SubmitFailure::Unknown)?;
        if let Some(receipt) = self
            .retained(reference.id, work)
            .map_err(SubmitFailure::Unknown)?
            && receipt.command.digest == reference.digest
            && receipt.decision_at == stamp
        {
            return Ok(AttemptResult::Published {
                receipt,
                local_health: self.local_health(&record.control, work),
            });
        }
        // Either the head advanced to exactly our decision (publication proven
        // but the receipt is not yet locally materialized) or it moved past a
        // version we could still win. In both cases this invocation cannot
        // establish the terminal receipt: the retained ref resolves it later.
        let _ = stamp;
        Err(SubmitFailure::Unknown(LogError::Backend))
    }

    /// Resolve a retained ref against the current head and local receipt state.
    ///
    /// # Errors
    /// Refuses foreign identity, deletion and backend failures.
    pub fn resolve(
        &self,
        command: CommandRef,
        work: &WorkContext,
    ) -> Result<ResolveOutcome, LogError> {
        if command.identity != self.identity {
            return Err(LogError::Identity);
        }
        let head_key = store::head_key(&self.prefix);
        let record = match self
            .backend
            .read_head(&head_key)
            .map_err(|_| LogError::Backend)?
        {
            HeadRead::Present { body, .. } => decode_record(&body, self.limits)?,
            HeadRead::Absent => return Err(LogError::NotInitialized),
        };
        self.catch_up_to(&record, work)?;
        let retained = self.retained(command.id, work)?;
        let view = record
            .control
            .admission_view()
            .ok_or(LogError::DatabaseDeleted)?;
        match view.resolve(command, retained.as_ref()) {
            Ok(crate::history::admission::Resolution::Found(receipt)) => {
                Ok(ResolveOutcome::Found(receipt.clone()))
            }
            Ok(crate::history::admission::Resolution::NotRecordedAt { decision_at }) => {
                Ok(ResolveOutcome::NotRecordedAt { decision_at })
            }
            Err(crate::history::admission::Refusal::CommandEpochClosed) => {
                Ok(ResolveOutcome::CommandEpochClosed)
            }
            Err(crate::history::admission::Refusal::ReceiptExpiredUnknown) => {
                Ok(ResolveOutcome::ReceiptExpiredUnknown)
            }
            Err(refusal) => Err(refusal.into()),
        }
    }

    /// Read-side catch-up (the P06R2 hub-requested verb): read the current
    /// composed head and advance the LOCAL materialization to its tip through
    /// the same bounded decision-window walk `submit`/`resolve` use, so a
    /// hosted snapshot with `consistency: latest` can be served from local
    /// state without riding a writer or reopen lane. Returns the reached tip.
    ///
    /// This verb performs NO head write: it never initializes a missing head
    /// (`NotInitialized`), never thaws or otherwise transitions the authority,
    /// and a warm cache older than the durable tail's checkpoint base still
    /// reports [`LogError::MaterializationStale`] — the caller routes that to
    /// recovery hydration (C08), never to an empty fallback.
    ///
    /// # Errors
    /// `NotInitialized` (absent head), `DatabaseDeleted` (tombstone head),
    /// `MaterializationStale`, corruption evidence and backend failures.
    pub fn catch_up(&self, work: &WorkContext) -> Result<DecisionStamp, LogError> {
        let head_key = store::head_key(&self.prefix);
        let record = match self
            .backend
            .read_head(&head_key)
            .map_err(|_| LogError::Backend)?
        {
            HeadRead::Present { body, .. } => decode_record(&body, self.limits)?,
            HeadRead::Absent => return Err(LogError::NotInitialized),
        };
        let target = record
            .control
            .position()
            .ok_or(LogError::DatabaseDeleted)?
            .decision;
        self.catch_up_to(&record, work)?;
        Ok(target)
    }

    /// Walk decision objects from the captured tip back to the local decision,
    /// then apply them forward. Decisions are fetched through the bounded
    /// epoch window `[recovery.epoch_floor, head.object_epoch]` (a decision
    /// lives under the epoch open at its publication; barriers advance the
    /// epoch afterwards). Bounded by the catch-up budget; a missing protected
    /// decision is corruption, and a local cache older than the checkpoint
    /// base is `MaterializationStale` — never an empty fallback.
    fn catch_up_to(&self, record: &HeadRecord, work: &WorkContext) -> Result<(), LogError> {
        let target_stamp = match record.control.position() {
            Some(position) => position.decision,
            None => return Ok(()), // A tombstone has no tip to materialize.
        };
        let mut local = self.local_authority(work)?;
        let local_stamp = local
            .position()
            .map(|position| position.decision)
            .ok_or(LogError::DatabaseDeleted)?;
        if local_stamp == target_stamp {
            return Ok(());
        }
        // A live composed head names its recovery root (decode invariant).
        let recovery = record.recovery.ok_or(LogError::Corruption)?;
        // Collect decision bytes from the tip backward until reaching local.
        let mut pending: Vec<Vec<u8>> = Vec::new();
        let mut cursor = target_stamp;
        for _ in 0..self.catch_up_bound {
            if cursor == local_stamp {
                break;
            }
            if recovery.checkpoint.is_some() && cursor == recovery.base {
                // The tail stops at the checkpoint base; the decisions below
                // it may be legitimately collected. This warm cache cannot
                // catch up — it must rehydrate through recovery (C08).
                return Err(LogError::MaterializationStale);
            }
            if cursor.seq == 0 {
                // Reached a genesis different from the local one: divergent
                // lineage or missing history.
                return Err(LogError::Corruption);
            }
            let (_, bytes) = store::fetch_decision(
                &self.backend,
                &self.prefix,
                recovery.epoch_floor,
                record.object_epoch,
                &cursor.hash,
            )
            .map_err(|error| map_object_error(&error))?;
            let envelope = decision::decode_decision(&bytes, self.limits)?;
            cursor = envelope.parent;
            pending.push(bytes);
            work.checkpoint()?;
        }
        if cursor != local_stamp {
            return Err(LogError::Backend); // Budget exhausted before catching up.
        }
        // Apply forward (oldest first).
        for bytes in pending.into_iter().rev() {
            local = apply::materialize(&self.db, &local, &bytes, self.limits, work)
                .map_err(map_apply_error)?;
        }
        Ok(())
    }

    fn local_authority(&self, _work: &WorkContext) -> Result<HeadAuthority, LogError> {
        let control = read_attachment(&self.db)?.ok_or(LogError::NotInitialized)?;
        Ok(decode_control(&control, self.limits.envelope_bytes)?)
    }

    fn local_health(&self, target: &HeadAuthority, work: &WorkContext) -> LocalHealth {
        match self.local_authority(work) {
            Ok(local) => match (local.position(), target.position()) {
                (Some(local), Some(target)) if local.decision == target.decision => {
                    LocalHealth::Ready { at: local.decision }
                }
                _ => LocalHealth::Unavailable {
                    error: LogError::Backend,
                },
            },
            Err(error) => LocalHealth::Unavailable { error },
        }
    }

    fn retained(
        &self,
        id: CommandId,
        work: &WorkContext,
    ) -> Result<Option<TerminalReceipt>, LogError> {
        work.checkpoint()?;
        let reference = CommandRef {
            identity: self.identity,
            id,
            digest: crate::history::CommandDigest::from_bytes([0; 32]),
        };
        let key = receipt_key(id);
        match read_host_record(&self.db, &key)? {
            Some(bytes) => Ok(Some(decode_receipt_row(reference, &bytes, self.limits)?)),
            None => Ok(None),
        }
    }
}

/// Decode a composed head body to its record. Malformed composed frames are
/// corruption-class at this boundary.
fn decode_record(body: &[u8], limits: Limits) -> Result<HeadRecord, LogError> {
    Ok(manifest::decode_head(body, limits.envelope_bytes)?)
}

/// Map a verified-object failure onto the machine's certainty vocabulary:
/// transport/unproven-durability keeps uncertainty; every definite verified
/// refusal (absence inside the protected window, wrong bytes, an immutable
/// name holding foreign bytes) is corruption-class evidence.
fn map_object_error(error: &ObjectError) -> LogError {
    match error {
        ObjectError::Backend(_) | ObjectError::Unverified { .. } => LogError::Backend,
        ObjectError::Missing { .. }
        | ObjectError::WrongLength { .. }
        | ObjectError::WrongDigest { .. }
        | ObjectError::ImmutableConflict { .. }
        | ObjectError::Frame(_) => LogError::Corruption,
    }
}

fn map_apply_error(error: ApplyError) -> LogError {
    match error {
        ApplyError::Frame(_) | ApplyError::Chain(_) | ApplyError::OutcomeMismatch => {
            LogError::Corruption
        }
        ApplyError::Command(error) | ApplyError::Local(error) => error,
    }
}

fn write_local_genesis<S>(
    db: &Db<S>,
    authority: &HeadAuthority,
    limits: Limits,
    work: &WorkContext,
) -> Result<(), LogError> {
    if read_attachment(db)?.is_some() {
        return Err(LogError::Corruption);
    }
    let control = encode_control(authority, limits.envelope_bytes)?;
    let mut session = db.integration_writer(work)?;
    let empty = ChangeSet::builder(db.schema(), work.clone())
        .finish()
        .map_err(|error| LogError::Core(error.into()))?;
    let prepared = match session.prepare(&empty)? {
        bumbledb::Admission::Accepted(prepared) => prepared,
        bumbledb::Admission::Rejected(_) => return Err(LogError::Corruption),
    };
    prepared
        .seal(HostChanges {
            records: &[],
            attachment: AttachmentChange::Put(&control),
        })?
        .commit()?;
    Ok(())
}

enum SubmitFailure {
    NotSubmitted(LogError),
    Unknown(LogError),
}

impl From<LogError> for SubmitFailure {
    fn from(error: LogError) -> Self {
        // A bare LogError at a read boundary means no dispatch happened.
        Self::NotSubmitted(error)
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "one AttemptResult is consumed on the frame that built it; \
              the terminal receipt is a fixed-size value, not stored state"
)]
enum AttemptResult {
    Published {
        receipt: TerminalReceipt,
        local_health: LocalHealth,
    },
    Lost,
    /// The CAS outcome is unknown; resolve it after the writer session drops.
    Unknown {
        stamp: DecisionStamp,
    },
}

fn fingerprint<S>(db: &Db<S>) -> SchemaId {
    bumbledb::schema::fingerprint::fingerprint(db.schema())
}

fn read_attachment<S>(db: &Db<S>) -> Result<Option<Vec<u8>>, LogError> {
    let mut owned = None;
    db.read(|read| {
        owned = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })?;
    Ok(owned)
}

fn read_host_record<S>(db: &Db<S>, key: &[u8]) -> Result<Option<Vec<u8>>, LogError> {
    let mut owned = None;
    let mut host_error = None;
    db.read(|read| {
        match read.integration_host_record(key) {
            Ok(record) => owned = record.map(<[u8]>::to_vec),
            Err(error) => host_error = Some(error),
        }
        Ok(())
    })?;
    if let Some(error) = host_error {
        return Err(error.into());
    }
    Ok(owned)
}

#[cfg(test)]
mod tests {
    //! The per-call bound arithmetic (P04R concern 5). Machine-level submit
    //! coverage lives in `tests/writer_hosted.rs`. Verification: `NotRun`.

    use super::*;

    #[test]
    fn per_call_attempts_can_only_narrow_the_machine_budget() {
        // No override: the machine bound governs.
        assert_eq!(effective_attempts(DEFAULT_ATTEMPTS, None), DEFAULT_ATTEMPTS);
        // A wider request clamps DOWN to the machine's own bound.
        assert_eq!(effective_attempts(16, Some(64)), 16);
        // A narrower request governs this call.
        assert_eq!(effective_attempts(16, Some(3)), 3);
        // Zero still makes exactly one attempt — a submit is never a no-op
        // that fabricates NotSubmitted without dispatching.
        assert_eq!(effective_attempts(16, Some(0)), 1);
        assert_eq!(effective_attempts(1, Some(9)), 1);
    }

    #[test]
    fn backoff_delays_double_and_clamp_to_the_machine_cap() {
        let options = SubmitOptions {
            attempts: None,
            backoff_base: Some(Duration::from_millis(10)),
            backoff_cap: Some(Duration::from_millis(35)),
        };
        assert_eq!(backoff_delay(options, 0), Some(Duration::from_millis(10)));
        assert_eq!(backoff_delay(options, 1), Some(Duration::from_millis(20)));
        // The per-call cap bounds the doubling.
        assert_eq!(backoff_delay(options, 2), Some(Duration::from_millis(35)));
        assert_eq!(backoff_delay(options, 63), Some(Duration::from_millis(35)));
    }

    #[test]
    fn a_hostile_wire_backoff_cannot_exceed_the_machine_bound() {
        // A huge cap AND base clamp to MAX_BACKOFF — a per-call override can
        // never park an owning worker beyond the machine's own bound.
        let hostile = SubmitOptions {
            attempts: None,
            backoff_base: Some(Duration::from_secs(3_600)),
            backoff_cap: Some(Duration::from_hours(24)),
        };
        assert_eq!(backoff_delay(hostile, 0), Some(MAX_BACKOFF));
        // An absent cap defaults to the machine bound, not to unbounded.
        let uncapped = SubmitOptions {
            attempts: None,
            backoff_base: Some(Duration::from_secs(60)),
            backoff_cap: None,
        };
        assert_eq!(backoff_delay(uncapped, 5), Some(MAX_BACKOFF));
    }

    #[test]
    fn zero_or_absent_backoff_means_immediate_retry() {
        assert_eq!(backoff_delay(SubmitOptions::DEFAULT, 0), None);
        let zero_base = SubmitOptions {
            attempts: None,
            backoff_base: Some(Duration::ZERO),
            backoff_cap: Some(Duration::from_secs(1)),
        };
        assert_eq!(backoff_delay(zero_base, 3), None);
        let zero_cap = SubmitOptions {
            attempts: None,
            backoff_base: Some(Duration::from_millis(10)),
            backoff_cap: Some(Duration::ZERO),
        };
        assert_eq!(backoff_delay(zero_cap, 0), None);
    }
}
