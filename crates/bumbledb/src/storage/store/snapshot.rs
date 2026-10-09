//! One coherent owned snapshot: rows, generation, host records and the head
//! all come from one LMDB read transaction, and export streams from it.
//! Concurrent commits are invisible; a pinned snapshot stays its
//! generation. `Send`, not `Sync`: the transaction moves between workers
//! whole but is used from one at a time.

use std::sync::Arc;

use bumbledb_theory::schema::RelationId;
use heed::{RoTxn, WithoutTls};

use super::error::{StoreError, StoreResult};
use super::format::{K_HEAD, K_HOST, RelationVersion, RowId, StoreIdentity};
use super::gate::GatePass;
use super::keys::HOST_KEY_MAX;
use super::rows;
use super::store_env::StoreInner;
use crate::schema::{CompiledProjection, ProjectionId};
use crate::storage::GenerationId;
use crate::work::WorkContext;

/// A visitor of host records: key, then value.
type HostVisit<'v, E> = dyn FnMut(&[u8], &[u8]) -> Result<(), E> + 'v;

pub(crate) struct OwnedSnapshot {
    // Field order is drop order: the transaction aborts before its gate
    // pass releases, and both go before the environment owner.
    txn: RoTxn<'static, WithoutTls>,
    _pass: GatePass,
    inner: Arc<StoreInner>,
    generation: GenerationId,
}

/// A projection resolved against its pinned store.
pub(crate) struct SnapshotProjection<'snapshot> {
    snapshot: &'snapshot OwnedSnapshot,
    compiled: &'snapshot CompiledProjection,
}

impl<'snapshot> SnapshotProjection<'snapshot> {
    pub(crate) fn compiled(&self) -> &'snapshot CompiledProjection {
        self.compiled
    }

    pub(crate) fn count_bounded(
        &self,
        projected: &[u8],
        limit: u64,
        work: &WorkContext,
    ) -> StoreResult<Option<u64>> {
        let snapshot = self.snapshot;
        let routing = rows::routing(&snapshot.inner, self.compiled, projected)?;
        rows::count_bucket_bounded(
            &snapshot.inner,
            &snapshot.txn,
            self.compiled,
            &routing,
            limit,
            work,
        )
    }

    /// Visit the group's candidates; `false` stops.
    pub(crate) fn probe(
        &self,
        projected: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(RowId, &'snapshot [u8]) -> StoreResult<bool>,
    ) -> StoreResult<()> {
        let snapshot = self.snapshot;
        let routing = rows::routing(&snapshot.inner, self.compiled, projected)?;
        rows::visit_bucket(
            &snapshot.inner,
            &snapshot.txn,
            self.compiled,
            &routing,
            work,
            visit,
        )
    }
}

