//! Bounded alignment buffers, reused only after a local contraction finishes.
//! Published identity and the resident arena never refer to this workspace.
use super::{Arena, Observation, Restricted, View};

pub(super) struct Workspace<const N: usize> {
    planes: [[u64; N]; 3],
    work: [u64; N],
}
impl<const N: usize> Workspace<N> {
    pub(super) fn new() -> Self {
        Self {
            planes: [[0; N]; 3],
            work: [0; N],
        }
    }
}

const LOW: [u64; 6] = [
    0x5555_5555_5555_5555,
    0x3333_3333_3333_3333,
    0x0f0f_0f0f_0f0f_0f0f,
    0x00ff_00ff_00ff_00ff,
    0x0000_ffff_0000_ffff,
    0x0000_0000_ffff_ffff,
];

fn cofactor_into(data: &[u64], count: u32, position: u32, high: bool, out: &mut [u64]) {
    assert!(position < count);
    assert_eq!(data.len(), (1usize << count).div_ceil(64));
    let cells = 1usize << (count - 1);
    assert_eq!(out.len(), cells.div_ceil(64));
    // The buffer may contain a previous plane or a previous recursive leaf.
    out.fill(0);
    if position >= 6 {
        let stride = 1usize << (position - 6);
        for (output, block) in out.chunks_mut(stride).zip(data.chunks_exact(2 * stride)) {
            let start = high as usize * stride;
            output.copy_from_slice(&block[start..start + stride]);
        }
    } else {
        let width = 1usize << position;
        let mask = (1u64 << width) - 1;
        for (i, &word) in data.iter().enumerate() {
            let word = word >> (high as usize * width);
            let mut packed = 0u64;
            for group in 0..32 / width {
                packed |= ((word >> (group * 2 * width)) & mask) << (group * width);
            }
            out[i / 2] |= packed << (32 * (i % 2));
        }
    }
    if cells < 64 {
        out[0] &= (1u64 << cells) - 1;
    }
}

fn broadcast_into(data: &[u64], count: u32, position: u32, out: &mut [u64]) {
    assert!(position <= count && count < 12);
    assert_eq!(data.len(), (1usize << count).div_ceil(64));
    let cells = 1usize << (count + 1);
    assert_eq!(out.len(), cells.div_ceil(64));
    if position >= 6 {
        let stride = 1usize << (position - 6);
        for (dst, src) in out.chunks_mut(2 * stride).zip(data.chunks(stride)) {
            dst[..stride].copy_from_slice(src);
            dst[stride..].copy_from_slice(src);
        }
    } else {
        for (i, output) in out.iter_mut().enumerate() {
            let mut spread = (data[i / 2] >> (32 * (i % 2))) & 0xffff_ffff;
            for level in (position..5).rev() {
                spread = (spread | (spread << (1 << level))) & LOW[level as usize];
            }
            *output = spread | (spread << (1 << position));
        }
    }
    if cells < 64 {
        out[0] &= (1u64 << cells) - 1;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Buffer {
    Borrowed,
    Plane,
    Work,
}

fn step<const N: usize>(
    words: &[u64],
    plane: &mut [u64; N],
    work: &mut [u64; N],
    current: Buffer,
    count: u32,
    position: u32,
    high: Option<bool>,
) -> Buffer {
    let before = (1usize << count).div_ceil(64);
    let after = (1usize << if high.is_some() { count - 1 } else { count + 1 }).div_ceil(64);
    assert!(before <= N && after <= N);
    let apply = |input: &[u64], output: &mut [u64]| {
        if let Some(high) = high {
            cofactor_into(input, count, position, high, output);
        } else {
            broadcast_into(input, count, position, output);
        }
    };
    match current {
        Buffer::Borrowed => {
            apply(words, &mut plane[..after]);
            Buffer::Plane
        }
        Buffer::Plane => {
            apply(&plane[..before], &mut work[..after]);
            Buffer::Work
        }
        Buffer::Work => {
            apply(&work[..before], &mut plane[..after]);
            Buffer::Plane
        }
    }
}

#[derive(Clone, Copy)]
enum Plane<'a> {
    Constant(bool),
    Borrowed { words: &'a [u64], flip: u64 },
    Aligned { flip: u64 },
}
impl<'a> Plane<'a> {
    fn new<const K: u32, const N: usize>(
        arena: &'a Arena<K>,
        view: Restricted,
        target: u64,
        plane: &mut [u64; N],
        work: &mut [u64; N],
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
                let mut current = Buffer::Borrowed;
                let mut pinned = view.pinned;
                while pinned != 0 {
                    let bit = 1u64 << pinned.trailing_zeros();
                    let position = (variables & (bit - 1)).count_ones();
                    current = step(
                        words,
                        plane,
                        work,
                        current,
                        variables.count_ones(),
                        position,
                        Some(view.values & bit != 0),
                    );
                    stats.scratch_alignments += 1;
                    variables &= !bit;
                    pinned &= !bit;
                }
                let mut missing = target & !variables;
                while missing != 0 {
                    let bit = 1u64 << missing.trailing_zeros();
                    let position = (variables & (bit - 1)).count_ones();
                    current = step(
                        words,
                        plane,
                        work,
                        current,
                        variables.count_ones(),
                        position,
                        None,
                    );
                    stats.scratch_alignments += 1;
                    variables |= bit;
                    missing &= !bit;
                }
                assert_eq!(variables, target);
                let flip = if complemented { !0 } else { 0 };
                if current == Buffer::Borrowed {
                    stats.borrowed_planes += 1;
                    Self::Borrowed { words, flip }
                } else {
                    if current == Buffer::Work {
                        let len = (1usize << variables.count_ones()).div_ceil(64);
                        plane[..len].copy_from_slice(&work[..len]);
                    }
                    stats.aligned_planes += 1;
                    Self::Aligned { flip }
                }
            }
        }
    }

    #[inline]
    fn word(&self, i: usize, aligned: &[u64]) -> u64 {
        match self {
            Self::Constant(value) => {
                if *value {
                    !0
                } else {
                    0
                }
            }
            Self::Borrowed { words, flip } => words[i] ^ flip,
            Self::Aligned { flip } => aligned[i] ^ flip,
        }
    }
}

