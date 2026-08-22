//! The shared probe primitives of the two ctrl-byte open-addressed
//! structures — COLT's bucket maps (`colt`) and the sink `WordMap`
//! (`wordmap`). The structures stay independent (bucket-of-8 vs window
//! probing, different growth laws), but the tag/hash idiom and its
//! copy on both sides of the boundary, drift waiting to happen.
//! Everything here is an `#[inline(always)]` pure-ALU leaf: the probe
//! constants are ONE thing; before this module each was a byte-identical
#![allow(clippy::inline_always)]
/// Tail-zero big-endian `bytes<N>` code words (encoding.rs pads at the tail;
/// `fact_word.rs` reads big-endian) put ALL their entropy up there — whole code
/// families collapsed into one home bucket before this.
#[inline(always)]
fn avalanche(h: u64) -> u64 {
    let h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 32)
}

#[inline(always)]
pub(crate) fn hash_words(words: &[u64]) -> u64 {
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for w in words {
        h ^= *w;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    avalanche(h)
}

#[inline(always)]
pub(super) fn hash_core<const K: usize>(words: &[u64]) -> u64 {
    debug_assert_eq!(words.len(), K);
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for &w in &words[..K] {
        h ^= w;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    avalanche(h)
}

#[inline(always)]
pub(super) fn ctrl_tag(hash: u64) -> u8 {
    0x80 | u8::try_from(hash >> 57).expect("7 bits")
}

#[inline(always)]
pub(super) fn zero_byte_mask(w: u64) -> u64 {
    w.wrapping_sub(0x0101_0101_0101_0101) & !w & 0x8080_8080_8080_8080
}

#[inline(always)]
pub(super) fn eq_byte_mask(w: u64, needle: u8) -> u64 {
    zero_byte_mask(w ^ (u64::from(needle) * 0x0101_0101_0101_0101))
}
