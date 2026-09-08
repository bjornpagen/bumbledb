//! One coherent owned snapshot (ENG-003 by construction).
//!
//! An [`OwnedSnapshot`] owns one real LMDB read transaction plus a clone of
//! the environment; rows, generation, host records and the attachment are
//! all read from that single transaction, and export streams from it —
//! never a second view opened midway. Concurrent commits are invisible; a
//! pinned old snapshot stays exactly its generation.
//!
//! `Send` and `!Sync` (the transaction moves between workers whole, but is
//! used from one at a time):
//!
//! ```compile_fail
//! fn require_sync<T: Sync>() {}
//! require_sync::<bumbledb::store::OwnedSnapshot>();
//! ```
//!
//! A held snapshot blocks map growth by design; the gate reports its count
//! and age instead of invalidating a live Rust borrow. Projection visits
//! take [`crate::schema::ProjectionId`], never a statement id.
//!
//! ```compile_fail
//! fn require_statement_visit(snap: &bumbledb::store::OwnedSnapshot) {
//!     let _ = snap.visit_statement;
//! }
//! ```

use std::ops::Bound;
use std::sync::Arc;
use std::time::Duration;

use bumbledb_theory::schema::RelationId;
use heed::{RoTxn, WithoutTls};

use super::error::{HostKeyFault, StoreCorruption, StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use super::format::{
    CoreStoreId, EnvironmentId, K_ATTACHMENT, K_HOST_RECORD_TAG, RowId, RowLocator, StoreIdentity,
};
use super::gate::{CachedRead, GatePass, ReadLease};
use super::host::{HostResume, HostWindow};
use super::keys::{self, HOST_KEY_MAX};
use super::rows;
use super::store_env::StoreInner;
use crate::storage::GenerationId;
use crate::work::WorkContext;

/// Everything a coherent logical export names, from the one transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportReport {
    pub store: CoreStoreId,
    pub environment: EnvironmentId,
    pub generation: GenerationId,
    pub rows: u64,
}

/// Page-level statistics of the store's LMDB trees, read through one
/// snapshot transaction (SPACE-01 seam, requested by P14). Counts sum the
/// three B-trees (the unnamed database directory, `_core_meta` and
/// `_core_data`); `depth` is the maximum tree height. Mixed namespaces
/// share pages, so no fictional per-namespace page attribution exists —
/// pair these with [`OwnedSnapshot::entry_census`] for live-byte numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorePageStats {
    /// LMDB page size for this environment.
    pub page_size: u64,
    /// Maximum B-tree depth across the three trees.
    pub depth: u64,
    /// Internal (non-leaf) pages, summed.
    pub branch_pages: u64,
    /// Leaf pages, summed.
    pub leaf_pages: u64,
    /// Overflow pages, summed.
    pub overflow_pages: u64,
    /// Data items, summed.
    pub entries: u64,
    /// Pages of the populated file used by no live tree and not one of the
    /// two LMDB meta pages — the freelist, derived from the last used page
    /// number (this binding exposes no direct freelist count).
    pub free_pages: u64,
}

pub struct OwnedSnapshot {
    // The complete lease, including its gate Arc, drops before inner. A
    // parked txn then lives only in inner's gate, which drops before its
    // directory lock; the environment can never outlive that lock.
    txn: ReadLease,
    inner: Arc<StoreInner>,
    generation: GenerationId,
}

/// A projection resolved against its exact pinned store. Keeping both
/// borrows together prevents callers from pairing another schema's
/// descriptor with this snapshot, without retaining another owner.
pub(crate) struct SnapshotProjection<'snapshot> {
    snapshot: &'snapshot OwnedSnapshot,
    compiled: &'snapshot crate::schema::CompiledProjection,
}

