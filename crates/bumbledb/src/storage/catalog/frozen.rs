//! Packed heap catalog: one [`FrozenMap`] per physical map, no per-entry
//! nodes. Keys are strictly increasing. Binary search parses only the
//! compared record headers.

use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::Bound;

use super::{
    Bounds, CatalogMap, CatalogRead, Entry, FactCursor, FactEntry, OrderedRead, ReadCursor,
    SortedGets, UnusedBounds,
};
use crate::encoding::InternId;
use crate::error::{CorruptionError, Error, Result};
use crate::storage::keys;
use bumbledb_theory::schema::RelationId;

const KEY_LEN_SIZE: usize = 4;
const VALUE_LEN_SIZE: usize = 8;
const HEADER_SIZE: usize = KEY_LEN_SIZE + VALUE_LEN_SIZE;

/// One packed ordered map. Each record is
/// `key_len: u32 | value_len: u64 | key bytes | value bytes`.
/// `offsets[i]` locates record `i`.
pub(crate) struct FrozenMap {
    records: Box<[u8]>,
    offsets: Box<[u64]>,
}

impl FrozenMap {
    /// Packs already-sorted unique keys. Caller proves order and
    /// uniqueness — merge is the one site that walks neighbors.
    pub(crate) fn pack(
        entries: impl IntoIterator<Item = (impl AsRef<[u8]>, impl AsRef<[u8]>)>,
    ) -> Self {
        let entries: Vec<(Vec<u8>, Vec<u8>)> = entries
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_vec(), v.as_ref().to_vec()))
            .collect();
        Self::pack_slices(entries.iter().map(|(k, v)| (k.as_slice(), v.as_slice())))
    }

    pub(crate) fn pack_slices<'a>(entries: impl IntoIterator<Item = (&'a [u8], &'a [u8])>) -> Self {
        let mut records = Vec::new();
        let mut offsets = Vec::new();
        for (key, value) in entries {
            let key_len = u32::try_from(key.len()).expect("key fits u32");
            let value_len = u64::try_from(value.len()).expect("value fits u64");
            let offset = u64::try_from(records.len()).expect("catalog fits u64");
            offsets.push(offset);
            records.extend_from_slice(&key_len.to_le_bytes());
            records.extend_from_slice(&value_len.to_le_bytes());
            records.extend_from_slice(key);
            records.extend_from_slice(value);
        }
        Self {
            records: records.into_boxed_slice(),
            offsets: offsets.into_boxed_slice(),
        }
    }

    #[must_use]
    pub(crate) fn empty() -> Self {
        Self {
            records: Box::new([]),
            offsets: Box::new([]),
        }
    }

    #[must_use]
    pub(crate) fn len(&self) -> u64 {
        u64::try_from(self.offsets.len()).expect("len fits u64")
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    #[must_use]
    pub(crate) fn byte_size(&self) -> usize {
        self.records.len() + self.offsets.len() * std::mem::size_of::<u64>()
    }

    fn record(&self, index: usize) -> Option<(&[u8], &[u8])> {
        let offset = usize::try_from(*self.offsets.get(index)?).ok()?;
        self.parse_at(offset)
    }

    fn parse_at(&self, offset: usize) -> Option<(&[u8], &[u8])> {
        let header = self.records.get(offset..offset + HEADER_SIZE)?;
        let key_len =
            usize::try_from(u32::from_le_bytes(header[..KEY_LEN_SIZE].try_into().ok()?)).ok()?;
        let value_len = usize::try_from(u64::from_le_bytes(
            header[KEY_LEN_SIZE..HEADER_SIZE].try_into().ok()?,
        ))
        .ok()?;
        let key_start = offset + HEADER_SIZE;
        let key_end = key_start.checked_add(key_len)?;
        let value_end = key_end.checked_add(value_len)?;
        Some((
            self.records.get(key_start..key_end)?,
            self.records.get(key_end..value_end)?,
        ))
    }

    fn key_at(&self, index: usize) -> Option<&[u8]> {
        self.record(index).map(|(key, _)| key)
    }

    fn search(&self, key: &[u8]) -> std::result::Result<usize, usize> {
        self.offsets.binary_search_by(|&offset| {
            let offset = usize::try_from(offset).expect("offset fits usize");
            let (stored, _) = self
                .parse_at(offset)
                .expect("packed offsets address valid records");
            stored.cmp(key)
        })
    }

    pub(crate) fn get(&self, key: &[u8]) -> Option<&[u8]> {
        match self.search(key) {
            Ok(index) => self.record(index).map(|(_, value)| value),
            Err(_) => None,
        }
    }

    pub(crate) fn entry(&self, index: usize) -> Option<Entry<'_>> {
        self.record(index).map(|(key, value)| Entry { key, value })
    }

    pub(crate) fn lower(&self, key: &[u8]) -> Option<Entry<'_>> {
        let index = match self.search(key) {
            Ok(index) | Err(index) => index.checked_sub(1)?,
        };
        self.entry(index)
    }

    pub(crate) fn greater(&self, key: &[u8]) -> Option<Entry<'_>> {
        let index = match self.search(key) {
            Ok(index) => index.checked_add(1)?,
            Err(index) => index,
        };
        self.entry(index)
    }

    pub(crate) fn greater_or_equal(&self, key: &[u8]) -> Option<Entry<'_>> {
        let index = match self.search(key) {
            Ok(index) | Err(index) => index,
        };
        self.entry(index)
    }

    pub(crate) fn start_index(&self, bound: Bound<&[u8]>) -> usize {
        match bound {
            Bound::Unbounded => 0,
            Bound::Included(key) => match self.search(key) {
                Ok(index) | Err(index) => index,
            },
            Bound::Excluded(key) => match self.search(key) {
                Ok(index) => index + 1,
                Err(index) => index,
            },
        }
    }

    pub(crate) fn end_index(&self, bound: Bound<&[u8]>) -> usize {
        match bound {
            Bound::Unbounded => self.offsets.len(),
            Bound::Included(key) => match self.search(key) {
                Ok(index) => index + 1,
                Err(index) => index,
            },
            Bound::Excluded(key) => match self.search(key) {
                Ok(index) | Err(index) => index,
            },
        }
    }

    pub(crate) fn range_bounds<'bounds>(
        &self,
        bounds: &Bounds<'bounds>,
    ) -> FrozenRange<'_, 'bounds> {
        let start = self.start_index(bounds.start);
        let end = self.end_index(bounds.end).max(start);
        FrozenRange {
            map: self,
            next: start,
            end,
            _bounds: PhantomData,
        }
    }
}

