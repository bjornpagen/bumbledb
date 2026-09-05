use bumbledb::{Id128, Violations};

use super::SchemaId;

macro_rules! identity_role {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Id128);

        impl $name {
            #[must_use]
            pub const fn from_core(value: Id128) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn as_core(self) -> Id128 {
                self.0
            }
        }
    };
}

identity_role!(
    DatabaseId,
    "Logical database identity, never a path or schema hash."
);
identity_role!(IncarnationId, "One non-forking history lineage.");
identity_role!(
    RequestId,
    "Named-request role, explicitly distinct from an entity ID."
);

/// Complete scope of every command, receipt, and materialized history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseIdentity {
    pub database_id: DatabaseId,
    pub incarnation_id: IncarnationId,
    pub schema_id: SchemaId,
}

macro_rules! digest_role {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name([u8; 32]);

        impl $name {
            /// Import a full-width digest; this does not authenticate its source.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }
    };
}

digest_role!(
    CommandDigest,
    "Full cryptographic binding of immutable command meaning."
);
digest_role!(
    DecisionDigest,
    "Full cryptographic binding of one decision or explicit genesis."
);

/// Every authoritative replacement, including maintenance, advances this role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadRevision(pub u64);

/// Every terminal outcome advances this role; genesis is sequence zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionStamp {
    pub seq: u64,
    pub hash: DecisionDigest,
}

/// Application state witness. A change-and-restore still advances revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateStamp {
    pub incarnation: IncarnationId,
    pub data_revision: u64,
}

/// Positive command-admission namespace. Zero means no epoch and is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReceiptEpoch(u64);

impl ReceiptEpoch {
    pub const INITIAL: Self = Self(1);

    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId {
    pub receipt_epoch: ReceiptEpoch,
    pub request_id: RequestId,
}

/// A copied recovery coordinate, independent of command/handle lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandRef {
    pub identity: DatabaseIdentity,
    pub id: CommandId,
    pub digest: CommandDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Unconditional,
    ExactState(StateStamp),
}

/// The first slice accepts only empty declared metadata. Nonempty scalar
/// metadata must use the future core result codec, not a log value vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyResult;

/// Net fact counts supplied by core judgment, never counts of input spelling.
/// A zero/zero report belongs to `NoChange`, not `Committed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangeSummary {
    added: u64,
    removed: u64,
}

impl ChangeSummary {
    #[must_use]
    pub const fn new(added: u64, removed: u64) -> Option<Self> {
        if added == 0 && removed == 0 {
            None
        } else {
            Some(Self { added, removed })
        }
    }

    #[must_use]
    pub const fn added(self) -> u64 {
        self.added
    }

    #[must_use]
    pub const fn removed(self) -> u64 {
        self.removed
    }
}

/// All four arms are terminal *only after* the authority's atomic commit.
/// Merely constructing this value is not evidence that a decision was durable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalOutcome {
    Committed {
        changed: ChangeSummary,
        result: EmptyResult,
    },
    NoChange {
        result: EmptyResult,
    },
    PreconditionFailed {
        expected: StateStamp,
        observed: StateStamp,
    },
    InvariantRejected {
        violations: Violations,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalReceipt {
    pub command: CommandRef,
    pub decision_at: DecisionStamp,
    pub state_at: StateStamp,
    pub outcome: TerminalOutcome,
}

/// The control projection needed by command admission; not a full authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    Active,
    Frozen,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptPolicy {
    open_epoch: ReceiptEpoch,
    retired_through: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    InvalidRetirement,
    EpochDidNotAdvance,
    RetirementMovedBackward,
}

impl ReceiptPolicy {
    pub const INITIAL: Self = Self {
        open_epoch: ReceiptEpoch::INITIAL,
        retired_through: 0,
    };

    /// # Errors
    /// Retirement must name only epochs strictly before the open epoch.
    pub const fn new(open_epoch: ReceiptEpoch, retired_through: u64) -> Result<Self, PolicyError> {
        if retired_through >= open_epoch.get() {
            Err(PolicyError::InvalidRetirement)
        } else {
            Ok(Self {
                open_epoch,
                retired_through,
            })
        }
    }

    #[must_use]
    pub const fn open_epoch(self) -> ReceiptEpoch {
        self.open_epoch
    }

    #[must_use]
    pub const fn retired_through(self) -> u64 {
        self.retired_through
    }

    /// Compute new controls. The authority must commit them atomically.
    /// # Errors
    /// Rotation must strictly advance and cannot wrap or reuse an epoch.
    pub const fn rotate(self, next: ReceiptEpoch) -> Result<Self, PolicyError> {
        if next.get() <= self.open_epoch.get() {
            Err(PolicyError::EpochDidNotAdvance)
        } else {
            Ok(Self {
                open_epoch: next,
                ..self
            })
        }
    }

    /// Compute retirement controls; current receipt rows must be removed in
    /// the same authority transaction, not as a later best-effort cleanup.
    /// # Errors
    /// Retirement is monotone and cannot include the open epoch.
    pub const fn retire(self, through: u64) -> Result<Self, PolicyError> {
        if through < self.retired_through {
            Err(PolicyError::RetirementMovedBackward)
        } else {
            Self::new(self.open_epoch, through)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterExhausted {
    HeadRevision,
    DecisionSequence,
    DataRevision,
}

/// A value-only calculation, not permission to publish or mutate LMDB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryPosition {
    pub head: HeadRevision,
    pub decision: DecisionStamp,
    pub state: StateStamp,
}

impl HistoryPosition {
    /// # Errors
    /// Refuses exhausted coordinates without mutating any prior value.
    pub fn decided(
        self,
        hash: DecisionDigest,
        facts_changed: bool,
    ) -> Result<Self, CounterExhausted> {
        let head = self.maintained()?.head;
        let seq = self
            .decision
            .seq
            .checked_add(1)
            .ok_or(CounterExhausted::DecisionSequence)?;
        let data_revision = if facts_changed {
            self.state
                .data_revision
                .checked_add(1)
                .ok_or(CounterExhausted::DataRevision)?
        } else {
            self.state.data_revision
        };
        Ok(Self {
            head,
            decision: DecisionStamp { seq, hash },
            state: StateStamp {
                data_revision,
                ..self.state
            },
        })
    }

    /// # Errors
    /// Refuses head exhaustion; maintenance changes neither other stamp.
    pub fn maintained(self) -> Result<Self, CounterExhausted> {
        let revision = self
            .head
            .0
            .checked_add(1)
            .ok_or(CounterExhausted::HeadRevision)?;
        Ok(Self {
            head: HeadRevision(revision),
            ..self
        })
    }
}
