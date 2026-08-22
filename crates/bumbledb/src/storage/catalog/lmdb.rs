use heed::types::Bytes;
use heed::{AnyTls, Database, RoRange, RoTxn, RwRange};

use super::{
    Bounds, CatalogMap, CatalogRead, CatalogWrite, Entry, FactCursor, FactEntry, OrderedRead,
    OrderedWrite, PutOutcome, ReadCursor, SortedGets, UnusedBounds, WriteCursor,
};
use crate::encoding::InternId;
use crate::error::{CorruptionError, Error, Result};
use crate::storage::env::{ReadTxn, WriteTxn};
use crate::storage::keys;
use bumbledb_theory::schema::RelationId;

#[derive(Clone, Copy)]
pub(crate) struct LmdbReadCatalog<'txn, 'env> {
    txn: &'txn ReadTxn<'env>,
}

impl<'txn, 'env> LmdbReadCatalog<'txn, 'env> {
    pub(crate) fn new(txn: &'txn ReadTxn<'env>) -> Self {
        Self { txn }
    }

    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "UFCS from CatalogRead passes &self; the handle is Copy but the trait shape is a borrow"
    )]
    pub(crate) fn dict_lookup(&self, raw: &[u8]) -> Result<Option<InternId>> {
        crate::storage::dict::lookup(self.txn, raw)
    }

    /// borrow of this `Copy` handle — store `CodecRead` can drop the

    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "UFCS from CatalogRead passes &self; the handle is Copy but the trait shape is a borrow"
    )]
    pub(crate) fn dict_resolve(&self, id: InternId) -> Result<&'txn [u8]> {
        self.txn
            .env()
            .dict()
            .get(self.txn.raw(), &crate::storage::dict::reverse_key(id))?
            .ok_or(Error::Corruption(CorruptionError::DanglingInternId(id)))
    }
}

pub(crate) struct LmdbWriteCatalog<'txn, 'env> {
    txn: &'txn mut WriteTxn<'env>,
}

impl<'txn, 'env> LmdbWriteCatalog<'txn, 'env> {
    pub(crate) fn new(txn: &'txn mut WriteTxn<'env>) -> Self {
        Self { txn }
    }

    pub(crate) fn dict_put_pending(&mut self, raw: &[u8], id: InternId) -> Result<()> {
        debug_assert!(!id.is_sentinel(), "dictionary id space exhausted");
        self.put(
            CatalogMap::Dictionary,
            &crate::storage::dict::forward_key(raw),
            &id.raw().to_be_bytes(),
        )?;
        match self.put_no_overwrite(
            CatalogMap::Dictionary,
            &crate::storage::dict::reverse_key(id),
            raw,
        )? {
            PutOutcome::Inserted => Ok(()),
            PutOutcome::Occupied => Err(Error::Corruption(CorruptionError::DictReverseIdReuse)),
        }
    }
}

fn map_db(env: &crate::storage::env::Environment, map: CatalogMap) -> Database<Bytes, Bytes> {
    match map {
        CatalogMap::Data => env.data(),
        CatalogMap::Dictionary => env.dict(),
    }
}

fn entry_of<'a>(pair: Option<(&'a [u8], &'a [u8])>) -> Option<Entry<'a>> {
    pair.map(|(key, value)| Entry { key, value })
}

pub(crate) struct LmdbRange<'catalog, 'bounds> {
    inner: RoRange<'catalog, Bytes, Bytes>,
    _bounds: UnusedBounds<'bounds>,
}

impl ReadCursor for LmdbRange<'_, '_> {
    fn next(&mut self) -> Result<Option<Entry<'_>>> {
        match self.inner.next() {
            None => Ok(None),
            Some(Ok((key, value))) => Ok(Some(Entry { key, value })),
            Some(Err(err)) => Err(err.into()),
        }
    }
}

enum Yielded {
    None,
    Live,
    Deleted,
}

pub(crate) struct LmdbWriteRange<'catalog, 'bounds> {
    inner: RwRange<'catalog, Bytes, Bytes>,
    yielded: Yielded,
    _bounds: UnusedBounds<'bounds>,
}

impl ReadCursor for LmdbWriteRange<'_, '_> {
    fn next(&mut self) -> Result<Option<Entry<'_>>> {
        self.yielded = Yielded::None;
        match self.inner.next() {
            None => Ok(None),
            Some(Ok((key, value))) => {
                self.yielded = Yielded::Live;
                Ok(Some(Entry { key, value }))
            }
            Some(Err(err)) => Err(err.into()),
        }
    }
}

