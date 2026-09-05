//! The one view-validity epoch: closed theory, per-execution heap tick, or
//! store generation. Not a dummy generation and not a second process clock.
use crate::storage::GenerationId;

/// Identity is checked before a memo uses this value. `Heap(tick)` is a
/// prepared-query-local execution counter: heap instances carry no durable
/// identity, so their images are rebuilt per execution and can never alias
/// another instance's rows — a fresh tick misses every memo by
/// construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewEpoch {
    Closed,
    Heap(u64),
    Store(GenerationId),
}

impl ViewEpoch {
    pub(crate) fn superseded_by(self, current: Self) -> bool {
        match (self, current) {
            (Self::Store(old), Self::Store(new)) => old < new,
            (Self::Heap(old), Self::Heap(new)) => old < new,
            _ => false,
        }
    }
}
