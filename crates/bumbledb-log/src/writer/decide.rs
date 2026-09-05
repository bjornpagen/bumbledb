//! Shared judgment: turn one admitted command into a sealed candidate whose
//! host records (retained receipt row + updated control attachment) ride the
//! same core LMDB transaction as its facts.
//!
//! Both backends and historical replay use these pieces. The difference is
//! only *when* the sealed transaction commits: `LocalHistory` commits
//! immediately; `HostedHistory` holds the sealed candidate on its owning worker
//! across the remote HEAD attempt and commits after publication is known. A
//! rejected application delta is aborted and replaced by an empty delta plus
//! its rejection receipt in the same exclusive writer session, so no
//! application fact changes after judgment and a losing candidate is never
//! committed.
//!
//! The API is deliberately split into `prepare_real` / `prepare_empty` /
//! `seal_candidate` called at the owner of the `WriterSession` local: a single
//! `&'owner mut session -> Candidate<'owner>` function cannot re-borrow the
//! session on its rejected path under stable borrowck (the accepted arm's
//! escaping `PreparedWrite` pins the borrow to the universal region — NLL
//! problem case #3). Callers match on [`RealPrepared`] where the session is a
//! local, so each borrow's region stays existential and the rejected arm may
//! prepare the empty replacement delta.

use bumbledb::integration::{
    AttachmentChange, HostChanges, HostRecordChange, PreparedWrite, SealedWrite, WriterSession,
};
use bumbledb::schema::Schema;
use bumbledb::schema::evidence::{self, EvidenceError};
use bumbledb::{Admission, ChangeSet, Violations, WorkContext};

use crate::history::authority::HeadAuthority;
use crate::history::command::{Command, Limits, UnverifiedOutcome};
use crate::history::decision::{self, DecisionParts};
use crate::history::receipt::{encode_receipt_row, receipt_key};
use crate::history::{
    ChangeSummary, CommandResult, DecisionStamp, RejectionEvidence, StateStamp, TerminalOutcome,
    TerminalReceipt,
};

use super::LogError;

/// What admission decided for this command before candidate preparation.
#[derive(Debug, Clone, Copy)]
pub enum Plan {
    /// Judge the command's real delta against the private candidate.
    Evaluate,
    /// The exact-state witness already lost; still a durable decision.
    PreconditionFailed {
        expected: StateStamp,
        observed: StateStamp,
    },
}

/// The judged terminal shape of one command, owned data only. Constructing
/// this value is not evidence a decision was durable; [`seal_candidate`] and
/// the outer commit make it one.
#[derive(Debug, Clone)]
pub enum Judged {
    Committed {
        changed: ChangeSummary,
    },
    NoChange,
    PreconditionFailed {
        expected: StateStamp,
        observed: StateStamp,
    },
    InvariantRejected {
        evidence: RejectionEvidence,
    },
}

/// A candidate whose facts and host records are sealed into one core
/// transaction, awaiting the backend's publication commit. The sealed write
/// borrows the writer session for its whole lifetime.
pub struct Candidate<'owner, 'db, S> {
    pub sealed: SealedWrite<'owner, 'db, S>,
    pub receipt: TerminalReceipt,
    pub new_authority: HeadAuthority,
    /// The exact immutable decision bytes; a hosted backend uploads these.
    /// `LocalHistory` needs the digest only and does not persist them.
    pub decision_bytes: Vec<u8>,
}

/// `prepare_real`'s outcome: either the admitted private candidate (which
/// borrows the session until sealed/committed) or the owned canonical
/// rejection evidence — the caller then prepares the empty replacement delta
/// on the same session.
pub enum RealPrepared<'owner, 'db, S> {
    Admitted {
        prepared: PreparedWrite<'owner, 'db, S>,
        judged: Judged,
    },
    Rejected {
        evidence: RejectionEvidence,
    },
}

