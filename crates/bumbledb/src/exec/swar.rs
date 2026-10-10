//! The probe primitives shared by the two ctrl-byte open-addressed structures,
//! COLT's bucket maps and the sink `WordMap`: one tag and hash implementation
//! behind independent probing and growth. Forced inlining keeps these pure-ALU
//! leaves inside their probe loops.
#![allow(clippy::inline_always)]
/// Mixes high bits down: tail-zero big-endian `bytes<N>` code words keep all
/// their entropy in the high bits, and without this whole code families would
/// share one home bucket.
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
    hash_words(&words[..K])
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
