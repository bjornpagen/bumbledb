//! Allen's interval algebra — the classification half
//! (docs/architecture/20-query-ir.md § the Allen operator).
//!
//! The mask vocabulary — [`Basic`], [`AllenMask`], the palindromic bit
//! order, and the one-instruction converse — lives in `bumbledb-theory`
//! and is re-exported here as this crate's own surface. What stays
//! engine-side is classification: [`classify`], the total scalar
//! reference implementation over the checked [`Interval`] type, and the
//! word-level [`classify_bounds`] entry the evaluators call (the batch
//! configuration kernel is `exec/kernel/allen.rs`, property-tested
//! bit-identical against `classify`).

use bumbledb_theory::Interval;

pub use bumbledb_theory::allen::{AllenMask, Basic};

/// Classifies an interval pair: **the** total reference implementation of
/// the algebra (the configuration kernel — `exec/kernel/allen.rs` — is
/// the batch form, property-tested bit-identical against this one; this
/// one is the semantics). Total by construction — the match covers the 3 × 3
/// endpoint orderings, and both operands are nonempty half-open intervals
/// by [`Interval`]'s parse — and exactly one basic is returned (JEPD is a
/// theorem of the match shape, property-tested against the point-set
/// oracle). Rays need no case: `end == MAX` is an ordinary bound under
/// `Ord`.
#[must_use]
pub fn classify<T: Ord + Copy>(a: Interval<T>, b: Interval<T>) -> Basic {
    let (a_start, a_end) = a.bounds();
    let (b_start, b_end) = b.bounds();
    classify_bounds(&a_start, &a_end, &b_start, &b_end)
}

/// [`classify`] over raw bounds — the evaluators' entry: encoded column
/// words preserve value order (biased I64, `docs/architecture/50-storage.md`),
/// so classification over words equals classification over values.
/// Precondition (every caller's invariant): `a_start < a_end` and
/// `b_start < b_end`.
pub(crate) fn classify_bounds<T: Ord>(a_start: &T, a_end: &T, b_start: &T, b_end: &T) -> Basic {
    use std::cmp::Ordering::{Equal, Greater, Less};
    match (a_start.cmp(b_start), a_end.cmp(b_end)) {
        (Equal, Equal) => Basic::Equals,
        (Equal, Less) => Basic::Starts,
        (Equal, Greater) => Basic::StartedBy,
        (Less, Equal) => Basic::FinishedBy,
        (Greater, Equal) => Basic::Finishes,
        (Greater, Less) => Basic::During,
        (Less, Greater) => Basic::Contains,
        (Less, Less) => match a_end.cmp(b_start) {
            Less => Basic::Before,
            Equal => Basic::Meets,
            Greater => Basic::Overlaps,
        },
        (Greater, Greater) => match b_end.cmp(a_start) {
            Less => Basic::After,
            Equal => Basic::MetBy,
            Greater => Basic::OverlappedBy,
        },
    }
}

#[cfg(test)]
mod tests {
    //! Classification tests only — the mask vocabulary's own laws
    //! (constants, parse boundary, the exhaustive converse involution)
    //! are pinned in `bumbledb-theory`, next to the definitions.

    use super::{AllenMask, Basic, classify};
    use bumbledb_theory::Interval;

