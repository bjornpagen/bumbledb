//! Allocation-bounded heap staging for [`crate::InstanceBuilder`].
//!
//! [`HeapStage`] is the construction representation: chunked arenas, a
//! compact [`FactRef`] table, an open-addressed identity index, compact
//! dictionary slots, and dense fresh floors. It is not a catalog. Packed
//! freeze (step 8) consumes the stage into sorted runs and then a
//! [`super::CandidateCatalog`]. A `BTreeMap<Vec<u8>, Vec<u8>>` heap
//! catalog is refused.

use crate::arena::{Arena, ArenaSlice};
use crate::encoding::{InternId, decode_u64, fact_hash, field_word_bytes};
use crate::error::{Error, Result};
use crate::schema::{Generation, KeyId, Schema};
use crate::storage::delta::{DeltaEffect, Disposition};
use crate::storage::keys::{self, DeterminantImage};
use bumbledb_theory::schema::{FieldId, RelationId};

#[cfg(test)]
mod tests;

const EMPTY: u32 = u32::MAX;
const TOMBSTONE: u32 = u32::MAX - 1;

/// Compact handle to one net-live staged fact. Bytes live in the fact
/// arena; identity is `(relation, hash)`. `seq` is insert order, so
/// keyed overlay last-wins survives [`Vec::swap_remove`].
#[derive(Clone, Copy)]
pub(crate) struct FactRef {
    pub(crate) relation: RelationId,
    pub(crate) hash: [u8; 32],
    bytes: ArenaSlice,
    seq: u32,
}

/// Open-addressed index: slot values are table indices, `EMPTY`, or
/// `TOMBSTONE`. Capacity is a power of two.
struct OpenIndex {
    slots: Box<[u32]>,
    live: usize,
    tombs: usize,
}

impl OpenIndex {
    fn new() -> Self {
        Self {
            slots: Box::new([]),
            live: 0,
            tombs: 0,
        }
    }

    fn byte_size(&self) -> usize {
        self.slots.len() * std::mem::size_of::<u32>()
    }

    fn needs_grow(&self) -> bool {
        let cap = self.slots.len();
        cap == 0 || (self.live + self.tombs) * 4 >= cap * 3
    }

    fn next_cap(&self) -> usize {
        self.slots.len().saturating_mul(2).max(8)
    }

    fn probe(hash: u64, cap: usize) -> usize {
        #[allow(clippy::cast_possible_truncation)]
        {
            hash as usize & (cap - 1)
        }
    }

    fn lookup(&self, hash: u64, mut eq: impl FnMut(u32) -> bool) -> Option<u32> {
        let cap = self.slots.len();
        if cap == 0 {
            return None;
        }
        let mut pos = Self::probe(hash, cap);
        for _ in 0..cap {
            match self.slots[pos] {
                EMPTY => return None,
                TOMBSTONE => {}
                idx if eq(idx) => return Some(idx),
                _ => {}
            }
            pos = (pos + 1) & (cap - 1);
        }
        None
    }

    fn insert_vacant(&mut self, hash: u64, value: u32) {
        debug_assert!(!self.slots.is_empty());
        let cap = self.slots.len();
        let mut pos = Self::probe(hash, cap);
        let mut tomb = None;
        loop {
            match self.slots[pos] {
                EMPTY => {
                    let at = tomb.unwrap_or(pos);
                    self.slots[at] = value;
                    self.live += 1;
                    if tomb.is_some() {
                        self.tombs -= 1;
                    }
                    return;
                }
                TOMBSTONE => {
                    tomb = tomb.or(Some(pos));
                }
                _ => {}
            }
            pos = (pos + 1) & (cap - 1);
        }
    }

    fn remove(&mut self, hash: u64, mut eq: impl FnMut(u32) -> bool) -> Option<u32> {
        let cap = self.slots.len();
        if cap == 0 {
            return None;
        }
        let mut pos = Self::probe(hash, cap);
        for _ in 0..cap {
            match self.slots[pos] {
                EMPTY => return None,
                TOMBSTONE => {}
                idx if eq(idx) => {
                    self.slots[pos] = TOMBSTONE;
                    self.live -= 1;
                    self.tombs += 1;
                    return Some(idx);
                }
                _ => {}
            }
            pos = (pos + 1) & (cap - 1);
        }
        None
    }

