//! COLT — the Column-Oriented Lazy Trie (docs/architecture/40-execution.md), per paper §4.2 with the
//! chunked-child-list deviation (`docs/architecture/40-execution.md`).
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

/// Positions per chunk: bounded pointer-chase, independent loads within a
/// chunk (the deviation from the paper's growable per-key vectors).
const CHUNK_LEN: usize = 64;

/// Labeled key count. The label records *what kind* of number this is —
/// `Exact` counts a forced map's distinct keys; `Estimate` counts an
/// unforced vector's positions, an **upper bound** on its distinct keys
/// (duplicate-inflated) and simultaneously the exact cost of iterating
/// it unforced. Both are admissible iteration-cost bounds, so cover
/// choice compares magnitudes first and uses the label only to break
/// ties (docs/architecture/40-execution.md) — label-first preference is exactly the bug that
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

/// One forced level's open-addressed map, bucket-of-8 layout
/// (docs/silicon2/05, exp 16): 8 slots per bucket — 8 ctrl bytes as one
/// aligned word in the ctrl slab, then `8 × arity` key words
/// **column-major within the bucket** (key word 0 of all 8 slots
/// contiguous — the NEON sweep's natural shape) and 8 packed child
/// words, contiguous in the bucket slab with stride `8·arity + 8`. A
/// probe loads the bucket ONCE and resolves all 8 candidates from it;
/// overflow steps to the NEXT bucket (bucket-linear probing — exp 16
/// measured displacement negligible below 0.4 load). No tombstones
/// (build-once, never deleted from). Sizing targets ≤ 0.4 load — the
/// occupancy-invariant band (exp 16: flat probes 0.15–0.4) — from the
/// position-count guess, rehash-doubling in bucket units when short;
/// iteration never touches the slot array — it walks the dense
/// occupied list. Slot indices everywhere (dense entries, probe
/// returns) stay GLOBAL (`bucket·8 + slot`), so ctrl indexing and the
/// dense list are unchanged from the linear layout.
#[derive(Debug, Clone, Copy)]
struct Map {
    arity: usize,
    /// Power-of-two bucket count; home bucket = `hash & (nbuckets−1)`.
    nbuckets: usize,
    len: u32,
    /// Start of this map's ctrl range (`nbuckets * 8` bytes, 8-aligned
    /// groups — a bucket's ctrl word never straddles groups).
    ctrl_start: usize,
    /// Start of this map's buckets (`nbuckets * (8·arity + 8)` words).
    bucket_start: usize,
    /// Start of this map's occupied-slot list in the dense slab —
    /// `len` entries, insertion-ordered, O(keys) to walk.
    dense_start: usize,
}

impl Map {
    /// Words per bucket: `8 × arity` keys (column-major) + 8 children.
    fn stride(&self) -> usize {
        8 * self.arity + 8
    }

    /// Slot capacity (8 per bucket) — the test-facing sizing number.
    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.nbuckets * 8
    }

    /// Bucket-slab word index of a global slot's bucket base.
    #[inline(always)]
    fn bucket_base(&self, idx: usize) -> usize {
        self.bucket_start + (idx >> 3) * self.stride()
    }

    /// Bucket-slab word index of one key word of a global slot
    /// (column-major within the bucket).
    #[inline(always)]
    fn key_word_at(&self, idx: usize, word: usize) -> usize {
        self.bucket_base(idx) + word * 8 + (idx & 7)
    }

    /// Bucket-slab word index of a global slot's packed child.
    #[inline(always)]
    fn child_at(&self, idx: usize) -> usize {
        self.bucket_base(idx) + 8 * self.arity + (idx & 7)
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

/// SWAR zero-byte mask over a bucket's ctrl word: bit 7 of each zero
/// (empty) byte sets (docs/silicon2/05; same masks as the wordmap's
/// window probe, docs/silicon/03).
#[inline(always)]
fn zero_byte_mask(w: u64) -> u64 {
    w.wrapping_sub(0x0101_0101_0101_0101) & !w & 0x8080_8080_8080_8080
}

/// SWAR byte-equality mask against a broadcast needle.
#[inline(always)]
fn eq_byte_mask(w: u64, needle: u8) -> u64 {
    zero_byte_mask(w ^ (u64::from(needle) * 0x0101_0101_0101_0101))
}

/// One prepended selection level's shape (docs/architecture/
/// 40-execution.md, § selection levels): the image columns its trie keys
/// decode from (one column for a scalar field, the start/end pair for an
/// interval field), and whether the level is **set-bound** — a
/// `Term::ParamSet` position, probed once per element with the survivor
/// union feeding the level below. Set-ness is a plan fact (a `ParamId`
/// is scalar or set, never both), so it lives in the trie's shape, not
/// in the per-execution key data.
#[derive(Debug, Clone)]
pub struct SelectionLevel {
    pub columns: Vec<usize>,
    pub set: bool,
}

/// Pool high-water snapshot taken just before a select builds its first
/// union node: everything appended past it — the union's position copies
/// and every map the join forces beneath it — belongs to one execution's
/// set values and is provably dead at the next `select`, which truncates
/// back to the mark (capacity retained: the warm fixpoint the allocation
/// contract requires, docs/architecture/40-execution.md).
#[derive(Debug, Clone, Copy)]
struct PoolMark {
    nodes: usize,
    chunks: usize,
    maps: usize,
    ctrl: usize,
    buckets: usize,
    dense: usize,
}

/// The lazy trie over one occurrence's view. Owns the view (a cheap
/// enum over an `Arc`'d image plus survivor positions) and its pools, so a
/// prepared query can hold and [`Colt::reset`] it across executions with
/// every capacity retained (the 30-execution doc's zero-alloc discipline).
pub struct Colt {
    view: View,
    /// Prepended selection levels (docs/architecture/40-execution.md): one trie
    /// level per Eq-constant, probed once per execution with the resolved
    /// words. Everything below a successful probe is exactly the filtered
    /// subtrie a view scan used to produce — built lazily, only for keys
    /// actually asked about.
    selection_levels: usize,
    /// Per selection level: whether it is set-bound ([`SelectionLevel`]).
    set_levels: Vec<bool>,
    /// The union watermark of the current execution's set probes, if any
    /// ([`PoolMark`]).
    union_mark: Option<PoolMark>,
    /// Per-select probe-hit scratch (capacity retained).
    select_hits: Vec<Cursor>,
    /// Per-select union-position scratch (capacity retained).
    select_positions: Vec<u32>,
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
    /// (docs/architecture/40-execution.md). A rehash abandons its old range at the slab's
    /// interior — reclaimed by [`Colt::reset`], a documented ≤2× slab
    /// transient within a generation.
    dense: Vec<u32>,
    /// Reused key-decoding scratch.
    scratch: Vec<u64>,
}

mod append_child;
mod count;
mod force;
mod gather;
mod grow;
mod hash;
mod iter;
mod new;
mod prefetch;
mod probe;
mod select;

pub use hash::hash_key;
use hash::hash_words;

#[cfg(test)]
mod tests;