/// Lending range cursor over one [`FrozenMap`].
pub(crate) struct FrozenRange<'catalog, 'bounds> {
    map: &'catalog FrozenMap,
    next: usize,
    end: usize,
    _bounds: UnusedBounds<'bounds>,
}

impl ReadCursor for FrozenRange<'_, '_> {
    fn next(&mut self) -> Result<Option<Entry<'_>>> {
        if self.next >= self.end {
            return Ok(None);
        }
        let index = self.next;
        self.next += 1;
        Ok(self.map.entry(index))
    }
}

/// Monotone exact-key walker. After a hit or miss, the next `get` may
/// only ask for a nondecreasing key until [`SortedGets::reset`].
pub(crate) struct FrozenGets<'a> {
    map: &'a FrozenMap,
    pos: usize,
    #[cfg(debug_assertions)]
    last: Option<Vec<u8>>,
}

impl<'a> FrozenGets<'a> {
    pub(crate) fn new(map: &'a FrozenMap) -> Self {
        Self {
            map,
            pos: 0,
            #[cfg(debug_assertions)]
            last: None,
        }
    }
}

impl SortedGets for FrozenGets<'_> {
    type Value<'a>
        = &'a [u8]
    where
        Self: 'a;

    fn reset(&mut self) {
        self.pos = 0;
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
        while self.pos < self.map.offsets.len() {
            let stored = self.map.key_at(self.pos).expect("pos in range");
            match stored.cmp(key) {
                Ordering::Less => self.pos += 1,
                Ordering::Equal => {
                    let value = self.map.record(self.pos).map(|(_, value)| value);
                    self.pos += 1;
                    return Ok(value);
                }
                Ordering::Greater => return Ok(None),
            }
        }
        Ok(None)
    }
}

/// Fact scan: `F` keys of one relation, already sorted by row id.
pub(crate) struct FrozenFactCursor<'a> {
    map: &'a FrozenMap,
    next: usize,
    end: usize,
}

