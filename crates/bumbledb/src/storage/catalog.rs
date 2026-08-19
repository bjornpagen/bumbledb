//! Ordered catalog algebra: GAT-bearing map contracts over LMDB first.
//!
//! Catalog algorithms stay generic. Bounds live on the method or use
//! site. No GAT-bearing trait is converted to `dyn` (E0038 on this
//! nightly). Broad HRTBs on lending items are refused (E0597). Every
//! range GAT requires `Self: 'catalog`.

use std::marker::PhantomData;
use std::ops::Bound;

use crate::encoding::InternId;
use crate::error::Result;
use crate::storage::env::{ReadTxn, WriteTxn};
use bumbledb_theory::schema::{FieldId, RelationId};

mod complete;
mod decorate;
mod freeze;
mod frozen;
mod heap;
mod lmdb;
#[cfg(test)]
mod tests;

pub(crate) use freeze::admit_catalog;
pub(crate) use frozen::FrozenCatalog;
pub(crate) use heap::HeapStage;
pub(crate) use lmdb::{LmdbPeekCatalog, LmdbReadCatalog, LmdbSortedGets, LmdbWriteCatalog};

/// One ordered-map entry, borrowed from the catalog.
pub(crate) struct Entry<'a> {
    pub key: &'a [u8],
    pub value: &'a [u8],
}

/// Range bounds whose lifetimes are independent of the catalog borrow.
pub(crate) struct Bounds<'a> {
    pub start: Bound<&'a [u8]>,
    pub end: Bound<&'a [u8]>,
}

impl<'a> Bounds<'a> {
    /// Unbounded pair used for raw export.
    pub(crate) const fn all() -> Self {
        Self {
            start: Bound::Unbounded,
            end: Bound::Unbounded,
        }
    }

    pub(crate) const fn as_tuple(&self) -> (Bound<&'a [u8]>, Bound<&'a [u8]>) {
        (self.start, self.end)
    }
}

/// The two physical ordered maps. Not a storage backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogMap {
    Data,
    Dictionary,
}

/// Verdict of [`OrderedWrite::put_no_overwrite`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PutOutcome {
    Inserted,
    Occupied,
}

/// Lending forward cursor over [`Entry`] values.
pub(crate) trait ReadCursor {
    fn next(&mut self) -> Result<Option<Entry<'_>>>;
}

/// Mutable sibling: delete the just-yielded entry.
pub(crate) trait WriteCursor: ReadCursor {
    /// May run exactly once after a yielded entry and before the next
    /// cursor move. Any other state is a programmer-invariant violation.
    fn del_current(&mut self) -> Result<()>;
}

/// Reusable monotone exact-key walker. `reset` starts a new group;
/// `get` requires nondecreasing keys until the next reset (debug builds
/// assert that).
pub(crate) trait SortedGets {
    type Value<'a>: AsRef<[u8]>
    where
        Self: 'a;

    fn reset(&mut self);

    fn get(&mut self, key: &[u8]) -> Result<Option<Self::Value<'_>>>;
}

/// Byte-ordered map reads. Catalog algorithms are generic over this.
pub(crate) trait OrderedRead {
    type Value<'a>: AsRef<[u8]>
    where
        Self: 'a;

    type Range<'catalog, 'bounds>: ReadCursor
    where
        Self: 'catalog;

    type Gets<'a>: SortedGets
    where
        Self: 'a;

