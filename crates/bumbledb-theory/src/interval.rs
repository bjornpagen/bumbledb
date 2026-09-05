//! The host-facing Interval value.
//! Construction is the validation boundary (parse, don't validate): the
//! constructors return `Option`, so a held [`Interval`] always satisfies
//! `start < end` over valid endpoints, and the encoder never re-checks it.
//! The engine's coalescing segment sweep is not theory and stays engine-side
//! (`bumbledb::interval::sweep`).
//!
//! Two point domains share the one half-open algebra:
//! - **Discrete integers** (`u64`, `i64`): the maximum integer is reserved
//!   as the ray endpoint and is never a represented temporal point.
//!   Fixed-width construction and exact integer duration exist here only.
//! - **The dense numeric line** (`F64`): canonical binary64 endpoints
//!   embedded as exact rationals; `-Infinity`/`+Infinity` are unbounded
//!   endpoints, never points; NaN is refused at either endpoint. There is
//!   deliberately no `FixedInterval<F64>` — rounded `start + width` is not
//!   an exact fixed-width representation — and no epsilon anywhere.
use crate::F64;

mod sealed {
    pub trait Sealed {}
    impl Sealed for u64 {}
    impl Sealed for i64 {}
    impl Sealed for crate::F64 {}
}

/// The interval element domain — the Rust face of the spec's endpoint
/// vocabulary, sealed to the three orderable scalars: no fourth element
/// type is constructible, and no host comparator enters the algebra.
/// Integer elements state the point-domain ceiling once
/// (`lean/Bumbledb/Values.lean: PointDomain`); the float element states
/// the dense-line endpoint/point split (NaN is no endpoint, nonfinite
/// values are no points).
pub trait Element: sealed::Sealed + Copy + Ord {
    /// The ordering ceiling: the reserved integer ray endpoint, or the
    /// float's `+Infinity` unbounded-above sentinel.
    const MAX_END: Self;

    /// Whether this value may appear as a bound. Total `true` for the
    /// integers; `false` exactly for the canonical NaN.
    fn valid_endpoint(self) -> bool {
        true
    }

    /// Whether this value denotes a point of the element's line — the
    /// membership prefilter. Integers are always points (the reserved
    /// ceiling probe is an ordinary nonmatch through `p < end`); floats
    /// are points exactly when finite.
    fn is_point(self) -> bool {
        true
    }
}

/// The discrete integer half of the element family: fixed-width
/// construction (`[start, start + width)`) and exact integer duration are
/// meaningful here and deliberately absent from the dense float line.
pub trait Discrete: Element {
    fn add_width(self, width: u64) -> Option<Self>;

    /// The exact width `end − start` as an unsigned integer.
    fn distance(self, end: Self) -> u64;
}

impl Element for u64 {
    const MAX_END: Self = u64::MAX;
}

impl Discrete for u64 {
    fn add_width(self, width: u64) -> Option<Self> {
        self.checked_add(width)
    }

    fn distance(self, end: Self) -> u64 {
        end - self
    }
}

impl Element for i64 {
    const MAX_END: Self = i64::MAX;
}

impl Discrete for i64 {
    fn add_width(self, width: u64) -> Option<Self> {
        self.checked_add_unsigned(width)
    }

    fn distance(self, end: Self) -> u64 {
        end.abs_diff(self)
    }
}

impl Element for F64 {
    const MAX_END: Self = F64::INFINITY;

    fn valid_endpoint(self) -> bool {
        !self.is_nan()
    }

    fn is_point(self) -> bool {
        self.is_finite()
    }
}

/// A half-open interval `[start, end)`: a set of points, written as its
/// bounds, strictly `start < end` — the empty interval is unrepresentable,
/// because a fact never denotes nothing. Half-open and nonempty are
/// Allen's algebra's preconditions, not conventions.
///
/// The generic constructors and the integer `const_new` twins all check
/// this invariant over valid endpoints; there is no unchecked constructor,
/// `Default`, or arithmetic. For `F64` the strict order also proves the
/// unbounded-endpoint placement: `-Infinity` fits only below a greater
/// end, `+Infinity` only above a lesser start, and NaN never parses.
/// Deliberately **not** `Ord`/`PartialOrd`: the value order the encoding
/// has (lexicographic by start) is an encoding accident, not semantics,
/// and must not leak into host code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Interval<T> {
    start: T,
    end: T,
}

