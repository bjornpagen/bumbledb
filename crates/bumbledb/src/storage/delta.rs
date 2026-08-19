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

pub(crate) use alloc::read_fresh_next;

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

/// What one [`WriteDelta::apply`] did to the fact map. Cancel and record
/// both change the in-memory final-state view; they are not the same
/// outcome — cancel *removes* an entry, record *adds* one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaEffect {
    /// Pending already matched `want`, or committed state already does.
    NoOp,
    /// A new disposition entry appeared.
    Recorded,
    /// The pending opposite disposition disappeared.
    Cancelled,
}

impl DeltaEffect {
    /// Whether the in-memory final-state view moved.
    #[must_use]
    pub const fn changed(self) -> bool {
        !matches!(self, Self::NoOp)
    }
}

/// A determinant-map hit, resolved for point readers: the pending fact that
/// establishes the key tuple in the final state, or its recorded absence.
/// A map miss (no overlay at all) means the committed state answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterminantOverlay<'a> {
    Present(&'a [u8]),
    Absent,
}

/// Pending owners of one determinant tuple: at most one *live* insert
/// (point-read last-wins), plus any deletes of other facts that share
/// the tuple (`delete(old); insert(new)`). A second insert of the same
/// tuple replaces the live owner and stashes the previous insert so
/// cancel can restore it — two live inserts are unrepresentable here,
/// but the earlier fact remains in the fact map until cancelled. Emptying
/// this value removes the overlay entry, never records `Absent`.
enum TupleOwners {
    Insert {
        fact: ArenaSlice,
        /// Inserts this tuple's later insert replaced. Not live for
        /// point-reads; cancel of the live insert restores the last one.
        replaced: Vec<ArenaSlice>,
        deletes: Vec<ArenaSlice>,
    },
    /// At least one delete owner. Empty is overlay-removed, not this variant.
    Deletes {
        head: ArenaSlice,
        rest: Vec<ArenaSlice>,
    },
}

/// The fact-disposition table's concrete shape ([`WriteDelta::facts`]).
type FactMap = std::collections::HashMap<
    (RelationId, [u8; 32]),
    (ArenaSlice, Disposition),
    std::hash::BuildHasherDefault<FactKeyHasher>,
>;

/// The fact map's hasher: the key already CONTAINS a blake3 hash — 32
/// uniform bytes — so hashing it again (`SipHash` over 40 bytes) is pure
/// waste. One xor-rotate fold per 8-byte chunk keeps the blake3
/// uniformity and costs five folds per probe; the rotate keeps the
/// relation prefix and the slice-length word from cancelling into the
/// hash bytes.
#[derive(Default)]
struct FactKeyHasher(u64);

impl std::hash::Hasher for FactKeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.0 = self.0.rotate_left(29) ^ u64::from_le_bytes(word);
        }
    }
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
    /// (collision axiom, `10-data-model.md`). A hash table, not an ordered
    /// map: accumulation is the write path's hot loop and pays O(1) probes
    /// against an already-uniform key ([`FactKeyHasher`] folds, never
    /// re-hashes); the deterministic `(relation, fact_hash)` commit order
    /// the 50-storage doc requires is restored by ONE exact sort at plan
    /// derivation (`commit::plan::plan_commit` — the sort-license
    /// precedent is the T8 probe sort, `commit/judgment.rs`).
    ///
    /// **The net-disposition invariant** (docs/architecture/50-storage.md):
    /// the insert set contains exactly the facts commit will add and the
    /// delete set exactly the facts it will remove — dispositions are
    /// proved against committed state at op time (authoritative under the
    /// single-writer mutex), redundant ops record nothing, and an op
    /// cancels a pending opposite instead of overwriting it. Judging a
    /// no-op insert is unrepresentable.
    facts: FactMap,
    /// `key statement → (determinant bytes → pending owners)` — the point-read
    /// index maintained beside the fact map by `insert`/`delete`
    /// (`docs/architecture/50-storage.md` § `WriteTx` point reads). Determinant
    /// bytes are derived by the one shared slicer
    /// ([`crate::storage::keys::determinant_image`]), exactly as commit derives
    /// them. No relation id in the key: the validation-minted key witness
    /// determines its relation. Nested so
    /// the probe borrows: `determinant_overlay` looks determinant bytes up as
    /// `&[u8]`, never boxing a key copy (the typed point read is
    /// host-allocation-free — PRD 22's gate).
    determinants: BTreeMap<KeyId, BTreeMap<DeterminantImage, TupleOwners>>,
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
    /// Novel strings interned this transaction: the next-id and the
    /// entries are one value — the counter cannot advance without them.
    interns: Option<PendingInterns>,
}

/// Provisional dictionary mints of one write: the next-id sits beside
/// the entries it accounts for. `None` on [`WriteDelta::interns`] is
/// "minted nothing"; an empty entry map is unrepresentable.
pub(crate) struct PendingInterns {
    next_id: u64,
    entries: BTreeMap<Box<[u8]>, crate::encoding::InternId>,
}

impl PendingInterns {
    fn first(raw: &[u8], id: crate::encoding::InternId, next_id: u64) -> Self {
        let mut entries = BTreeMap::new();
        entries.insert(Box::from(raw), id);
        Self { next_id, entries }
    }

    /// The dictionary next-id to flush with these entries.
    #[must_use]
    pub(crate) fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Pending intern entries, keyed by raw bytes.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&[u8], crate::encoding::InternId)> + '_ {
        self.entries.iter().map(|(raw, id)| (raw.as_ref(), *id))
    }

    fn get(&self, raw: &[u8]) -> Option<crate::encoding::InternId> {
        self.entries.get(raw).copied()
    }

    fn pending_raw(&self, id: crate::encoding::InternId) -> Option<&[u8]> {
        self.entries
            .iter()
            .find_map(|(raw, &candidate)| (candidate == id).then_some(raw.as_ref()))
    }

    fn insert(&mut self, raw: &[u8], id: crate::encoding::InternId, next_id: u64) {
        self.entries.insert(Box::from(raw), id);
        self.next_id = next_id;
    }
}

impl WriteDelta<'_> {
    /// The schema this delta was accumulated against (reader: commit).
    pub(crate) fn schema(&self) -> &Schema {
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

    /// Pending interns of this transaction, if any were minted. The
    /// next-id travels with the entries.
    pub(crate) fn interns(&self) -> Option<&PendingInterns> {
        self.interns.as_ref()
    }

    /// The dictionary next-id to flush, if this transaction minted any
    /// provisional ids (reader: the 50-storage doc phase 4).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn dict_next(&self) -> Option<u64> {
        self.interns.as_ref().map(PendingInterns::next_id)
    }
}
