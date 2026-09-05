//! The internal durable history machine.
//!
//! Two backends expose the *same* selected command semantics. [`local`]
//! commits facts, the retained receipt and the head attachment in one LMDB
//! transaction; [`hosted`] publishes over one never-reused HEAD through the
//! C07 [`verbs::ConditionalStore`] seam owned by P05. Neither simulates the
//! other: LocalHistory does not emulate an object store, and no JS layer
//! reimplements this machine.
//!
//! There is no braid, vector floor, split commit, writer-ID fence,
//! deposition, issued-ID lease or callback replay here — those mechanisms are
//! deleted. A command is owned canonical data sealed before submission; a
//! losing candidate is never readable; certainty survives delayed/lost
//! responses, interruption and subsequent decisions.

pub(crate) mod decide;
pub mod hosted;
pub mod local;
pub mod verbs;

use bumbledb::WorkError;
use bumbledb::integration::{HostSealError, IntegrationError};

use crate::history::admission::Refusal;
use crate::history::authority::AuthorityError;
use crate::history::command::{CommandError, FrameError};
use crate::history::receipt::ReceiptRowError;
use crate::history::{CommandRef, DecisionStamp, TerminalReceipt};

pub use hosted::{HostedHistory, SubmitOptions};
pub use local::LocalHistory;

/// Where this materialization is, independent of the durable receipt. A
/// resolved receipt does not itself prove the local cache reached that
/// decision; `Unavailable` carries a cause without changing the outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalHealth {
    Ready { at: DecisionStamp },
    Unavailable { error: LogError },
}

/// The submission certainty union. This is A; there is no E. Interruption and
/// finalizer defects live in the caller's fiber/`Cause`, never rewritten here
/// into `NotSubmitted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// A durable terminal decision. All four terminal outcomes (commit,
    /// no-change, precondition-failed, invariant-rejected) arrive here.
    Decided {
        receipt: TerminalReceipt,
        local_health: LocalHealth,
    },
    /// This invocation dispatched no authoritative publication attempt. It is
    /// NOT proof that a prior/concurrent invocation of the same command never
    /// published.
    NotSubmitted {
        command: CommandRef,
        error: LogError,
    },
    /// Dispatched, but this invocation could not establish the outcome. The
    /// retained ref resolves it later; never downgraded to a rejection.
    OutcomeUnknown {
        command: CommandRef,
        error: LogError,
    },
}

/// A point-in-time resolution of a retained ref. `NotRecordedAt` is an
/// observation, not proof of nonpublication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveOutcome {
    Found(TerminalReceipt),
    NotRecordedAt { decision_at: DecisionStamp },
    CommandEpochClosed,
    ReceiptExpiredUnknown,
}

/// The internal machine's operational error. Distinct from the terminal
/// receipt data above: a durable rejection is a value, not an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogError {
    /// The command's identity/schema/witness does not match this authority.
    Identity,
    /// A durable identity conflict: same command ID, different digest.
    CommandIdentityConflict,
    /// The database authority is a terminal tombstone.
    DatabaseDeleted,
    /// New-command admission is frozen (retained lookup still works).
    DatabaseFrozen,
    /// The command's receipt epoch is closed to new commands.
    CommandEpochClosed,
    /// The command's receipt epoch was retired; execution permanently refused.
    ReceiptExpiredUnknown,
    /// The materialization has not been initialized; open never initializes.
    NotInitialized,
    /// A retained row / control / decision frame was malformed or foreign.
    Corruption,
    /// Bounded work/deadline/cancellation reached actual native/I/O work.
    Work(WorkError),
    /// A core admission/change/storage failure, retaining its typed cause.
    Core(CommandError),
    /// A core storage/read failure with its typed cause.
    Storage(bumbledb::Error),
    /// A host-record seal/read failure with its typed cause.
    HostSeal(bumbledb::integration::HostSealError),
    /// A reentrant/foreign-schema native writer refusal.
    Misuse,
    /// The rejection diagnostic could not be produced within budget; refuse
    /// before deciding rather than record a falsely complete rejection.
    IncompleteRejectionEvidence,
    /// A backend transport/lifecycle failure with no definite outcome.
    Backend,
    /// Admitting another decision would exceed the configured durable-tail
    /// envelope (count AND bytes, C08); checkpoint before new admission.
    /// A `NotSubmitted` backpressure refusal — retained lookup still works, and
    /// nothing was dispatched.
    MaintenanceRequired { count: u64, bytes: u64 },
    /// The local materialization predates the durable tail's checkpoint base:
    /// the decisions between them may legitimately be collected, so bounded
    /// catch-up cannot reach the tip. Re-open through recovery hydration
    /// (C08); never treated as corruption or an empty database.
    MaterializationStale,
}

