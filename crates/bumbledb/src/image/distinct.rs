//! Exact planner statistics, requested per column and retained as scalars.
//! Image construction does no counting. Temporary tables belong to the
//! requesting operation, not the cache; a failed computation publishes nothing.
use super::{ColumnView, RelationImage};
use crate::api::prepared::source::work_error;
use crate::error::{Error, Result};
use crate::exec::sink::STEP_QUANTUM;
use crate::work::{ByteKind, ByteReservation, WorkContext};

impl RelationImage {
    /// Compute once on demand. Concurrent callers may compute independently:
    /// none waits behind another operation's allocation or cancellation.
    pub(crate) fn distinct_count(&self, column: usize, work: &WorkContext) -> Result<u64> {
        work.checkpoint().map_err(work_error)?;
        let cached = &self.distincts[column];
        if let Some(&count) = cached.get() {
            return Ok(count);
        }
        let mut budget = CountWork { work, pending: 0 };
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
                    work.step(chunk.len() as u64).map_err(work_error)?;
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
}

impl CountWork<'_> {
    // Input words, rehash slots and collision probes all count, so skewed
    // values cannot hide an unbounded probe chain between checkpoints.
    #[inline]
    fn note(&mut self) -> Result<()> {
        self.pending += 1;
        if self.pending == STEP_QUANTUM {
            self.finish()?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.work
            .step(u64::from(std::mem::take(&mut self.pending)))
            .map_err(work_error)
    }
}

struct WordSet {
    slots: Vec<u64>,
    len: usize,
    zero_seen: bool,
    _charge: ByteReservation,
}

fn allocation_error() -> Error {
    Error::from_store(crate::storage::store::StoreError::Allocation)
}

impl WordSet {
    fn allocate(capacity: usize, work: &WorkContext) -> Result<Self> {
        let bytes = capacity.checked_mul(8).ok_or_else(allocation_error)?;
        let charge = work
            .reserve(ByteKind::Working, bytes as u64)
            .map_err(work_error)?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(capacity)
            .map_err(|_| allocation_error())?;
        slots.resize(capacity, 0);
        Ok(Self {
            slots,
            len: 0,
            zero_seen: false,
            _charge: charge,
        })
    }

    fn insert(&mut self, word: u64, budget: &mut CountWork<'_>) -> Result<()> {
        if word == 0 {
            self.zero_seen = true;
            return Ok(());
        }
        // The measured table stays half empty. Growing keeps both allocations
        // charged until all old slots have been scanned and the old Vec drops.
        if self.len + 1 > self.slots.len() / 2 {
            self.grow(budget)?;
        }
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
        let old = std::mem::replace(self, Self::allocate(doubled, budget.work)?);
        self.zero_seen = old.zero_seen;
        for &word in &old.slots {
            budget.note()?;
            if word != 0 {
                self.insert_word(word, budget)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::prepared::source::{UNBOUNDED_POLICY, unbounded_work};
    use crate::image::{RelationImage, TransientImage, test_generation};
    use crate::work::{Resource, WorkError};
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
            let work = unbounded_work().unwrap();
            let expected = words.iter().collect::<std::collections::HashSet<_>>().len() as u64;
            assert!(image.distincts[0].get().is_none());
            assert_eq!(image.distinct_count(0, &work).unwrap(), expected);
            assert_eq!(work.used(Resource::WorkingBytes), 0);
            let steps = work.used(Resource::WorkUnits);
            assert_eq!(image.distinct_count(0, &work).unwrap(), expected);
            assert_eq!(
                work.used(Resource::WorkUnits),
                steps,
                "cached scalar lookup"
            );
        }
    }

    #[test]
    fn failures_release_scratch_and_never_publish_counts() {
        let image = image(&(0..1000).collect::<Vec<_>>());
        for policy in [
            crate::work::ExecutionPolicy {
                working_bytes: 0,
                ..UNBOUNDED_POLICY
            },
            crate::work::ExecutionPolicy {
                work_units: 1,
                ..UNBOUNDED_POLICY
            },
            crate::work::ExecutionPolicy {
                timeout: std::time::Duration::ZERO,
                ..UNBOUNDED_POLICY
            },
        ] {
            let work = policy.start().unwrap();
            assert!(image.distinct_count(0, &work).is_err());
            assert_eq!(work.used(Resource::WorkingBytes), 0);
            assert!(image.distincts[0].get().is_none());
        }
        let work = unbounded_work().unwrap();
        work.cancel();
        assert!(matches!(
            image.distinct_count(0, &work),
            Err(crate::error::Error::Store(error))
                if matches!(*error, crate::storage::store::StoreError::Work(WorkError::Cancelled))
        ));
        assert!(image.distincts[0].get().is_none());
        assert_eq!(
            image.distinct_count(0, &unbounded_work().unwrap()).unwrap(),
            1000
        );
        assert!(
            image.distinct_count(0, &work).is_err(),
            "even a cache hit must stop"
        );
    }

    #[test]
    fn growth_admits_old_and_new_tables_together() {
        let image = image(&(1..=9).collect::<Vec<_>>());
        let work = crate::work::ExecutionPolicy {
            working_bytes: 16 * 8 + 32 * 8 - 1,
            ..UNBOUNDED_POLICY
        }
        .start()
        .unwrap();
        assert!(matches!(image.distinct_count(0, &work),
        Err(crate::error::Error::Store(error)) if matches!(*error,
            crate::storage::store::StoreError::Work(WorkError::Exhausted {
                resource: Resource::WorkingBytes, used: 128, requested: 256, ..
            }))));
        assert_eq!(work.used(Resource::WorkingBytes), 0);
        assert!(image.distincts[0].get().is_none());
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
                        let work = unbounded_work().unwrap();
                        barrier.wait();
                        assert_eq!(image.distinct_count(0, &work).unwrap(), 37);
                        assert_eq!(work.used(Resource::WorkingBytes), 0);
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
    fn collision_probes_are_bounded_work_too() {
        let words: Vec<_> = (1..)
            .filter(|word| {
                crate::exec::swar::hash_words(std::slice::from_ref(word)).trailing_zeros() >= 10
            })
            .take(30)
            .collect();
        let image = image(&words);
        let work = crate::work::ExecutionPolicy {
            work_units: 256,
            ..UNBOUNDED_POLICY
        }
        .start()
        .unwrap();
        assert!(matches!(image.distinct_count(0, &work),
        Err(crate::error::Error::Store(error)) if matches!(*error,
            crate::storage::store::StoreError::Work(WorkError::Exhausted {
                resource: Resource::WorkUnits, ..
            }))));
        assert!(image.distincts[0].get().is_none());
        assert_eq!(work.used(Resource::WorkingBytes), 0);
        assert_eq!(
            image.distinct_count(0, &unbounded_work().unwrap()).unwrap(),
            30
        );
    }

    #[test]
    fn multiword_and_byte_counts_reset_on_reuse_growth_and_failed_drain() {
        let fields = [ValueType::Uuid, ValueType::Bool];
        let generation = test_generation();
        let work = unbounded_work().unwrap();
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
