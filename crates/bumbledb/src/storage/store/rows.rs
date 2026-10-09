//! Row and determinant primitives shared by the candidate write path and
//! owned snapshots: home and routing derivation, bucket lookups and visits,
//! and the row writer that keeps rows, determinant entries and relation
//! metadata in step inside one transaction.

use std::collections::BTreeMap;

use bumbledb_theory::schema::RelationId;
use heed::{RoTxn, RwTxn};

use super::format::{self, K_NEXT_ROW_ID, RelationMeta, RowId};
use super::keys::{self, Route};
use super::store_env::StoreInner;
use crate::error::CorruptionError;
use crate::error::{Error, Result};
use crate::schema::CompiledProjection;
use crate::schema::compiled::KeyEncoding;
use crate::work::WorkContext;

/// The home of a row given only its bytes.
pub(crate) fn home_of_row(
    inner: &StoreInner,
    relation: RelationId,
    row: &[u8],
    work: &WorkContext,
) -> Result<Route> {
    let Some(projection) = inner.det.home_projection(relation) else {
        return Ok(inner.fingerprinter.row(relation, row));
    };
    let fields = inner.det.fields_of(relation).ok_or(Error::ForeignSchema)?;
    let mut exact = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
    let exact =
        crate::canonical::exact_scalar_projection(fields, row, projection, work, &mut exact)?
            .ok_or(Error::ForeignSchema)?;
    keys::padded(exact)
}

/// The home of a row whose decoded values are at hand.
fn home_of_decoded(
    inner: &StoreInner,
    relation: RelationId,
    row: &[u8],
    values: &[crate::Value],
) -> Result<Route> {
    let Some(projection) = inner.det.home_projection(relation) else {
        return Ok(inner.fingerprinter.row(relation, row));
    };
    let mut exact = [0; crate::schema::MAX_EXACT_SCALAR_BYTES];
    keys::padded(
        projection
            .encode_scalar_row(values, &mut exact)
            .ok_or(Error::ForeignSchema)?,
    )
}

/// The routing of one projection's projected determinant bytes.
pub(crate) fn routing(
    inner: &StoreInner,
    compiled: &CompiledProjection,
    projected: &[u8],
) -> Result<Route> {
    match compiled.encoding {
        KeyEncoding::ExactBounded { .. } => keys::padded(projected),
        KeyEncoding::FingerprintBucket => {
            Ok(inner.fingerprinter.determinant(compiled.id, projected))
        }
    }
}

pub(crate) struct Lookup {
    pub(crate) found: Option<RowId>,
    /// The home bucket held a different row: a scalar-key competitor.
    pub(crate) conflicting: bool,
}

/// Exact membership inside one home bucket: full canonical bytes decide.
pub(crate) fn lookup(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
    home: &Route,
    row: &[u8],
    work: &WorkContext,
) -> Result<Lookup> {
    let bucket = keys::bucket(keys::row_prefix(relation)?, home);
    let mut found = None;
    let mut conflicting = false;
    for entry in inner.rows.prefix_iter(txn, &bucket).map_err(Error::from)? {
        work.checkpoint()?;
        let (key, stored) = entry.map_err(Error::from)?;
        if stored == row {
            found = Some(keys::parse(key)?.row);
            break;
        }
        conflicting = true;
    }
    work.checkpoint()?;
    Ok(Lookup { found, conflicting })
}

pub(crate) fn contains(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
    row: &[u8],
    work: &WorkContext,
) -> Result<bool> {
    let home = home_of_row(inner, relation, row, work)?;
    Ok(lookup(inner, txn, relation, &home, row, work)?
        .found
        .is_some())
}

pub(crate) fn fetch<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
    home: &Route,
    row: RowId,
) -> Result<Option<&'txn [u8]>> {
    inner
        .rows
        .get(txn, &keys::entry(keys::row_prefix(relation)?, home, row))
        .map_err(Error::from)
}

