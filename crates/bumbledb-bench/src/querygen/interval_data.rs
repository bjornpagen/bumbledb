//! The dedicated interval-value generator:
//! seeded, random-access interval mixes for the corpus's interval
//! columns. Every scalar-prefix collision group carries [`PER_GROUP`]
//! intervals in a fixed shape roster — **disjoint** (a gap), **adjacent**
//! (`end == next.start`, the neighbor-probe boundary as data, not just a
//! unit test), **nested**, and (every third group) a **`MAX_END`-sentinel
//! open end. Widths jitter per (seed, group); the adjacency and nesting
//! are exact by construction, so query literals recomputed from the same
//! function touch corpus intervals at precisely their endpoints.
//!
//! Group `g`'s intervals live in `[origin(g), origin(g) + GROUP_SPAN)`
//! (sentinel ends excepted), so distinct groups never overlap — the
//! collision structure is entirely within a group's scalar prefix.

use crate::gen::Rng;

/// Intervals per collision group.
pub const PER_GROUP: u64 = 4;

/// The span reserved per group in the element domain.
pub const GROUP_SPAN: u64 = 1_024;

/// I64 groups sit above the timestamp region, far from any scalar
/// vocabulary.
pub const I64_ORIGIN: i64 = 2_000_000_000_000_000;

/// U64 groups start above zero so a left-touching query literal always
/// has room.
pub const U64_ORIGIN: u64 = 1_024;

/// The U64 lane's sentinel end: the largest value inside the oracle
/// mapping (`docs/architecture/60-validation.md`: oracle-checked u64
/// data stays below 2⁶³ — full-range U64 is the encoding fuzz's lane,
/// not `SQLite`'s). The **genuine** `MAX_END` boundary is exercised by
/// the I64 lane, whose sentinel `i64::MAX` *is* that domain's
/// `Interval::MAX_END` and maps to `SQLite` exactly.
pub const U64_SENTINEL_END: u64 = i64::MAX as u64;

/// The group-local shape roster, in offsets within `0..GROUP_SPAN`:
/// `k = 0` and `k = 1` are the adjacent pair, `k = 2` is disjoint from
/// both (a gap) and parents `k = 3`'s nested interval — except every
/// third group, where `k = 3` is the open-ended sentinel instead.
fn offsets(seed: u64, group: u64, k: u64) -> (u64, u64, bool) {
    debug_assert!(k < PER_GROUP);
    let mut rng = Rng::new(seed ^ group.wrapping_mul(0xD1B5_4A32_D192_ED03));
    // Jittered but ordered widths: the adjacent pair, the gap, the nest.
    let w0 = 32 + rng.range(64);
    let w1 = 32 + rng.range(64);
    let gap = 16 + rng.range(32);
    let inset = 8 + rng.range(16);
    let parent_start = w0 + w1 + gap;
    let parent_end = parent_start + 256;
    match k {
        0 => (0, w0, false),
        1 => (w0, w0 + w1, false), // end == next.start with k = 0
        2 => (parent_start, parent_end, false),
        _ if group.is_multiple_of(3) => (parent_end + gap, 0, true), // sentinel end
        _ => (parent_start + inset, parent_end - inset, false),      // nested in k = 2
    }
}

/// Interval `k` of I64-element collision group `group`.
///
/// # Panics
///
/// On a programmer-invariant violation only: the group-local offsets
/// always fit the element domain.
#[must_use]
pub fn group_i64(seed: u64, group: u64, k: u64) -> (i64, i64) {
    let (lo, hi, sentinel) = offsets(seed, group, k);
    let base = I64_ORIGIN
        + i64::try_from(group % (i64::MAX as u64 / GROUP_SPAN / 4)).expect("fits")
            * i64::try_from(GROUP_SPAN).expect("fits");
    let start = base + i64::try_from(lo).expect("in span");
    if sentinel {
        (start, i64::MAX)
    } else {
        (start, base + i64::try_from(hi).expect("in span"))
    }
}

