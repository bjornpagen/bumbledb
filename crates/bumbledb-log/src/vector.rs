//! The per-braid generation map. Sum, domination, checkpoint order,
//! and apply's increment live here. Overflow is a refusal of
//! [`Vector::sum`] and of nowhere else.

use std::collections::BTreeMap;

use crate::braids::BraidId;

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
    ///
    /// # Errors
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

#[cfg(test)]
mod tests {
    use bumbledb::schema::{RelationDescriptor, SchemaDescriptor};

    use super::{CheckpointOrder, Overflow, Vector};
    use crate::braids::{BraidId, braids};

    /// Mints braid `raw` the one honest way: a statement-free theory of
    /// disconnected relations makes every relation its own braid.
    fn braid(raw: u32) -> BraidId {
        let descriptor = SchemaDescriptor {
            relations: (0..=raw)
                .map(|id| RelationDescriptor {
                    name: format!("r{id}").into(),
                    fields: vec![],
                    extension: None,
                })
                .collect(),
            statements: vec![],
        };
        braids(&descriptor)
            .parse(raw)
            .expect("a disconnected relation is its own braid")
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