impl<T: Element> Interval<T> {
    pub const MAX_END: T = T::MAX_END;

    #[must_use]
    pub fn new(start: T, end: T) -> Option<Self> {
        (start.valid_endpoint() && end.valid_endpoint() && start < end)
            .then_some(Self { start, end })
    }

    /// The interval unbounded above: `[start, MAX_END)` for the integers,
    /// `[start, +Infinity)` on the dense line. `ray(MAX)`/`ray(+Infinity)`
    /// refuses — an empty set is not a value.
    #[must_use]
    pub fn ray(start: T) -> Option<Self> {
        Self::new(start, Self::MAX_END)
    }

    /// Whether the interval is unbounded above (`end` is the reserved
    /// integer ceiling or `+Infinity`).
    #[must_use]
    pub fn is_ray(&self) -> bool {
        self.end == Self::MAX_END
    }

    /// Exact membership after the element's point prefilter: integers at
    /// the reserved ceiling and every nonfinite float probe are
    /// well-defined nonmatches, never errors.
    #[must_use]
    pub fn contains(&self, point: T) -> bool {
        point.is_point() && self.start <= point && point < self.end
    }

    #[must_use]
    pub const fn start(&self) -> T {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> T {
        self.end
    }
}

impl<T: Discrete> Interval<T> {
    /// Fixed-width `[start, start + width)`; never a ray.
    /// `lean/Bumbledb/Values.lean: FixedU64.not_ray`,
    /// `lean/Bumbledb/Countermodels.lean: unit_slot_at_ceiling_unconstructible`.
    /// Absent for `F64` by the [`Discrete`] bound: rounded float widths do
    /// not establish constant exact length.
    ///
    /// ```compile_fail
    /// use bumbledb_theory::{F64, Interval};
    /// let _ = Interval::<F64>::fixed(F64::ZERO, 5); // no dense fixed width
    /// ```
    #[must_use]
    pub fn fixed(start: T, width: u64) -> Option<Self> {
        let end = start.add_width(width).filter(|end| *end < Self::MAX_END)?;
        Self::new(start, end)
    }

    /// The exact integer duration `end − start` of a bounded interval.
    /// A ray refuses instead of returning a finite measure — the ceiling
    /// is a sentinel, not a point.
    #[must_use]
    pub fn duration(&self) -> Option<u64> {
        (!self.is_ray()).then(|| self.start.distance(self.end))
    }
}

/// A float interval's refused numerical length, two distinct cases:
/// an unbounded interval has no finite measure at all, while a bounded
/// interval's exact endpoint difference can round past the finite range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatMeasureError {
    /// Either endpoint is an infinity: the length is undefined, not large.
    Unbounded,
    /// Both endpoints are finite but the once-rounded difference is not:
    /// e.g. `[-MAX_FINITE, +MAX_FINITE)` is bounded with an overflowing
    /// F64 length.
    Overflow,
}

impl core::fmt::Display for FloatMeasureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unbounded => "an unbounded float interval has no finite length",
            Self::Overflow => "the bounded float length overflows binary64",
        })
    }
}

impl std::error::Error for FloatMeasureError {}

impl Interval<F64> {
    /// Whether both endpoints are finite. `[-Infinity, x)` and
    /// `[x, +Infinity)` are legal values whose length refuses.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        self.start.is_finite() && self.end.is_finite()
    }

    /// The correctly rounded numerical length `end − start`: the exact
    /// rational endpoint difference rounded once to canonical binary64,
    /// computed in integer arithmetic (no FPU state can influence it).
    /// This is a numerical length on the dense line, never a count of
    /// representable machine floats, and never an approximate capacity.
    ///
    /// # Errors
    /// [`FloatMeasureError::Unbounded`] for either infinite endpoint;
    /// [`FloatMeasureError::Overflow`] when the exact finite difference
    /// rounds to infinity.
    pub fn length(&self) -> Result<F64, FloatMeasureError> {
        if !self.is_bounded() {
            return Err(FloatMeasureError::Unbounded);
        }
        let length = crate::float::exact::sub_rounded(self.end, self.start);
        if length.is_finite() {
            Ok(length)
        } else {
            Err(FloatMeasureError::Overflow)
        }
    }
}