    fn update_value(&mut self, hash: u64, old: u32, new: u32) {
        let cap = self.slots.len();
        debug_assert!(cap > 0);
        let mut pos = Self::probe(hash, cap);
        for _ in 0..cap {
            if self.slots[pos] == old {
                self.slots[pos] = new;
                return;
            }
            if self.slots[pos] == EMPTY {
                unreachable!("moved fact is live in the identity index");
            }
            pos = (pos + 1) & (cap - 1);
        }
        unreachable!("moved fact is live in the identity index");
    }
}

/// Compact dictionary forward slot: hash, id, and arena coordinates.
struct DictSlot {
    hash: [u8; 32],
    id: InternId,
    bytes: ArenaSlice,
}

/// Chunked heap construction state. Net-live facts only — an empty base
/// never records a delete disposition.
pub(crate) struct HeapStage {
    fact_arena: Arena,
    facts: Vec<FactRef>,
    identity: OpenIndex,
    dict_arena: Arena,
    dict_entries: Vec<DictSlot>,
    dict_index: OpenIndex,
    /// Reverse slots, dense from intern id 0.
    dict_reverse: Vec<ArenaSlice>,
    dict_next: u64,
    /// Schema fresh-field roster; [`Self::floors`] is parallel.
    roster: Box<[(RelationId, FieldId)]>,
    floors: Box<[u64]>,
    /// Monotone insert stamp for keyed last-wins.
    next_seq: u32,
}

impl HeapStage {
    pub(crate) fn new(schema: &Schema) -> Self {
        let roster = fresh_roster(schema);
        let floors = vec![0u64; roster.len()].into_boxed_slice();
        Self {
            fact_arena: Arena::new(),
            facts: Vec::new(),
            identity: OpenIndex::new(),
            dict_arena: Arena::new(),
            dict_entries: Vec::new(),
            dict_index: OpenIndex::new(),
            dict_reverse: Vec::new(),
            dict_next: 0,
            roster,
            floors,
            next_seq: 0,
        }
    }

    #[must_use]
    pub(crate) fn fact_refs(&self) -> &[FactRef] {
        &self.facts
    }

    pub(crate) fn fact_bytes(&self, fact: FactRef) -> &[u8] {
        self.fact_arena.get(fact.bytes)
    }

    pub(crate) fn apply(
        &mut self,
        schema: &Schema,
        relation: RelationId,
        fact_bytes: &[u8],
        want: Disposition,
    ) -> DeltaEffect {
        if want == Disposition::Insert {
            self.advance_fresh(schema, relation, fact_bytes);
        }
        let hash = fact_hash(fact_bytes);
        match (self.find(relation, &hash), want) {
            (Some(_), Disposition::Delete) => {
                self.remove(relation, &hash);
                DeltaEffect::Cancelled
            }
            (None, Disposition::Insert) => {
                self.insert_fact(relation, hash, fact_bytes);
                DeltaEffect::Recorded
            }
            (Some(_), Disposition::Insert) | (None, Disposition::Delete) => DeltaEffect::NoOp,
        }
    }

    pub(crate) fn contains(&self, relation: RelationId, fact_bytes: &[u8]) -> bool {
        self.find(relation, &fact_hash(fact_bytes)).is_some()
    }

    pub(crate) fn intern_str(&mut self, value: &str) -> InternId {
        self.intern(value.as_bytes())
    }

    pub(crate) fn resolve_str(&self, value: &str) -> Option<InternId> {
        self.lookup_raw(value.as_bytes())
    }

    pub(crate) fn pending_raw(&self, id: InternId) -> Option<&[u8]> {
        let idx = usize::try_from(id.raw()).ok()?;
        let slice = *self.dict_reverse.get(idx)?;
        Some(self.dict_arena.get(slice))
    }

    pub(crate) fn reserve(
        &mut self,
        schema: &Schema,
        relation: RelationId,
        field: FieldId,
        count: std::num::NonZeroU64,
    ) -> Result<u64> {
        schema.check_fresh_field(relation, field)?;
        let idx = self
            .floor_index(relation, field)
            .expect("fresh roster covers every Fresh field");
        let next = self.floors[idx];
        let end = next
            .checked_add(count.get())
            .ok_or(Error::FreshExhausted { relation, field })?;
        self.floors[idx] = end;
        Ok(next)
    }

