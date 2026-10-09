//! Physical storage: one LMDB environment ([`store`]). Rows are canonical
//! rows keyed by home, and every commit is one durable LMDB transaction
//! holding the rows, the generation and the host records together.

pub(crate) mod store;

/// The persisted storage transaction id: the generation a snapshot
/// witnessed and a state-changing commit advances. This is not a
/// process-local reader-cache sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GenerationId(u64);

impl GenerationId {
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_storage(word: u64) -> Self {
        Self(word)
    }

    pub(crate) const fn storage_word(self) -> u64 {
        self.0
    }

    pub(crate) const fn initial() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for GenerationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
