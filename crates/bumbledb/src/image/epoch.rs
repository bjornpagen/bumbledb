//! The one view-validity epoch: closed theory, frozen heap, or store
//! generation. Not a dummy generation and not a second process clock.
use crate::storage::env::GenerationId;

/// Identity is checked before a memo uses this value, so [`Self::Frozen`]
/// cannot alias another owned instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewEpoch {
    Closed,
    Frozen,
    Store(GenerationId),
}

impl ViewEpoch {
    pub(crate) fn superseded_by(self, current: Self) -> bool {
        match (self, current) {
            (Self::Store(old), Self::Store(new)) => old < new,
            _ => false,
        }
    }
}
