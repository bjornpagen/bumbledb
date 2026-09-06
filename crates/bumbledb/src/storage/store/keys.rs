//! Schema-fixed physical keys. Ordinal widths are selected once from the
//! sealed schema; every key uses those widths. Logical IDs, fingerprints,
//! metadata and canonical row bytes are unchanged.
use super::error::{StoreCorruption, StoreError, StoreResult};
use super::fingerprint::FP_LEN;
use super::format::{RowId, RowLocator};
use crate::schema::{ProjectionId, Schema};
use bumbledb_theory::schema::RelationId;

/// A validated physical key, borrowing its routing bytes from the snapshot.
/// Scans that consume only row bodies never need an owned physical address.
pub(crate) struct RowKey<'a> {
    pub(crate) relation: RelationId,
    pub(crate) home: &'a [u8],
    ordinal: &'a [u8; 8],
}

impl RowKey<'_> {
    pub(crate) fn locator(self) -> StoreResult<RowLocator> {
        RowLocator::new(RowId(u64::from_be_bytes(*self.ordinal)), self.home)
    }
}

/// Byte-for-byte LMDB's ordinary lexicographic order, compared a machine
/// word at a time. No schema, alignment, platform, or key-shape assumption
/// enters ordering: malformed and prefix keys must compare correctly too.
#[derive(Debug)]
pub(crate) struct PhysicalComparator;

impl heed::Comparator for PhysicalComparator {
    fn compare(left: &[u8], right: &[u8]) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        let common = left.len().min(right.len());
        let a_prefix = &left[..common];
        let b_prefix = &right[..common];
        if let (Some(a_last), Some(b_last)) =
            (a_prefix.last_chunk::<8>(), b_prefix.last_chunk::<8>())
        {
            // Compare every full word starting before the final word. Its
            // overlap contains only bytes already proved equal, so the
            // final comparison replaces a byte-at-a-time tail without
            // changing prefix order or reading beyond either slice.
            let (a_heads, _) = a_prefix[..common - 1].as_chunks::<8>();
            let (b_heads, _) = b_prefix[..common - 1].as_chunks::<8>();
            for (a, b) in a_heads.iter().zip(b_heads) {
                let order = u64::from_be_bytes(*a).cmp(&u64::from_be_bytes(*b));
                if order != Ordering::Equal {
                    return order;
                }
            }
            return u64::from_be_bytes(*a_last)
                .cmp(&u64::from_be_bytes(*b_last))
                .then_with(|| left.len().cmp(&right.len()));
        }
        for (&a, &b) in a_prefix.iter().zip(b_prefix) {
            let order = a.cmp(&b);
            if order != Ordering::Equal {
                return order;
            }
        }
        left.len().cmp(&right.len())
    }
}

/// The data database's sole prefix adapter. Heed's prefix convenience API
/// requires a byte-at-a-time comparator; its public range API permits our
/// equivalent wordwise comparator. Other database operations are unchanged.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DataTree(
    pub(crate) heed::Database<heed::types::Bytes, heed::types::Bytes, PhysicalComparator>,
);

impl std::ops::Deref for DataTree {
    type Target = heed::Database<heed::types::Bytes, heed::types::Bytes, PhysicalComparator>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DataTree {
    pub(crate) fn prefix_iter<'txn>(
        &self,
        txn: &'txn heed::RoTxn<'_, heed::AnyTls>,
        prefix: &[u8],
    ) -> heed::Result<impl Iterator<Item = heed::Result<(&'txn [u8], &'txn [u8])>> + use<'txn>>
    {
        // The widest data key is tag + relation + scalar home + row id.
        // Projection ordinals are narrower. Metadata/host keys use another DB.
        let mut owned_prefix = [0u8; 1 + 4 + 16 + 8];
        owned_prefix
            .get_mut(..prefix.len())
            .ok_or_else(|| heed::Error::Encoding(Box::new(malformed())))?
            .copy_from_slice(prefix);
        let length = prefix.len();
        // LMDB refuses a zero-length seek key. An empty prefix is the
        // whole tree, so begin at its first entry without a seek key.
        let start = if prefix.is_empty() {
            std::ops::Bound::Unbounded
        } else {
            std::ops::Bound::Included(prefix)
        };
        let range = self.0.range(txn, &(start, std::ops::Bound::Unbounded))?;
        Ok(range.take_while(move |entry| match entry {
            Ok((key, _)) => key.starts_with(&owned_prefix[..length]),
            Err(_) => true,
        }))
    }
}

