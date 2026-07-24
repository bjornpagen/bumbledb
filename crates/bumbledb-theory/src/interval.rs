//! The host-facing Interval value (docs/architecture/10-data-model.md).
//!
//! Construction is the validation boundary (parse, don't validate): the
//! constructors return `Option`, so a held [`Interval`] always satisfies
//! `start < end` and the encoder never re-checks it. The engine's
//! coalescing segment sweep is not theory and stays engine-side
//! (`bumbledb::interval::sweep`).

mod sealed {
    pub trait Sealed {}
    impl Sealed for u64 {}
    impl Sealed for i64 {}
}

/// The interval element domain — the Rust face of the spec's
/// `PointDomain` class (`lean/Bumbledb/Values.lean: PointDomain`): the
/// domain ceiling plus the width step, so the point-domain law is stated
/// once and [`Interval`]'s one impl serves every element. Sealed to the
/// two orderable scalars: no third element type is constructible.
pub trait Element: sealed::Sealed + Copy + Ord {
    /// The domain ceiling: `end == MAX_END` denotes the unbounded ray.
    const MAX_END: Self;

    /// `self + width`, `None` on overflow — `width` is a point count,
    /// so it is unsigned in both element domains (the dual of the
    /// spec's `gap`).
    fn add_width(self, width: u64) -> Option<Self>;
}

impl Element for u64 {
    const MAX_END: Self = u64::MAX;

    fn add_width(self, width: u64) -> Option<Self> {
        self.checked_add(width)
    }
}

impl Element for i64 {
    const MAX_END: Self = i64::MAX;

    fn add_width(self, width: u64) -> Option<Self> {
        self.checked_add_unsigned(width)
    }
}

/// A half-open interval `[start, end)`: a set of points, written as its
/// bounds, strictly `start < end` — the empty interval is unrepresentable,
/// because a fact never denotes nothing. Half-open and nonempty are
/// Allen's algebra's preconditions, not conventions
/// (docs/architecture/10-data-model.md, the point-domain law).
///
/// The element domain is closed by the sealed [`Element`] trait — the
/// generic impl below is the whole surface: no other constructors, no
/// `Default`, no arithmetic. Deliberately **not** `Ord`/`PartialOrd`: the
/// value order the encoding has (lexicographic by start) is an encoding
/// accident, not semantics, and must not leak into host code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval<T> {
    start: T,
    end: T,
}

impl<T: Element> Interval<T> {
    /// The point-domain law: points are `MIN ..= MAX_END − 1`, and
    /// `end == MAX_END` denotes the unbounded ray `[start, ∞)` — ∞ is a
    /// value of the representation, not a sentinel.
    pub const MAX_END: T = T::MAX_END;

    /// Parses the bounds; `None` on `start >= end`.
    #[must_use]
    pub fn new(start: T, end: T) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }

    /// The unbounded ray `[start, ∞)`; `None` when `start` is `MAX_END`
    /// itself (outside the point domain — the ray would begin past every
    /// point).
    #[must_use]
    pub fn ray(start: T) -> Option<Self> {
        Self::new(start, Self::MAX_END)
    }

    /// The fixed-width value `[start, start + width)` — the
    /// `interval<T, w>` constructor, discharging the Q2 bound by
    /// parsing: `None` unless `width ≥ 1` and `start + width < MAX_END`
    /// (strictly — the ceiling end denotes the unbounded ray, so a
    /// fixed-width value is NEVER a ray, by construction;
    /// `lean/Bumbledb/Values.lean: FixedU64.not_ray`,
    /// `lean/Bumbledb/Countermodels.lean:
    /// unit_slot_at_ceiling_unconstructible`).
    #[must_use]
    pub fn fixed(start: T, width: u64) -> Option<Self> {
        let end = start.add_width(width).filter(|end| *end < Self::MAX_END)?;
        Self::new(start, end)
    }

    /// Whether this interval is the unbounded ray `[start, ∞)`.
    #[must_use]
    pub fn is_ray(&self) -> bool {
        self.end == Self::MAX_END
    }

    /// The inclusive lower bound.
    #[must_use]
    pub const fn start(&self) -> T {
        self.start
    }

    /// The exclusive upper bound.
    #[must_use]
    pub const fn end(&self) -> T {
        self.end
    }
}

impl<T: Copy> Interval<T> {
    /// The parsed bounds `(start, end)` — the algebra's one read: the
    /// engine's `classify` and the canonical interval encoders are
    /// generic over the element order, and this is their sanctioned
    /// crossing into the checked type. Hosts read through the
    /// per-element accessors above; the bounds carry no order semantics
    /// beyond the parse invariant `start < end`.
    #[must_use]
    pub const fn bounds(self) -> (T, T) {
        (self.start, self.end)
    }
}

