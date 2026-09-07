//! Pipelined Free Join: pending bindings and carried cursors flow between
//! nodes in batches. Batch size one uses the same execution path. Middle
//! nodes expand and probe shared batches; leaves emit directly to the sink.
use std::num::NonZeroUsize;

use crate::exec::colt::{BatchToken, Colt, Cursor, KeyCount};
use crate::image::view::OperandAddr;
use crate::plan::fj::ValidatedPlan;

/// The sink's reply to one emitted binding.
///
/// `SkipSuffix` requests a witnessed subtree skip (legal only for the projection
/// sink). `Stop` is work/deadline/scratch refusal; `Error` is cardinality
/// or corruption. Every executor path propagates terminal replies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    SkipSuffix,
    Stop,
    Error,
}

impl Flow {
    /// Work/scratch refusal or a recorded sink error — later probes must not run.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
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
pub struct LeafBatch<'a> {
    pub keys: &'a [u64],
    pub arity: usize,

    pub survivors: &'a [u32],

    pub key_slots: &'a [usize],

    pub bindings: &'a Bindings,
}

/// Where a leaf-batch output slot's value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafSource {
    Key(usize),

    Outer,
}

impl LeafBatch<'_> {
    #[must_use]
    pub fn source_of(&self, slot: usize) -> LeafSource {
        self.key_slots
            .iter()
            .position(|s| *s == slot)
            .map_or(LeafSource::Outer, LeafSource::Key)
    }

    #[must_use]
    pub fn key(&self, entry: u32, word: usize) -> u64 {
        self.keys[entry as usize * self.arity + word]
    }
}

/// A fused leaf scan: no intermediate key batch is materialized.
/// The sink reads leaf words through
/// [`Colt::suffix_column`] and outer slots through `bindings`.
pub struct LeafScan<'a> {
    pub colt: &'a Colt,

    pub level: usize,

    pub key_slots: &'a [usize],
    pub bindings: &'a Bindings,
}

/// Consumes complete bindings (D3: the executor emits to a sink, never an
/// `output`).
pub trait Sink {
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
pub enum ScanOffer {
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
pub enum SkipCapability {
    Forbidden,
    Licensed,
}

/// Structural test observer. Production instantiates [`NoopCounters`], a
/// zero-sized type whose callbacks compile away. Tests count work and assert
/// batch ordering without timing or recording a profile.
pub trait Counters {
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
pub struct NoopCounters;

/// Dense slot-indexed binding array with an epoch discipline instead of
/// `Option` (branch-light: stale slots are never read — reads are
/// plan-scoped — the epoch exists for debug assertions).
#[derive(Debug)]
pub struct Bindings {
    slots: Vec<u64>,

    #[cfg(debug_assertions)]
    epochs: Vec<u64>,
    #[cfg(debug_assertions)]
    current: u64,
}

/// Default probe batch: enough independent work to overlap memory loads
/// while amortizing per-batch bookkeeping. A tuning parameter, not a
/// hardware constant; batch size one follows the same execution path.
pub const BATCH: usize = 128;

#[derive(Debug, Clone, Copy)]
enum Source {
    Batch(usize),
    Slot(usize),
}

#[derive(Debug, Clone, Copy)]
enum CursorSrc {
    Cover,

    Sibling(usize),

    Carried(usize),

    Const(Cursor),
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

/// Grow-only scratch sizing (the pooled high-water contract): the buffer
/// zero-fills only above its high-water mark, never per pass — `clear` +
/// `resize(n, 0)` re-memset the full window every pass (`_platform_memset`,
/// 3.7% of `meets_chain`) though every element of `[..n]` is written before it
/// is read. Shared by both line-parallel passes and the anti-probe;
/// each caller writes its active window before reading it.
fn grow_scratch<T: Copy + Default>(v: &mut Vec<T>, n: usize) {
    if v.len() < n {
        v.resize(n, T::default());
    }
}

fn word_base(
    cover_vars: &[crate::ir::VarId],
    target: crate::ir::VarId,
    width_of: impl Fn(crate::ir::VarId) -> usize,
) -> Option<usize> {
    let mut base = 0;
    for var in cover_vars {
        if *var == target {
            return Some(base);
        }
        base += width_of(*var);
    }
    None
}

#[derive(Clone, Copy)]
enum Operand<'a> {
    Col(crate::image::ColumnView<'a>),
    Const(u64),
}

const PREFETCH_WIDTH_FLOOR: usize = 4;

/// Fields group by lifecycle, marked by the dividers below (named sub-structs
/// were refused: the grouping buys no new invariant — every field is already
/// private to the executor — and would rename every access in the two hot
/// passes for it).
#[derive(Default)]
struct NodeScratch {
    entry_keys: Vec<u64>,

    children: Vec<Cursor>,

    survivors: Vec<u32>,

