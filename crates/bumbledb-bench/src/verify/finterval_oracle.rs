//! The independent dense float-interval oracle (P11, chapters 10 §2 and
//! 11 §5; gate `F-INTERVAL`).
//!
//! `Interval<F64>` denotes a half-open range on a DENSE numeric line, not a
//! set of representable machine floats. This oracle models exactly the
//! fragment the fixtures need without big rationals: a dense point is either
//! a representable canonical payload or the open midpoint strictly between a
//! representable value and its successor — enough to witness every
//! adjacency/gap/ray fixture, because interval endpoints are always
//! representable and any two distinct reals with a representable between
//! them are separated at this granularity.
//!
//! Nothing here consults production interval, Allen, or comparison helpers;
//! ordering is the sibling `f64_oracle::order_key` (itself independent).
//! The Lean twin is `lean/Bumbledb/FloatInterval.lean` (exact rational
//! points); the two models must agree on every shared fixture.

use super::f64_oracle::{Class, INF, NEG_INF, SIGN, canonical, classify, order_key};

/// A point on the dense line: a representable canonical non-NaN finite
/// payload, or the open gap point strictly between a representable payload
/// and its successor in the canonical order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dense {
    /// The representable finite value with these canonical payload bits.
    At(u64),
    /// A point strictly between `At(bits)` and its representable successor.
    JustAbove(u64),
}

impl Dense {
    /// Strictly-less on the dense line, via the total order key. `NaN` and
    /// the infinities are not points and must not be smuggled in.
    #[must_use]
    pub fn key(self) -> (u64, u8) {
        match self {
            Dense::At(bits) => (order_key(bits), 0),
            Dense::JustAbove(bits) => (order_key(bits), 1),
        }
    }
}

/// A checked float interval: canonical endpoints, NaN refused at both
/// bounds, strict numeric order; `-Infinity` therefore only ever bounds
/// below and `+Infinity` only above (nothing sorts beyond them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FInterval {
    start: u64,
    stop: u64,
}

/// The constructor refusals, typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NanEndpoint,
    EmptyOrInverted,
}

impl FInterval {
    /// Parse two raw endpoint payloads: normalize signed zero, refuse NaN,
    /// require strict numeric order. Placement of the infinities follows
    /// from the order alone.
    ///
    /// # Errors
    /// `NanEndpoint` for a NaN bound; `EmptyOrInverted` when
    /// `start >= stop` numerically (including `[-0, +0)`, whose endpoints
    /// normalize to one canonical zero).
    pub fn new(start_raw: u64, stop_raw: u64) -> Result<Self, Refusal> {
        let start = canonical(start_raw);
        let stop = canonical(stop_raw);
        if classify(start) == Class::Nan || classify(stop) == Class::Nan {
            return Err(Refusal::NanEndpoint);
        }
        if order_key(start) >= order_key(stop) {
            return Err(Refusal::EmptyOrInverted);
        }
        Ok(Self { start, stop })
    }

    #[must_use]
    pub fn start(self) -> u64 {
        self.start
    }

    #[must_use]
    pub fn stop(self) -> u64 {
        self.stop
    }

    /// Dense membership: inclusive at a finite start, exclusive at the
    /// stop, unbounded at an infinity payload.
    #[must_use]
    pub fn contains_dense(self, p: Dense) -> bool {
        let lower_ok = self.start == NEG_INF || {
            // start is finite (a +inf start cannot construct).
            Dense::At(self.start).key() <= p.key()
        };
        let upper_ok = self.stop == INF || p.key() < Dense::At(self.stop).key();
        lower_ok && upper_ok
    }

    /// The membership PROBE for an F64 scalar: a strictly finite probe
    /// embeds exactly; NaN and both infinities are well-defined
    /// nonmatches — never errors, never matches.
    #[must_use]
    pub fn contains_probe(self, probe_raw: u64) -> bool {
        let probe = canonical(probe_raw);
        if classify(probe) != Class::Finite {
            return false;
        }
        self.contains_dense(Dense::At(probe))
    }