pub const HOST_KEY_MAX: usize = 510;
pub(crate) const TAG_ROW: u8 = 0x04;
pub(crate) const TAG_MEMBERSHIP: u8 = 0x02;
pub(crate) const TAG_DETERMINANT: u8 = 0x03;

/// Physical census attribution owned by the codec, independent of tag values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalKeyKind {
    Row,
    Membership,
    Determinant,
    Metadata,
    Unknown,
}

impl PhysicalKeyKind {
    #[must_use]
    pub const fn from_census_tag(is_meta: bool, tag: u8) -> Self {
        if is_meta {
            return Self::Metadata;
        }
        match tag {
            TAG_ROW => Self::Row,
            TAG_MEMBERSHIP => Self::Membership,
            TAG_DETERMINANT => Self::Determinant,
            _ => Self::Unknown,
        }
    }
}

/// Schema-fixed base widths. Row keys additionally carry their relation's
/// exact scalar home (0–16 bytes); other widths are already complete or
/// explicitly named overheads, not worst-case bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalKeyWidths {
    /// Row key width without a home: namespace + relation + ordinal.
    pub row: usize,
    pub membership: usize,
    pub determinant_overhead: usize,
}

impl PhysicalKeyWidths {
    /// Derive exact physical widths from the same codec used to open a store.
    /// # Errors
    /// Compiled projection ordinal exhaustion.
    pub fn for_schema(schema: &Schema) -> Result<Self, crate::schema::compiled::CompileError> {
        Ok(KeyLayout::for_schema(schema)?.widths())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyLayout {
    relation: usize,
    projection: usize,
}

/// Inline maximum-capacity storage; only the key prefix is submitted to LMDB.
pub(crate) struct Key<const N: usize> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> Key<N> {
    fn new(tag: u8) -> Self {
        let mut key = Self {
            bytes: [0; N],
            len: 1,
        };
        key.bytes[0] = tag;
        key
    }
    fn push(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let end = self.len.checked_add(bytes.len()).ok_or_else(malformed)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or_else(malformed)?
            .copy_from_slice(bytes);
        self.len = end;
        Ok(())
    }
    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl<const N: usize> std::ops::Deref for Key<N> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

fn malformed() -> StoreError {
    StoreError::Corruption(StoreCorruption::MalformedKey("physical key shape"))
}

fn suffix(bytes: &[u8], width: usize) -> StoreResult<&[u8]> {
    let start = bytes.len().checked_sub(width).ok_or_else(malformed)?;
    if bytes[..start].iter().any(|byte| *byte != 0) {
        return Err(StoreError::ForeignSchema);
    }
    Ok(&bytes[start..])
}

fn decode_relation(bytes: &[u8]) -> RelationId {
    let mut word = [0; 4];
    word[4 - bytes.len()..].copy_from_slice(bytes);
    RelationId(u32::from_be_bytes(word))
}

impl KeyLayout {
    pub(crate) fn for_schema(
        schema: &Schema,
    ) -> Result<Self, crate::schema::compiled::CompileError> {
        let relation = u32::try_from(schema.relations().len().saturating_sub(1))
            .expect("validated relation ordinals fit u32");
        let projection = u16::try_from(
            schema
                .compiled_theory()?
                .projections()
                .len()
                .saturating_sub(1),
        )
        .expect("compiled projection ordinals fit u16");
        Ok(Self::from_maxima(relation, projection))
    }
    fn from_maxima(relation: u32, projection: u16) -> Self {
        Self {
            relation: (32 - relation.leading_zeros()).max(1).div_ceil(8) as usize,
            projection: (16 - projection.leading_zeros()).max(1).div_ceil(8) as usize,
        }
    }
    pub(crate) fn widths(self) -> PhysicalKeyWidths {
        PhysicalKeyWidths {
            row: 1 + self.relation + 8,
            membership: 1 + self.relation + FP_LEN + 8,
            determinant_overhead: 1 + self.projection + 8,
        }
    }
    pub(crate) fn row_key(self, relation: RelationId, row: RowLocator) -> StoreResult<Key<29>> {
        let mut key = Key::new(TAG_ROW);
        key.push(suffix(&relation.0.to_be_bytes(), self.relation)?)?;
        key.push(row.home())?;
        key.push(&row.id.0.to_be_bytes())?;
        Ok(key)
    }
    pub(crate) fn row_bucket(self, relation: RelationId, home: &[u8]) -> StoreResult<Key<21>> {
        if home.len() > crate::schema::MAX_EXACT_SCALAR_BYTES {
            return Err(malformed());
        }
        let mut key = Key::new(TAG_ROW);
        key.push(suffix(&relation.0.to_be_bytes(), self.relation)?)?;
        key.push(home)?;
        Ok(key)
    }
    pub(crate) fn row_prefix(self, relation: RelationId) -> StoreResult<Key<5>> {
        let mut key = Key::new(TAG_ROW);
        key.push(suffix(&relation.0.to_be_bytes(), self.relation)?)?;
        Ok(key)
    }
    pub(crate) fn membership_key(
        self,
        relation: RelationId,
        fp: &[u8; FP_LEN],
        row: RowId,
    ) -> StoreResult<Key<29>> {
        let mut key = Key::new(TAG_MEMBERSHIP);
        key.push(suffix(&relation.0.to_be_bytes(), self.relation)?)?;
        key.push(fp)?;
        key.push(&row.0.to_be_bytes())?;
        Ok(key)
    }
    pub(crate) fn membership_bucket(
        self,
        relation: RelationId,
        fp: &[u8; FP_LEN],
    ) -> StoreResult<Key<21>> {
        let mut key = Key::new(TAG_MEMBERSHIP);
        key.push(suffix(&relation.0.to_be_bytes(), self.relation)?)?;
        key.push(fp)?;
        Ok(key)
    }
    pub(crate) fn determinant_bucket(
        self,
        projection: ProjectionId,
        routing: &[u8],
    ) -> StoreResult<Key<19>> {
        let mut key = Key::new(TAG_DETERMINANT);
        key.push(suffix(&projection.0.to_be_bytes(), self.projection)?)?;
        if routing.len() > FP_LEN {
            return Err(malformed());
        }
        key.push(routing)?;
        Ok(key)
    }
    pub(crate) fn determinant_key(
        self,
        projection: ProjectionId,
        routing: &[u8],
        row: RowId,
    ) -> StoreResult<Key<27>> {
        let mut key = Key::new(TAG_DETERMINANT);
        key.push(suffix(&projection.0.to_be_bytes(), self.projection)?)?;
        if routing.len() > FP_LEN {
            return Err(malformed());
        }
        key.push(routing)?;
        key.push(&row.0.to_be_bytes())?;
        Ok(key)
    }
    pub(crate) fn decode_row(self, key: &[u8]) -> StoreResult<(RelationId, RowLocator)> {
        let key = self.decode_row_key(key)?;
        Ok((key.relation, key.locator()?))
    }

    #[inline]
    pub(crate) fn decode_row_key(self, key: &[u8]) -> StoreResult<RowKey<'_>> {
        let base = self.widths().row;
        if !(base..=base + crate::schema::MAX_EXACT_SCALAR_BYTES).contains(&key.len())
            || key.first() != Some(&TAG_ROW)
        {
            return Err(malformed());
        }
        Ok(RowKey {
            relation: decode_relation(&key[1..=self.relation]),
            home: &key[1 + self.relation..key.len() - 8],
            ordinal: key.last_chunk().ok_or_else(malformed)?,
        })
    }
    pub(crate) fn decode_membership(
        self,
        key: &[u8],
    ) -> StoreResult<(RelationId, [u8; FP_LEN], RowId)> {
        if key.len() != self.widths().membership || key.first() != Some(&TAG_MEMBERSHIP) {
            return Err(malformed());
        }
        let start = 1 + self.relation;
        let fp = key[start..start + FP_LEN]
            .try_into()
            .map_err(|_| malformed())?;
        Ok((
            decode_relation(&key[1..start]),
            fp,
            row_id_from_suffix(key, key.len())?,
        ))
    }
    pub(crate) fn decode_determinant(
        self,
        key: &[u8],
    ) -> StoreResult<(ProjectionId, &[u8], RowId)> {
        if !(self.widths().determinant_overhead..=self.widths().determinant_overhead + FP_LEN)
            .contains(&key.len())
            || key.first() != Some(&TAG_DETERMINANT)
        {
            return Err(malformed());
        }
        let start = 1 + self.projection;
        let mut projection = [0; 2];
        projection[2 - self.projection..].copy_from_slice(&key[1..start]);
        Ok((
            ProjectionId(u16::from_be_bytes(projection)),
            &key[start..key.len() - 8],
            row_id_from_suffix(key, key.len())?,
        ))
    }
}

pub(crate) fn row_id_from_suffix(key: &[u8], expected_len: usize) -> StoreResult<RowId> {
    if key.len() != expected_len || expected_len < 8 {
        return Err(malformed());
    }
    let suffix = key[expected_len - 8..]
        .try_into()
        .map_err(|_| malformed())?;
    Ok(RowId(u64::from_be_bytes(suffix)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use heed::Comparator as _;

    #[test]
    fn borrowed_row_keys_preserve_every_home_width_and_ordinal_without_copying() {
        for maximum in [0, 255, 256, 65_536, u32::MAX] {
            let layout = KeyLayout::from_maxima(maximum, 0);
            for home_len in 0..=crate::schema::MAX_EXACT_SCALAR_BYTES {
                let home: Vec<u8> = (0..home_len)
                    .map(|i| u8::try_from(i + 1).unwrap())
                    .collect();
                for id in [RowId(0), RowId(1), RowId(u64::MAX)] {
                    let relation = RelationId(maximum);
                    let locator = RowLocator::new(id, &home).unwrap();
                    let key = layout.row_key(relation, locator).unwrap();
                    let borrowed = layout.decode_row_key(&key).unwrap();
                    assert_eq!(borrowed.relation, relation);
                    assert_eq!(borrowed.home, home);
                    assert_eq!(borrowed.home.as_ptr(), key[1 + layout.relation..].as_ptr());
                    assert_eq!(borrowed.locator().unwrap(), locator);
                }
            }
            for length in 0..=layout.widths().row + crate::schema::MAX_EXACT_SCALAR_BYTES + 1 {
                let mut key = vec![0; length];
                if let Some(tag) = key.first_mut() {
                    *tag = TAG_ROW;
                }
                let expected = (layout.widths().row
                    ..=layout.widths().row + crate::schema::MAX_EXACT_SCALAR_BYTES)
                    .contains(&length);
                assert_eq!(layout.decode_row_key(&key).is_ok(), expected);
                if let Some(tag) = key.first_mut() {
                    *tag = TAG_MEMBERSHIP;
                    assert!(layout.decode_row_key(&key).is_err());
                }
            }
        }
    }

    #[test]
    fn physical_comparator_matches_byte_order_exhaustively_and_at_word_boundaries() {
        for a in 0..=u8::MAX {
            for b in 0..=u8::MAX {
                assert_eq!(PhysicalComparator::compare(&[a], &[b]), a.cmp(&b));
            }
        }
        for length in (0..=24).chain([29, 43, 64, 255, 511]) {
            for fill in [0, 1, 127, 128, 255] {
                let key = vec![fill; length];
                for end in 0..=length {
                    assert_eq!(
                        PhysicalComparator::compare(&key, &key[..end]),
                        key.as_slice().cmp(&key[..end])
                    );
                    assert_eq!(
                        PhysicalComparator::compare(&key[..end], &key),
                        key[..end].cmp(&key)
                    );
                }
                for offset in 0..length {
                    let mut changed = key.clone();
                    changed[offset] ^= 255;
                    assert_eq!(
                        PhysicalComparator::compare(&key, &changed),
                        key.cmp(&changed)
                    );
                    assert_eq!(
                        PhysicalComparator::compare(&changed, &key),
                        changed.cmp(&key)
                    );
                    if length <= 24 {
                        for end in 0..=length {
                            assert_eq!(
                                PhysicalComparator::compare(&key, &changed[..end]),
                                key.as_slice().cmp(&changed[..end])
                            );
                            assert_eq!(
                                PhysicalComparator::compare(&changed[..end], &key),
                                changed[..end].cmp(&key)
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn physical_comparator_matches_varied_long_and_shared_prefix_keys() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut left_backing = [0; 518];
        let mut right_backing = [0; 518];
        for round in 0..2048usize {
            // Every relative byte alignment, including independently
            // unaligned left/right inputs and all overlapping-tail widths.
            let left_offset = round % 8;
            let right_offset = (round / 8) % 8;
            let left = &mut left_backing[left_offset..left_offset + 511];
            let right = &mut right_backing[right_offset..right_offset + 511];
            for (a, b) in left.iter_mut().zip(right.iter_mut()) {
                *a = next().to_le_bytes()[0];
                *b = next().to_le_bytes()[0];
            }
            let left_len = usize::from(next().to_le_bytes()[0]) * 2;
            let right_len = usize::from(next().to_le_bytes()[0]) * 2 + 1;
            let shared = round % (left_len.min(right_len) + 1);
            right[..shared].copy_from_slice(&left[..shared]);
            let (a, b) = (&left[..left_len], &right[..right_len]);
            assert_eq!(PhysicalComparator::compare(a, b), a.cmp(b));
            assert_eq!(PhysicalComparator::compare(b, a), b.cmp(a));
        }
    }

    #[test]
    fn relation_width_boundaries_round_trip_without_truncation() {
        for (maximum, width) in [
            (0, 1),
            (255, 1),
            (256, 2),
            (65_535, 2),
            (65_536, 3),
            (16_777_215, 3),
            (16_777_216, 4),
            (u32::MAX, 4),
        ] {
            let layout = KeyLayout::from_maxima(maximum, 0);
            assert_eq!(layout.relation, width);
            for row in [RowId(0), RowId(1), RowId(u64::MAX)] {
                let relation = RelationId(maximum);
                let locator = RowLocator::unclustered(row);
                let key = layout.row_key(relation, locator).expect("row key");
                assert_eq!(key.len(), 1 + width + 8);
                assert_eq!(
                    layout.decode_row(&key).expect("decode"),
                    (relation, locator)
                );
                assert!(key.starts_with(&layout.row_prefix(relation).expect("prefix")));
                let member = layout
                    .membership_key(relation, &[0xAB; FP_LEN], row)
                    .expect("member");
                assert_eq!(
                    layout.decode_membership(&member).expect("decode"),
                    (relation, [0xAB; FP_LEN], row)
                );
                assert!(
                    member.starts_with(
                        &layout
                            .membership_bucket(relation, &[0xAB; FP_LEN])
                            .expect("bucket")
                    )
                );
            }
            if width < 4 {
                let overflow = RelationId(1 << (width * 8));
                assert!(
                    layout
                        .row_key(overflow, RowLocator::unclustered(RowId(1)))
                        .is_err()
                );
                assert!(layout.row_prefix(overflow).is_err());
                assert!(
                    layout
                        .membership_key(overflow, &[0; FP_LEN], RowId(1))
                        .is_err()
                );
                assert!(layout.membership_bucket(overflow, &[0; FP_LEN]).is_err());
            }
        }
    }

    #[test]
    fn clustered_row_keys_round_trip_order_and_preserve_duplicate_home_ordinals() {
        for maximum in [255, 65_535, 16_777_215, u32::MAX] {
            let layout = KeyLayout::from_maxima(maximum, 0);
            for home_width in [0, 1, 8, 16] {
                let mut keys = Vec::new();
                for relation in [RelationId(0), RelationId(maximum)] {
                    for byte in [0, 1, 255] {
                        // Width zero has exactly one possible home.
                        if home_width == 0 && byte != 0 {
                            continue;
                        }
                        let home = [byte; 16];
                        let home = &home[..home_width];
                        let bucket = layout.row_bucket(relation, home).expect("bucket");
                        assert_eq!(bucket.len(), 1 + layout.relation + home_width);
                        for id in [RowId(0), RowId(1), RowId(u64::MAX)] {
                            let locator = RowLocator::new(id, home).expect("locator");
                            let key = layout.row_key(relation, locator).expect("key");
                            assert_eq!(key.len(), layout.widths().row + home_width);
                            assert_eq!(locator.home(), home);
                            assert_eq!(
                                layout.decode_row(&key).expect("decode"),
                                (relation, locator)
                            );
                            assert!(key.starts_with(&bucket));
                            assert!(
                                key.starts_with(&layout.row_prefix(relation).expect("relation"))
                            );
                            keys.push(key.to_vec());
                        }
                    }
                }
                assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
            }
            if layout.relation < 4 {
                let overflow = RelationId(1 << (layout.relation * 8));
                assert!(layout.row_bucket(overflow, &[0; 16]).is_err());
            }
            assert!(layout.row_bucket(RelationId(0), &[0; 17]).is_err());
        }
        assert!(RowLocator::new(RowId(0), &[0; 17]).is_err());
        assert_eq!(
            RowLocator::new(RowId(7), &[]).unwrap(),
            RowLocator::unclustered(RowId(7))
        );
        assert_ne!(
            RowLocator::new(RowId(7), &[0]).unwrap(),
            RowLocator::new(RowId(7), &[1]).unwrap()
        );
    }

    #[test]
    fn projection_width_boundaries_preserve_routing_and_row() {
        for (maximum, width) in [(0, 1), (255, 1), (256, 2), (u16::MAX, 2)] {
            let layout = KeyLayout::from_maxima(0, maximum);
            assert_eq!(layout.projection, width);
            for routing in [&[][..], &[0x7F; 8][..], &[0xAB; 16][..]] {
                let projection = ProjectionId(maximum);
                let key = layout
                    .determinant_key(projection, routing, RowId(u64::MAX))
                    .expect("key");
                let (id, payload, row) = layout.decode_determinant(&key).expect("decode");
                assert_eq!(id, projection);
                assert_eq!(payload, routing);
                assert_eq!(row, RowId(u64::MAX));
                assert!(
                    key.starts_with(
                        &layout
                            .determinant_bucket(projection, routing)
                            .expect("prefix")
                    )
                );
            }
            if width == 1 {
                assert!(layout.determinant_bucket(ProjectionId(256), &[]).is_err());
                assert!(
                    layout
                        .determinant_key(ProjectionId(256), &[], RowId(0))
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn key_order_is_ordinal_then_payload_then_row_at_every_width() {
        for maximum in [255, 65_535, 16_777_215, u32::MAX] {
            let layout = KeyLayout::from_maxima(maximum, u16::MAX);
            let relations = [RelationId(0), RelationId(maximum / 2), RelationId(maximum)];
            let rows = [RowId(0), RowId(1), RowId(u64::MAX)];
            let mut encoded = Vec::new();
            let mut membership = Vec::new();
            for relation in relations {
                for row in rows {
                    encoded.push(
                        layout
                            .row_key(relation, RowLocator::unclustered(row))
                            .expect("key")
                            .to_vec(),
                    );
                    membership.push(
                        layout
                            .membership_key(relation, &[0; FP_LEN], row)
                            .expect("key")
                            .to_vec(),
                    );
                }
            }
            assert!(encoded.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(membership.windows(2).all(|pair| pair[0] < pair[1]));
        }
        for maximum in [255, u16::MAX] {
            let layout = KeyLayout::from_maxima(0, maximum);
            let mut keys = Vec::new();
            for projection in [ProjectionId(0), ProjectionId(maximum)] {
                for routing in [0u64.to_be_bytes(), u64::MAX.to_be_bytes()] {
                    for row in [RowId(0), RowId(u64::MAX)] {
                        keys.push(
                            layout
                                .determinant_key(projection, &routing, row)
                                .expect("key")
                                .to_vec(),
                        );
                    }
                }
            }
            assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        }
    }

    #[test]
    fn decoders_refuse_short_long_and_wrong_family_keys_before_slicing() {
        for maximum in [255, 65_535, 16_777_215, u32::MAX] {
            let layout = KeyLayout::from_maxima(maximum, u16::MAX);
            let row = layout
                .row_key(RelationId(0), RowLocator::unclustered(RowId(1)))
                .expect("row");
            let member = layout
                .membership_key(RelationId(0), &[0; FP_LEN], RowId(1))
                .expect("member");
            let determinant = layout
                .determinant_key(ProjectionId(0), &[], RowId(1))
                .expect("det");
            for end in 0..row.len() {
                assert!(layout.decode_row(&row[..end]).is_err());
            }
            for end in 0..member.len() {
                assert!(layout.decode_membership(&member[..end]).is_err());
            }
            for end in 0..determinant.len() {
                assert!(layout.decode_determinant(&determinant[..end]).is_err());
            }
            let mut oversized = determinant.to_vec();
            oversized.extend_from_slice(&[0; FP_LEN + 1]);
            assert!(layout.decode_determinant(&oversized).is_err());
            for (key, is_row) in [(row.to_vec(), true), (member.to_vec(), false)] {
                let mut wrong = key;
                wrong[0] = 0xFF;
                if is_row {
                    assert!(layout.decode_row(&wrong).is_err());
                } else {
                    assert!(layout.decode_membership(&wrong).is_err());
                }
                wrong[0] = if is_row { TAG_ROW } else { TAG_MEMBERSHIP };
                wrong.extend(std::iter::repeat_n(0, if is_row { 17 } else { 1 }));
                if is_row {
                    assert!(layout.decode_row(&wrong).is_err());
                } else {
                    assert!(layout.decode_membership(&wrong).is_err());
                }
            }
        }
        assert!(row_id_from_suffix(&[], 0).is_err());
        let layout = KeyLayout::from_maxima(0, 0);
        assert!(
            layout
                .determinant_bucket(ProjectionId(0), &[0; 17])
                .is_err()
        );
        assert!(
            layout
                .determinant_key(ProjectionId(0), &[0; 17], RowId(1))
                .is_err()
        );
    }
}
