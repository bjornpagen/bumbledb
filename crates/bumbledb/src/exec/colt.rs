//! COLT, a column-oriented lazy trie with chunked child-position lists. Nodes,
//! chunks, map slots and key words live in index-addressed pools, not pointers
//! into shared images. A singleton child pins an image row instead of allocating a
//! node; both kinds are a `Cursor`. Pool access stays bounds-checked.
#![allow(clippy::inline_always)]
pub(super) use crate::image::view::{BoundView, View};

const CHUNK_LEN: usize = 64;

const FIRST_CHUNK_CAP: usize = 8;

/// Labeled key count. The label records *what kind* of number this is —
/// `Exact` counts a forced map's distinct keys; `Estimate` counts an
/// unforced vector's positions, an **upper bound** on its distinct keys
/// (duplicate-inflated) and simultaneously the exact cost of iterating
/// it unforced. Both are admissible iteration-cost bounds, so cover
/// ties — label-first preference is exactly the bug that
/// iterated a 500-key forced map instead of a 7-row view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyCount {
    Exact(u64),

    Estimate(u64),
}

impl KeyCount {
    #[must_use]
    pub(crate) fn magnitude(self) -> u64 {
        match self {
            Self::Exact(n) | Self::Estimate(n) => n,
        }
    }
}

/// A reference into the trie: either a real node or a single image
/// position pinned by a singleton child (no node is allocated for it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cursor {
    Node(NodeRef),
    Row(u32),
}

/// One position run under an unforced suffix node:
/// either the all-rows identity range (positions are the indices) or a
/// borrowed position slice (survivor roots, chunk-chain segments).
#[derive(Debug, Clone, Copy)]
pub(crate) enum SuffixRun<'a> {
    Identity { start: usize, len: usize },
    Positions(&'a [u32]),
}

impl SuffixRun<'_> {
    /// Split borrowed position windows without materializing identity rows.
    pub(crate) fn split_at(self, mid: usize) -> (Self, Self) {
        assert!(mid <= self.len(), "suffix split stays within the run");
        match self {
            Self::Identity { start, len } => (
                Self::Identity { start, len: mid },
                Self::Identity {
                    start: start + mid,
                    len: len - mid,
                },
            ),
            Self::Positions(positions) => {
                let (head, tail) = positions.split_at(mid);
                (Self::Positions(head), Self::Positions(tail))
            }
        }
    }

    #[must_use]
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Identity { len, .. } => *len,
            Self::Positions(p) => p.len(),
        }
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Index of a node in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeRef(u32);

/// Opaque resume token for [`Colt::iter_batch`]; start at `default`.
/// Bit 63 tags every nonzero token with the node state that minted it
/// (clear = positions iteration, set = forced-map iteration), and bits
/// 56–62 carry the minting [`Colt::reset`] epoch — so a token that
/// changed state — the silent-omission wrong-results class, closed on
/// both staleness axes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct BatchToken(u64);

const DENSE_TOKEN_TAG: u64 = 1 << 63;

const TOKEN_EPOCH_MASK: u64 = 0x7F << 56;

const TOKEN_PAYLOAD_MASK: u64 = !(DENSE_TOKEN_TAG | TOKEN_EPOCH_MASK);

/// The token-kind mismatch message: fired when a resume token minted under one
/// node state is presented after that state changed.
const STALE_TOKEN: &str = "iteration token outlived a force — drain before probing this cursor";

/// The token-epoch mismatch message: fired when a resume token minted in one
/// generation is presented after a reset re-minted the pools.
const STALE_EPOCH: &str = "iteration token outlived a reset — drain before the next execution";

#[derive(Debug, Clone, Copy)]
enum Positions {
    Root,

    Chunks { first: u32, last: u32, count: u32 },
}

#[derive(Debug, Clone, Copy)]
enum NodeState {
    Unforced(Positions),
    Forced { map: u32 },
}

#[derive(Debug, Clone, Copy)]
struct Chunk {
    start: u32,

    cap: u8,

    len: u8,

    next: u32,
}

/// Sizing targets ≤ 0.4 load — the measured occupancy-invariant band (flat
/// probes 0.15–0.4) — from the position-count guess, rehash-doubling in bucket
/// units when the next insert would cross it: `(len + 1) * 5 > nbuckets * 16`
/// (5/16 = 1/(8·0.4), 8 slots per bucket); iteration never touches the slot
/// array — it walks the dense occupied list.
#[derive(Debug, Clone, Copy)]
struct Map {
    arity: usize,

