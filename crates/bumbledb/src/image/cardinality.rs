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
            ColumnView::Words(words) => CardinalityCounter::new(self.row_count).count_words(words),
            ColumnView::Bytes(bytes) => CardinalityCounter::count_bytes(bytes),
        })
    }
}

/// The build-time distinct counter: a power-of-two open-addressed word
/// set sized once for the row count and memset-cleared per column.
struct CardinalityCounter {
    slots: Vec<u64>,
    occupied: Vec<bool>,
}

impl CardinalityCounter {
    fn new(row_count: usize) -> Self {
        let capacity = (row_count.max(1) * 2).next_power_of_two();
        Self {
            slots: vec![0; capacity],
            occupied: vec![false; capacity],
        }
    }

    fn count_words(&mut self, column: &[u64]) -> u64 {
        self.occupied.fill(false);
        let mask = self.slots.len() - 1;
        let mut distinct = 0u64;
        for &word in column {
            // The shared probe hash (`exec::swar`) — one avalanche,
            // linear probe; this counter's former byte-identical private
            // copy was the drift that module exists to prevent.
            let h = crate::exec::swar::hash_words(std::slice::from_ref(&word));
            let mut idx = usize::try_from(h).expect("64-bit usize") & mask;
            loop {
                if !self.occupied[idx] {
                    self.occupied[idx] = true;
                    self.slots[idx] = word;
                    distinct += 1;
                    break;
                }
                if self.slots[idx] == word {
                    break;
                }
                idx = (idx + 1) & mask;
            }
        }
        distinct
    }

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
}
