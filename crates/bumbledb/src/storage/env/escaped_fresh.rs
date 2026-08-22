//! Process-lifetime escaped fresh high-water and the parked Q-burn retry.
//! Disk flush can still fail; this slot is what keeps `alloc` from going
//! backwards in this process (`never_reissue_observable`).

use std::collections::BTreeMap;
use std::sync::PoisonError;
#[cfg(test)]
use std::sync::atomic::Ordering;

use bumbledb_theory::schema::{FieldId, RelationId};

use super::Environment;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FreshMarks {
    inner: BTreeMap<(RelationId, FieldId), u64>,
}

impl FreshMarks {
    pub(crate) fn join(&mut self, rel: RelationId, field: FieldId, next: u64) {
        self.inner
            .entry((rel, field))
            .and_modify(|held| *held = (*held).max(next))
            .or_insert(next);
    }

    pub(crate) fn join_all(
        &mut self,
        other: impl IntoIterator<Item = ((RelationId, FieldId), u64)>,
    ) {
        for ((rel, field), next) in other {
            self.join(rel, field, next);
        }
    }

    pub(crate) fn get(&self, rel: RelationId, field: FieldId) -> Option<u64> {
        self.inner.get(&(rel, field)).copied()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = ((RelationId, FieldId), u64)> + '_ {
        self.inner.iter().map(|(k, v)| (*k, *v))
    }
}

impl IntoIterator for FreshMarks {
    type Item = ((RelationId, FieldId), u64);
    type IntoIter = std::collections::btree_map::IntoIter<(RelationId, FieldId), u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a FreshMarks {
    type Item = (&'a (RelationId, FieldId), &'a u64);
    type IntoIter = std::collections::btree_map::Iter<'a, (RelationId, FieldId), u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) enum FlushState {
    #[default]
    Clean,
    Parked(FreshMarks),
}

impl FlushState {
    fn take(&mut self) -> FreshMarks {
        match std::mem::replace(self, Self::Clean) {
            Self::Clean => FreshMarks::default(),
            Self::Parked(marks) => marks,
        }
    }

    fn park(&mut self, marks: FreshMarks) {
        if marks.is_empty() {
            return;
        }
        match self {
            Self::Clean => *self = Self::Parked(marks),
            Self::Parked(held) => held.join_all(marks),
        }
    }

    fn peek(&self) -> FreshMarks {
        match self {
            Self::Clean => FreshMarks::default(),
            Self::Parked(marks) => marks.clone(),
        }
    }

    fn clear(&mut self) {
        *self = Self::Clean;
    }
}

fn lock_marks(map: &std::sync::Mutex<FreshMarks>) -> std::sync::MutexGuard<'_, FreshMarks> {
    map.lock().unwrap_or_else(PoisonError::into_inner)
}

fn lock_flush(state: &std::sync::Mutex<FlushState>) -> std::sync::MutexGuard<'_, FlushState> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Environment {
    /// before a flush that may fail: the next `read_fresh_next` never

    pub(crate) fn note_escaped_fresh(
        &self,
        marks: impl IntoIterator<Item = (RelationId, FieldId, u64)>,
    ) {
        let mut held = lock_marks(&self.escaped_fresh);
        for (rel, field, next) in marks {
            held.join(rel, field, next);
        }
    }

    pub(crate) fn in_process_fresh_next(&self, rel: RelationId, field: FieldId) -> u64 {
        lock_marks(&self.escaped_fresh).get(rel, field).unwrap_or(0)
    }

    pub(crate) fn park_fresh_flush(&self, marks: FreshMarks) {
        lock_flush(&self.pending_fresh_flush).park(marks);
    }

    pub(crate) fn take_pending_fresh_flush(&self) -> FreshMarks {
        lock_flush(&self.pending_fresh_flush).take()
    }

    pub(crate) fn peek_pending_fresh_flush(&self) -> FreshMarks {
        lock_flush(&self.pending_fresh_flush).peek()
    }

    /// The main commit persisted every parked mark; drop the retry set.
    pub(crate) fn clear_pending_fresh_flush(&self) {
        lock_flush(&self.pending_fresh_flush).clear();
    }

    #[cfg(test)]
    pub(crate) fn fail_next_fresh_flushes(&self, n: u32) {
        self.fail_fresh_flush.store(n, Ordering::SeqCst);
    }

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
