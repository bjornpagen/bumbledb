//! The per-braid generation map. Sum, domination, checkpoint order,
//! apply's increment, and the coordinate's one binary encoding live
//! here. Overflow is a refusal of [`Vector::sum`] and of nowhere else.

use std::collections::BTreeMap;

use crate::braids::BraidId;

/// One braid id and its applied count on the wire: `u32le` + `u64le`.
const PAIR_BYTES: usize = 4 + 8;

/// The vector sum overflowed `u64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Overflow;

/// The total order the manifest CAS installs: the candidate replaces
/// the incumbent iff its vector sum is strictly greater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointOrder {
    Before,
    Equal,
    After,
}

/// Why a vector payload refused to parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorError {
    Truncated { offset: usize },
    TrailingBytes { at: usize },
    Malformed { at: usize },
    Overflow,
}

impl VectorError {
    #[must_use]
    pub const fn identity(&self) -> &'static str {
        match self {
            Self::Truncated { .. } => "Truncated",
            Self::TrailingBytes { .. } => "TrailingBytes",
            Self::Malformed { .. } => "Malformed",
            Self::Overflow => "Overflow",
        }
    }
}

/// Applied counts keyed by braid. Any vector is a legal restore point;
/// pointwise dominance is the read-your-writes order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vector {
    counts: BTreeMap<BraidId, u64>,
}

impl Vector {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The wholeness arithmetic. The one overflow site.
    #[must_use]
    pub fn sum(&self) -> Result<u64, Overflow> {
        self.counts
            .values()
            .try_fold(0u64, |acc, &g| acc.checked_add(g))
            .ok_or(Overflow)
    }

    /// Pointwise `self[braid] >= other[braid]`; an absent braid is zero.
    #[must_use]
    pub fn dominates(&self, other: &Self) -> bool {
        other.counts.iter().all(|(braid, g)| self.at(*braid) >= *g)
    }

    /// The total order the manifest CAS installs.
    #[must_use]
    pub fn order(&self, other: &Self) -> CheckpointOrder {
        match (self.sum(), other.sum()) {
            (Ok(left), Ok(right)) if left < right => CheckpointOrder::Before,
            (Ok(left), Ok(right)) if left > right => CheckpointOrder::After,
            (Ok(_), Ok(_)) | (Err(Overflow), Err(Overflow)) => CheckpointOrder::Equal,
            (Err(Overflow), Ok(_)) => CheckpointOrder::After,
            (Ok(_), Err(Overflow)) => CheckpointOrder::Before,
        }
    }

    /// The applied count for `braid`; absent is zero.
    #[must_use]
    pub fn at(&self, braid: BraidId) -> u64 {
        self.counts.get(&braid).copied().unwrap_or(0)
    }

    /// Apply's one mutation: this braid's count advances by one.
    pub fn advance(&mut self, braid: BraidId) {
        let count = self.counts.entry(braid).or_insert(0);
        *count = count.saturating_add(1);
    }

    /// Sets the applied count for `braid`.
    pub fn set(&mut self, braid: BraidId, g: u64) {
        self.counts.insert(braid, g);
    }

    pub fn braids(&self) -> impl Iterator<Item = BraidId> + '_ {
        self.counts.keys().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (BraidId, u64)> + '_ {
        self.counts.iter().map(|(&braid, &g)| (braid, g))
    }

    /// `u32le` count, then `(u32le braid, u64le g)` pairs in braid order,
    /// bounded by the bytes behind the count.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let count = u32::try_from(self.counts.len()).expect("braid count fits u32");
        let mut out = Vec::with_capacity(4 + self.counts.len() * PAIR_BYTES);
        out.extend_from_slice(&count.to_le_bytes());
        for (braid, g) in &self.counts {
            out.extend_from_slice(&braid.raw().to_le_bytes());
            out.extend_from_slice(&g.to_le_bytes());
        }
        out
    }

    /// Inverse of [`Self::encode`]. A count the remaining bytes cannot
    /// open is truncated; entries must ascend; overflow is [`Self::sum`].
    pub fn parse(bytes: &[u8]) -> Result<Self, VectorError> {
        let mut cur = Cursor::new(bytes);
        let count = cur.u32()?;
        let need = usize::try_from(count)
            .ok()
            .and_then(|n| n.checked_mul(PAIR_BYTES))
            .ok_or(VectorError::Truncated { offset: cur.at })?;
        if cur.remaining() < need {
            return Err(VectorError::Truncated { offset: cur.at });
        }
        let mut vector = Self::new();
        for _ in 0..count {
            let braid = BraidId::from_raw(cur.u32()?);
            let g = cur.u64()?;
            if vector
                .counts
                .last_key_value()
                .is_some_and(|(last, _)| *last >= braid)
            {
                return Err(VectorError::Malformed { at: cur.at });
            }
            vector.set(braid, g);
        }
        if vector.sum().is_err() {
            return Err(VectorError::Overflow);
        }
        if cur.remaining() != 0 {
            return Err(VectorError::TrailingBytes { at: cur.at });
        }
        Ok(vector)
    }
}

