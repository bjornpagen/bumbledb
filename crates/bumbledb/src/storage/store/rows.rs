//! Row/index primitives shared by the candidate write path and owned
//! snapshots: exact-checked bucket lookup, insert/remove with symmetric
//! index maintenance, bounded visits.
//!
//! Equality is always full canonical bytes. Routing bytes (exact scalar group
//! or fingerprint) only narrow the candidate bucket.

use bumbledb_theory::schema::RelationId;
use heed::RoTxn;

use super::candidate::RowIndexer;
use super::det_index;
use super::error::{StoreCorruption, StoreError, StoreResult};
use super::format::{self, K_NEXT_ROW_ID, RowId, RowLocator};
use super::store_env::{GatedRwTxn, StoreInner, map_txn_error};
use crate::schema::ProjectionId;
use crate::schema::compiled::KeyEncoding;
use crate::work::WorkContext;

/// Bounded compare/copy polling quantum (bytes per work step).
pub(crate) const BYTE_QUANTUM: usize = 4096;

pub(crate) fn chunked_eq(a: &[u8], b: &[u8], work: &WorkContext) -> StoreResult<bool> {
    if a.len() != b.len() {
        work.step(1)?;
        return Ok(false);
    }
    for (left, right) in a.chunks(BYTE_QUANTUM).zip(b.chunks(BYTE_QUANTUM)) {
        work.step(left.len() as u64)?;
        if left != right {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn chunked_cmp(
    a: &[u8],
    b: &[u8],
    work: &WorkContext,
) -> StoreResult<std::cmp::Ordering> {
    for (left, right) in a.chunks(BYTE_QUANTUM).zip(b.chunks(BYTE_QUANTUM)) {
        work.step(left.len().min(right.len()) as u64)?;
        let order = left.cmp(right);
        if order != std::cmp::Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(a.len().cmp(&b.len()))
}

pub(crate) fn fetch_row<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
    row: RowLocator,
) -> StoreResult<Option<&'txn [u8]>> {
    if row.home().len() != inner.det.home_width(relation) {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "row home width",
        )));
    }
    inner
        .data
        .get(txn, inner.keys.row_key(relation, row)?.as_slice())
        .map_err(StoreError::from_heed)
}

/// Exact membership: a selected scalar-key bucket or the full-row fingerprint
/// narrows candidates; full canonical bytes always decide equality.
pub(crate) fn exact_lookup(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
    row: &[u8],
    work: &WorkContext,
) -> StoreResult<Option<RowLocator>> {
    if let Some(projection) = inner.det.membership_projection(relation) {
        let fields = inner
            .det
            .fields_of(relation)
            .ok_or(StoreError::ForeignSchema)?;
        let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
        let route =
            crate::canonical::exact_scalar_projection(fields, row, projection, work, &mut route)?
                .ok_or(StoreError::ForeignSchema)?;
        return exact_lookup_projected(inner, txn, projection, route, row, work);
    }
    let fp = inner.fingerprinter.row(relation, row);
    exact_lookup_hashed(inner, txn, relation, row, &fp, work)
}

fn exact_lookup_projected(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    projection: &crate::schema::CompiledProjection,
    routing: &[u8],
    row: &[u8],
    work: &WorkContext,
) -> StoreResult<Option<RowLocator>> {
    Ok(lookup_projected(inner, txn, projection, routing, row, work)?.found)
}

struct ProjectedLookup {
    found: Option<RowLocator>,
    conflicting_row: bool,
}

/// The same canonical comparisons used for membership also establish
/// whether this insertion encountered a different row in its exact home.
/// No extra seek, decoded row, or retained determinant is required.
#[inline]
fn lookup_projected(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    projection: &crate::schema::CompiledProjection,
    routing: &[u8],
    row: &[u8],
    work: &WorkContext,
) -> StoreResult<ProjectedLookup> {
    let mut found = None;
    let mut conflicting_row = false;
    visit_determinant_bucket(inner, txn, projection, routing, work, &mut |id, stored| {
        if chunked_eq(stored, row, work)? {
            found = Some(id);
            Ok(false)
        } else {
            conflicting_row = true;
            Ok(true)
        }
    })?;
    Ok(ProjectedLookup {
        found,
        conflicting_row,
    })
}

/// Reuse the operation's fingerprint, never its answer: every bucket hit
/// still confirms the full canonical bytes, including forced collisions.
fn exact_lookup_hashed(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
    row: &[u8],
    fp: &[u8; super::fingerprint::FP_LEN],
    work: &WorkContext,
) -> StoreResult<Option<RowLocator>> {
    let bucket = inner.keys.membership_bucket(relation, fp)?;
    let range = inner
        .data
        .prefix_iter(txn, bucket.as_slice())
        .map_err(StoreError::from_heed)?;
    for entry in range {
        work.step(1)?;
        let (key, _) = entry.map_err(StoreError::from_heed)?;
        let candidate = RowLocator::unclustered(inner.keys.decode_membership(key)?.2);
        let Some(stored) = fetch_row(inner, txn, relation, candidate)? else {
            return Err(StoreError::Corruption(StoreCorruption::DanglingIndexEntry));
        };
        if chunked_eq(stored, row, work)? {
            return Ok(Some(candidate));
        }
    }
    work.checkpoint()?;
    Ok(None)
}

/// Routing bytes for one compiled projection's projected canonical bytes.
pub(crate) enum RoutingBytes<'a> {
    Exact(&'a [u8]),
    Fingerprint([u8; super::fingerprint::FP_LEN]),
}

impl RoutingBytes<'_> {
    pub(crate) fn as_slice(&self) -> &[u8] {
        match self {
            Self::Exact(bytes) => bytes,
            Self::Fingerprint(bytes) => bytes,
        }
    }
}

impl std::ops::Deref for RoutingBytes<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

fn routing_bytes<'a>(
    inner: &StoreInner,
    projection: ProjectionId,
    projected: &'a [u8],
    encoding: KeyEncoding,
) -> RoutingBytes<'a> {
    match encoding {
        KeyEncoding::ExactBounded { .. } => RoutingBytes::Exact(projected),
        KeyEncoding::FingerprintBucket => RoutingBytes::Fingerprint(
            det_index::fingerprint_routing(inner.fingerprinter, projection, projected),
        ),
    }
}

