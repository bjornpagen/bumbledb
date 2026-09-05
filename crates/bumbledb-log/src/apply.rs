//! Apply: the one place a published decision becomes local engine state.
//!
//! Materialization is historical replay, not a call to current command
//! submission. It verifies identity/sequence/parent, decodes the embedded
//! canonical command with the core's one strict decoder, re-judges the delta
//! at the exact predecessor, and checks the recomputed outcome/decision digest
//! against the recorded decision. It ignores current admission guards
//! (freezing/retirement/deletion are maintenance, not decision-chain facts)
//! and never runs a host callback.
//!
//! Ready-only: the destination is an admitted [`bumbledb::Db`] that already
//! holds a committed authority attachment. Unready staging is a different
//! owner and cannot enter this function. Dest name is not readiness.
//!
//! ```compile_fail
//! fn unready_cannot_materialize(
//!     unready: &bumbledb::store::UnreadyStore,
//!     authority: &bumbledb_log::history::authority::HeadAuthority,
//!     limits: bumbledb_log::history::command::Limits,
//!     work: &bumbledb::WorkContext,
//! ) {
//!     let _ = bumbledb_log::apply::materialize(unready, authority, b"", limits, work);
//! }
//! ```
//!
//! Set idempotence (chapter 02) means re-applying a decision whose effects are
//! already present nets no fact change, so the crash window between a remote
//! publication and the local commit needs no extra detection state: replay is
//! safe to repeat.

use std::time::Duration;

use bumbledb::{Db, ExecutionPolicy, WorkContext};

use crate::history::authority::{HeadAuthority, decode_control};
use crate::history::command::{Command, Limits, UnverifiedOutcome};
use crate::history::decision::{self, ChainError};
use crate::history::{Condition, DecisionStamp, StateStamp, TerminalOutcome};

use crate::writer::LogError;
use crate::writer::decide::{self, Judged, Plan, RealPrepared};

/// Why a materialization refused. Every arm is corruption-class: the object,
/// the chain it claims, or the outcome it records disagrees with an honest
/// re-judgment, and no retry mends the bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    /// The decision frame or its embedded command frame was malformed.
    Frame(crate::history::FrameError),
    /// The decision does not extend exactly this local parent.
    Chain(ChainError),
    /// The embedded command decoded but disagreed with the local schema.
    Command(LogError),
    /// The recorded outcome (or resulting digest) disagrees with re-judgment.
    OutcomeMismatch,
    /// A core storage/work failure while replaying.
    Local(LogError),
    /// No committed authority on this admitted handle. Not a dest-name
    /// check and not a post-install I/O failure — those stay [`Self::Local`].
    UnpublishedDestination,
}

impl From<crate::history::FrameError> for ApplyError {
    fn from(error: crate::history::FrameError) -> Self {
        Self::Frame(error)
    }
}

impl From<ChainError> for ApplyError {
    fn from(error: ChainError) -> Self {
        Self::Chain(error)
    }
}

/// Materialize one published decision onto an admitted local store that
/// already holds authority, extending exactly `authority_before`.
///
/// Readiness is admitted [`Db`] ownership plus a committed control
/// attachment. L10 hydrates an unready owner and publishes once; this
/// function does not rename, admit, or promote. A dest whose name looks
/// like a staging sibling is still ready when those two facts hold.
/// Failures after that check are [`ApplyError::Local`] / replay
/// disagreement — settlement evidence, not "nothing was installed".
///
/// # Errors
/// Refuses a handle with no authority, malformed frames, wrong parents,
/// foreign commands, replay divergence and local storage failures — never
/// committing on any of them.
pub fn materialize<S>(
    db: &Db<S>,
    authority_before: &HeadAuthority,
    decision_bytes: &[u8],
    limits: Limits,
    work: &WorkContext,
) -> Result<HeadAuthority, ApplyError> {
    work.checkpoint().map_err(|e| ApplyError::Local(e.into()))?;
    require_published_destination(db, work)?;
    let envelope = decision::decode_decision(decision_bytes, limits)?;
    if envelope.identity != authority_before.identity {
        return Err(ApplyError::Command(LogError::Identity));
    }

    // Re-decode the embedded command through the core's strict decoder. This
    // re-derives and checks its digest against the decision's recorded command.
    let command = Command::parse(db.schema(), envelope.canonical_command, limits, work)
        .map_err(|e| ApplyError::Command(e.into()))?;
    if command.command_ref() != envelope.command {
        return Err(ApplyError::OutcomeMismatch);
    }

    let parent_object = envelope.parent_object;
    let mut session = db
        .integration_writer(work)
        .map_err(|e| ApplyError::Local(e.into()))?;
    // Validate the actual predecessor while holding the writer fence (LOG-003):
    // a stale caller snapshot cannot seal control against later local facts.
    let actual = read_committed_authority(db, limits.envelope_bytes)
        .map_err(ApplyError::Local)?;
    if already_at(&actual, envelope.stamp()) {
        return Ok(actual);
    }
    let live = actual.live().map_err(|e| ApplyError::Local(e.into()))?;
    decision::verify_step(live.decision, &envelope)?;
    // Historical precondition is the command's exact predecessor under this
    // writer — never current admission, freeze, or retirement (LOG-006).
    let plan = plan_for(&command, &actual);
    let schema = db.schema();
    let candidate = match plan {
        Plan::PreconditionFailed { expected, observed } => {
            let prepared =
                decide::prepare_empty(&mut session, schema, work).map_err(ApplyError::Local)?;
            decide::seal_candidate(
                prepared,
                &actual,
                &command,
                Judged::PreconditionFailed { expected, observed },
                parent_object,
                limits,
            )
            .map_err(ApplyError::Local)?
        }
        Plan::Evaluate => {
            match decide::prepare_real(&mut session, schema, &command, limits, work)
                .map_err(ApplyError::Local)?
            {
                RealPrepared::Admitted { prepared, judged } => {
                    decide::seal_candidate(
                        prepared,
                        &actual,
                        &command,
                        judged,
                        parent_object,
                        limits,
                    )
                    .map_err(ApplyError::Local)?
                }
                RealPrepared::Rejected { evidence } => {
                    let prepared = decide::prepare_empty(&mut session, schema, work)
                        .map_err(ApplyError::Local)?;
                    decide::seal_candidate(
                        prepared,
                        &actual,
                        &command,
                        Judged::InvariantRejected { evidence },
                        parent_object,
                        limits,
                    )
                    .map_err(ApplyError::Local)?
                }
            }
        }
    };

    // The re-judged outcome and resulting decision digest must equal the
    // recorded decision exactly; otherwise this is a divergent replay.
    if !outcomes_agree(&candidate.receipt.outcome, &envelope.outcome)
        || candidate.receipt.decision_at != envelope.stamp()
    {
        // Dropping the candidate (SealedWrite) aborts the private transaction.
        return Err(ApplyError::OutcomeMismatch);
    }
    let new_authority = candidate.new_authority;
    candidate
        .sealed
        .commit()
        .map_err(|e| ApplyError::Local(e.into()))?;
    Ok(new_authority)
}

