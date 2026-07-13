//! Exhaustive boundary suite for [`closed_member`] — the closed-target
//! membership judgment over its `[u64; 4]` bitset
//! (the crucible packet (git ecec1dc3), suite 2): every in-range
//! index against a structured pattern family, judged against a naive
//! bit walk that shares none of `closed_member`'s word/shift arithmetic.

use crate::schema::closed_member;

/// The naive oracle: walk all 256 bits by (word, bit) coordinates and
/// report whether any SET bit's position equals `id`. Out-of-range ids
/// are absent by construction — the walk never visits a position ≥ 256.
fn naive_member(members: &[u64; 4], id: u64) -> bool {
    let mut found = false;
    for (word_idx, &word) in members.iter().enumerate() {
        for bit in 0..64u64 {
            let position = u64::try_from(word_idx).expect("4 words") * 64 + bit;
            if word & (1 << bit) != 0 && position == id {
                found = true;
            }
        }
    }
    found
}

/// A splitmix64 step — the repo's no-dependency randomness.
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Low `k` bits set, `k ∈ 0..=256`: the prefix family. `k = 0` is the
/// empty set, `k = 256` all-set, and `k ∈ {63, 64, 65, 127, 128, 129,
/// 191, 192, 193}` are exactly the word-boundary straddles.
fn prefix_pattern(k: usize) -> [u64; 4] {
    let mut members = [0u64; 4];
    for (word_idx, word) in members.iter_mut().enumerate() {
        let low = word_idx * 64;
        *word = match k.saturating_sub(low) {
            0 => 0,
            n if n >= 64 => u64::MAX,
            n => (1 << n) - 1,
        };
    }
    members
}

/// The ids every pattern is probed with: all 256 in-range ids
/// exhaustively, plus the out-of-range probes (word-boundary neighbors
/// beyond the roster cap, the u32/u64 extremes — all must be absent).
fn probe_ids() -> Vec<u64> {
    let mut ids: Vec<u64> = (0..256).collect();
    ids.extend([
        256,
        257,
        319,
        320,
        383,
        384,
        447,
        448,
        511,
        512,
        1 << 32,
        u64::MAX - 1,
        u64::MAX,
    ]);
    ids
}

/// Exhaustive: every probe id × the whole structured pattern family,
/// vectorized-arithmetic membership vs the naive bit walk.
///
/// Domain arithmetic — the claimed domain, counted and asserted:
///   patterns: 257 prefix patterns (low k bits set, k = 0..=256 —
///     includes empty, all-set, and the 63/64, 127/128, 191/192 word
///     boundaries), 257 suffix patterns (their bitwise complements,
///     covering the same boundaries from the other side), 256
///     singletons (exactly one bit set, every position), and 64
///     splitmix-filled random words = 834 patterns;
///   ids: all 256 in-range ids exhaustively + 13 out-of-range probes
///     = 269 ids.
/// Cells: 834 × 269 = 224,346, every one judged against the oracle.
#[test]
fn exhaustive_closed_member_matches_the_naive_bit_walk() {
    let mut patterns: Vec<[u64; 4]> = Vec::new();
    for k in 0..=256 {
        let prefix = prefix_pattern(k);
        patterns.push(prefix);
        patterns.push(prefix.map(|w| !w)); // the suffix (complement) family
    }
    for bit in 0..256usize {
        let mut singleton = [0u64; 4];
        singleton[bit / 64] = 1 << (bit % 64);
        patterns.push(singleton);
    }
    let mut state = 0xC105_EDBE_u64;
    for _ in 0..64 {
        patterns.push([0; 4].map(|_| splitmix(&mut state)));
    }
    assert_eq!(patterns.len(), 834, "257 + 257 + 256 + 64 patterns");

    let ids = probe_ids();
    assert_eq!(ids.len(), 269, "256 in-range + 13 out-of-range ids");

    let mut cells = 0u32;
    for members in &patterns {
        for &id in &ids {
            assert_eq!(
                closed_member(members, id),
                naive_member(members, id),
                "members {members:?}, id {id}"
            );
            cells += 1;
        }
    }
    assert_eq!(cells, 224_346, "834 × 269 cells enumerated");
}

/// The Miri-lane representative: the word-boundary prefixes, empty,
/// all-set, and the corner singletons — every in-range id, plus the
/// out-of-range probes. 10 patterns × 269 ids = 2,690 cells.
#[test]
fn representative_closed_member_boundaries() {
    let mut patterns: Vec<[u64; 4]> = [0usize, 63, 64, 128, 192, 256]
        .iter()
        .map(|&k| prefix_pattern(k))
        .collect();
    patterns.push([0; 4]);
    patterns.push([u64::MAX; 4]);
    patterns.push({
        let mut first = [0u64; 4];
        first[0] = 1;
        first
    });
    patterns.push({
        let mut last = [0u64; 4];
        last[3] = 1 << 63;
        last
    });
    for members in &patterns {
        for &id in &probe_ids() {
            assert_eq!(closed_member(members, id), naive_member(members, id));
        }
    }
}