/// Route and consume one entry at a time. A custom emitter cannot turn a
/// failed mutation into success by swallowing its callback error: the first
/// failure is sticky, and every subsequent callback refuses without writing.
/// The compiled producer can reuse a decoded row or decode on demand.
fn visit_determinants<I: RowIndexer + ?Sized>(
    inner: &StoreInner,
    indexer: &I,
    relation: RelationId,
    row: &[u8],
    work: &WorkContext,
    compiled: impl FnOnce(super::ProjectionEmitter<'_>) -> StoreResult<()>,
    mut visit: impl FnMut(ProjectionId, &[u8]) -> StoreResult<()>,
) -> StoreResult<()> {
    let mut first_error: Option<StoreError> = None;
    let result = {
        let mut emit = |projection, projected: &[u8]| {
            if let Some(error) = &first_error {
                return Err(error.clone());
            }
            let result = (|| {
                work.step(1)?;
                let compiled = inner
                    .det
                    .projection(projection)
                    .ok_or(StoreError::ForeignSchema)?;
                if compiled.relation != relation {
                    return Err(StoreError::ForeignSchema);
                }
                let routing = routing_bytes(inner, projection, projected, compiled.encoding);
                visit(projection, &routing)
            })();
            if let Err(error) = &result {
                first_error = Some(error.clone());
            }
            result
        };
        compiled(&mut emit).and_then(|()| indexer.index_row(relation, row, work, &mut emit))
    };
    first_error.map_or(result, Err)
}

/// Persist each physical family in one owning transaction. A selected scalar
/// projection supplies membership already, so its fingerprint is absent.
fn persist_insert<I: RowIndexer + ?Sized>(
    inner: &StoreInner,
    txn: &mut heed::RwTxn<'_>,
    indexer: &I,
    (relation, locator, row): (RelationId, RowLocator, &[u8]),
    fingerprint: Option<&[u8; super::fingerprint::FP_LEN]>,
    work: &WorkContext,
    compiled: impl FnOnce(super::ProjectionEmitter<'_>) -> StoreResult<()>,
) -> StoreResult<()> {
    work.step(row.len() as u64)?;
    inner
        .data
        .put(txn, inner.keys.row_key(relation, locator)?.as_slice(), row)
        .map_err(map_txn_error)?;
    if let Some(fp) = fingerprint {
        inner
            .data
            .put(
                txn,
                inner
                    .keys
                    .membership_key(relation, fp, locator.id)?
                    .as_slice(),
                &[],
            )
            .map_err(map_txn_error)?;
    }
    visit_determinants(
        inner,
        indexer,
        relation,
        row,
        work,
        compiled,
        |projection, routing| {
            if inner.det.is_home(projection) {
                if routing != locator.home() {
                    return Err(StoreError::ForeignSchema);
                }
                return Ok(());
            }
            work.step(1)?;
            inner
                .data
                .put(
                    txn,
                    inner
                        .keys
                        .determinant_key(projection, routing, locator.id)?
                        .as_slice(),
                    locator.home(),
                )
                .map_err(map_txn_error)
        },
    )
}

fn persist_remove<I: RowIndexer + ?Sized>(
    inner: &StoreInner,
    txn: &mut heed::RwTxn<'_>,
    indexer: &I,
    (relation, locator, row): (RelationId, RowLocator, &[u8]),
    fingerprint: Option<&[u8; super::fingerprint::FP_LEN]>,
    work: &WorkContext,
    compiled: impl FnOnce(super::ProjectionEmitter<'_>) -> StoreResult<()>,
) -> StoreResult<()> {
    inner
        .data
        .delete(txn, inner.keys.row_key(relation, locator)?.as_slice())
        .map_err(map_txn_error)?;
    if let Some(fp) = fingerprint {
        inner
            .data
            .delete(
                txn,
                inner
                    .keys
                    .membership_key(relation, fp, locator.id)?
                    .as_slice(),
            )
            .map_err(map_txn_error)?;
    }
    visit_determinants(
        inner,
        indexer,
        relation,
        row,
        work,
        compiled,
        |projection, routing| {
            if inner.det.is_home(projection) {
                if routing != locator.home() {
                    return Err(StoreError::ForeignSchema);
                }
                return Ok(());
            }
            work.step(1)?;
            inner
                .data
                .delete(
                    txn,
                    inner
                        .keys
                        .determinant_key(projection, routing, locator.id)?
                        .as_slice(),
                )
                .map(|_| ())
                .map_err(map_txn_error)
        },
    )
}

fn primary_bucket_locator(
    inner: &StoreInner,
    compiled: &crate::schema::CompiledProjection,
    routing: &[u8],
    key: &[u8],
) -> StoreResult<RowLocator> {
    let (relation, locator) = inner.keys.decode_row(key)?;
    if relation != compiled.relation || locator.home() != routing {
        return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
            "primary row bucket",
        )));
    }
    Ok(locator)
}

/// First-match probe only. The ordinary visitor remains the full-scan path
/// for judgment and public snapshots. A continuing probe still sees every
/// candidate, including conflicts in an unready store.
pub(crate) fn probe_determinant_bucket<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    compiled: &crate::schema::CompiledProjection,
    routing: &[u8],
    work: &WorkContext,
    visit: &mut dyn FnMut(RowLocator, &'txn [u8]) -> StoreResult<bool>,
) -> StoreResult<()> {
    if !inner.det.is_home_compiled(compiled) {
        return visit_determinant_bucket(inner, txn, compiled, routing, work, visit);
    }
    if routing.len() != compiled.encoding.routing_width() {
        return Err(StoreError::ForeignSchema);
    }
    let bucket = inner.keys.row_bucket(compiled.relation, routing)?;
    // Unlike range(), this supported heed operation borrows its seek key:
    // no owned range-bound Vec is constructed for the first result.
    // Native failure aborts the probe; do not manufacture a row charge
    // which could replace an allocation/storage fault with budget refusal.
    let first = inner
        .data
        .get_greater_than_or_equal_to(txn, bucket.as_slice())
        .map_err(StoreError::from_heed)?;
    let Some((first_key, first_value)) =
        first.filter(|(key, _)| key.starts_with(bucket.as_slice()))
    else {
        work.checkpoint()?;
        return Ok(());
    };
    work.step(1)?;
    let locator = primary_bucket_locator(inner, compiled, routing, first_key)?;
    if !visit(locator, first_value)? {
        return Ok(());
    }
    // Only a continuing callback pays for a range bound and a second seek.
    // Excluding the exact first key preserves visit order and row charges.
    let range = inner
        .data
        .range(
            txn,
            &(
                std::ops::Bound::Excluded(first_key),
                std::ops::Bound::Unbounded,
            ),
        )
        .map_err(StoreError::from_heed)?;
    for entry in range.take_while(|entry| match entry {
        Ok((key, _)) => key.starts_with(bucket.as_slice()),
        Err(_) => true,
    }) {
        work.step(1)?;
        let (key, value) = entry.map_err(StoreError::from_heed)?;
        let locator = primary_bucket_locator(inner, compiled, routing, key)?;
        if !visit(locator, value)? {
            return Ok(());
        }
    }
    // Continuing can seek/fault past the last candidate. Check that
    // terminal seek too; an explicit callback stop above adds no seek.
    work.checkpoint()?;
    Ok(())
}

