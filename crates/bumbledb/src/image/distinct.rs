//! Exact planner statistics, requested per column and retained as scalars.
//! Image construction does no counting. Temporary tables belong to the
//! requesting operation, not the cache; a failed computation publishes nothing.
use super::{ColumnView, RelationImage};
use crate::api::prepared::source::work_error;
use crate::error::{Error, Result};
use crate::exec::sink::STEP_QUANTUM;
use crate::work::WorkContext;

impl RelationImage {
    /// Compute once on demand. Concurrent callers may compute independently:
    /// none waits behind another operation's allocation or cancellation.
    pub(crate) fn distinct_count(&self, column: usize, work: &WorkContext) -> Result<u64> {
        work.checkpoint().map_err(work_error)?;
        let cached = &self.distincts[column];
        if let Some(&count) = cached.get() {
            return Ok(count);
        }
        let mut budget = CountWork::new(work);
        let count = match self.column(column) {
            ColumnView::Words([]) => 0,
            ColumnView::Words(words) => {
                let capacity = (words.len() / 8).max(16).next_power_of_two();
                let mut set = WordSet::allocate(capacity, work)?;
                for &word in words {
                    budget.note()?;
                    set.insert(word, &mut budget)?;
                }
                set.len as u64 + u64::from(set.zero_seen)
            }
            ColumnView::Bytes(bytes) => {
                let mut mask = [0u64; 4];
                for chunk in bytes.chunks(STEP_QUANTUM as usize) {
                    work.checkpoint().map_err(work_error)?;
                    for &byte in chunk {
                        mask[usize::from(byte >> 6)] |= 1 << (byte & 63);
                    }
                }
                mask.iter().map(|word| u64::from(word.count_ones())).sum()
            }
        };
        // Flush sub-quantum work and check cancellation before publishing.
        // All temporary allocations have already dropped at this boundary.
        budget.finish()?;
        let _ = cached.set(count);
        Ok(count)
    }
}

struct CountWork<'a> {
    work: &'a WorkContext,
    pending: u32,
    #[cfg(test)]
    cancel_after: Option<u32>,
}

impl<'a> CountWork<'a> {
    fn new(work: &'a WorkContext) -> Self {
        Self {
            work,
            pending: 0,
            #[cfg(test)]
            cancel_after: None,
        }
    }

    // Input words, rehash slots and collision probes all count, so skewed
    // values cannot hide an unbounded probe chain between checkpoints.
    #[inline]
    fn note(&mut self) -> Result<()> {
        #[cfg(test)]
        if let Some(remaining) = &mut self.cancel_after {
            *remaining = remaining.saturating_sub(1);
            if *remaining == 0 {
                self.work.cancel();
            }
        }
        self.pending += 1;
        if self.pending == STEP_QUANTUM {
            self.finish()?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.pending = 0;
        self.work.checkpoint().map_err(work_error)
    }
}

struct WordSet {
    slots: Vec<u64>,
    len: usize,
    zero_seen: bool,
}

fn allocation_error() -> Error {
    Error::from_store(crate::storage::store::StoreError::Allocation)
}

impl WordSet {
    fn allocate(capacity: usize, work: &WorkContext) -> Result<Self> {
        work.checkpoint().map_err(work_error)?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(capacity)
            .map_err(|_| allocation_error())?;
        slots.resize(capacity, 0);
        Ok(Self {
            slots,
            len: 0,
            zero_seen: false,
        })
    }

    fn insert(&mut self, word: u64, budget: &mut CountWork<'_>) -> Result<()> {
        if word == 0 {
            self.zero_seen = true;
            return Ok(());
        }
        // Probe before growing: a duplicate at the load boundary needs no
        // new slot or allocation.
        self.insert_word(word, budget)
    }