impl From<WorkError> for LogError {
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<CommandError> for LogError {
    fn from(error: CommandError) -> Self {
        Self::Core(error)
    }
}

impl From<FrameError> for LogError {
    fn from(_: FrameError) -> Self {
        Self::Corruption
    }
}

impl From<ReceiptRowError> for LogError {
    fn from(error: ReceiptRowError) -> Self {
        match error {
            ReceiptRowError::Frame(_) | ReceiptRowError::ForeignRow => Self::Corruption,
        }
    }
}

impl From<Refusal> for LogError {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::IdentityMismatch | Refusal::StateIdentityMismatch => Self::Identity,
            Refusal::DatabaseDeleted => Self::DatabaseDeleted,
            Refusal::DatabaseFrozen => Self::DatabaseFrozen,
            Refusal::ReceiptExpiredUnknown => Self::ReceiptExpiredUnknown,
            Refusal::CommandIdentityConflict => Self::CommandIdentityConflict,
            Refusal::CommandEpochClosed | Refusal::CommandEpochNotOpen { .. } => {
                Self::CommandEpochClosed
            }
            Refusal::InvalidRetainedReceipt => Self::Corruption,
        }
    }
}

impl From<HostSealError> for LogError {
    fn from(error: HostSealError) -> Self {
        match error {
            HostSealError::Work(work) => Self::Work(work),
            other => Self::HostSeal(other),
        }
    }
}

impl From<bumbledb::Error> for LogError {
    fn from(error: bumbledb::Error) -> Self {
        Self::Storage(error)
    }
}

impl From<IntegrationError> for LogError {
    fn from(error: IntegrationError) -> Self {
        match error {
            IntegrationError::Core(error) => Self::Storage(error),
            IntegrationError::Changes(error) => Self::Core(CommandError::Core(error)),
            IntegrationError::Host(error) => Self::from(error),
            IntegrationError::Work(error) => Self::Work(error),
            IntegrationError::ForeignSchema | IntegrationError::ReentrantWriter => Self::Misuse,
        }
    }
}

impl From<crate::manifest::HeadError> for LogError {
    fn from(error: crate::manifest::HeadError) -> Self {
        use crate::manifest::HeadError;
        match error {
            HeadError::MaintenanceRequired { count, bytes } => {
                Self::MaintenanceRequired { count, bytes }
            }
            HeadError::Deleted => Self::DatabaseDeleted,
            // The publication machine never composes root mutations; a
            // malformed frame, missing recovery root or root-list refusal on
            // a read/decided composition is corruption-class evidence.
            HeadError::Frame(_)
            | HeadError::NoRecovery
            | HeadError::RootCapacityExceeded
            | HeadError::UnknownRoot
            | HeadError::DuplicateRoot => Self::Corruption,
            HeadError::Authority(error) => Self::from(error),
        }
    }
}

impl From<AuthorityError> for LogError {
    fn from(error: AuthorityError) -> Self {
        match error {
            AuthorityError::Deleted => Self::DatabaseDeleted,
            AuthorityError::Frozen { .. } => Self::DatabaseFrozen,
            AuthorityError::NotFrozen
            | AuthorityError::OperationMismatch { .. }
            | AuthorityError::ActivationEvidenceMismatch
            | AuthorityError::InvalidGenesis
            | AuthorityError::Exhausted(_)
            | AuthorityError::Policy(_) => Self::Corruption,
        }
    }
}