    /// The RAW order-key comparison an engine would run WITHOUT the
    /// finite-probe guard — exposed so the tests can demonstrate the guard
    /// is load-bearing (`lean/Bumbledb/FloatInterval.lean:
    /// neg_inf_probe_needs_guard`).
    #[must_use]
    pub fn raw_key_compare(self, probe_raw: u64) -> bool {
        let probe = canonical(probe_raw);
        order_key(self.start) <= order_key(probe) && order_key(probe) < order_key(self.stop)
    }
}

/// The measure roster: unbounded and overflow are DISTINCT failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measure {
    Unbounded,
    Overflow,
    Length(u64),
}

/// Bounded length: the exact scaled endpoint difference rounded ONCE, ties
/// to even; a nonfinite bound is `Unbounded`; a finite difference whose one
/// rounding lands on the infinity payload is `Overflow`.
#[must_use]
pub fn measure(interval: FInterval) -> Measure {
    use super::f64_oracle::{round_dyadic, scaled_magnitude};
    if interval.start == NEG_INF || interval.stop == INF {
        return Measure::Unbounded;
    }
    // Both endpoints strictly finite; exact difference of signed scaled
    // magnitudes, always positive by the constructor's strict order.
    let mag = |bits: u64| scaled_magnitude(bits & !SIGN);
    let neg = |bits: u64| bits & SIGN != 0;
    let diff = match (neg(interval.start), neg(interval.stop)) {
        (false, false) => mag(interval.stop).sub(&mag(interval.start)),
        (true, true) => mag(interval.start).sub(&mag(interval.stop)),
        (true, false) => mag(interval.start).add(&mag(interval.stop)),
        (false, true) => unreachable!("inverted interval cannot construct"),
    };
    let bits = round_dyadic(&diff, 1, 0);
    if bits == INF {
        Measure::Overflow
    } else {
        Measure::Length(bits)
    }
}

/// Coalesce two intervals when they overlap or touch (half-open adjacency);
/// a strict gap — one interval ending strictly below where the next begins,
/// representable-successor gaps included — refuses.
#[must_use]
pub fn coalesce(a: FInterval, b: FInterval) -> Option<FInterval> {
    // Order by start.
    let (lo, hi) = if order_key(a.start) <= order_key(b.start) {
        (a, b)
    } else {
        (b, a)
    };
    // Touching or overlapping: hi.start <= lo.stop in the endpoint order.
    if order_key(hi.start) <= order_key(lo.stop) {
        let stop = if order_key(lo.stop) < order_key(hi.stop) {
            hi.stop
        } else {
            lo.stop
        };
        Some(FInterval {
            start: lo.start,
            stop,
        })
    } else {
        None
    }
}

/// The representable successor of a finite canonical payload in the total
/// order — test plumbing for adjacency fixtures (pure integer stepping on
/// the order key, no float arithmetic).
#[must_use]
pub fn next_up(bits: u64) -> u64 {
    debug_assert_eq!(classify(bits), Class::Finite);
    if bits == SIGN | 1 {
        // nextUp(-min_subnormal) is canonical zero.
        return 0;
    }
    if bits & SIGN != 0 { bits - 1 } else { bits + 1 }
}

#[cfg(test)]
mod tests {
    use super::super::f64_oracle::{INF, MAX_FINITE, NAN, NEG_INF, SIGN};
    use super::{Dense, FInterval, Measure, Refusal, coalesce, measure, next_up};

    const ONE: u64 = 0x3ff0_0000_0000_0000;
    const TWO: u64 = 0x4000_0000_0000_0000;
    const NEG_MAX: u64 = SIGN | MAX_FINITE;