    fn insert_word(&mut self, word: u64, budget: &mut CountWork<'_>) -> Result<()> {
        let mask = self.slots.len() - 1;
        let hash = crate::exec::swar::hash_words(std::slice::from_ref(&word));
        let mut index = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {
            let slot = self.slots[index];
            if slot == word {
                return Ok(());
            }
            if slot == 0 {
                if self.len >= self.slots.len() / 2 {
                    self.grow(budget)?;
                    return self.insert_word(word, budget);
                }
                self.slots[index] = word;
                self.len += 1;
                return Ok(());
            }
            budget.note()?;
            index = (index + 1) & mask;
        }
    }

    fn grow(&mut self, budget: &mut CountWork<'_>) -> Result<()> {
        let doubled = self
            .slots
            .len()
            .checked_mul(2)
            .ok_or_else(allocation_error)?;
        let mut replacement = Self::allocate(doubled, budget.work)?;
        replacement.zero_seen = self.zero_seen;
        for &word in &self.slots {
            budget.note()?;
            if word != 0 {
                replacement.insert_word(word, budget)?;
            }
        }
        budget.finish()?;
        *self = replacement;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::prepared::source::unbounded_work;
    use crate::image::{RelationImage, TransientImage, test_generation};
    use crate::work::WorkError;
    use bumbledb_theory::schema::ValueType;
    use std::sync::Arc;

    fn image(words: &[u64]) -> Arc<RelationImage> {
        TransientImage::default().refill(
            &[ValueType::U64],
            words.len(),
            &test_generation(),
            words.iter().map(std::slice::from_ref),
        )
    }

    #[test]
    fn exact_counts_are_lazy_and_leave_no_live_tables() {
        for words in [
            vec![],
            vec![0, 0, 0],
            vec![0, 1, 0, 1, u64::MAX],
            (0..10_000).map(|i| i * 2_654_435_761 + 1).collect(),
            (0..1000).map(|i| i % 17).collect(),
        ] {
            let image = image(&words);
            let work = unbounded_work();
            let expected = words.iter().collect::<std::collections::HashSet<_>>().len() as u64;
            assert!(image.distincts[0].get().is_none());
            #[cfg(feature = "alloc-counter")]
            let before = crate::alloc_counter::snapshot();
            assert_eq!(image.distinct_count(0, &work).unwrap(), expected);
            #[cfg(feature = "alloc-counter")]
            {
                let after = crate::alloc_counter::snapshot();
                assert_eq!(
                    after.absolute.live_bytes, before.absolute.live_bytes,
                    "statistics retain only the scalar, not their working table"
                );
            }
            #[cfg(feature = "alloc-counter")]
            let warm = crate::alloc_counter::snapshot().window;
            for _ in 0..32 {
                assert_eq!(image.distinct_count(0, &work).unwrap(), expected);
            }
            #[cfg(feature = "alloc-counter")]
            assert_eq!(
                crate::alloc_counter::snapshot().window,
                warm,
                "cached count allocates nothing"
            );
        }
    }

    #[test]
    fn cancellation_never_publishes_counts_and_also_stops_cached_reads() {
        let image = image(&(0..1000).collect::<Vec<_>>());
        let work = unbounded_work();
        work.cancel();
        assert!(matches!(
            image.distinct_count(0, &work),
            Err(crate::error::Error::Store(error))
                if matches!(*error, crate::storage::store::StoreError::Work(WorkError::Cancelled))
        ));
        assert!(image.distincts[0].get().is_none());
        assert_eq!(image.distinct_count(0, &unbounded_work()).unwrap(), 1000);
        assert!(
            image.distinct_count(0, &work).is_err(),
            "even a cache hit must stop"
        );
    }

    #[test]
    fn growing_statistics_release_the_old_table_and_failed_growth_leaves_it_whole() {
        use super::{CountWork, WordSet};
        let work = unbounded_work();
        let mut table = WordSet::allocate(16, &work).unwrap();
        let mut count = CountWork::new(&work);
        for word in 1..=8 {
            table.insert(word, &mut count).unwrap();
        }
        #[cfg(feature = "alloc-counter")]
        let before = crate::alloc_counter::snapshot().window;
        table.insert(9, &mut count).unwrap();
        #[cfg(feature = "alloc-counter")]
        {
            let after = crate::alloc_counter::snapshot().window;
            assert_eq!(after.alloc_bytes - before.alloc_bytes, 32 * 8);
            assert_eq!(after.dealloc_bytes - before.dealloc_bytes, 16 * 8);
        }
        assert_eq!(table.len, 9);
        let pointer = table.slots.as_ptr();
        work.cancel();
        assert!(table.grow(&mut count).is_err());
        assert_eq!(table.slots.as_ptr(), pointer);
        let mut words: Vec<_> = table
            .slots
            .iter()
            .copied()
            .filter(|word| *word != 0)
            .collect();
        words.sort_unstable();
        assert_eq!(words, (1..=9).collect::<Vec<_>>());
        assert!(WordSet::allocate(usize::MAX, &unbounded_work()).is_err());
    }

    #[test]
    fn concurrent_statistics_publish_the_same_exact_scalar() {
        let image = image(&(0..4096).map(|i| i % 37).collect::<Vec<_>>());
        let barrier = std::sync::Barrier::new(4);
        std::thread::scope(|scope| {
            let barrier = &barrier;
            let handles: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        let work = unbounded_work();
                        barrier.wait();
                        assert_eq!(image.distinct_count(0, &work).unwrap(), 37);
                    })
                })
                .collect();
            for handle in handles {
                handle.join().unwrap();
            }
        });
        assert_eq!(image.distincts[0].get(), Some(&37));
    }

    #[test]
    fn duplicate_statistics_at_half_capacity_do_not_grow_or_allocate() {
        use super::{CountWork, WordSet};
        let work = unbounded_work();
        let mut table = WordSet::allocate(16, &work).unwrap();
        let mut count = CountWork::new(&work);
        for word in 0..=8 {
            table.insert(word, &mut count).unwrap();
        }
        let pointer = table.slots.as_ptr();
        #[cfg(feature = "alloc-counter")]
        let before = crate::alloc_counter::snapshot().window;
        for word in (0..=8).cycle().take(4096) {
            table.insert(word, &mut count).unwrap();
        }
        #[cfg(feature = "alloc-counter")]
        assert_eq!(crate::alloc_counter::snapshot().window, before);
        assert_eq!(table.slots.len(), 16);
        assert_eq!(table.slots.as_ptr(), pointer);
        assert_eq!(table.len, 8);
        assert!(table.zero_seen);
    }

    #[test]
    fn cancellation_during_statistics_rehash_preserves_the_original_set() {
        use super::{CountWork, WordSet};
        let work = unbounded_work();
        let mut table = WordSet::allocate(64, &work).unwrap();
        let mut count = CountWork::new(&work);
        for word in 0..=32 {
            table.insert(word, &mut count).unwrap();
        }
        let expected = table.slots.clone();
        let pointer = table.slots.as_ptr();
        // Cancel after entering rehash, not at its allocation checkpoint.
        count.pending = super::STEP_QUANTUM - 8;
        count.cancel_after = Some(8);
        assert!(table.grow(&mut count).is_err());
        assert_eq!(count.cancel_after, Some(0));
        assert_eq!(table.slots.as_ptr(), pointer);
        assert_eq!(table.slots, expected);
        assert_eq!(table.len, 32);
        assert!(table.zero_seen);
        // A fresh operation can still grow the intact set.
        let retry = unbounded_work();
        let mut count = CountWork::new(&retry);
        table.insert(33, &mut count).unwrap();
        assert_eq!(table.len, 33);
        assert!(table.zero_seen);
        let mut actual: Vec<_> = table.slots.iter().copied().filter(|&v| v != 0).collect();
        actual.sort_unstable();
        assert!(actual.into_iter().eq(1..=33));
    }

    #[test]
    fn collision_probes_are_exact_and_observe_cancellation_within_a_quantum() {
        use super::{CountWork, WordSet};
        let words: Vec<_> = (1..)
            .filter(|word| {
                crate::exec::swar::hash_words(std::slice::from_ref(word)).trailing_zeros() >= 10
            })
            .take(30)
            .collect();
        let image = image(&words);
        assert_eq!(image.distinct_count(0, &unbounded_work()).unwrap(), 30);

        let work = unbounded_work();
        let mut table = WordSet::allocate(16, &work).unwrap();
        let mut count = CountWork::new(&work);
        table.insert(words[0], &mut count).unwrap();
        count.pending = super::STEP_QUANTUM - 1;
        work.cancel();
        let error = table.insert(words[1], &mut count).unwrap_err();
        assert!(matches!(error, crate::Error::Store(error) if matches!(
            error.as_ref(), crate::storage::store::StoreError::Work(WorkError::Cancelled)
        )));
        assert_eq!(
            table.len, 1,
            "a cancelled collision probe publishes no new slot"
        );
    }

    #[test]
    fn multiword_and_byte_counts_reset_on_reuse_growth_and_failed_drain() {
        let fields = [ValueType::Uuid, ValueType::Bool];
        let generation = test_generation();
        let work = unbounded_work();
        let mut buffer = TransientImage::default();
        let rows = [[0, 1, 0], [0, 2, 1], [7, 1, 1]];
        let first = buffer.refill(&fields, 3, &generation, rows.iter().map(<[_; 3]>::as_slice));
        for column in 0..3 {
            assert!(first.distincts[column].get().is_none());
            assert_eq!(first.distinct_count(column, &work).unwrap(), 2);
        }
        // A shared snapshot must retain both its columns and statistics.
        let rows = [[5, 3, 0], [5, 3, 0], [5, 3, 0]];
        let next = buffer.refill(&fields, 3, &generation, rows.iter().map(<[_; 3]>::as_slice));
        assert!(!Arc::ptr_eq(&first, &next));
        assert_eq!(next.distinct_count(0, &work).unwrap(), 1);
        assert_eq!(first.distinct_count(0, &work).unwrap(), 2);
        let address = Arc::as_ptr(&next);
        drop(next);

        let one = [[0, 0, 0]];
        let next = buffer.refill(&fields, 1, &generation, one.iter().map(<[_; 3]>::as_slice));
        assert_eq!(
            address,
            Arc::as_ptr(&next),
            "unique slab reuses its storage"
        );
        assert!(next.distincts.iter().all(|count| count.get().is_none()));
        assert_eq!(next.distinct_count(0, &work).unwrap(), 1);
        drop(next);
        let next = buffer
            .refill_drained(None, &fields, 3, &generation, |base, write| {
                assert_eq!(base, 0);
                write(&[0, 0, 0]);
                write(&[1, 2, 1]);
                write(&[2, 3, 0]);
                Ok(())
            })
            .unwrap();
        assert_eq!(next.distinct_count(0, &work).unwrap(), 3);
        assert_eq!(next.distinct_count(2, &work).unwrap(), 2);
        drop(next);
        assert!(
            buffer
                .refill_drained(None, &fields, 2, &generation, |_, write| {
                    write(&[9, 9, 1]);
                    Err(crate::api::prepared::source::work_error(
                        WorkError::Cancelled,
                    ))
                })
                .is_err()
        );
        let TransientImage::Occupied { image, .. } = &buffer else {
            panic!("occupied")
        };
        assert!(image.distincts.iter().all(|count| count.get().is_none()));
        let empty = buffer.refill(&fields, 0, &generation, std::iter::empty());
        for column in 0..3 {
            assert_eq!(empty.distinct_count(column, &work).unwrap(), 0);
        }
    }
}
