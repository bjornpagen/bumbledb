use super::*;

/// Transaction-scoped scan over one current access path.
#[cfg(test)]
pub(crate) struct FactCursor<'borrow, 'env, 'schema> {
    pub(super) iter: heed::RoPrefix<'borrow, heed::types::Bytes, heed::types::Bytes>,
    pub(super) txn: &'borrow heed::RoTxn<'env, heed::WithoutTls>,
    pub(super) index_db: crate::RawDatabase,
    pub(super) dict: crate::RawDatabase,
    pub(super) relation: &'schema RelationDescriptor,
    pub(super) layout: &'schema AccessLayout,
    pub(super) range: Option<EncodedRange>,
}

/// Transaction-scoped encoded scan over one current access path.
pub(crate) struct EncodedFactCursor<'borrow, 'env, 'schema> {
    pub(super) iter: heed::RoPrefix<'borrow, heed::types::Bytes, heed::types::Bytes>,
    pub(super) layout: &'schema AccessLayout,
    pub(super) index_prefix: Vec<u8>,
    pub(super) _env: std::marker::PhantomData<&'env ()>,
}

#[cfg(test)]
impl Iterator for FactCursor<'_, '_, '_> {
    type Item = Result<FactCursorRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (key, _) = match self.iter.next()? {
                Ok(item) => item,
                Err(error) => return Some(Err(error.into())),
            };

            if !self.range_matches(key) {
                continue;
            }

            return Some(decode_access_scan_entry(
                self.dict,
                self.index_db,
                self.txn,
                self.relation,
                self.layout,
                key,
            ));
        }
    }
}

impl Iterator for EncodedFactCursor<'_, '_, '_> {
    type Item = Result<EncodedAccessItem>;

    fn next(&mut self) -> Option<Self::Item> {
        let (key, _) = match self.iter.next()? {
            Ok(item) => item,
            Err(error) => return Some(Err(error.into())),
        };
        Some(encoded_access_item(self.layout, &self.index_prefix, key))
    }
}

#[cfg(test)]
impl FactCursor<'_, '_, '_> {
    fn range_matches(&self, key: &[u8]) -> bool {
        let Some(range) = &self.range else {
            return true;
        };
        let Some(value) = key.get(range.offset..range.offset + range.width) else {
            return false;
        };
        if let Some(start) = &range.start
            && value < start.as_slice()
        {
            return false;
        }
        if let Some(end) = &range.end
            && value >= end.as_slice()
        {
            return false;
        }
        true
    }
}

pub(super) fn encoded_access_item(
    layout: &AccessLayout,
    index_prefix: &[u8],
    key: &[u8],
) -> Result<EncodedAccessItem> {
    let prefix_len = index_prefix.len();
    if key.len() != layout.encoded_len {
        return Err(Error::corrupt("index key width does not match layout"));
    }
    if key.get(0..prefix_len) != Some(index_prefix) {
        return Err(Error::corrupt("index key prefix does not match layout"));
    }
    Ok(EncodedAccessItem {
        key: key.to_vec(),
        prefix_len,
    })
}
