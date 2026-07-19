//! The write transaction delta core (docs/architecture/50-storage.md): a write transaction is an
//! in-memory net insert-set and delete-set of canonical fact bytes — **net
//! dispositions against committed state** — plus in-memory counters
//! (`docs/architecture/50-storage.md`).
//!
//! During accumulation, `insert`/`delete` are pure set arithmetic: encode is
//! the caller's job; membership is the delta's own disposition if present,
//! else an `M` probe against the borrowed read view. That op-time probe is
//! **authoritative**: the single-writer mutex holds committed state stable
//! for the delta's whole lifetime, so a disposition proved against it at op
//! time is still true at commit. The recording rules keep every entry a
//! genuine state change — a redundant op records nothing, and an op whose
//! net effect is nothing *cancels* the pending opposite (`insert`/`delete`
//! doc comments carry the four cases). **Nothing touches an LMDB data page
//! until commit** (docs/architecture) — the LMDB write transaction opens at
//! commit, keeping the write-lock window to the commit step; an abort
//! (error or panic) just drops this struct and LMDB was never written.

use std::collections::BTreeMap;

use crate::arena::{Arena, ArenaSlice};
use crate::schema::{KeyId, Schema};
use crate::storage::keys::DeterminantImage;
use bumbledb_theory::schema::{FieldId, RelationId};

mod accessors;
mod alloc;
mod delete;
mod determinants;
mod insert;
mod intern;
mod new;

#[cfg(test)]
mod tests;

/// The net effect recorded for one fact, proved against committed state at
/// op time (docs/architecture/50-storage.md): an `Insert` entry's fact is
/// committed-absent, a `Delete` entry's fact committed-present — so every
/// entry applies at commit, by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Insert,
    Delete,
}

/// The net effect recorded for one key statement's determinant tuple — the
/// point-read index (`docs/architecture/50-storage.md` § `WriteTx`
/// point reads): inserts record the establishing fact, deletes record absence;
/// last disposition wins, mirroring the fact map — except that a delete
/// never erases a record established by a *different* pending fact under
/// the same key bytes (the `delete(old); insert(new)`-in-either-order
/// idiom), and a delete that *cancels* a pending insert restores the
/// tuple's pre-insert overlay instead of recording absence
/// (`restore_determinants` — the net effect of a cancelled pair is
/// nothing, so the committed owner must keep answering).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeterminantDisposition {
    Present(ArenaSlice),
    Absent,
}

/// A determinant-map hit, resolved for point readers: the pending fact that
/// establishes the key tuple in the final state, or its recorded absence.
/// A map miss (no overlay at all) means the committed state answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterminantOverlay<'a> {
    Present(&'a [u8]),
    Absent,
}

/// One fresh sequence's transaction-local state
/// ([`WriteDelta::marks`]): initialized in one piece from the lazy `Q`
/// read, so an entry without its base is unrepresentable.
#[derive(Debug, Clone, Copy)]
struct FreshMark {
    /// The committed `Q` value the sequence started from — the
    /// dirtiness baseline.
    base: u64,
    /// The next value to issue; a transaction sees its own allocations.
    next: u64,
}

/// The accumulated write transaction.
pub struct WriteDelta<'s> {
    schema: &'s Schema,
    arena: Arena,
    /// `(relation, fact_hash) → (fact bytes, net disposition)`. Keyed by the
    /// full 32-byte blake3 of `fact_bytes` — hash equality *is* fact equality
    /// (collision axiom, `10-data-model.md`), and the `BTreeMap` gives the
    /// deterministic commit order the 50-storage doc requires.
    ///
    /// **The net-disposition invariant** (docs/architecture/50-storage.md):
    /// the insert set contains exactly the facts commit will add and the
    /// delete set exactly the facts it will remove — dispositions are
    /// proved against committed state at op time (authoritative under the
    /// single-writer mutex), redundant ops record nothing, and an op
    /// cancels a pending opposite instead of overwriting it. Judging a
    /// no-op insert is unrepresentable.
    facts: BTreeMap<(RelationId, [u8; 32]), (ArenaSlice, Disposition)>,
    /// `key statement → (determinant bytes → net disposition)` — the point-read
    /// index maintained beside the fact map by `insert`/`delete`
    /// (`docs/architecture/50-storage.md` § `WriteTx` point reads). Determinant
    /// bytes are derived by the one shared slicer
    /// ([`crate::storage::keys::determinant_image`]), exactly as commit derives
    /// them. No relation id in the key: the validation-minted key witness
    /// determines its relation. Nested so
    /// the probe borrows: `determinant_overlay` looks determinant bytes up as
    /// `&[u8]`, never boxing a key copy (the typed point read is
    /// host-allocation-free — PRD 22's gate).
    determinants: BTreeMap<KeyId, BTreeMap<DeterminantImage, DeterminantDisposition>>,
    /// Scratch for determinant derivation, reused across `insert`/`delete` calls
    /// (the write path may allocate, but not per key statement per fact):
    /// cloned into the determinant map only the first time a tuple is
    /// recorded — an overwrite updates the resident entry in place.
    determinant_scratch: DeterminantImage,
    /// Test-only pin of the scratch's clone discipline: how many times the
    /// scratch was cloned into the determinant map — exactly once per
    /// distinct `(key statement, determinant)` tuple recorded, never per
    /// overwrite.
    #[cfg(test)]
    determinant_scratch_clones: u64,
    /// Fresh sequences touched this transaction, lazily initialized
    /// from `Q` once per `(relation, field)`. A mark is *dirty* — it
    /// escaped as an allocation the closure may have returned — iff its
    /// `next` advanced past its `base`. Dirty marks persist even on a
    /// no-op commit (`50-storage.md`).
    marks: BTreeMap<(RelationId, FieldId), FreshMark>,
    /// Net row-count change per relation, maintained alongside the
    /// changed-state reports (flushed to `S` by the 50-storage doc).
    row_count_delta: BTreeMap<RelationId, i64>,
    /// Novel strings interned by this transaction: provisional ids
    /// assigned from the committed dictionary counter (the counter is
    /// in-memory-then-flush like every other counter; single-writer
    /// discipline makes provisional = final). The dictionary is str-only
    /// — bytes<N> values are inline, never interned — so one untagged
    /// map; probes borrow the raw bytes (`BTreeMap<Box<[u8]>, _>` looks
    /// up by `&[u8]`).
    pending_interns: BTreeMap<Box<[u8]>, u64>,
    /// The next dictionary id, lazily read once per transaction.
    dict_next: Option<u64>,
}

impl<'s> WriteDelta<'s> {
    /// The schema this delta was accumulated against (reader: commit).
    pub(crate) fn schema(&self) -> &'s Schema {
        self.schema
    }

    /// Whether the delta records no dispositions at all (reader: the 50-storage doc's
    /// skip-empty-commit rule). A successful commit of an empty delta
    /// still persists any *dirty* fresh marks — the closure may have
    /// returned those ids to the host, and a successful commit persists
    /// every fresh value it issued (`10-data-model.md`). Pending interns
    /// of an empty delta are deliberately dropped: intern ids never
    /// escape (hosts see values, not words).
    pub(crate) fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// The dictionary next-id to flush, if this transaction minted any
    /// provisional ids (reader: the 50-storage doc phase 4).
    pub(crate) fn dict_next(&self) -> Option<u64> {
        self.dict_next
    }
}
