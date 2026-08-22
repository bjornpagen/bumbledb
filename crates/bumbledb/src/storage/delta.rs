//! The write transaction delta core: a write transaction is an
//! in-memory net insert-set and delete-set of canonical fact bytes — **net
//! dispositions against committed state** — plus in-memory counters
//! .
//! During accumulation, `insert`/`delete` are pure set arithmetic: encode is
//! the caller's job; membership is the delta's own disposition if present,

use std::cell::RefCell;
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
/// op time: an `Insert` entry's fact is
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
    NoOp,

    Recorded,

    Cancelled,
}

impl DeltaEffect {
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

enum TupleOwners {
    Insert {
        fact: ArenaSlice,

        replaced: Vec<ArenaSlice>,
        deletes: Vec<ArenaSlice>,
    },

    Deletes {
        head: ArenaSlice,
        rest: Vec<ArenaSlice>,
    },
}

type FactMap = std::collections::HashMap<
    (RelationId, [u8; 32]),
    (ArenaSlice, Disposition),
    std::hash::BuildHasherDefault<FactKeyHasher>,
>;

type MemoMap = std::collections::HashMap<
    [u8; 32],
    crate::encoding::InternId,
    std::hash::BuildHasherDefault<FactKeyHasher>,
>;

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

#[derive(Debug, Clone, Copy)]
struct FreshMark {
    base: u64,

    next: u64,
}

/// The accumulated write transaction.
pub struct WriteDelta<'s> {
    schema: &'s Schema,
    arena: Arena,

    facts: FactMap,

    determinants: BTreeMap<KeyId, BTreeMap<DeterminantImage, TupleOwners>>,

    determinant_scratch: DeterminantImage,

    #[cfg(test)]
    determinant_scratch_clones: u64,

    marks: BTreeMap<(RelationId, FieldId), FreshMark>,

    row_count_delta: BTreeMap<RelationId, i64>,

    interns: Option<PendingInterns>,

    /// transaction's mints — so before it, a COMMITTED string paid
    /// blake3 + one LMDB get on every occurrence. Sound because the
    committed_memo: CommittedMemo,
}

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

    #[must_use]
    pub(crate) fn next_id(&self) -> u64 {
        self.next_id
    }

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

/// Interior mutability because recording a read must not demand `&mut`:
/// `resolve` is `&self` — the commit judgment's selection-literal encoding
/// resolves through a shared borrow — and the delta is confined to the single
/// writer (`MutationCore` is `Send + !Sync` by construction,
/// `api/db/mutation_core.rs`: it moves whole to the binding's async task, it is
/// never shared across threads), so the `RefCell` is uncontended by law and its
/// borrows are straight-line inside `resolve`.
#[derive(Default)]
struct CommittedMemo {
    ids: RefCell<MemoMap>,
}

impl CommittedMemo {
    fn get(&self, hash: &[u8; 32]) -> Option<crate::encoding::InternId> {
        self.ids.borrow().get(hash).copied()
    }

    fn record(&self, hash: [u8; 32], id: crate::encoding::InternId) {
        self.ids.borrow_mut().insert(hash, id);
    }
}

impl WriteDelta<'_> {
    pub(crate) fn schema(&self) -> &Schema {
        self.schema
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    pub(crate) fn interns(&self) -> Option<&PendingInterns> {
        self.interns.as_ref()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn dict_next(&self) -> Option<u64> {
        self.interns.as_ref().map(PendingInterns::next_id)
    }
}
