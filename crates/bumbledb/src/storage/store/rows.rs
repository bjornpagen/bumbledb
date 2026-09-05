//! Row/index primitives shared by the candidate write path and owned
//! snapshots: exact-checked bucket lookup, insert/remove with symmetric
//! index maintenance, bounded scans.
//!
//! Equality is always full canonical bytes. A fingerprint (forced-constant
//! in collision tests) only narrows the candidate bucket; a collision adds
//! lookup work, never merges two facts.

use bumbledb_theory::schema::{RelationId, StatementId};
use heed::{RoTxn, RwTxn};

use super::candidate::RowIndexer;
use super::error::{StoreCorruption, StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use super::format::{self, K_NEXT_ROW_ID, RowId};
use super::keys;
use super::store_env::{StoreInner, map_txn_error};
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

/// Exact membership: walk the fingerprint bucket, compare full canonical
/// bytes against every candidate.
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

fn determinant_entries<I: RowIndexer + ?Sized>(
    inner: &StoreInner,
    indexer: &I,
    relation: RelationId,
    row: &[u8],
    work: &WorkContext,
) -> StoreResult<Vec<(StatementId, [u8; FP_LEN])>> {
    let mut entries = Vec::new();
    indexer.index_row(relation, row, work, &mut |statement, projected| {
        work.step(1)?;
        entries.push((
            statement,
            inner.fingerprinter.determinant(statement, projected),
        ));
        Ok(())
    })?;
    Ok(entries)
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

/// Insert one canonical row: `None` when the exact fact is already present
/// (an ordinary no-op), otherwise the freshly allocated local row id. Writes
/// the row value, its membership entry, and every indexer-declared
/// determinant entry in the same transaction.
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
    for (statement, det_fp) in entries {
        work.step(1)?;
        inner
            .data
            .put(
                txn,
                keys::determinant_key(statement, &det_fp, id).as_slice(),
                &[],
            )
            .map_err(map_txn_error)?;
    }
    shift_row_count(inner, txn, relation, 1)?;
    Ok(Some(id))
}

/// Remove one canonical row by exact bytes: `false` when absent (an
/// ordinary no-op). Removes the row, its membership entry, and every
/// indexer-declared determinant entry symmetrically. The caller's bytes are
/// canonical and equal to the stored bytes, so indexing runs on them.
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
    for (statement, det_fp) in entries {
        work.step(1)?;
        inner
            .data
            .delete(
                txn,
                keys::determinant_key(statement, &det_fp, id).as_slice(),
            )
            .map_err(map_txn_error)?;
    }
    shift_row_count(inner, txn, relation, -1)?;
    Ok(true)
}

/// Bounded scan of one relation's rows in local row-id order.
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
