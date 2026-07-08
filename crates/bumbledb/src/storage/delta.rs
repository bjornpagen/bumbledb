//! The write transaction delta core (docs/architecture/40-storage.md): a write transaction is an
//! in-memory net insert-set and delete-set of canonical fact bytes — last
//! disposition per fact wins — plus in-memory counters
//! (`docs/architecture/40-storage.md`).
//!
//! During accumulation, `insert`/`delete` are pure set arithmetic: encode is
//! the caller's job; membership is the delta's own disposition if present,
//! else an `M` probe against the borrowed read view. **Nothing touches an
//! LMDB data page until commit** (docs/architecture) — the LMDB write transaction
//! opens at commit, keeping the write-lock window to the commit step; an
//! abort (error or panic) just drops this struct and LMDB was never written.

use std::collections::BTreeMap;

use crate::arena::{Arena, ArenaSlice};
use crate::schema::{FieldId, RelationId, Schema};

mod accessors;
mod alloc;
mod delete;
mod insert;
mod intern;
mod new;

#[cfg(test)]
mod tests;

/// The net effect recorded for one fact. Last disposition wins; whether it
/// actually applies is decided against base state at commit (docs/architecture/40-storage.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Insert,
    Delete,
}

/// The accumulated write transaction.
pub struct WriteDelta<'s> {
    schema: &'s Schema,
    arena: Arena,
    /// `(relation, fact_hash) → (fact bytes, last disposition)`. Keyed by the
    /// full 32-byte blake3 of `fact_bytes` — hash equality *is* fact equality
    /// (collision axiom, `10-data-model.md`), and the `BTreeMap` gives the
    /// deterministic commit order the 40-storage doc requires.
    facts: BTreeMap<(RelationId, [u8; 32]), (ArenaSlice, Disposition)>,
    /// Serial next-values, lazily initialized from `Q` once per
    /// `(relation, field)` per transaction; a transaction sees its own
    /// allocations. The stored value is the *next* value to issue.
    serial_next: BTreeMap<(RelationId, FieldId), u64>,
    /// The committed `Q` value each sequence started from this
    /// transaction (populated by the same lazy read): a mark is *dirty* —
    /// it escaped as an allocation the closure may have returned — iff it
    /// advanced past this base. Dirty marks persist even on a no-op
    /// commit (`40-storage.md`).
    serial_base: BTreeMap<(RelationId, FieldId), u64>,
    /// Net row-count change per relation, maintained alongside the
    /// changed-state reports (flushed to `S` by the 40-storage doc).
    row_count_delta: BTreeMap<RelationId, i64>,
    /// Novel strings/bytes interned by this transaction: provisional ids
    /// assigned from the committed dictionary counter (the counter is
    /// in-memory-then-flush like every other counter; single-writer
    /// discipline makes provisional = final). One map per tag so probes
    /// borrow the raw bytes (`BTreeMap<Box<[u8]>, _>` looks up by
    /// `&[u8]`); the old `(u8, Box<[u8]>)` key boxed a copy per probe.
    pending_interns: [BTreeMap<Box<[u8]>, u64>; 2],
    /// The next dictionary id, lazily read once per transaction.
    dict_next: Option<u64>,
}

impl<'s> WriteDelta<'s> {
    /// The schema this delta was accumulated against (reader: commit).
    pub(crate) fn schema(&self) -> &'s Schema {
        self.schema
    }

    /// Whether the delta records no dispositions at all (reader: the 40-storage doc's
    /// skip-empty-commit rule). A successful commit of an empty delta
    /// still persists any *dirty* serial marks — the closure may have
    /// returned those ids to the host, and a successful commit persists
    /// every serial value it issued (`10-data-model.md`). Pending interns
    /// of an empty delta are deliberately dropped: intern ids never
    /// escape (hosts see values, not words).
    pub(crate) fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// The dictionary next-id to flush, if this transaction minted any
    /// provisional ids (reader: the 40-storage doc phase 4).
    pub(crate) fn dict_next(&self) -> Option<u64> {
        self.dict_next
    }
}
