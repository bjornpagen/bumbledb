//! Pipelined Free Join: pending bindings and carried cursors flow between
//! nodes in batches. Batch size one uses the same execution path. Middle
//! nodes expand and probe shared batches; leaves emit directly to the sink.
use std::num::NonZeroUsize;

use crate::exec::colt::{BatchToken, Colt, Cursor, KeyCount};
use crate::plan::fj::ValidatedPlan;

/// The sink's reply to one emitted binding.
///
/// `SkipSuffix` requests a witnessed subtree skip (legal only for the projection
/// sink). `Stop` is cancellation/allocation/scratch refusal; `Error` is cardinality
/// or corruption. Every executor path propagates terminal replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    SkipSuffix,
    Stop,
    Error,
}

impl Flow {
    /// Work/scratch refusal or a recorded sink error — later probes must not run.
    #[must_use]
    pub(crate) const fn is_terminal(self) -> bool {
        matches!(self, Self::Stop | Self::Error)
    }

    pub(crate) fn from_sink_progress(progress: crate::exec::sink::SinkProgress) -> Self {
        match progress {
            crate::exec::sink::SinkProgress::Continue | crate::exec::sink::SinkProgress::Finish => {
                Self::Continue
            }
            crate::exec::sink::SinkProgress::Stop => Self::Stop,
            crate::exec::sink::SinkProgress::Error => Self::Error,
        }
    }

    /// Prefer a terminal progress over a licensed skip.
    pub(crate) const fn or_skip(self, emitted: Self) -> Self {
        if self.is_terminal() { self } else { emitted }
    }
}

/// One leaf batch, borrowed from the executor: the
/// last plan node's surviving cover entries, handed to the sink whole.
/// A sink reads each output slot either from the batch's cover
/// keys (slots in `key_slots`, varying per entry) or from `bindings`
/// (everything else — bound by ancestor nodes, constant across the
/// batch).
pub(crate) struct LeafBatch<'a> {
    pub keys: &'a [u64],
    pub arity: usize,

    pub survivors: &'a [u32],

    pub key_slots: &'a [usize],

    pub bindings: &'a Bindings,
}

/// Where a leaf-batch output slot's value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeafSource {
    Key(usize),

    Outer,
}

impl LeafBatch<'_> {
    #[must_use]
    pub(crate) fn source_of(&self, slot: usize) -> LeafSource {
        self.key_slots
            .iter()
            .position(|s| *s == slot)
            .map_or(LeafSource::Outer, LeafSource::Key)
    }

    #[must_use]
    pub(crate) fn key(&self, entry: u32, word: usize) -> u64 {
        self.keys[entry as usize * self.arity + word]
    }
}

/// A fused leaf scan: no intermediate key batch is materialized.
/// The sink reads leaf words through
/// [`Colt::suffix_column`] and outer slots through `bindings`.
pub(crate) struct LeafScan<'a> {
    pub colt: &'a Colt,

    pub level: usize,

    pub key_slots: &'a [usize],
    pub bindings: &'a Bindings,
}

/// Consumes complete bindings: the executor emits to a sink, never to an output.
pub(crate) trait Sink {
    /// Whether this sink can consume a witnessed physical set traversal.
    /// Static dispatch erases the extra traversal machinery for ordinary
    /// projection/computed sinks. The executor still requires the plan's
    /// opaque witness before enabling it; capability alone proves nothing.
    #[inline]
    fn may_use_distinct_traversal() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn emit(&mut self, bindings: &Bindings) -> Flow;

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow;

    /// Sticky progress after emit. Infallible harness sinks use the default;
    /// production sinks report latched errors and refusals.
    fn progress(&self) -> crate::exec::sink::SinkProgress {
        crate::exec::sink::SinkProgress::Continue
    }

    fn take_error(&mut self) -> Option<crate::error::Error> {
        None
    }

    fn emit_batch_until_skip(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.emit_batch(batch)
    }

    fn skip_capability(&self) -> SkipCapability {
        SkipCapability::Forbidden
    }

    /// Prepare immutable output routing for this execution's single-cover
    /// fast leaf, after the sink has been aimed at the current rule. Scan
    /// calls keep this key-slot layout; outer binding values may change.
    /// Other sinks need no setup and retain their existing scan behavior.
    fn prepare_scan(&mut self, _key_slots: &[usize]) {}

    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> ScanOffer {
        let _ = scan;
        ScanOffer::Declined
    }

    fn scan_run(&mut self, scan: &LeafScan<'_>, run: crate::exec::colt::SuffixRun<'_>) {
        let _ = (scan, run);
        unreachable!("scan_run without ScanOffer::Open");
    }

    fn end_scan(&mut self, scan: &LeafScan<'_>) -> u64 {
        let _ = scan;
        unreachable!("end_scan without ScanOffer::Open");
    }
}

