use crate::schema::{AxiomIndex, MemberSet};

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

fn member_set(words: &[u64; 4]) -> MemberSet {
    let mut members = MemberSet::empty();
    for index in 0..256u16 {
        let word = usize::from(index / 64);
        if words[word] & (1 << (index % 64)) != 0 {
            members.insert(AxiomIndex(u8::try_from(index).expect("index < 256")));
        }
    }
    members
}

fn contains(members: &MemberSet, id: u64) -> bool {
    AxiomIndex::try_from(id).is_ok_and(|index| members.contains(index))
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

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

#[test]
fn exhaustive_member_set_matches_the_naive_bit_walk() {
    let mut patterns: Vec<[u64; 4]> = Vec::new();
    for k in 0..=256 {
        let prefix = prefix_pattern(k);
        patterns.push(prefix);
        patterns.push(prefix.map(|w| !w));
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
    for words in &patterns {
        let members = member_set(words);
        for &id in &ids {
            assert_eq!(
                contains(&members, id),
                naive_member(words, id),
                "members {words:?}, id {id}"
            );
            cells += 1;
        }
    }
    assert_eq!(cells, 224_346, "834 × 269 cells enumerated");
}

#[test]
fn representative_member_set_boundaries() {
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
    for words in &patterns {
        let members = member_set(words);
        for &id in &probe_ids() {
            assert_eq!(contains(&members, id), naive_member(words, id));
        }
    }
}
