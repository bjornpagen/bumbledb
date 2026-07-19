//! The shared probe primitives of the two ctrl-byte open-addressed
//! structures — COLT's bucket maps (`colt`) and the sink `WordMap`
//! (`wordmap`). The structures stay independent (bucket-of-8 vs window
//! probing, different growth laws), but the tag/hash idiom and its
//! constants are ONE thing; before this module each was a byte-identical
//! copy on both sides of the boundary, drift waiting to happen.
//!
//! Everything here is an `#[inline(always)]` pure-ALU leaf: the probe
//! loops these feed are machine-checked call-free by
//! `scripts/check-asm.sh`, not trusted to the attribute.
#![allow(clippy::inline_always)]

/// The word-tuple probe hash (runtime length). `pub(crate)`: the image
/// cardinality counter's distinct-word set probes with the same hash —
/// its former private copy was exactly the drift this module exists to
/// prevent.
#[inline(always)]
pub(crate) fn hash_words(words: &[u64]) -> u64 {
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for w in words {
        h ^= *w;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    h
}

/// [`hash_words`] with the word count fixed at compile time — same
/// seed, same fold order, same constants, so the two forms are
/// hash-identical (pinned by test). Under const K, LLVM fully unrolls
/// the fold, hoists prefix hashes of batch-constant words out of the
/// caller's row loop, and fuses the key gather with the hash — the
/// free transformations runtime arity blocks (hand-fused variants
/// measured redundant or worse).
#[inline(always)]
pub(super) fn hash_core<const K: usize>(words: &[u64]) -> u64 {
    debug_assert_eq!(words.len(), K);
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for &w in &words[..K] {
        h ^= w;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    h
}

/// The 7-bit hash tag a ctrl byte carries (bit 7 marks occupancy).
#[inline(always)]
pub(super) fn ctrl_tag(hash: u64) -> u8 {
    0x80 | u8::try_from(hash >> 57).expect("7 bits")
}

/// SWAR zero-byte mask: bit 7 of each zero (empty) byte in `w` sets.
#[inline(always)]
pub(super) fn zero_byte_mask(w: u64) -> u64 {
    w.wrapping_sub(0x0101_0101_0101_0101) & !w & 0x8080_8080_8080_8080
}

/// SWAR byte-equality mask against a broadcast needle.
#[inline(always)]
pub(super) fn eq_byte_mask(w: u64, needle: u8) -> u64 {
    zero_byte_mask(w ^ (u64::from(needle) * 0x0101_0101_0101_0101))
}
