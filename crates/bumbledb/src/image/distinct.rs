//! Exact per-column distinct-value counts, persisted WITH the image:
//! computed once while the build's rows are still warm and extended
//! incrementally on the append path — the planner reads a stored
//! number, never walks a column. The former shape — a lazy exact
//! O(rows) open-addressed walk plus a 2×rows scratch allocation per
//! column at first planner demand — made every cold prepare pay the
//! whole count pass per demanded column and every re-prepare after a
//! commit pay it again; persisting the counting state makes the append
//! path O(tail) exact (the image oracle's served-vs-rebuilt equality
//! holds by construction, not by re-walking).

use super::{Column, RelationImage};

impl RelationImage {
    /// The exact distinct-value count of one column
    /// (docs/architecture/40-execution.md): a stored read — the counting
    /// pass ran at build/append/synthesis. Intern ids are injective, so
    /// a String/Bytes column's word distincts are its value distincts.
    /// Column indices come from
    /// [`ColumnSpan`](crate::image::ColumnSpan)s — an interval field has
    /// two counts, one per word column.
    #[must_use]
    pub fn distinct_count(&self, column: usize) -> u64 {
        self.distincts[column].count()
    }
}

/// One column's persistent distinct-counting state — the image carries
/// it so the append path extends instead of re-walking. Word columns
/// hold a growable open-addressed word set; byte columns a 256-bit
/// mask.
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

/// A power-of-two open-addressed word set in one array. Zero is the
/// in-band empty sentinel; the zero word (a legal value) counts through
/// its own flag instead of a slot, so no second occupancy array exists.
/// Sized to the DISTINCTS (grown by doubling at 0.5 load), never to the
/// rows — a low-cardinality column's set stays tiny for the image's
/// whole lifetime.
#[derive(Debug, Clone, Default)]
pub(super) struct WordSet {
    slots: Vec<u64>,
    /// Distinct nonzero words inserted.
    len: u64,
    zero_seen: bool,
}

impl WordSet {
    fn with_hint(rows: usize) -> Self {
        // The colt force's deterministic guess: distincts are unknown
        // before the pass, so start at rows/8 (min 16) and double when
        // short — amortized O(rows) inserts either way.
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
        // stay short and the loop below always finds an empty slot.
        if (usize::try_from(self.len).expect("64-bit usize") + 1) * 2 > self.slots.len() {
            self.grow();
        }
        let mask = self.slots.len() - 1;
        // The shared probe hash (`exec::swar`) — one avalanche, linear
        // probe; a byte-identical private copy is the drift that module
        // exists to prevent.
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

    /// Doubles the slot array and reinserts (zero rides its flag, so
    /// every occupied slot is a real word).
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

/// The byte-column twin: 256 possible values, one 256-bit mask.
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

/// The build-path counting pass: every column of a freshly filled frame,
/// while its slabs are still warm — one state per column, sized to the
/// distincts, persisted by `seal`.
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

/// The append-path extension: rows `[from, row_count)` insert into the
/// (cloned) base states — O(tail) exact, the whole reason the state
/// persists with the image.
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

/// Uncounted states for images the planner never costs — the fixpoint
/// driver's transient images (`Interior` occurrences pin no row counts: the
/// selectivity guard costs recursion on the ladder's floors, so
/// `distinct_count` is unreachable there).
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

    /// The growable set agrees with a naive distinct count — zero words
    /// (the in-band sentinel's legal twin) included, growth crossed.
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
        // High cardinality from a tiny hint: the doubling growth path.
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

    /// Incremental extension equals the one-shot count — the append
    /// path's exactness, at the unit level.
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
