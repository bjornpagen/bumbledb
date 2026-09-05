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
//! Set idempotence (chapter 02) means re-applying a decision whose effects are
//! already present nets no fact change, so the crash window between a remote
//! publication and the local commit needs no extra detection state: replay is
//! safe to repeat.

use bumbledb::{Db, WorkContext};

use crate::history::authority::HeadAuthority;
use crate::history::command::{Command, Limits, UnverifiedOutcome};
use crate::history::decision::{self, ChainError};
use crate::history::{DecisionStamp, TerminalOutcome};

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

/// Materialize one published decision onto the local store, extending exactly
/// `authority_before`. Returns the new local authority on success.
///
/// # Errors
/// Refuses malformed frames, wrong parents, foreign commands, replay
/// divergence and local storage failures — never committing on any of them.
pub fn materialize<S>(
    db: &Db<S>,
    authority_before: &HeadAuthority,
    decision_bytes: &[u8],
    limits: Limits,
    work: &WorkContext,
) -> Result<HeadAuthority, ApplyError> {
    work.checkpoint().map_err(|e| ApplyError::Local(e.into()))?;
    let envelope = decision::decode_decision(decision_bytes, limits)?;
    let live = authority_before
        .live()
        .map_err(|e| ApplyError::Local(e.into()))?;
    decision::verify_step(live.decision, &envelope)?;
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

    let plan = plan_for(&envelope.outcome);
    let mut session = db
        .integration_writer(work)
        .map_err(|e| ApplyError::Local(e.into()))?;
    let schema = db.schema();
    let candidate = match plan {
        Plan::PreconditionFailed { expected, observed } => {
            let prepared =
                decide::prepare_empty(&mut session, schema, work).map_err(ApplyError::Local)?;
            decide::seal_candidate(
                prepared,
                authority_before,
                &command,
                Judged::PreconditionFailed { expected, observed },
                limits,
            )
            .map_err(ApplyError::Local)?
        }
        Plan::Evaluate => {
            match decide::prepare_real(&mut session, schema, &command, limits, work)
                .map_err(ApplyError::Local)?
            {
                RealPrepared::Admitted { prepared, judged } => {
                    decide::seal_candidate(prepared, authority_before, &command, judged, limits)
                        .map_err(ApplyError::Local)?
                }
                RealPrepared::Rejected { evidence } => {
                    let prepared = decide::prepare_empty(&mut session, schema, work)
                        .map_err(ApplyError::Local)?;
                    decide::seal_candidate(
                        prepared,
                        authority_before,
                        &command,
                        Judged::InvariantRejected { evidence },
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

fn plan_for(outcome: &UnverifiedOutcome<'_>) -> Plan {
    match outcome {
        UnverifiedOutcome::PreconditionFailed { expected, observed } => Plan::PreconditionFailed {
            expected: *expected,
            observed: *observed,
        },
        // Committed / NoChange / InvariantRejected all re-judge the delta.
        _ => Plan::Evaluate,
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
