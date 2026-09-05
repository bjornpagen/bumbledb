//! Publication certainty: one attempt sum per authoritative operation (C5).
//!
//! Each variant carries state-specific payloads only — phase is implicit in
//! the arm, never a freely chosen field beside a freely chosen outcome.
//! `NotStarted` cannot represent a dispatched attempt; `OutcomeUnknown` always
//! means dispatched-unresolved. A generic I/O error after dispatch never
//! becomes pre-dispatch refusal.
//!
//! These types are the E-facing contract: the native bridge maps them to
//! Effect outcomes without inferring phase from error variants.

use crate::history::{
    CommandRef, DatabaseIdentity, DecisionStamp, HeadRevision, ReceiptEpoch, TerminalReceipt,
};
use crate::writer::LogError;

/// Where an authoritative operation is in its publication lifecycle.
/// Derived from certainty arms for wire tagging — not stored independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicationPhase {
    /// No authoritative publication attempt was dispatched this invocation.
    Prepared,
    /// A publication attempt was dispatched; outcome not yet established.
    DispatchedUnresolved,
    /// Positive terminal evidence is known (receipt or admin transition).
    Confirmed,
    /// A valid negative proof: the exact conditional version was consumed
    /// and complete retained lookup excludes this attempt.
    ProvedNonpublication,
}

/// Command submission certainty — state-specific payloads E consumes.
#[derive(Debug)]
pub enum SubmitCertainty {
    /// Pre-dispatch refusal; no authoritative attempt was dispatched.
    NotSubmitted {
        command: CommandRef,
        error: LogError,
    },
    /// Dispatched; this invocation could not establish the outcome.
    OutcomeUnknown {
        command: CommandRef,
        error: LogError,
    },
    /// Positive terminal publication evidence is known.
    Decided {
        receipt: TerminalReceipt,
        local_health: crate::writer::LocalHealth,
    },
}

impl SubmitCertainty {
    #[must_use]
    pub const fn publication_phase(&self) -> PublicationPhase {
        match self {
            Self::NotSubmitted { .. } => PublicationPhase::Prepared,
            Self::OutcomeUnknown { .. } => PublicationPhase::DispatchedUnresolved,
            Self::Decided { .. } => PublicationPhase::Confirmed,
        }
    }
}

/// Administrative transition certainty — same attempt-sum grammar as commands.
#[derive(Debug)]
pub enum AdminCertainty<T> {
    /// Pre-dispatch refusal; no CAS was dispatched this invocation.
    NotStarted {
        error: crate::admin::AdminError,
    },
    /// Dispatched; this invocation could not establish the outcome.
    OutcomeUnknown {
        error: crate::admin::AdminError,
    },
    /// The transition completed (evidence or fresh publication).
    Completed {
        value: T,
    },
}

impl<T> AdminCertainty<T> {
    #[must_use]
    pub const fn publication_phase(&self) -> PublicationPhase {
        match self {
            Self::NotStarted { .. } => PublicationPhase::Prepared,
            Self::OutcomeUnknown { .. } => PublicationPhase::DispatchedUnresolved,
            Self::Completed { .. } => PublicationPhase::Confirmed,
        }
    }

    #[must_use]
    pub const fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    #[must_use]
    pub const fn dispatched(&self) -> bool {
        !matches!(self, Self::NotStarted { .. })
    }
}

/// One coherent local parent captured under the same writer that will
/// install a later control/receipt transition (C5 / LOG-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalParent {
    pub identity: DatabaseIdentity,
    pub decision: DecisionStamp,
    pub revision: HeadRevision,
}

/// Covered negative proof: consumed conditional version plus absence from
/// one owned frontier/receipt snapshot (C5 / LOG-005). Retirement is never
/// this value — retirement is [`ResolveEvidence::ReceiptExpiredUnknown`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveredNegativeProof {
    pub command: CommandRef,
    pub consumed_version: Box<[u8]>,
    pub identity: DatabaseIdentity,
    pub decision_at: DecisionStamp,
    pub revision: HeadRevision,
    pub retired_through: u64,
    pub open_epoch: ReceiptEpoch,
}

impl CoveredNegativeProof {
    /// Build a covered-loss proof only when the version token was consumed
    /// and the same snapshot still retains this command's epoch.
    #[must_use]
    pub fn try_covered_loss(
        command: CommandRef,
        consumed_version: Box<[u8]>,
        identity: DatabaseIdentity,
        decision_at: DecisionStamp,
        revision: HeadRevision,
        retired_through: u64,
        open_epoch: ReceiptEpoch,
        version_consumed: bool,
        row_present: bool,
    ) -> Option<Self> {
        if !version_consumed || row_present {
            return None;
        }
        if command.identity != identity {
            return None;
        }
        let epoch = command.id.receipt_epoch.get();
        if epoch <= retired_through {
            return None;
        }
        Some(Self {
            command,
            consumed_version,
            identity,
            decision_at,
            revision,
            retired_through,
            open_epoch,
        })
    }
}

/// Point-in-time resolve evidence for E. `NotRecordedAt` is an observation.
/// `CoveredLoss` is the only proved nonpublication arm; retirement is
/// expired-unprovable, never loss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveEvidence {
    Found(TerminalReceipt),
    NotRecordedAt { decision_at: DecisionStamp },
    CommandEpochClosed,
    ReceiptExpiredUnknown,
    CoveredLoss(CoveredNegativeProof),
}

impl ResolveEvidence {
    #[must_use]
    pub const fn publication_phase(&self) -> PublicationPhase {
        match self {
            Self::Found(_) => PublicationPhase::Confirmed,
            Self::NotRecordedAt { .. } | Self::CommandEpochClosed => {
                PublicationPhase::DispatchedUnresolved
            }
            Self::ReceiptExpiredUnknown => PublicationPhase::DispatchedUnresolved,
            Self::CoveredLoss(_) => PublicationPhase::ProvedNonpublication,
        }
    }
}