    /// A splitmix64 step — the repo's no-dependency randomness.
    fn splitmix(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The point-set definition of one basic over half-open intervals —
    /// the brute-force oracle, written from the denotation (shared-point
    /// tests are honest set intersections over a small domain), never
    /// from the endpoint formulas under test.
    fn oracle_holds(basic: Basic, a: (u64, u64), b: (u64, u64)) -> bool {
        let ((a_s, a_e), (b_s, b_e)) = (a, b);
        let intersects = a_s < b_e && b_s < a_e; // nonempty ∩ via sortedness
        match basic {
            Basic::Before => a_e < b_s,
            Basic::Meets => a_e == b_s,
            Basic::Overlaps => a_s < b_s && intersects && a_e < b_e,
            Basic::Starts => a_s == b_s && a_e < b_e,
            Basic::During => b_s < a_s && a_e < b_e,
            Basic::Finishes => b_s < a_s && a_e == b_e,
            Basic::Equals => a_s == b_s && a_e == b_e,
            Basic::FinishedBy => a_s < b_s && a_e == b_e,
            Basic::Contains => a_s < b_s && b_e < a_e,
            Basic::StartedBy => a_s == b_s && b_e < a_e,
            Basic::OverlappedBy => b_s < a_s && intersects && b_e < a_e,
            Basic::MetBy => b_e == a_s,
            Basic::After => b_e < a_s,
        }
    }

    /// Random and boundary pairs: adjacent, nested, equal, and rays.
    fn pair_corpus() -> Vec<((u64, u64), (u64, u64))> {
        const MAX: u64 = u64::MAX;
        let mut pairs = vec![
            // Boundary shapes, both polarities.
            ((0, 5), (5, 9)),  // adjacent (meets)
            ((5, 9), (0, 5)),  // adjacent (met-by)
            ((0, 10), (3, 7)), // nested (contains)
            ((3, 7), (0, 10)), // nested (during)
            ((2, 6), (2, 6)),  // equal
            ((2, 6), (2, 9)),  // starts
            ((2, 9), (2, 6)),  // started-by
            ((4, 9), (1, 9)),  // finishes
            ((1, 9), (4, 9)),  // finished-by
            ((0, 5), (3, 8)),  // overlaps
            ((3, 8), (0, 5)),  // overlapped-by
            ((0, 2), (7, 9)),  // before
            ((7, 9), (0, 2)),  // after
            // Rays: end == MAX is the unbounded [s, ∞).
            ((3, MAX), (3, MAX)), // equal rays
            ((3, MAX), (7, MAX)), // two rays: finished-by at ∞
            ((7, MAX), (3, MAX)), // ...and finishes
            ((0, 5), (5, MAX)),   // meets a ray
            ((5, MAX), (0, 5)),   // met-by a ray
            ((0, 9), (4, MAX)),   // overlaps into a ray
            ((4, MAX), (0, 9)),
            ((2, MAX), (2, 6)), // started-by, bounded inside a ray
            ((2, 6), (2, MAX)),
            ((0, 3), (8, MAX)), // before a ray
        ];
        let mut state = 0xA11E_5EED_u64;
        for _ in 0..4096 {
            // Small domain: boundary coincidences (equal endpoints,
            // adjacency) occur constantly instead of almost never.
            let draw = |state: &mut u64| {
                let s = splitmix(state) % 16;
                let e = s + 1 + splitmix(state) % 16;
                (s, e)
            };
            pairs.push((draw(&mut state), draw(&mut state)));
            // And a ray flavor per iteration.
            let (s, _) = draw(&mut state);
            pairs.push(((s, MAX), draw(&mut state)));
        }
        pairs
    }

    fn iv(bounds: (u64, u64)) -> Interval<u64> {
        Interval::<u64>::new(bounds.0, bounds.1).expect("test pairs are nonempty")
    }

    /// `classify` against the point-set oracle, with JEPD: the returned
    /// basic's definition holds and **no other** basic's does.
    #[test]
    fn classify_matches_the_point_set_oracle_jepd() {
        for (a, b) in pair_corpus() {
            let got = classify(iv(a), iv(b));
            for basic in Basic::ALL {
                assert_eq!(
                    oracle_holds(basic, a, b),
                    basic == got,
                    "JEPD violated at {a:?} vs {b:?}: classified {got:?}, oracle disagrees on {basic:?}"
                );
            }
        }
    }

    /// Converse involution, and classification duality:
    /// `classify(a, b).converse() == classify(b, a)`.
    #[test]
    fn converse_is_an_involution_and_dualizes_classification() {
        for (a, b) in pair_corpus() {
            let ab = classify(iv(a), iv(b));
            let ba = classify(iv(b), iv(a));
            assert_eq!(ab.converse(), ba, "{a:?} vs {b:?}");
            assert_eq!(ab.converse().converse(), ab);
        }
        for bits in 0..=0x1FFF_u16 {
            let mask = AllenMask::new(bits).expect("13-bit range");
            assert_eq!(mask.converse().converse(), mask);
            // Mask converse agrees with per-basic converse.
            for basic in Basic::ALL {
                assert_eq!(
                    mask.contains(basic),
                    mask.converse().contains(basic.converse())
                );
            }
        }
    }

    /// The 13 × 13 composition table, enumerated exhaustively:
    /// `table[r1][r2]` collects every basic `r3` witnessed as
    /// `classify(a, c)` over interval triples with `classify(a, b) = r1`
    /// and `classify(b, c) = r2`.
    ///
    /// Domain arithmetic: endpoints are the dense grid `0..=8` — 9
    /// values, so C(9,2) = 36 nonempty intervals and 36³ = 46,656
    /// ordered triples, all enumerated. Completeness: a witness for any
    /// table cell involves 3 intervals = 6 endpoints, so at most 6
    /// distinct values; every order type (with ties) of 6 endpoints is
    /// realizable inside a 9-value grid, hence the enumerated table is
    /// the WHOLE composition table, not a sample of it.
    fn enumerated_composition_table(points: &[u64]) -> [[u16; 13]; 13] {
        let mut intervals = Vec::new();
        for (i, &s) in points.iter().enumerate() {
            for &e in &points[i + 1..] {
                intervals.push((s, e));
            }
        }
        let mut table = [[0u16; 13]; 13];
        for &a in &intervals {
            for &b in &intervals {
                let r1 = classify(iv(a), iv(b)) as usize;
                for &c in &intervals {
                    let r2 = classify(iv(b), iv(c)) as usize;
                    let r3 = classify(iv(a), iv(c));
                    table[r1][r2] |= r3.bit();
                }
            }
        }
        table
    }

    /// Composition-table spot laws over the exhaustively enumerated
    /// table (46,656 triples — the arithmetic on
    /// [`enumerated_composition_table`]): the identity row/column, the
    /// hand-provable singleton entries, the full-uncertainty entry, the
    /// converse anti-homomorphism over all 13 × 13 = 169 cells, and
    /// `equals ∈ r ∘ r⁻¹` for every basic.
    #[test]
    fn exhaustive_composition_table_spot_laws() {
        let points: Vec<u64> = (0..=8).collect();
        let table = enumerated_composition_table(&points);
        let entry = |r1: Basic, r2: Basic| table[r1 as usize][r2 as usize];
        let singleton = |r: Basic| r.bit();

        // Equals is the two-sided identity: e ∘ r = r ∘ e = {r}.
        for r in Basic::ALL {
            assert_eq!(entry(Basic::Equals, r), singleton(r), "e;{r:?}");
            assert_eq!(entry(r, Basic::Equals), singleton(r), "{r:?};e");
        }
        // Hand-provable singleton entries (endpoint-inequality chains).
        assert_eq!(
            entry(Basic::Before, Basic::Before),
            singleton(Basic::Before)
        );
        assert_eq!(entry(Basic::After, Basic::After), singleton(Basic::After));
        assert_eq!(entry(Basic::Meets, Basic::Meets), singleton(Basic::Before));
        assert_eq!(
            entry(Basic::During, Basic::During),
            singleton(Basic::During)
        );
        assert_eq!(
            entry(Basic::Starts, Basic::During),
            singleton(Basic::During)
        );
        assert_eq!(
            entry(Basic::Finishes, Basic::During),
            singleton(Basic::During)
        );
        // a overlaps b, b overlaps c: a starts first, ends inside b; only
        // a.end vs c.start is open — before, meets, or overlaps.
        assert_eq!(
            entry(Basic::Overlaps, Basic::Overlaps),
            Basic::Before.bit() | Basic::Meets.bit() | Basic::Overlaps.bit()
        );
        // Total uncertainty: before ∘ after constrains a vs c not at all.
        assert_eq!(
            AllenMask::new(entry(Basic::Before, Basic::After)).expect("13-bit"),
            AllenMask::FULL
        );
        // The converse anti-homomorphism over every cell:
        // (r1 ∘ r2)⁻¹ = r2⁻¹ ∘ r1⁻¹.
        for r1 in Basic::ALL {
            for r2 in Basic::ALL {
                let lhs = AllenMask::new(entry(r1, r2)).expect("13-bit").converse();
                let rhs = AllenMask::new(entry(r2.converse(), r1.converse())).expect("13-bit");
                assert_eq!(lhs, rhs, "anti-homomorphism at ({r1:?}, {r2:?})");
            }
        }
        // Identity membership: equals ∈ r ∘ r⁻¹ (witness: a = c).
        for r in Basic::ALL {
            assert!(
                entry(r, r.converse()) & Basic::Equals.bit() != 0,
                "equals ∉ {r:?} ∘ {r:?}⁻¹"
            );
        }
    }

    /// The Miri-lane representative of the composition laws: the same
    /// enumeration on the 5-value grid `0..=4` (C(5,2) = 10 intervals,
    /// 10³ = 1,000 triples). The identity and anti-homomorphism laws
    /// hold on ANY grid-enumerated table (the enumeration is closed
    /// under triple reversal), so the subset is a sound fast pin; the
    /// completeness-dependent equalities live in the exhaustive test.
    #[test]
    fn representative_composition_laws_on_the_small_grid() {
        let points: Vec<u64> = (0..=4).collect();
        let table = enumerated_composition_table(&points);
        for r in Basic::ALL {
            assert_eq!(table[Basic::Equals as usize][r as usize], r.bit());
            assert_eq!(table[r as usize][Basic::Equals as usize], r.bit());
        }
        for r1 in Basic::ALL {
            for r2 in Basic::ALL {
                let lhs = AllenMask::new(table[r1 as usize][r2 as usize])
                    .expect("13-bit")
                    .converse();
                let rhs = AllenMask::new(table[r2.converse() as usize][r1.converse() as usize])
                    .expect("13-bit");
                assert_eq!(lhs, rhs, "anti-homomorphism at ({r1:?}, {r2:?})");
            }
        }
    }

    /// The composite masks agree with their point-set meanings across the
    /// corpus: INTERSECTS ⇔ shared point, COVERS ⇔ ⊇, DISJOINT ⇔ no
    /// shared point.
    #[test]
    fn composites_mean_their_point_set_definitions() {
        for (a, b) in pair_corpus() {
            let basic = classify(iv(a), iv(b));
            let intersects = a.0 < b.1 && b.0 < a.1;
            let covers = a.0 <= b.0 && b.1 <= a.1;
            assert_eq!(AllenMask::INTERSECTS.contains(basic), intersects);
            assert_eq!(AllenMask::COVERS.contains(basic), covers);
            assert_eq!(AllenMask::DISJOINT.contains(basic), !intersects);
            assert_eq!(
                AllenMask::COVERED_BY.contains(basic),
                b.0 <= a.0 && a.1 <= b.1
            );
        }
    }
}