/// One relation's rows in key order: home, ordinal and canonical bytes.
pub(crate) fn scan<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
) -> Result<impl Iterator<Item = Result<(keys::Parsed, &'txn [u8])>> + use<'txn>> {
    let prefix = keys::row_prefix(relation)?;
    let range = inner.rows.prefix_iter(txn, &prefix).map_err(Error::from)?;
    Ok(range.map(|entry| {
        let (key, value) = entry.map_err(Error::from)?;
        Ok((keys::parse(key)?, value))
    }))
}

/// Visit one determinant group: the home bucket for the home projection,
/// the `det` bucket otherwise. Candidates only; callers confirm values.
pub(crate) fn visit_bucket<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    compiled: &CompiledProjection,
    routing: &Route,
    work: &WorkContext,
    visit: &mut dyn FnMut(RowId, &'txn [u8]) -> Result<bool>,
) -> Result<()> {
    if inner.det.is_home(compiled) {
        let bucket = keys::bucket(keys::row_prefix(compiled.relation)?, routing);
        for entry in inner.rows.prefix_iter(txn, &bucket).map_err(Error::from)? {
            work.checkpoint()?;
            let (key, bytes) = entry.map_err(Error::from)?;
            if !visit(keys::parse(key)?.row, bytes)? {
                return Ok(());
            }
        }
    } else {
        let bucket = keys::bucket(keys::det_prefix(compiled.id), routing);
        for entry in inner.dets.prefix_iter(txn, &bucket).map_err(Error::from)? {
            work.checkpoint()?;
            let (key, home) = entry.map_err(Error::from)?;
            let row = keys::parse(key)?.row;
            let home: &Route = home.try_into().map_err(|_| {
                Error::Corruption(CorruptionError::MalformedKey("determinant home"))
            })?;
            let bytes = fetch(inner, txn, compiled.relation, home, row)?
                .ok_or(Error::Corruption(CorruptionError::DanglingIndexEntry))?;
            if !visit(row, bytes)? {
                return Ok(());
            }
        }
    }
    work.checkpoint()?;
    Ok(())
}

/// Count one determinant group's entries without fetching rows; `None` when
/// the group holds more than `limit`.
pub(crate) fn count_bucket_bounded(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    compiled: &CompiledProjection,
    routing: &Route,
    limit: u64,
    work: &WorkContext,
) -> Result<Option<u64>> {
    let (db, prefix) = if inner.det.is_home(compiled) {
        (&inner.rows, keys::row_prefix(compiled.relation)?)
    } else {
        (&inner.dets, keys::det_prefix(compiled.id))
    };
    let bucket = keys::bucket(prefix, routing);
    let mut count = 0u64;
    for entry in db.prefix_iter(txn, &bucket).map_err(Error::from)? {
        work.checkpoint()?;
        entry.map_err(Error::from)?;
        if count == limit {
            return Ok(None);
        }
        count += 1;
    }
    work.checkpoint()?;
    Ok(Some(count))
}

/// One relation's metadata while a writer changes it.
struct Touched {
    parent: RelationMeta,
    count: u64,
}

/// Applies row mutations inside one write transaction and flushes relation
/// metadata and the row-id high-water mark at [`Self::finish`]. An error
/// leaves partial private mutations: callers propagate it and drop the
/// transaction.
pub(crate) struct RowWriter<'a, 't> {
    inner: &'a StoreInner,
    txn: &'a mut RwTxn<'t>,
    work: &'a WorkContext,
    next_id: Option<u64>,
    touched: BTreeMap<RelationId, Touched>,
    /// Sticky: no insertion met a different row in its exact home bucket,
    /// so a scalar home key satisfied by the parent still holds.
    home_keys_preserved: bool,
    decode: crate::canonical::DecodeScratch<'a>,
}

impl<'a, 't> RowWriter<'a, 't> {
    pub(crate) fn new(
        inner: &'a StoreInner,
        txn: &'a mut RwTxn<'t>,
        work: &'a WorkContext,
    ) -> Self {
        Self {
            inner,
            txn,
            work,
            next_id: None,
            touched: BTreeMap::new(),
            home_keys_preserved: true,
            decode: crate::canonical::DecodeScratch::new(work),
        }
    }

    pub(crate) fn home_keys_preserved(&self) -> bool {
        self.home_keys_preserved
    }

    fn shift_count(&mut self, relation: RelationId, delta: i64) -> Result<()> {
        let touched = match self.touched.entry(relation) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let parent = format::read_relation_meta(&self.inner.meta, self.txn, relation)?;
                entry.insert(Touched {
                    parent,
                    count: parent.count,
                })
            }
        };
        touched.count = touched
            .count
            .checked_add_signed(delta)
            .ok_or(Error::Corruption(CorruptionError::MetaMissing(
                "row count underflow",
            )))?;
        Ok(())
    }

    fn allocate(inner: &StoreInner, txn: &RwTxn<'_>, next_id: &mut Option<u64>) -> Result<RowId> {
        let next = match *next_id {
            Some(next) => next,
            None => format::read_u64(&inner.meta, txn, K_NEXT_ROW_ID, "next row id")?,
        };
        *next_id = Some(
            next.checked_add(1)
                .ok_or(Error::Exhausted(crate::error::Counter::RowIds))?,
        );
        Ok(RowId(next))
    }

    /// Insert one canonical row; `Some(ordinal)` when it was absent.
    pub(crate) fn insert(&mut self, relation: RelationId, row: &[u8]) -> Result<Option<RowId>> {
        let inner = self.inner;
        let work = self.work;
        let fields = inner.det.fields_of(relation).ok_or(Error::ForeignSchema)?;
        let has_home = inner.det.home_projection(relation).is_some();
        let Self {
            txn,
            decode,
            next_id,
            home_keys_preserved,
            ..
        } = self;
        let mut allocated = None;
        decode.with_decoded(fields, row, |values| -> Result<()> {
            let home = home_of_decoded(inner, relation, row, values)?;
            let found = lookup(inner, txn, relation, &home, row, work)?;
            if has_home {
                *home_keys_preserved &= !found.conflicting;
            }
            if found.found.is_some() {
                return Ok(());
            }
            let id = Self::allocate(inner, txn, next_id)?;
            inner
                .rows
                .put(
                    txn,
                    &keys::entry(keys::row_prefix(relation)?, &home, id),
                    row,
                )
                .map_err(|error| inner.txn_error(error))?;
            inner
                .det
                .emit_decoded(relation, values, work, &mut |compiled, projected| {
                    if inner.det.is_home(compiled) {
                        return Ok(());
                    }
                    let route = routing(inner, compiled, projected)?;
                    inner
                        .dets
                        .put(
                            txn,
                            &keys::entry(keys::det_prefix(compiled.id), &route, id),
                            &home,
                        )
                        .map_err(|error| inner.txn_error(error))
                })?;
            allocated = Some(id);
            Ok(())
        })?;
        if allocated.is_some() {
            self.shift_count(relation, 1)?;
        }
        Ok(allocated)
    }

    /// Remove one canonical row; true when it was present.
    pub(crate) fn remove(&mut self, relation: RelationId, row: &[u8]) -> Result<bool> {
        let inner = self.inner;
        let work = self.work;
        let fields = inner.det.fields_of(relation).ok_or(Error::ForeignSchema)?;
        let Self { txn, decode, .. } = self;
        let removed = decode.with_decoded(fields, row, |values| -> Result<bool> {
            let home = home_of_decoded(inner, relation, row, values)?;
            let Some(id) = lookup(inner, txn, relation, &home, row, work)?.found else {
                return Ok(false);
            };
            inner
                .rows
                .delete(txn, &keys::entry(keys::row_prefix(relation)?, &home, id))
                .map_err(|error| inner.txn_error(error))?;
            inner
                .det
                .emit_decoded(relation, values, work, &mut |compiled, projected| {
                    if inner.det.is_home(compiled) {
                        return Ok(());
                    }
                    let route = routing(inner, compiled, projected)?;
                    inner
                        .dets
                        .delete(txn, &keys::entry(keys::det_prefix(compiled.id), &route, id))
                        .map(|_| ())
                        .map_err(|error| inner.txn_error(error))
                })?;
            Ok(true)
        })?;
        if removed {
            self.shift_count(relation, -1)?;
        }
        Ok(removed)
    }

    /// Flush relation metadata and the row-id high-water mark; returns the
    /// relations whose rows changed.
    pub(crate) fn finish(self) -> Result<Vec<RelationId>> {
        let inner = self.inner;
        let mut changed = Vec::with_capacity(self.touched.len());
        for (relation, touched) in &self.touched {
            self.work.checkpoint()?;
            let meta = touched.parent.changed(touched.count)?;
            inner
                .meta
                .put(self.txn, &format::relation_key(*relation)?, &meta.encode())
                .map_err(|error| inner.txn_error(error))?;
            changed.push(*relation);
        }
        if let Some(next) = self.next_id {
            inner
                .meta
                .put(self.txn, K_NEXT_ROW_ID, &next.to_be_bytes())
                .map_err(|error| inner.txn_error(error))?;
        }
        Ok(changed)
    }
}