/// Interval `k` of U64-element collision group `group`.
///
/// # Panics
///
/// Never — documented for symmetry with [`group_i64`]; the arithmetic
/// is modular by construction.
#[must_use]
pub fn group_u64(seed: u64, group: u64, k: u64) -> (u64, u64) {
    let (lo, hi, sentinel) = offsets(seed, group, k);
    let base = U64_ORIGIN + (group % (u64::MAX / GROUP_SPAN / 4)) * GROUP_SPAN;
    if sentinel {
        (base + lo, U64_SENTINEL_END)
    } else {
        (base + lo, base + hi)
    }
}

/// One rung of the boundary-shape ladder — the relation a drawn query
/// literal bears to the group's corpus intervals. The ladder is
/// systematized for **every** interval literal draw: equal (an
/// exact corpus interval), adjacent (touching a corpus endpoint —
/// `[a,b) [b,c)` as query and data), nested (strictly inside the
/// group's parent), and ray (an open end at the lane's sentinel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    Equal,
    Adjacent,
    Nested,
    Ray,
}

/// A rung draw: equal and adjacent dominate (they are where boundary
/// bugs live); rays are the heavy filters, drawn least.
fn draw_rung(rng: &mut Rng) -> Rung {
    match rng.range(8) {
        0..=2 => Rung::Equal,
        3 | 4 => Rung::Adjacent,
        5 | 6 => Rung::Nested,
        _ => Rung::Ray,
    }
}

/// A ladder literal over the I64 lane: `(start, end)` in the drawn
/// group's neighborhood, at a drawn rung.
///
/// # Panics
///
/// Never in practice: the group-local arithmetic stays inside the span.
#[must_use]
pub fn ladder_i64(seed: u64, group: u64, rng: &mut Rng) -> ((i64, i64), Rung) {
    let drawn = draw_rung(rng);
    let interval = match drawn {
        Rung::Equal => group_i64(seed, group, rng.range(PER_GROUP)),
        Rung::Adjacent => {
            let width = 16 + i64::try_from(rng.range(48)).expect("small");
            if rng.chance(1, 2) {
                let (s0, _) = group_i64(seed, group, 0);
                (s0 - width, s0)
            } else {
                let (_, e1) = group_i64(seed, group, 1);
                (e1, e1 + width)
            }
        }
        Rung::Nested => {
            // Strictly inside the parent (k = 2, width 256): both
            // endpoints interior.
            let (start, end) = group_i64(seed, group, 2);
            let inset = 1 + i64::try_from(rng.range(64)).expect("small");
            (start + inset, end - inset)
        }
        Rung::Ray => (group_i64(seed, group, 2).0, i64::MAX),
    };
    (interval, drawn)
}