impl<'snapshot> SnapshotProjection<'snapshot> {
    pub(crate) fn compiled(&self) -> &'snapshot crate::schema::CompiledProjection {
        self.compiled
    }

    pub(crate) fn count_bounded(
        &self,
        projected: &[u8],
        limit: u64,
        work: &WorkContext,
    ) -> StoreResult<Option<u64>> {
        let snapshot = self.snapshot;
        let routing = rows::routing_for_compiled(&snapshot.inner, self.compiled, projected);
        rows::count_determinant_bucket_bounded(
            &snapshot.inner,
            &snapshot.txn,
            self.compiled,
            &routing,
            limit,
            work,
        )
    }

    /// Internal first-match probe. Generic projection visitation and
    /// judgment deliberately keep their ordinary streaming cursor.
    pub(crate) fn probe(
        &self,
        projected: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(RowId, &'snapshot [u8]) -> StoreResult<bool>,
    ) -> StoreResult<()> {
        let snapshot = self.snapshot;
        let routing = rows::routing_for_compiled(&snapshot.inner, self.compiled, projected);
        rows::probe_determinant_bucket(
            &snapshot.inner,
            &snapshot.txn,
            self.compiled,
            &routing,
            work,
            &mut |locator, bytes| visit(locator.id, bytes),
        )
    }

    pub(crate) fn visit(
        &self,
        projected: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(RowId, &'snapshot [u8]) -> StoreResult<bool>,
    ) -> StoreResult<()> {
        let snapshot = self.snapshot;
        let routing = rows::routing_for_compiled(&snapshot.inner, self.compiled, projected);
        rows::visit_determinant_bucket(
            &snapshot.inner,
            &snapshot.txn,
            self.compiled,
            &routing,
            work,
            &mut |locator, bytes| visit(locator.id, bytes),
        )
    }
}

impl std::fmt::Debug for OwnedSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OwnedSnapshot")
            .field("store", &self.inner.identity.store)
            .field("generation", &self.generation)
            .field("age", &self.age())
            .finish_non_exhaustive()
    }
}

impl OwnedSnapshot {
    pub(crate) fn capture(inner: Arc<StoreInner>, pass: GatePass, reader: CachedRead) -> Self {
        let generation = reader.generation;
        Self {
            txn: ReadLease::new(reader, pass),
            inner,
            generation,
        }
    }

    /// The generation this snapshot witnessed — read from this transaction
    /// at capture, immutable afterwards.
    #[must_use]
    pub fn generation(&self) -> GenerationId {
        self.generation
    }

    /// One relation's committed change version at this snapshot — one small
    /// meta read from this snapshot's own transaction (bounded and cheap; a
    /// pinned old snapshot keeps reading its own versions). The version
    /// advances exactly when a committed transaction changed that relation's
    /// rows, so equal versions across two snapshots of one environment prove
    /// the relation's rows are identical — the safe image-reuse key
    /// (PERF-001). Host-record/attachment-only seals advance the generation
    /// but no relation version.
    /// # Errors
    /// Storage failure or a malformed stored version word.
    pub fn relation_version(
        &self,
        relation: RelationId,
    ) -> StoreResult<super::format::RelationVersion> {
        super::format::read_relation_version(&self.inner.meta, &self.txn, relation)
    }

    #[must_use]
    pub fn identity(&self) -> StoreIdentity {
        self.inner.identity
    }

    /// Time since this borrow was admitted to the transaction gate,
    /// including native transaction setup. This is the same growth-blocking
    /// admission tracked by gate diagnostics, not the age of a reused reader.
    #[must_use]
    pub fn age(&self) -> Duration {
        self.txn.age()
    }

