//! COLT — the Column-Oriented Lazy Trie (docs/architecture/30-execution.md), per paper §4.2 with the
//! chunked-child-list deviation (`docs/architecture/30-execution.md`).
//!
//! Aliasing safety is representational: nodes, chunks, map slots, and key
//! words live in index-addressed pools (`NodeRef`-style u32 indices, never
//! pointers) — the fix for v5's `UnsafeCell` aliasing UB (post-mortem
//! §36). Since docs/perf/ PRD 04 the *bounds* checks on iteration
//! gathers are debug-asserted once per batch segment and elided in
//! release (`get_unchecked` per the 00-product unsafe policy: this
//! module is on the allowlist, and every unchecked read sits behind a
//! segment-level invariant stated at the site). Nothing is ever built
//! eagerly: a node is offsets into the base columns until a `get` (or a
//! non-suffix `iter`) forces exactly one level.
//!
//! Iteration is batched copy-out ([`Colt::iter_batch`]): entries are
//! `(key words, child cursor)` pairs — **the child comes with the key**;
//! re-probing the map just enumerated is inexpressible through this API
//! (post-mortem §34).
//!
//! The probe path is `#[inline(always)]` end to end (docs/silicon/02):
//! an L2-resident probe stream's surviving cost class is instructions
//! retired per probe, and call ceremony was first on the bill. The
//! lint's "usually a bad idea" is measured wrong here, and the inlining
//! is machine-checked by `scripts/check-asm.sh`, not trusted to the
//! attribute.
#![allow(clippy::inline_always)]

use crate::image::view::View;
use crate::image::ColumnView;

/// Positions per chunk: bounded pointer-chase, independent loads within a
/// chunk (the deviation from the paper's growable per-key vectors).
const CHUNK_LEN: usize = 64;

/// Labeled key count. The label records *what kind* of number this is —
/// `Exact` counts a forced map's distinct keys; `Estimate` counts an
/// unforced vector's positions, an **upper bound** on its distinct keys
/// (duplicate-inflated) and simultaneously the exact cost of iterating
/// it unforced. Both are admissible iteration-cost bounds, so cover
/// choice compares magnitudes first and uses the label only to break
/// ties (docs/architecture/30-execution.md) — label-first preference is exactly the bug that
/// iterated a 500-key forced map instead of a 7-row view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCount {
    /// Distinct keys of a forced map.
    Exact(u64),
    /// Position count of an unforced vector (duplicate-inflated).
    Estimate(u64),
}

impl KeyCount {
    /// The iteration-cost bound the label qualifies.
    #[must_use]
    pub fn magnitude(self) -> u64 {
        match self {
            Self::Exact(n) | Self::Estimate(n) => n,
        }
    }
}

/// A reference into the trie: either a real node or a single image
/// position pinned by a singleton child (no node is allocated for it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Node(NodeRef),
    Row(u32),
}

/// One position run under an unforced suffix node (docs/perf/ PRD 05):
/// either the all-rows identity range (positions are the indices) or a
/// borrowed position slice (survivor roots, chunk-chain segments).
#[derive(Debug, Clone, Copy)]
pub enum SuffixRun<'a> {
    Identity { start: usize, len: usize },
    Positions(&'a [u32]),
}

impl SuffixRun<'_> {
    /// Positions in this run.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Identity { len, .. } => *len,
            Self::Positions(p) => p.len(),
        }
    }

    /// Whether the run is empty (clippy's `len` companion; the executor
    /// counts, sinks fold — nothing branches on emptiness yet).
    #[must_use]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Index of a node in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRef(u32);

/// Opaque resume token for [`Colt::iter_batch`]; start at `default()`.
///
/// Bit 63 tags every nonzero token with the node state that minted it
/// (clear = positions iteration, set = forced-map iteration), so a token
/// that outlives a force of its node is caught by a release assert
/// instead of being silently reinterpreted as a dense index — the
/// silent-omission wrong-results class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BatchToken(u64);

/// The [`BatchToken`] state tag: set on forced-map (dense-index) tokens,
/// clear on positions tokens. Positions tokens cannot collide with it:
/// the root form is a view index (≤ u32 space) and the chunked form
/// packs `(chunk + 2) << 32 | offset`, which reaches bit 63 only past
/// 2³⁰ chunks (≈2³⁶ positions) — beyond the u32 position space itself;
/// debug-checked at the mint site.
const DENSE_TOKEN_TAG: u64 = 1 << 63;

/// The token-kind mismatch message: fired when a resume token minted
/// under one node state is presented after that state changed.
const STALE_TOKEN: &str = "iteration token outlived a force — drain before probing this cursor";

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

/// A decoded occupied-slot child (PRD 07: emptiness lives in the ctrl
/// bytes; a bucket's packed child word is always one of these).
#[derive(Debug, Clone, Copy)]
enum Slot {
    /// Singleton optimization: the first position lives inline; a chunked
    /// node is allocated only on the second.
    Single(u32),
    Node(NodeRef),
}

/// One forced level's open-addressed map: power-of-two capacity, linear
/// probing, no tombstones (build-once, never deleted from). Capacity
/// starts from the position-count guess and rehash-doubles at 75% load
/// (docs/architecture/30-execution.md); iteration never touches the slot
/// array — it walks the dense occupied list.
///
/// Layout (docs/perf/ PRD 07): a ctrl byte per slot (0 = empty, else
/// `0x80 | top-7-hash-bits`) plus one interleaved bucket row of
/// `arity + 1` words — key words then the packed child — so a probe
/// step reads the ctrl line and, on a tag match, ONE bucket line.
#[derive(Debug, Clone, Copy)]
struct Map {
    arity: usize,
    capacity: usize,
    len: u32,
    /// Start of this map's ctrl range in the shared ctrl slab.
    ctrl_start: usize,
    /// Start of this map's bucket rows (`capacity * (arity + 1)` words).
    bucket_start: usize,
    /// Start of this map's occupied-slot list in the dense slab —
    /// `len` entries, insertion-ordered, O(keys) to walk.
    dense_start: usize,
}

impl Map {
    /// Words per bucket row: the key then the packed child.
    fn stride(&self) -> usize {
        self.arity + 1
    }
}

/// The packed child word's node tag (bit 63): set = `NodeRef` index,
/// clear = a single pinned position. Both payloads are u32.
const CHILD_NODE_TAG: u64 = 1 << 63;

/// Packs a slot child into its bucket word.
fn pack_child(slot: Slot) -> u64 {
    match slot {
        Slot::Single(position) => u64::from(position),
        Slot::Node(node) => CHILD_NODE_TAG | u64::from(node.0),
    }
}