/// Admitted [`Db`] plus a committed authority attachment. Dest name is
/// ignored. A later I/O failure here is [`ApplyError::Local`], not
/// unpublished — the handle may already be the installed incarnation.
///
/// # Errors
/// [`ApplyError::UnpublishedDestination`] when no control attachment
/// exists; [`ApplyError::Local`] on work/storage failure.
pub fn require_published_destination<S>(
    db: &Db<S>,
    work: &WorkContext,
) -> Result<(), ApplyError> {
    work.checkpoint().map_err(|e| ApplyError::Local(e.into()))?;
    let mut owned: Option<Vec<u8>> = None;
    db.read(work.clone(), |read| {
        owned = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })
    .map_err(|e| ApplyError::Local(e.into()))?;
    match owned {
        Some(_) => Ok(()),
        None => Err(ApplyError::UnpublishedDestination),
    }
}

fn read_committed_authority<S>(
    db: &Db<S>,
    cap: usize,
) -> Result<HeadAuthority, LogError> {
    let mut owned: Option<Vec<u8>> = None;
    let work = ExecutionPolicy {
        input_bytes: 64 * 1024 * 1024,
        working_bytes: 64 * 1024 * 1024,
        scratch_bytes: 64 * 1024 * 1024,
        result_bytes: 64 * 1024 * 1024,
        rows: 1_000_000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(3600),
    }
    .start()
    .map_err(|error| LogError::Work(error))?;
    db.read(work, |read| {
        owned = read.integration_host_attachment()?.map(<[u8]>::to_vec);
        Ok(())
    })?;
    let control = owned.ok_or(LogError::NotInitialized)?;
    Ok(decode_control(&control, cap)?)
}

fn plan_for(command: &Command, authority: &HeadAuthority) -> Plan {
    match command.metadata().condition {
        Condition::Unconditional => Plan::Evaluate,
        Condition::ExactState(stamp) => {
            let position = authority.position();
            match position {
                Some(pos) if pos.state == stamp => Plan::Evaluate,
                Some(pos) => Plan::PreconditionFailed {
                    expected: stamp,
                    observed: pos.state,
                },
                None => Plan::PreconditionFailed {
                    expected: stamp,
                    observed: StateStamp {
                        incarnation: authority.identity.incarnation_id,
                        data_revision: 0,
                    },
                },
            }
        }
    }
}

fn outcomes_agree(judged: &TerminalOutcome, recorded: &UnverifiedOutcome<'_>) -> bool {
    match (judged, recorded) {
        (
            TerminalOutcome::Committed { changed, result },
            UnverifiedOutcome::Committed {
                changed: recorded_changed,
                result: recorded_result,
            },
        ) => changed == recorded_changed && result.as_bytes() == *recorded_result,
        (
            TerminalOutcome::NoChange { result },
            UnverifiedOutcome::NoChange {
                result: recorded_result,
            },
        ) => result.as_bytes() == *recorded_result,
        (
            TerminalOutcome::PreconditionFailed { expected, observed },
            UnverifiedOutcome::PreconditionFailed {
                expected: recorded_expected,
                observed: recorded_observed,
            },
        ) => expected == recorded_expected && observed == recorded_observed,
        (
            TerminalOutcome::InvariantRejected { evidence },
            UnverifiedOutcome::InvariantRejected {
                core_evidence: recorded_evidence,
            },
        ) => evidence.as_bytes() == *recorded_evidence,
        _ => false,
    }
}

/// The stamp a caller compares to recognize whether a decision has already
/// been materialized locally.
#[must_use]
pub fn already_at(authority: &HeadAuthority, stamp: DecisionStamp) -> bool {
    authority.live().is_ok_and(|live| live.decision == stamp)
}
