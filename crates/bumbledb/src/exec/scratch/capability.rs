//! Explicit scratch capability: one execution-owned transient substrate
//! passed to admission, grouping, derived stages and results. An error's
//! Rust type is never a capability detector (chapter 61).

use crate::work::{ExecutionPolicy, Resource, WorkContext, WorkError};

use super::{ScratchRelation, DEFAULT_RAM_BYTES};

/// The bounded scratch policy carried beside an operation ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScratchPolicy {
    pub scratch_bytes: u64,
    pub ram_bytes_per_relation: usize,
}

impl ScratchPolicy {
    #[must_use]
    pub const fn from_execution(policy: ExecutionPolicy) -> Self {
        Self {
            scratch_bytes: policy.scratch_bytes,
            ram_bytes_per_relation: DEFAULT_RAM_BYTES,
        }
    }

    /// Policy from the live execute ledger. Does not start a new deadline.
    #[must_use]
    pub fn from_work(work: &WorkContext) -> Self {
        Self {
            scratch_bytes: work.limit(Resource::ScratchBytes),
            ram_bytes_per_relation: DEFAULT_RAM_BYTES,
        }
    }

    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            scratch_bytes: u64::MAX,
            ram_bytes_per_relation: DEFAULT_RAM_BYTES,
        }
    }

    /// # Errors
    /// Refuses a policy whose scratch allowance exceeds the operation ledger.
    pub fn enforce(&self, work: &WorkContext) -> Result<(), WorkError> {
        if self.scratch_bytes > work.limit(Resource::ScratchBytes) {
            return Err(WorkError::Exhausted {
                resource: Resource::ScratchBytes,
                used: work.used(Resource::ScratchBytes),
                requested: self.scratch_bytes,
                limit: work.limit(Resource::ScratchBytes),
            });
        }
        Ok(())
    }

    /// Enforce the declared disk policy before a retained-byte growth.
    /// # Errors
    /// Refuses when the live scratch charge plus `additional` exceeds the
    /// policy or the operation ledger.
    pub fn allow_growth(&self, work: &WorkContext, additional: u64) -> Result<(), WorkError> {
        self.enforce(work)?;
        let used = work.used(Resource::ScratchBytes);
        let next = used.checked_add(additional).ok_or(WorkError::Exhausted {
            resource: Resource::ScratchBytes,
            used,
            requested: additional,
            limit: self.scratch_bytes.min(work.limit(Resource::ScratchBytes)),
        })?;
        let limit = self.scratch_bytes.min(work.limit(Resource::ScratchBytes));
        if next > limit {
            return Err(WorkError::Exhausted {
                resource: Resource::ScratchBytes,
                used,
                requested: additional,
                limit,
            });
        }
        Ok(())
    }
}

/// Explicit scratch capability for one operation attempt.
#[derive(Debug, Clone)]
pub struct ScratchCapability {
    work: WorkContext,
    policy: ScratchPolicy,
}

impl ScratchCapability {
    /// # Errors
    /// [`WorkError::InvalidTimeout`] for an unrepresentable deadline, or a
    /// scratch policy that exceeds the operation ledger.
    pub fn start(
        execution: ExecutionPolicy,
        scratch: ScratchPolicy,
    ) -> Result<Self, WorkError> {
        let work = execution.start()?;
        scratch.enforce(&work)?;
        Ok(Self { work, policy: scratch })
    }

    /// Bind scratch to the **execute** ledger. Clones the context (shared
    /// counters and deadline); does not reconstruct a twin policy/timeout.
    /// # Errors
    /// A scratch policy that exceeds the live ledger.
    pub fn on_work(work: &WorkContext, scratch: ScratchPolicy) -> Result<Self, WorkError> {
        scratch.enforce(work)?;
        Ok(Self {
            work: work.clone(),
            policy: scratch,
        })
    }

    #[must_use]
    pub fn work(&self) -> &WorkContext {
        &self.work
    }

    #[must_use]
    pub const fn policy(&self) -> ScratchPolicy {
        self.policy
    }

    /// Open one exact transient relation under this capability. Named maps
    /// (`ScratchMapId`) share this relation's environment.
    #[must_use]
    pub fn relation(&self) -> ScratchRelation {
        ScratchRelation::with_policy(&self.work, self.policy)
    }

    /// Open a relation with a caller-selected RAM crossover.
    #[must_use]
    pub fn relation_with_ram(&self, ram_limit: usize) -> ScratchRelation {
        ScratchRelation::with_policy_and_ram(&self.work, self.policy, ram_limit)
    }
}
