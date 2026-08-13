//! Process-lifetime escaped fresh high-water and the parked Q-burn retry.
//! Disk flush can still fail; this slot is what keeps `alloc` from going
//! backwards in this process (`never_reissue_observable`).

use std::collections::BTreeMap;
use std::sync::PoisonError;
#[cfg(test)]
use std::sync::atomic::Ordering;

use bumbledb_theory::schema::{FieldId, RelationId};

use super::Environment;

type FreshMarks = BTreeMap<(RelationId, FieldId), u64>;

fn lock_map(map: &std::sync::Mutex<FreshMarks>) -> std::sync::MutexGuard<'_, FreshMarks> {
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

fn merge_mark(map: &mut FreshMarks, rel: RelationId, field: FieldId, next: u64) {
    map.entry((rel, field))
        .and_modify(|held| *held = (*held).max(next))
        .or_insert(next);
}

impl Environment {
    /// Raises the in-process high-water for every dirty mark. Safe to call
    /// before a flush that may fail: the next `read_fresh_next` never
    /// returns below these values in this process.
    pub(crate) fn note_escaped_fresh(
        &self,
        marks: impl IntoIterator<Item = (RelationId, FieldId, u64)>,
    ) {
        let mut held = lock_map(&self.escaped_fresh);
        for (rel, field, next) in marks {
            merge_mark(&mut held, rel, field, next);
        }
    }

    /// The in-process floor for `(relation, field)`, or 0 if this process
    /// has not issued from that sequence.
    pub(crate) fn in_process_fresh_next(&self, rel: RelationId, field: FieldId) -> u64 {
        lock_map(&self.escaped_fresh)
            .get(&(rel, field))
            .copied()
            .unwrap_or(0)
    }

    /// Parks marks whose durable `Q` write has not succeeded.
    pub(crate) fn park_fresh_flush(&self, marks: FreshMarks) {
        let mut pending = lock_map(&self.pending_fresh_flush);
        for ((rel, field), next) in marks {
            merge_mark(&mut pending, rel, field, next);
        }
    }

    /// Takes the parked burn set (empty if every prior flush succeeded).
    pub(crate) fn take_pending_fresh_flush(&self) -> FreshMarks {
        std::mem::take(&mut *lock_map(&self.pending_fresh_flush))
    }

    /// A clone of the parked burn set — the main commit merges these into
    /// phase-4 `Q` puts without taking them until that commit is durable.
    pub(crate) fn peek_pending_fresh_flush(&self) -> FreshMarks {
        lock_map(&self.pending_fresh_flush).clone()
    }

    /// The main commit persisted every parked mark; drop the retry set.
    pub(crate) fn clear_pending_fresh_flush(&self) {
        lock_map(&self.pending_fresh_flush).clear();
    }

    /// Test-only: the next `n` escaped-id flushes fail without touching disk.
    #[cfg(test)]
    pub(crate) fn fail_next_fresh_flushes(&self, n: u32) {
        self.fail_fresh_flush.store(n, Ordering::SeqCst);
    }

    /// Test-only: consume one injected flush failure, if any remain.
    #[cfg(test)]
    pub(crate) fn consume_fresh_flush_failure(&self) -> bool {
        loop {
            let current = self.fail_fresh_flush.load(Ordering::SeqCst);
            if current == 0 {
                return false;
            }
            if self
                .fail_fresh_flush
                .compare_exchange_weak(current, current - 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }
}