pub(super) fn classify<const K: u32, const N: usize>(
    arena: &Arena<K>,
    views: [Restricted; 3],
    variables: u64,
    workspace: &mut Workspace<N>,
    stats: &mut Observation,
) -> u8 {
    let mut planes = [Plane::Constant(false); 3];
    for (i, view) in views.into_iter().enumerate() {
        planes[i] = Plane::new(
            arena,
            view,
            variables,
            &mut workspace.planes[i],
            &mut workspace.work,
            stats,
        );
    }
    let cells = 1usize << variables.count_ones();
    let valid = if cells < 64 { (1u64 << cells) - 1 } else { !0 };
    let mut sig = 0;
    for i in 0..cells.div_ceil(64) {
        stats.word_batches += 1;
        let s = planes[0].word(i, &workspace.planes[0]) & valid;
        let a = planes[1].word(i, &workspace.planes[1]);
        let b = planes[2].word(i, &workspace.planes[2]);
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
    use super::super::raw::word_kernels;
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
    fn reused_axis_buffers_match_allocated_and_pointwise_kernels() {
        let mut buffer = [!0u64; 64];
        for count in 0..=12 {
            for seed in 0..8 {
                let mut input: Vec<_> = (0..(1usize << count).div_ceil(64))
                    .map(|i| mix(i as u64 + seed * 101))
                    .collect();
                if count < 6 {
                    input[0] &= (1u64 << (1usize << count)) - 1;
                }
                for position in 0..count {
                    for high in [false, true] {
                        buffer.fill(!0);
                        let cells = 1usize << (count - 1);
                        let out = &mut buffer[..cells.div_ceil(64)];
                        cofactor_into(&input, count, position, high, out);
                        assert_eq!(out, word_kernels::cofactor(&input, count, position, high));
                        for w in 0..cells {
                            let low = (1usize << position) - 1;
                            let source =
                                (w & low) | ((w & !low) << 1) | ((high as usize) << position);
                            assert_eq!(
                                (out[w / 64] >> (w % 64)) & 1,
                                (input[source / 64] >> (source % 64)) & 1
                            );
                        }
                    }
                }
                if count < 12 {
                    for position in 0..=count {
                        buffer.fill(!0);
                        let cells = 1usize << (count + 1);
                        let out = &mut buffer[..cells.div_ceil(64)];
                        broadcast_into(&input, count, position, out);
                        assert_eq!(out, word_kernels::broadcast(&input, count, position));
                        for w in 0..cells {
                            let low = (1usize << position) - 1;
                            let source = (w & low) | ((w >> 1) & !low);
                            assert_eq!(
                                (out[w / 64] >> (w % 64)) & 1,
                                (input[source / 64] >> (source % 64)) & 1
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn reused_planes_match_scattered_assignments_without_alignment_allocations() {
        let roster = [0, 3, 9, 12, 18, 27, 32, 35, 44, 51, 58, 61]
            .iter()
            .fold(0u64, |m, &v| m | (1 << v));
        let mut arena = Arena::<12>::new(62, &(0..62).rev().collect::<Vec<_>>());
        let mut stats = Observation::default();
        let mut workspace = Workspace::<64>::new();
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
            let root = arena.table(
                variables,
                (0..cells.div_ceil(64))
                    .map(|w| mix(seed * 31 + w as u64))
                    .collect(),
            );
            let variables = arena.variables(root);
            let before = (arena.nodes(), arena.bytes());
            for pattern in [0, !0, 0x5555_5555_5555_5555, mix(seed + 413)] {
                let pinned = variables & pattern;
                for values in [0, pinned, pinned & mix(seed + 27)] {
                    for target in [variables & !pinned, roster & !pinned] {
                        for flip in [0, 1] {
                            let view = Restricted {
                                root: root ^ flip,
                                pinned,
                                values,
                            };
                            // Reuse dirty buffers across all masks, sizes and polarities.
                            let plane = Plane::new(
                                &arena,
                                view,
                                target,
                                &mut workspace.planes[0],
                                &mut workspace.work,
                                &mut stats,
                            );
                            for i in 0..1usize << target.count_ones() {
                                assert_eq!(
                                    (plane.word(i / 64, &workspace.planes[0]) >> (i % 64)) & 1 != 0,
                                    arena.evaluate(view.root, scatter(i, target) | values)
                                );
                            }
                        }
                    }
                }
            }
            assert_eq!((arena.nodes(), arena.bytes()), before);
        }
        assert!(stats.borrowed_planes > 0 && stats.aligned_planes > 0);
        assert!(stats.scratch_alignments > 0);
        assert_eq!(stats.alignment_allocations, 0);
        assert_eq!(std::mem::size_of::<Workspace<1>>(), 32);
        assert_eq!(std::mem::size_of::<Workspace<8>>(), 256);
        assert_eq!(std::mem::size_of::<Workspace<64>>(), 2048);
    }
}