impl From<BTreeMap<BraidId, u64>> for Vector {
    fn from(counts: BTreeMap<BraidId, u64>) -> Self {
        Self { counts }
    }
}

impl FromIterator<(BraidId, u64)> for Vector {
    fn from_iter<I: IntoIterator<Item = (BraidId, u64)>>(iter: I) -> Self {
        Self {
            counts: iter.into_iter().collect(),
        }
    }
}

struct Cursor<'b> {
    bytes: &'b [u8],
    at: usize,
}

impl<'b> Cursor<'b> {
    const fn new(bytes: &'b [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    const fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }

    fn take(&mut self, len: usize) -> Result<&'b [u8], VectorError> {
        let end = self
            .at
            .checked_add(len)
            .filter(|&end| end <= self.bytes.len())
            .ok_or(VectorError::Truncated { offset: self.at })?;
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, VectorError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, VectorError> {
        let bytes = self.take(8)?;
        let mut raw = [0u8; 8];
        raw.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(raw))
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckpointOrder, Overflow, Vector, VectorError};
    use crate::braids::BraidId;

    fn braid(raw: u32) -> BraidId {
        BraidId::from_raw(raw)
    }

    #[test]
    fn sum_is_the_overflow_site() {
        let mut vector = Vector::new();
        vector.set(braid(1), u64::MAX);
        vector.set(braid(2), 1);
        assert_eq!(vector.sum(), Err(Overflow));
    }

    #[test]
    fn dominates_is_pointwise_and_absent_is_zero() {
        let have: Vector = [(braid(1), 2), (braid(2), 1)].into_iter().collect();
        let want: Vector = [(braid(1), 2)].into_iter().collect();
        assert!(have.dominates(&want));
        assert!(!want.dominates(&have));
        let zero_want: Vector = [(braid(3), 0)].into_iter().collect();
        assert!(have.dominates(&zero_want));
    }

    #[test]
    fn order_follows_the_checked_sum() {
        let low: Vector = [(braid(1), 1)].into_iter().collect();
        let high: Vector = [(braid(1), 2)].into_iter().collect();
        assert_eq!(low.order(&high), CheckpointOrder::Before);
        assert_eq!(high.order(&low), CheckpointOrder::After);
        assert_eq!(low.order(&low), CheckpointOrder::Equal);
    }

    #[test]
    fn encode_is_count_then_pairs_and_parse_is_the_inverse() {
        let vector: Vector = [(braid(1), 4), (braid(3), 9)].into_iter().collect();
        let bytes = vector.encode();
        assert_eq!(&bytes[..4], 2u32.to_le_bytes());
        assert_eq!(Vector::parse(&bytes), Ok(vector));
    }

    #[test]
    fn parse_refuses_a_sum_that_overflows() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&u64::MAX.to_le_bytes());
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&1u64.to_le_bytes());
        assert_eq!(Vector::parse(&bytes), Err(VectorError::Overflow));
    }

    #[test]
    fn advance_is_the_one_mutation() {
        let mut vector = Vector::new();
        let id = braid(7);
        assert_eq!(vector.at(id), 0);
        vector.advance(id);
        assert_eq!(vector.at(id), 1);
        vector.advance(id);
        assert_eq!(vector.at(id), 2);
    }
}
