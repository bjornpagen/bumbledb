//! `HostedHistory`: one never-reused HEAD over immutable decision objects.
//!
//! The publication machine prepares a private LMDB candidate against the
//! captured head, uploads the one immutable decision object, then conditionally
//! replaces HEAD. The successful atomic replacement is the linearization point;
//! only then does the local LMDB transaction commit. A losing CAS re-evaluates
//! the *same* immutable command against the winner. An unknown CAS/upload —
//! including an adapter transport error DURING the dispatched CAS — never
//! becomes a fabricated success, rejection or `NotSubmitted`: it resolves by
//! reading the head and the retained receipt (a receipt is the decision; a
//! consumed version token with no receipt is a proven loss that re-attempts),
//! or returns `OutcomeUnknown` with the retained ref when nothing is provable.
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
use crate::certainty::{CoveredNegativeProof, LocalParent, SubmitCertainty};
use crate::history::admission::{Resolution, Refusal};
use crate::checkpointer::{Headroom, admission_headroom};
use crate::history::admission::Submission;
use crate::history::authority::{
    Activation, ActivationCause, HeadAuthority, decode_control, encode_control,
};
use crate::history::command::{Command, Limits};
use crate::history::decision::{self, GenesisProvenance, GenesisRecord};
use crate::history::locator::{ChainVisitor, walk_decision_chain};
use crate::history::receipt::{decode_receipt_row, receipt_key};
use crate::history::{
    CommandRef, DatabaseId, DatabaseIdentity, DecisionStamp, IncarnationId, OperationId,
    SchemaId, TerminalReceipt,
};
use crate::manifest::{self, HeadRecord, TailPolicy};
use crate::replica::WitnessCheck;
use crate::store::{
    self, BackendError, ChargedBytes, ObjectError, ObjectKind, ObservedError, ReceiveLimits,
    ReceivedHead, ReceivingStore, TransportContext, TransportObservation, read_head_bounded,
};

use super::decide::{self, Judged, Plan, RealPrepared};
use super::verbs::{ConditionalOutcome, HeadVersion};
use super::{LocalHealth, LogError, ResolveOutcome, SubmitOutcome};

/// Default bounded publication-attempt budget across CAS losses.
pub const DEFAULT_ATTEMPTS: u32 = 16;
/// Default bounded catch-up decisions applied per refresh.
pub const DEFAULT_CATCH_UP: u32 = 4096;
/// The default FINITE durable-tail envelope every hosted machine enforces
/// (C08). `create`/`open` start here; `UNBOUNDED` exists only by explicit
/// [`HostedHistory::with_tail_policy`] configuration — an unconfigured
/// deployment must not grow an unbounded tail.
///
/// The bounds are justified against the machine's own walk budget: a tail of
/// at most [`DEFAULT_CATCH_UP`] decisions means any warm cache inside the
/// tail window catches up (or witnesses ancestry) within one default-budget
/// walk, and the 1 GiB byte bound caps cold recovery's tail-replay I/O
/// whatever the individual decision sizes are.
pub const DEFAULT_TAIL_POLICY: TailPolicy = TailPolicy {
    max_count: DEFAULT_CATCH_UP as u64,
    max_bytes: 1 << 30,
};
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
    /// The configured durable-tail envelope (C08). Starts at the finite
    /// [`DEFAULT_TAIL_POLICY`]; `UNBOUNDED` only by the explicit
    /// [`Self::with_tail_policy`] option. The composed-head grammar enforces
    /// whatever is configured on every decided composition, no-ops included.
    tail_policy: TailPolicy,
    attempts: u32,
    catch_up_bound: u32,
}

