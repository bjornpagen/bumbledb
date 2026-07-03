//! COLT — the Column-Oriented Lazy Trie (docs/architecture/30-execution.md), per paper §4.2 with the
//! chunked-child-list deviation (`docs/architecture/30-execution.md`).
//!
//! No `unsafe` anywhere: nodes, chunks, map slots, and key words live in
//! index-addressed pools (`NodeRef`-style u32 indices, never pointers) —
//! the representational fix for v5's `UnsafeCell` aliasing UB (post-mortem
//! §36). Nothing is ever built eagerly: a node is offsets into the base
//! columns until a `get` (or a non-suffix `iter`) forces exactly one level.
//!
//! Iteration is batched copy-out ([`Colt::iter_batch`]): entries are
//! `(key words, child cursor)` pairs — **the child comes with the key**;
//! re-probing the map just enumerated is inexpressible through this API
//! (post-mortem §34).

use crate::image::view::View;
use crate::image::ColumnView;

/// Positions per chunk: bounded pointer-chase, independent loads within a
/// chunk (the deviation from the paper's growable per-key vectors).
const CHUNK_LEN: usize = 64;

/// Labeled key count (post-mortem §40: an `Estimate` is duplicate-inflated
/// by construction; the two must never be compared as the same quantity).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCount {
    /// Distinct keys of a forced map.
    Exact(u64),
    /// Position count of an unforced vector (duplicate-inflated).
    Estimate(u64),
}

/// A reference into the trie: either a real node or a single image
/// position pinned by a singleton child (no node is allocated for it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Node(NodeRef),
    Row(u32),
}

/// Index of a node in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRef(u32);

/// Opaque resume token for [`Colt::iter_batch`]; start at `default()`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BatchToken(u64);

/// Where an unforced node's positions come from.
#[derive(Debug, Clone, Copy)]
enum Positions {
    /// The root: iterate the view directly (all positions or survivors).
    Root,
    /// A chunked child list: `(first, last, count)`.
    Chunks { first: u32, last: u32, count: u32 },
}

#[derive(Debug, Clone, Copy)]
enum NodeState {
    Unforced(Positions),
    Forced { map: u32 },
}

#[derive(Debug, Clone, Copy)]
struct Chunk {
    positions: [u32; CHUNK_LEN],
    len: u8,
    /// Next chunk index, or `u32::MAX`.
    next: u32,
}

#[derive(Debug, Clone, Copy)]
enum Slot {
    Empty,
    /// Singleton optimization: the first position lives inline; a chunked
    /// node is allocated only on the second.
    Single(u32),
    Node(NodeRef),
}

/// One forced level's open-addressed map: power-of-two capacity, inline
/// key words, linear probing, no tombstones (build-once, never deleted
/// from). Capacity is sized once from the position count — no rehash.
#[derive(Debug, Clone, Copy)]
struct Map {
    arity: usize,
    capacity: usize,
    len: u32,
    keys_start: usize,
    slots_start: usize,
}

/// The lazy trie over one occurrence's view. Owns the view (a cheap
/// enum over an `Arc`'d image plus survivor positions) and its pools, so a
/// prepared query can hold and [`Colt::reset`] it across executions with
/// every capacity retained (the 30-execution doc's zero-alloc discipline).
pub struct Colt {
    view: View,
    /// Per trie level, the image column index of each key variable.
    schema_columns: Vec<Vec<usize>>,
    nodes: Vec<NodeState>,
    chunks: Vec<Chunk>,
    maps: Vec<Map>,
    slots: Vec<Slot>,
    keys: Vec<u64>,
    /// Reused key-decoding scratch.
    scratch: Vec<u64>,
}

/// The probe hash for a key — exposed so the vectorized executor's phase 1
/// can compute all hashes (pure ALU) before phase 2 issues any bucket load
/// (D4's two-phase probing, the 30-execution doc).
#[must_use]
pub fn hash_key(words: &[u64]) -> u64 {
    hash_words(words)
}

fn hash_words(words: &[u64]) -> u64 {
    let mut h = 0x517C_C1B7_2722_0A95_u64;
    for w in words {
        h ^= *w;
        h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= h >> 29;
    }
    h
}