impl FactCursor for FrozenFactCursor<'_> {
    fn next(&mut self) -> Result<Option<FactEntry<'_>>> {
        if self.next >= self.end {
            return Ok(None);
        }
        let index = self.next;
        self.next += 1;
        let Some((key, bytes)) = self.map.record(index) else {
            return Ok(None);
        };
        let (_, row) = keys::parse_fact_key(key).ok_or(Error::Corruption(
            CorruptionError::MalformedValue("F key length"),
        ))?;
        Ok(Some(FactEntry { row, bytes }))
    }
}

/// Admitted packed catalog: `_data`, `_dict`, and the dictionary next-id.
/// No `_meta`. Typestate-identical to [`super::heap::CandidateCatalog`]
/// after the complete key phase — freeze is a move.
pub(crate) struct FrozenCatalog {
    pub(crate) data: FrozenMap,
    pub(crate) dict: FrozenMap,
    pub(crate) dict_next: InternId,
}

impl FrozenCatalog {
    #[must_use]
    pub(crate) fn byte_size(&self) -> usize {
        self.data.byte_size() + self.dict.byte_size()
    }

    pub(crate) fn from_parts(data: FrozenMap, dict: FrozenMap, dict_next: InternId) -> Self {
        Self {
            data,
            dict,
            dict_next,
        }
    }

    pub(crate) fn empty() -> Self {
        Self::from_parts(
            FrozenMap::empty(),
            FrozenMap::empty(),
            InternId::from_raw(0),
        )
    }

    fn map(&self, map: CatalogMap) -> &FrozenMap {
        match map {
            CatalogMap::Data => &self.data,
            CatalogMap::Dictionary => &self.dict,
        }
    }
}

impl OrderedRead for FrozenCatalog {
    type Value<'a>
        = &'a [u8]
    where
        Self: 'a;

    type Range<'catalog, 'bounds>
        = FrozenRange<'catalog, 'bounds>
    where
        Self: 'catalog;

    type Gets<'a>
        = FrozenGets<'a>
    where
        Self: 'a;

    fn get(&self, map: CatalogMap, key: &[u8]) -> Result<Option<&[u8]>> {
        Ok(self.map(map).get(key))
    }

    fn lower(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(self.map(map).lower(key))
    }

    fn greater(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(self.map(map).greater(key))
    }

    fn greater_or_equal(&self, map: CatalogMap, key: &[u8]) -> Result<Option<Entry<'_>>> {
        Ok(self.map(map).greater_or_equal(key))
    }

    fn range<'catalog, 'bounds>(
        &'catalog self,
        map: CatalogMap,
        bounds: Bounds<'bounds>,
    ) -> Result<FrozenRange<'catalog, 'bounds>> {
        Ok(self.map(map).range_bounds(&bounds))
    }

    fn sorted_gets(&self, map: CatalogMap) -> Result<FrozenGets<'_>> {
        Ok(FrozenGets::new(self.map(map)))
    }

    fn len(&self, map: CatalogMap) -> Result<u64> {
        Ok(self.map(map).len())
    }
}

impl CatalogRead for FrozenCatalog {
    type Facts<'a>
        = FrozenFactCursor<'a>
    where
        Self: 'a;

    fn scan_facts(&self, relation: RelationId) -> Result<FrozenFactCursor<'_>> {
        let mut lo = [0u8; keys::MAX_KEY];
        let start = keys::fact_prefix(&mut lo, relation);
        let mut hi = [0u8; keys::MAX_KEY];
        let hi_len = start.len();
        hi[..hi_len].copy_from_slice(start);
        let start_index = self.data.start_index(Bound::Included(start));
        let end_index = if increment_prefix(&mut hi[..hi_len]) {
            self.data.end_index(Bound::Excluded(&hi[..hi_len]))
        } else {
            self.data.offsets.len()
        };
        Ok(FrozenFactCursor {
            map: &self.data,
            next: start_index,
            end: end_index.max(start_index),
        })
    }

    fn dict_next_id(&self) -> Result<InternId> {
        Ok(self.dict_next)
    }
}

fn increment_prefix(prefix: &mut [u8]) -> bool {
    for byte in prefix.iter_mut().rev() {
        if *byte != 0xff {
            *byte += 1;
            return true;
        }
        *byte = 0;
    }
    false
}

#[cfg(test)]
mod tests;