// The stable const surface cannot call a generic `Ord` operation. These
// sealed integer twins use the same strict comparison as `new`, rather than
// granting a macro or any other safe caller an unchecked construction path.
macro_rules! const_constructor {
    ($($element:ty),+ $(,)?) => {
        $(
            impl Interval<$element> {
                /// The const-evaluable twin of [`Self::new`], with the same
                /// strict nonempty-bound check. Invalid bounds return `None`
                /// at both compile time and runtime.
                #[must_use]
                pub const fn const_new(start: $element, end: $element) -> Option<Self> {
                    if start < end {
                        Some(Self { start, end })
                    } else {
                        None
                    }
                }
            }
        )+
    };
}

const_constructor!(u64, i64);

impl<T: Copy> Interval<T> {
    /// beyond the parse invariant `start < end`.
    #[must_use]
    pub const fn bounds(self) -> (T, T) {
        (self.start, self.end)
    }
}

impl From<Interval<u64>> for crate::value::Value {
    fn from(interval: Interval<u64>) -> Self {
        Self::IntervalU64(interval)
    }
}

impl From<Interval<i64>> for crate::value::Value {
    fn from(interval: Interval<i64>) -> Self {
        Self::IntervalI64(interval)
    }
}

impl From<Interval<F64>> for crate::value::Value {
    fn from(interval: Interval<F64>) -> Self {
        Self::IntervalF64(interval)
    }
}

#[cfg(test)]
mod tests {
    use super::{Discrete, Element, FloatMeasureError, Interval};
    use crate::F64;

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
    fn const_construction_has_no_unchecked_escape() {
        const UNSIGNED: Option<Interval<u64>> = Interval::<u64>::const_new(0, u64::MAX);
        const SIGNED: Option<Interval<i64>> = Interval::<i64>::const_new(i64::MIN, i64::MAX);
        const EMPTY: Option<Interval<u64>> = Interval::<u64>::const_new(7, 7);
        const REVERSED: Option<Interval<i64>> = Interval::<i64>::const_new(7, -7);

        assert_eq!(UNSIGNED.map(Interval::bounds), Some((0, u64::MAX)));
        assert_eq!(SIGNED.map(Interval::bounds), Some((i64::MIN, i64::MAX)));
        assert!(EMPTY.is_none());
        assert!(REVERSED.is_none());
    }

    #[test]
    fn const_and_runtime_integer_construction_agree_at_every_boundary_pair() {
        let unsigned = [0, 1, 2, u64::MAX / 2, u64::MAX - 1, u64::MAX];
        for start in unsigned {
            for end in unsigned {
                assert_eq!(
                    Interval::<u64>::const_new(start, end),
                    Interval::<u64>::new(start, end)
                );
            }
        }
        let signed = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
        for start in signed {
            for end in signed {
                assert_eq!(
                    Interval::<i64>::const_new(start, end),
                    Interval::<i64>::new(start, end)
                );
            }
        }
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

        assert!(
            Interval::<i64>::new(0, i64::MAX)
                .expect("ray by new")
                .is_ray()
        );

        assert!(Interval::<u64>::ray(u64::MAX).is_none());
        assert!(Interval::<i64>::ray(i64::MAX).is_none());
    }

    #[test]
    fn ray_duration_refuses_and_bounded_duration_is_exact() {
        assert_eq!(Interval::<u64>::ray(7).expect("ray").duration(), None);
        assert_eq!(Interval::<i64>::ray(-7).expect("ray").duration(), None);
        assert_eq!(
            Interval::<u64>::new(3, 9).expect("bounded").duration(),
            Some(6)
        );
        assert_eq!(
            Interval::<i64>::new(-4, 3).expect("bounded").duration(),
            Some(7)
        );
        // The full signed span is exact in u64.
        assert_eq!(
            Interval::<i64>::new(i64::MIN, i64::MAX - 1)
                .expect("bounded")
                .duration(),
            Some(u64::MAX - 1)
        );
    }

    #[test]
    fn integer_membership_at_the_ceiling_is_an_ordinary_nonmatch() {
        let ray = Interval::<u64>::ray(7).expect("ray");
        assert!(ray.contains(7));
        assert!(ray.contains(u64::MAX - 1));
        assert!(!ray.contains(u64::MAX), "the ceiling is not a point");
        assert!(!ray.contains(0));
        let signed = Interval::<i64>::ray(0).expect("ray");
        assert!(!signed.contains(i64::MAX));
    }

