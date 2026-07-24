//! Lazy per-column exact distinct-value counts: computed
//! on first planner demand and memoized on the image's per-column
//! `OnceLock`.

use super::{ColumnView, RelationImage};

impl RelationImage {
    /// The exact distinct-value count of one column (docs/architecture/40-execution.md):
    /// word columns counted through a scratch hash set, byte columns
    /// through a 256-slot table. Intern ids are injective, so a
    /// String/Bytes column's word distincts are its value distincts.
    /// Column indices come from [`ColumnSpan`](crate::image::ColumnSpan)s —
    /// an interval field has two counts, one per word column.
    /// Computed on first demand and memoized on the image; a plan that
    /// never asks — every key probe — never pays the walk.
    #[must_use]
    pub fn cardinality(&self, column: usize) -> u64 {
        *self.distincts[column].get_or_init(|| match self.column(column) {
            ColumnView::Words(words) => count_words(words),
            ColumnView::Bytes(bytes) => count_bytes(bytes),
        })
    }
}

/// The distinct counter behind a word column's first demand — built
/// fresh per column, dropped after the memoized count lands (never
/// shared, never cleared): a power-of-two open-addressed word set in
/// one array. Zero is the in-band empty sentinel; the zero word (a
/// legal value) counts through its own flag instead of a slot, so no
/// second occupancy array exists to allocate or clear.
fn count_words(column: &[u64]) -> u64 {
    let capacity = (column.len().max(1) * 2).next_power_of_two();
    let mask = capacity - 1;
    let mut slots = vec![0u64; capacity];
    let mut distinct = 0u64;
    let mut zero_seen = false;
    for &word in column {
        if word == 0 {
            distinct += u64::from(!zero_seen);
            zero_seen = true;
            continue;
        }
        // The shared probe hash (`exec::swar`) — one avalanche,
        // linear probe; this counter's former byte-identical private
        // copy was the drift that module exists to prevent.
        let h = crate::exec::swar::hash_words(std::slice::from_ref(&word));
        let mut idx = usize::try_from(h).expect("64-bit usize") & mask;
        loop {
            if slots[idx] == 0 {
                slots[idx] = word;
                distinct += 1;
                break;
            }
            if slots[idx] == word {
                break;
            }
            idx = (idx + 1) & mask;
        }
    }
    distinct
}

/// The byte-column twin: 256 possible values, one flag table.
fn count_bytes(column: &[u8]) -> u64 {
    let mut seen = [false; 256];
    let mut distinct = 0u64;
    for &byte in column {
        if !seen[usize::from(byte)] {
            seen[usize::from(byte)] = true;
            distinct += 1;
        }
    }
    distinct
}

#[cfg(test)]
mod tests {
    use super::{count_bytes, count_words};

    /// The one-array set agrees with a naive distinct count — zero
    /// words (the in-band sentinel's legal twin) included.
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
            let naive = words
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len() as u64;
            assert_eq!(count_words(&words), naive, "len {len}");
        }
        assert_eq!(count_words(&[0, 0, 0]), 1, "the zero word counts once");
        assert_eq!(count_words(&[0, 1, 0, 1, u64::MAX]), 3);
        assert_eq!(count_bytes(&[7, 7, 0, 255, 0]), 3);
    }
}
