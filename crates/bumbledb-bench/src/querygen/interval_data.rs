use crate::corpus_gen::Rng;

pub const PER_GROUP: u64 = 4;

pub const GROUP_SPAN: u64 = 1_024;

pub const I64_ORIGIN: i64 = 2_000_000_000_000_000;

pub const U64_ORIGIN: u64 = 1_024;

pub const U64_SENTINEL_END: u64 = i64::MAX as u64;

fn offsets(seed: u64, group: u64, k: u64) -> (u64, u64, bool) {
    debug_assert!(k < PER_GROUP);
    let mut rng = Rng::new(seed ^ group.wrapping_mul(0xD1B5_4A32_D192_ED03));

    let w0 = 32 + rng.range(64);
    let w1 = 32 + rng.range(64);
    let gap = 16 + rng.range(32);
    let inset = 8 + rng.range(16);
    let parent_start = w0 + w1 + gap;
    let parent_end = parent_start + 256;
    match k {
        0 => (0, w0, false),
        1 => (w0, w0 + w1, false), 
        2 => (parent_start, parent_end, false),
        _ if group.is_multiple_of(3) => (parent_end + gap, 0, true), 
        _ => (parent_start + inset, parent_end - inset, false),      
    }
}

/// # Panics
/// On a programmer-invariant violation only: the group-local offsets
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

/// # Panics
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    Equal,
    Adjacent,
    Nested,
    Ray,
}

fn draw_rung(rng: &mut Rng) -> Rung {
    match rng.range(8) {
        0..=2 => Rung::Equal,
        3 | 4 => Rung::Adjacent,
        5 | 6 => Rung::Nested,
        _ => Rung::Ray,
    }
}

/// # Panics
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

            let (start, end) = group_i64(seed, group, 2);
            let inset = 1 + i64::try_from(rng.range(64)).expect("small");
            (start + inset, end - inset)
        }
        Rung::Ray => (group_i64(seed, group, 2).0, i64::MAX),
    };
    (interval, drawn)
}

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

    #[test]
    fn k0_and_k1_are_adjacent() {
        for group in 0..64 {
            assert_eq!(group_i64(SEED, group, 0).1, group_i64(SEED, group, 1).0);
            assert_eq!(group_u64(SEED, group, 0).1, group_u64(SEED, group, 1).0);
        }
    }

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

    #[test]
    fn groups_are_disjoint() {
        for group in 0..8 {

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