/// Whether [`Sink::begin_scan`] opened a fused scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanOffer {
    Declined,
    Open,
}

fn emit_node_batch<S: Sink>(
    sink: &mut S,
    suffix_skip: crate::plan::fj::SuffixSkip,
    batch: &LeafBatch<'_>,
) -> Flow {
    match (suffix_skip, sink.skip_capability()) {
        (crate::plan::fj::SuffixSkip::Licensed, SkipCapability::Licensed) => {
            sink.emit_batch_until_skip(batch)
        }
        _ => sink.emit_batch(batch),
    }
}

/// Sink-side evidence for subtree cancellation. Only projection sinks
/// mint `Licensed`; aggregate sinks inherit the forbidden default because
/// existential variables still multiply their fold domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkipCapability {
    Forbidden,
    Licensed,
}

/// Structural test observer. Production instantiates [`NoopCounters`], a
/// zero-sized type whose callbacks compile away. Tests count work and assert
/// batch ordering without timing or recording a profile.
pub(crate) trait Counters {
    fn node_entry(&mut self, node: usize);

    fn batch(&mut self, node: usize, len: usize);

    fn cover_choice(&mut self, node: usize, subatom: usize, count: KeyCount);

    fn probe_hash(&mut self, node: usize, subatom: usize);
    fn probe(&mut self, node: usize, subatom: usize, hit: bool);
    fn residual(&mut self, node: usize, pass: bool);

    fn anti_probe(&mut self, node: usize, hit: bool);
    fn emit(&mut self);

    fn skip(&mut self, node: usize);

    /// Keys scheduled together for one sibling probe. Tests distinguish
    /// cross-parent batches from premature singleton drains without timers.
    #[inline]
    fn probe_batch(&mut self, _node: usize, _subatom: usize, _len: usize) {}
}

/// The release-path counters: every method compiles to nothing.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct NoopCounters;

/// Dense slot-indexed binding array with an epoch discipline instead of
/// `Option` (branch-light: stale slots are never read — reads are
/// plan-scoped — the epoch exists for debug assertions).
#[derive(Debug)]
pub(crate) struct Bindings {
    slots: Vec<u64>,

    #[cfg(debug_assertions)]
    epochs: Vec<u64>,
    #[cfg(debug_assertions)]
    current: u64,
}

/// Default probe batch: enough independent work to overlap memory loads
/// while amortizing per-batch bookkeeping. A tuning parameter, not a
/// hardware constant; batch size one follows the same execution path.
pub(crate) const BATCH: usize = 128;

#[derive(Debug, Clone, Copy)]
enum Source {
    Batch(usize),
    Slot(usize),
}

impl Source {
    /// Resolve an already-lowered binding word against the cover's layout.
    /// Wide values and interval endpoints use the same physical slot map.
    fn of(slot: usize, cover_slots: &[usize]) -> Self {
        cover_slots
            .iter()
            .position(|s| *s == slot)
            .map_or(Self::Slot(slot), Self::Batch)
    }
}

#[derive(Debug, Clone, Copy)]
enum CursorSrc {
    Subatom(usize),

    Carried(usize),

    /// Read this execution's selected start, never a cached cursor value.
    Start,
}

fn compare_wide(
    op: crate::ir::WordCmp,
    width: usize,
    lhs: impl Fn(usize) -> u64,
    rhs: impl Fn(usize) -> u64,
) -> bool {
    if width == 1 {
        return op.compare(&lhs(0), &rhs(0));
    }
    // Canonical words are big-endian significance order. Equality and order
    // share the same comparison, including UUIDs differing only in word two.
    let order = (0..width)
        .map(|i| lhs(i).cmp(&rhs(i)))
        .find(|order| !order.is_eq())
        .unwrap_or(std::cmp::Ordering::Equal);
    op.compare(&order, &std::cmp::Ordering::Equal)
}

/// Grow-only scratch sizing: the buffer zero-fills only above its high-water
/// mark, never per pass, because every caller writes its active window
/// `[..n]` before reading it (a per-pass `resize(n, 0)` memset is measurable).
fn grow_scratch<T: Copy + Default>(v: &mut Vec<T>, n: usize) {
    if v.len() < n {
        v.resize(n, T::default());
    }
}

#[derive(Clone, Copy)]
enum Operand<'a> {
    Col(crate::image::ColumnView<'a>),
    Const(u64),
}

const PREFETCH_WIDTH_FLOOR: usize = 4;

type PointSource = (usize, usize, Source, bool);

/// One execution's borrowed world: the validated plan, one trie per
/// occurrence, the binding array, the sink and the structural observer.
struct JoinCtx<'a, S, C> {
    plan: &'a ValidatedPlan,
    colts: &'a mut [Colt],
    bindings: &'a mut Bindings,
    sink: &'a mut S,
    counters: &'a mut C,
}