    /// Last-wins overlay among net-live facts. Empty base: a miss is
    /// absence, never a committed shadow.
    pub(crate) fn overlay_fact(
        &self,
        schema: &Schema,
        relation: RelationId,
        key: KeyId,
        determinant: &[u8],
    ) -> Option<&[u8]> {
        let statement = schema.key(key);
        let rel = schema.relation(relation);
        let mut scratch = DeterminantImage::scratch_with_capacity(determinant.len());
        let mut best: Option<(u32, ArenaSlice)> = None;
        for fact in &self.facts {
            if fact.relation != relation {
                continue;
            }
            keys::determinant_image(
                rel.layout().encoded(self.fact_arena.get(fact.bytes)),
                &statement.projection,
                &mut scratch,
            );
            if scratch.as_bytes() == determinant && best.is_none_or(|(seq, _)| fact.seq > seq) {
                best = Some((fact.seq, fact.bytes));
            }
        }
        best.map(|(_, bytes)| self.fact_arena.get(bytes))
    }

    #[cfg(test)]
    pub(crate) fn intern_count(&self) -> usize {
        self.dict_entries.len()
    }

    /// Staged fact + dictionary arena capacity and compact tables — $A$.
    #[must_use]
    pub(crate) fn phase_a(&self) -> u64 {
        let tables = self.facts.capacity() * std::mem::size_of::<FactRef>()
            + self.dict_entries.capacity() * std::mem::size_of::<DictSlot>()
            + self.dict_reverse.capacity() * std::mem::size_of::<ArenaSlice>()
            + self.floors.len() * std::mem::size_of::<u64>()
            + self.roster.len() * std::mem::size_of::<(RelationId, FieldId)>();
        u64::try_from(self.fact_arena.capacity() + self.dict_arena.capacity() + tables)
            .expect("stage bytes fit u64")
    }

    /// Open-addressed identity + dictionary index capacity — $I$.
    #[must_use]
    pub(crate) fn phase_i(&self) -> u64 {
        u64::try_from(self.identity.byte_size() + self.dict_index.byte_size())
            .expect("index bytes fit u64")
    }

    fn find(&self, relation: RelationId, hash: &[u8; 32]) -> Option<u32> {
        let mixed = mix_identity(relation, hash);
        self.identity.lookup(mixed, |idx| {
            let fact = &self.facts[idx as usize];
            fact.relation == relation && fact.hash == *hash
        })
    }

    fn insert_fact(&mut self, relation: RelationId, hash: [u8; 32], fact_bytes: &[u8]) {
        let bytes = self.fact_arena.alloc(fact_bytes);
        let idx = u32::try_from(self.facts.len()).expect("fact count fits u32");
        let seq = self.next_seq;
        self.next_seq = self.next_seq.checked_add(1).expect("fact seq fits u32");
        self.facts.push(FactRef {
            relation,
            hash,
            bytes,
            seq,
        });
        if self.identity.needs_grow() {
            self.rehash_identity();
        } else {
            self.identity
                .insert_vacant(mix_identity(relation, &hash), idx);
        }
    }

    fn remove(&mut self, relation: RelationId, hash: &[u8; 32]) {
        let mixed = mix_identity(relation, hash);
        let idx = self
            .identity
            .remove(mixed, |slot| {
                let fact = &self.facts[slot as usize];
                fact.relation == relation && fact.hash == *hash
            })
            .expect("cancel removes a live identity");
        let idx = idx as usize;
        let last = self.facts.len() - 1;
        self.facts.swap_remove(idx);
        if idx != last {
            let moved = self.facts[idx];
            self.identity.update_value(
                mix_identity(moved.relation, &moved.hash),
                u32::try_from(last).expect("fact count fits u32"),
                u32::try_from(idx).expect("fact count fits u32"),
            );
        }
    }

    fn rehash_identity(&mut self) {
        let cap = self.identity.next_cap();
        self.identity.slots = vec![EMPTY; cap].into_boxed_slice();
        self.identity.live = 0;
        self.identity.tombs = 0;
        for (i, fact) in self.facts.iter().enumerate() {
            self.identity.insert_vacant(
                mix_identity(fact.relation, &fact.hash),
                u32::try_from(i).expect("fact count fits u32"),
            );
        }
    }