    probe_keys: Vec<u64>,

    hashes: Vec<u64>,

    sibling_children: Vec<Vec<Cursor>>,

    sources: Vec<Vec<Source>>,

    residual_sources: Vec<(Source, Source)>,

    word_residual_sources: Vec<(Source, Source)>,

    allen_sources: Vec<(Source, Source)>,

    allen_gather: Vec<u64>,

    allen_codes: Vec<u8>,

    anti_sources: Vec<Vec<Source>>,

    point_checks: Vec<(usize, usize, u64)>,

    point_sources: Vec<(usize, usize, Source, bool)>,

    point_rows: Vec<u32>,

    point_row_ks: Vec<u32>,

    cursor_srcs: Vec<CursorSrc>,

    mask: Vec<u8>,

    parents: Vec<u32>,

    element_origins: Vec<u32>,

    pending_bindings: Vec<u64>,

    pending_cursors: Vec<Cursor>,

    pending_len: usize,

    pending_origins: Vec<u32>,
}

#[derive(Clone, Copy)]
struct ResidualSpec {
    op: crate::ir::WordCmp,
    lhs: crate::ir::VarId,
    rhs: crate::ir::VarId,
    lhs_slot: usize,
    rhs_slot: usize,
    width: usize,
}

#[derive(Clone, Copy)]
struct WordResidualSpec {
    op: crate::ir::WordCmp,
    left: OperandAddr,
    right: OperandAddr,
    lhs_slot: usize,
    rhs_slot: usize,
}

#[derive(Clone, Copy)]
struct AllenResidualSpec {
    lhs: crate::ir::VarId,
    rhs: crate::ir::VarId,
    lhs_slot: usize,
    rhs_slot: usize,
    mask: crate::allen::AllenMask,
}

struct NodePrecompute {
    residual_slots: Vec<ResidualSpec>,
    word_residual_slots: Vec<WordResidualSpec>,
    allen_residual_slots: Vec<AllenResidualSpec>,

    /// [`Executor::bind_allen_masks`] before every execution.
    allen_masks: Vec<crate::allen::AllenMask>,
    point_probes: Vec<PointProbeSpec>,
    anti_probes: Vec<AntiProbeSpec>,
}

/// The executor scratch for one plan shape: per-execution cursor state and
/// per-node buffers, sized once at construction. It does not borrow the
/// plan — the same `&ValidatedPlan` is passed to [`Executor::execute`]
/// (the prepared query owns both, the 40-execution doc).
pub struct Executor {
    batch: usize,

    physical_distinct: Option<crate::plan::fj::ScalarSetTraversal>,

    cursors: Vec<(Cursor, usize)>,

    slot_map: Vec<Vec<Vec<usize>>>,

    precompute: Vec<NodePrecompute>,

    /// suffix) must not fire on it.
    point_probed: Vec<bool>,

    var_widths: Vec<(crate::ir::VarId, usize)>,
    scratch: Vec<NodeScratch>,

    leaf: LeafPrecompute,

    /// filter positions before the sink folds them).
    scan_filter: Vec<u32>,

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
    /// The per-execution ledger refused at a bounded-quantum poll:
    /// cancellation, deadline, work-unit exhaustion, or a working-byte
    /// reservation refusal on COLT growth. Surfaced as the typed work
    /// error by [`Executor::execute`].
    Work(crate::work::WorkError),
    /// Sink Stop/Error — later probes must not run (D10).
    SinkStop,
    SinkError,
}

/// The warm executor's per-execution ledger handle:
/// binding exploration steps and COLT pool growth are charged in bounded
/// quanta — one poll per [`crate::exec::sink::STEP_QUANTUM`] explored
/// cover entries, the same published maximum unpolled quantum as the
/// sinks' — so cancellation/deadlines fire inside the join recursion
/// (emitting nothing included, and under the Elided-witness regime) and
/// the bounded-restart trigger can fire from join growth. Installed per
/// execution by `run_join`; absent (executor unit harnesses) nothing is
/// polled or charged.
pub(super) struct ExecLedger {
    work: crate::work::WorkContext,
    /// Explored cover entries not yet charged (bounded by the quantum).
    pending: u32,
}

enum Drive {
    Leaf,
    Pipeline(std::rc::Rc<PipeTables>),
}

struct PipeTables {
    entry_level: Vec<Vec<usize>>,

    carried: Vec<Vec<usize>>,

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
        parts: Vec<(crate::ir::VarId, usize, usize)>,
        key_words: NonZeroUsize,
    },
}

struct AntiProbeSpec {
    occ: usize,
    form: AntiProbeForm,

    point_parts: Vec<(usize, usize, crate::ir::VarId, usize, bool)>,
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

    parts: Vec<(usize, usize, crate::ir::VarId, usize, bool)>,
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