    nbuckets: usize,
    len: u32,

    ctrl_start: usize,

    bucket_start: usize,

    dense_start: usize,
}

impl Map {
    fn stride(&self) -> usize {
        8 * self.arity + 8
    }

    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.nbuckets * 8
    }

    #[inline(always)]
    fn bucket_base(&self, idx: usize) -> usize {
        self.bucket_start + (idx >> 3) * self.stride()
    }

    #[inline(always)]
    fn key_word_at(&self, idx: usize, word: usize) -> usize {
        self.bucket_base(idx) + word * 8 + (idx & 7)
    }

    #[inline(always)]
    fn child_at(&self, idx: usize) -> usize {
        self.bucket_base(idx) + 8 * self.arity + (idx & 7)
    }
}

const CHILD_NODE_TAG: u64 = 1 << 63;

fn pack_child(cursor: Cursor) -> u64 {
    match cursor {
        Cursor::Row(position) => u64::from(position),
        Cursor::Node(node) => CHILD_NODE_TAG | u64::from(node.0),
    }
}

/// Internal pool words, never persisted or decoded from database input.
/// New children and singleton promotion use `pack_child`; rehash and bound
/// cloning only copy those words. The u32 payload and node tag are disjoint,
/// so extract the payload once instead of validating the tagged u64 as an
/// integer position at every probe. Pool dereferences remain bounds-checked.
#[inline(always)]
fn unpack_child(word: u64) -> Cursor {
    debug_assert_eq!(word & !(CHILD_NODE_TAG | u64::from(u32::MAX)), 0);
    let payload = u32::try_from(word & u64::from(u32::MAX)).expect("masked u32 child payload");
    if word & CHILD_NODE_TAG == 0 {
        Cursor::Row(payload)
    } else {
        Cursor::Node(NodeRef(payload))
    }
}

/// One prepended selection level's shape: the image columns its trie keys
/// decode from (one column for a scalar field, the start/end pair for an
/// interval field), and whether the level is **set-bound** — a
/// `Term::ParamSet` position, probed once per element with the survivor
/// union feeding the level below. Set-ness is a plan fact (a `ParamId`
/// is scalar or set, never both), so it lives in the trie's shape, not
/// in the per-execution key data.
#[derive(Debug, Clone)]
pub(crate) enum SelectionLevel {
    Point { columns: Vec<usize> },
    Set { columns: Vec<usize> },
}

impl SelectionLevel {
    fn columns(&self) -> &[usize] {
        match self {
            Self::Point { columns } | Self::Set { columns } => columns,
        }
    }