    fn get(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Self::Value<'_>>>;

    fn lower(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>>;

    fn greater(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>>;

    fn greater_or_equal(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>>;

    fn range<'catalog, 'bounds>(
        &'catalog self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<Self::Range<'catalog, 'bounds>>;

    fn sorted_gets(&self, map: CatalogMap) -> Result<Self::Gets<'_>>;

    fn len(&self, map: CatalogMap) -> Result<u64>;
}

/// Byte-ordered map writes.
pub(crate) trait OrderedWrite: OrderedRead {
    type WriteRange<'catalog, 'bounds>: WriteCursor
    where
        Self: 'catalog;

    fn put(&mut self, map: CatalogMap, key: &[u8], value: &[u8]) -> Result<()>;

    fn put_no_overwrite(&mut self, map: CatalogMap, key: &[u8], value: &[u8])
    -> Result<PutOutcome>;

    fn delete(&mut self, map: CatalogMap, key: &[u8]) -> Result<bool>;

    fn range_mut<'catalog, 'bounds>(
        &'catalog mut self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<Self::WriteRange<'catalog, 'bounds>>;
}

/// One fact in a relation scan: row id and stored bytes.
pub(crate) struct FactEntry<'a> {
    pub row: u64,
    pub bytes: &'a [u8],
}

/// Lending cursor over [`FactEntry`] values.
pub(crate) trait FactCursor {
    fn next(&mut self) -> Result<Option<FactEntry<'_>>>;
}

/// Catalog reads over facts, counters, and the dictionary.
pub(crate) trait CatalogRead: OrderedRead {
    type Facts<'a>: FactCursor
    where
        Self: 'a;

    fn scan_facts(&self, relation: RelationId) -> Result<Self::Facts<'_>>;

    fn fetch_fact(&self, relation: RelationId, row: u64) -> Result<Option<Self::Value<'_>>> {
        self.get(
            CatalogMap::Data,
            &crate::storage::keys::fact_key(relation, row),
        )
    }

    fn membership_row(&self, relation: RelationId, hash: &[u8; 32]) -> Result<Option<u64>> {
        stored_row_id(self.get(
            CatalogMap::Data,
            &crate::storage::keys::membership_key(relation, hash),
        )?)
    }

    fn determinant_row(&self, key: &[u8]) -> Result<Option<u64>> {
        stored_row_id(self.get(CatalogMap::Data, key)?)
    }

    fn row_count(&self, relation: RelationId) -> Result<u64> {
        stored_counter(
            self.get(
                CatalogMap::Data,
                &crate::storage::keys::stat_key(relation, crate::storage::keys::StatKind::RowCount),
            )?,
            "S row count",
        )
    }

    fn row_id_high_water(&self, relation: RelationId) -> Result<u64> {
        stored_counter(
            self.get(
                CatalogMap::Data,
                &crate::storage::keys::stat_key(
                    relation,
                    crate::storage::keys::StatKind::RowIdHighWater,
                ),
            )?,
            "S row-id high-water",
        )
    }

    fn fresh_next(&self, relation: RelationId, field: FieldId) -> Result<u64> {
        stored_counter(
            self.get(
                CatalogMap::Data,
                &crate::storage::keys::fresh_key(relation, field),
            )?,
            "Q fresh next",
        )
    }

    fn dict_lookup(&self, raw: &[u8]) -> Result<Option<InternId>> {
        match self.get(
            CatalogMap::Dictionary,
            &crate::storage::dict::forward_key(raw),
        )? {
            None => Ok(None),
            Some(bytes) => crate::storage::dict::intern_id_from_stored(bytes.as_ref()).map(Some),
        }
    }

    fn dict_resolve(&self, id: InternId) -> Result<Self::Value<'_>> {
        self.get(
            CatalogMap::Dictionary,
            &crate::storage::dict::reverse_key(id),
        )?
        .ok_or(crate::error::Error::Corruption(
            crate::error::CorruptionError::DanglingInternId(id),
        ))
    }

    fn dict_next_id(&self) -> Result<InternId>;
}

/// Catalog writes for counters and the dictionary next-id.
pub(crate) trait CatalogWrite: CatalogRead + OrderedWrite {
    fn set_row_count(&mut self, relation: RelationId, value: u64) -> Result<()> {
        self.put(
            CatalogMap::Data,
            &crate::storage::keys::stat_key(relation, crate::storage::keys::StatKind::RowCount),
            &value.to_le_bytes(),
        )
    }

    fn set_row_id_high_water(&mut self, relation: RelationId, value: u64) -> Result<()> {
        self.put(
            CatalogMap::Data,
            &crate::storage::keys::stat_key(
                relation,
                crate::storage::keys::StatKind::RowIdHighWater,
            ),
            &value.to_le_bytes(),
        )
    }

    fn set_fresh_next(&mut self, relation: RelationId, field: FieldId, value: u64) -> Result<()> {
        self.put(
            CatalogMap::Data,
            &crate::storage::keys::fresh_key(relation, field),
            &value.to_le_bytes(),
        )
    }

    fn set_dict_next_id(&mut self, value: InternId) -> Result<()>;
}

fn stored_row_id<V: AsRef<[u8]>>(value: Option<V>) -> Result<Option<u64>> {
    value
        .map(|bytes| crate::storage::stored_u64(bytes.as_ref(), "M/U row id"))
        .transpose()
}

fn stored_counter<V: AsRef<[u8]>>(value: Option<V>, what: &'static str) -> Result<u64> {
    match value {
        Some(bytes) => crate::storage::stored_u64(bytes.as_ref(), what),
        None => Ok(0),
    }
}

/// GAT / lending probes that must stay compiling on the pinned nightly:
/// associated type projections, lending range cursors, and mutable
/// deletion cursors. Broad HRTBs on lending items are not written.
#[allow(dead_code)]
fn _catalog_read_gats<C: OrderedRead>(catalog: &C) -> Result<()> {
    let _ = catalog.get(CatalogMap::Data, b"")?;
    let mut range = catalog.range(CatalogMap::Data, Bounds::all())?;
    let _ = ReadCursor::next(&mut range)?;
    Ok(())
}

#[allow(dead_code)]
fn _catalog_write_gats<C: OrderedWrite>(catalog: &mut C) -> Result<()> {
    let mut writes = catalog.range_mut(CatalogMap::Data, Bounds::all())?;
    if ReadCursor::next(&mut writes)?.is_some() {
        WriteCursor::del_current(&mut writes)?;
    }
    Ok(())
}

impl<'env> ReadTxn<'env> {
    pub(crate) fn catalog(&self) -> LmdbReadCatalog<'_, 'env> {
        LmdbReadCatalog::new(self)
    }
}

impl<'env> WriteTxn<'env> {
    pub(crate) fn catalog(&mut self) -> LmdbWriteCatalog<'_, 'env> {
        LmdbWriteCatalog::new(self)
    }
}

/// Marker so GAT associated types can name an unused bounds lifetime.
pub(crate) type UnusedBounds<'bounds> = PhantomData<&'bounds [u8]>;