/// Exactly one physical prefix: a home bucket or a secondary index bucket.
enum BucketPrefix {
    Home(super::keys::Key<21>),
    Secondary(super::keys::Key<19>),
}

impl BucketPrefix {
    fn new(
        inner: &StoreInner,
        compiled: &crate::schema::CompiledProjection,
        routing: &[u8],
    ) -> StoreResult<Self> {
        if routing.len() != compiled.encoding.routing_width() {
            return Err(StoreError::ForeignSchema);
        }
        if inner.det.is_home_compiled(compiled) {
            inner
                .keys
                .row_bucket(compiled.relation, routing)
                .map(Self::Home)
        } else {
            inner
                .keys
                .determinant_bucket(compiled.id, routing)
                .map(Self::Secondary)
        }
    }

    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Home(key) => key.as_slice(),
            Self::Secondary(key) => key.as_slice(),
        }
    }
}

/// Count routing entries without fetching secondary row bodies. None means
/// the bucket exceeds the caller's access-path budget, never an exact count.
/// Inspect at most limit+1 entries, with ordinary per-entry work charging.
pub(crate) fn count_determinant_bucket_bounded(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    compiled: &crate::schema::CompiledProjection,
    routing: &[u8],
    limit: u64,
    work: &WorkContext,
) -> StoreResult<Option<u64>> {
    let prefix = BucketPrefix::new(inner, compiled, routing)?;
    let bucket = prefix.as_slice();
    let mut count = 0u64;
    for entry in inner
        .data
        .prefix_iter(txn, bucket)
        .map_err(StoreError::from_heed)?
    {
        work.step(1)?;
        let (key, _) = entry.map_err(StoreError::from_heed)?;
        if key.len() != bucket.len() + 8 {
            return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                "scalar determinant bucket",
            )));
        }
        if count == limit {
            return Ok(None);
        }
        count += 1;
    }
    work.checkpoint()?;
    Ok(Some(count))
}

/// Bounded visitor over one determinant bucket — one row at a time, no
/// materialized id or decoded-row collection.
pub(crate) fn visit_determinant_bucket<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    compiled: &crate::schema::CompiledProjection,
    routing: &[u8],
    work: &WorkContext,
    visit: &mut dyn FnMut(RowLocator, &'txn [u8]) -> StoreResult<bool>,
) -> StoreResult<()> {
    let prefix = BucketPrefix::new(inner, compiled, routing)?;
    let home = matches!(prefix, BucketPrefix::Home(_));
    let bucket = prefix.as_slice();
    let range = inner
        .data
        .prefix_iter(txn, bucket)
        .map_err(StoreError::from_heed)?;
    let mut visited = false;
    for entry in range {
        visited = true;
        work.step(1)?;
        let (key, value) = entry.map_err(StoreError::from_heed)?;
        let (locator, bytes) = if home {
            (
                primary_bucket_locator(inner, compiled, routing, key)?,
                value,
            )
        } else {
            // The scalar prefix is complete: no physical interval tail or
            // additional routing bytes may hide before the ordinal suffix.
            if key.len() != bucket.len() + 8 {
                return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                    "scalar determinant bucket",
                )));
            }
            let id = inner.keys.decode_determinant(key)?.2;
            let locator = RowLocator::new(id, value)?;
            let bytes = fetch_row(inner, txn, compiled.relation, locator)?
                .ok_or(StoreError::Corruption(StoreCorruption::DanglingIndexEntry))?;
            (locator, bytes)
        };
        if !visit(locator, bytes)? {
            break;
        }
    }
    if !visited {
        // The first next() performs the seek, which can fault in pages.
        // Empty buckets still observe cancellation/deadlines after that
        // seek; hits retain their existing per-row check and charge.
        work.checkpoint()?;
    }
    Ok(())
}

/// Legacy enumeration — prefer [`visit_determinant_bucket`]. Still bounded
/// by work steps; collects ids only when callers require a vec.
pub(crate) fn determinant_bucket_ids(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    projection: ProjectionId,
    projected: &[u8],
    work: &WorkContext,
) -> StoreResult<Vec<RowLocator>> {
    let compiled = inner
        .det
        .projection(projection)
        .ok_or(StoreError::ForeignSchema)?;
    let routing = routing_for_compiled(inner, compiled, projected);
    let mut ids = Vec::new();
    visit_determinant_bucket(inner, txn, compiled, &routing, work, &mut |id, _| {
        ids.push(id);
        Ok(true)
    })?;
    Ok(ids)
}

/// Owns the private write transaction until all row metadata is flushed.
/// The single active relation slot needs no allocation: ordered row streams
/// flush once per relation, while arbitrary ordering remains correct by
/// flushing on every switch. Dropping an unfinished writer aborts its rows
/// and counters together; a map-growth retry starts with a fresh owner.
/// An operation error may follow partial private mutations: callers must
/// immediately propagate it and drop this writer, never catch it and finish.
pub(crate) struct RowWriter<'inner, 'env, 'work> {
    inner: &'inner StoreInner,
    txn: GatedRwTxn<'env>,
    work: &'work WorkContext,
    next_id: Option<u64>,
    active_count: Option<(RelationId, u64)>,
    // Sticky for this transaction only. With a lawful parent, removals
    // preserve scalar uniqueness; a new conflict must be encountered by
    // a projected insertion. A duplicate's early hit cannot repair it.
    home_keys_preserved: bool,
    decode: crate::canonical::DecodeScratch<'work>,
}

fn allocate_row_id(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    next_id: &mut Option<u64>,
) -> StoreResult<RowId> {
    let next = match *next_id {
        Some(next) => next,
        None => format::read_u64(&inner.meta, txn, K_NEXT_ROW_ID, "next row id")?,
    };
    *next_id = Some(next.checked_add(1).ok_or(StoreError::RowIdExhausted)?);
    Ok(RowId(next))
}

