//! Collection-valued mutation reports and fresh-id ranges.

use super::Fresh;
use std::iter::FusedIterator;
use std::num::NonZeroU64;
use std::ops::Range;

/// Facts consumed vs facts that changed the in-memory final-state view
/// at call time. The length-1 report is `{ submitted: 1, changed: 0|1 }`.
/// `changed` counts recorded and cancelled net dispositions; intern-skips
/// reports through [`MutationReport::from_counts`]; `changed <= submitted`
/// and already-matching state do not increment. The engine constructs
/// is the invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutationReport {
    submitted: u64,
    changed: u64,
}

impl MutationReport {
    pub const EMPTY: Self = Self {
        submitted: 0,
        changed: 0,
    };

    pub(super) const fn from_counts(submitted: u64, changed: u64) -> Self {
        debug_assert!(changed <= submitted);
        Self { submitted, changed }
    }

    #[must_use]
    pub const fn submitted(self) -> u64 {
        self.submitted
    }

    #[must_use]
    pub const fn changed(self) -> u64 {
        self.changed
    }
}

impl Default for MutationReport {
    fn default() -> Self {
        Self::EMPTY
    }
}

trait FreshWord: Copy {
    fn from_word(raw: u64) -> Self;
    fn to_word(self) -> u64;
}

impl FreshWord for u64 {
    fn from_word(raw: u64) -> Self {
        raw
    }
    fn to_word(self) -> u64 {
        self
    }
}

impl<T: Fresh> FreshWord for T {
    fn from_word(raw: u64) -> Self {
        T::from_fresh(raw)
    }
    fn to_word(self) -> u64 {
        self.fresh()
    }
}

/// Fresh ids from one [`super::WriteTx::reserve`] / `reserve_at`.
/// Empty is absence of a range — `reserve(0)` does not read or advance
/// the sequence — not a degenerate `[0, 0)` interval of minted ids.
/// The exclusive bound is a count, never a minted `T`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshRange<T> {
    Empty,

    NonEmpty { start: T, count: NonZeroU64 },
}

impl<T> FreshRange<T> {
    #[must_use]
    pub fn start(self) -> Option<T> {
        match self {
            Self::Empty => None,
            Self::NonEmpty { start, .. } => Some(start),
        }
    }

    #[must_use]
    pub const fn len(&self) -> u64 {
        match self {
            Self::Empty => 0,
            Self::NonEmpty { count, .. } => count.get(),
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

#[allow(private_bounds)]
impl<T: FreshWord> FreshRange<T> {
    pub(super) fn minted(start: u64, count: NonZeroU64) -> Self {
        debug_assert!(
            start.checked_add(count.get()).is_some(),
            "reserve refuses a range whose exclusive end overflows"
        );
        Self::NonEmpty {
            start: T::from_word(start),
            count,
        }
    }

    fn exclusive_end(start: u64, count: NonZeroU64) -> u64 {
        start
            .checked_add(count.get())
            .expect("FreshRange::minted is the one overflow check")
    }

    #[must_use]
    pub fn end_exclusive_raw(self) -> Option<u64> {
        match self {
            Self::Empty => None,
            Self::NonEmpty { start, count } => Some(Self::exclusive_end(start.to_word(), count)),
        }
    }

    #[must_use]
    pub fn ids(self) -> Option<Range<u64>> {
        match self {
            Self::Empty => None,
            Self::NonEmpty { start, count } => {
                let raw = start.to_word();
                Some(raw..Self::exclusive_end(raw, count))
            }
        }
    }

    #[must_use]
    pub fn get(self, index: u64) -> Option<T> {
        let Self::NonEmpty { start, count } = self else {
            return None;
        };
        (index < count.get()).then(|| T::from_word(start.to_word() + index))
    }

    pub fn iter(self) -> FreshRangeIter<T> {
        match self {
            Self::Empty => FreshRangeIter {
                next: 0,
                end: 0,
                marker: std::marker::PhantomData,
            },
            Self::NonEmpty { start, count } => {
                let raw = start.to_word();
                FreshRangeIter {
                    next: raw,
                    end: Self::exclusive_end(raw, count),
                    marker: std::marker::PhantomData,
                }
            }
        }
    }
}

#[allow(private_bounds)]
impl<T: FreshWord> IntoIterator for FreshRange<T> {
    type Item = T;
    type IntoIter = FreshRangeIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over a [`FreshRange`].
#[derive(Debug, Clone)]
pub struct FreshRangeIter<T> {
    next: u64,
    end: u64,
    marker: std::marker::PhantomData<fn() -> T>,
}

#[allow(private_bounds)]
impl<T: FreshWord> Iterator for FreshRangeIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next >= self.end {
            return None;
        }
        let id = self.next;
        self.next += 1;
        Some(T::from_word(id))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = usize::try_from(self.end.saturating_sub(self.next)).unwrap_or(usize::MAX);
        (n, Some(n))
    }
}

#[allow(private_bounds)]
impl<T: FreshWord> ExactSizeIterator for FreshRangeIter<T> {}
#[allow(private_bounds)]
impl<T: FreshWord> FusedIterator for FreshRangeIter<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::db::Fresh;
    use bumbledb_theory::schema::{FieldId, RelationId};

    #[derive(Clone, Copy)]
    struct Id(u64);
    impl Fresh for Id {
        type Schema = ();
        const RELATION: RelationId = RelationId(0);
        const FIELD: FieldId = FieldId(0);
        fn from_fresh(raw: u64) -> Self {
            Self(raw)
        }
        fn fresh(self) -> u64 {
            self.0
        }
    }

    #[test]
    fn empty_cannot_yield_a_minted_id() {
        let empty: FreshRange<Id> = FreshRange::Empty;
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty.start().is_none());
        assert!(empty.get(0).is_none());
        assert!(empty.iter().next().is_none());
        assert!(empty.ids().is_none());
        assert!(empty.end_exclusive_raw().is_none());
        let collected: Vec<Id> = FreshRange::<Id>::Empty.into_iter().collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn nonempty_start_is_the_minted_id() {
        let range: FreshRange<Id> = FreshRange::minted(7, NonZeroU64::new(3).unwrap());
        assert_eq!(range.start().map(|Id(v)| v), Some(7));
        assert_eq!(range.get(0).map(|Id(v)| v), Some(7));
        assert_eq!(range.get(2).map(|Id(v)| v), Some(9));
        assert!(range.get(3).is_none());
        assert_eq!(range.end_exclusive_raw(), Some(10));
        let ids: Vec<u64> = range.iter().map(|Id(v)| v).collect();
        assert_eq!(ids, vec![7, 8, 9]);
    }

    #[test]
    fn raw_range_uses_the_same_start() {
        let range: FreshRange<u64> = FreshRange::minted(0, NonZeroU64::new(1).unwrap());
        assert_eq!(range.start(), Some(0));
        assert_eq!(range.get(0), Some(0));
        let empty: FreshRange<u64> = FreshRange::Empty;
        assert!(empty.start().is_none());
    }
}
