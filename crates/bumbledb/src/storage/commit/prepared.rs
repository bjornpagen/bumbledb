//! Narrow private write capability: application judgment is final, opaque
//! wrapper records can be sealed, then the only choices are commit or drop.

use crate::error::Result;
use crate::storage::env::host::{HostChanges, HostSealError};
use crate::storage::env::{GenerationId, WriteTxn};
use crate::work::WorkContext;

use super::CommitReport;

/// Net application facts, independent of metadata-only core generation changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationChanges {
    pub added: u64,
    pub removed: u64,
}

pub(crate) struct PreparedCommit<'env> {
    txn: WriteTxn<'env>,
    report: CommitReport,
    application_changes: ApplicationChanges,
}

pub(crate) struct SealedCommit<'env> {
    txn: WriteTxn<'env>,
    report: CommitReport,
}

impl<'env> PreparedCommit<'env> {
    pub(super) fn changed(
        txn: WriteTxn<'env>,
        new_generation: GenerationId,
        application_changes: ApplicationChanges,
    ) -> Self {
        Self {
            txn,
            report: CommitReport::Changed { new_generation },
            application_changes,
        }
    }

    /// Metadata-only/no-change/rejection receipt transaction. A rejected
    /// candidate must already be dropped while the external writer guard is
    /// still held; this does not itself provide that writer-session guard.
    pub(crate) fn unchanged(txn: WriteTxn<'env>) -> Result<Self> {
        let generation = txn.generation()?;
        Ok(Self {
            txn,
            report: CommitReport::Noop { generation },
            application_changes: ApplicationChanges {
                added: 0,
                removed: 0,
            },
        })
    }

    /// Seal only opaque host records. Failure drops the entire private txn,
    /// including application facts and any already-written host-record prefix.
    pub(crate) fn seal(
        mut self,
        changes: HostChanges<'_>,
        work: &WorkContext,
    ) -> std::result::Result<SealedCommit<'env>, HostSealError> {
        let host_changed = self.txn.apply_host_changes(changes, work)?;
        if host_changed && let CommitReport::Noop { generation } = self.report {
            let next = generation
                .value()
                .checked_add(1)
                .ok_or(HostSealError::GenerationExhausted)?;
            let new_generation = GenerationId::from_storage(next);
            self.txn.put_generation(new_generation)?;
            self.report = CommitReport::Changed { new_generation };
        }
        Ok(self.without_host_changes())
    }

    /// The existing callback commit owns no host adjunct or work context.
    pub(super) fn without_host_changes(self) -> SealedCommit<'env> {
        SealedCommit {
            txn: self.txn,
            report: self.report,
        }
    }

    pub(crate) fn report(&self) -> CommitReport {
        self.report
    }

    pub(crate) fn application_changes(&self) -> ApplicationChanges {
        self.application_changes
    }
}

impl SealedCommit<'_> {
    /// One LMDB durability point for application facts, generation, host rows,
    /// and attachment. No application or host writes are exposed after seal.
    pub(crate) fn commit(self) -> Result<CommitReport> {
        let _span = crate::obs::span(crate::obs::names::LMDB_COMMIT);
        self.txn.commit()?;
        Ok(self.report)
    }
}