    #[test]
    fn fixed_parses_the_q2_bound() {
        let iv = Interval::<u64>::fixed(3, 5).expect("in-domain fixed value");
        assert_eq!((iv.start(), iv.end()), (3, 8));
        assert!(!iv.is_ray());
        let iv = Interval::<i64>::fixed(-4, 7).expect("in-domain fixed value");
        assert_eq!((iv.start(), iv.end()), (-4, 3));
        // Zero width denotes nothing: refused.
        assert!(Interval::<u64>::fixed(3, 0).is_none());
        assert!(Interval::<i64>::fixed(3, 0).is_none());

        assert!(Interval::<u64>::fixed(u64::MAX - 1, 1).is_none());
        assert!(Interval::<u64>::fixed(u64::MAX - 2, 1).is_some());
        assert!(Interval::<u64>::fixed(1, u64::MAX).is_none());
        assert!(Interval::<i64>::fixed(i64::MAX - 1, 1).is_none());
        assert!(Interval::<i64>::fixed(i64::MAX - 2, 1).is_some());
        assert!(Interval::<i64>::fixed(-1, u64::MAX).is_none());

        assert!(Interval::<i64>::fixed(i64::MIN, u64::MAX - 1).is_some());
        assert!(Interval::<i64>::fixed(i64::MIN, u64::MAX).is_none());
        assert!(Interval::<u64>::fixed(0, u64::MAX - 1).is_some());
        assert!(Interval::<u64>::fixed(0, u64::MAX).is_none());
    }

    #[test]
    fn one_impl_serves_the_discrete_elements() {
        fn probe<T: Discrete>(start: T, width: u64) -> Option<Interval<T>> {
            Interval::fixed(start, width)
        }
        assert_eq!(probe(3u64, 5), Interval::<u64>::fixed(3, 5));
        assert_eq!(probe(-4i64, 7), Interval::<i64>::fixed(-4, 7));
    }

    #[test]
    fn value_variants_accept_only_checked_intervals() {
        let unsigned = Interval::<u64>::new(3, 9).expect("checked");
        let signed = Interval::<i64>::new(-5, 9).expect("checked");
        let dense = Interval::<F64>::new(F64::ZERO, F64::from(1.0)).expect("checked");
        assert_eq!(crate::Value::IntervalU64(unsigned), unsigned.into());
        assert_eq!(crate::Value::IntervalI64(signed), signed.into());
        assert_eq!(crate::Value::IntervalF64(dense), dense.into());
    }

    // ---- The dense float line (F-INTERVAL fixtures). ----

    fn dense(start: f64, end: f64) -> Option<Interval<F64>> {
        Interval::<F64>::new(F64::from(start), F64::from(end))
    }

    #[test]
    fn nan_is_refused_at_either_endpoint() {
        assert!(Interval::<F64>::new(F64::NAN, F64::ZERO).is_none());
        assert!(Interval::<F64>::new(F64::ZERO, F64::NAN).is_none());
        assert!(Interval::<F64>::new(F64::NAN, F64::NAN).is_none());
        // NaN sorts above +Infinity in the total order, so without the
        // endpoint gate `[x, NaN)` would parse; the gate closes it.
        assert!(Interval::<F64>::new(F64::INFINITY, F64::NAN).is_none());
    }

    #[test]
    fn signed_zero_normalizes_before_validation_so_neg0_pos0_refuses() {
        // Both zeros are the one canonical zero: `[-0, +0)` is `[0, 0)`.
        assert!(dense(-0.0, 0.0).is_none());
        assert!(dense(0.0, -0.0).is_none());
        // And a normalized zero endpoint is an ordinary bound.
        let iv = dense(-0.0, 1.0).expect("[-0,1) is [0,1)");
        assert_eq!(iv.start(), F64::ZERO);
    }

    #[test]
    fn infinity_placement_is_enforced_by_strict_order() {
        // -Infinity is legal only as a lower bound; +Infinity only upper.
        assert!(Interval::<F64>::new(F64::NEG_INFINITY, F64::INFINITY).is_some());
        assert!(Interval::<F64>::new(F64::INFINITY, F64::NEG_INFINITY).is_none());
        assert!(Interval::<F64>::new(F64::INFINITY, F64::INFINITY).is_none());
        assert!(Interval::<F64>::new(F64::NEG_INFINITY, F64::NEG_INFINITY).is_none());
        // A right ray by `ray` is `[x, +Infinity)`.
        let ray = Interval::<F64>::ray(F64::ZERO).expect("[0, +inf)");
        assert!(ray.is_ray());
        assert!(!ray.is_bounded());
        assert!(Interval::<F64>::ray(F64::INFINITY).is_none());
        assert!(Interval::<F64>::ray(F64::NAN).is_none());
    }

