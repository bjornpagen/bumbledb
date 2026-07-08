pub(super) fn hash_words(words: &[u64]) -> u64 {
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
/// free transformations runtime arity blocks (docs/silicon2/03,
/// exp 15: hand-fused variants measured redundant or worse).
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
