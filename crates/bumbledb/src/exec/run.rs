//! The pipelined Free Join executor (the architecture docs) —
//! vectorized execution is the default and only path;
//! batch size 1 is merely its degenerate setting, never a mode
//! .
//! path. Middle nodes pump: pending binding rows + carried cursor sets
//! flow node to node, each node expanding pending entries into shared
use std::num::NonZeroUsize;

use crate::exec::colt::{BatchToken, Colt, Cursor, KeyCount};
use crate::image::view::OperandAddr;
use crate::plan::fj::ValidatedPlan;

/// The sink's reply to one emitted binding: `SkipSuffix` requests the D2
/// subtree skip (legal only for the projection sink; the executor enforces
/// the plan's per-node sink-relevance bits, the sink just reports
/// staleness).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Continue,
    SkipSuffix,
}

/// One leaf batch, borrowed from the executor: the
/// last plan node's surviving cover entries, handed to the sink whole —
/// the per-row recursion that used to carry them one binding at a time
/// is gone. A sink reads each output slot either from the batch's cover
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

/// A fused leaf scan: the last node's suffix
/// batch is materialized at all. The sink reads leaf words through
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
    fn emit(&mut self, bindings: &Bindings) -> Flow;

    fn emit_batch(&mut self, batch: &LeafBatch<'_>) -> Flow;

    fn emit_batch_until_skip(&mut self, batch: &LeafBatch<'_>) -> Flow {
        self.emit_batch(batch)
    }

    fn skip_capability(&self) -> SkipCapability {
        SkipCapability::Forbidden
    }

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

/// Sink-side evidence for D2 subtree cancellation. Only projection sinks
/// mint `Licensed`; aggregate sinks inherit the forbidden default because
/// existential variables still multiply their fold domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipCapability {
    Forbidden,
    Licensed,
}

/// One executor phase, for per-(node, phase) time attribution
/// : the sequential segments of
/// a node entry's batch loop. `Descend` wraps the per-survivor recursion
/// loop, so its exclusive time (total minus the next node's phases) is
/// the per-row bookkeeping — binds, journal restores, and leaf emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinPhase {
    Iter,

    Hash,

    Probe,

    Residual,

    Descend,

    Force,

    Gather,
}

/// Execution observability seam (40-execution): the normal path
/// instantiates [`NoopCounters`] — zero-sized, compiled to nothing; the
/// introspection entry point instantiates the counting variant.
pub trait Counters {
    fn node_entry(&mut self, node: usize);

    fn batch(&mut self, node: usize, len: usize);

    fn cover_choice(&mut self, node: usize, subatom: usize, count: KeyCount);

    fn probe_hash(&mut self, node: usize, subatom: usize);
    fn probe(&mut self, node: usize, subatom: usize, hit: bool);
    fn residual(&mut self, node: usize, pass: bool);

    fn anti_probe(&mut self, node: usize, hit: bool);
    fn emit(&mut self);

    fn emits(&self) -> u64 {
        0
    }

    fn skip(&mut self, node: usize);

    /// before the round's rec arms run. Default no-op.
    #[inline]
    fn fixpoint_delta(&mut self, rows: u64) {
        let _ = rows;
    }

    #[inline]
    fn fixpoint_round(&mut self, emitted: u64, absorbed: u64) {
        let _ = (emitted, absorbed);
    }

    /// unimplemented is exactly zero after monomorphization).
    #[inline]
    fn phase_start(&mut self, node: usize, phase: JoinPhase) {
        let _ = (node, phase);
    }
    #[inline]
    fn phase_end(&mut self, node: usize, phase: JoinPhase) {
        let _ = (node, phase);
    }
}

/// Node-index cap for phase attribution tables: indices past the cap
/// share the overflow bucket (`nX` names) — plans deeper than this are
/// attributed coarsely, never dropped.
#[cfg(feature = "trace")]
pub const PHASE_NODE_CAP: usize = 8;

/// The trace-mode phase accumulator:
/// per (node, phase) tick totals via the obs fast clock, flushed as
/// `Category::Phase` point events at capture end. Never in a timing
/// path — the prepared-query execute path selects it only under an
/// active obs capture.
#[cfg(feature = "trace")]
pub struct PhaseTimers {
    acc: [[(u64, u64); JoinPhase::COUNT]; PHASE_NODE_CAP + 1],

    open: [[u64; JoinPhase::COUNT]; PHASE_NODE_CAP + 1],

    /// in-cap (node, phase) never reopens before it closes — but every
    depth: [[u32; JoinPhase::COUNT]; PHASE_NODE_CAP + 1],

    emits: u64,
}

/// The release-path counters: every method compiles to nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCounters;

/// The phase accumulator's inert twin (the `trace` feature is off): a
/// ZST with empty bodies, so the execute path's capture branch is
/// written once, `#[cfg]`-free — the obs.rs law. `obs::capturing` is
/// a compile-time `false` off, so this arm is dead code the optimizer
/// drops; the timing path monomorphizes [`NoopCounters`] exactly as
/// before.
#[cfg(not(feature = "trace"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct PhaseTimers;

#[cfg(not(feature = "trace"))]
impl PhaseTimers {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[expect(
        clippy::unused_self,
        clippy::trivially_copy_pass_by_ref,
        reason = "signature twin of the trace-mode flush (the obs.rs law)"
    )]
    pub fn flush(&self) {}
}

#[cfg(not(feature = "trace"))]
impl Counters for PhaseTimers {
    #[inline]
    fn node_entry(&mut self, _: usize) {}
    #[inline]
    fn batch(&mut self, _: usize, _: usize) {}
    #[inline]
    fn cover_choice(&mut self, _: usize, _: usize, _: KeyCount) {}
    #[inline]
    fn probe_hash(&mut self, _: usize, _: usize) {}
    #[inline]
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn residual(&mut self, _: usize, _: bool) {}
    #[inline]
    fn anti_probe(&mut self, _: usize, _: bool) {}
    #[inline]
    fn emit(&mut self) {}
    #[inline]
    fn skip(&mut self, _: usize) {}
}

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

/// The starting batch size: sized so ~28 MLP lanes see >=28 independent
/// probes in flight with bookkeeping amortized over several waves (D4's
/// model). The exact number is measurement-owned (OPEN, architecture
/// README) — this is the one place it lives.
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
    match op {
        crate::ir::WordCmp::Eq => (0..width).all(|i| lhs(i) == rhs(i)),
        crate::ir::WordCmp::Ne => (0..width).any(|i| lhs(i) != rhs(i)),
        _ => unreachable!("validated: multi-word values admit Eq/Ne only as whole values"),
    }
}

/// Grow-only scratch sizing (the pooled high-water contract): the buffer
/// zero-fills only above its high-water mark, never per pass — `clear` +
/// `resize(n, 0)` re-memset the full window every pass (`_platform_memset`,
/// 3.7% of `meets_chain`) though every element of `[..n]` is written before it
/// is read. Shared by both line-parallel passes and the anti-probe — the
/// contract is behavior, not the refused pass extraction.
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
mod overlap_leaf;
mod pipe_tables;
mod probe_pass;
mod pump;
mod run_node;
mod scan_table;

use cover::better_cover;

#[cfg(test)]
mod tests;
