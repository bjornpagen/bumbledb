//! Exact local-table kernels. Scalar and word paths share the same normal form.
const LOW: [u64; 6] = [
    0x5555_5555_5555_5555,
    0x3333_3333_3333_3333,
    0x0f0f_0f0f_0f0f_0f0f,
    0x00ff_00ff_00ff_00ff,
    0x0000_ffff_0000_ffff,
    0x0000_0000_ffff_ffff,
];
pub fn literal_word(position: u32, word: usize) -> u64 {
    if position < 6 {
        !LOW[position as usize]
    } else if word >> (position - 6) & 1 != 0 {
        !0
    } else {
        0
    }
}
pub fn irrelevant(data: &[u64], position: u32) -> bool {
    if position < 6 {
        data.iter()
            .all(|w| (w ^ (w >> (1 << position))) & LOW[position as usize] == 0)
    } else {
        let stride = 1usize << (position - 6);
        data.chunks_exact(2 * stride)
            .all(|block| block[..stride] == block[stride..])
    }
}
pub fn cofactor(data: &[u64], count: u32, position: u32, high: bool) -> Vec<u64> {
    assert!(position < count);
    let cells = 1usize << (count - 1);
    let mut out = vec![0; cells.div_ceil(64)];
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
    out
}
// Insert an ignored local axis. Numeric local-axis order is preserved. The
// result is temporary alignment storage, never an interned/canonical node.
pub fn broadcast(data: &[u64], count: u32, position: u32) -> Vec<u64> {
    assert!(position <= count && count < 12);
    assert_eq!(data.len(), (1usize << count).div_ceil(64));
    let cells = 1usize << (count + 1);
    let mut out = vec![0; cells.div_ceil(64)];
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
    out
}
fn binary<const OP: u8>(a: u64, b: u64) -> u64 {
    match OP {
        0 => 0,
        1 => !(a | b),
        2 => !a & b,
        3 => !a,
        4 => a & !b,
        5 => !b,
        6 => a ^ b,
        7 => !(a & b),
        8 => a & b,
        9 => !(a ^ b),
        10 => b,
        11 => !a | b,
        12 => a,
        13 => a | !b,
        14 => a | b,
        15 => !0,
        _ => unreachable!(),
    }
}
pub fn combine(op: u8, a: &[u64], b: &[u64]) -> Vec<u64> {
    assert_eq!(a.len(), b.len());
    fn fill<const OP: u8>(a: &[u64], b: &[u64]) -> Vec<u64> {
        a.iter().zip(b).map(|(&a, &b)| binary::<OP>(a, b)).collect()
    }
    macro_rules! select {($($n:literal),*)=>{match op { $($n=>fill::<$n>(a,b),)* _=>unreachable!() }};}
    select!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15)
}
// Aligned local conditional: LLVM may lower this to vector BSL/BIT/BIF.
// Assembly inspection, not the source expression, establishes actual lowering.
pub fn select(selector: &[u64], high: &[u64], low: &[u64]) -> Vec<u64> {
    assert_eq!(selector.len(), high.len());
    assert_eq!(selector.len(), low.len());
    selector
        .iter()
        .zip(high)
        .zip(low)
        .map(|((&s, &h), &l)| (s & h) | (!s & l))
        .collect()
}
// Old bit axis i moves to destinations[i]. This is the same broadword swap
// decomposition used by the lab's general permutation, kept standalone here.
pub fn permute(words: &mut [u64], destinations: &[u32]) {
    assert!(destinations.len() <= 20);
    assert_eq!(words.len(), (1usize << destinations.len()).div_ceil(64));
    let mut seen = 0u64;
    for &v in destinations {
        assert!((v as usize) < destinations.len() && seen >> v & 1 == 0);
        seen |= 1 << v;
    }
    let mut labels: Vec<_> = (0..destinations.len() as u32).collect();
    for (old, &destination) in destinations.iter().enumerate() {
        let from = labels.iter().position(|&v| v == old as u32).unwrap() as u32;
        if from == destination {
            continue;
        }
        let i = from.min(destination);
        let j = from.max(destination);
        labels.swap(from as usize, destination as usize);
        if j < 6 {
            let distance = (1 << j) - (1 << i);
            let mask = !LOW[i as usize] & LOW[j as usize];
            for word in words.iter_mut() {
                let moved = (*word ^ (*word >> distance)) & mask;
                *word ^= moved ^ (moved << distance);
            }
        } else if i < 6 {
            let stride = 1usize << (j - 6);
            let shift = 1 << i;
            let mask = LOW[i as usize];
            for block in words.chunks_mut(2 * stride) {
                let (low, high) = block.split_at_mut(stride);
                for (a, b) in low.iter_mut().zip(high) {
                    let moved = ((*a >> shift) ^ *b) & mask;
                    *a ^= moved << shift;
                    *b ^= moved;
                }
            }
        } else {
            let a = 1usize << (i - 6);
            let b = 1usize << (j - 6);
            for index in 0..words.len() {
                if index & a != 0 && index & b == 0 {
                    words.swap(index, index ^ a ^ b);
                }
            }
        }
    }
}
