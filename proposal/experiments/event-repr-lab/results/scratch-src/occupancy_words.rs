//! Borrow raw tables, cofactor pinned axes, then broadcast onto one bounded cube.
//! Temporary aligned vectors are deliberately separate from resident storage.
use super::raw::word_kernels;
use super::{Arena, Observation, Restricted, View};
use std::borrow::Cow;

enum Plane<'a> {
    Constant(bool),
    Table { words: Cow<'a, [u64]>, flip: u64 },
}
impl<'a> Plane<'a> {
    fn new<const K: u32>(
        arena: &'a Arena<K>,
        view: Restricted,
        target: u64,
        stats: &mut Observation,
    ) -> Self {
        assert!(target.count_ones() <= K);
        match arena.view(view.root) {
            View::Constant(value) => {
                stats.constant_planes += 1;
                Self::Constant(value)
            }
            View::Branch { .. } => {
                unreachable!("a canonical unpinned branch has more than K remaining axes")
            }
            View::Table {
                mut variables,
                words,
                complemented,
            } => {
                assert_eq!(view.pinned & !variables, 0);
                assert_eq!(view.values & !view.pinned, 0);
                assert_eq!(variables & !view.pinned & !target, 0);
                assert_eq!(target & view.pinned, 0);
                let mut data = Cow::Borrowed(words);
                let mut pinned = view.pinned;
                while pinned != 0 {
                    let bit = 1u64 << pinned.trailing_zeros();
                    let position = (variables & (bit - 1)).count_ones();
                    data = Cow::Owned(word_kernels::cofactor(
                        &data,
                        variables.count_ones(),
                        position,
                        view.values & bit != 0,
                    ));
                    stats.alignment_allocations += 1;
                    variables &= !bit;
                    pinned &= !bit;
                }
                let mut missing = target & !variables;
                while missing != 0 {
                    let bit = 1u64 << missing.trailing_zeros();
                    let position = (variables & (bit - 1)).count_ones();
                    data = Cow::Owned(word_kernels::broadcast(
                        &data,
                        variables.count_ones(),
                        position,
                    ));
                    stats.alignment_allocations += 1;
                    variables |= bit;
                    missing &= !bit;
                }
                assert_eq!(variables, target);
                if matches!(data, Cow::Borrowed(_)) {
                    stats.borrowed_planes += 1;
                } else {
                    stats.aligned_planes += 1;
                }
                Self::Table {
                    words: data,
                    flip: if complemented { !0 } else { 0 },
                }
            }
        }
    }
    #[inline]
    fn word(&self, i: usize) -> u64 {
        match self {
            Self::Constant(value) => {
                if *value {
                    !0
                } else {
                    0
                }
            }
            Self::Table { words, flip } => words[i] ^ flip,
        }
    }
}

pub(super) fn classify<const K: u32>(
    arena: &Arena<K>,
    views: [Restricted; 3],
    variables: u64,
    stats: &mut Observation,
) -> u8 {
    let planes = views.map(|v| Plane::new(arena, v, variables, stats));
    let cells = 1usize << variables.count_ones();
    let valid = if cells < 64 { (1u64 << cells) - 1 } else { !0 };
    let mut sig = 0;
    for i in 0..cells.div_ceil(64) {
        stats.word_batches += 1;
        let s = planes[0].word(i) & valid;
        let a = planes[1].word(i);
        let b = planes[2].word(i);
        sig |= (s & !a & !b != 0) as u8;
        sig |= ((s & !a & b != 0) as u8) << 1;
        sig |= ((s & a & !b != 0) as u8) << 2;
        sig |= ((s & a & b != 0) as u8) << 3;
        if sig == 15 {
            break;
        }
    }
    sig
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mix(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9e3779b97f4a7c15);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        x ^ (x >> 31)
    }
    fn scatter(mut value: usize, mut mask: u64) -> u64 {
        let mut out = 0;
        while mask != 0 {
            let bit = mask & mask.wrapping_neg();
            if value & 1 != 0 {
                out |= bit;
            }
            value >>= 1;
            mask &= !bit;
        }
        out
    }
    #[test]
    fn broadcast_matches_pointwise_axis_insertion() {
        for count in 0..12 {
            for position in 0..=count {
                for seed in 0..8 {
                    let mut input: Vec<_> = (0..(1usize << count).div_ceil(64))
                        .map(|i| mix(i as u64 + seed * 101))
                        .collect();
                    if count < 6 {
                        input[0] &= (1u64 << (1usize << count)) - 1;
                    }
                    let output = word_kernels::broadcast(&input, count, position);
                    for w in 0..1usize << (count + 1) {
                        let low = (1usize << position) - 1;
                        let source = (w & low) | ((w >> 1) & !low);
                        assert_eq!(
                            (output[w / 64] >> (w % 64)) & 1,
                            (input[source / 64] >> (source % 64)) & 1
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn aligned_borrowed_views_match_scattered_assignments() {
        let axes = [0, 3, 9, 12, 18, 27, 32, 35, 44, 51, 58, 61];
        let roster = axes.iter().fold(0u64, |m, &v| m | (1 << v));
        let mut arena = Arena::<12>::new(62, &(0..62).rev().collect::<Vec<_>>());
        let mut stats = Observation::default();
        for seed in 0..96u64 {
            let variables = scatter(
                if seed < 8 {
                    4095
                } else {
                    mix(seed) as usize & 4095
                },
                roster,
            );
            let cells = 1usize << variables.count_ones();
            let data: Vec<_> = (0..cells.div_ceil(64))
                .map(|w| mix(seed * 31 + w as u64))
                .collect();
            let root = arena.table(variables, data);
            let variables = arena.variables(root);
            let before = (arena.nodes(), arena.bytes());
            for pattern in [0, !0, 0x5555_5555_5555_5555, mix(seed + 413)] {
                let pinned = variables & pattern;
                let remaining = variables & !pinned;
                for values in [0, pinned, pinned & mix(seed + 27)] {
                    for target in [remaining, roster & !pinned] {
                        for flip in [0, 1] {
                            let view = Restricted {
                                root: root ^ flip,
                                pinned,
                                values,
                            };
                            let plane = Plane::new(&arena, view, target, &mut stats);
                            for i in 0..1usize << target.count_ones() {
                                let expected =
                                    arena.evaluate(view.root, scatter(i, target) | values);
                                assert_eq!((plane.word(i / 64) >> (i % 64)) & 1 != 0, expected);
                            }
                        }
                    }
                }
            }
            assert_eq!((arena.nodes(), arena.bytes()), before);
        }
        assert!(stats.borrowed_planes > 0 && stats.aligned_planes > 0);
        assert!(stats.alignment_allocations > 0);
    }
}