    #[test]
    fn adjacent_representable_endpoints_form_a_valid_positive_interval() {
        // [a, nextUp(a)) with finite ordered bounds is nonempty on the
        // dense line — no successor arithmetic enters the algebra.
        let a = F64::from(1.0);
        let next_up = F64::from_bits(a.to_bits() + 1);
        let iv = Interval::<F64>::new(a, next_up).expect("positive width");
        assert!(iv.contains(a));
        assert!(!iv.contains(next_up));
    }

    #[test]
    fn the_left_ray_below_min_finite_is_a_nonempty_value() {
        // `[-Infinity, -MAX_FINITE)` is the distinguishing dense fixture:
        // valid and nonempty though no finite representable F64 lies inside.
        let iv =
            Interval::<F64>::new(F64::NEG_INFINITY, F64::MIN_FINITE).expect("nonempty left ray");
        assert!(!iv.is_bounded());
        assert!(!iv.contains(F64::MIN_FINITE));
        assert!(!iv.contains(F64::NEG_INFINITY), "no nonfinite point");
        assert_eq!(iv.length(), Err(FloatMeasureError::Unbounded));
    }

    #[test]
    fn the_whole_line_is_a_value_and_nonfinite_probes_are_false() {
        let line = Interval::<F64>::new(F64::NEG_INFINITY, F64::INFINITY).expect("[-inf, +inf)");
        assert!(line.contains(F64::ZERO));
        assert!(line.contains(F64::MAX_FINITE));
        assert!(line.contains(F64::MIN_FINITE));
        assert!(!line.contains(F64::NEG_INFINITY));
        assert!(!line.contains(F64::INFINITY));
        assert!(!line.contains(F64::NAN));
        assert_eq!(line.length(), Err(FloatMeasureError::Unbounded));
    }

    #[test]
    fn float_membership_is_exact_half_open() {
        let iv = dense(0.0, 1.0).expect("[0,1)");
        assert!(iv.contains(F64::ZERO));
        assert!(iv.contains(F64::from(0.5)));
        assert!(!iv.contains(F64::from(1.0)));
        // The quotient has one zero: the -0 probe IS the +0 probe.
        assert_eq!(iv.contains(F64::from(-0.0)), iv.contains(F64::ZERO));
        assert!(!iv.contains(F64::from(2.0)));
        assert!(!iv.contains(F64::NAN));
    }

    #[test]
    fn bounded_length_rounds_once_and_overflow_is_distinct_from_unbounded() {
        assert_eq!(dense(0.0, 1.0).expect("[0,1)").length(), Ok(F64::from(1.0)));
        assert_eq!(
            dense(-1.5, 2.5).expect("[-1.5,2.5)").length(),
            Ok(F64::from(4.0))
        );
        // Bounded, but the exact difference rounds past the finite range.
        let wide = Interval::<F64>::new(F64::MIN_FINITE, F64::MAX_FINITE).expect("bounded");
        assert!(wide.is_bounded());
        assert_eq!(wide.length(), Err(FloatMeasureError::Overflow));
        // Unbounded is a different refusal.
        assert_eq!(
            Interval::<F64>::ray(F64::ZERO).expect("ray").length(),
            Err(FloatMeasureError::Unbounded)
        );
        // Adjacent representable endpoints have the exact positive gap.
        let a = F64::from(1.0);
        let next_up = F64::from_bits(a.to_bits() + 1);
        assert_eq!(
            Interval::<F64>::new(a, next_up).expect("gap").length(),
            Ok(F64::from_bits(0x3cb0_0000_0000_0000)) // 2^-52
        );
    }

    #[test]
    fn allen_endpoint_order_serves_the_dense_line() {
        fn generic_meets<T: Element>(a: Interval<T>, b: Interval<T>) -> bool {
            a.end() == b.start()
        }
        let left = dense(0.0, 1.0).expect("[0,1)");
        let right = dense(1.0, 2.0).expect("[1,2)");
        assert!(generic_meets(left, right));
        // End b and next start nextUp(b) is a real gap, never adjacency.
        let b = F64::from(1.0);
        let gap_right = Interval::<F64>::new(F64::from_bits(b.to_bits() + 1), F64::from(2.0))
            .expect("gap start");
        assert!(!generic_meets(left, gap_right));
    }
}