impl<'inner, 'env, 'work> RowWriter<'inner, 'env, 'work> {
    pub(crate) fn new(
        inner: &'inner StoreInner,
        txn: GatedRwTxn<'env>,
        work: &'work WorkContext,
    ) -> Self {
        Self {
            inner,
            txn,
            work,
            next_id: None,
            active_count: None,
            home_keys_preserved: true,
            decode: crate::canonical::DecodeScratch::new(work),
        }
    }

    pub(crate) fn home_keys_preserved(&self) -> bool {
        self.home_keys_preserved
    }

    fn next_row_id(&mut self) -> StoreResult<RowId> {
        allocate_row_id(self.inner, &self.txn.txn, &mut self.next_id)
    }

    fn flush_row_count(&mut self) -> StoreResult<()> {
        if let Some((relation, count)) = self.active_count {
            self.work.checkpoint()?;
            self.inner
                .meta
                .put(
                    &mut self.txn.txn,
                    format::row_count_key(relation).as_slice(),
                    &count.to_be_bytes(),
                )
                .map_err(map_txn_error)?;
            self.active_count = None;
        }
        Ok(())
    }

    fn shift_row_count(&mut self, relation: RelationId, delta: i64) -> StoreResult<()> {
        let current = match self.active_count {
            Some((active, count)) if active == relation => count,
            _ => {
                self.flush_row_count()?;
                row_count(self.inner, &self.txn.txn, relation)?
            }
        };
        let next = current
            .checked_add_signed(delta)
            .ok_or(StoreError::Corruption(StoreCorruption::MetaMissing(
                "row count underflow",
            )))?;
        self.active_count = Some((relation, next));
        Ok(())
    }

    /// Return the transaction only after its own metadata agrees with the
    /// final rows, before judgment, host records, or a commit can observe it.
    pub(crate) fn finish(mut self) -> StoreResult<GatedRwTxn<'env>> {
        self.flush_row_count()?;
        if let Some(next) = self.next_id {
            self.work.checkpoint()?;
            self.inner
                .meta
                .put(&mut self.txn.txn, K_NEXT_ROW_ID, &next.to_be_bytes())
                .map_err(map_txn_error)?;
        }
        Ok(self.txn)
    }
}

pub(crate) fn row_count(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
) -> StoreResult<u64> {
    let key = format::row_count_key(relation);
    match inner
        .meta
        .get(txn, key.as_slice())
        .map_err(StoreError::from_heed)?
    {
        Some(bytes) => Ok(u64::from_be_bytes(bytes.try_into().map_err(|_| {
            StoreError::Corruption(StoreCorruption::MetaMissing("row count"))
        })?)),
        None => Ok(0),
    }
}

impl RowWriter<'_, '_, '_> {
    pub(crate) fn insert<I: RowIndexer + ?Sized>(
        &mut self,
        relation: RelationId,
        row: &[u8],
        indexer: &I,
    ) -> StoreResult<Option<RowId>> {
        let inner = self.inner;
        let work = self.work;
        if let Some(projection) = inner.det.membership_projection(relation) {
            return self.insert_projected(relation, row, indexer, projection);
        }
        let fp = inner.fingerprinter.row(relation, row);
        if exact_lookup_hashed(inner, &self.txn.txn, relation, row, &fp, work)?.is_some() {
            return Ok(None);
        }
        let id = self.next_row_id()?;
        persist_insert(
            inner,
            &mut self.txn.txn,
            indexer,
            (relation, RowLocator::unclustered(id), row),
            Some(&fp),
            work,
            |emit| inner.det.emit_row(relation, row, &mut self.decode, emit),
        )?;
        self.shift_row_count(relation, 1)?;
        Ok(Some(id))
    }

    pub(crate) fn remove<I: RowIndexer + ?Sized>(
        &mut self,
        relation: RelationId,
        row: &[u8],
        indexer: &I,
    ) -> StoreResult<bool> {
        let inner = self.inner;
        let work = self.work;
        if let Some(projection) = inner.det.membership_projection(relation) {
            return self.remove_projected(relation, row, indexer, projection);
        }
        let fp = inner.fingerprinter.row(relation, row);
        let Some(id) = exact_lookup_hashed(inner, &self.txn.txn, relation, row, &fp, work)? else {
            return Ok(false);
        };
        persist_remove(
            inner,
            &mut self.txn.txn,
            indexer,
            (relation, id, row),
            Some(&fp),
            work,
            |emit| inner.det.emit_row(relation, row, &mut self.decode, emit),
        )?;
        self.shift_row_count(relation, -1)?;
        Ok(true)
    }

    fn insert_projected<I: RowIndexer + ?Sized>(
        &mut self,
        relation: RelationId,
        row: &[u8],
        indexer: &I,
        projection: &crate::schema::CompiledProjection,
    ) -> StoreResult<Option<RowId>> {
        let inner = self.inner;
        let work = self.work;
        let fields = inner
            .det
            .fields_of(relation)
            .ok_or(StoreError::ForeignSchema)?;
        let txn = &mut self.txn.txn;
        let next_id = &mut self.next_id;
        let home_keys_preserved = &mut self.home_keys_preserved;
        let inserted =
            self.decode
                .with_decoded(fields, row, |values| -> StoreResult<Option<RowId>> {
                    let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
                    let route = projection
                        .encode_scalar_row(values, &mut route)
                        .ok_or(StoreError::ForeignSchema)?;
                    let lookup = lookup_projected(inner, txn, projection, route, row, work)?;
                    *home_keys_preserved &= !lookup.conflicting_row;
                    if lookup.found.is_some() {
                        return Ok(None);
                    }
                    let id = allocate_row_id(inner, txn, next_id)?;
                    persist_insert(
                        inner,
                        txn,
                        indexer,
                        (relation, RowLocator::new(id, route)?, row),
                        None,
                        work,
                        |emit| inner.det.emit_decoded(relation, values, work, emit),
                    )?;
                    Ok(Some(id))
                })?;
        if inserted.is_some() {
            self.shift_row_count(relation, 1)?;
        }
        Ok(inserted)
    }

    fn remove_projected<I: RowIndexer + ?Sized>(
        &mut self,
        relation: RelationId,
        row: &[u8],
        indexer: &I,
        projection: &crate::schema::CompiledProjection,
    ) -> StoreResult<bool> {
        let inner = self.inner;
        let work = self.work;
        let fields = inner
            .det
            .fields_of(relation)
            .ok_or(StoreError::ForeignSchema)?;
        let txn = &mut self.txn.txn;
        let removed = self
            .decode
            .with_decoded(fields, row, |values| -> StoreResult<bool> {
                let mut route = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
                let route = projection
                    .encode_scalar_row(values, &mut route)
                    .ok_or(StoreError::ForeignSchema)?;
                let Some(id) = exact_lookup_projected(inner, txn, projection, route, row, work)?
                else {
                    return Ok(false);
                };
                persist_remove(
                    inner,
                    txn,
                    indexer,
                    (relation, id, row),
                    None,
                    work,
                    |emit| inner.det.emit_decoded(relation, values, work, emit),
                )?;
                Ok(true)
            })?;
        if removed {
            self.shift_row_count(relation, -1)?;
        }
        Ok(removed)
    }
}