/// Where a batch element's operand words live: its own cover key words
/// (`Source::Batch`), or the binding row it extends (`Source::Slot`).
trait BatchRows {
    fn word(&self, element: usize, source: Source, offset: usize) -> u64;

    /// The operand's word when it is the same for every element of the batch.
    fn constant(&self, source: Source, offset: usize) -> Option<u64>;
}

/// A leaf batch: every element extends the one current binding row.
struct LeafRows<'a> {
    keys: &'a [u64],
    arity: usize,
    bindings: &'a Bindings,
}

impl BatchRows for LeafRows<'_> {
    #[inline]
    fn word(&self, element: usize, source: Source, offset: usize) -> u64 {
        match source {
            Source::Batch(word) => self.keys[element * self.arity + word + offset],
            Source::Slot(slot) => self.bindings.get(slot + offset),
        }
    }

    #[inline]
    fn constant(&self, source: Source, offset: usize) -> Option<u64> {
        match source {
            Source::Batch(_) => None,
            Source::Slot(slot) => Some(self.bindings.get(slot + offset)),
        }
    }
}

/// A pipelined batch: each element extends its parent's pending binding row
/// (`slot_count` words per parent).
struct PendingRows<'a> {
    keys: &'a [u64],
    arity: usize,
    parents: &'a [u32],
    bindings: &'a [u64],
    slot_count: usize,
}

impl BatchRows for PendingRows<'_> {
    #[inline]
    fn word(&self, element: usize, source: Source, offset: usize) -> u64 {
        match source {
            Source::Batch(word) => self.keys[element * self.arity + word + offset],
            Source::Slot(slot) => {
                self.bindings[self.parents[element] as usize * self.slot_count + slot + offset]
            }
        }
    }

    #[inline]
    fn constant(&self, _: Source, _: usize) -> Option<u64> {
        None
    }
}

/// Where each probe, residual and membership operand of a node is read for
/// one cover choice; rebuilt only when the cover changes.
#[derive(Default)]
struct SourceLayout {
    cover: Option<usize>,

    /// One key layout per subatom.
    sources: Vec<Vec<Source>>,

    residual_sources: Vec<(Source, Source)>,

    allen_sources: Vec<(Source, Source)>,

    anti_sources: Vec<Vec<Source>>,

    point_sources: Vec<Vec<PointSource>>,

    anti_point_sources: Vec<Vec<PointSource>>,
}

/// Per-batch working buffers. They grow to the batch high-water mark and
/// every pass writes its active window before reading it.
#[derive(Default)]
struct BatchBuffers {
    survivors: Vec<u32>,

    probe_keys: Vec<u64>,

    hashes: Vec<u64>,

    mask: Vec<u8>,

    point_checks: Vec<(usize, usize, u64)>,

    allen_gather: Vec<u64>,

    allen_codes: Vec<u8>,

    point_rows: Vec<u32>,

    point_row_ks: Vec<u32>,

    /// One residual's gathered words and the indices its kernel kept.
    words: Vec<u64>,

    kept: Vec<u32>,
}

/// Retained per-node execution buffers.
#[derive(Default)]
struct NodeScratch {
    entry_keys: Vec<u64>,

    /// One buffer per subatom, whether it enumerates or probes this batch.
    /// Empty means no current or later consumer needs that subatom's child.
    children: Vec<Vec<Cursor>>,

    layout: SourceLayout,

    batch: BatchBuffers,

    parents: Vec<u32>,

    element_origins: Vec<u32>,

    pending_bindings: Vec<u64>,

    pending_cursors: Vec<Cursor>,

    pending_len: usize,

    pending_origins: Vec<u32>,
}

/// The chosen cover subatom's trie position at this node.
#[derive(Clone, Copy)]
struct CoverAt {
    occ: usize,
    cursor: Cursor,
    level: usize,
}

/// A direct leaf scan's residual buffers: the surviving positions, and the
/// gathered words and kept indices of one kernel filter.
#[derive(Default)]
struct ScanBuffers {
    filtered: Vec<u32>,
    words: Vec<u64>,
    kept: Vec<u32>,
}

/// A middle node's accumulated cover batch: `fill` entries of `arity` key
/// words from subatom `cover_sub`.
#[derive(Clone, Copy)]
struct CoverBatch {
    node: usize,
    cover_sub: usize,
    arity: usize,
    fill: usize,
}

/// One sibling subatom's membership probe over the batch survivors.
#[derive(Clone, Copy)]
struct SiblingProbe {
    node: usize,
    sub: usize,
    level: usize,
    arity: usize,
    cursor: ProbeCursor,
}