impl Colt {
    /// Builds the root over a view: O(1) — nothing decodes until a force.
    #[must_use]
    pub fn new(view: View, schema_columns: Vec<Vec<usize>>) -> Self {
        Self {
            view,
            schema_columns,
            nodes: vec![NodeState::Unforced(Positions::Root)],
            chunks: Vec::new(),
            maps: Vec::new(),
            slots: Vec::new(),
            keys: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// Swaps in a fresh view for the next execution, clearing every pool
    /// while retaining capacity (post-warmup executions of same-shaped
    /// data allocate nothing here). Returns the old view so its survivor
    /// buffer can be recycled.
    pub fn reset(&mut self, view: View) -> View {
        let old = std::mem::replace(&mut self.view, view);
        self.nodes.clear();
        self.nodes.push(NodeState::Unforced(Positions::Root));
        self.chunks.clear();
        self.maps.clear();
        self.slots.clear();
        self.keys.clear();
        old
    }

    /// The root cursor (level 0).
    #[must_use]
    pub fn root() -> Cursor {
        Cursor::Node(NodeRef(0))
    }

    /// Key arity at a level.
    #[must_use]
    pub fn arity(&self, level: usize) -> usize {
        self.schema_columns[level].len()
    }

    /// Total pool footprint — the test observability for laziness
    /// (allocations only ever grow this).
    #[cfg(test)]
    #[must_use]
    pub fn watermark(&self) -> usize {
        self.nodes.len() + self.chunks.len() + self.maps.len() + self.slots.len() + self.keys.len()
    }

    /// The labeled key count at a cursor (never forces).
    #[must_use]
    pub fn key_count(&self, cursor: Cursor) -> KeyCount {
        match cursor {
            Cursor::Row(_) => KeyCount::Estimate(1),
            Cursor::Node(node) => match self.nodes[node.0 as usize] {
                NodeState::Forced { map } => {
                    KeyCount::Exact(u64::from(self.maps[map as usize].len))
                }
                NodeState::Unforced(Positions::Root) => KeyCount::Estimate(self.view.len() as u64),
                NodeState::Unforced(Positions::Chunks { count, .. }) => {
                    KeyCount::Estimate(u64::from(count))
                }
            },
        }
    }

    /// Decodes the key word of one column at one position (1-byte columns
    /// widen to u64 — binding slots are words everywhere).
    fn word_at(&self, column: usize, position: u32) -> u64 {
        match self.view.image().column(column) {
            ColumnView::Words(words) => words[position as usize],
            ColumnView::Bytes(bytes) => u64::from(bytes[position as usize]),
        }
    }

    /// Whether the position's key words at `level` equal `key`.
    fn position_matches(&self, level: usize, position: u32, key: &[u64]) -> bool {
        self.schema_columns[level]
            .iter()
            .zip(key)
            .all(|(col, expected)| self.word_at(*col, position) == *expected)
    }

    /// Probes for `key` at `cursor`'s level, forcing the node if needed.
    /// Returns the child cursor on a hit. (The executor probes through
    /// [`Colt::get_prehashed`]; this convenience form serves the tests.)
    #[cfg(test)]
    pub fn get(&mut self, cursor: Cursor, level: usize, key: &[u64]) -> Option<Cursor> {
        self.get_prehashed(cursor, level, key, hash_words(key))
    }

    /// Probe with a precomputed hash (phase 2 of the two-phase batched
    /// probe): the load chain starts here; the hash was phase-1 ALU work.
    pub fn get_prehashed(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Option<Cursor> {
        debug_assert_eq!(key.len(), self.arity(level));
        match cursor {
            // A pinned row: the probe is a field-equality check, and the
            // child stays pinned to the same position.
            Cursor::Row(position) => self
                .position_matches(level, position, key)
                .then_some(Cursor::Row(position)),
            Cursor::Node(node) => {
                let map = self.force(node, level);
                let m = self.maps[map as usize];
                let (found, idx) = self.probe_hashed(&m, key, hash);
                if !found {
                    return None;
                }
                match self.slots[m.slots_start + idx] {
                    Slot::Empty => unreachable!("probe hit an occupied slot"),
                    Slot::Single(position) => Some(Cursor::Row(position)),
                    Slot::Node(child) => Some(Cursor::Node(child)),
                }
            }
        }
    }

    /// Forces a node cursor ahead of a probe batch (no-op for pinned rows
    /// and already-forced nodes): phase 2's loads then hit a ready map.
    pub fn ensure_forced(&mut self, cursor: Cursor, level: usize) {
        if let Cursor::Node(node) = cursor {
            self.force(node, level);
        }
    }

    /// Copies up to `max` entries into the caller's buffers, returning the
    /// yielded count and the resume token. `keys_out` receives
    /// `yielded * arity(level)` words; `children_out` one cursor per entry.
    ///
    /// An unforced node iterates its positions directly only at the last
    /// level (the suffix rule, paper §4.2); anywhere else it forces first.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations: undersized caller buffers.
    pub fn iter_batch(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let arity = self.arity(level);
        assert!(keys_out.len() >= max * arity && children_out.len() >= max);
        match cursor {
            Cursor::Row(position) => {
                if token.0 > 0 {
                    return (0, token);
                }
                for (i, col) in self.schema_columns[level].iter().enumerate() {
                    keys_out[i] = self.word_at(*col, position);
                }
                children_out[0] = Cursor::Row(position);
                (1, BatchToken(1))
            }
            Cursor::Node(node) => {
                let is_suffix = level + 1 == self.schema_columns.len();
                match self.nodes[node.0 as usize] {
                    NodeState::Unforced(_) if is_suffix => {
                        self.iter_positions(node, level, token, keys_out, children_out, max)
                    }
                    NodeState::Unforced(_) => {
                        let map = self.force(node, level);
                        self.iter_map(map, level, token, keys_out, children_out, max)
                    }
                    NodeState::Forced { map } => {
                        self.iter_map(map, level, token, keys_out, children_out, max)
                    }
                }
            }
        }
    }

    /// Suffix iteration: yield each position's key words with a pinned-row
    /// child — no forcing, no allocation.
    ///
    /// The resume token is O(1) to advance: the root token is a plain view
    /// index; a chunked node's token packs `(chunk + 2, offset)` into the
    /// u64 (0 = start, high half 1 = exhausted) so a drain is O(k), never
    /// the O(k²/64) of re-walking the chain per position.
    fn iter_positions(
        &mut self,
        node: NodeRef,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let arity = self.arity(level);
        let mut yielded = 0;
        match self.nodes[node.0 as usize] {
            NodeState::Forced { .. } => unreachable!("caller checked unforced"),
            NodeState::Unforced(Positions::Root) => {
                let mut index = usize::try_from(token.0).expect("64-bit usize");
                while yielded < max && index < self.view.len() {
                    let position = self.view.position_at(index);
                    for (i, col) in self.schema_columns[level].iter().enumerate() {
                        keys_out[yielded * arity + i] = self.word_at(*col, position);
                    }
                    children_out[yielded] = Cursor::Row(position);
                    yielded += 1;
                    index += 1;
                }
                (yielded, BatchToken(index as u64))
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                const EXHAUSTED: u64 = 1 << 32;
                let (mut chunk, mut offset) = match token.0 {
                    0 => (first, 0usize),
                    EXHAUSTED => return (0, token),
                    packed => (
                        u32::try_from((packed >> 32) - 2).expect("packed chunk index"),
                        usize::try_from(packed & 0xFFFF_FFFF).expect("64-bit usize"),
                    ),
                };
                loop {
                    if yielded >= max {
                        break;
                    }
                    let c = self.chunks[chunk as usize];
                    if offset >= usize::from(c.len) {
                        if c.next == u32::MAX {
                            return (yielded, BatchToken(EXHAUSTED));
                        }
                        chunk = c.next;
                        offset = 0;
                        continue;
                    }
                    let position = c.positions[offset];
                    for (i, col) in self.schema_columns[level].iter().enumerate() {
                        keys_out[yielded * arity + i] = self.word_at(*col, position);
                    }
                    children_out[yielded] = Cursor::Row(position);
                    yielded += 1;
                    offset += 1;
                }
                (
                    yielded,
                    BatchToken((u64::from(chunk) + 2) << 32 | offset as u64),
                )
            }
        }
    }

    /// Map iteration: yield `(key words, child)` per occupied slot — the
    /// child comes with the key; no re-probe is possible.
    fn iter_map(
        &self,
        map: u32,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let m = self.maps[map as usize];
        let arity = self.arity(level);
        debug_assert_eq!(arity, m.arity);
        let mut slot_idx = usize::try_from(token.0).expect("64-bit usize");
        let mut yielded = 0;
        while yielded < max && slot_idx < m.capacity {
            match self.slots[m.slots_start + slot_idx] {
                Slot::Empty => {}
                Slot::Single(position) => {
                    let key = &self.keys[m.keys_start + slot_idx * arity..];
                    keys_out[yielded * arity..yielded * arity + arity]
                        .copy_from_slice(&key[..arity]);
                    children_out[yielded] = Cursor::Row(position);
                    yielded += 1;
                }
                Slot::Node(child) => {
                    let key = &self.keys[m.keys_start + slot_idx * arity..];
                    keys_out[yielded * arity..yielded * arity + arity]
                        .copy_from_slice(&key[..arity]);
                    children_out[yielded] = Cursor::Node(child);
                    yielded += 1;
                }
            }
            slot_idx += 1;
        }
        (yielded, BatchToken(slot_idx as u64))
    }

    /// Linear probe: returns (found, slot index within the map).
    fn probe(&self, m: &Map, key: &[u64]) -> (bool, usize) {
        self.probe_hashed(m, key, hash_words(key))
    }

    /// Linear probe with a precomputed hash.
    fn probe_hashed(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        let mask = m.capacity - 1;
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {
            if matches!(self.slots[m.slots_start + idx], Slot::Empty) {
                return (false, idx);
            }
            let stored = &self.keys[m.keys_start + idx * m.arity..];
            if &stored[..m.arity] == key {
                return (true, idx);
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Single-pass force: iterate the node's positions once, decoding key
    /// words and appending each position to its key's chunked child list.
    /// Returns the map index (idempotent).
    fn force(&mut self, node: NodeRef, level: usize) -> u32 {
        if let NodeState::Forced { map } = self.nodes[node.0 as usize] {
            return map;
        }
        let arity = self.arity(level);
        let count = match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => self.view.len() as u64,
            NodeState::Unforced(Positions::Chunks { count, .. }) => u64::from(count),
            NodeState::Forced { .. } => unreachable!("checked above"),
        };
        // Capacity sized once for <=50% load: no rehash, ever.
        let capacity = usize::try_from(count.max(1) * 2)
            .expect("64-bit usize")
            .next_power_of_two();
        let map_idx = u32::try_from(self.maps.len()).expect("map count fits u32");
        let slots_start = self.slots.len();
        let keys_start = self.keys.len();
        self.slots.resize(slots_start + capacity, Slot::Empty);
        self.keys.resize(keys_start + capacity * arity, 0);
        let mut m = Map {
            arity,
            capacity,
            len: 0,
            keys_start,
            slots_start,
        };

        // Single pass, O(1) advance per position: the root walks the view
        // by index (O(1) each); a chunked list walks its chain directly —
        // never `nth_position`'s from-the-head re-walk, which made forcing
        // a k-position child O(k²/64).
        match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => {
                for idx in 0..self.view.len() {
                    let position = self.view.position_at(idx);
                    self.force_ingest(&mut m, level, position);
                }
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = self.chunks[chunk as usize];
                    for i in 0..usize::from(c.len) {
                        self.force_ingest(&mut m, level, c.positions[i]);
                    }
                    chunk = c.next;
                }
            }
            NodeState::Forced { .. } => unreachable!("checked above"),
        }

        crate::obs::event(
            crate::obs::names::COLT_FORCE,
            crate::obs::Category::Execute,
            count,
            u64::from(m.len),
        );
        self.maps.push(m);
        self.nodes[node.0 as usize] = NodeState::Forced { map: map_idx };
        map_idx
    }

    /// One position of a [`Colt::force`] pass: decode its key words, probe,
    /// and land it (new slot or appended child).
    fn force_ingest(&mut self, m: &mut Map, level: usize, position: u32) {
        let arity = m.arity;
        self.scratch.clear();
        for col in &self.schema_columns[level] {
            let w = self.word_at(*col, position);
            self.scratch.push(w);
        }
        let key = std::mem::take(&mut self.scratch);
        let (found, idx) = self.probe(m, &key);
        if found {
            self.append_child(m.slots_start + idx, position);
        } else {
            self.keys[m.keys_start + idx * arity..m.keys_start + idx * arity + arity]
                .copy_from_slice(&key);
            self.slots[m.slots_start + idx] = Slot::Single(position);
            m.len += 1;
        }
        self.scratch = key;
    }

    /// Appends a position to an occupied slot's child: singleton inline
    /// first, a chunked node from the second position on.
    fn append_child(&mut self, slot_idx: usize, position: u32) {
        match self.slots[slot_idx] {
            Slot::Empty => unreachable!("appending to an occupied slot"),
            Slot::Single(first_position) => {
                // Second position: allocate the chunked child node now.
                let chunk_idx = u32::try_from(self.chunks.len()).expect("chunk count fits u32");
                let mut chunk = Chunk {
                    positions: [0; CHUNK_LEN],
                    len: 2,
                    next: u32::MAX,
                };
                chunk.positions[0] = first_position;
                chunk.positions[1] = position;
                self.chunks.push(chunk);
                let node_ref =
                    NodeRef(u32::try_from(self.nodes.len()).expect("node count fits u32"));
                self.nodes.push(NodeState::Unforced(Positions::Chunks {
                    first: chunk_idx,
                    last: chunk_idx,
                    count: 2,
                }));
                self.slots[slot_idx] = Slot::Node(node_ref);
            }
            Slot::Node(node_ref) => {
                let NodeState::Unforced(Positions::Chunks { first, last, count }) =
                    self.nodes[node_ref.0 as usize]
                else {
                    unreachable!("chunked children stay unforced during their parent's force");
                };
                let last_chunk = &mut self.chunks[last as usize];
                if usize::from(last_chunk.len) < CHUNK_LEN {
                    last_chunk.positions[usize::from(last_chunk.len)] = position;
                    last_chunk.len += 1;
                    self.nodes[node_ref.0 as usize] = NodeState::Unforced(Positions::Chunks {
                        first,
                        last,
                        count: count + 1,
                    });
                } else {
                    let new_idx = u32::try_from(self.chunks.len()).expect("chunk count fits u32");
                    let mut chunk = Chunk {
                        positions: [0; CHUNK_LEN],
                        len: 1,
                        next: u32::MAX,
                    };
                    chunk.positions[0] = position;
                    self.chunks.push(chunk);
                    self.chunks[last as usize].next = new_idx;
                    self.nodes[node_ref.0 as usize] = NodeState::Unforced(Positions::Chunks {
                        first,
                        last: new_idx,
                        count: count + 1,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::image::view::apply;
    use crate::schema::{
        FieldDescriptor, Generation, RelationDescriptor, RelationId, Schema, SchemaDescriptor,
        ValueType,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// R(k u64, v u64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "k".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                    FieldDescriptor {
                        name: "v".into(),
                        value_type: ValueType::U64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);

    /// Builds an image over committed (k, v) pairs.
    fn view_of(
        dir: &TempDir,
        schema: &Schema,
        rows: &[(u64, u64)],
    ) -> Arc<crate::image::RelationImage> {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (k, v) in rows {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(*k), ValueRef::U64(*v)],
                schema.relation(R).layout(),
                &mut bytes,
            );
            delta.insert(&view, R, &bytes).expect("insert");
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        crate::image::build(&txn, schema, R).expect("build")
    }

    fn all(image: &Arc<crate::image::RelationImage>) -> View {
        apply(image, &[], &[], Vec::new())
    }

    /// Drains every entry at a cursor/level into (key words, child) pairs.
    fn drain(colt: &mut Colt, cursor: Cursor, level: usize) -> Vec<(Vec<u64>, Cursor)> {
        let arity = colt.arity(level);
        let mut keys = vec![0u64; 8 * arity.max(1)];
        let mut children = vec![Cursor::Row(0); 8];
        let mut token = BatchToken::default();
        let mut out = Vec::new();
        loop {
            let (n, next) = colt.iter_batch(cursor, level, token, &mut keys, &mut children, 8);
            if n == 0 {
                break;
            }
            for i in 0..n {
                out.push((keys[i * arity..(i + 1) * arity].to_vec(), children[i]));
            }
            token = next;
        }
        out
    }

    #[test]
    fn construction_is_lazy_until_the_first_get() {
        let dir = TempDir::new("colt-lazy");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..10_000).map(|i| (i % 100, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), vec![vec![0], vec![1]]);
        let baseline = colt.watermark();
        assert_eq!(baseline, 1, "one root node, nothing else");
        // The first get forces exactly one level.
        let root = Colt::root();
        let child = colt.get(root, 0, &[7]).expect("key 7 exists");
        assert!(colt.watermark() > baseline);
        // The child is a real (chunked) node, still unforced.
        assert!(matches!(child, Cursor::Node(_)));
        assert!(matches!(colt.key_count(child), KeyCount::Estimate(100)));
    }

    #[test]
    fn suffix_iteration_never_forces() {
        let dir = TempDir::new("colt-suffix");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..500).map(|i| (i, i * 2)).collect();
        let view = view_of(&dir, &schema, &rows);
        // Single-level schema: the root's remaining schema is a suffix.
        let mut colt = Colt::new(all(&view), vec![vec![0, 1]]);
        let before = colt.watermark();
        let root = Colt::root();
        let entries = drain(&mut colt, root, 0);
        assert_eq!(entries.len(), 500);
        assert_eq!(colt.watermark(), before, "no forcing, no allocation");
        // Every child is a pinned row.
        assert!(entries.iter().all(|(_, c)| matches!(c, Cursor::Row(_))));
    }

    #[test]
    fn get_and_iter_agree_with_a_naive_oracle() {
        let dir = TempDir::new("colt-oracle");
        let schema = schema();
        // Duplicate-heavy: keys follow i % 17, some singleton keys on top.
        let mut rows: Vec<(u64, u64)> = (0..2_000u64).map(|i| (i % 17, i)).collect();
        rows.extend((100..110u64).map(|k| (k, k * 1000)));
        let view = view_of(&dir, &schema, &rows);
        let mut oracle: HashMap<u64, Vec<u64>> = HashMap::new();
        for (k, v) in &rows {
            oracle.entry(*k).or_default().push(*v);
        }

        let mut colt = Colt::new(all(&view), vec![vec![0], vec![1]]);
        let root = Colt::root();
        // Root iteration (non-suffix -> forces): keys match the oracle's.
        let entries = drain(&mut colt, root, 0);
        assert_eq!(entries.len(), oracle.len());
        assert!(matches!(
            colt.key_count(root),
            KeyCount::Exact(n) if n == oracle.len() as u64
        ));
        for (key, child) in entries {
            let expected = &oracle[&key[0]];
            // get() agrees with the iterated child.
            let got = colt.get(root, 0, &key).expect("iterated key resolves");
            assert_eq!(got, child);
            // Level-1 values match the oracle multiset.
            let mut values: Vec<u64> = drain(&mut colt, child, 1)
                .into_iter()
                .map(|(k, _)| k[0])
                .collect();
            values.sort_unstable();
            let mut want = expected.clone();
            want.sort_unstable();
            assert_eq!(values, want, "key {}", key[0]);
        }
        // Missing keys miss.
        assert_eq!(colt.get(root, 0, &[9999]), None);
    }

    #[test]
    fn chunked_lists_round_trip_far_beyond_one_chunk() {
        let dir = TempDir::new("colt-chunks");
        let schema = schema();
        // 300 duplicates of one key: 64-position chunks must chain.
        let rows: Vec<(u64, u64)> = (0..300).map(|i| (42, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), vec![vec![0], vec![1]]);
        let child = colt.get(Colt::root(), 0, &[42]).expect("hit");
        assert!(matches!(colt.key_count(child), KeyCount::Estimate(300)));
        let values = drain(&mut colt, child, 1);
        assert_eq!(values.len(), 300);
        let mut got: Vec<u64> = values.into_iter().map(|(k, _)| k[0]).collect();
        got.sort_unstable();
        assert_eq!(got, (0..300).collect::<Vec<u64>>());
    }

    #[test]
    fn singleton_keys_allocate_no_chunks() {
        let dir = TempDir::new("colt-singleton");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..100).map(|i| (i, i)).collect(); // all unique
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), vec![vec![0], vec![1]]);
        let child = colt.get(Colt::root(), 0, &[5]).expect("hit");
        // Singletons pin rows inline: no chunk, no extra node.
        assert!(matches!(child, Cursor::Row(_)));
        assert_eq!(colt.chunks.len(), 0);
    }

    #[test]
    fn key_count_labels_are_honest_in_both_states() {
        let dir = TempDir::new("colt-key-count");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..60).map(|i| (i % 3, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), vec![vec![0], vec![1]]);
        let root = Colt::root();
        // Unforced: duplicate-inflated Estimate.
        assert_eq!(colt.key_count(root), KeyCount::Estimate(60));
        colt.get(root, 0, &[0]);
        // Forced: exact distinct keys.
        assert_eq!(colt.key_count(root), KeyCount::Exact(3));
    }

    #[test]
    fn zero_arity_levels_gate_on_nonemptiness() {
        let dir = TempDir::new("colt-nullary");
        let schema = schema();
        let rows: Vec<(u64, u64)> = vec![(1, 2), (3, 4)];
        let view = view_of(&dir, &schema, &rows);
        // A zero-binding occurrence: one empty level.
        let mut colt = Colt::new(all(&view), vec![vec![]]);
        let root = Colt::root();
        let entries = drain(&mut colt, root, 0);
        // Suffix iteration yields one entry per position (empty keys);
        // a probe with the empty key forces and hits iff nonempty.
        assert_eq!(entries.len(), 2);
        let mut colt = Colt::new(all(&view), vec![vec![]]);
        assert!(colt.get(Colt::root(), 0, &[]).is_some());
    }
}