pub(crate) fn scan_rows<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
) -> StoreResult<impl Iterator<Item = StoreResult<(RowLocator, &'txn [u8])>>> {
    Ok(scan_row_entries(inner, txn, relation)?.map(|entry| {
        let (key, value) = entry?;
        Ok((key.locator()?, value))
    }))
}

pub(crate) fn scan_row_bytes<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
) -> StoreResult<impl Iterator<Item = StoreResult<&'txn [u8]>>> {
    Ok(scan_row_entries(inner, txn, relation)?.map(|entry| entry.map(|(_, value)| value)))
}

/// One cursor and one validation path for both address-bearing and body-only
/// scans. Borrowed keys defer address materialization, never validation.
fn scan_row_entries<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
) -> StoreResult<impl Iterator<Item = StoreResult<(super::keys::RowKey<'txn>, &'txn [u8])>>> {
    let prefix = inner.keys.row_prefix(relation)?;
    let range = inner
        .data
        .prefix_iter(txn, prefix.as_slice())
        .map_err(StoreError::from_heed)?;
    let layout = inner.keys;
    let home_width = inner.det.home_width(relation);
    Ok(range.map(move |entry| {
        let (key, value) = entry.map_err(StoreError::from_heed)?;
        let key = layout.decode_row_key(key)?;
        if key.relation != relation || key.home.len() != home_width {
            return Err(StoreError::Corruption(StoreCorruption::MalformedKey(
                "row home width",
            )));
        }
        Ok((key, value))
    }))
}

/// Routing bytes for one interned projection's projected canonical bytes.
pub(crate) fn routing_for_projected<'a>(
    inner: &StoreInner,
    projection: ProjectionId,
    projected: &'a [u8],
) -> StoreResult<RoutingBytes<'a>> {
    let compiled = inner
        .det
        .projection(projection)
        .ok_or(StoreError::ForeignSchema)?;
    Ok(routing_for_compiled(inner, compiled, projected))
}