impl std::fmt::Debug for OwnedSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedSnapshot")
            .field("database", &self.inner.identity.database)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl OwnedSnapshot {
    pub(crate) fn capture(
        inner: Arc<StoreInner>,
        pass: GatePass,
        txn: RoTxn<'static, WithoutTls>,
        generation: GenerationId,
    ) -> Self {
        Self {
            txn,
            _pass: pass,
            inner,
            generation,
        }
    }

    pub(crate) fn generation(&self) -> GenerationId {
        self.generation
    }

    pub(crate) fn relation_version(&self, relation: RelationId) -> StoreResult<RelationVersion> {
        Ok(super::format::read_relation_meta(&self.inner.meta, &self.txn, relation)?.version)
    }

    pub(crate) fn row_count(&self, relation: RelationId) -> StoreResult<u64> {
        Ok(super::format::read_relation_meta(&self.inner.meta, &self.txn, relation)?.count)
    }

    pub(crate) fn identity(&self) -> StoreIdentity {
        self.inner.identity
    }

    pub(crate) fn read_txn(&self) -> &RoTxn<'static, WithoutTls> {
        &self.txn
    }

    pub(crate) fn store_inner(&self) -> &StoreInner {
        &self.inner
    }

    pub(crate) fn head(&self) -> StoreResult<Option<&[u8]>> {
        self.inner
            .meta
            .get(&self.txn, K_HEAD)
            .map_err(StoreError::from_heed)
    }

    pub(crate) fn host_record(&self, key: &[u8]) -> StoreResult<Option<&[u8]>> {
        let mut buffer = [0u8; 1 + HOST_KEY_MAX];
        let len = super::host::host_key(key, &mut buffer)?;
        self.inner
            .meta
            .get(&self.txn, &buffer[..len])
            .map_err(StoreError::from_heed)
    }

    /// Every host record under `prefix`, in key order; the visitor borrows
    /// each key and value for one call.
    pub(crate) fn host_scan<E: From<StoreError>>(
        &self,
        prefix: &[u8],
        work: &WorkContext,
        visit: &mut HostVisit<'_, E>,
    ) -> Result<(), E> {
        let mut buffer = [0u8; 1 + HOST_KEY_MAX];
        let len = super::host::host_key(prefix, &mut buffer)?;
        let range = self
            .inner
            .meta
            .prefix_iter(&self.txn, &buffer[..len])
            .map_err(StoreError::from_heed)?;
        for entry in range {
            work.checkpoint().map_err(StoreError::Work)?;
            let (key, value) = entry.map_err(StoreError::from_heed)?;
            debug_assert_eq!(key.first(), Some(&K_HOST));
            visit(&key[1..], value)?;
        }
        Ok(())
    }

    /// One relation's rows in key order; values borrow this snapshot.
    pub(crate) fn rows(
        &self,
        relation: RelationId,
    ) -> StoreResult<impl Iterator<Item = StoreResult<(RowId, &[u8])>>> {
        Ok(rows::scan(&self.inner, &self.txn, relation)?
            .map(|entry| entry.map(|(key, bytes)| (key.row, bytes))))
    }

    pub(crate) fn row_bytes(
        &self,
        relation: RelationId,
    ) -> StoreResult<impl Iterator<Item = StoreResult<&[u8]>>> {
        Ok(
            rows::scan(&self.inner, &self.txn, relation)?
                .map(|entry| entry.map(|(_, bytes)| bytes)),
        )
    }

    pub(crate) fn contains(
        &self,
        relation: RelationId,
        row: &[u8],
        work: &WorkContext,
    ) -> StoreResult<bool> {
        rows::contains(&self.inner, &self.txn, relation, row, work)
    }

    /// Visit one projection group's candidate rows; values borrow this
    /// snapshot and may be retained after the visit.
    pub(crate) fn visit_projection<'snapshot>(
        &'snapshot self,
        projection: ProjectionId,
        projected: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(RowId, &'snapshot [u8]) -> StoreResult<bool>,
    ) -> StoreResult<()> {
        let Some(projection) = self.projection(projection) else {
            return Ok(());
        };
        projection.probe(projected, work, visit)
    }

    pub(crate) fn projection(&self, projection: ProjectionId) -> Option<SnapshotProjection<'_>> {
        self.inner
            .det
            .projection(projection)
            .map(|compiled| SnapshotProjection {
                snapshot: self,
                compiled,
            })
    }

    pub(crate) fn determinants(&self) -> &super::det_index::DeterminantTable {
        &self.inner.det
    }

    /// The canonical export: relations in declaration order, rows by home,
    /// rows of one home by canonical bytes. Ordinals never enter it, so it
    /// is a function of the content alone.
    pub(crate) fn export(
        &self,
        work: &WorkContext,
        sink: &mut dyn FnMut(RelationId, &[u8]) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let mut bucket: Vec<&[u8]> = Vec::new();
        for relation in self.inner.det.relations() {
            let mut home = None;
            for entry in rows::scan(&self.inner, &self.txn, relation)? {
                work.checkpoint()?;
                let (key, bytes) = entry?;
                if home != Some(key.route) {
                    flush(relation, &mut bucket, sink)?;
                    home = Some(key.route);
                }
                bucket.push(bytes);
            }
            flush(relation, &mut bucket, sink)?;
        }
        Ok(())
    }

    /// BLAKE3 over the canonical export, each row framed by its relation and
    /// length: equal digests mean equal content on any platform.
    pub(crate) fn content_digest(&self, work: &WorkContext) -> StoreResult<[u8; 32]> {
        let mut digest = crate::digest::Digest::new();
        self.export(work, &mut |relation, row| {
            digest.update(&relation.0.to_be_bytes());
            digest.update(&(row.len() as u64).to_be_bytes());
            digest.update(row);
            Ok(())
        })?;
        Ok(digest.finalize())
    }
}

fn flush(
    relation: RelationId,
    bucket: &mut Vec<&[u8]>,
    sink: &mut dyn FnMut(RelationId, &[u8]) -> StoreResult<()>,
) -> StoreResult<()> {
    bucket.sort_unstable();
    for row in bucket.drain(..) {
        sink(relation, row)?;
    }
    Ok(())
}
