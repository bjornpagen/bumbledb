//! Pure current-admission guards. Historical replay must not call these:
//! present-day freezing/retirement cannot invalidate an earlier decision.

use super::{
    AccessMode, CommandRef, Condition, DatabaseIdentity, DecisionStamp, ReceiptEpoch,
    ReceiptPolicy, StateStamp, TerminalReceipt,
};

/// One coherently captured authority projection. It is not assembled from
/// independently observed facts, stamps, and a sidecar receipt table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionView {
    pub identity: DatabaseIdentity,
    pub access: AccessMode,
    pub receipts: ReceiptPolicy,
    pub decision: DecisionStamp,
    pub state: StateStamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    IdentityMismatch,
    DatabaseDeleted,
    ReceiptExpiredUnknown,
    CommandIdentityConflict,
    CommandEpochClosed,
    CommandEpochNotOpen {
        open: ReceiptEpoch,
        requested: ReceiptEpoch,
    },
    DatabaseFrozen,
    StateIdentityMismatch,
    InvalidRetainedReceipt,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Resolution<'a> {
    Found(&'a TerminalReceipt),
    NotRecordedAt { decision_at: DecisionStamp },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Submission<'a> {
    AlreadyDecided(&'a TerminalReceipt),
    Evaluate,
    /// A terminal outcome *candidate*, still requiring an atomic durable receipt.
    PreconditionFailed {
        expected: StateStamp,
        observed: StateStamp,
    },
}

impl AdmissionView {
    /// Inspect one keyed receipt from this exact view. Wrong identity/deletion
    /// and retirement precede receipt lookup; receipt lookup precedes freezing
    /// or closed-epoch admission. Open absence is only a frontier observation.
    ///
    /// # Errors
    /// Refuses wrong identities, deleted/retired/closed namespaces, digest
    /// conflicts, invalid retained rows, and future epochs.
    pub fn resolve(
        self,
        command: CommandRef,
        retained: Option<&TerminalReceipt>,
    ) -> Result<Resolution<'_>, Refusal> {
        if command.identity != self.identity {
            return Err(Refusal::IdentityMismatch);
        }
        if self.access == AccessMode::Deleted {
            return Err(Refusal::DatabaseDeleted);
        }
        let epoch = command.id.receipt_epoch;
        if epoch.get() <= self.receipts.retired_through() {
            return Err(Refusal::ReceiptExpiredUnknown);
        }
        if epoch > self.receipts.open_epoch() {
            return Err(Refusal::CommandEpochNotOpen {
                open: self.receipts.open_epoch(),
                requested: epoch,
            });
        }
        if let Some(receipt) = retained {
            if receipt.command.identity != self.identity
                || receipt.command.id != command.id
                || receipt.decision_at.seq == 0
                || receipt.decision_at.seq > self.decision.seq
                || (receipt.decision_at.seq == self.decision.seq
                    && (receipt.decision_at != self.decision || receipt.state_at != self.state))
                || receipt.state_at.incarnation != self.identity.incarnation_id
                || receipt.state_at.data_revision > receipt.decision_at.seq
                || receipt.state_at.data_revision > self.state.data_revision
            {
                return Err(Refusal::InvalidRetainedReceipt);
            }
            if receipt.command.digest != command.digest {
                return Err(Refusal::CommandIdentityConflict);
            }
            return Ok(Resolution::Found(receipt));
        }
        if epoch < self.receipts.open_epoch() {
            return Err(Refusal::CommandEpochClosed);
        }
        Ok(Resolution::NotRecordedAt {
            decision_at: self.decision,
        })
    }

    /// Current submission only. The same command must be re-evaluated after a
    /// hosted CAS loss, not mutated or given a fresh request ID.
    ///
    /// # Errors
    /// Resolution refusals precede new-command access and state checks.
    pub fn submit(
        self,
        command: CommandRef,
        condition: Condition,
        retained: Option<&TerminalReceipt>,
    ) -> Result<Submission<'_>, Refusal> {
        match self.resolve(command, retained)? {
            Resolution::Found(receipt) => return Ok(Submission::AlreadyDecided(receipt)),
            Resolution::NotRecordedAt { .. } => {}
        }
        if self.access == AccessMode::Frozen {
            return Err(Refusal::DatabaseFrozen);
        }
        if self.state.incarnation != self.identity.incarnation_id {
            return Err(Refusal::StateIdentityMismatch);
        }
        if let Condition::ExactState(expected) = condition {
            if expected.incarnation != self.identity.incarnation_id {
                return Err(Refusal::StateIdentityMismatch);
            }
            if expected != self.state {
                return Ok(Submission::PreconditionFailed {
                    expected,
                    observed: self.state,
                });
            }
        }
        Ok(Submission::Evaluate)
    }
}