    #[test]
    fn constructors_parse_and_refuse() {
        assert!(FInterval::new(ONE, TWO).is_ok());
        assert!(FInterval::new(NEG_INF, INF).is_ok(), "the whole line");
        assert_eq!(FInterval::new(NAN, ONE).unwrap_err(), Refusal::NanEndpoint);
        assert_eq!(
            FInterval::new(ONE, 0x7ff0_0000_0000_0001).unwrap_err(),
            Refusal::NanEndpoint,
            "a noncanonical NaN encoding is still NaN"
        );
        assert_eq!(
            FInterval::new(TWO, ONE).unwrap_err(),
            Refusal::EmptyOrInverted
        );
        assert_eq!(
            FInterval::new(ONE, ONE).unwrap_err(),
            Refusal::EmptyOrInverted
        );
        // [-0, +0) refuses: both endpoints normalize to one canonical zero.
        assert_eq!(
            FInterval::new(SIGN, 0).unwrap_err(),
            Refusal::EmptyOrInverted
        );
        // +Infinity as a lower bound / -Infinity as an upper bound cannot
        // construct: nothing sorts beyond them.
        assert_eq!(
            FInterval::new(INF, NAN & !SIGN).unwrap_err(),
            Refusal::NanEndpoint
        );
        assert_eq!(
            FInterval::new(INF, INF).unwrap_err(),
            Refusal::EmptyOrInverted
        );
        assert_eq!(
            FInterval::new(ONE, NEG_INF).unwrap_err(),
            Refusal::EmptyOrInverted
        );
    }

    #[test]
    fn neg_inf_to_neg_max_ray_is_nonempty() {
        // The distinguishing dense fixture: no representable finite F64
        // lies inside [-Infinity, -MAX_FINITE), yet the dense line does.
        let ray = FInterval::new(NEG_INF, NEG_MAX).expect("a valid left ray");
        // No representable probe is inside…
        assert!(!ray.contains_probe(NEG_MAX));
        assert!(!ray.contains_probe(NEG_INF), "nonfinite probes are false");
        assert!(!ray.contains_probe(ONE));
        // …but the dense point just above -MAX's PREDECESSOR side — i.e.
        // strictly beyond every finite value — is: the gap point above
        // -inf's neighbor is modeled from the other side: the point just
        // below -MAX is `JustAbove` of nothing representable, so witness
        // density from the interval's own frame: the dense point strictly
        // between -MAX and its successor is NOT in the ray (it is above
        // -MAX)…
        assert!(!ray.contains_dense(Dense::JustAbove(NEG_MAX)));
        // …while nonemptiness is witnessed at [-inf, x) for any x with a
        // representable below it: [-inf, -MAX) has no representable
        // member, and the oracle's dense fragment cannot spell a point
        // below every representable — that HALF of the fixture is the
        // Lean model's rational witness
        // (`lean/Bumbledb/FloatInterval.lean: negInfRay_witness`). What
        // this side pins is the discrete-model REFUTATION: an engine that
        // enumerates representable points calls this valid interval
        // empty.
        let representable_members = [NEG_MAX, SIGN | ONE, 0, ONE, MAX_FINITE, NEG_INF, INF, NAN]
            .iter()
            .filter(|&&p| ray.contains_probe(p))
            .count();
        assert_eq!(representable_members, 0, "no representable member exists");
        // The constructor still accepts it: valid and nonempty by the
        // dense contract, not by point enumeration.
    }

    #[test]
    fn order_key_execution_matches_dense_membership() {
        // On strictly finite probes, raw key comparison IS membership.
        let cases = [
            FInterval::new(ONE, TWO).expect("valid"),
            FInterval::new(NEG_MAX, MAX_FINITE).expect("valid"),
            FInterval::new(NEG_INF, ONE).expect("valid"),
            FInterval::new(ONE, INF).expect("valid"),
            FInterval::new(NEG_INF, INF).expect("valid"),
        ];
        let probes = [
            0u64,
            1,
            ONE,
            next_up(ONE),
            TWO,
            MAX_FINITE,
            NEG_MAX,
            SIGN | 1,
        ];
        for interval in cases {
            for probe in probes {
                assert_eq!(
                    interval.contains_probe(probe),
                    interval.raw_key_compare(probe),
                    "finite probe {probe:#x} in [{:#x},{:#x})",
                    interval.start(),
                    interval.stop()
                );
            }
        }
    }

