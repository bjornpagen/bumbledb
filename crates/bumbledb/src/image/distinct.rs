//! Exact per-column distinct-value counts, persisted WITH the image:
//! computed once while the build's rows are still warm and extended
//! number, never walks a column. The former shape — a lazy exact
//! O(rows) open-addressed walk plus a 2×rows scratch allocation per
//! commit pay it again; persisting the counting state makes the append
//! path O(tail) exact (the image oracle's served-vs-rebuilt equality
//! whole count pass per demanded column and every re-prepare after a

use super::{Column, RelationImage};

impl RelationImage {
    #[must_use]
    pub fn distinct_count(&self, column: usize) -> u64 {
        self.distincts[column].count()
    }
}

#[derive(Debug, Clone)]
pub(super) enum DistinctState {
    Words(WordSet),
    Bytes(ByteSet),
}

impl DistinctState {
    fn count(&self) -> u64 {
        match self {
            Self::Words(set) => set.count(),
            Self::Bytes(set) => set.count(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct WordSet {
    slots: Vec<u64>,

    len: u64,
    zero_seen: bool,
}

impl WordSet {
    fn with_hint(rows: usize) -> Self {
        // before the pass, so start at rows/8 (min 16) and double when

        let capacity = (rows / 8).max(16).next_power_of_two();
        Self {
            slots: vec![0; capacity],
            len: 0,
            zero_seen: false,
        }
    }

    fn count(&self) -> u64 {
        self.len + u64::from(self.zero_seen)
    }

    fn insert_all(&mut self, words: &[u64]) {
        for &word in words {
            self.insert(word);
        }
    }

    fn insert(&mut self, word: u64) {
        if word == 0 {
            self.zero_seen = true;
            return;
        }
        // Grow before the probe at the 0.5 max load, so probe chains

        if (usize::try_from(self.len).expect("64-bit usize") + 1) * 2 > self.slots.len() {
            self.grow();
        }
        let mask = self.slots.len() - 1;

        let h = crate::exec::swar::hash_words(std::slice::from_ref(&word));
        let mut idx = usize::try_from(h).expect("64-bit usize") & mask;
        loop {
            let slot = self.slots[idx];
            if slot == word {
                return;
            }
            if slot == 0 {
                self.slots[idx] = word;
                self.len += 1;
                return;
            }
            idx = (idx + 1) & mask;
        }
    }

    fn grow(&mut self) {
        let doubled = (self.slots.len() * 2).max(16);
        let old = std::mem::replace(&mut self.slots, vec![0; doubled]);
        let mask = self.slots.len() - 1;
        for word in old {
            if word == 0 {
                continue;
            }
            let h = crate::exec::swar::hash_words(std::slice::from_ref(&word));
            let mut idx = usize::try_from(h).expect("64-bit usize") & mask;
            while self.slots[idx] != 0 {
                idx = (idx + 1) & mask;
            }
            self.slots[idx] = word;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct ByteSet {
    mask: [u64; 4],
}

impl ByteSet {
    fn count(&self) -> u64 {
        self.mask.iter().map(|w| u64::from(w.count_ones())).sum()
    }

    fn insert_all(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.mask[usize::from(byte >> 6)] |= 1 << (byte & 63);
        }
    }
}

pub(super) fn count_columns(
    columns: &[Column],
    row_count: usize,
    words: &[u64],
    bytes: &[u8],
) -> Box<[DistinctState]> {
    let mut states: Box<[DistinctState]> = columns
        .iter()
        .map(|column| match column {
            Column::Words { .. } => DistinctState::Words(WordSet::with_hint(row_count)),
            Column::Bytes { .. } => DistinctState::Bytes(ByteSet::default()),
        })
        .collect();
    extend_columns(&mut states, columns, 0, row_count, words, bytes);
    states
}

pub(super) fn extend_columns(
    states: &mut [DistinctState],
    columns: &[Column],
    from: usize,
    row_count: usize,
    words: &[u64],
    bytes: &[u8],
) {
    for (state, column) in states.iter_mut().zip(columns) {
        match (state, *column) {
            (DistinctState::Words(set), Column::Words { start }) => {
                set.insert_all(&words[start + from..start + row_count]);
            }
            (DistinctState::Bytes(set), Column::Bytes { start }) => {
                set.insert_all(&bytes[start + from..start + row_count]);
            }
            _ => unreachable!("one field→column map drives states and columns"),
        }
    }
}

pub(super) fn uncounted_columns(columns: &[Column]) -> Box<[DistinctState]> {
    columns
        .iter()
        .map(|column| match column {
            Column::Words { .. } => DistinctState::Words(WordSet::default()),
            Column::Bytes { .. } => DistinctState::Bytes(ByteSet::default()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ByteSet, WordSet};

    #[test]
    fn distinct_counts_match_the_naive_set() {
        let mut rng = 0x2026_0723_u64;
        let mut next = move || {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            rng >> 33
        };
        for len in [0usize, 1, 2, 63, 64, 500] {
            let words: Vec<u64> = (0..len).map(|_| next() % 17).collect();
            let naive = words.iter().collect::<std::collections::HashSet<_>>().len() as u64;
            let mut set = WordSet::with_hint(len);
            set.insert_all(&words);
            assert_eq!(set.count(), naive, "len {len}");
        }

        let many: Vec<u64> = (0..10_000u64).map(|i| i * 2_654_435_761 + 1).collect();
        let naive = many.iter().collect::<std::collections::HashSet<_>>().len() as u64;
        let mut set = WordSet::with_hint(16);
        set.insert_all(&many);
        assert_eq!(set.count(), naive, "growth preserves the count");

        let mut zeros = WordSet::with_hint(3);
        zeros.insert_all(&[0, 0, 0]);
        assert_eq!(zeros.count(), 1, "the zero word counts once");
        let mut mixed = WordSet::with_hint(5);
        mixed.insert_all(&[0, 1, 0, 1, u64::MAX]);
        assert_eq!(mixed.count(), 3);

        let mut bytes = ByteSet::default();
        bytes.insert_all(&[7, 7, 0, 255, 0]);
        assert_eq!(bytes.count(), 3);
    }

    #[test]
    fn incremental_extension_matches_the_one_shot_count() {
        let words: Vec<u64> = (0..300u64).map(|i| i % 37).collect();
        let mut one_shot = WordSet::with_hint(words.len());
        one_shot.insert_all(&words);
        let mut incremental = WordSet::with_hint(100);
        incremental.insert_all(&words[..100]);
        let cloned = incremental.clone();
        let mut extended = cloned;
        extended.insert_all(&words[100..]);
        assert_eq!(extended.count(), one_shot.count());
    }
}
