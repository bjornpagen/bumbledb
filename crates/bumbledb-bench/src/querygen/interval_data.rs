//! The dedicated interval-value generator (PRD 23 § data generation):
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
}