impl WriteCursor for LmdbWriteRange<'_, '_> {
    fn del_current(&mut self) -> Result<()> {
        match self.yielded {
            Yielded::Live => {
                #[expect(
                    unsafe_code,
                    reason = "heed's cursor delete is the foreign contract; no database borrow survives"
                )]
                unsafe {
                    self.inner.del_current()?;
                }
                self.yielded = Yielded::Deleted;
                Ok(())
            }
            Yielded::None | Yielded::Deleted => {
                panic!(
                    "del_current may run exactly once after a yielded entry and before the next cursor move"
                )
            }
        }
    }
}

enum Walk<'a> {
    Idle,
    Open {
        range: RoRange<'a, Bytes, Bytes>,
        pending: Option<(&'a [u8], &'a [u8])>,
    },
    Drained,
}

pub(crate) struct LmdbSortedGets<'a> {
    txn: &'a RoTxn<'a, AnyTls>,
    data: Database<Bytes, Bytes>,
    walk: Walk<'a>,
    #[cfg(debug_assertions)]
    last: Option<Vec<u8>>,
}

impl<'a> LmdbSortedGets<'a> {
    const WALK_BUDGET: usize = 16;

    pub(crate) fn new(txn: &'a RoTxn<'a, AnyTls>, data: Database<Bytes, Bytes>) -> Self {
        Self {
            txn,
            data,
            walk: Walk::Idle,
            #[cfg(debug_assertions)]
            last: None,
        }
    }
}

impl SortedGets for LmdbSortedGets<'_> {
    type Value<'a>
        = &'a [u8]
    where
        Self: 'a;

    fn reset(&mut self) {
        self.walk = Walk::Idle;
        #[cfg(debug_assertions)]
        {
            self.last = None;
        }
    }

    fn get(&mut self, key: &[u8]) -> Result<Option<&[u8]>> {
        #[cfg(debug_assertions)]
        {
            if let Some(last) = &self.last {
                debug_assert!(
                    key >= last.as_slice(),
                    "SortedGets::get requires nondecreasing keys until reset"
                );
            }
            self.last = Some(key.to_vec());
        }
        for _ in 0..Self::WALK_BUDGET {
            match &self.walk {
                Walk::Drained => return Ok(None),
                Walk::Open {
                    pending: Some((k, v)),
                    ..
                } => match (*k).cmp(key) {
                    std::cmp::Ordering::Equal => return Ok(Some(*v)),
                    std::cmp::Ordering::Greater => return Ok(None),
                    std::cmp::Ordering::Less => {}
                },
                Walk::Idle => break,
                Walk::Open { pending: None, .. } => {}
            }
            let mut drained = false;
            if let Walk::Open { range, pending } = &mut self.walk {
                *pending = None;
                match range.next().transpose()? {
                    Some(entry) => *pending = Some(entry),
                    None => drained = true,
                }
            } else {
                break;
            }
            if drained {
                self.walk = Walk::Drained;
                return Ok(None);
            }
        }
        let bounds: (std::ops::Bound<&[u8]>, std::ops::Bound<&[u8]>) =
            (std::ops::Bound::Included(key), std::ops::Bound::Unbounded);
        let mut range = self.data.range(self.txn, &bounds)?;
        let pending = range.next().transpose()?;
        let hit = match pending {
            Some((k, v)) if k == key => Some(v),
            _ => None,
        };
        self.walk = Walk::Open { range, pending };
        Ok(hit)
    }
}

pub(crate) struct LmdbFactCursor<'a> {
    inner: heed::RoPrefix<'a, Bytes, Bytes>,
}

impl FactCursor for LmdbFactCursor<'_> {
    fn next(&mut self) -> Result<Option<FactEntry<'_>>> {
        match self.inner.next() {
            None => Ok(None),
            Some(Err(err)) => Err(err.into()),
            Some(Ok((raw_key, bytes))) => {
                let (_, row) = keys::parse_fact_key(raw_key).ok_or(Error::Corruption(
                    CorruptionError::MalformedValue("F key length"),
                ))?;
                Ok(Some(FactEntry { row, bytes }))
            }
        }
    }
}

impl OrderedRead for LmdbReadCatalog<'_, '_> {
    type Value<'a>
        = &'a [u8]
    where
        Self: 'a;

    type Range<'catalog, 'bounds>
        = LmdbRange<'catalog, 'bounds>
    where
        Self: 'catalog;

    type Gets<'a>
        = LmdbSortedGets<'a>
    where
        Self: 'a;

    fn get(&self, map: CatalogMap, key: &[u8]) -> Result<Option<&[u8]>> {
        Ok(map_db(self.txn.env(), map).get(self.txn.raw(), key)?)
    }