/// Prepare and judge the command's real delta as the private candidate.
///
/// # Errors
/// Refuses core admission/storage failures, exhausted work, and
/// (deliberately, before deciding) an oversized rejection diagnostic rather
/// than recording a falsely complete rejection.
pub fn prepare_real<'owner, 'db, S>(
    session: &'owner mut WriterSession<'db, S>,
    schema: &Schema,
    command: &Command,
    limits: Limits,
    work: &WorkContext,
) -> Result<RealPrepared<'owner, 'db, S>, LogError> {
    work.checkpoint()?;
    match session.prepare(command.changes())? {
        Admission::Accepted(prepared) => {
            let counts = prepared.application_changes();
            let judged = match ChangeSummary::new(counts.added, counts.removed) {
                Some(changed) => Judged::Committed { changed },
                None => Judged::NoChange,
            };
            Ok(RealPrepared::Admitted { prepared, judged })
        }
        Admission::Rejected(violations) => {
            let evidence = encode_rejection_evidence(schema, &violations, limits, work)?;
            Ok(RealPrepared::Rejected { evidence })
        }
    }
}

/// Prepare the empty replacement delta for a precondition-failed or rejected
/// decision: the same exclusive session prepares it after the real candidate
/// aborted, so no application facts change after judgment.
///
/// # Errors
/// Core admission/storage failures and exhausted work.
pub fn prepare_empty<'owner, 'db, S>(
    session: &'owner mut WriterSession<'db, S>,
    schema: &Schema,
    work: &WorkContext,
) -> Result<PreparedWrite<'owner, 'db, S>, LogError> {
    let empty = ChangeSet::builder(schema, work.clone())
        .finish()
        .map_err(|error| LogError::Core(error.into()))?;
    match session.prepare(&empty)? {
        Admission::Accepted(prepared) => Ok(prepared),
        // The empty delta cannot violate a law that the current state satisfies.
        Admission::Rejected(_) => Err(LogError::Corruption),
    }
}

/// Frame the decision for one judged outcome and seal its receipt row plus the
/// advanced control attachment into the prepared transaction. On return the
/// private application facts are decided and immutable; only the outer commit
/// remains.
///
/// # Errors
/// Refuses exhausted coordinates, frame limits and host-record seal failures.
pub fn seal_candidate<'owner, 'db, S>(
    prepared: PreparedWrite<'owner, 'db, S>,
    authority: &HeadAuthority,
    command: &Command,
    judged: Judged,
    parent_object: Option<crate::store::ObjectRef>,
    limits: Limits,
) -> Result<Candidate<'owner, 'db, S>, LogError> {
    let facts_changed = matches!(judged, Judged::Committed { .. });
    let position = authority.position().ok_or(LogError::DatabaseDeleted)?;
    let before_state = position.state;
    let after_state = if facts_changed {
        StateStamp {
            data_revision: before_state
                .data_revision
                .checked_add(1)
                .ok_or(LogError::Corruption)?,
            ..before_state
        }
    } else {
        before_state
    };

    let result_bytes = command.result().as_bytes().to_vec();
    let outcome_view = judged.as_unverified(&result_bytes);
    let decision_bytes = decision::encode_decision(
        DecisionParts {
            identity: authority.identity,
            seq: position
                .decision
                .seq
                .checked_add(1)
                .ok_or(LogError::Corruption)?,
            parent: position.decision,
            parent_object,
            before_state,
            after_state,
            canonical_command: &command.encode(limits)?,
            outcome: outcome_view,
        },
        limits,
    )?;
    let digest = decision::decision_digest(&decision_bytes);
    let new_authority = authority.decided(digest, facts_changed)?;
    let stamp = DecisionStamp {
        seq: position.decision.seq + 1,
        hash: digest,
    };

    let receipt = TerminalReceipt {
        command: command.command_ref(),
        decision_at: stamp,
        state_at: after_state,
        outcome: judged.into_owned(command.result().clone()),
    };

    let row = encode_receipt_row(&receipt, limits)?;
    let control = crate::history::authority::encode_control(&new_authority, limits.envelope_bytes)?;
    let key = receipt_key(command.command_ref().id);
    let records = [HostRecordChange::Put {
        key: &key,
        value: &row,
    }];
    let sealed = prepared.seal(HostChanges {
        records: &records,
        attachment: AttachmentChange::Put(&control),
    })?;

    Ok(Candidate {
        sealed,
        receipt,
        new_authority,
        decision_bytes,
    })
}