/// Where each survivor's probe starts.
#[derive(Clone, Copy)]
enum ProbeCursor {
    /// One cursor for the whole batch.
    Shared(Cursor),
    /// The cursor the element's parent carried: column `column` of its
    /// `width`-wide row in `pending_cursors`.
    Carried { column: usize, width: usize },
}

#[derive(Clone, Copy)]
struct ResidualSpec {
    op: crate::ir::WordCmp,
    lhs_slot: usize,
    rhs_slot: usize,
    width: usize,
}

#[derive(Clone, Copy)]
struct AllenResidualSpec {
    lhs_slot: usize,
    rhs_slot: usize,
    mask: crate::allen::AllenMask,
}

struct NodePrecompute {
    residual_slots: Vec<ResidualSpec>,
    allen_residual_slots: Vec<AllenResidualSpec>,

    point_probes: Vec<PointProbeSpec>,
    anti_probes: Vec<AntiProbeSpec>,
}

/// The executor scratch for one plan shape: per-execution cursor state and
/// per-node buffers, sized once at construction. It does not borrow the
/// plan: the prepared query owns both and passes the same `&ValidatedPlan` to
/// [`Executor::execute`].
pub(crate) struct Executor {
    batch: usize,

    physical_distinct: Option<crate::plan::fj::ScalarSetTraversal>,

    cursors: Vec<(Cursor, usize)>,

    slot_map: Vec<Vec<Vec<usize>>>,

    precompute: Vec<NodePrecompute>,

    /// Occurrences whose positions are consumed by membership at any node.
    /// An empty-key cover is not a one-row gate when those positions matter.
    point_probed: Vec<bool>,

    var_widths: Vec<(crate::ir::VarId, usize)>,
    scratch: Vec<NodeScratch>,

    leaf: LeafPrecompute,

    scan_buffers: ScanBuffers,

    drive: Drive,

    /// This execution's work ledger, if the caller installed one
    /// ([`Executor::begin_work`]); cleared when `execute` returns.
    ledger: Option<ExecLedger>,

    cancelled: Vec<u32>,
    cancel_epoch: u32,
    next_origin: u32,

    drive_state: DriveState,

    overlap: crate::interval::overlap::OverlapCache,

    overlap_hits: Vec<u32>,

    overlap_key: Vec<u64>,
}

enum DriveState {
    Running,
    SkipDone,
    Poisoned(Poison),
}

enum Poison {
    OriginOverflow,
    /// Cancellation at a cooperative poll, or allocation failure during
    /// COLT growth. Surfaced as the typed work error by [`Executor::execute`].
    Work(crate::work::WorkError),
    /// Sink Stop or Error: later probes must not run.
    SinkStop,
    SinkError,
}

/// Cooperative cancellation within join recursion: one poll per
/// [`crate::exec::sink::STEP_QUANTUM`] explored cover entries, even if no
/// binding reaches a sink. Installed per execution by `run_join`; direct
/// executor unit harnesses may omit it. No resources are reserved or charged.
pub(super) struct ExecLedger {
    work: crate::work::WorkContext,
    /// Explored cover entries since the last cancellation poll.
    pending: u32,
}

enum Drive {
    Leaf,
    Pipeline(std::rc::Rc<PipeTables>),
}

struct PipeTables {
    entry_level: Vec<Vec<usize>>,

    carried: Vec<Vec<usize>>,

    outgoing: Vec<Vec<CursorSrc>>,

    absorb: SkipAbsorb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkipAbsorb {
    Root,

    Node(usize),
}

enum AntiProbeForm {
    Gate,
    Keyed {
        parts: Vec<(usize, usize)>,
        key_words: NonZeroUsize,
    },
}

struct AntiProbeSpec {
    occ: usize,
    form: AntiProbeForm,

    point_parts: Vec<(usize, usize, usize, bool)>,
}

impl AntiProbeSpec {
    fn key_words(&self) -> usize {
        match &self.form {
            AntiProbeForm::Gate => 0,
            AntiProbeForm::Keyed { key_words, .. } => key_words.get(),
        }
    }
}

struct PointProbeSpec {
    occ: usize,

    parts: Vec<(usize, usize, usize, bool)>,
}

enum LeafPrecompute {
    Generic,
    Fast {
        scan_residuals: Vec<(crate::ir::WordCmp, Source, Source)>,
        const_residuals: Vec<(crate::ir::WordCmp, usize, usize)>,
        row: Vec<u64>,
    },
}

mod anti_probe;
mod bindings;
mod cancel;
mod counters;
mod cover;
mod execute;
mod filter;
mod leaf;
mod leaf_precompute;
mod ledger;
mod overlap_leaf;
mod pipe_tables;
mod probe_pass;
mod pump;
mod run_node;
mod scan_table;

use cover::better_cover;

#[cfg(test)]
mod tests;