    fn lower(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_lower_than(self.txn.raw(), key)?,
        ))
    }

    fn greater(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_greater_than(self.txn.raw(), key)?,
        ))
    }

    fn greater_or_equal(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_greater_than_or_equal_to(self.txn.raw(), key)?,
        ))
    }

    fn range<'catalog, 'bounds>(
        &'catalog self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<LmdbRange<'catalog, 'bounds>> {
        Ok(LmdbRange {
            inner: map_db(self.txn.env(), map).range(self.txn.raw(), &bounds.as_tuple())?,
            _bounds: std::marker::PhantomData,
        })
    }

    fn sorted_gets(&self, map: CatalogMap) -> Result<LmdbSortedGets<'_>> {
        Ok(LmdbSortedGets::new(
            self.txn.raw(),
            map_db(self.txn.env(), map),
        ))
    }

    fn len(&self, map: CatalogMap) -> Result<u64> {
        Ok(map_db(self.txn.env(), map).len(self.txn.raw())?)
    }
}

impl CatalogRead for LmdbReadCatalog<'_, '_> {
    type Facts<'a>
        = LmdbFactCursor<'a>
    where
        Self: 'a;

    fn scan_facts(&self, relation: RelationId) -> Result<LmdbFactCursor<'_>> {
        let mut buf = [0u8; keys::MAX_KEY];
        let prefix = keys::fact_prefix(&mut buf, relation);
        Ok(LmdbFactCursor {
            inner: self.txn.env().data().prefix_iter(self.txn.raw(), prefix)?,
        })
    }

    fn dict_next_id(&self) -> Result<InternId> {
        Ok(InternId::from_raw(self.txn.dict_next_id()?))
    }

    fn dict_lookup(&self, raw: &[u8]) -> Result<Option<InternId>> {
        LmdbReadCatalog::dict_lookup(self, raw)
    }

    fn dict_resolve(&self, id: InternId) -> Result<Self::Value<'_>> {
        LmdbReadCatalog::dict_resolve(self, id)
    }
}

impl OrderedRead for LmdbWriteCatalog<'_, '_> {
    type Value<'a>
        = &'a [u8]
    where
        Self: 'a;

    type Range<'catalog, 'bounds>
        = LmdbRange<'catalog, 'bounds>
    where
        Self: 'catalog;

    type Gets<'a>
        = LmdbSortedGets<'a>
    where
        Self: 'a;

    fn get(&self, map: CatalogMap, key: &[u8]) -> Result<Option<&[u8]>> {
        Ok(map_db(self.txn.env(), map).get(self.txn.raw(), key)?)
    }

    fn lower(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_lower_than(self.txn.raw(), key)?,
        ))
    }

    fn greater(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_greater_than(self.txn.raw(), key)?,
        ))
    }

    fn greater_or_equal(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_greater_than_or_equal_to(self.txn.raw(), key)?,
        ))
    }

    fn range<'catalog, 'bounds>(
        &'catalog self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<LmdbRange<'catalog, 'bounds>> {
        Ok(LmdbRange {
            inner: map_db(self.txn.env(), map).range(self.txn.raw(), &bounds.as_tuple())?,
            _bounds: std::marker::PhantomData,
        })
    }

    fn sorted_gets(&self, map: CatalogMap) -> Result<LmdbSortedGets<'_>> {
        Ok(LmdbSortedGets::new(
            self.txn.raw(),
            map_db(self.txn.env(), map),
        ))
    }

    fn len(&self, map: CatalogMap) -> Result<u64> {
        Ok(map_db(self.txn.env(), map).len(self.txn.raw())?)
    }
}

impl OrderedWrite for LmdbWriteCatalog<'_, '_> {
    type WriteRange<'catalog, 'bounds>
        = LmdbWriteRange<'catalog, 'bounds>
    where
        Self: 'catalog;

    fn put(&mut self, map: CatalogMap, key: &[u8], value: &[u8]) -> Result<()> {
        map_db(self.txn.env(), map).put(self.txn.raw_mut(), key, value)?;
        Ok(())
    }

    fn put_no_overwrite(
        &mut self,
        map: CatalogMap,
        key: &[u8],
        value: &[u8],
    ) -> Result<PutOutcome> {
        match map_db(self.txn.env(), map).put_with_flags(
            self.txn.raw_mut(),
            heed::PutFlags::NO_OVERWRITE,
            key,
            value,
        ) {
            Ok(()) => Ok(PutOutcome::Inserted),
            Err(heed::Error::Mdb(heed::MdbError::KeyExist)) => Ok(PutOutcome::Occupied),
            Err(other) => Err(other.into()),
        }
    }