/// Unpacks a bucket child word (the slot is occupied by ctrl).
#[inline(always)]
fn unpack_child(word: u64) -> Slot {
    if word & CHILD_NODE_TAG == 0 {
        Slot::Single(u32::try_from(word).expect("positions fit u32"))
    } else {
        Slot::Node(NodeRef(
            u32::try_from(word & !CHILD_NODE_TAG).expect("node refs fit u32"),
        ))
    }
}

/// The 7-bit hash tag a ctrl byte carries (bit 7 marks occupancy).
#[inline(always)]
fn ctrl_tag(hash: u64) -> u8 {
    0x80 | u8::try_from(hash >> 57).expect("7 bits")
}

/// The lazy trie over one occurrence's view. Owns the view (a cheap
/// enum over an `Arc`'d image plus survivor positions) and its pools, so a
/// prepared query can hold and [`Colt::reset`] it across executions with
/// every capacity retained (the 30-execution doc's zero-alloc discipline).
pub struct Colt {
    view: View,
    /// Prepended selection levels (docs/architecture/30-execution.md): one single-column trie
    /// level per Eq-constant, probed once per execution with the resolved
    /// words. Everything below a successful probe is exactly the filtered
    /// subtrie a view scan used to produce — built lazily, only for keys
    /// actually asked about.
    selection_levels: usize,
    /// The post-selection start cursor for the current execution.
    start: Cursor,
    /// Whether [`Colt::select`] ran since the last reset (always true for
    /// selection-free tries).
    selected: bool,
    /// Per trie level — selection levels first, then join levels — the
    /// image column index of each key variable. Public APIs take *join*
    /// levels; internal code indexes this directly.
    schema_columns: Vec<Vec<usize>>,
    nodes: Vec<NodeState>,
    chunks: Vec<Chunk>,
    maps: Vec<Map>,
    /// Ctrl bytes for every forced map, range per map (PRD 07).
    ctrl: Vec<u8>,
    /// Interleaved bucket rows for every forced map, range per map.
    buckets: Vec<u64>,
    /// The dense occupied-slot lists, one contiguous range per map
    /// (docs/architecture/30-execution.md). A rehash abandons its old range at the slab's
    /// interior — reclaimed by [`Colt::reset`], a documented ≤2× slab
    /// transient within a generation.
    dense: Vec<u32>,
    /// Reused key-decoding scratch.
    scratch: Vec<u64>,
}

/// The probe hash for a key — exposed so the vectorized executor's phase 1
/// can compute all hashes (pure ALU) before phase 2 issues any bucket load
/// (D4's two-phase probing, the 30-execution doc).
#[must_use]
#[inline(always)]
pub fn hash_key(words: &[u64]) -> u64 {
    hash_words(words)
}

