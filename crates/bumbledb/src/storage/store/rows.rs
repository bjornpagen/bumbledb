//! Row/index primitives shared by the candidate write path and owned
//! snapshots: exact-checked bucket lookup, insert/remove with symmetric
//! index maintenance, bounded visits.
//!
//! Equality is always full canonical bytes. Routing bytes (exact scalar group
//! or fingerprint) only narrow the candidate bucket.

use bumbledb_theory::schema::RelationId;
use heed::{RoTxn, RwTxn};

use super::candidate::RowIndexer;
use super::det_index;
use super::error::{StoreCorruption, StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use super::format::{self, K_NEXT_ROW_ID, RowId};
use super::keys;
use super::store_env::{StoreInner, map_txn_error};
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
    row: RowId,
) -> StoreResult<Option<&'txn [u8]>> {
    inner
        .data
        .get(txn, keys::row_key(relation, row).as_slice())
        .map_err(StoreError::from_heed)
}

/// Exact membership: walk the fingerprint bucket, compare full canonical bytes.
pub(crate) fn exact_lookup(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
    row: &[u8],
    work: &WorkContext,
) -> StoreResult<Option<RowId>> {
    let fp = inner.fingerprinter.row(relation, row);
    let bucket = keys::membership_bucket(relation, &fp);
    let range = inner
        .data
        .prefix_iter(txn, bucket.as_slice())
        .map_err(StoreError::from_heed)?;
    for entry in range {
        work.step(1)?;
        let (key, _) = entry.map_err(StoreError::from_heed)?;
        let candidate = keys::row_id_from_suffix(key, keys::MEMBERSHIP_KEY_LEN)?;
        let stored = fetch_row(inner, txn, relation, candidate)?
            .ok_or(StoreError::Corruption(StoreCorruption::DanglingIndexEntry))?;
        if chunked_eq(stored, row, work)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

/// Routing bytes for one compiled projection's projected canonical bytes.
fn routing_bytes(
    inner: &StoreInner,
    projection: ProjectionId,
    projected: &[u8],
    encoding: KeyEncoding,
) -> StoreResult<Vec<u8>> {
    match encoding {
        KeyEncoding::ExactBounded { .. } => Ok(projected.to_vec()),
        KeyEncoding::FingerprintBucket => Ok(det_index::fingerprint_routing(
            inner.fingerprinter,
            projection,
            projected,
        )
        .to_vec()),
    }
}

struct DeterminantEntry {
    projection: ProjectionId,
    routing: Vec<u8>,
    tail: Option<Vec<u8>>,
}

fn determinant_entries<I: RowIndexer + ?Sized>(
    inner: &StoreInner,
    indexer: &I,
    relation: RelationId,
    row: &[u8],
    work: &WorkContext,
) -> StoreResult<Vec<DeterminantEntry>> {
    let mut entries = Vec::new();
    inner
        .det
        .emit_row(relation, row, work, &mut |projection, projected, tail| {
            work.step(1)?;
            let compiled = inner
                .det
                .projection(projection)
                .ok_or(StoreError::ForeignSchema)?;
            let routing = routing_bytes(inner, projection, projected, compiled.encoding)?;
            entries.push(DeterminantEntry {
                projection,
                routing,
                tail: tail.map(ToOwned::to_owned),
            });
            Ok(())
        })?;
    indexer.index_row(relation, row, work, &mut |projection, projected, tail| {
        work.step(1)?;
        let compiled = inner.det.projection(projection);
        let encoding = compiled.map_or(KeyEncoding::FingerprintBucket, |item| item.encoding);
        let routing = routing_bytes(inner, projection, projected, encoding)?;
        entries.push(DeterminantEntry {
            projection,
            routing,
            tail: tail.map(ToOwned::to_owned),
        });
        Ok(())
    })?;
    Ok(entries)
}

/// Bounded visitor over one determinant bucket — one row at a time, no
/// materialized id or decoded-row collection (CORE-002).
pub(crate) fn visit_determinant_bucket(
    inner: &StoreInner,
    txn: &RoTxn<'_, heed::AnyTls>,
    projection: ProjectionId,
    routing: &[u8],
    work: &WorkContext,
    visit: &mut dyn FnMut(RowId) -> StoreResult<bool>,
) -> StoreResult<()> {
    let bucket = keys::determinant_bucket(projection, routing);
    let range = inner
        .data
        .prefix_iter(txn, bucket.as_slice())
        .map_err(StoreError::from_heed)?;
    for entry in range {
        work.step(1)?;
        let (key, _) = entry.map_err(StoreError::from_heed)?;
        let id = keys::row_id_from_determinant_key(key)?;
        if !visit(id)? {
            break;
        }
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
) -> StoreResult<Vec<RowId>> {
    let compiled = inner
        .det
        .projection(projection)
        .ok_or(StoreError::ForeignSchema)?;
    let routing = routing_bytes(inner, projection, projected, compiled.encoding)?;
    let mut ids = Vec::new();
    visit_determinant_bucket(inner, txn, projection, &routing, work, &mut |id| {
        ids.push(id);
        Ok(true)
    })?;
    Ok(ids)
}

fn next_row_id(inner: &StoreInner, txn: &mut RwTxn<'_>) -> StoreResult<RowId> {
    let next = format::read_u64(&inner.meta, txn, K_NEXT_ROW_ID, "next row id")?;
    let bumped = next.checked_add(1).ok_or(StoreError::RowIdExhausted)?;
    inner
        .meta
        .put(txn, K_NEXT_ROW_ID, &bumped.to_be_bytes())
        .map_err(map_txn_error)?;
    Ok(RowId(next))
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

fn shift_row_count(
    inner: &StoreInner,
    txn: &mut RwTxn<'_>,
    relation: RelationId,
    delta: i64,
) -> StoreResult<()> {
    let current = row_count(inner, txn, relation)?;
    let next = current
        .checked_add_signed(delta)
        .ok_or(StoreError::Corruption(StoreCorruption::MetaMissing(
            "row count underflow",
        )))?;
    inner
        .meta
        .put(
            txn,
            format::row_count_key(relation).as_slice(),
            &next.to_be_bytes(),
        )
        .map_err(map_txn_error)?;
    Ok(())
}

pub(crate) fn insert_row<I: RowIndexer + ?Sized>(
    inner: &StoreInner,
    txn: &mut RwTxn<'_>,
    relation: RelationId,
    row: &[u8],
    indexer: &I,
    work: &WorkContext,
) -> StoreResult<Option<RowId>> {
    if exact_lookup(inner, txn, relation, row, work)?.is_some() {
        return Ok(None);
    }
    let entries = determinant_entries(inner, indexer, relation, row, work)?;
    let id = next_row_id(inner, txn)?;
    work.step(row.len() as u64)?;
    inner
        .data
        .put(txn, keys::row_key(relation, id).as_slice(), row)
        .map_err(map_txn_error)?;
    let fp = inner.fingerprinter.row(relation, row);
    inner
        .data
        .put(txn, keys::membership_key(relation, &fp, id).as_slice(), &[])
        .map_err(map_txn_error)?;
    for entry in entries {
        work.step(1)?;
        inner
            .data
            .put(
                txn,
                keys::determinant_key(
                    entry.projection,
                    &entry.routing,
                    entry.tail.as_deref(),
                    id,
                )
                .as_slice(),
                &[],
            )
            .map_err(map_txn_error)?;
    }
    shift_row_count(inner, txn, relation, 1)?;
    Ok(Some(id))
}

pub(crate) fn remove_row<I: RowIndexer + ?Sized>(
    inner: &StoreInner,
    txn: &mut RwTxn<'_>,
    relation: RelationId,
    row: &[u8],
    indexer: &I,
    work: &WorkContext,
) -> StoreResult<bool> {
    let Some(id) = exact_lookup(inner, txn, relation, row, work)? else {
        return Ok(false);
    };
    let entries = determinant_entries(inner, indexer, relation, row, work)?;
    inner
        .data
        .delete(txn, keys::row_key(relation, id).as_slice())
        .map_err(map_txn_error)?;
    let fp = inner.fingerprinter.row(relation, row);
    inner
        .data
        .delete(txn, keys::membership_key(relation, &fp, id).as_slice())
        .map_err(map_txn_error)?;
    for entry in entries {
        work.step(1)?;
        inner
            .data
            .delete(
                txn,
                keys::determinant_key(
                    entry.projection,
                    &entry.routing,
                    entry.tail.as_deref(),
                    id,
                )
                .as_slice(),
            )
            .map_err(map_txn_error)?;
    }
    shift_row_count(inner, txn, relation, -1)?;
    Ok(true)
}

pub(crate) fn scan_rows<'txn>(
    inner: &StoreInner,
    txn: &'txn RoTxn<'_, heed::AnyTls>,
    relation: RelationId,
) -> StoreResult<impl Iterator<Item = StoreResult<(RowId, &'txn [u8])>>> {
    let prefix = keys::row_prefix(relation);
    let range = inner
        .data
        .prefix_iter(txn, prefix.as_slice())
        .map_err(StoreError::from_heed)?;
    Ok(range.map(|entry| {
        let (key, value) = entry.map_err(StoreError::from_heed)?;
        Ok((keys::row_id_from_row_key(key)?, value))
    }))
}

/// Routing bytes for one interned projection's projected canonical bytes.
pub(crate) fn routing_for_projected(
    inner: &StoreInner,
    projection: ProjectionId,
    projected: &[u8],
) -> StoreResult<Vec<u8>> {
    let compiled = inner
        .det
        .projection(projection)
        .ok_or(StoreError::ForeignSchema)?;
    routing_bytes(inner, projection, projected, compiled.encoding)
}