// ---------------------------------------------------------------------------
// Canonical rejection evidence (C01/C03 — the core-owned codec)
// ---------------------------------------------------------------------------

/// Encode one rejection's complete violation set through the CORE's canonical
/// evidence codec (`bumbledb::schema::evidence`, family
/// `bumbledb.evidence.v1`): the complete violated-statement set with bounded,
/// deterministically truncated example facts. The log frames these bytes
/// verbatim into decisions and receipts; the native runtime decodes them back
/// through the same module for the public `Violation[]` surface. Deterministic
/// over `(schema, violations, budget)`, so historical replay re-derives
/// byte-identical evidence (`apply` compares exactly).
///
/// This replaces the earlier `format!("{violations}")` Display placeholder
/// recorded as a C12 defect in implementation/packets/P04.md: Display bytes
/// were neither strict nor version-stable.
///
/// A diagnostic whose complete statement skeleton exceeds the budget — or a
/// resource failure while producing it — refuses before deciding
/// (`LogError::IncompleteRejectionEvidence` / `Work`) rather than recording a
/// falsely complete rejection. Resource exhaustion is never shorter evidence.
fn encode_rejection_evidence(
    schema: &Schema,
    violations: &Violations,
    limits: Limits,
    work: &WorkContext,
) -> Result<RejectionEvidence, LogError> {
    let bytes = evidence::encode_violations(schema, violations, limits.evidence_bytes, work)
        .map_err(|error| match error {
            EvidenceError::Work(work) => LogError::Work(work),
            // Budget, allocation, row-encode and impossible-shape refusals all
            // mean this invocation could not produce COMPLETE bounded
            // evidence: a nonterminal refusal before deciding (chapter 20),
            // never a shorter recorded verdict.
            EvidenceError::Budget { .. }
            | EvidenceError::Row(_)
            | EvidenceError::Empty
            | EvidenceError::Unordered
            | EvidenceError::PointwiseConflict
            | EvidenceError::ForeignStatement
            | EvidenceError::ForeignRelation { .. }
            | EvidenceError::LengthOverflow
            | EvidenceError::Allocation => LogError::IncompleteRejectionEvidence,
        })?;
    RejectionEvidence::from_canonical_bytes(bytes.into_boxed_slice())
        .ok_or(LogError::IncompleteRejectionEvidence)
}

impl Judged {
    fn as_unverified<'a>(&'a self, result: &'a [u8]) -> UnverifiedOutcome<'a> {
        match self {
            Self::Committed { changed } => UnverifiedOutcome::Committed {
                changed: *changed,
                result,
            },
            Self::NoChange => UnverifiedOutcome::NoChange { result },
            Self::PreconditionFailed { expected, observed } => {
                UnverifiedOutcome::PreconditionFailed {
                    expected: *expected,
                    observed: *observed,
                }
            }
            Self::InvariantRejected { evidence } => UnverifiedOutcome::InvariantRejected {
                core_evidence: evidence.as_bytes(),
            },
        }
    }

    fn into_owned(self, result: CommandResult) -> TerminalOutcome {
        match self {
            Self::Committed { changed } => TerminalOutcome::Committed { changed, result },
            Self::NoChange => TerminalOutcome::NoChange { result },
            Self::PreconditionFailed { expected, observed } => {
                TerminalOutcome::PreconditionFailed { expected, observed }
            }
            Self::InvariantRejected { evidence } => TerminalOutcome::InvariantRejected { evidence },
        }
    }
}