impl From<Interval<u64>> for crate::value::Value {
    /// Hosts construct interval literals through the checked
    /// [`Interval`] type, so a converted literal already satisfies
    /// `start < end`.
    fn from(interval: Interval<u64>) -> Self {
        Self::IntervalU64(interval)
    }
}

impl From<Interval<i64>> for crate::value::Value {
    /// Bounds discipline as [`From<Interval<u64>>`].
    fn from(interval: Interval<i64>) -> Self {
        Self::IntervalI64(interval)
    }
}

#[cfg(test)]
mod tests {
    use super::{Element, Interval};

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
    fn ray_is_the_unbounded_denotation() {
        let iv = Interval::<u64>::ray(7).expect("ray");
        assert_eq!(iv.end(), Interval::<u64>::MAX_END);
        assert!(iv.is_ray());
        assert!(!Interval::<u64>::new(7, 9).expect("bounded").is_ray());
        // `new` admits the ray end directly — `ray` is a name, not a mode.
        assert!(
            Interval::<i64>::new(0, i64::MAX)
                .expect("ray by new")
                .is_ray()
        );
        // MAX is not a point: a ray starting at the ceiling is empty.
        assert!(Interval::<u64>::ray(u64::MAX).is_none());
        assert!(Interval::<i64>::ray(i64::MAX).is_none());
    }

    #[test]
    fn fixed_parses_the_q2_bound() {
        // The happy path: [start, start + w), never a ray.
        let iv = Interval::<u64>::fixed(3, 5).expect("in-domain fixed value");
        assert_eq!((iv.start(), iv.end()), (3, 8));
        assert!(!iv.is_ray());
        let iv = Interval::<i64>::fixed(-4, 7).expect("in-domain fixed value");
        assert_eq!((iv.start(), iv.end()), (-4, 3));
        // Zero width denotes nothing: refused.
        assert!(Interval::<u64>::fixed(3, 0).is_none());
        assert!(Interval::<i64>::fixed(3, 0).is_none());
        // The Q2 bound is STRICT: start + w == MAX_END would make the
        // derived end the ceiling — ray territory — and anything past
        // it overflows; both are unconstructible.
        assert!(Interval::<u64>::fixed(u64::MAX - 1, 1).is_none());
        assert!(Interval::<u64>::fixed(u64::MAX - 2, 1).is_some());
        assert!(Interval::<u64>::fixed(1, u64::MAX).is_none());
        assert!(Interval::<i64>::fixed(i64::MAX - 1, 1).is_none());
        assert!(Interval::<i64>::fixed(i64::MAX - 2, 1).is_some());
        assert!(Interval::<i64>::fixed(-1, u64::MAX).is_none());
        // The widest representable fixed values, exactly at the Q2
        // bound (`0 < w ∧ start + w < maxEnd`, `FixedI64`/`FixedU64`):
        // i64's widest is `w = u64::MAX - 1` at `start = i64::MIN`
        // (derived end `i64::MAX - 1`, strictly below the ceiling);
        // one wider hits the ceiling and is unconstructible.
        assert!(Interval::<i64>::fixed(i64::MIN, u64::MAX - 1).is_some());
        assert!(Interval::<i64>::fixed(i64::MIN, u64::MAX).is_none());
        assert!(Interval::<u64>::fixed(0, u64::MAX - 1).is_some());
        assert!(Interval::<u64>::fixed(0, u64::MAX).is_none());
    }

    #[test]
    fn one_impl_serves_both_elements() {
        // The law is stated once: element-generic code reaches every
        // constructor through the sealed trait, and the per-element
        // spellings above are instantiations, not twins.
        fn probe<T: Element>(start: T, width: u64) -> Option<Interval<T>> {
            Interval::fixed(start, width)
        }
        assert_eq!(probe(3u64, 5), Interval::<u64>::fixed(3, 5));
        assert_eq!(probe(-4i64, 7), Interval::<i64>::fixed(-4, 7));
    }

    #[test]
    fn value_variants_accept_only_checked_intervals() {
        let unsigned = Interval::<u64>::new(3, 9).expect("checked");
        let signed = Interval::<i64>::new(-5, 9).expect("checked");
        assert_eq!(crate::Value::IntervalU64(unsigned), unsigned.into());
        assert_eq!(crate::Value::IntervalI64(signed), signed.into());
    }
}