impl<S, B> HostedHistory<S, B>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    /// Create a hosted incarnation: install the genesis composed head by
    /// conditional create. The local materialization is initialized to the
    /// same genesis. Refuses a foreign existing head. `object_epoch` is the
    /// initial open object epoch recorded in the genesis head (GC barriers
    /// advance it afterwards; this machine reads the current epoch from the
    /// head).
    ///
    /// Creation certainty is resolved by EVIDENCE, never by status guessing
    /// (the backup manifest install pattern): a create whose conditional
    /// outcome is ambiguous — or whose precondition failed — reads the head
    /// back and compares it against this call's deterministic genesis. Our
    /// own bytes (or our own activation evidence on an advanced head) are an
    /// idempotent created-by-us success; foreign bytes are a real
    /// `CommandIdentityConflict` (the wire's `AuthorityExists`); an unreadable
    /// head stays unknown-typed (`Backend`), never a fabricated refusal.
    ///
    /// # Errors
    /// Refuses a foreign existing head (`CommandIdentityConflict`), a
    /// foreign-schema local database, backend failures and frame/work limits.
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
        match backend.create_head(&head_key, &body) {
            Ok(ConditionalOutcome::Published { .. }) => {}
            // Publication is only a typed Published arm. Predispatch
            // PreconditionFailed, typed Indeterminate, and transport
            // observations (Denied/Capped/generic Indeterminate) are not
            // published or lost — resolve by reading the evidence.
            Ok(ConditionalOutcome::PreconditionFailed | ConditionalOutcome::Indeterminate)
            | Err(_) => {
                resolve_create_evidence(
                    &backend,
                    &head_key,
                    &body,
                    identity,
                    operation,
                    genesis.hash,
                    limits,
                    work,
                )?;
            }
        }
        Ok(Self {
            db,
            backend,
            prefix,
            identity,
            limits,
            tail_policy: DEFAULT_TAIL_POLICY,
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
        let record = read_decoded_head(&backend, &head_key, limits, work)?;
        if record.control.identity.schema_id != fingerprint(&db) {
            return Err(LogError::Identity);
        }
        let history = Self {
            db,
            backend,
            prefix,
            identity: record.control.identity,
            limits,
            tail_policy: DEFAULT_TAIL_POLICY,
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
    /// decided composition that would exceed them. Machines start at the
    /// finite [`DEFAULT_TAIL_POLICY`]; [`TailPolicy::UNBOUNDED`] is available
    /// only through this explicit option.
    #[must_use]
    pub const fn with_tail_policy(mut self, policy: TailPolicy) -> Self {
        self.tail_policy = policy;
        self
    }

    /// Configure the bounded decision-window walk budget shared by catch-up
    /// and the ancestry witness ([`Self::witness_ancestor`]). At least one
    /// step is always taken; the budget bounds fetches, never correctness —
    /// an exhausted witness budget is `WitnessUnavailable`, and an exhausted
    /// catch-up reports failure rather than a partial materialization.
    #[must_use]
    pub const fn with_catch_up_bound(mut self, bound: u32) -> Self {
        self.catch_up_bound = if bound == 0 { 1 } else { bound };
        self
    }

    /// Submit one owned sealed command through the hosted authority, under
    /// the machine's own configured bounds.
    #[must_use]
    pub fn submit(&self, command: &Command, work: &WorkContext) -> SubmitOutcome {
        self.submit_with(command, SubmitOptions::DEFAULT, work)
    }

    /// Submit with per-call bounds, returning phase-carrying certainty for the
    /// native bridge (chapter 61 captured authority).
    #[must_use]
    pub fn submit_certain_with(
        &self,
        command: &Command,
        options: SubmitOptions,
        work: &WorkContext,
    ) -> SubmitCertainty {
        let reference = command.command_ref();
        match self.try_submit(command, options, work) {
            Ok(SubmitOutcome::Decided {
                receipt,
                local_health,
            }) => SubmitCertainty::Decided {
                receipt,
                local_health,
            },
            Ok(SubmitOutcome::NotSubmitted { command, error }) => SubmitCertainty::NotSubmitted {
                command,
                error,
            },
            Ok(SubmitOutcome::OutcomeUnknown { command, error }) => {
                SubmitCertainty::OutcomeUnknown { command, error }
            }
            Err(SubmitFailure::NotSubmitted(error)) => SubmitCertainty::NotSubmitted {
                command: reference,
                error,
            },
            Err(SubmitFailure::Unknown(error)) => SubmitCertainty::OutcomeUnknown {
                command: reference,
                error,
            },
        }
    }

    /// Phase-carrying submit under the machine's configured bounds.
    #[must_use]
    pub fn submit_certain(&self, command: &Command, work: &WorkContext) -> SubmitCertainty {
        self.submit_certain_with(command, SubmitOptions::DEFAULT, work)
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
        let mut dispatched = false;
        for attempt in 0..attempts {
            let predispatch = |error: LogError| {
                if dispatched {
                    SubmitFailure::Unknown(error)
                } else {
                    SubmitFailure::NotSubmitted(error)
                }
            };
            work.checkpoint()
                .map_err(|e| predispatch(e.into()))?;
            let head_key = store::head_key(&self.prefix);
            let (record, version, body) = read_captured_head(
                &self.backend,
                &head_key,
                self.limits,
                work,
            )
            .map_err(predispatch)?;
            // Bring the local materialization to this captured tip before
            // preparing a candidate against it.
            self.catch_up_to(&record, work)
                .map_err(predispatch)?;

            let frontier = self.local_frontier(reference, work).map_err(predispatch)?;
            if frontier.row_present && frontier.receipt.is_none() {
                return Err(predispatch(LogError::IncompleteRejectionEvidence));
            }
            let view = record
                .control
                .admission_view()
                .ok_or_else(|| predispatch(LogError::DatabaseDeleted))?;
            let plan = match view.submit(
                reference,
                command.metadata().condition,
                frontier.receipt.as_ref(),
            )
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
                Err(refusal) => return Err(predispatch(refusal.into())),
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
                return Err(predispatch(LogError::MaintenanceRequired {
                    count: recovery.tail_count(),
                    bytes: recovery.tail_bytes,
                }));
            }

            // `attempt_publish` returns with its writer session already
            // dropped, so resolving an unknown CAS here cannot deadlock on the
            // writer lock.
            match self.attempt_publish(
                command,
                &record,
                body.as_bytes(),
                &version,
                plan,
                dispatched,
                work,
            )? {
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
                AttemptResult::Unknown { stamp, attempted } => {
                    dispatched = true;
                    match self.resolve_after_unknown(reference, stamp, &attempted, work)? {
                        UnknownResolution::Decided {
                            receipt,
                            local_health,
                        } => {
                            return Ok(SubmitOutcome::Decided {
                                receipt,
                                local_health,
                            });
                        }
                        // The version token this attempt conditioned on was
                        // consumed by ANOTHER writer and the complete retained
                        // lookup after catch-up holds no receipt for this
                        // command: the dispatched CAS provably lost and can
                        // never win (head versions are never reused). Chapter
                        // 20's ladder: a proven loss re-attempts under the
                        // bounded budget, exactly like a typed CAS loss.
                        UnknownResolution::ProvenLoss => {
                            if attempt + 1 < attempts
                                && let Some(delay) = backoff_delay(options, attempt)
                            {
                                std::thread::sleep(delay);
                            }
                        }
                        UnknownResolution::ExpiredUnprovable => {
                            return Err(SubmitFailure::Unknown(LogError::ReceiptExpiredUnknown));
                        }
                        UnknownResolution::Unresolved => {
                            return Err(SubmitFailure::Unknown(LogError::Backend));
                        }
                    }
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
        already_dispatched: bool,
        work: &WorkContext,
    ) -> Result<AttemptResult, SubmitFailure> {
        let fail = |error: LogError| {
            if already_dispatched {
                SubmitFailure::Unknown(error)
            } else {
                SubmitFailure::NotSubmitted(error)
            }
        };
        // Prepare and seal the private candidate on the owning worker. The
        // sealed LMDB transaction is held across the remote attempt and only
        // commits after publication is known. Encode/admit bytes before the
        // HEAD CAS (C5).
        let mut session = self
            .db
            .integration_writer(work)
            .map_err(|e| fail(e.into()))?;
        let schema = self.db.schema();
        let authority = &parent.control;
        let parent_object = parent.recovery.and_then(|recovery| recovery.tip_object);
        let candidate = match plan {
            Plan::PreconditionFailed { expected, observed } => {
                let prepared = decide::prepare_empty(&mut session, schema, work).map_err(fail)?;
                decide::seal_candidate(
                    prepared,
                    authority,
                    command,
                    Judged::PreconditionFailed { expected, observed },
                    parent_object,
                    self.limits,
                )
                .map_err(fail)?
            }
            Plan::Evaluate => {
                match decide::prepare_real(&mut session, schema, command, self.limits, work)
                    .map_err(fail)?
                {
                    RealPrepared::Admitted { prepared, judged } => {
                        decide::seal_candidate(
                            prepared,
                            authority,
                            command,
                            judged,
                            parent_object,
                            self.limits,
                        )
                        .map_err(fail)?
                    }
                    RealPrepared::Rejected { evidence } => {
                        let prepared =
                            decide::prepare_empty(&mut session, schema, work).map_err(fail)?;
                        decide::seal_candidate(
                            prepared,
                            authority,
                            command,
                            Judged::InvariantRejected { evidence },
                            parent_object,
                            self.limits,
                        )
                        .map_err(fail)?
                    }
                }
            }
        };
        let stamp = candidate.receipt.decision_at;

        // Upload the one immutable decision object under the PARENT head's
        // open object epoch (the GC reference-introduction rule). Immutable
        // content equality absorbs an ambiguous re-PUT; an unresolved upload
        // is a local abort — the candidate was never proven durable, no CAS.
        let decision_ref = match store::put_verified(
            &self.backend,
            &self.prefix,
            parent.object_epoch,
            ObjectKind::Decision,
            &candidate.decision_bytes,
        ) {
            Ok(reference) => reference,
            Err(error) => {
                return Err(fail(map_object_error(&error)));
            }
        };

        // Compose HEAD' from this exact parent body — control advanced,
        // retention fields preserved, tail accounting grown and the envelope
        // enforced — and conditionally replace it. Encoding finishes before
        // the dispatch boundary.
        let body = manifest::decided_head_body(
            parent_body,
            &candidate.new_authority,
            candidate.decision_bytes.len() as u64,
            Some(decision_ref),
            &self.tail_policy,
            self.limits.envelope_bytes,
        )
        .map_err(|error| fail(error.into()))?;
        let head_key = store::head_key(&self.prefix);
        match self.backend.replace_head(&head_key, version, &body) {
            Ok(ConditionalOutcome::Published { .. }) => {
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
            Ok(ConditionalOutcome::PreconditionFailed) => {
                drop(candidate);
                Ok(AttemptResult::Lost)
            }
            Ok(ConditionalOutcome::Indeterminate) => {
                let attempted = version.clone();
                drop(candidate);
                Ok(AttemptResult::Unknown { stamp, attempted })
            }
            Err(error) => {
                // Transport observations are never a publication verdict.
                match error.observation() {
                    TransportObservation::Missing
                    | TransportObservation::Denied
                    | TransportObservation::Bucket
                    | TransportObservation::Region
                    | TransportObservation::Precondition
                    | TransportObservation::Conflict
                    | TransportObservation::Capped
                    | TransportObservation::Indeterminate => {
                        let attempted = version.clone();
                        drop(candidate);
                        Ok(AttemptResult::Unknown { stamp, attempted })
                    }
                }
            }
        }
    }

    // After an unknown CAS, install the applicable authority then read one
    // owned local snapshot of control + receipt (C5 / LOG-005). A remote
    // retirement floor plus a later unrelated local lookup is not a proof.
    // Changed HEAD alone is never proved loss; retirement is expired-unprovable.
    fn resolve_after_unknown(
        &self,
        reference: CommandRef,
        stamp: DecisionStamp,
        attempted: &HeadVersion,
        work: &WorkContext,
    ) -> Result<UnknownResolution, SubmitFailure> {
        let head_key = store::head_key(&self.prefix);
        let (record, current, charged) = read_captured_head(
            &self.backend,
            &head_key,
            self.limits,
            work,
        )
        .map_err(SubmitFailure::Unknown)?;
        drop(charged);
        self.catch_up_to(&record, work)
            .map_err(SubmitFailure::Unknown)?;
        let frontier = self
            .local_frontier(reference, work)
            .map_err(SubmitFailure::Unknown)?;
        if let Some(receipt) = frontier.receipt {
            let _ = stamp;
            if receipt.command.digest != reference.digest {
                return Err(SubmitFailure::Unknown(LogError::CommandIdentityConflict));
            }
            return Ok(UnknownResolution::Decided {
                receipt,
                local_health: self.local_health(&frontier.control, work),
            });
        }
        if frontier.row_present {
            // A present row that failed optional diagnostic decode is not
            // absence (LOG-029). The original identity stays resolvable.
            return Ok(UnknownResolution::Unresolved);
        }
        let version_consumed = current != *attempted;
        if !version_consumed {
            return Ok(UnknownResolution::Unresolved);
        }
        let view = frontier
            .control
            .admission_view()
            .ok_or(SubmitFailure::Unknown(LogError::DatabaseDeleted))?;
        let decision_at = match view.resolve(reference, None) {
            Err(Refusal::ReceiptExpiredUnknown) => {
                return Ok(UnknownResolution::ExpiredUnprovable);
            }
            Ok(Resolution::NotRecordedAt { decision_at }) => decision_at,
            Err(Refusal::CommandEpochClosed) => view.decision,
            Ok(Resolution::Found(_)) => unreachable!("same snapshot had no receipt"),
            Err(refusal) => return Err(SubmitFailure::Unknown(refusal.into())),
        };
        match CoveredNegativeProof::try_covered_loss(
            reference,
            attempted.0.clone(),
            frontier.control.identity,
            decision_at,
            frontier.control.revision,
            view.receipts.retired_through(),
            view.receipts.open_epoch(),
            true,
            frontier.row_present,
        ) {
            Some(_) => Ok(UnknownResolution::ProvenLoss),
            None => Ok(UnknownResolution::ExpiredUnprovable),
        }
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
        let record = read_decoded_head(&self.backend, &head_key, self.limits, work)?;
        self.catch_up_to(&record, work)?;
        let frontier = self.local_frontier(command, work)?;
        let view = frontier
            .control
            .admission_view()
            .ok_or(LogError::DatabaseDeleted)?;
        if frontier.row_present && frontier.receipt.is_none() {
            // Present receipt row, diagnostic decode failed: not absence
            // and not a discarded decided receipt (LOG-029).
            return Err(LogError::IncompleteRejectionEvidence);
        }
        match view.resolve(command, frontier.receipt.as_ref()) {
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
        let record = read_decoded_head(&self.backend, &head_key, self.limits, work)?;
        let target = record
            .control
            .position()
            .ok_or(LogError::DatabaseDeleted)?
            .decision;
        self.catch_up_to(&record, work)?;
        Ok(target)
    }

    /// Bounded same-lineage ancestry check over RETAINED authoritative
    /// evidence (chapter 20 receipts/roots / chapter 30 `AtLeast`): prove
    /// that `requested` is an ancestor of the CAPTURED `tip` using the
    /// composed head's root evidence and the protected decision chain — a
    /// sequence-zero request is judged against the one-time activation
    /// evidence, the checkpoint base stamp is itself root evidence, and any
    /// other retained height is reached by walking the verified decision
    /// objects backward from `tip` (each object is fetched by the digest its
    /// parent named, so the walk authenticates the exact lineage). Evidence
    /// pruned below the checkpoint base — or a walk that exhausts the
    /// bounded window budget — is `WitnessUnavailable`, never a claimed
    /// validation and never a sequence-integer comparison.
    ///
    /// # Errors
    /// Refuses backend transport failures (`Backend`), verified-object
    /// corruption, a tombstoned head (`DatabaseDeleted`), a missing head
    /// (`NotInitialized`) and stopped work.
    pub fn witness_ancestor(
        &self,
        tip: DecisionStamp,
        requested: DecisionStamp,
        work: &WorkContext,
    ) -> Result<WitnessCheck, LogError> {
        work.checkpoint()?;
        let head_key = store::head_key(&self.prefix);
        let record = read_decoded_head(&self.backend, &head_key, self.limits, work)?;
        if record.control.position().is_none() {
            return Err(LogError::DatabaseDeleted);
        }
        // Exact-height shortcuts over retained control/root evidence.
        if requested.seq == tip.seq {
            return Ok(if requested.hash == tip.hash {
                WitnessCheck::Ancestor
            } else {
                WitnessCheck::NotAncestor
            });
        }
        if requested.seq > tip.seq {
            // A future coordinate is the caller's NotYetAvailable lane, not
            // an ancestry verdict; nothing retained can witness it.
            return Ok(WitnessCheck::Unavailable);
        }
        if requested.seq == 0 {
            return Ok(match record.control.activation {
                crate::history::authority::Activation::Activated { target_genesis, .. } => {
                    if target_genesis == requested.hash {
                        WitnessCheck::Ancestor
                    } else {
                        WitnessCheck::NotAncestor
                    }
                }
                crate::history::authority::Activation::NotActivated => WitnessCheck::Unavailable,
            });
        }
        let recovery = record.recovery.ok_or(LogError::Corruption)?;
        if recovery.base.seq == requested.seq {
            // The checkpoint base stamp IS retained root evidence.
            return Ok(if recovery.base.hash == requested.hash {
                WitnessCheck::Ancestor
            } else {
                WitnessCheck::NotAncestor
            });
        }
        if requested.seq < recovery.base.seq {
            // C6: walk stops at the captured base. Below-base evidence is
            // unavailable, never a private older fetch.
            return Ok(WitnessCheck::Unavailable);
        }
        // Checkpoint-only: base == tip, no tip locator, no suffix walk.
        if recovery.base == recovery.tip {
            return Ok(WitnessCheck::Unavailable);
        }
        if tip != recovery.tip {
            return Ok(WitnessCheck::Unavailable);
        }
        let tip_object = recovery.tip_object.ok_or(LogError::Corruption)?;
        let mut budget = self.catch_up_bound as u64;
        let mut visitor = AncestryVisitor {
            requested,
            found: None,
        };
        walk_decision_chain(
            &self.backend,
            &self.prefix,
            recovery.tip,
            recovery.base,
            Some(tip_object),
            self.limits,
            &mut budget,
            work,
            &mut visitor,
        )
        .map_err(map_object_error)?;
        Ok(match visitor.found {
            Some(hash) if hash == requested.hash => WitnessCheck::Ancestor,
            Some(_) => WitnessCheck::NotAncestor,
            None => WitnessCheck::Unavailable,
        })
    }

    /// Stream the authenticated suffix from the captured tip back to the
    /// local decision, then apply oldest-first within `catch_up_bound`.
    /// Checkpoint-only roots (`base == tip`) have no tip locator and cannot
    /// walk a suffix. A local cache older than the checkpoint base is
    /// `MaterializationStale` — never an empty fallback or epoch probe.
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
        let target_revision = record.control.revision;
        let local_revision = local.revision;
        if local_stamp == target_stamp && local_revision == target_revision {
            return Ok(());
        }
        let recovery = record.recovery.ok_or(LogError::Corruption)?;
        if local_stamp != target_stamp {
            if recovery.base == recovery.tip {
                return Err(LogError::MaterializationStale);
            }
            if local_stamp.seq < recovery.base.seq {
                return Err(LogError::MaterializationStale);
            }
            if local_stamp.seq == recovery.base.seq && local_stamp != recovery.base {
                return Err(LogError::Corruption);
            }
            if target_stamp != recovery.tip {
                return Err(LogError::Corruption);
            }
            let tip_object = recovery.tip_object.ok_or(LogError::Corruption)?;
            let mut budget = self.catch_up_bound as u64;
            let mut visitor = CatchUpBuffer {
                pending: Vec::new(),
                cap: self.catch_up_bound,
            };
            walk_decision_chain(
                &self.backend,
                &self.prefix,
                recovery.tip,
                local_stamp,
                Some(tip_object),
                self.limits,
                &mut budget,
                work,
                &mut visitor,
            )
            .map_err(map_object_error)?;
            // Stream is newest-first; apply the bounded suffix oldest-first.
            for bytes in visitor.pending.into_iter().rev() {
                local = apply::materialize(&self.db, &local, &bytes, self.limits, work)
                    .map_err(map_apply_error)?;
            }
        }
        // Control-only HEAD changes still need installation (LOG-004).
        if local.revision != target_revision || local != record.control {
            self.install_captured_control(&record.control, work)?;
        }
        Ok(())
    }

    fn install_captured_control(
        &self,
        control: &HeadAuthority,
        work: &WorkContext,
    ) -> Result<(), LogError> {
        let control_bytes =
            crate::history::authority::encode_control(control, self.limits.envelope_bytes)
                .map_err(|_| LogError::Corruption)?;
        let mut session = self
            .db
            .integration_writer(work)
            .map_err(|e| LogError::from(e))?;
        // Revalidate identity + decision + control under this writer (C5).
        // Same-tip maintenance is legal; a newer local revision must not regress.
        let local = read_attachment(&self.db, work)?
            .ok_or(LogError::NotInitialized)
            .and_then(|bytes| {
                decode_control(&bytes, self.limits.envelope_bytes).map_err(LogError::from)
            })?;
        if local.identity != control.identity {
            return Err(LogError::Identity);
        }
        match (local.position(), control.position()) {
            (Some(here), Some(incoming)) if here.decision != incoming.decision => {
                return Err(LogError::Corruption);
            }
            (Some(_), Some(_)) if local.revision.0 > control.revision.0 => {
                return Err(LogError::Corruption);
            }
            _ => {}
        }
        if local == *control {
            return Ok(());
        }
        let _captured = LocalParent {
            identity: local.identity,
            decision: local
                .position()
                .map(|position| position.decision)
                .ok_or(LogError::DatabaseDeleted)?,
            revision: local.revision,
        };
        let empty = bumbledb::ChangeSet::builder(self.db.schema(), work.clone())
            .finish()
            .map_err(|e| LogError::Core(e.into()))?;
        let prepared = match session.prepare(&empty)? {
            bumbledb::Admission::Accepted(prepared) => prepared,
            bumbledb::Admission::Rejected(_) => return Err(LogError::Corruption),
        };
        prepared
            .seal(bumbledb::integration::HostChanges {
                records: &[],
                attachment: bumbledb::integration::AttachmentChange::Put(&control_bytes),
            })
            .map_err(|e| LogError::from(e))?
            .commit()
            .map_err(|e| LogError::from(e))?;
        Ok(())
    }

    fn local_authority(&self, work: &WorkContext) -> Result<HeadAuthority, LogError> {
        let control = read_attachment(&self.db, work)?.ok_or(LogError::NotInitialized)?;
        Ok(decode_control(&control, self.limits.envelope_bytes)?)
    }

    /// Control and receipt from one owned read (C5 coherent frontier).
    fn local_frontier(
        &self,
        command: CommandRef,
        work: &WorkContext,
    ) -> Result<LocalFrontier, LogError> {
        work.checkpoint()?;
        let key = receipt_key(command.id);
        let scoped = CommandRef {
            identity: self.identity,
            id: command.id,
            digest: crate::history::CommandDigest::from_bytes([0; 32]),
        };
        let mut attachment = None;
        let mut row = None;
        let mut host_error = None;
        self.db.read(work.clone(), |read| {
            attachment = read.integration_host_attachment()?.map(<[u8]>::to_vec);
            match read.integration_host_record(&key) {
                Ok(record) => row = record.map(<[u8]>::to_vec),
                Err(error) => host_error = Some(error),
            }
            Ok(())
        })?;
        if let Some(error) = host_error {
            return Err(error.into());
        }
        let control = decode_control(
            &attachment.ok_or(LogError::NotInitialized)?,
            self.limits.envelope_bytes,
        )?;
        let (receipt, row_present) = match row {
            None => (None, false),
            Some(bytes) => match decode_receipt_row(scoped, &bytes, self.limits) {
                Ok(receipt) => (Some(receipt), true),
                Err(_) => (None, true),
            },
        };
        Ok(LocalFrontier {
            control,
            receipt,
            row_present,
        })
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

}

/// Decode a composed head body to its record. Malformed composed frames are
/// corruption-class at this boundary.
fn decode_record(body: &[u8], limits: Limits) -> Result<HeadRecord, LogError> {
    Ok(manifest::decode_head(body, limits.envelope_bytes)?)
}

fn read_captured_head<B>(
    backend: &B,
    head_key: &str,
    limits: Limits,
    work: &WorkContext,
) -> Result<(HeadRecord, HeadVersion, ChargedBytes), LogError>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    work.checkpoint()?;
    match read_head_bounded(
        backend,
        head_key,
        TransportContext::new(work, ReceiveLimits::capped(limits.envelope_bytes as u64)),
    )
    .map_err(map_object_error)?
    {
        ReceivedHead::Present { version, body } => {
            let charged = body.into_charged().ok_or(LogError::Backend)?;
            let record = decode_record(charged.as_bytes(), limits)?;
            Ok((record, version, charged.into_owner()))
        }
        ReceivedHead::Absent => Err(LogError::NotInitialized),
    }
}

fn read_decoded_head<B>(
    backend: &B,
    head_key: &str,
    limits: Limits,
    work: &WorkContext,
) -> Result<HeadRecord, LogError>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    let (record, _, charged) = read_captured_head(backend, head_key, limits, work)?;
    drop(charged);
    Ok(record)
}

/// Resolve an uncertain (or precondition-failed) hosted creation by EVIDENCE
/// (the backup manifest install pattern): read the head back and compare it
/// against this call's own deterministic genesis.
///
/// - Byte-exact genesis body: created by us (this attempt, or an earlier
///   identical one) — idempotent success.
/// - A decodable head whose identity AND one-time activation evidence
///   (operation + target genesis) are ours: our creation already advanced —
///   still success.
/// - Any other decodable head: a foreign authority holds this prefix —
///   `CommandIdentityConflict` (the wire's `AuthorityExists`).
/// - A malformed body is corruption-class evidence; an unreadable or absent
///   head stays unknown-typed (`Backend`) — the create may still land, and
///   uncertainty is never rewritten into a refusal.
fn resolve_create_evidence<B>(
    backend: &B,
    head_key: &str,
    genesis_body: &[u8],
    identity: DatabaseIdentity,
    operation: OperationId,
    target_genesis: crate::history::DecisionDigest,
    limits: Limits,
    work: &WorkContext,
) -> Result<(), LogError>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    let found = match read_head_bounded(
        backend,
        head_key,
        TransportContext::new(work, ReceiveLimits::capped(limits.envelope_bytes as u64)),
    )
    .map_err(map_object_error)?
    {
        ReceivedHead::Present { body, .. } => body,
        ReceivedHead::Absent => return Err(LogError::Backend),
    };
    if found.as_bytes() == genesis_body {
        drop(found);
        return Ok(());
    }
    let record = decode_record(found.as_bytes(), limits)?;
    drop(found);
    if record.control.identity == identity
        && matches!(
            record.control.activation,
            Activation::Activated {
                operation: held,
                target_genesis: held_genesis,
                ..
            } if held == operation && held_genesis == target_genesis
        )
    {
        return Ok(());
    }
    Err(LogError::CommandIdentityConflict)
}

/// Map a verified-object failure onto the machine's certainty vocabulary:
/// transport/unproven-durability keeps uncertainty; every definite verified
/// refusal (absence inside the protected window, wrong bytes, an immutable
/// name holding foreign bytes) is corruption-class evidence.
fn map_object_error(error: &ObjectError) -> LogError {
    match error {
        ObjectError::Backend(_) | ObjectError::Unverified { .. } => LogError::Backend,
        // Denied/bucket/region are transport observations, not publication
        // and not a covered loss.
        ObjectError::Denied { .. } | ObjectError::Bucket { .. } | ObjectError::Region { .. } => {
            LogError::Backend
        }
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
        // No authority yet — not a post-install settlement failure.
        ApplyError::UnpublishedDestination => LogError::NotInitialized,
        ApplyError::Command(error) | ApplyError::Local(error) => error,
    }
}

fn write_local_genesis<S>(
    db: &Db<S>,
    authority: &HeadAuthority,
    limits: Limits,
    work: &WorkContext,
) -> Result<(), LogError> {
    let control = encode_control(authority, limits.envelope_bytes)?;
    if let Some(existing) = read_attachment(db, work)? {
        // The genesis control encoding is deterministic: a byte-identical
        // attachment is an earlier attempt of THIS same creation — idempotent
        // evidence (a create retry must resolve, not refuse). Anything else
        // is a foreign local materialization.
        if existing == control {
            return Ok(());
        }
        return Err(LogError::Corruption);
    }
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

/// There is deliberately NO blanket `From<LogError> for SubmitFailure`: every
/// conversion names its certainty arm explicitly, so a post-dispatch failure
/// can never silently ride a `?` into a fabricated `NotSubmitted`.
enum SubmitFailure {
    NotSubmitted(LogError),
    Unknown(LogError),
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
    /// `attempted` is the exact head version the CAS conditioned on — the
    /// evidence that distinguishes a proven loss from genuine uncertainty.
    Unknown {
        stamp: DecisionStamp,
        attempted: HeadVersion,
    },
}

/// What the post-Indeterminate evidence ladder established.
#[expect(
    clippy::large_enum_variant,
    reason = "consumed immediately on the resolving frame; the terminal \
              receipt is a fixed-size value, not stored state"
)]
enum UnknownResolution {
    /// The command is durably decided; the retained receipt is authoritative.
    Decided {
        receipt: TerminalReceipt,
        local_health: LocalHealth,
    },
    /// The conditioned version token was consumed by another writer and the
    /// coherent receipt frontier excludes this command: re-attempt.
    ProvenLoss,
    /// The receipt epoch was retired; loss cannot be proved from absence.
    ExpiredUnprovable,
    /// Nothing provable: the dispatched request may still land.
    Unresolved,
}

fn fingerprint<S>(db: &Db<S>) -> SchemaId {
    bumbledb::schema::fingerprint::fingerprint(db.schema())
}

fn read_attachment<S>(db: &Db<S>, work: &WorkContext) -> Result<Option<Vec<u8>>, LogError> {
    let mut owned = None;
    db.read(work.clone(), |read| {
        owned = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })?;
    Ok(owned)
}

struct LocalFrontier {
    control: HeadAuthority,
    receipt: Option<TerminalReceipt>,
    row_present: bool,
}

/// Bounded newest-first suffix records. The walk contract is the visitor;
/// this buffer exists only so apply can run oldest-first within `cap`.
struct CatchUpBuffer {
    pending: Vec<Vec<u8>>,
    cap: u32,
}

impl ChainVisitor for CatchUpBuffer {
    type Error = crate::store::ObjectError;

    fn visit(
        &mut self,
        _stamp: DecisionStamp,
        bytes: &[u8],
        _reference: crate::store::ObjectRef,
    ) -> Result<bool, Self::Error> {
        if self.pending.len() as u32 >= self.cap {
            return Err(crate::store::ObjectError::Backend(Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "decision walk budget exhausted",
            ))));
        }
        self.pending.push(bytes.to_vec());
        Ok(true)
    }
}

struct AncestryVisitor {
    requested: DecisionStamp,
    found: Option<crate::history::DecisionDigest>,
}

impl ChainVisitor for AncestryVisitor {
    type Error = crate::store::ObjectError;

    fn visit(
        &mut self,
        stamp: DecisionStamp,
        _bytes: &[u8],
        _reference: crate::store::ObjectRef,
    ) -> Result<bool, Self::Error> {
        if stamp.seq == self.requested.seq {
            self.found = Some(stamp.hash);
            return Ok(false);
        }
        Ok(true)
    }
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