    fn kind(&self) -> SelectionKind {
        match self {
            Self::Point { .. } => SelectionKind::Point,
            Self::Set { .. } => SelectionKind::Set,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionKind {
    Point,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Start {
    Vacuous(Cursor),
    Pending,
    Selected(Cursor),
}

/// Pool high-water snapshot taken just before a select builds its first union
/// node: everything appended past it — the union's position copies and every
/// map the join forces beneath it — belongs to one execution's set values and
/// is provably dead at the next `select`, which truncates back to the mark.
#[derive(Debug, Clone, Copy)]
struct PoolMark {
    nodes: usize,
    chunks: usize,
    chunk_positions: usize,
    maps: usize,
    ctrl: usize,
    buckets: usize,
    dense: usize,
}

/// The lazy trie over one occurrence's view. Owns the view (a cheap
/// enum over an `Arc`'d image plus survivor positions) and its pools, so a
/// prepared query can hold and [`Colt::reset`] it across executions with
/// every capacity retained: refill and advance truncate to a [`PoolMark`] and
/// reuse, never allocate.
pub(crate) struct Colt {
    view: View,

    selection_kinds: Vec<SelectionKind>,

    union_mark: Option<PoolMark>,

    select_hits: Vec<Cursor>,

    select_positions: Vec<u32>,

    /// The execution's start cursor: a selected cursor exists only after a
    /// vacuous or completed select.
    start: Start,

    schema_columns: Vec<Vec<usize>>,
    nodes: Vec<NodeState>,
    chunks: Vec<Chunk>,

    chunk_positions: Vec<u32>,

    maps: Vec<Map>,

    ctrl: Vec<u8>,

    buckets: Vec<u64>,

    dense: Vec<u32>,

    scratch: Vec<u64>,

    stage_keys: Vec<u64>,

    stage_positions: Vec<u32>,

    /// Stamped into bits 56 and up of every batch token, so a token that
    /// crosses a [`Colt::reset`] is refused.
    epoch: u8,

    /// Cancellation context for construction and cold pool growth.
    work: Option<crate::work::WorkContext>,
}

impl Colt {
    /// The trie's retained pool footprint in bytes — O(1) over the pools'
    /// capacities. The shared image is excluded; this trie's owned survivor
    /// positions are included. This is not process heap or resident memory.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn retained_bytes(&self) -> usize {
        use std::mem::size_of;
        let survivors = match &self.view {
            View::Bound(crate::image::view::BoundView::Survivors { positions, .. }) => {
                positions.capacity() * size_of::<u32>()
            }
            View::Unbound | View::Bound(crate::image::view::BoundView::All(_)) => 0,
        };
        survivors
            + self.select_hits.capacity() * size_of::<Cursor>()
            + self.select_positions.capacity() * size_of::<u32>()
            + self.nodes.capacity() * size_of::<NodeState>()
            + self.chunks.capacity() * size_of::<Chunk>()
            + self.chunk_positions.capacity() * size_of::<u32>()
            + self.maps.capacity() * size_of::<Map>()
            + self.ctrl.capacity()
            + self.buckets.capacity() * size_of::<u64>()
            + self.dense.capacity() * size_of::<u32>()
            + self.scratch.capacity() * size_of::<u64>()
            + self.stage_keys.capacity() * size_of::<u64>()
            + self.stage_positions.capacity() * size_of::<u32>()
    }

    /// Install this operation's cancellation context. A cancelled prior
    /// context cannot refuse the next execution after rebind.
    pub(crate) fn bind(&mut self, work: Option<&crate::work::WorkContext>) {
        self.work = work.cloned();
    }

    fn poll_force_batch(&self, units: usize) -> Result<(), crate::work::WorkError> {
        let Some(work) = &self.work else {
            return Ok(());
        };
        work.checkpoint()?;
        if units > 0 {
            work.checkpoint()?;
        }
        Ok(())
    }

    fn selection_depth(&self) -> usize {
        self.selection_kinds.len()
    }

    fn join_index(&self, level: usize) -> usize {
        self.selection_depth() + level
    }

    fn initial_start(selection_depth: usize) -> Start {
        if selection_depth == 0 {
            Start::Vacuous(Cursor::Node(NodeRef(0)))
        } else {
            Start::Pending
        }
    }
}

/// Grow one pool geometrically; ordinary resets retain this capacity.
/// Cancellation is checked before cold growth. Capacity overflow or a
/// fallible reserve failure leaves the vector's existing contents intact.
fn reserve_pool<T>(
    needed: usize,
    vec: &mut Vec<T>,
    work: Option<&crate::work::WorkContext>,
) -> Result<(), crate::work::WorkError> {
    if needed <= vec.capacity() {
        return Ok(());
    }
    if let Some(work) = work {
        work.checkpoint()?;
    }
    let capacity = needed.max(vec.capacity().saturating_mul(2)).max(8);
    vec.try_reserve_exact(capacity - vec.len())
        .map_err(|_| crate::work::WorkError::Allocation)
}

mod append_child;
mod count;
mod force;
mod gather;
mod grow;
mod iter;
mod new;
mod prefetch;
mod probe;
pub(crate) use probe::Probe;
mod select;

use super::swar::{ctrl_tag, eq_byte_mask, hash_core, hash_words, zero_byte_mask};

/// The probe hash for a key, so the executor can compute every hash of a batch
/// (pure ALU) before it issues any bucket load.
#[must_use]
#[inline(always)]
pub(crate) fn hash_key(words: &[u64]) -> u64 {
    hash_words(words)
}

/// [`hash_key`] at a constant width `K`, for the executor's fixed-width
/// gathers. Hash-identical to [`hash_key`]: both delegate to the one `swar`
/// fold, and the wordmap contract test pins the equivalence.
#[must_use]
#[inline(always)]
pub(crate) fn hash_key_core<const K: usize>(words: &[u64]) -> u64 {
    hash_core::<K>(words)
}

#[cfg(test)]
mod tests;