#[inline(always)]
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
    /// `selections` are the image columns of the occurrence's Eq-constant
    /// predicates, in plan order; `join_schema` the join levels below them.
    #[must_use]
    pub fn new(view: View, selections: &[usize], join_schema: Vec<Vec<usize>>) -> Self {
        let selection_levels = selections.len();
        let schema_columns: Vec<Vec<usize>> = selections
            .iter()
            .map(|column| vec![*column])
            .chain(join_schema)
            .collect();
        Self {
            view,
            selection_levels,
            start: Cursor::Node(NodeRef(0)),
            selected: selection_levels == 0,
            schema_columns,
            nodes: vec![NodeState::Unforced(Positions::Root)],
            chunks: Vec::new(),
            maps: Vec::new(),
            ctrl: Vec::new(),
            buckets: Vec::new(),
            dense: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// A structurally identical trie with empty pools over no view — the
    /// shape without the data (reader: the view memo's first park of an
    /// empty slot, inside the sanctioned view-rebuild window).
    #[must_use]
    pub fn unbound_sibling(&self) -> Self {
        Self {
            view: View::Unbound,
            selection_levels: self.selection_levels,
            start: Cursor::Node(NodeRef(0)),
            selected: self.selection_levels == 0,
            schema_columns: self.schema_columns.clone(),
            nodes: vec![NodeState::Unforced(Positions::Root)],
            chunks: Vec::new(),
            maps: Vec::new(),
            ctrl: Vec::new(),
            buckets: Vec::new(),
            dense: Vec::new(),
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
        self.ctrl.clear();
        self.buckets.clear();
        self.dense.clear();
        self.start = Cursor::Node(NodeRef(0));
        self.selected = self.selection_levels == 0;
        old
    }

    /// Probes the selection levels with this execution's resolved words,
    /// in level order, forcing lazily exactly like join-level probes.
    /// The amortization contract: forcing selection level 0 walks the
    /// view once per generation; every subsequent param value is O(1)
    /// probes. `Some` sits at the first join level; `None` = no fact
    /// matches — the occurrence, and with it the whole conjunctive
    /// query, is empty on this snapshot.
    pub fn select(&mut self, keys: &[u64]) -> Option<Cursor> {
        debug_assert_eq!(
            keys.len(),
            self.selection_levels,
            "one resolved word per selection level"
        );
        let mut cursor = Self::root();
        for (level, key) in keys.iter().enumerate() {
            let key = std::slice::from_ref(key);
            cursor = self.probe_child_at(cursor, level, key, hash_words(key))?;
        }
        self.start = cursor;
        self.selected = true;
        Some(cursor)
    }

    /// The executor's per-execution start cursor: the root, or the
    /// post-selection cursor once [`Colt::select`] ran this execution.
    ///
    /// # Panics
    ///
    /// A release assert: starting a selection-bearing colt before
    /// `select()` would silently drop its selections — wrong results.
    /// Once per occurrence per execution; noise against the join.
    #[must_use]
    pub fn start(&self) -> Cursor {
        assert!(self.selected, "select() runs before the join");
        self.start
    }

    /// The root cursor (level 0).
    #[must_use]
    pub fn root() -> Cursor {
        Cursor::Node(NodeRef(0))
    }

    /// Key arity at a *join* level (public APIs speak join levels; the
    /// selection prefix is internal). Production callers derive arity
    /// from the plan; this is the test-facing accessor.
    #[cfg(test)]
    #[must_use]
    pub fn arity(&self, level: usize) -> usize {
        self.arity_at(self.selection_levels + level)
    }

    /// Key arity at an internal (selection-inclusive) level.
    fn arity_at(&self, level: usize) -> usize {
        self.schema_columns[level].len()
    }

    /// A forced node's map capacity (`None` when unforced) — the test
    /// observability for the sizing formula (docs/architecture/30-execution.md).
    #[cfg(test)]
    #[must_use]
    pub fn forced_capacity(&self, cursor: Cursor) -> Option<usize> {
        match cursor {
            Cursor::Row(_) => None,
            Cursor::Node(node) => match self.nodes[node.0 as usize] {
                NodeState::Forced { map } => Some(self.maps[map as usize].capacity),
                NodeState::Unforced(_) => None,
            },
        }
    }

    /// Total pool footprint — the test observability for laziness
    /// (allocations only ever grow this).
    #[cfg(test)]
    #[must_use]
    pub fn watermark(&self) -> usize {
        self.nodes.len()
            + self.chunks.len()
            + self.maps.len()
            + self.ctrl.len()
            + self.buckets.len()
            + self.dense.len()
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
    #[inline(always)]
    fn word_at(&self, column: usize, position: u32) -> u64 {
        match self.view.image().column(column) {
            ColumnView::Words(words) => words[position as usize],
            ColumnView::Bytes(bytes) => u64::from(bytes[position as usize]),
        }
    }

    /// Whether the position's key words at `level` equal `key`.
    #[inline(always)]
    fn position_matches(&self, level: usize, position: u32, key: &[u64]) -> bool {
        // The zip truncates to the shorter side — correct only when the
        // arities agree, so the invariant is asserted where the
        // truncation lives.
        debug_assert_eq!(key.len(), self.schema_columns[level].len());
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
    /// `level` is a join level.
    ///
    /// Inlined into the executor's probe loops (docs/silicon/02): an
    /// L2-resident probe stream's surviving cost class is instructions
    /// retired per probe — call ceremony here was first on the bill.
    #[inline(always)]
    pub fn get_prehashed(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Option<Cursor> {
        self.probe_child_at(cursor, self.selection_levels + level, key, hash)
    }

    /// [`Colt::get_prehashed`] over an internal (selection-inclusive)
    /// level — the shared body selection probes also walk.
    #[inline(always)]
    fn probe_child_at(
        &mut self,
        cursor: Cursor,
        level: usize,
        key: &[u64],
        hash: u64,
    ) -> Option<Cursor> {
        debug_assert_eq!(key.len(), self.arity_at(level));
        match cursor {
            // A pinned row: the probe is a field-equality check, and the
            // child stays pinned to the same position.
            Cursor::Row(position) => self
                .position_matches(level, position, key)
                .then_some(Cursor::Row(position)),
            Cursor::Node(node) => {
                let map = self.force(node, level);
                // By reference (docs/silicon/02): `Map` is a 48-byte
                // Copy struct — a by-value bind here was one stack copy
                // per probe, a first-class suspect in the emulation that
                // reproduced the 55–60 ns plateau.
                let m = &self.maps[map as usize];
                let (found, idx) = self.probe_hashed(m, key, hash);
                if !found {
                    return None;
                }
                match unpack_child(self.buckets[m.bucket_start + idx * m.stride() + m.arity]) {
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
            self.force(node, self.selection_levels + level);
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
    /// Gathers one pinned row's key words at a join level into `out`
    /// (docs/perf/ PRD 05's pinned-leaf elision: the executor skips the
    /// batch machinery for `Cursor::Row` leaves and reads the row
    /// directly).
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: `out` shorter than the
    /// level's arity.
    pub fn gather_row(&self, level: usize, position: u32, out: &mut [u64]) {
        let level = self.selection_levels + level;
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            out[i] = match self.view.image().column(*col) {
                ColumnView::Words(words) => words[position as usize],
                ColumnView::Bytes(bytes) => u64::from(bytes[position as usize]),
            };
        }
    }

    /// The column view backing one key word of a join level — the
    /// scan-fold pushdown reads columns directly instead of copying key
    /// batches (docs/perf/ PRD 05).
    #[must_use]
    pub fn suffix_column(&self, level: usize, word: usize) -> ColumnView<'_> {
        self.view
            .image()
            .column(self.schema_columns[self.selection_levels + level][word])
    }

    /// Whether a cursor is an unforced node at a suffix — the scan-fold
    /// pushdown's cheap pre-check (docs/perf/ PRD 05), so a fallback to
    /// the batch path never has to unwind a half-opened scan.
    #[must_use]
    pub fn suffix_scannable(&self, cursor: Cursor) -> bool {
        matches!(
            cursor,
            Cursor::Node(node)
                if matches!(self.nodes[node.0 as usize], NodeState::Unforced(_))
        )
    }

    /// Drives `f` over every position run under an **unforced** node at
    /// the given join level (the scan-fold pushdown's position source):
    /// the all-rows root yields one `Identity` run, survivor roots and
    /// chunk chains yield position slices. Returns `false` — with `f`
    /// never called — when the cursor is a pinned row or a forced node
    /// (the caller falls back to the batch path).
    pub fn for_each_suffix_run(&self, cursor: Cursor, mut f: impl FnMut(SuffixRun<'_>)) -> bool {
        let Cursor::Node(node) = cursor else {
            return false;
        };
        match self.nodes[node.0 as usize] {
            NodeState::Forced { .. } => false,
            NodeState::Unforced(Positions::Root) => {
                if self.view.is_empty() {
                    return true;
                }
                match &self.view {
                    View::Survivors { positions, .. } => f(SuffixRun::Positions(positions)),
                    _ => f(SuffixRun::Identity {
                        start: 0,
                        len: self.view.len(),
                    }),
                }
                true
            }
            NodeState::Unforced(Positions::Chunks { first, .. }) => {
                let mut chunk = first;
                while chunk != u32::MAX {
                    let c = &self.chunks[chunk as usize];
                    if c.next != u32::MAX {
                        crate::exec::kernel::prefetch_read(&raw const self.chunks[c.next as usize]);
                    }
                    f(SuffixRun::Positions(&c.positions[..usize::from(c.len)]));
                    chunk = c.next;
                }
                true
            }
        }
    }

    pub fn iter_batch(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        self.iter_batch_at(
            cursor,
            self.selection_levels + level,
            token,
            keys_out,
            children_out,
            max,
        )
    }

    /// [`Colt::iter_batch`] over an internal (selection-inclusive) level.
    fn iter_batch_at(
        &mut self,
        cursor: Cursor,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        let arity = self.arity_at(level);
        assert!(keys_out.len() >= max * arity && children_out.len() >= max);
        match cursor {
            Cursor::Row(position) => {
                // `max == 0` yields nothing — the same contract every
                // other arm honors (an over-yield here both violated the
                // contract and wrote past a zero-sized buffer).
                if token.0 > 0 || max == 0 {
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
    ///
    /// Gathers are column-hoisted and unchecked (docs/perf/ PRD 04): each
    /// key column resolves its slice once per segment, positions are
    /// debug-asserted in-bounds once, and the interior runs bare loads —
    /// ~1 load per (position, column) instead of an enum match and two
    /// bounds checks each.
    fn iter_positions(
        &mut self,
        node: NodeRef,
        level: usize,
        token: BatchToken,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        max: usize,
    ) -> (usize, BatchToken) {
        // A dense-tagged token here means the node was un-forced under an
        // outstanding iteration — impossible within a generation; a stale
        // token from before a reset lands here too.
        assert!(token.0 & DENSE_TOKEN_TAG == 0, "{STALE_TOKEN}");
        match self.nodes[node.0 as usize] {
            NodeState::Forced { .. } => unreachable!("caller checked unforced"),
            NodeState::Unforced(Positions::Root) => {
                let index = usize::try_from(token.0).expect("64-bit usize");
                let take = max.min(self.view.len().saturating_sub(index));
                if take == 0 {
                    return (0, token);
                }
                match &self.view {
                    View::Survivors { positions, .. } => {
                        let segment = &positions[index..index + take];
                        self.gather_segment(level, segment, keys_out, children_out, 0);
                    }
                    // The all-rows view: positions ARE the indices — the
                    // fully contiguous gather, no position loads at all.
                    _ => self.gather_identity(level, index, take, keys_out, children_out),
                }
                (take, BatchToken((index + take) as u64))
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
                let mut yielded = 0;
                loop {
                    if yielded >= max {
                        break;
                    }
                    let c = &self.chunks[chunk as usize];
                    let len = usize::from(c.len);
                    if offset >= len {
                        if c.next == u32::MAX {
                            return (yielded, BatchToken(EXHAUSTED));
                        }
                        chunk = c.next;
                        offset = 0;
                        continue;
                    }
                    // One chunk ahead: the chain walk is this loop's only
                    // dependent-load sequence.
                    if c.next != u32::MAX {
                        crate::exec::kernel::prefetch_read(&raw const self.chunks[c.next as usize]);
                    }
                    let take = (len - offset).min(max - yielded);
                    let segment = &c.positions[offset..offset + take];
                    self.gather_segment(level, segment, keys_out, children_out, yielded);
                    yielded += take;
                    offset += take;
                }
                let packed = (u64::from(chunk) + 2) << 32 | offset as u64;
                // Bit 63 (the dense tag) is unreachable below 2³⁰ chunks
                // — the scale axiom sits orders of magnitude under it,
                // and the u32 chunk space itself wraps first.
                debug_assert_eq!(packed & DENSE_TOKEN_TAG, 0);
                (yielded, BatchToken(packed))
            }
        }
    }

    /// Column-hoisted gather of one position segment into
    /// `keys_out[out_base..]` + pinned-row children (PRD 04's interior).
    #[allow(unsafe_code)]
    fn gather_segment(
        &self,
        level: usize,
        segment: &[u32],
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
        out_base: usize,
    ) {
        let arity = self.arity_at(level);
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            match self.view.image().column(*col) {
                ColumnView::Words(words) => {
                    debug_assert!(segment.iter().all(|&p| (p as usize) < words.len()));
                    for (k, &position) in segment.iter().enumerate() {
                        // SAFETY: positions index the image the view was
                        // built over — debug-asserted per segment above.
                        let word = unsafe { *words.get_unchecked(position as usize) };
                        keys_out[(out_base + k) * arity + i] = word;
                    }
                }
                ColumnView::Bytes(bytes) => {
                    debug_assert!(segment.iter().all(|&p| (p as usize) < bytes.len()));
                    for (k, &position) in segment.iter().enumerate() {
                        // SAFETY: as above.
                        let byte = unsafe { *bytes.get_unchecked(position as usize) };
                        keys_out[(out_base + k) * arity + i] = u64::from(byte);
                    }
                }
            }
        }
        for (k, &position) in segment.iter().enumerate() {
            children_out[out_base + k] = Cursor::Row(position);
        }
    }

    /// The all-rows-view gather: positions are `start..start + take`, so
    /// word columns copy contiguously — no position loads at all.
    fn gather_identity(
        &self,
        level: usize,
        start: usize,
        take: usize,
        keys_out: &mut [u64],
        children_out: &mut [Cursor],
    ) {
        let arity = self.arity_at(level);
        for (i, col) in self.schema_columns[level].iter().enumerate() {
            match self.view.image().column(*col) {
                ColumnView::Words(words) => {
                    let src = &words[start..start + take];
                    if arity == 1 {
                        keys_out[..take].copy_from_slice(src);
                    } else {
                        for (k, &word) in src.iter().enumerate() {
                            keys_out[k * arity + i] = word;
                        }
                    }
                }
                ColumnView::Bytes(bytes) => {
                    let src = &bytes[start..start + take];
                    for (k, &byte) in src.iter().enumerate() {
                        keys_out[k * arity + i] = u64::from(byte);
                    }
                }
            }
        }
        for (k, position) in (start..start + take).enumerate() {
            children_out[k] = Cursor::Row(u32::try_from(position).expect("positions fit u32"));
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
        let arity = self.arity_at(level);
        debug_assert_eq!(arity, m.arity);
        // Walk the dense occupied list — O(keys), never O(capacity)
        // (docs/architecture/30-execution.md). The token is a tagged
        // dense index: an untagged nonzero token was minted by positions
        // iteration before this node was forced — reinterpreting it as a
        // dense index would silently omit entries (the audit's
        // wrong-results scenario). Once per batch: noise.
        assert!(
            token.0 == 0 || token.0 & DENSE_TOKEN_TAG != 0,
            "{STALE_TOKEN}"
        );
        let start = usize::try_from(token.0 & !DENSE_TOKEN_TAG).expect("64-bit usize");
        let len = usize::try_from(m.len).expect("64-bit usize");
        let take = max.min(len.saturating_sub(start));
        // Hoisted slices (docs/perf/ PRD 04): the dense walk touches the
        // occupied list, the key slab, and the slot array — resolved once,
        // with the key line prefetched a few entries ahead (insertion
        // order scatters slots across the map).
        let dense = &self.dense[m.dense_start..m.dense_start + len];
        let stride = m.stride();
        for k in 0..take {
            let dense_idx = start + k;
            if dense_idx + 8 < len {
                let ahead = usize::try_from(dense[dense_idx + 8]).expect("64-bit usize");
                crate::exec::kernel::prefetch_read(
                    &raw const self.buckets[m.bucket_start + ahead * stride],
                );
            }
            let slot_idx = usize::try_from(dense[dense_idx]).expect("64-bit usize");
            let row = &self.buckets[m.bucket_start + slot_idx * stride..];
            keys_out[k * arity..k * arity + arity].copy_from_slice(&row[..arity]);
            children_out[k] = match unpack_child(row[arity]) {
                Slot::Single(position) => Cursor::Row(position),
                Slot::Node(child) => Cursor::Node(child),
            };
        }
        (take, BatchToken((start + take) as u64 | DENSE_TOKEN_TAG))
    }

    /// Linear probe with a precomputed hash: the ctrl byte gates the
    /// bucket read — a mismatched tag steps without touching the bucket
    /// line (docs/perf/ PRD 07). Arity-monomorphic (docs/silicon/02):
    /// the dispatch happens once per probe, and each walk loop's key
    /// compare is straight-line word compares — a runtime-length slice
    /// equality here compiled to a `bcmp` call per tag match.
    #[inline(always)]
    fn probe_hashed(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        match key.len() {
            1 => self.probe_walk::<1>(m, key, hash),
            2 => self.probe_walk::<2>(m, key, hash),
            3 => self.probe_walk::<3>(m, key, hash),
            4 => self.probe_walk::<4>(m, key, hash),
            _ => self.probe_walk_general(m, key, hash),
        }
    }

    /// The monomorphic walk: `A` is the key arity, so the compare unrolls
    /// to `A` word compares with no call and no length test.
    #[inline(always)]
    fn probe_walk<const A: usize>(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        debug_assert_eq!(key.len(), A);
        debug_assert_eq!(m.arity, A);
        let mask = m.capacity - 1;
        let wanted = ctrl_tag(hash);
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {
            let c = self.ctrl[m.ctrl_start + idx];
            if c == 0 {
                return (false, idx);
            }
            if c == wanted {
                let bucket = m.bucket_start + idx * (A + 1);
                let stored = &self.buckets[bucket..bucket + A];
                let mut matches = true;
                for i in 0..A {
                    matches &= stored[i] == key[i];
                }
                if matches {
                    return (true, idx);
                }
            }
            idx = (idx + 1) & mask;
        }
    }

    /// The rare wide-key fallback (arity > 4 — beyond every bench plan).
    fn probe_walk_general(&self, m: &Map, key: &[u64], hash: u64) -> (bool, usize) {
        let mask = m.capacity - 1;
        let wanted = ctrl_tag(hash);
        let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
        loop {
            let c = self.ctrl[m.ctrl_start + idx];
            if c == 0 {
                return (false, idx);
            }
            if c == wanted {
                let stored = &self.buckets[m.bucket_start + idx * m.stride()..];
                if &stored[..m.arity] == key {
                    return (true, idx);
                }
            }
            idx = (idx + 1) & mask;
        }
    }

    /// Prefetches the bucket a hash will probe (phase 1.5, docs/perf/
    /// PRD 07): the ctrl byte's line and the bucket row's line. A no-op
    /// for pinned rows and unforced nodes.
    #[inline(always)]
    pub fn prefetch_bucket(&self, cursor: Cursor, hash: u64) {
        let Cursor::Node(node) = cursor else { return };
        let NodeState::Forced { map } = self.nodes[node.0 as usize] else {
            return;
        };
        let m = &self.maps[map as usize];
        let idx = usize::try_from(hash).expect("64-bit usize") & (m.capacity - 1);
        crate::exec::kernel::prefetch_read(&raw const self.ctrl[m.ctrl_start + idx]);
        crate::exec::kernel::prefetch_read(
            &raw const self.buckets[m.bucket_start + idx * m.stride()],
        );
    }

    /// Single-pass force: iterate the node's positions once, decoding key
    /// words and appending each position to its key's chunked child list.
    /// Returns the map index (idempotent).
    fn force(&mut self, node: NodeRef, level: usize) -> u32 {
        if let NodeState::Forced { map } = self.nodes[node.0 as usize] {
            return map;
        }
        let arity = self.arity_at(level);
        let count = match self.nodes[node.0 as usize] {
            NodeState::Unforced(Positions::Root) => self.view.len() as u64,
            NodeState::Unforced(Positions::Chunks { count, .. }) => u64::from(count),
            NodeState::Forced { .. } => unreachable!("checked above"),
        };
        // Initial capacity (docs/architecture/30-execution.md): distinct keys are unknown
        // before the pass, so start from the deterministic guess
        // `next_pow2(clamp(count/8, 16, 2*count))` — tiny nodes keep
        // their old tight sizing, big skewed levels start 16x smaller
        // than the old 2x-positions rule — and rehash-double at 75%
        // load when the guess was short.
        let count_usize = usize::try_from(count).expect("64-bit usize");
        let capacity = (count_usize / 8)
            .max(16)
            .min(count_usize.max(1) * 2)
            .next_power_of_two();
        let map_idx = u32::try_from(self.maps.len()).expect("map count fits u32");
        let ctrl_start = self.ctrl.len();
        let bucket_start = self.buckets.len();
        let dense_start = self.dense.len();
        self.ctrl.resize(ctrl_start + capacity, 0);
        self.buckets
            .resize(bucket_start + capacity * (arity + 1), 0);
        let mut m = Map {
            arity,
            capacity,
            len: 0,
            ctrl_start,
            bucket_start,
            dense_start,
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
    /// and land it (new slot or appended child), rehash-doubling first
    /// when the next insert would cross 75% load.
    fn force_ingest(&mut self, m: &mut Map, level: usize, position: u32) {
        // Growth is checked before the probe, so a position that merely
        // appends to an existing key can still trigger a double — an
        // over-size by at most one doubling step, closed by audit as
        // no-action: checking after the probe would probe the old table
        // and insert into the new one.
        if (usize::try_from(m.len).expect("64-bit usize") + 1) * 4 >= m.capacity * 3 {
            self.grow_map(m);
        }
        let arity = m.arity;
        self.scratch.clear();
        for col in &self.schema_columns[level] {
            let w = self.word_at(*col, position);
            self.scratch.push(w);
        }
        let key = std::mem::take(&mut self.scratch);
        let hash = hash_words(&key);
        let (found, idx) = self.probe_hashed(m, &key, hash);
        let row_at = m.bucket_start + idx * m.stride();
        if found {
            self.append_child(row_at + arity, position);
        } else {
            self.ctrl[m.ctrl_start + idx] = ctrl_tag(hash);
            self.buckets[row_at..row_at + arity].copy_from_slice(&key);
            self.buckets[row_at + arity] = pack_child(Slot::Single(position));
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
            m.len += 1;
        }
        self.scratch = key;
    }

    /// Rehash-doubles a map mid-force: fresh slot/key/dense ranges at
    /// the slab tails (the old ranges are abandoned until `reset` — the
    /// documented ≤2× transient), keys re-probed in dense (insertion)
    /// order so iteration order survives growth. All keys are distinct
    /// by construction, so the re-probe never compares keys — it takes
    /// the first empty slot.
    fn grow_map(&mut self, m: &mut Map) {
        let arity = m.arity;
        let stride = m.stride();
        let new_capacity = m.capacity * 2;
        let ctrl_start = self.ctrl.len();
        let bucket_start = self.buckets.len();
        let dense_start = self.dense.len();
        self.ctrl.resize(ctrl_start + new_capacity, 0);
        self.buckets.resize(bucket_start + new_capacity * stride, 0);
        let mask = new_capacity - 1;
        for i in 0..usize::try_from(m.len).expect("64-bit usize") {
            let old_slot = usize::try_from(self.dense[m.dense_start + i]).expect("64-bit usize");
            let old_row_at = m.bucket_start + old_slot * stride;
            let hash = hash_words(&self.buckets[old_row_at..old_row_at + arity]);
            let mut idx = usize::try_from(hash).expect("64-bit usize") & mask;
            while self.ctrl[ctrl_start + idx] != 0 {
                idx = (idx + 1) & mask;
            }
            self.ctrl[ctrl_start + idx] = ctrl_tag(hash);
            self.buckets
                .copy_within(old_row_at..old_row_at + stride, bucket_start + idx * stride);
            self.dense
                .push(u32::try_from(idx).expect("slot index fits u32"));
        }
        m.capacity = new_capacity;
        m.ctrl_start = ctrl_start;
        m.bucket_start = bucket_start;
        m.dense_start = dense_start;
    }

    /// Appends a position to an occupied slot's child: singleton inline
    /// first, a chunked node from the second position on. `child_at`
    /// indexes the bucket slab's packed child word.
    fn append_child(&mut self, child_at: usize, position: u32) {
        match unpack_child(self.buckets[child_at]) {
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
                self.buckets[child_at] = pack_child(Slot::Node(node_ref));
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

    /// PRD 07 (docs/perf/): the ctrl-gated bucket probe is behavior-
    /// identical to a model across adversarial keys (equal low bits —
    /// same slot, different tags), probe hits AND misses, singleton
    /// upgrades, and growth across the 75% boundary.
    #[test]
    fn bucket_probes_match_the_model_under_adversarial_keys() {
        let dir = TempDir::new("colt-bucket-model");
        let schema = schema();
        // Keys collide mod any small capacity (equal low 8 bits) and
        // repeat (singleton -> chunk upgrades); enough distinct keys to
        // force several rehash doubles from the /8 initial guess.
        let mut rows: Vec<(u64, u64)> = Vec::new();
        for i in 0..400u64 {
            let key = (i % 97) << 8;
            rows.push((key, i));
        }
        rows.sort_unstable();
        rows.dedup();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let root = Colt::root();
        colt.ensure_forced(root, 0);

        // Model: key -> positions (image order).
        let k_col: Vec<u64> = view.column_words(0).to_vec();
        let mut model: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
        for (pos, k) in k_col.iter().enumerate() {
            model
                .entry(*k)
                .or_default()
                .push(u32::try_from(pos).expect("small"));
        }
        for (key, positions) in &model {
            let child = colt.get(root, 0, &[*key]).expect("present key probes");
            let got: Vec<u32> = drain(&mut colt, child, 1)
                .into_iter()
                .map(|(_, c)| match c {
                    Cursor::Row(p) => p,
                    Cursor::Node(_) => unreachable!("suffix children pin rows"),
                })
                .collect();
            assert_eq!(&got, positions, "key {key}");
        }
        // Misses: same low bits as present keys, absent values.
        for i in 0..97u64 {
            let absent = (i << 8) | 1;
            assert!(colt.get(root, 0, &[absent]).is_none(), "key {absent}");
        }
    }

    /// PRD 04 (docs/perf/): the column-hoisted unchecked gathers are
    /// bit-identical to a first-principles per-position reference, across
    /// word and byte columns, the identity (all-rows) root, chunked
    /// children, and resume-token splits at every batch size.
    #[test]
    #[allow(clippy::too_many_lines)] // one fixture, five batch sizes, two node shapes
    fn hoisted_gathers_match_the_per_position_reference() {
        let dir = TempDir::new("colt-hoisted-gather");
        // R(k u64, v u64, b bool): a byte-backed column beside the words.
        let schema = SchemaDescriptor {
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
                    FieldDescriptor {
                        name: "b".into(),
                        value_type: ValueType::Bool,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture");

        // Skewed keys force multi-chunk children (k=0 holds >64 rows).
        let mut rows: Vec<(u64, u64, bool)> = (0..200u64)
            .map(|i| (if i % 3 == 0 { 0 } else { i % 7 }, i * 31 % 191, i % 2 == 0))
            .collect();
        rows.sort_unstable();
        rows.dedup();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let txn0 = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        for (k, v, b) in &rows {
            let mut bytes = Vec::new();
            encode_fact(
                &[ValueRef::U64(*k), ValueRef::U64(*v), ValueRef::Bool(*b)],
                schema.relation(R).layout(),
                &mut bytes,
            );
            delta.insert(&txn0, R, &bytes).expect("insert");
        }
        drop(txn0);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        let image = crate::image::build(&txn, &schema, R).expect("build");
        // The reference reads the image columns per position — the exact
        // access the hoisted gather replaces; no assumption about the
        // image's position order.
        let k_col: Vec<u64> = image.column_words(0).to_vec();
        let v_col: Vec<u64> = image.column_words(1).to_vec();
        let b_col: Vec<u64> = image
            .column_bytes(2)
            .iter()
            .map(|&b| u64::from(b))
            .collect();
        let n_rows = k_col.len();
        assert_eq!(n_rows, rows.len());

        let drain_at = |colt: &mut Colt, cursor: Cursor, level: usize, size: usize| {
            let arity = colt.arity(level);
            let mut keys = vec![0u64; size * arity.max(1)];
            let mut children = vec![Cursor::Row(0); size];
            let mut token = BatchToken::default();
            let mut out = Vec::new();
            loop {
                let (n, next) =
                    colt.iter_batch(cursor, level, token, &mut keys, &mut children, size);
                if n == 0 {
                    break;
                }
                for i in 0..n {
                    out.push((keys[i * arity..(i + 1) * arity].to_vec(), children[i]));
                }
                token = next;
            }
            out
        };

        for &size in &[1usize, 3, 8, 64, 128] {
            // Identity root suffix over (k, b): word + byte columns.
            let mut colt = Colt::new(apply(&image, &[], &[], Vec::new()), &[], vec![vec![0, 2]]);
            let got = drain_at(&mut colt, Colt::root(), 0, size);
            let expected: Vec<(Vec<u64>, Cursor)> = (0..n_rows)
                .map(|pos| {
                    (
                        vec![k_col[pos], b_col[pos]],
                        Cursor::Row(u32::try_from(pos).expect("small")),
                    )
                })
                .collect();
            assert_eq!(got, expected, "identity root, batch {size}");

            // Chunked child suffix over (v, b) under each key.
            let mut colt = Colt::new(
                apply(&image, &[], &[], Vec::new()),
                &[],
                vec![vec![0], vec![1, 2]],
            );
            for key in 0..7u64 {
                let Some(child) = colt.get(Colt::root(), 0, &[key]) else {
                    continue;
                };
                let got = drain_at(&mut colt, child, 1, size);
                let expected: Vec<(Vec<u64>, Cursor)> = (0..n_rows)
                    .filter(|&pos| k_col[pos] == key)
                    .map(|pos| {
                        (
                            vec![v_col[pos], b_col[pos]],
                            Cursor::Row(u32::try_from(pos).expect("small")),
                        )
                    })
                    .collect();
                assert_eq!(got, expected, "key {key} suffix, batch {size}");
            }
        }
    }

    /// Dense iteration (docs/architecture/30-execution.md): draining a forced map costs
    /// O(keys) batches, never O(capacity), and capacity follows the
    /// documented sizing formula exactly.
    #[test]
    fn skewed_maps_size_by_the_formula_and_iterate_densely() {
        let dir = TempDir::new("colt-dense-skew");
        let schema = schema();
        // 100k positions, 500 distinct keys — the balance-family shape
        // that used to force a 2x-positions map and walk every slot.
        let rows: Vec<(u64, u64)> = (0..100_000).map(|i| (i % 500, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let root = Colt::root();
        colt.ensure_forced(root, 0);
        // next_pow2(clamp(100_000 / 8, 16, 200_000)) = 16_384; 500 keys
        // never cross 75% load, so no growth.
        assert_eq!(colt.forced_capacity(root), Some(16_384));

        // ceil(500 / 64) batches of 64 (last: the remainder), by count.
        let mut keys = vec![0u64; 64];
        let mut children = vec![Cursor::Row(0); 64];
        let mut token = BatchToken::default();
        let mut calls = 0;
        let mut total = 0;
        loop {
            let (n, next) = colt.iter_batch(root, 0, token, &mut keys, &mut children, 64);
            if n == 0 {
                break;
            }
            calls += 1;
            total += n;
            assert_eq!(n, if calls <= 7 { 64 } else { 500 - 7 * 64 });
            token = next;
        }
        assert_eq!((calls, total), (8, 500), "O(keys) drain");
    }

    /// Near-unique keys rehash-double to the pinned final capacity and
    /// iterate each key exactly once, in dense (insertion) order.
    #[test]
    fn near_unique_maps_grow_to_the_pinned_capacity() {
        let dir = TempDir::new("colt-dense-grow");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..10_000).map(|i| (i, i * 2)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let root = Colt::root();
        colt.ensure_forced(root, 0);
        // Init next_pow2(clamp(1250, 16, 20_000)) = 2048, then doubles
        // at 75% load: 4096, 8192, 16384 (10_000 < 12_288 stops there).
        assert_eq!(colt.forced_capacity(root), Some(16_384));
        let entries = drain(&mut colt, root, 0);
        assert_eq!(entries.len(), 10_000);
        let keys: Vec<u64> = entries.iter().map(|(k, _)| k[0]).collect();
        let mut seen = keys.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 10_000, "each key exactly once");
        // Dense order is ingestion order — deterministic: a second force
        // over the same view drains identically, growth included.
        let mut again = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let repeat: Vec<u64> = drain(&mut again, root, 0)
            .iter()
            .map(|(k, _)| k[0])
            .collect();
        assert_eq!(keys, repeat, "ingestion order survives growth");
    }

    /// The resume token survives growth and interleaved probes: max = 1
    /// stepping equals a single-shot drain.
    #[test]
    fn dense_tokens_resume_across_interleaved_probes() {
        let dir = TempDir::new("colt-dense-token");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..300).map(|i| (i % 40, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let root = Colt::root();
        let single_shot = drain(&mut colt, root, 0);

        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let mut keys = vec![0u64; 1];
        let mut children = vec![Cursor::Row(0); 1];
        let mut token = BatchToken::default();
        let mut stepped = Vec::new();
        loop {
            let (n, next) = colt.iter_batch(root, 0, token, &mut keys, &mut children, 1);
            if n == 0 {
                break;
            }
            stepped.push((keys.clone(), children[0]));
            // An interleaved probe must not disturb the resume token.
            let _ = colt.get(root, 0, &[stepped.len() as u64 % 40]);
            token = next;
        }
        assert_eq!(stepped.len(), single_shot.len());
        for (a, b) in stepped.iter().zip(single_shot.iter()) {
            assert_eq!((&a.0, a.1), (&b.0, b.1));
        }
    }

    /// Selection levels (docs/architecture/30-execution.md): probing lands exactly on the
    /// filtered subtrie a view scan used to produce.
    #[test]
    fn selection_levels_probe_to_the_filtered_subtrie() {
        let dir = TempDir::new("colt-select");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        // Selection on k (column 0); one join level on v (column 1).
        let mut colt = Colt::new(all(&view), &[0], vec![vec![1]]);
        let cursor = colt.select(&[7]).expect("key 7 exists");
        assert_eq!(colt.start(), cursor);
        let entries = drain(&mut colt, cursor, 0);
        assert_eq!(entries.len(), 100, "exactly k = 7's positions");
        assert!(entries.iter().all(|(key, _)| key[0] % 10 == 7));
        // An absent key: the occurrence is empty on this snapshot.
        assert!(colt.select(&[42]).is_none());
    }

    /// Two selections chain; a contradictory pair yields `None` with no
    /// special casing.
    #[test]
    fn chained_selections_intersect_and_contradict() {
        let dir = TempDir::new("colt-select-chain");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..100).map(|i| (i % 10, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        // Selections on k then v; the join level is 0-arity (a constant
        // atom's shape: trie_schema = [[]]).
        let mut colt = Colt::new(all(&view), &[0, 1], vec![vec![]]);
        let cursor = colt.select(&[3, 13]).expect("(3, 13) exists");
        let entries = drain(&mut colt, cursor, 0);
        assert_eq!(entries.len(), 1, "one fact carries (3, 13)");
        // 14 % 10 == 4, so (k = 3, v = 14) contradicts at level 1.
        assert!(colt.select(&[3, 14]).is_none());
    }

    /// A selection-free trie is the old trie: `select(&[])` is the root
    /// and iteration is identical.
    #[test]
    fn zero_selection_tries_are_the_old_tries() {
        let dir = TempDir::new("colt-select-zero");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..200).map(|i| (i % 20, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut plain = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let mut selected = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        assert_eq!(selected.start(), Colt::root());
        let cursor = selected.select(&[]).expect("no selections always hit");
        assert_eq!(cursor, Colt::root());
        let a = drain(&mut plain, Colt::root(), 0);
        let b = drain(&mut selected, cursor, 0);
        assert_eq!(a.len(), b.len());
        assert_eq!(
            a.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            b.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>()
        );
    }

    /// `key_count` labels stay honest below a selection probe.
    #[test]
    fn key_count_labels_below_selections() {
        let dir = TempDir::new("colt-select-count");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..1000).map(|i| (i % 10, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), &[0], vec![vec![1]]);
        let cursor = colt.select(&[7]).expect("key 7 exists");
        // Unforced below the selection: a position-count Estimate.
        assert_eq!(colt.key_count(cursor), KeyCount::Estimate(100));
        // Forcing the join level turns it Exact (v values are distinct).
        colt.ensure_forced(cursor, 0);
        assert_eq!(colt.key_count(cursor), KeyCount::Exact(100));
    }

    /// Two reset + select rounds land on the same pool shape — slabs are
    /// recycled, not regrown.
    #[test]
    fn reset_retains_selection_capacity() {
        let dir = TempDir::new("colt-select-reset");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..500).map(|i| (i % 5, i)).collect();
        let image = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&image), &[0], vec![vec![1]]);
        colt.select(&[3]).expect("key 3 exists");
        let first = colt.watermark();
        colt.reset(apply(&image, &[], &[], Vec::new()));
        assert_eq!(colt.watermark(), 1, "reset empties the pools");
        colt.select(&[3]).expect("key 3 exists");
        assert_eq!(colt.watermark(), first, "same shape, same footprint");
    }

    #[test]
    fn construction_is_lazy_until_the_first_get() {
        let dir = TempDir::new("colt-lazy");
        let schema = schema();
        let rows: Vec<(u64, u64)> = (0..10_000).map(|i| (i % 100, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
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
        let mut colt = Colt::new(all(&view), &[], vec![vec![0, 1]]);
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

        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
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
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
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
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
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
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
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
        let mut colt = Colt::new(all(&view), &[], vec![vec![]]);
        let root = Colt::root();
        let entries = drain(&mut colt, root, 0);
        // Suffix iteration yields one entry per position (empty keys);
        // a probe with the empty key forces and hits iff nonempty.
        assert_eq!(entries.len(), 2);
        let mut colt = Colt::new(all(&view), &[], vec![vec![]]);
        assert!(colt.get(Colt::root(), 0, &[]).is_some());
    }

    /// PRD 04 (docs/hardening): a resume token minted under positions
    /// iteration is refused after its node is forced — the release
    /// assert fires instead of silently reinterpreting the token as a
    /// dense index (the omission wrong-results class). A fresh token
    /// after the force drains the full, correct key set.
    #[test]
    fn a_token_that_outlives_a_force_is_refused() {
        let dir = TempDir::new("colt-stale-token");
        let schema = schema();
        // One key, 200 duplicate positions: the level-1 child is a
        // chunked node, and level 1 is the suffix — positions iteration.
        let rows: Vec<(u64, u64)> = (0..200).map(|i| (7, i)).collect();
        let view = view_of(&dir, &schema, &rows);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let child = colt.get(Colt::root(), 0, &[7]).expect("key 7 exists");
        let mut keys = vec![0u64; 8];
        let mut children = vec![Cursor::Row(0); 8];
        let (n, token) =
            colt.iter_batch(child, 1, BatchToken::default(), &mut keys, &mut children, 8);
        assert_eq!(n, 8);
        let (n, stale) = colt.iter_batch(child, 1, token, &mut keys, &mut children, 8);
        assert_eq!(n, 8, "two positions batches drained");

        // Force the node with the token still outstanding.
        colt.ensure_forced(child, 1);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut keys = vec![0u64; 8];
            let mut children = vec![Cursor::Row(0); 8];
            colt.iter_batch(child, 1, stale, &mut keys, &mut children, 8)
        }))
        .expect_err("the stale token must be refused");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("string panic payload");
        assert!(message.contains("outlived a force"), "{message}");

        // Recovery: a fresh default token drains everything, correctly.
        let entries = drain(&mut colt, child, 1);
        assert_eq!(entries.len(), 200);
        let mut values: Vec<u64> = entries.iter().map(|(k, _)| k[0]).collect();
        values.sort_unstable();
        assert_eq!(values, (0..200).collect::<Vec<u64>>());
    }

    /// PRD 04: `Cursor::Row` iteration honors `max` — `max = 0` yields
    /// nothing into zero-sized buffers (no panic, no over-yield).
    #[test]
    fn row_cursor_iteration_honors_max() {
        let dir = TempDir::new("colt-row-max");
        let schema = schema();
        let view = view_of(&dir, &schema, &[(1, 5)]);
        let mut colt = Colt::new(all(&view), &[], vec![vec![0], vec![1]]);
        let child = colt.get(Colt::root(), 0, &[1]).expect("key 1 exists");
        assert!(matches!(child, Cursor::Row(_)), "singleton pins a row");

        let (n, token) = colt.iter_batch(child, 1, BatchToken::default(), &mut [], &mut [], 0);
        assert_eq!(n, 0, "max = 0 yields nothing");
        let mut keys = vec![0u64; 1];
        let mut children = vec![Cursor::Row(0); 1];
        let (n, token) = colt.iter_batch(child, 1, token, &mut keys, &mut children, 1);
        assert_eq!((n, keys[0]), (1, 5), "max = 1 yields exactly the row");
        let (n, _) = colt.iter_batch(child, 1, token, &mut keys, &mut children, 1);
        assert_eq!(n, 0, "the row yields once");
    }

    /// PRD 04: starting a selection-bearing colt before `select()` is a
    /// release panic — silently dropped selections are wrong results.
    #[test]
    fn start_before_select_panics() {
        let dir = TempDir::new("colt-hard-start");
        let schema = schema();
        let view = view_of(&dir, &schema, &[(1, 5)]);
        let colt = Colt::new(all(&view), &[0], vec![vec![1]]);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| colt.start()))
            .expect_err("unselected start must panic");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .expect("string panic payload");
        assert!(
            message.contains("select() runs before the join"),
            "{message}"
        );
    }
}