/// The descriptor is already resolved against this store's compiled table.
pub(crate) fn routing_for_compiled<'a>(
    inner: &StoreInner,
    compiled: &crate::schema::CompiledProjection,
    projected: &'a [u8],
) -> RoutingBytes<'a> {
    routing_bytes(inner, compiled.id, projected, compiled.encoding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::CanonicalRow;
    use crate::storage::store::UnindexedRows;
    use crate::storage::store::tests::{NOTE, TAG, create_default, note, schema, store_dir, work};

    #[test]
    fn body_only_scan_retains_key_validation_order_and_lazy_stopping() {
        let (_dir, path) = store_dir("body-only-scan");
        let store = create_default(&path);
        let context = work();
        let _owner = store.writer(&context).unwrap();
        for relation in [NOTE, TAG] {
            let width = store.inner.det.home_width(relation);
            let home = vec![0x40; width];
            let locator = RowLocator::new(RowId(7), &home).unwrap();
            let valid = store.inner.keys.row_key(relation, locator).unwrap();
            for malformed_first in [true, false] {
                let mut txn = store.gated_write_txn(&context).unwrap();
                let mut malformed = store.inner.keys.row_prefix(relation).unwrap().to_vec();
                malformed.extend(std::iter::repeat_n(
                    if malformed_first { 0 } else { u8::MAX },
                    width + if malformed_first { 7 } else { 9 },
                ));
                store.inner.data.put(&mut txn.txn, &valid, b"body").unwrap();
                store
                    .inner
                    .data
                    .put(&mut txn.txn, &malformed, b"bad")
                    .unwrap();

                let expected: Vec<_> = scan_rows(&store.inner, &txn.txn, relation)
                    .unwrap()
                    .map(|entry| entry.map(|(_, bytes)| bytes))
                    .collect();
                let actual: Vec<_> = scan_row_bytes(&store.inner, &txn.txn, relation)
                    .unwrap()
                    .collect();
                assert_eq!(
                    actual, expected,
                    "same values and exact errors in source order"
                );
                assert_eq!(actual.len(), 2);
                assert!(actual[usize::from(!malformed_first)].is_err());
                assert_eq!(actual[usize::from(malformed_first)], Ok(b"body".as_slice()));
                if !malformed_first {
                    let mut bytes = scan_row_bytes(&store.inner, &txn.txn, relation).unwrap();
                    let borrowed = bytes.next().unwrap().unwrap();
                    assert_eq!(borrowed, b"body", "no eager parse of corrupt successor");
                    drop(bytes);
                    assert_eq!(borrowed, b"body", "body borrows transaction, not cursor");
                }
            }
        }
    }

    struct FailAfterDuplicate;

    impl RowIndexer for FailAfterDuplicate {
        fn index_row(
            &self,
            relation: RelationId,
            row: &[u8],
            work: &WorkContext,
            emit: super::super::ProjectionEmitter<'_>,
        ) -> StoreResult<()> {
            crate::storage::store::tests::FirstFieldKey.index_row(relation, row, work, emit)?;
            Err(StoreError::Allocation)
        }
    }

    #[test]
    fn scalar_key_membership_keeps_competitors_and_keyless_collision_buckets() {
        use super::super::fingerprint::FP_LEN;
        use crate::storage::store::{MapPolicy, Store};
        let (_dir, path) = store_dir("selected-membership");
        let schema = schema();
        let store =
            Store::create_forced_fingerprint(&path, &schema, MapPolicy::default(), [0; FP_LEN])
                .unwrap();
        let context = work();
        let notes = ["first", "second", "missing"].map(|body| {
            CanonicalRow::encode(schema.relation(NOTE).fields(), &note(7, body), &context).unwrap()
        });
        let tags = ["left", "right"].map(|label| {
            CanonicalRow::encode(
                schema.relation(TAG).fields(),
                &[crate::Value::String(label.into())],
                &context,
            )
            .unwrap()
        });
        let _owner = store.writer(&context).unwrap();
        let mut writer = RowWriter::new(
            &store.inner,
            store.gated_write_txn(&context).unwrap(),
            &context,
        );
        for (at, row) in notes[..2].iter().enumerate() {
            assert_eq!(
                writer.insert(NOTE, row, &UnindexedRows).unwrap(),
                Some(RowId(at as u64 + 1))
            );
        }
        assert_eq!(
            writer.insert(NOTE, &notes[0], &UnindexedRows).unwrap(),
            None
        );
        assert_eq!(
            exact_lookup(&store.inner, &writer.txn.txn, NOTE, &notes[2], &context).unwrap(),
            None,
            "an existing declared key does not make a different full row present"
        );
        for row in &tags {
            writer.insert(TAG, row, &UnindexedRows).unwrap();
        }
        for (relation, count) in [(NOTE, 0), (TAG, 2)] {
            let prefix = store
                .inner
                .keys
                .membership_bucket(relation, &[0; FP_LEN])
                .unwrap();
            assert_eq!(
                store
                    .inner
                    .data
                    .prefix_iter(&writer.txn.txn, &prefix)
                    .unwrap()
                    .count(),
                count
            );
        }
        for (at, row) in notes[..2].iter().enumerate() {
            assert_eq!(
                exact_lookup(&store.inner, &writer.txn.txn, NOTE, row, &context).unwrap(),
                Some(RowLocator::new(RowId(at as u64 + 1), &7u64.to_be_bytes()).unwrap()),
                "competing full rows remain independently discoverable"
            );
        }
        assert!(writer.remove(NOTE, &notes[0], &UnindexedRows).unwrap());
        assert!(writer.remove(TAG, &tags[0], &UnindexedRows).unwrap());
        assert_eq!(
            exact_lookup(&store.inner, &writer.txn.txn, NOTE, &notes[1], &context).unwrap(),
            Some(RowLocator::new(RowId(2), &7u64.to_be_bytes()).unwrap())
        );
        assert_eq!(
            exact_lookup(&store.inner, &writer.txn.txn, TAG, &tags[1], &context).unwrap(),
            Some(RowLocator::unclustered(RowId(4))),
            "removing one forced-collision member preserves the other"
        );
        assert_eq!(store.inner.data.len(&writer.txn.txn).unwrap(), 3);
    }

    #[test]
    fn streaming_insert_and_remove_failures_roll_back_rows_indexes_and_counters() {
        use crate::storage::store::tests::{change_set, commit_changes};
        for inserting in [true, false] {
            let (_dir, path) = store_dir("stream-rollback");
            let store = create_default(&path);
            let context = work();
            let schema = schema();
            let values = note(1, "body");
            if !inserting {
                commit_changes(&store, &change_set(&schema, &[(NOTE, values.clone())], &[]));
            }
            let row =
                CanonicalRow::encode(schema.relation(NOTE).fields(), &values, &context).unwrap();
            let before = store.snapshot(&context).unwrap();
            let generation = before.generation();
            let next =
                format::read_u64(&store.inner.meta, before.read_txn(), K_NEXT_ROW_ID, "next")
                    .unwrap();
            drop(before);
            let owner = store.writer(&context).unwrap();
            let mut writer = RowWriter::new(
                &store.inner,
                store.gated_write_txn(&context).unwrap(),
                &context,
            );
            let result = if inserting {
                writer.insert(NOTE, &row, &FailAfterDuplicate).map(|_| ())
            } else {
                writer.remove(NOTE, &row, &FailAfterDuplicate).map(|_| ())
            };
            assert_eq!(result, Err(StoreError::Allocation));
            assert_eq!(
                store.inner.data.len(&writer.txn.txn).unwrap(),
                u64::from(inserting),
                "the failed operation already mutated all physical families privately"
            );
            drop(writer);
            drop(owner);
            let after = store.snapshot(&context).unwrap();
            assert_eq!(after.generation(), generation);
            assert_eq!(after.row_count(NOTE).unwrap(), u64::from(!inserting));
            assert_eq!(
                after.contains(NOTE, row.as_bytes(), &context).unwrap(),
                !inserting
            );
            assert_eq!(
                store.inner.data.len(after.read_txn()).unwrap(),
                u64::from(!inserting)
            );
            assert_eq!(
                format::read_u64(&store.inner.meta, after.read_txn(), K_NEXT_ROW_ID, "next")
                    .unwrap(),
                next
            );
            assert!(
                super::super::verify::sweep(&after, &schema, &context)
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn swallowed_sink_error_stays_first_and_blocks_every_later_mutation() {
        use crate::storage::store::tests::NOTE_KEY;
        struct Swallow {
            later_error: bool,
        }
        impl RowIndexer for Swallow {
            fn index_row(
                &self,
                _relation: RelationId,
                _row: &[u8],
                _work: &WorkContext,
                emit: super::super::ProjectionEmitter<'_>,
            ) -> StoreResult<()> {
                for value in [1u64, 2] {
                    assert_eq!(
                        emit(NOTE_KEY, &value.to_be_bytes()),
                        Err(StoreError::Allocation)
                    );
                }
                if self.later_error {
                    Err(StoreError::ForeignSchema)
                } else {
                    Ok(())
                }
            }
        }
        let (_dir, path) = store_dir("stream-sticky");
        let store = create_default(&path);
        let context = work();
        let schema = schema();
        let row = CanonicalRow::encode(schema.relation(NOTE).fields(), &note(1, "body"), &context)
            .unwrap();
        for later_error in [false, true] {
            let mut visits = 0;
            let mut scratch = crate::canonical::DecodeScratch::new(&context);
            let result = visit_determinants(
                &store.inner,
                &Swallow { later_error },
                NOTE,
                &row,
                &context,
                |emit| store.inner.det.emit_row(NOTE, &row, &mut scratch, emit),
                |_, _| {
                    visits += 1;
                    if visits == 2 {
                        Err(StoreError::Allocation)
                    } else {
                        Ok(())
                    }
                },
            );
            assert_eq!(result, Err(StoreError::Allocation));
            assert_eq!(
                visits, 2,
                "built-in visit plus first custom failure; no later sink call"
            );
        }
    }

    struct RetryDuplicate {
        fail_once: std::cell::Cell<bool>,
        calls: std::cell::Cell<usize>,
    }

    impl RowIndexer for RetryDuplicate {
        fn index_row(
            &self,
            relation: RelationId,
            row: &[u8],
            work: &WorkContext,
            emit: super::super::ProjectionEmitter<'_>,
        ) -> StoreResult<()> {
            self.calls.set(self.calls.get() + 1);
            // Automatic emission plus two duplicate custom emissions must
            // remain one physical entry, for insertion and deletion alike.
            for _ in 0..2 {
                crate::storage::store::tests::FirstFieldKey.index_row(relation, row, work, emit)?;
            }
            if self.fail_once.replace(false) {
                Err(StoreError::MapFull { map_bytes: 0 })
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn retry_after_partial_streaming_preserves_set_indexes_and_single_counter_change() {
        use crate::storage::store::tests::{NO_HOST, change_set, tiny_map};
        use crate::storage::store::{Prepared, Store};
        let (_dir, path) = store_dir("stream-retry");
        let schema = schema();
        let store = Store::create(&path, &schema, tiny_map()).unwrap().0;
        let values = note(1, "body");
        for inserting in [true, false] {
            let context = work();
            let map_before = store.current_map_bytes();
            let indexer = RetryDuplicate {
                fail_once: std::cell::Cell::new(true),
                calls: std::cell::Cell::new(0),
            };
            let batch = [(NOTE, values.clone())];
            let changes = if inserting {
                change_set(&schema, &batch, &[])
            } else {
                change_set(&schema, &[], &batch)
            };
            let mut owner = store.writer(&context).unwrap();
            let committed = match owner
                .prepare(&changes, &indexer, &ExpectHomePreserved(&schema))
                .unwrap()
            {
                Prepared::Admitted(prepared) => prepared.seal(NO_HOST).unwrap().commit().unwrap(),
                Prepared::Rejected(never) => match never {},
            };
            drop(owner);
            assert_eq!(indexer.calls.get(), 2, "one failed attempt and one replay");
            assert!(store.current_map_bytes() > map_before);
            assert_eq!(committed.application.added, u64::from(inserting));
            assert_eq!(committed.application.removed, u64::from(!inserting));
            let snapshot = store.snapshot(&context).unwrap();
            assert_eq!(snapshot.row_count(NOTE).unwrap(), u64::from(inserting));
            assert_eq!(
                store.inner.data.len(snapshot.read_txn()).unwrap(),
                u64::from(inserting)
            );
            assert_eq!(
                format::read_u64(
                    &store.inner.meta,
                    snapshot.read_txn(),
                    K_NEXT_ROW_ID,
                    "next"
                )
                .unwrap(),
                2
            );
            assert!(
                super::super::verify::sweep(&snapshot, &schema, &context)
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[test]
    fn row_writer_defers_metadata_and_flushes_relation_switches_and_finish() {
        let (_dir, path) = store_dir("row-writer-counters");
        let store = create_default(&path);
        let context = work();
        let schema = schema();
        let rows: Vec<_> = (1..=9)
            .map(|id| {
                CanonicalRow::encode(schema.relation(NOTE).fields(), &note(id, "body"), &context)
                    .unwrap()
            })
            .collect();
        let tag = CanonicalRow::encode(
            schema.relation(TAG).fields(),
            &[crate::Value::String("tag".into())],
            &context,
        )
        .unwrap();
        let _owner = store.writer(&context).unwrap();
        let mut writer = RowWriter::new(
            &store.inner,
            store.gated_write_txn(&context).unwrap(),
            &context,
        );
        for (index, row) in rows[..8].iter().enumerate() {
            assert_eq!(
                writer.insert(NOTE, row, &UnindexedRows).unwrap(),
                Some(RowId(index as u64 + 1))
            );
        }
        // Eight actual inserts update no persistent counters yet.
        assert_eq!(row_count(&store.inner, &writer.txn.txn, NOTE).unwrap(), 0);
        assert_eq!(
            format::read_u64(&store.inner.meta, &writer.txn.txn, K_NEXT_ROW_ID, "next").unwrap(),
            1
        );
        assert!(
            writer
                .insert(NOTE, &rows[0], &UnindexedRows)
                .unwrap()
                .is_none()
        );
        assert!(!writer.remove(NOTE, &rows[8], &UnindexedRows).unwrap());
        assert_eq!(
            writer.insert(TAG, &tag, &UnindexedRows).unwrap(),
            Some(RowId(9))
        );
        assert_eq!(row_count(&store.inner, &writer.txn.txn, NOTE).unwrap(), 8);
        assert_eq!(row_count(&store.inner, &writer.txn.txn, TAG).unwrap(), 0);
        assert!(writer.remove(NOTE, &rows[0], &UnindexedRows).unwrap());
        assert!(!writer.remove(NOTE, &rows[0], &UnindexedRows).unwrap());
        assert_eq!(row_count(&store.inner, &writer.txn.txn, TAG).unwrap(), 1);
        let txn = writer.finish().unwrap();
        assert_eq!(row_count(&store.inner, &txn.txn, NOTE).unwrap(), 7);
        assert_eq!(
            format::read_u64(&store.inner.meta, &txn.txn, K_NEXT_ROW_ID, "next").unwrap(),
            10
        );
        txn.commit().unwrap();
        let snapshot = store.snapshot(&context).unwrap();
        assert_eq!(snapshot.row_count(NOTE).unwrap(), 7);
        assert_eq!(snapshot.row_count(TAG).unwrap(), 1);
    }

    #[test]
    fn unfinished_or_cancelled_row_writer_aborts_flushed_and_cached_counters() {
        let (_dir, path) = store_dir("row-writer-abort");
        let store = create_default(&path);
        let schema = schema();
        for cancel in [false, true] {
            let context = work();
            let first =
                CanonicalRow::encode(schema.relation(NOTE).fields(), &note(1, "one"), &context)
                    .unwrap();
            let tag = CanonicalRow::encode(
                schema.relation(TAG).fields(),
                &[crate::Value::String("tag".into())],
                &context,
            )
            .unwrap();
            let _owner = store.writer(&context).unwrap();
            let mut writer = RowWriter::new(
                &store.inner,
                store.gated_write_txn(&context).unwrap(),
                &context,
            );
            assert_eq!(
                writer.insert(NOTE, &first, &UnindexedRows).unwrap(),
                Some(RowId(1))
            );
            writer.insert(TAG, &tag, &UnindexedRows).unwrap();
            if cancel {
                context.cancel();
                assert!(matches!(
                    writer.finish(),
                    Err(StoreError::Work(crate::WorkError::Cancelled))
                ));
            } else {
                drop(writer);
            }
            let snapshot = store.snapshot(&work()).unwrap();
            assert_eq!(snapshot.row_count(NOTE).unwrap(), 0);
            assert_eq!(snapshot.row_count(TAG).unwrap(), 0);
            assert_eq!(
                format::read_u64(
                    &store.inner.meta,
                    snapshot.read_txn(),
                    K_NEXT_ROW_ID,
                    "next"
                )
                .unwrap(),
                1
            );
        }
        let context = work();
        let first =
            CanonicalRow::encode(schema.relation(NOTE).fields(), &note(2, "retry"), &context)
                .unwrap();
        let _owner = store.writer(&context).unwrap();
        let mut writer = RowWriter::new(
            &store.inner,
            store.gated_write_txn(&context).unwrap(),
            &context,
        );
        assert_eq!(
            writer.insert(NOTE, &first, &UnindexedRows).unwrap(),
            Some(RowId(1))
        );
        writer.finish().unwrap().commit().unwrap();
        assert_eq!(
            store.snapshot(&context).unwrap().row_count(NOTE).unwrap(),
            1
        );
    }

    #[test]
    fn cached_row_count_overflow_and_underflow_abort_fact_changes() {
        let (_dir, path) = store_dir("row-writer-count-overflow");
        let store = create_default(&path);
        let context = work();
        let schema = schema();
        let row = CanonicalRow::encode(schema.relation(NOTE).fields(), &note(1, "one"), &context)
            .unwrap();
        let _owner = store.writer(&context).unwrap();
        for count in [u64::MAX, 0] {
            let mut txn = store.gated_write_txn(&context).unwrap();
            // Install the deliberate corrupt count independently; the row
            // operation must detect it without committing any fact change.
            store
                .inner
                .meta
                .put(
                    &mut txn.txn,
                    format::row_count_key(NOTE).as_slice(),
                    &count.to_be_bytes(),
                )
                .unwrap();
            if count == 0 {
                // The underflow arm has a real row to remove.
                store
                    .inner
                    .data
                    .put(
                        &mut txn.txn,
                        store
                            .inner
                            .keys
                            .row_key(
                                NOTE,
                                RowLocator::new(RowId(1), &1u64.to_be_bytes()).unwrap(),
                            )
                            .unwrap()
                            .as_slice(),
                        row.as_bytes(),
                    )
                    .unwrap();
            }
            txn.commit().unwrap();
            let mut writer = RowWriter::new(
                &store.inner,
                store.gated_write_txn(&context).unwrap(),
                &context,
            );
            let result = if count == u64::MAX {
                writer.insert(NOTE, &row, &UnindexedRows).map(|_| ())
            } else {
                writer.remove(NOTE, &row, &UnindexedRows).map(|_| ())
            };
            assert!(matches!(
                result,
                Err(StoreError::Corruption(StoreCorruption::MetaMissing(
                    "row count underflow"
                )))
            ));
            drop(writer);
            let snapshot = store.snapshot(&context).unwrap();
            assert_eq!(snapshot.row_count(NOTE).unwrap(), count);
            assert_eq!(
                snapshot.contains(NOTE, row.as_bytes(), &context).unwrap(),
                count == 0
            );
        }
    }

    #[test]
    fn projected_conflict_is_sticky_and_aborted_writer_resets_it() {
        let (_dir, path) = store_dir("home-preservation-retry");
        let schema = schema();
        let store = create_default(&path);
        let context = work();
        let rows = [note(1, "z"), note(1, "a"), note(2, "new")].map(|values| {
            CanonicalRow::encode(schema.relation(NOTE).fields(), &values, &context).unwrap()
        });
        let _owner = store.writer(&context).unwrap();
        let mut writer = RowWriter::new(
            &store.inner,
            store.gated_write_txn(&context).unwrap(),
            &context,
        );
        assert!(writer.home_keys_preserved());
        writer.insert(NOTE, &rows[0], &UnindexedRows).unwrap();
        assert!(writer.home_keys_preserved());
        writer.insert(NOTE, &rows[1], &UnindexedRows).unwrap();
        assert!(!writer.home_keys_preserved());
        // The oldest ordinal is the exact duplicate: membership stops before
        // the later conflicting row, but must not clear the sticky conflict.
        assert!(
            writer
                .insert(NOTE, &rows[0], &UnindexedRows)
                .unwrap()
                .is_none()
        );
        assert!(!writer.home_keys_preserved());
        let fail = RetryDuplicate {
            fail_once: std::cell::Cell::new(true),
            calls: std::cell::Cell::new(0),
        };
        assert!(matches!(
            writer.insert(NOTE, &rows[2], &fail),
            Err(StoreError::MapFull { .. })
        ));
        drop(writer);
        assert_eq!(
            store.snapshot(&context).unwrap().row_count(NOTE).unwrap(),
            0
        );
        let mut retry = RowWriter::new(
            &store.inner,
            store.gated_write_txn(&context).unwrap(),
            &context,
        );
        assert!(
            retry.home_keys_preserved(),
            "new transaction, not stale evidence"
        );
        assert_eq!(
            retry.insert(NOTE, &rows[0], &UnindexedRows).unwrap(),
            Some(RowId(1))
        );
        assert!(retry.home_keys_preserved());
        retry.remove(NOTE, &rows[0], &UnindexedRows).unwrap();
        retry.insert(NOTE, &rows[1], &UnindexedRows).unwrap();
        assert!(
            retry.home_keys_preserved(),
            "remove then replacement is lawful"
        );
        drop(retry);
        assert_eq!(
            store.snapshot(&context).unwrap().row_count(NOTE).unwrap(),
            0
        );
    }

    struct ExpectHomePreserved<'s>(&'s crate::Schema);

    impl crate::storage::store::CandidateJudge for ExpectHomePreserved<'_> {
        type Rejection = std::convert::Infallible;
        fn judge(
            &self,
            candidate: &crate::storage::store::CandidateState<'_, '_>,
            work: &WorkContext,
        ) -> StoreResult<crate::storage::store::Judgment<Self::Rejection>> {
            assert!(candidate.preserves_home_key(self.0, bumbledb_theory::schema::StatementId(0)));
            crate::storage::store::CandidateJudge::judge(
                &crate::storage::store::tests::AdmitAll,
                candidate,
                work,
            )
        }
    }
}
