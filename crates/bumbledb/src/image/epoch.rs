//! The one view-validity epoch: closed theory, frozen heap, or store
//! generation. Not a dummy generation and not a second process clock.

use crate::storage::env::GenerationId;

/// Identity of one executable view binding.
///
/// Closed images are theory-constant. Frozen ordinary images belong to
/// one admitted heap instance. Store images belong to one persisted
/// generation. Identity is checked before a memo uses this value, so
/// [`Self::Frozen`] cannot alias another owned instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewEpoch {
    Closed,
    Frozen,
    Store(GenerationId),
}

impl ViewEpoch {
    /// A parked store binding is unhittable once the store has moved
    /// past it. Closed and frozen epochs never advance.
    pub(crate) fn superseded_by(self, current: Self) -> bool {
        match (self, current) {
            (Self::Store(old), Self::Store(new)) => old < new,
            _ => false,
        }
    }
}