    #[test]
    fn nonfinite_probes_return_false() {
        let left_ray = FInterval::new(NEG_INF, ONE).expect("valid");
        let right_ray = FInterval::new(ONE, INF).expect("valid");
        let line = FInterval::new(NEG_INF, INF).expect("valid");
        for interval in [left_ray, right_ray, line] {
            for probe in [NAN, INF, NEG_INF, 0xfff8_0000_0000_0000] {
                assert!(
                    !interval.contains_probe(probe),
                    "nonfinite probe {probe:#x} is a well-defined nonmatch"
                );
            }
        }
        // The guard is LOAD-BEARING: the raw key comparison would admit a
        // -Infinity probe on a left-unbounded interval.
        assert!(left_ray.raw_key_compare(NEG_INF));
        assert!(!left_ray.contains_probe(NEG_INF));
    }

    #[test]
    fn adjacent_coalesces_and_representable_gap_does_not() {
        // Half-open adjacency [1,2) + [2,3): coalesces to [1,3), and no
        // dense point is gained or lost.
        let three: u64 = 0x4008_0000_0000_0000;
        let a = FInterval::new(ONE, TWO).expect("valid");
        let b = FInterval::new(TWO, three).expect("valid");
        let joined = coalesce(a, b).expect("adjacency coalesces");
        assert_eq!((joined.start(), joined.stop()), (ONE, three));
        for p in [
            Dense::At(ONE),
            Dense::JustAbove(ONE),
            Dense::At(TWO),
            Dense::JustAbove(TWO),
            Dense::At(next_up(TWO)),
        ] {
            assert_eq!(
                joined.contains_dense(p),
                a.contains_dense(p) || b.contains_dense(p),
                "coalescing changes representation, never points"
            );
        }
        // End b, next start nextUp(b): a REAL gap — never coalesce merely
        // because the bounds are adjacent machine floats.
        let c = FInterval::new(next_up(TWO), three).expect("valid");
        assert_eq!(coalesce(a, c), None, "representable-neighbor gap");
        // The dense witness in the gap: strictly between TWO and its
        // successor, in neither interval.
        let witness = Dense::JustAbove(TWO);
        assert!(!a.contains_dense(witness));
        assert!(!c.contains_dense(witness));
        // [a, nextUp(a)) is a valid positive-width interval containing
        // exactly one representable and the dense points above it.
        let ulp = FInterval::new(TWO, next_up(TWO)).expect("valid");
        assert!(ulp.contains_dense(Dense::At(TWO)));
        assert!(!ulp.contains_dense(Dense::At(next_up(TWO))));
    }

    #[test]
    fn measure_distinguishes_unbounded_overflow_and_length() {
        let ray = FInterval::new(NEG_INF, NEG_MAX).expect("valid");
        assert_eq!(measure(ray), Measure::Unbounded);
        let up_ray = FInterval::new(ONE, INF).expect("valid");
        assert_eq!(measure(up_ray), Measure::Unbounded);
        // Bounded but the once-rounded length overflows: a DIFFERENT
        // failure from unbounded.
        let span = FInterval::new(NEG_MAX, MAX_FINITE).expect("valid");
        assert_eq!(measure(span), Measure::Overflow);
        // [1, 2) measures exactly 1.0.
        let unit = FInterval::new(ONE, TWO).expect("valid");
        assert_eq!(measure(unit), Measure::Length(ONE));
        // [2, nextUp(2)) has the positive exact length of one ulp at that
        // binade — 2^-51 — not zero and not a point count.
        let ulp = FInterval::new(TWO, next_up(TWO)).expect("valid");
        assert_eq!(measure(ulp), Measure::Length(0x3cc0_0000_0000_0000));
    }
}
