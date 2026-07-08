/// The probe hash for a key — exposed so the vectorized executor's phase 1
/// can compute all hashes (pure ALU) before phase 2 issues any bucket load
/// (D4's two-phase probing, the 30-execution doc).
#[must_use]
#[inline(always)]
pub fn hash_key(words: &[u64]) -> u64 {
    hash_words(words)
}

#[inline(always)]
pub(super) fn hash_words(words: &[u64]) -> u64 {
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for w in words {
        h ^= *w;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    h
}
