//! The query lane's one storage seam (C05 consuming C04): every prepare,
//! statistic, image build, key probe, cursor fallback and result copy reads
//! committed rows through this enum — an owned coherent LMDB snapshot
//! ([`OwnedSnapshot`], one real read transaction) or an admitted heap
//! instance's sorted canonical rows. Closed relations never reach a source:
//! they synthesize from the schema's sealed extension.
//!
//! Identity discipline: a prepared query pins its source identity at
//! prepare. Store sources carry the store+environment identity
//! ([`StoreIdentity`]); executing against any other environment's snapshot
//! is `Error::ForeignPreparedQuery` before any work. Heap instances carry
//! no durable identity, so a heap-prepared query never memoizes images
//! across executions (the `ViewEpoch::Heap` tick) — correctness never
//! rides on an address comparison.

use crate::error::{Error, Result};
use crate::image::ViewEpoch;
use crate::storage::store::{OwnedSnapshot, StoreError, StoreIdentity};
use crate::work::{ExecutionPolicy, WorkContext, WorkError};
use bumbledb_theory::schema::RelationId;

/// The embedded-process allowance (mirrors `api::db::embedded_work`): the
/// host process is the budget authority for direct Rust embedding. Zero is
/// never "unlimited" — these are explicit maxima.
pub(crate) const UNBOUNDED_POLICY: ExecutionPolicy = ExecutionPolicy {
    input_bytes: u64::MAX,
    working_bytes: u64::MAX,
    scratch_bytes: u64::MAX,
    result_bytes: u64::MAX,
    rows: u64::MAX,
    work_units: u64::MAX,
    timeout: std::time::Duration::from_hours(24 * 365),
};

/// # Errors
/// Only an invalid timeout, which the constant policy cannot produce in
/// practice; kept fallible so callers stay in the one ledger constructor.
pub(crate) fn unbounded_work() -> std::result::Result<WorkContext, WorkError> {
    UNBOUNDED_POLICY.start()
}

pub(crate) fn work_error(error: WorkError) -> Error {
    Error::from_store(StoreError::Work(error))
}

pub(crate) fn store_error(error: StoreError) -> Error {
    Error::from_store(error)
}

/// Heap row access, type-erased over the instance's schema typestate.
/// Implemented for [`crate::api::db::OwnedInstance`] here (the query lane
/// owns its consumption; the instance's file is not edited).
pub(crate) trait HeapRows {
    /// Sorted canonical rows of one ordinary relation (empty for closed
    /// or unpopulated relations).
    fn rows(&self, relation: RelationId) -> &[Box<[u8]>];
}

impl<S> HeapRows for crate::api::db::OwnedInstance<S> {
    fn rows(&self, relation: RelationId) -> &[Box<[u8]>] {
        self.relation_rows(relation)
    }
}

/// What a prepared query pinned at prepare — checked at every execution
/// entry before any bind or read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PinnedSource {
    Store(StoreIdentity),
    /// Heap-prepared: no durable identity exists; every execution rebuilds
    /// its images from the instance it was handed (never a memo hit).
    Heap,
}

/// One execution's row source.
pub(crate) enum QuerySource<'a> {
    Store {
        snapshot: &'a OwnedSnapshot,
        work: &'a WorkContext,
    },
    Heap {
        rows: &'a dyn HeapRows,
        work: WorkContext,
        /// The prepared query's per-execution heap tick (epochs below).
        tick: u64,
    },
}

impl<'a> QuerySource<'a> {
    pub(crate) fn store(snapshot: &'a OwnedSnapshot, work: &'a WorkContext) -> Self {
        Self::Store { snapshot, work }
    }

    /// # Errors
    /// Ledger construction only (see [`unbounded_work`]).
    pub(crate) fn heap(rows: &'a dyn HeapRows, tick: u64) -> Result<Self> {
        Ok(Self::Heap {
            rows,
            work: unbounded_work().map_err(work_error)?,
            tick,
        })
    }

    pub(crate) fn work(&self) -> &WorkContext {
        match self {
            Self::Store { work, .. } => work,
            Self::Heap { work, .. } => work,
        }
    }

    pub(crate) fn pinned(&self) -> PinnedSource {
        match self {
            Self::Store { snapshot, .. } => PinnedSource::Store(snapshot.identity()),
            Self::Heap { .. } => PinnedSource::Heap,
        }
    }

    /// The view-validity epoch of one ordinary relation on this source.
    pub(crate) fn epoch(&self) -> ViewEpoch {
        match self {
            Self::Store { snapshot, .. } => ViewEpoch::Store(snapshot.generation()),
            Self::Heap { tick, .. } => ViewEpoch::Heap(*tick),
        }
    }

    /// Committed row count of one ordinary relation.
    /// # Errors
    /// Storage failure.
    pub(crate) fn row_count(&self, relation: RelationId) -> Result<u64> {
        match self {
            Self::Store { snapshot, .. } => snapshot.row_count(relation).map_err(store_error),
            Self::Heap { rows, .. } => Ok(rows.rows(relation).len() as u64),
        }
    }

    /// Walk one ordinary relation's canonical row bytes in source order,
    /// charging one work step per row.
    /// # Errors
    /// Storage failure, stopped work, or the sink's failure.
    pub(crate) fn scan(
        &self,
        relation: RelationId,
        sink: &mut dyn FnMut(&[u8]) -> Result<()>,
    ) -> Result<()> {
        let work = self.work();
        match self {
            Self::Store { snapshot, .. } => {
                let iterator = snapshot.rows(relation).map_err(store_error)?;
                for entry in iterator {
                    work.step(1).map_err(work_error)?;
                    let (_, bytes) = entry.map_err(store_error)?;
                    sink(bytes)?;
                }
                Ok(())
            }
            Self::Heap { rows, .. } => {
                for row in rows.rows(relation) {
                    work.step(1).map_err(work_error)?;
                    sink(row)?;
                }
                Ok(())
            }
        }
    }

    /// Exact membership of one canonical row: fingerprint bucket plus full
    /// canonical bytes on a store; binary search on a heap instance.
    /// # Errors
    /// Storage failure or stopped work.
    pub(crate) fn contains(&self, relation: RelationId, row: &[u8]) -> Result<bool> {
        match self {
            Self::Store { snapshot, work } => {
                snapshot.contains(relation, row, work).map_err(store_error)
            }
            Self::Heap { rows, work, .. } => {
                work.step(1).map_err(work_error)?;
                Ok(rows
                    .rows(relation)
                    .binary_search_by(|candidate| candidate.as_ref().cmp(row))
                    .is_ok())
            }
        }
    }
}

/// The one condition licensing the bounded resident→fallback restart: the
/// working-byte ledger refused a reservation (image slabs, decoded
/// batches). Other failures — semantic errors, cancellation, deadlines,
/// storage faults — are never retried into a different path.
pub(crate) fn is_working_exhaustion(error: &Error) -> bool {
    matches!(
        error,
        Error::Store(store) if matches!(
            **store,
            StoreError::Work(WorkError::Exhausted {
                resource: crate::work::Resource::WorkingBytes,
                ..
            })
        )
    )
}