    fn delete(&mut self, map: CatalogMap, key: &[u8]) -> Result<bool> {
        Ok(map_db(self.txn.env(), map).delete(self.txn.raw_mut(), key)?)
    }

    fn range_mut<'catalog, 'bounds>(
        &'catalog mut self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<LmdbWriteRange<'catalog, 'bounds>> {
        let db = map_db(self.txn.env(), map);
        Ok(LmdbWriteRange {
            inner: db.range_mut(self.txn.raw_mut(), &bounds.as_tuple())?,
            yielded: Yielded::None,
            _bounds: std::marker::PhantomData,
        })
    }
}

impl CatalogRead for LmdbWriteCatalog<'_, '_> {
    type Facts<'a>
        = LmdbFactCursor<'a>
    where
        Self: 'a;

    fn scan_facts(&self, relation: RelationId) -> Result<LmdbFactCursor<'_>> {
        let mut buf = [0u8; keys::MAX_KEY];
        let prefix = keys::fact_prefix(&mut buf, relation);
        Ok(LmdbFactCursor {
            inner: self.txn.env().data().prefix_iter(self.txn.raw(), prefix)?,
        })
    }

    fn dict_next_id(&self) -> Result<InternId> {
        Ok(InternId::from_raw(self.txn.dict_next_id()?))
    }
}

impl CatalogWrite for LmdbWriteCatalog<'_, '_> {
    fn set_dict_next_id(&mut self, value: InternId) -> Result<()> {
        debug_assert!(
            !value.is_sentinel(),
            "dictionary id space exhausted (u64::MAX is the miss sentinel)"
        );
        self.txn.put_dict_next_id(value.raw())
    }
}

/// Incremental judgment only reads; it must not take the mutable
/// [`LmdbWriteCatalog`] owner borrow.
#[derive(Clone, Copy)]
pub(crate) struct LmdbPeekCatalog<'txn, 'env> {
    txn: &'txn WriteTxn<'env>,
}

impl<'txn, 'env> LmdbPeekCatalog<'txn, 'env> {
    pub(crate) fn new(txn: &'txn WriteTxn<'env>) -> Self {
        Self { txn }
    }
}

impl OrderedRead for LmdbPeekCatalog<'_, '_> {
    type Value<'a>
        = &'a [u8]
    where
        Self: 'a;

    type Range<'catalog, 'bounds>
        = LmdbRange<'catalog, 'bounds>
    where
        Self: 'catalog;

    type Gets<'a>
        = LmdbSortedGets<'a>
    where
        Self: 'a;

    fn get(&self, map: CatalogMap, key: &[u8]) -> Result<Option<&[u8]>> {
        Ok(map_db(self.txn.env(), map).get(self.txn.raw(), key)?)
    }

    fn lower(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_lower_than(self.txn.raw(), key)?,
        ))
    }

    fn greater(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_greater_than(self.txn.raw(), key)?,
        ))
    }

    fn greater_or_equal(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(entry_of(
            map_db(self.txn.env(), map).get_greater_than_or_equal_to(self.txn.raw(), key)?,
        ))
    }

    fn range<'catalog, 'bounds>(
        &'catalog self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<LmdbRange<'catalog, 'bounds>> {
        Ok(LmdbRange {
            inner: map_db(self.txn.env(), map).range(self.txn.raw(), &bounds.as_tuple())?,
            _bounds: std::marker::PhantomData,
        })
    }

    fn sorted_gets(&self, map: CatalogMap) -> Result<LmdbSortedGets<'_>> {
        Ok(LmdbSortedGets::new(
            self.txn.raw(),
            map_db(self.txn.env(), map),
        ))
    }

    fn len(&self, map: CatalogMap) -> Result<u64> {
        Ok(map_db(self.txn.env(), map).len(self.txn.raw())?)
    }
}

impl CatalogRead for LmdbPeekCatalog<'_, '_> {
    type Facts<'a>
        = LmdbFactCursor<'a>
    where
        Self: 'a;

    fn scan_facts(&self, relation: RelationId) -> Result<LmdbFactCursor<'_>> {
        let mut buf = [0u8; keys::MAX_KEY];
        let prefix = keys::fact_prefix(&mut buf, relation);
        Ok(LmdbFactCursor {
            inner: self.txn.env().data().prefix_iter(self.txn.raw(), prefix)?,
        })
    }

    fn dict_next_id(&self) -> Result<InternId> {
        Ok(InternId::from_raw(self.txn.dict_next_id()?))
    }
}
