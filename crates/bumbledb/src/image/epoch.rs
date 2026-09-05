//! The one view-validity epoch: closed theory, per-execution heap tick, or
//! one store relation's committed change version. Not a dummy generation and
//! not a second process clock.
//!
//! [`CacheGeneration`] and [`TextGeneration`] scope the database-owned
//! resident cache: text tokens are valid only within their generation and
//! are never persisted.
use crate::storage::store::RelationVersion;

/// Identity is checked before a memo uses this value. `Store(version)` is
/// PER RELATION: the store advances a relation's change version exactly when
/// a committed transaction changed that relation's rows, so an unrelated
/// write — another relation, or a host-record/attachment-only seal — leaves
/// the epoch equal and every memo valid (PERF-001). Equal versions within
/// one environment prove equal rows; a mismatch rebuilds. `Heap(tick)` is a
/// prepared-query-local execution counter: heap instances carry no durable
/// identity, so their images are rebuilt per execution and can never alias
/// another instance's rows — a fresh tick misses every memo by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewEpoch {
    Closed,
    Heap(u64),
    Store(RelationVersion),
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

/// The database-owned resident cache generation. Whole-generation eviction
/// invalidates dependent text tokens and idle prepared memos together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheGeneration(u64);

impl CacheGeneration {
    #[must_use]
    pub const fn initial() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

/// Text token identity scoped to one [`CacheGeneration`]. Cross-source word
/// comparison requires the same generation or an explicit exact remap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextGeneration(CacheGeneration);

impl TextGeneration {
    #[must_use]
    pub const fn initial() -> Self {
        Self(CacheGeneration::initial())
    }

    #[must_use]
    pub const fn of(generation: CacheGeneration) -> Self {
        Self(generation)
    }

    #[must_use]
    pub const fn cache(self) -> CacheGeneration {
        self.0
    }
}