    /// The exact pinned transaction used by store copy and verification.
    pub(crate) fn read_txn(&self) -> &RoTxn<'static, WithoutTls> {
        &self.txn
    }

    /// The owning store state, for in-module raw walks (copy, verify).
    pub(crate) fn store_inner(&self) -> &StoreInner {
        &self.inner
    }

    /// The schema fingerprint the snapshot's store was opened with.
    #[must_use]
    pub fn schema_fingerprint(&self) -> crate::schema::fingerprint::SchemaFingerprint {
        self.inner.schema_fp
    }

    /// The opaque host attachment from this exact transaction. Borrowed
    /// from the snapshot; adapters budget and copy before releasing it.
    /// # Errors
    /// Storage failure.
    pub fn attachment(&self) -> StoreResult<Option<&[u8]>> {
        self.inner
            .meta
            .get(&self.txn, K_ATTACHMENT)
            .map_err(StoreError::from_heed)
    }

    /// One opaque host record from this exact transaction.
    /// # Errors
    /// Host-key grammar or storage failure.
    pub fn host_record(&self, key: &[u8]) -> StoreResult<Option<&[u8]>> {
        if key.len() > HOST_KEY_MAX {
            return Err(StoreError::HostKey(HostKeyFault::TooLong {
                actual: key.len(),
            }));
        }
        let mut buffer = [0u8; 1 + HOST_KEY_MAX];
        buffer[0] = K_HOST_RECORD_TAG;
        buffer[1..=key.len()].copy_from_slice(key);
        self.inner
            .meta
            .get(&self.txn, &buffer[..=key.len()])
            .map_err(StoreError::from_heed)
    }

    /// Visit every committed host record whose key starts with `prefix`,
    /// in ascending key order, from this exact transaction.
    /// Already streams: logical host keys — the storage tag never escapes —
    /// and values borrow the snapshot's mapped pages for the duration of
    /// one visit only; cancellation is checked per record. No resume and no
    /// byte cap — do not accumulate every key. Use [`Self::host_scan_batch`]
    /// for bounded windows.
    /// # Errors
    /// Host-key grammar or storage failure, stopped work, or the
    /// visitor's own refusal.
    #[expect(
        clippy::type_complexity,
        reason = "borrowed key/value visitor with the caller's integration error"
    )]
    pub fn host_scan<E: From<StoreError>>(
        &self,
        prefix: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(&[u8], &[u8]) -> Result<(), E>,
    ) -> Result<(), E> {
        if prefix.len() > HOST_KEY_MAX {
            return Err(E::from(StoreError::HostKey(HostKeyFault::TooLong {
                actual: prefix.len(),
            })));
        }
        let mut buffer = [0u8; 1 + HOST_KEY_MAX];
        buffer[0] = K_HOST_RECORD_TAG;
        buffer[1..=prefix.len()].copy_from_slice(prefix);
        let range = self
            .inner
            .meta
            .prefix_iter(&self.txn, &buffer[..=prefix.len()])
            .map_err(|error| E::from(StoreError::from_heed(error)))?;
        for entry in range {
            work.checkpoint()
                .map_err(|error| E::from(StoreError::Work(error)))?;
            let (key, value) = entry.map_err(|error| E::from(StoreError::from_heed(error)))?;
            visit(&key[1..], value)?;
        }
        Ok(())
    }

    /// One bounded host window under `prefix`, exclusive after `after`.
    /// Streams visits — peak is the visitor plus one resume key, never
    /// every matching record. Stops when `key+value` bytes would exceed
    /// `byte_cap` after at least one record, or the prefix is exhausted.
    /// Full-prefix [`Self::host_scan`] already streams (one work step per
    /// record) but has no resume or byte cap; do not accumulate its keys.
    ///
    /// # Errors
    /// Host-key grammar or storage failure, stopped work, or the visitor.
    #[expect(
        clippy::type_complexity,
        reason = "windowed twin of host_scan's visitor"
    )]
    pub fn host_scan_batch<E: From<StoreError>>(
        &self,
        prefix: &[u8],
        after: Option<&[u8]>,
        work: &WorkContext,
        byte_cap: u64,
        visit: &mut dyn FnMut(&[u8], &[u8]) -> Result<(), E>,
    ) -> Result<HostWindow, E> {
        if prefix.len() > HOST_KEY_MAX {
            return Err(E::from(StoreError::HostKey(HostKeyFault::TooLong {
                actual: prefix.len(),
            })));
        }
        if let Some(after) = after
            && after.len() > HOST_KEY_MAX
        {
            return Err(E::from(StoreError::HostKey(HostKeyFault::TooLong {
                actual: after.len(),
            })));
        }
        let mut prefix_buf = [0u8; 1 + HOST_KEY_MAX];
        prefix_buf[0] = K_HOST_RECORD_TAG;
        prefix_buf[1..=prefix.len()].copy_from_slice(prefix);
        let mut after_buf = [0u8; 1 + HOST_KEY_MAX];
        let start = if let Some(after) = after {
            after_buf[0] = K_HOST_RECORD_TAG;
            after_buf[1..=after.len()].copy_from_slice(after);
            Bound::Excluded(&after_buf[..=after.len()])
        } else {
            Bound::Included(&prefix_buf[..=prefix.len()])
        };
        let bounds = (start, Bound::Unbounded);
        let range = self
            .inner
            .meta
            .range(&self.txn, &bounds)
            .map_err(|error| E::from(StoreError::from_heed(error)))?;
        let mut records = 0u64;
        let mut bytes = 0u64;
        let mut last: Option<HostResume> = None;
        for entry in range {
            work.checkpoint()
                .map_err(|error| E::from(StoreError::Work(error)))?;
            let (key, value) = entry.map_err(|error| E::from(StoreError::from_heed(error)))?;
            if key.first() != Some(&K_HOST_RECORD_TAG) {
                break;
            }
            let logical = &key[1..];
            if !logical.starts_with(prefix) {
                break;
            }
            let record_bytes = (logical.len() + value.len()) as u64;
            if records > 0 && bytes.saturating_add(record_bytes) > byte_cap {
                return Ok(match last {
                    Some(resume) => HostWindow::More {
                        resume,
                        records,
                        bytes,
                    },
                    None => HostWindow::Done { records, bytes },
                });
            }
            work.checkpoint()
                .map_err(|error| E::from(StoreError::Work(error)))?;
            visit(logical, value)?;
            last = Some(HostResume::from_key(logical).map_err(E::from)?);
            bytes = bytes.saturating_add(record_bytes);
            records += 1;
        }
        Ok(HostWindow::Done { records, bytes })
    }

    /// Cursor over one relation's committed rows in physical key order.
    /// Locators retain stable ordinals independently of traversal order.
    /// Values borrow this snapshot's mapped pages; they are valid
    /// exactly as long as the snapshot.
    /// # Errors
    /// Storage failure.
    pub fn rows(
        &self,
        relation: RelationId,
    ) -> StoreResult<impl Iterator<Item = StoreResult<(RowLocator, &[u8])>>> {
        rows::scan_rows(&self.inner, &self.txn, relation)
    }

    /// Same validated scan and pinned row lifetime as `rows`, without
    /// constructing physical locators that the query image would discard.
    pub(crate) fn row_bytes(
        &self,
        relation: RelationId,
    ) -> StoreResult<impl Iterator<Item = StoreResult<&[u8]>>> {
        rows::scan_row_bytes(&self.inner, &self.txn, relation)
    }

    /// Exact membership: selected scalar-home or fingerprint bucket,
    /// followed by full canonical byte confirmation.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn contains(
        &self,
        relation: RelationId,
        row: &[u8],
        work: &WorkContext,
    ) -> StoreResult<bool> {
        Ok(rows::exact_lookup(&self.inner, &self.txn, relation, row, work)?.is_some())
    }

    /// Fetch one row's canonical bytes by its physical locator.
    /// # Errors
    /// Storage failure.
    pub fn fetch(&self, relation: RelationId, row: RowLocator) -> StoreResult<Option<&[u8]>> {
        rows::fetch_row(&self.inner, &self.txn, relation, row)
    }

    /// Bounded visitor over one interned projection's determinant bucket.
    /// Named by [`crate::schema::ProjectionId`], never `StatementId`. The
    /// visitor receives each `(row id, canonical row bytes)`; return
    /// `false` to stop early. Row bytes borrow this pinned snapshot, not
    /// the temporary index cursor, and may be retained after the visit.
    ///
    /// # Errors
    /// Storage failure, stopped work, or visitor failure.
    pub fn visit_projection<'snapshot>(
        &'snapshot self,
        projection: crate::schema::ProjectionId,
        projected: &[u8],
        work: &WorkContext,
        visit: &mut dyn FnMut(RowId, &'snapshot [u8]) -> StoreResult<bool>,
    ) -> StoreResult<()> {
        let Some(projection) = self.projection(projection) else {
            return Ok(());
        };
        projection.visit(projected, work, visit)
    }

    pub(crate) fn projection(
        &self,
        projection: crate::schema::ProjectionId,
    ) -> Option<SnapshotProjection<'_>> {
        self.inner
            .det
            .projection(projection)
            .map(|compiled| SnapshotProjection {
                snapshot: self,
                compiled,
            })
    }

    /// Enumerate one committed determinant bucket at this snapshot: every
    /// physical locator under the projection's exact or fingerprint routing.
    /// Prefer [`Self::visit_projection`] when walking rows.
    /// Candidates only — the caller confirms each row with exact decoded
    /// values (a forced collision widens this set, never an answer).
    /// `projected` follows the store's one projection convention.
    /// # Errors
    /// Storage failure or stopped work.
    pub fn determinant_candidates(
        &self,
        projection: crate::schema::ProjectionId,
        projected: &[u8],
        work: &WorkContext,
    ) -> StoreResult<Vec<RowLocator>> {
        rows::determinant_bucket_ids(&self.inner, &self.txn, projection, projected, work)
    }

    /// The store's compiled theory (projection table and law adjacency).
    #[cfg(test)]
    pub(crate) fn compiled(&self) -> &crate::schema::CompiledTheory {
        self.inner.det.theory()
    }

    /// The store's compiled determinant table (probe-side projection).
    pub(crate) fn determinants(&self) -> &super::det_index::DeterminantTable {
        &self.inner.det
    }

    /// Live row count of one relation at this snapshot.
    /// # Errors
    /// Storage failure.
    pub fn row_count(&self, relation: RelationId) -> StoreResult<u64> {
        rows::row_count(&self.inner, &self.txn, relation)
    }

    /// Exact key-family widths used by this snapshot's store codec.
    #[must_use]
    pub fn physical_key_widths(&self) -> super::PhysicalKeyWidths {
        self.inner.keys.widths()
    }

    /// Raw physical census (SPACE-01 seam for the bench walker, P14): every
    /// entry of the data and meta databases as (database, leading namespace
    /// tag, key bytes, value bytes) sizes, from this one coherent
    /// transaction. The data/meta tag spaces overlap, so the sink receives
    /// `is_meta` explicitly. Harness tier — never a hot path;
    /// interpretation (chapter 41's byte model) lives with the census owner.
    /// # Errors
    /// Storage failure or stopped work.
    #[doc(hidden)]
    pub fn entry_census(
        &self,
        work: &WorkContext,
        sink: &mut dyn FnMut(bool, u8, usize, usize) -> StoreResult<()>,
    ) -> StoreResult<()> {
        for (is_meta, range) in [
            (false, self.inner.data.iter(&self.txn)),
            (true, self.inner.meta.iter(&self.txn)),
        ] {
            let range = range.map_err(StoreError::from_heed)?;
            for entry in range {
                work.checkpoint()?;
                let (key, value) = entry.map_err(StoreError::from_heed)?;
                sink(
                    is_meta,
                    key.first().copied().unwrap_or(0),
                    key.len(),
                    value.len(),
                )?;
            }
        }
        Ok(())
    }

    /// Page statistics of the store's trees through this exact transaction
    /// (the other half of the SPACE-01 seam, with [`Self::entry_census`]).
    /// Harness tier — never a hot path.
    /// # Errors
    /// Storage failure.
    #[doc(hidden)]
    pub fn page_stats(&self) -> StoreResult<StorePageStats> {
        let main: heed::Database<heed::types::Bytes, heed::types::Bytes> = self
            .inner
            .env
            .open_database(&self.txn, None)
            .map_err(StoreError::from_heed)?
            .ok_or(StoreError::Corruption(StoreCorruption::MetaMissing(
                "main database",
            )))?;
        let mut stats = StorePageStats {
            page_size: 0,
            depth: 0,
            branch_pages: 0,
            leaf_pages: 0,
            overflow_pages: 0,
            entries: 0,
            free_pages: 0,
        };
        for stat in [
            main.stat(&self.txn).map_err(StoreError::from_heed)?,
            self.inner
                .meta
                .stat(&self.txn)
                .map_err(StoreError::from_heed)?,
            self.inner
                .data
                .stat(&self.txn)
                .map_err(StoreError::from_heed)?,
        ] {
            stats.page_size = u64::from(stat.page_size);
            stats.depth = stats.depth.max(u64::from(stat.depth));
            stats.branch_pages += stat.branch_pages as u64;
            stats.leaf_pages += stat.leaf_pages as u64;
            stats.overflow_pages += stat.overflow_pages as u64;
            stats.entries += stat.entries as u64;
        }
        // `last_page_number` is the id of the last used page; the two LMDB
        // meta pages and every live tree page are subtracted, the remainder
        // is reclaimable (the freelist and pages it references).
        let page_count = self.inner.env.info().last_page_number as u64 + 1;
        let used = 2 + stats.branch_pages + stats.leaf_pages + stats.overflow_pages;
        stats.free_pages = page_count.saturating_sub(used);
        Ok(stats)
    }

    /// Canonical logical export: relation order, then the schema-selected
    /// exact scalar key where available, otherwise the tuple fingerprint.
    /// Rows sharing a routing key are ordered by full canonical bytes.
    /// Physical row ids never enter the logical identity. This layout's
    /// export order is part of its logical digest contract. An adversarial
    /// collision bucket uses repeated bounded-memory minimum scans, never
    /// an unbounded in-memory list or a full-relation sort.
    ///
    /// Every emitted row, the returned generation, and the attachment all
    /// come from this snapshot's one transaction; the copy helper consumes
    /// no second source view.
    /// # Errors
    /// Storage failure, stopped work, or the sink's failure.
    pub fn export(
        &self,
        work: &WorkContext,
        sink: &mut dyn FnMut(RelationId, &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ExportReport> {
        let mut emitted = 0u64;
        for relation in self.inner.det.relations() {
            let mut prefix = [0; 1 + 4];
            let (prefix_len, routing_width) =
                if let Some(home) = self.inner.det.membership_projection(relation) {
                    let key = self.inner.keys.row_bucket(relation, &[])?;
                    prefix[..key.len()].copy_from_slice(&key);
                    (key.len(), home.encoding.routing_width())
                } else {
                    let key = self.inner.keys.membership_bucket(relation, &[0; FP_LEN])?;
                    let len = key.len() - FP_LEN;
                    prefix[..len].copy_from_slice(&key[..len]);
                    (len, FP_LEN)
                };
            emitted +=
                self.export_relation(relation, &prefix[..prefix_len], routing_width, work, sink)?;
        }
        Ok(ExportReport {
            store: self.inner.identity.store,
            environment: self.inner.identity.environment,
            generation: self.generation,
            rows: emitted,
        })
    }

    fn export_relation(
        &self,
        relation: RelationId,
        prefix: &[u8],
        routing_width: usize,
        work: &WorkContext,
        sink: &mut dyn FnMut(RelationId, &[u8]) -> StoreResult<()>,
    ) -> StoreResult<u64> {
        let mut emitted = 0;
        let mut lower = [0; 1 + 4 + FP_LEN + 8];
        let mut lower_len = prefix.len();
        lower[..lower_len].copy_from_slice(prefix);
        let mut included_lower = true;
        loop {
            work.checkpoint()?;
            let head = {
                let bounds: (Bound<&[u8]>, Bound<&[u8]>) = (
                    if included_lower {
                        Bound::Included(&lower[..lower_len])
                    } else {
                        Bound::Excluded(&lower[..lower_len])
                    },
                    Bound::Unbounded,
                );
                let mut range = self
                    .inner
                    .data
                    .range(&self.txn, &bounds)
                    .map_err(StoreError::from_heed)?;
                match range.next() {
                    None => None,
                    Some(entry) => {
                        let (key, _) = entry.map_err(StoreError::from_heed)?;
                        if key.starts_with(prefix) {
                            keys::row_id_from_suffix(key, prefix.len() + routing_width + 8)?;
                            Some(&key[..key.len() - 8])
                        } else {
                            None
                        }
                    }
                }
            };
            let Some(bucket) = head else {
                break;
            };
            emitted += self.export_bucket(relation, bucket, work, sink)?;
            lower_len = bucket.len() + 8;
            lower[..bucket.len()].copy_from_slice(bucket);
            lower[bucket.len()..lower_len].copy_from_slice(&u64::MAX.to_be_bytes());
            included_lower = false;
        }
        Ok(emitted)
    }

    fn export_bucket(
        &self,
        relation: RelationId,
        bucket: &[u8],
        work: &WorkContext,
        sink: &mut dyn FnMut(RelationId, &[u8]) -> StoreResult<()>,
    ) -> StoreResult<u64> {
        // Count first: the common bucket has one row and no ordering work.
        let mut count = 0u64;
        let mut first = None;
        {
            let range = self
                .inner
                .data
                .prefix_iter(&self.txn, bucket)
                .map_err(StoreError::from_heed)?;
            for entry in range {
                work.checkpoint()?;
                let (key, value) = entry.map_err(StoreError::from_heed)?;
                keys::row_id_from_suffix(key, bucket.len() + 8)?;
                if first.is_none() {
                    first = Some(self.export_row(relation, key, value)?);
                }
                count += 1;
            }
        }
        if count == 1 {
            let Some(row) = first else {
                return Err(StoreError::Corruption(StoreCorruption::DanglingIndexEntry));
            };
            sink(relation, row)?;
            return Ok(1);
        }
        // Collision bucket: repeated bounded-memory minimum scan. Holds one
        // borrowed row from this immutable snapshot, never the whole bucket.
        // LMDB row slices outlive each cursor because the read transaction
        // remains pinned throughout export.
        let mut last: Option<&[u8]> = None;
        for _ in 0..count {
            let mut best: Option<&[u8]> = None;
            {
                let range = self
                    .inner
                    .data
                    .prefix_iter(&self.txn, bucket)
                    .map_err(StoreError::from_heed)?;
                for entry in range {
                    work.checkpoint()?;
                    let (key, value) = entry.map_err(StoreError::from_heed)?;
                    keys::row_id_from_suffix(key, bucket.len() + 8)?;
                    let row = self.export_row(relation, key, value)?;
                    if let Some(emitted_bytes) = last
                        && rows::chunked_cmp(row, emitted_bytes, work)?
                            != std::cmp::Ordering::Greater
                    {
                        continue;
                    }
                    match &best {
                        Some(best_bytes)
                            if rows::chunked_cmp(row, best_bytes, work)?
                                != std::cmp::Ordering::Less => {}
                        _ => best = Some(row),
                    }
                }
            }
            let Some(row) = best else {
                return Err(StoreError::Corruption(StoreCorruption::DanglingIndexEntry));
            };
            sink(relation, row)?;
            last = Some(row);
        }
        Ok(count)
    }

    fn export_row<'snapshot>(
        &'snapshot self,
        relation: RelationId,
        key: &[u8],
        value: &'snapshot [u8],
    ) -> StoreResult<&'snapshot [u8]> {
        if key.first() == Some(&keys::TAG_ROW) {
            let (stored_relation, locator) = self.inner.keys.decode_row(key)?;
            if stored_relation != relation
                || locator.home().len() != self.inner.det.home_width(relation)
            {
                return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                    "export row home width",
                )));
            }
            return Ok(value);
        }
        let (stored_relation, _, id) = self.inner.keys.decode_membership(key)?;
        if stored_relation != relation || !value.is_empty() {
            return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                "export membership entry",
            )));
        }
        let Some(row) = rows::fetch_row(
            &self.inner,
            &self.txn,
            relation,
            RowLocator::unclustered(id),
        )?
        else {
            return Err(StoreError::Corruption(StoreCorruption::DanglingIndexEntry));
        };
        Ok(row)
    }
}

// The whole point of the owned snapshot: it moves between workers whole.
// (RoTxn<WithoutTls> is Send; the gate pass and Arc are Send.)
#[cfg(test)]
fn _assert_snapshot_send(snapshot: OwnedSnapshot) -> impl Send {
    snapshot
}
