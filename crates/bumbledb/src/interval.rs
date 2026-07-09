//! The host-facing Interval value (docs/architecture/10-data-model.md).
//!
//! Construction is the validation boundary (parse, don't validate): the
//! constructors return `Option`, so a held [`Interval`] always satisfies
//! `start < end` and the encoder never re-checks it.

/// A half-open interval `[start, end)`: a finite set of points, written as
/// its bounds, strictly `start < end` — the empty interval is
/// unrepresentable, because a fact never denotes nothing.
///
/// The element domain is closed to the two orderable scalars; the two
/// inherent impls below are the whole surface — no other constructors, no
/// `Default`, no arithmetic. Deliberately **not** `Ord`/`PartialOrd`: the
/// value order the encoding has (lexicographic by start) is an encoding
/// accident, not semantics, and must not leak into host code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval<T> {
    start: T,
    end: T,
}

impl Interval<u64> {
    /// The unbounded-end convention: `[s, ∞)` is written `[s, MAX_END)`.
    /// Stated consequence: `MAX_END` itself is unusable as a point.
    pub const MAX_END: u64 = u64::MAX;

    /// Parses the bounds; `None` on `start >= end`.
    #[must_use]
    pub fn new(start: u64, end: u64) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }

    /// The open-ended interval `[start, MAX_END)`; `None` when `start` is
    /// `MAX_END` itself (the interval would be empty).
    #[must_use]
    pub fn from_start(start: u64) -> Option<Self> {
        Self::new(start, Self::MAX_END)
    }

    /// The inclusive lower bound.
    #[must_use]
    pub const fn start(&self) -> u64 {
        self.start
    }

    /// The exclusive upper bound.
    #[must_use]
    pub const fn end(&self) -> u64 {
        self.end
    }
}

impl Interval<i64> {
    /// The unbounded-end convention: `[s, ∞)` is written `[s, MAX_END)`.
    /// Stated consequence: `MAX_END` itself is unusable as a point.
    pub const MAX_END: i64 = i64::MAX;

    /// Parses the bounds; `None` on `start >= end`.
    #[must_use]
    pub fn new(start: i64, end: i64) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }

    /// The open-ended interval `[start, MAX_END)`; `None` when `start` is
    /// `MAX_END` itself (the interval would be empty).
    #[must_use]
    pub fn from_start(start: i64) -> Option<Self> {
        Self::new(start, Self::MAX_END)
    }

    /// The inclusive lower bound.
    #[must_use]
    pub const fn start(&self) -> i64 {
        self.start
    }

    /// The exclusive upper bound.
    #[must_use]
    pub const fn end(&self) -> i64 {
        self.end
    }
}

impl From<Interval<u64>> for crate::value::Value {
    /// Hosts construct interval literals through the checked
    /// [`Interval`] type, so a converted literal already satisfies
    /// `start < end`.
    fn from(interval: Interval<u64>) -> Self {
        Self::IntervalU64(interval.start(), interval.end())
    }
}

impl From<Interval<i64>> for crate::value::Value {
    /// Bounds discipline as [`From<Interval<u64>>`].
    fn from(interval: Interval<i64>) -> Self {
        Self::IntervalI64(interval.start(), interval.end())
    }
}

#[cfg(test)]
mod tests {
    use super::Interval;

    #[test]
    fn new_parses_strict_start_before_end() {
        assert!(Interval::<i64>::new(1, 5).is_some());
        assert!(Interval::<i64>::new(5, 5).is_none());
        assert!(Interval::<i64>::new(5, 1).is_none());
        assert!(Interval::<u64>::new(0, 1).is_some());
        assert!(Interval::<u64>::new(1, 0).is_none());
        assert!(Interval::<u64>::new(0, 0).is_none());
    }

    #[test]
    fn accessors_return_the_parsed_bounds() {
        let iv = Interval::<i64>::new(i64::MIN, i64::MAX).expect("widest interval");
        assert_eq!(iv.start(), i64::MIN);
        assert_eq!(iv.end(), i64::MAX);
    }

    #[test]
    fn from_start_is_the_unbounded_convention() {
        let iv = Interval::<u64>::from_start(7).expect("open-ended");
        assert_eq!(iv.end(), Interval::<u64>::MAX_END);
        // MAX_END is unusable as a point: an interval starting there is empty.
        assert!(Interval::<u64>::from_start(u64::MAX).is_none());
        assert!(Interval::<i64>::from_start(i64::MAX).is_none());
    }
}