/// A ladder literal over the U64 lane — [`ladder_i64`]'s twin; the ray
/// rung ends at [`U64_SENTINEL_END`] (the lane's sentinel, inside the
/// oracle mapping).
#[must_use]
pub fn ladder_u64(seed: u64, group: u64, rng: &mut Rng) -> ((u64, u64), Rung) {
    let drawn = draw_rung(rng);
    let interval = match drawn {
        Rung::Equal => group_u64(seed, group, rng.range(PER_GROUP)),
        Rung::Adjacent => {
            let width = 16 + rng.range(48);
            if rng.chance(1, 2) {
                let (s0, _) = group_u64(seed, group, 0);
                (s0 - width, s0)
            } else {
                let (_, e1) = group_u64(seed, group, 1);
                (e1, e1 + width)
            }
        }
        Rung::Nested => {
            let (start, end) = group_u64(seed, group, 2);
            let inset = 1 + rng.range(64);
            (start + inset, end - inset)
        }
        Rung::Ray => (group_u64(seed, group, 2).0, U64_SENTINEL_END),
    };
    (interval, drawn)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 5;

    #[test]
    fn every_interval_is_nonempty() {
        for group in 0..64 {
            for k in 0..PER_GROUP {
                let (s, e) = group_i64(SEED, group, k);
                assert!(s < e, "i64 group {group} k {k}");
                let (s, e) = group_u64(SEED, group, k);
                assert!(s < e, "u64 group {group} k {k}");
            }
        }
    }

    /// The adjacent pair touches exactly: `[a,b) [b,c)`.
    #[test]
    fn k0_and_k1_are_adjacent() {
        for group in 0..64 {
            assert_eq!(group_i64(SEED, group, 0).1, group_i64(SEED, group, 1).0);
            assert_eq!(group_u64(SEED, group, 0).1, group_u64(SEED, group, 1).0);
        }
    }

    /// The pair and the parent are disjoint (a real gap), and the fourth
    /// interval nests strictly inside the parent when it is not the
    /// sentinel.
    #[test]
    fn gap_nesting_and_sentinel_mix() {
        let mut sentinels = 0;
        for group in 0..64 {
            let pair_end = group_i64(SEED, group, 1).1;
            let (parent_start, parent_end) = group_i64(SEED, group, 2);
            assert!(pair_end < parent_start, "the gap is real");
            let (s, e) = group_i64(SEED, group, 3);
            if e == i64::MAX {
                sentinels += 1;
                assert!(s > parent_end, "the sentinel starts past the parent");
            } else {
                assert!(parent_start < s && e < parent_end, "strict nesting");
            }
        }
        assert!(sentinels > 0, "sentinel ends occur");
        assert!(sentinels < 64, "bounded ends occur");
    }

    /// Distinct groups' bounded intervals never overlap (the collision
    /// structure is the scalar prefix's, not an accident of ranges).
    #[test]
    fn groups_are_disjoint() {
        for group in 0..8 {
            // k = 2 is never the sentinel; it ends the group's span.
            let bounded_end = group_i64(SEED, group, 2).1;
            let next_start = group_i64(SEED, group + 1, 0).0;
            assert!(bounded_end <= next_start, "ordered groups");
        }
    }

    #[test]
    fn generation_is_a_pure_function() {
        assert_eq!(group_u64(SEED, 9, 3), group_u64(SEED, 9, 3));
        assert_ne!(group_u64(SEED, 9, 0), group_u64(SEED + 1, 9, 0));
    }

    /// The ladder draws every rung on both lanes, each literal nonempty,
    /// and the rungs mean what they say: equal recomputes a corpus
    /// interval, adjacent touches one exactly, nested sits strictly
    /// inside the parent, a ray ends at the lane's sentinel.
    #[test]
    fn the_ladder_draws_every_rung_and_each_rung_is_exact() {
        let mut rng = Rng::new(SEED);
        let mut seen = [0u64; 4];
        for group in 0..64 {
            let ((start, end), drawn) = ladder_i64(SEED, group, &mut rng);
            assert!(start < end, "i64 ladder literals are nonempty");
            match drawn {
                Rung::Equal => {
                    assert!((0..PER_GROUP).any(|k| group_i64(SEED, group, k) == (start, end)));
                    seen[0] += 1;
                }
                Rung::Adjacent => {
                    let (s0, _) = group_i64(SEED, group, 0);
                    let (_, e1) = group_i64(SEED, group, 1);
                    assert!(end == s0 || start == e1, "the touch is exact");
                    seen[1] += 1;
                }
                Rung::Nested => {
                    let (ps, pe) = group_i64(SEED, group, 2);
                    assert!(ps < start && end < pe, "strict nesting");
                    seen[2] += 1;
                }
                Rung::Ray => {
                    assert_eq!(end, i64::MAX);
                    seen[3] += 1;
                }
            }
            let ((start, end), drawn) = ladder_u64(SEED, group, &mut rng);
            assert!(start < end, "u64 ladder literals are nonempty");
            if drawn == Rung::Ray {
                assert_eq!(end, U64_SENTINEL_END);
            }
        }
        assert!(seen.iter().all(|count| *count > 0), "every rung: {seen:?}");
    }
}