    fn intern(&mut self, raw: &[u8]) -> InternId {
        let hash = *blake3::hash(raw).as_bytes();
        if let Some(id) = self.lookup(&hash, raw) {
            return id;
        }
        let id = InternId::from_raw(self.dict_next);
        assert!(
            !id.is_sentinel(),
            "dictionary id space exhausted (u64::MAX is the miss sentinel)"
        );
        let bytes = self.dict_arena.alloc(raw);
        let idx = u32::try_from(self.dict_entries.len()).expect("intern count fits u32");
        self.dict_entries.push(DictSlot { hash, id, bytes });
        self.dict_reverse.push(bytes);
        self.dict_next += 1;
        if self.dict_index.needs_grow() {
            self.rehash_dict();
        } else {
            self.dict_index.insert_vacant(mix_bytes(&hash), idx);
        }
        id
    }

    pub(crate) fn discard_identity(&mut self) {
        self.identity = OpenIndex::new();
    }

    pub(crate) fn release_facts(&mut self) {
        self.facts.clear();
        self.facts.shrink_to_fit();
        self.fact_arena = Arena::new();
        self.identity = OpenIndex::new();
    }

    pub(crate) fn roster(&self) -> &[(RelationId, FieldId)] {
        &self.roster
    }

    pub(crate) fn floors(&self) -> &[u64] {
        &self.floors
    }

    pub(crate) fn lookup_raw(&self, raw: &[u8]) -> Option<InternId> {
        let hash = *blake3::hash(raw).as_bytes();
        self.lookup(&hash, raw)
    }

    fn lookup(&self, hash: &[u8; 32], raw: &[u8]) -> Option<InternId> {
        let mixed = mix_bytes(hash);
        self.dict_index
            .lookup(mixed, |idx| {
                let slot = &self.dict_entries[idx as usize];
                slot.hash == *hash && self.dict_arena.get(slot.bytes) == raw
            })
            .map(|idx| self.dict_entries[idx as usize].id)
    }

    fn rehash_dict(&mut self) {
        let cap = self.dict_index.next_cap();
        self.dict_index.slots = vec![EMPTY; cap].into_boxed_slice();
        self.dict_index.live = 0;
        self.dict_index.tombs = 0;
        for (i, slot) in self.dict_entries.iter().enumerate() {
            self.dict_index.insert_vacant(
                mix_bytes(&slot.hash),
                u32::try_from(i).expect("intern count fits u32"),
            );
        }
    }

    fn floor_index(&self, relation: RelationId, field: FieldId) -> Option<usize> {
        self.roster
            .iter()
            .position(|pair| *pair == (relation, field))
    }

    fn advance_fresh(&mut self, schema: &Schema, relation: RelationId, fact_bytes: &[u8]) {
        let rel = schema.relation(relation);
        for (idx, field) in rel.fields().iter().enumerate() {
            if field.generation != Generation::Fresh {
                continue;
            }
            let field_id = FieldId(u16::try_from(idx).expect("field count fits u16"));
            let Some(floor) = self.floor_index(relation, field_id) else {
                continue;
            };
            let value = decode_u64(field_word_bytes(rel.layout().encoded(fact_bytes), idx));
            self.floors[floor] = self.floors[floor].max(value.saturating_add(1));
        }
    }
}

fn fresh_roster(schema: &Schema) -> Box<[(RelationId, FieldId)]> {
    let mut roster = Vec::new();
    for (ri, rel) in schema.relations().iter().enumerate() {
        let relation = RelationId(u32::try_from(ri).expect("relation count fits u32"));
        for (fi, field) in rel.fields().iter().enumerate() {
            if field.generation == Generation::Fresh {
                roster.push((
                    relation,
                    FieldId(u16::try_from(fi).expect("field count fits u16")),
                ));
            }
        }
    }
    roster.into_boxed_slice()
}

fn mix_identity(relation: RelationId, hash: &[u8; 32]) -> u64 {
    mix(u64::from(relation.0), hash)
}

fn mix_bytes(hash: &[u8; 32]) -> u64 {
    mix(0, hash)
}

fn mix(seed: u64, hash: &[u8; 32]) -> u64 {
    let mut h = seed;
    for chunk in hash.chunks(8) {
        let mut word = [0u8; 8];
        word[..chunk.len()].copy_from_slice(chunk);
        h = h.rotate_left(29) ^ u64::from_le_bytes(word);
    }
    h
}
