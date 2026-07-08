//! The pipelined Free Join executor (the architecture docs; docs/perf/
//! PRDs 01–10) — vectorized execution is the default and only path;
//! batch size 1 is merely its degenerate setting, never a mode
//! (`docs/architecture/30-execution.md` D4, post-mortem §31).
//!
//! Everything is a monomorphized generic — no `dyn` anywhere in the hot
//! path. Middle nodes pump: pending binding rows + carried cursor sets
//! flow node to node, each node expanding pending entries into shared
//! probe batches (dynamic cover choice per entry; flush on cover
//! change), two-phase-probing every sibling ACROSS parents (phase 1
//! hashes — pure ALU; phase 1.5 prefetches; phase 2 issues all bucket
//! loads — independent chains the out-of-order window overlaps),
//! compacting survivors branchlessly, and routing them onward. The last
//! node runs per parent: leaf fast paths (pinned-row elision, scan-fold
//! pushdown) or the generic leaf batch, emitting to the sink whole.
//! D2 suffix skips cancel origins — the subtree of one absorb-node
//! element — as pure work-skipping: a late cancel re-emits rows the
//! seen-set already holds (set semantics make cancellation
//! correctness-free). The paper's "cross-node-entry accumulation is
//! future work" caveat is retired: deep nodes see full batches.

use crate::exec::colt::{BatchToken, Colt, Cursor, KeyCount};
use crate::ir::normalize::PlacedComparison;
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

/// One leaf batch, borrowed from the executor (docs/perf/ PRD 01): the
/// last plan node's surviving cover entries, handed to the sink whole —
/// the per-row recursion that used to carry them one binding at a time
/// is gone. A sink reads each output slot either from the batch's cover
/// keys (slots in `key_slots`, varying per entry) or from `bindings`
/// (everything else — bound by ancestor nodes, constant across the
/// batch).
pub struct LeafBatch<'a> {
    /// Cover-entry key words, entry-major (`entry * arity + word`).
    pub keys: &'a [u64],
    pub arity: usize,
    /// Surviving entry indices into `keys` (post probe/residual
    /// compaction).
    pub survivors: &'a [u32],
    /// Binding slot of each cover key word, in word order.
    pub key_slots: &'a [usize],
    /// Outer bindings; slots not in `key_slots` are already bound.
    pub bindings: &'a Bindings,
}

/// Where a leaf-batch output slot's value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafSource {
    /// The batch's cover keys, at this word index.
    Key(usize),
    /// The outer bindings (constant across the batch).
    Outer,
}

impl LeafBatch<'_> {
    /// Resolves one slot's source — a linear scan over the (tiny) cover
    /// arity; sinks call this once per batch per output slot, never per
    /// row.
    #[must_use]
    pub fn source_of(&self, slot: usize) -> LeafSource {
        self.key_slots
            .iter()
            .position(|s| *s == slot)
            .map_or(LeafSource::Outer, LeafSource::Key)
    }

    /// The key word for a surviving entry at a word index.
    #[must_use]
    pub fn key(&self, entry: u32, word: usize) -> u64 {
        self.keys[entry as usize * self.arity + word]
    }
}

/// A fused leaf scan (docs/perf/ PRD 05): the last node's suffix
/// positions handed to the sink as runs over live column views — no key
/// batch is materialized at all. The sink reads leaf words through
/// [`Colt::suffix_column`] and outer slots through `bindings`.
pub struct LeafScan<'a> {
    pub colt: &'a Colt,
    /// The leaf join level (the occurrence's last).
    pub level: usize,
    /// Binding slot of each leaf key word, in word order.
    pub key_slots: &'a [usize],
    pub bindings: &'a Bindings,
}

/// Consumes complete bindings (D3: the executor emits to a sink, never an
/// `output()`).
pub trait Sink {
    /// Emits one complete binding — the guard-probe path (single row by
    /// construction) and tests; the join executor's leaf path is
    /// [`Sink::emit_batch`].
    fn emit(&mut self, bindings: &Bindings) -> Flow;

    /// Emits every surviving element of a leaf batch. `stop_on_skip` is
    /// the executor's translation of the leaf node's sink-relevance:
    /// when true, the sink must stop at the first row whose per-row emit
    /// would have signaled [`Flow::SkipSuffix`] and return `SkipSuffix`
    /// (the executor unwinds — the batch's remaining rows bind nothing
    /// sink-relevant, exactly the rows the recursive path never
    /// visited); when false it must consume the entire batch and return
    /// `Continue`. An empty batch returns `Continue`.
    fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow;

    /// Whether this sink can ever signal [`Flow::SkipSuffix`]. D2 is
    /// legal for projections only; aggregate plans additionally mark
    /// every node sink-relevant (hardening PRD 05), so a skip under a
    /// fold is absorbed at the node that produced it — this method
    /// backs the debug tripwire that a skip never *crosses* a node
    /// unless the sink is allowed to skip at all.
    fn may_skip(&self) -> bool {
        false
    }

    /// Opens a fused leaf scan (docs/perf/ PRD 05). `false` — the
    /// default, and an honest capability report, not a shim — sends the
    /// executor to the batch path. A `true` return is followed by any
    /// number of [`Sink::scan_run`] calls and exactly one
    /// [`Sink::end_scan`].
    fn begin_scan(&mut self, scan: &LeafScan<'_>) -> bool {
        let _ = scan;
        false
    }

    /// Folds one position run of an open scan.
    fn scan_run(&mut self, scan: &LeafScan<'_>, run: crate::exec::colt::SuffixRun<'_>) {
        let _ = (scan, run);
        unreachable!("scan_run without begin_scan == true");
    }

    /// Closes an open scan (accumulator write-back). Returns the number
    /// of rows the scan consumed (EXPLAIN's `emits` accounting).
    fn end_scan(&mut self, scan: &LeafScan<'_>) -> u64 {
        let _ = scan;
        unreachable!("end_scan without begin_scan == true");
    }
}

/// One executor phase, for per-(node, phase) time attribution
/// (docs/architecture/50-validation.md): the five sequential segments of
/// a node entry's batch loop. `Descend` wraps the per-survivor recursion
/// loop, so its exclusive time (total minus the next node's phases) is
/// the per-row bookkeeping — binds, journal restores, and leaf emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinPhase {
    /// Drawing a cover batch (`iter_batch`).
    Iter,
    /// Phase 1: gather probe keys + compute hashes (pure ALU).
    Hash,
    /// Phase 2: bucket loads + survivor compaction.
    Probe,
    /// Residual comparisons + compaction.
    Residual,
    /// The per-survivor recursion loop (contains deeper nodes' phases).
    Descend,
    /// Sibling map construction (`ensure_forced`) ahead of a probe pass —
    /// separated because a force ingests every position under the
    /// sibling's cursor: the single biggest non-amortized cost a node
    /// entry can pay.
    Force,
}

#[cfg(feature = "trace")]
impl JoinPhase {
    /// Index into per-phase tables (matches `obs::names::JOIN_PHASE`).
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Iter => 0,
            Self::Hash => 1,
            Self::Probe => 2,
            Self::Residual => 3,
            Self::Descend => 4,
            Self::Force => 5,
        }
    }
}

/// Execution observability seam (30-execution): the normal path
/// instantiates [`NoopCounters`] — zero-sized, compiled to nothing; the
/// EXPLAIN entry point (docs/architecture/30-execution.md) instantiates the counting variant.
pub trait Counters {
    fn node_entry(&mut self, node: usize);
    /// One cover batch was drawn (`len` entries) — EXPLAIN's "batching
    /// engaged" observable: at batch size B over N tuples this fires
    /// ~N/B times, not N times.
    fn batch(&mut self, node: usize, len: usize);
    /// A cover was chosen: which subatom, and whether its count was Exact.
    fn cover_choice(&mut self, node: usize, subatom: usize, exact: bool);
    /// Phase 1 computed one probe hash (ordering assertions: every hash of
    /// a batch precedes its first probe).
    fn probe_hash(&mut self, node: usize, subatom: usize);
    fn probe(&mut self, node: usize, subatom: usize, hit: bool);
    fn residual(&mut self, node: usize, pass: bool);
    fn emit(&mut self);
    /// A D2 subtree skip propagated through this node.
    fn skip(&mut self, node: usize);
    /// A timed phase segment opens/closes (default no-op: only the trace
    /// harness's [`PhaseTimers`] implements these; hot-path cost when
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

/// The trace-mode phase accumulator (docs/architecture/50-validation.md):
/// per (node, phase) tick totals via the obs fast clock, flushed as
/// `Category::Phase` point events at capture end. Never in a timing
/// path — the prepared-query execute path selects it only under an
/// active obs capture.
#[cfg(feature = "trace")]
pub struct PhaseTimers {
    /// `[node][phase] -> (accumulated ticks, calls)`.
    acc: [[(u64, u64); 6]; PHASE_NODE_CAP + 1],
    /// `[node][phase] -> open segment's start tick`.
    open: [[u64; 6]; PHASE_NODE_CAP + 1],
}

#[cfg(feature = "trace")]
impl PhaseTimers {
    #[must_use]
    pub fn new() -> Self {
        Self {
            acc: [[(0, 0); 6]; PHASE_NODE_CAP + 1],
            open: [[0; 6]; PHASE_NODE_CAP + 1],
        }
    }

    /// Emits one `Category::Phase` point event per touched (node, phase):
    /// `a0` = accumulated nanoseconds, `a1` = calls.
    pub fn flush(&self) {
        for (node, phases) in self.acc.iter().enumerate() {
            for (phase, &(ticks, calls)) in phases.iter().enumerate() {
                if calls == 0 {
                    continue;
                }
                crate::obs::event(
                    crate::obs::names::JOIN_PHASE[phase][node],
                    crate::obs::Category::Phase,
                    crate::obs::fastclock::ticks_to_ns(ticks),
                    calls,
                );
            }
        }
    }
}

#[cfg(feature = "trace")]
impl Default for PhaseTimers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "trace")]
impl Counters for PhaseTimers {
    #[inline]
    fn node_entry(&mut self, _: usize) {}
    #[inline]
    fn batch(&mut self, _: usize, _: usize) {}
    #[inline]
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn probe_hash(&mut self, _: usize, _: usize) {}
    #[inline]
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn residual(&mut self, _: usize, _: bool) {}
    #[inline]
    fn emit(&mut self) {}
    #[inline]
    fn skip(&mut self, _: usize) {}
    #[inline]
    fn phase_start(&mut self, node: usize, phase: JoinPhase) {
        self.open[node.min(PHASE_NODE_CAP)][phase.index()] = crate::obs::fastclock::ticks();
    }
    #[inline]
    fn phase_end(&mut self, node: usize, phase: JoinPhase) {
        let (node, phase) = (node.min(PHASE_NODE_CAP), phase.index());
        let cell = &mut self.acc[node][phase];
        cell.0 += crate::obs::fastclock::ticks().wrapping_sub(self.open[node][phase]);
        cell.1 += 1;
    }
}

/// The release-path counters: every method compiles to nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCounters;

impl Counters for NoopCounters {
    #[inline]
    fn node_entry(&mut self, _: usize) {}
    #[inline]
    fn batch(&mut self, _: usize, _: usize) {}
    #[inline]
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn probe_hash(&mut self, _: usize, _: usize) {}
    #[inline]
    fn probe(&mut self, _: usize, _: usize, _: bool) {}
    #[inline]
    fn residual(&mut self, _: usize, _: bool) {}
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
    /// Staleness tracking exists only to power the `debug_assert` in
    /// [`Bindings::get`]; the release path pays no epoch store in the
    /// innermost loop.
    #[cfg(debug_assertions)]
    epochs: Vec<u64>,
    #[cfg(debug_assertions)]
    current: u64,
}

impl Bindings {
    #[must_use]
    pub fn new(slot_count: usize) -> Self {
        Self {
            slots: vec![0; slot_count],
            #[cfg(debug_assertions)]
            epochs: vec![0; slot_count],
            #[cfg(debug_assertions)]
            current: 0,
        }
    }

    /// Starts a fresh execution: every slot becomes stale at once.
    #[allow(clippy::unused_self)] // the epoch bump is debug-only; release reads no state
    pub fn reset(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.current += 1;
        }
    }

    pub fn set(&mut self, slot: usize, value: u64) {
        self.slots[slot] = value;
        #[cfg(debug_assertions)]
        {
            self.epochs[slot] = self.current;
        }
    }

    /// Loads a complete binding row (the pipelined executor's parent
    /// rows, docs/perf/ PRD 09): every slot becomes bound.
    pub fn load_row(&mut self, row: &[u64]) {
        self.slots.copy_from_slice(row);
        #[cfg(debug_assertions)]
        {
            self.current += 1;
            for epoch in &mut self.epochs {
                *epoch = self.current;
            }
        }
    }

    /// Reads a bound slot.
    #[must_use]
    pub fn get(&self, slot: usize) -> u64 {
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            self.epochs[slot], self.current,
            "reads are plan-scoped: slot {slot} must be bound"
        );
        self.slots[slot]
    }

    #[must_use]
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }
}

/// The starting batch size: sized so ~28 MLP lanes see >=28 independent
/// probes in flight with bookkeeping amortized over several waves (D4's
/// model). The exact number is measurement-owned (OPEN, architecture
/// README) — this is the one place it lives.
pub const BATCH: usize = 128;

/// Where a value read during batched probing comes from: a word of the
/// current batch's cover keys (varying per element) or an already-bound
/// outer slot (constant across the batch).
#[derive(Debug, Clone, Copy)]
enum Source {
    Batch(usize),
    Slot(usize),
}

/// A leaf-scan residual operand resolved for one big run (docs/perf/
/// PRD 05): a live column view or the outer binding's constant word.
/// Small runs skip the table. [`SCAN_HOIST_THRESHOLD`] splits them.
#[derive(Clone, Copy)]
enum Operand<'a> {
    Col(crate::image::ColumnView<'a>),
    Const(u64),
}

/// Run length at which hoisting operand/column tables pays for itself:
/// L* = `build_cost` ÷ `per-item saving` (docs/silicon/08). The old value
/// of 32 was forced by a `from_fn`-of-Options table costing ~34 ns/run
/// (rust-lang/rust#108765 — eight outlined closure calls plus a 448 B
/// memcpy, the "+48 ns/row" that docs/perf PRD 05 attributed to
/// hoisting itself); the Option-free prefix table builds in ~3.4 ns
/// straight-line, putting the measured crossover at 4–8.
const SCAN_HOIST_THRESHOLD: usize = 8;

/// The prefetch gates, re-founded by fleet round two (docs/silicon2/01,
/// exp 19): **residency is a property of phase interleaving, not
/// structure footprint.** Between two probe passes over one node's map,
/// the executor's other phases displace the map's lines — reuse
/// distance dwarfs the L2 — so silicon-10's isolation law ("resident ⇒
/// prefetch is pure loss") holds only in isolation; in situ, full
/// phase-1.5 coverage measured 34.7–40.9 → 11.4–12.1 ns/probe at EVERY
/// pressure tier, and a uselessly-covered resident pass costs only
/// +0.2–2.6 ns/probe. The footprint gate therefore exempts only maps
/// small enough to be L1-hot even in situ (guard-scale); the width
/// floor exempts passes too small to amortize the pass overhead.
///
/// The budget's teeth were measured BOTH ways (docs/silicon2/01's
/// Result): dropping it to 32 KiB covered triangle n1's 54 KB colt at
/// 98.8% of passes and bought NOTHING — `jp_probe_n1` was already at the
/// covered floor (12.3 ns/probe over 299k probes; the campaign's "37 ns
/// residual" was an attribution error, probes/pass ≈ 117 not 39) while
/// the ~600k added prefetch µops cost triangle +4.8%. Sub-256 KiB maps
/// on this corpus probe at floor without help; the gate keeps them
/// exempt.
const PREFETCH_L2_BUDGET_BYTES: usize = 256 << 10;

/// Minimum survivors for a phase-1.5 pass (docs/silicon2/01): exp 19
/// measured the pass at ~12 ns fixed + ~0.3 ns/probe — a 4-survivor
/// pass amortizes it; below that it is pure overhead.
const PREFETCH_WIDTH_FLOOR: usize = 4;

/// Appends the positions of `run` that pass `eval` to `out`.
fn push_surviving(
    run: crate::exec::colt::SuffixRun<'_>,
    out: &mut Vec<u32>,
    eval: &mut impl FnMut(u32) -> bool,
) {
    match run {
        crate::exec::colt::SuffixRun::Identity { start, len } => {
            for position in start..start + len {
                let position = u32::try_from(position).expect("positions fit u32");
                if eval(position) {
                    out.push(position);
                }
            }
        }
        crate::exec::colt::SuffixRun::Positions(positions) => {
            for &position in positions {
                if eval(position) {
                    out.push(position);
                }
            }
        }
    }
}

/// Residual specs one scan can hold; the resolve site asserts.
const MAX_LEAF_RESIDUALS: usize = 8;

/// Per-node reusable scratch: each node's frame is active at most once in
/// the recursion (frames advance strictly by node index), so scratch is
/// indexed by node and allocated once per executor construction.
#[derive(Default)]
struct NodeScratch {
    /// Cover-entry key words, entry-major (`entry * arity + word`).
    entry_keys: Vec<u64>,
    /// Cover-entry child cursors.
    children: Vec<Cursor>,
    /// Surviving batch-entry indices (branchlessly compacted).
    survivors: Vec<u32>,
    /// Phase-1 gathered probe keys, entry-major per sibling pass.
    probe_keys: Vec<u64>,
    /// Phase-1 hashes, aligned with `survivors`.
    hashes: Vec<u64>,
    /// Per subatom, per entry: the probed child cursor.
    sibling_children: Vec<Vec<Cursor>>,
    /// Per sibling-var value sources, recomputed per node entry (the
    /// runtime cover choice decides what comes from the batch).
    sources: Vec<Vec<Source>>,
    /// Residual operand sources, aligned with the node's residual list.
    residual_sources: Vec<(Source, Source)>,
    /// Per-entry survivor mask for the compaction kernel.
    mask: Vec<u8>,
    /// Pipeline probe-batch parent indices (docs/perf/ PRD 09): the
    /// pending entry each batch element expanded from.
    parents: Vec<u32>,
    /// Pending binding rows awaiting this node, entry-major
    /// (stride = slot count).
    pending_bindings: Vec<u64>,
    /// Pending carried cursors, entry-major (stride = the node's carried
    /// occurrence count).
    pending_cursors: Vec<Cursor>,
    /// Entries in the pending buffers.
    pending_len: usize,
    /// Per pending entry: the D2 origin it descends from (docs/perf/
    /// PRD 10) — minted at the absorb node's routing, inherited below.
    pending_origins: Vec<u32>,
    /// Per probe-batch element: the origin (aligned with `parents`).
    element_origins: Vec<u32>,
}

/// The executor scratch for one plan shape: per-execution cursor state and
/// per-node buffers, sized once at construction. It does not borrow the
/// plan — the same `&ValidatedPlan` is passed to [`Executor::execute`]
/// (the prepared query owns both, the 30-execution doc).
pub struct Executor {
    batch: usize,
    /// Per occurrence: (current cursor, current trie level).
    cursors: Vec<(Cursor, usize)>,
    /// Per subatom slot maps, precomputed: `slot_map[node][subatom][i]` is
    /// the binding slot of that subatom's i-th variable.
    slot_map: Vec<Vec<Vec<usize>>>,
    /// Per residual: (lhs slot, rhs slot), aligned with each node's list.
    residual_slots: Vec<Vec<(PlacedComparison, usize, usize)>>,
    scratch: Vec<NodeScratch>,
    /// The leaf fast paths (docs/perf/ PRD 05) apply when the last node
    /// has exactly one subatom — its cover is fixed, so the per-entry
    /// source resolution is precomputed here once.
    leaf_single: bool,
    /// Leaf residual value sources, fixed at construction (single-subatom
    /// leaves only; `Batch` = leaf key word, `Slot` = outer binding).
    leaf_residual_sources: Vec<(Source, Source)>,
    /// The scan arm's residual partition, also fixed at construction
    /// (zero-alloc warm contract: nothing recomputes per node entry):
    /// per-position specs (at least one side reads a leaf column) and
    /// batch-constant specs (both sides outer).
    leaf_scan_residuals: Vec<(crate::ir::CmpOp, Source, Source)>,
    leaf_const_residuals: Vec<(crate::ir::CmpOp, usize, usize)>,
    /// One pinned row's gathered key words (the pinned-leaf elision's
    /// only buffer).
    leaf_row: Vec<u64>,
    /// Residual-surviving positions of one scan run (docs/perf/ PRD 05:
    /// leaf residuals filter positions before the sink folds them).
    scan_filter: Vec<u32>,
    /// The pipelined executor's shape tables (docs/perf/ PRD 09/10):
    /// `Some` for every multi-node plan — the one executor.
    pipe: Option<PipeTables>,
    /// D2 origin cancellation (docs/perf/ PRD 10), epoch-stamped:
    /// `cancelled[origin] == cancel_epoch` marks a dead subtree. Grows
    /// to the per-execution origin high-water and is never cleared.
    cancelled: Vec<u32>,
    cancel_epoch: u32,
    next_origin: u32,
    /// A skip crossed the virtual root: the whole execution is done.
    all_cancelled: bool,
}

/// The pipelined executor's static shape tables (docs/perf/ PRD 09):
/// levels and carried-cursor columns are plan facts, derived once.
struct PipeTables {
    /// `[node][occ]` — the join level an occurrence's cursor sits at when
    /// the node begins (= its appearances in earlier nodes).
    entry_level: Vec<Vec<usize>>,
    /// `[node]` — occurrences whose cursors pending entries carry INTO
    /// the node (advanced by an earlier node, used at this node or
    /// later).
    carried: Vec<Vec<usize>>,
    /// `[node][occ]` — the carried column, aligned with `carried[node]`.
    carried_col: Vec<Vec<Option<usize>>>,
    /// The D2 absorb node (docs/perf/ PRD 10): the deepest sink-relevant
    /// node — a leaf skip cancels the subtree of one of its elements.
    /// `Some(N-1)` (the leaf itself) means skips never cross a node;
    /// `None` means a skip ends the whole execution. Skips only exist
    /// under sinks that `may_skip`; cancellation is an optimization —
    /// a late cancel re-emits rows the seen-set already holds.
    absorb: Option<usize>,
}

impl PipeTables {
    fn of(plan: &ValidatedPlan) -> Self {
        let n_nodes = plan.nodes().len();
        let n_occ = plan.occurrences().len();
        let mut appears = vec![vec![false; n_nodes]; n_occ];
        for (node_idx, node) in plan.nodes().iter().enumerate() {
            for subatom in &node.subatoms {
                appears[usize::from(subatom.occ.0)][node_idx] = true;
            }
        }
        let mut entry_level = Vec::with_capacity(n_nodes);
        let mut carried = Vec::with_capacity(n_nodes);
        let mut carried_col = Vec::with_capacity(n_nodes);
        for node_idx in 0..n_nodes {
            let mut levels = Vec::with_capacity(n_occ);
            let mut occs = Vec::new();
            let mut cols = vec![None; n_occ];
            for (occ, at) in appears.iter().enumerate() {
                levels.push(at[..node_idx].iter().filter(|b| **b).count());
                let before = at[..node_idx].iter().any(|b| *b);
                let at_or_after = at[node_idx..].iter().any(|b| *b);
                if before && at_or_after {
                    cols[occ] = Some(occs.len());
                    occs.push(occ);
                }
            }
            entry_level.push(levels);
            carried.push(occs);
            carried_col.push(cols);
        }
        let absorb = (0..n_nodes).rev().find(|&m| plan.nodes()[m].sink_relevant);
        Self {
            entry_level,
            carried,
            carried_col,
            absorb,
        }
    }
}

/// The single-subatom-leaf precompute (docs/perf/ PRD 05): everything
/// the leaf fast paths would otherwise re-derive per node entry.
struct LeafPrecompute {
    single: bool,
    residual_sources: Vec<(Source, Source)>,
    scan_residuals: Vec<(crate::ir::CmpOp, Source, Source)>,
    const_residuals: Vec<(crate::ir::CmpOp, usize, usize)>,
    row: Vec<u64>,
}

impl LeafPrecompute {
    fn of(plan: &ValidatedPlan, residual_slots: &[Vec<(PlacedComparison, usize, usize)>]) -> Self {
        let last = plan.nodes().len() - 1;
        let single = plan.nodes()[last].subatoms.len() == 1;
        if !single {
            return Self {
                single,
                residual_sources: Vec::new(),
                scan_residuals: Vec::new(),
                const_residuals: Vec::new(),
                row: Vec::new(),
            };
        }
        let cover_vars = &plan.nodes()[last].subatoms[0].vars;
        let residual_sources: Vec<(Source, Source)> = residual_slots[last]
            .iter()
            .map(|(residual, lhs_slot, rhs_slot)| {
                let resolve = |var: crate::ir::VarId, slot: usize| {
                    cover_vars
                        .iter()
                        .position(|cv| *cv == var)
                        .map_or(Source::Slot(slot), Source::Batch)
                };
                (
                    resolve(residual.lhs, *lhs_slot),
                    resolve(residual.rhs, *rhs_slot),
                )
            })
            .collect();
        let mut scan_residuals = Vec::new();
        let mut const_residuals = Vec::new();
        for (idx, (lhs, rhs)) in residual_sources.iter().enumerate() {
            let op = residual_slots[last][idx].0.op;
            match (lhs, rhs) {
                (Source::Slot(l), Source::Slot(r)) => const_residuals.push((op, *l, *r)),
                _ => scan_residuals.push((op, *lhs, *rhs)),
            }
        }
        Self {
            single,
            residual_sources,
            scan_residuals,
            const_residuals,
            row: vec![0u64; cover_vars.len().max(1)],
        }
    }
}

impl Executor {
    /// An executor with the default batch size ([`BATCH`]).
    #[must_use]
    pub fn new(plan: &ValidatedPlan) -> Self {
        Self::with_batch_size(plan, BATCH)
    }

    /// An executor with an explicit batch size — the scalar/vectorized
    /// equality tests parameterize this; there is no mode, only the number.
    ///
    /// # Panics
    ///
    /// Only on a programmer-invariant violation: a zero batch size.
    #[must_use]
    pub fn with_batch_size(plan: &ValidatedPlan, batch: usize) -> Self {
        assert!(
            batch > 0,
            "a batch has at least one element (set_batch_size is the caller-facing knob)"
        );
        let slot_map: Vec<Vec<Vec<usize>>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.subatoms
                    .iter()
                    .map(|s| s.vars.iter().map(|v| plan.slot_of(*v)).collect())
                    .collect()
            })
            .collect();
        let residual_slots: Vec<Vec<(PlacedComparison, usize, usize)>> = plan
            .nodes()
            .iter()
            .map(|node| {
                node.residuals
                    .iter()
                    .map(|r| (*r, plan.slot_of(r.lhs), plan.slot_of(r.rhs)))
                    .collect()
            })
            .collect();
        let scratch = plan
            .nodes()
            .iter()
            .map(|node| {
                let max_arity = node
                    .subatoms
                    .iter()
                    .map(|s| s.vars.len())
                    .max()
                    .unwrap_or(0)
                    .max(1);
                NodeScratch {
                    entry_keys: vec![0; batch * max_arity],
                    children: vec![Cursor::Row(0); batch],
                    survivors: Vec::with_capacity(batch),
                    probe_keys: vec![0; batch * max_arity],
                    hashes: Vec::with_capacity(batch),
                    sibling_children: node
                        .subatoms
                        .iter()
                        .map(|_| vec![Cursor::Row(0); batch])
                        .collect(),
                    sources: node.subatoms.iter().map(|_| Vec::new()).collect(),
                    residual_sources: Vec::new(),
                    mask: Vec::with_capacity(batch),
                    parents: Vec::with_capacity(batch),
                    pending_bindings: Vec::new(),
                    pending_cursors: Vec::new(),
                    pending_len: 0,
                    pending_origins: Vec::new(),
                    element_origins: Vec::with_capacity(batch),
                }
            })
            .collect();
        let leaf = LeafPrecompute::of(plan, &residual_slots);
        Self {
            batch,
            cursors: Vec::new(),
            slot_map,
            residual_slots,
            scratch,
            leaf_single: leaf.single,
            leaf_residual_sources: leaf.residual_sources,
            leaf_scan_residuals: leaf.scan_residuals,
            leaf_const_residuals: leaf.const_residuals,
            leaf_row: leaf.row,
            scan_filter: Vec::new(),
            pipe: (plan.nodes().len() >= 2).then(|| PipeTables::of(plan)),
            cancelled: Vec::new(),
            cancel_epoch: 0,
            next_origin: 0,
            all_cancelled: false,
        }
    }

    /// Runs the plan over the COLT sources (one per occurrence, indexed by
    /// occurrence id), emitting complete bindings to the sink.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (sources not matching the
    /// plan's occurrences).
    pub fn execute<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        assert_eq!(colts.len(), plan.occurrences().len());
        debug_assert_eq!(plan.nodes().len(), self.scratch.len(), "same plan shape");
        bindings.reset();
        self.cursors.clear();
        // Each occurrence starts below its selection levels — the root
        // when it has none, the post-`select` cursor otherwise
        // (docs/architecture/30-execution.md).
        self.cursors
            .extend(colts.iter().map(|colt| (colt.start(), 0usize)));
        // The one executor (docs/perf/ PRD 09/10): multi-node plans
        // pipeline — probes batch ACROSS parent entries, D2 skips cancel
        // origins — and single-node plans are one leaf pass. The
        // recursive per-survivor executor is gone.
        if self.pipe.is_some() {
            self.run_pipeline(plan, colts, bindings, sink, counters);
        } else {
            self.run_node(plan, 0, colts, bindings, sink, counters);
        }
    }

    /// The pipelined executor (docs/perf/ PRD 09): pending binding rows
    /// and carried cursor sets flow node to node; each middle node
    /// expands pending entries into shared probe batches (flushed on
    /// cover change), probes every sibling across parents, and appends
    /// survivors to the next node's pending. The last node runs per
    /// parent through the ordinary `run_node` machinery — leaf fast
    /// paths, counters, phases and all.
    fn run_pipeline<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let tables = self.pipe.take().expect("dispatched on Some");
        let slot_count = bindings.slot_count();
        for scratch in &mut self.scratch {
            scratch.pending_bindings.clear();
            scratch.pending_cursors.clear();
            scratch.pending_origins.clear();
            scratch.pending_len = 0;
        }
        // D2 state (PRD 10): a fresh epoch outlives any prior execution's
        // cancellations without clearing the high-water table.
        self.cancel_epoch = self.cancel_epoch.wrapping_add(1);
        self.next_origin = 0;
        self.all_cancelled = false;
        // The virtual root entry: no bindings, no carried cursors.
        self.scratch[0].pending_bindings.resize(slot_count, 0);
        self.scratch[0].pending_len = 1;
        self.scratch[0].pending_origins.push(0);
        self.pump(&tables, plan, 0, colts, bindings, sink, counters);
        self.pipe = Some(tables);
    }

    /// Whether an origin's subtree was cancelled (PRD 10).
    fn origin_cancelled(&self, origin: u32) -> bool {
        self.cancelled
            .get(origin as usize)
            .is_some_and(|&e| e == self.cancel_epoch)
    }

    /// Cancels one origin's subtree.
    fn cancel_origin(&mut self, origin: u32) {
        let idx = origin as usize;
        if self.cancelled.len() <= idx {
            self.cancelled
                .resize(idx + 1, self.cancel_epoch.wrapping_sub(1));
        }
        self.cancelled[idx] = self.cancel_epoch;
    }

    /// Consumes every pending entry at a middle node, cascading full
    /// child batches immediately and draining the remainder at the end.
    #[allow(clippy::too_many_lines)] // one batch loop; the invariants read in order
    #[allow(clippy::too_many_arguments)]
    fn pump<S: Sink, C: Counters>(
        &mut self,
        tables: &PipeTables,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let n_nodes = plan.nodes().len();
        debug_assert!(node_idx + 1 < n_nodes, "the leaf runs per parent");
        let mut scratch = std::mem::take(&mut self.scratch[node_idx]);
        let carried_w = tables.carried[node_idx].len();

        // One in-order pass (docs/silicon2/08): per-entry dynamic cover
        // choice at processing time, probe_pass flushed on cover change.
        // Cover-stable segregation (docs/silicon/14) precomputed covers
        // and grouped entries to lift probe-batch means 37 → 39 — then
        // exp 14 priced the per-pass overhead it amortizes at 11–30 ns,
        // TWENTY TIMES below the campaign's assumption, making the whole
        // batch-mean lever class a ~1% effect; the two-pass machinery is
        // deleted. Cross-call fill carry is rejected by the same number
        // before ever being built: lifting batch means to ~128 is worth
        // 0.2–1.2% of triangle p50 at the measured pass overhead
        // (bumblebench exp 14) — the lever class is closed. The cover
        // choice itself is a performance heuristic — any cover is
        // correct — so choosing from live colt state (a force during an
        // earlier flush could have flipped an Estimate to Exact) changes
        // nothing semantic.
        let node = &plan.nodes()[node_idx];
        let mut fill = 0usize;
        // The open cover run: (cover_sub, arity, occ, level).
        let mut group: Option<(usize, usize, usize, usize)> = None;
        for entry in 0..scratch.pending_len {
            if self.all_cancelled {
                break;
            }
            // D2 (PRD 10): a cancelled origin's pending work is dead —
            // its outputs could only duplicate rows already seen. Origin
            // ids are meaningful strictly BELOW the absorb node (minted
            // at its routing); above it entries carry the meaningless
            // seed and must never be filtered. Cancellation fired during
            // an earlier entry's flush is seen here: the check runs at
            // each entry's turn.
            if tables.absorb.is_some_and(|a| node_idx > a)
                && self.origin_cancelled(scratch.pending_origins[entry])
            {
                continue;
            }
            counters.node_entry(node_idx);
            let mut best: Option<(usize, KeyCount)> = None;
            for &cover in &node.covers {
                let sub_idx = usize::from(cover);
                let occ = usize::from(node.subatoms[sub_idx].occ.0);
                let cursor = match tables.carried_col[node_idx][occ] {
                    Some(col) => scratch.pending_cursors[entry * carried_w + col],
                    None => colts[occ].start(),
                };
                let count = colts[occ].key_count(cursor);
                let better = match &best {
                    None => true,
                    Some((_, incumbent)) => better_cover(count, *incumbent),
                };
                if better {
                    best = Some((sub_idx, count));
                }
            }
            let (cover_sub, count) = best.expect("validated plans have non-empty cover sets");
            counters.cover_choice(node_idx, cover_sub, matches!(count, KeyCount::Exact(_)));
            let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
            let cover_level = tables.entry_level[node_idx][cover_occ];
            let cur_arity = node.subatoms[cover_sub].vars.len();
            if let Some((open_sub, open_arity, _, _)) = group {
                if open_sub != cover_sub && fill > 0 {
                    self.probe_pass(
                        tables, plan, node_idx, open_sub, open_arity, fill, &mut scratch,
                        colts, bindings, sink, counters,
                    );
                    fill = 0;
                }
            }
            group = Some((cover_sub, cur_arity, cover_occ, cover_level));
            let cover_cursor = match tables.carried_col[node_idx][cover_occ] {
                Some(col) => scratch.pending_cursors[entry * carried_w + col],
                None => colts[cover_occ].start(),
            };
            let mut token = BatchToken::default();
            loop {
                let want = self.batch - fill;
                let (yielded, next) = colts[cover_occ].iter_batch(
                    cover_cursor,
                    cover_level,
                    token,
                    &mut scratch.entry_keys[fill * cur_arity..],
                    &mut scratch.children[fill..],
                    want,
                );
                counters.batch(node_idx, yielded);
                for _ in 0..yielded {
                    scratch
                        .parents
                        .push(u32::try_from(entry).expect("pending fits u32"));
                    scratch.element_origins.push(scratch.pending_origins[entry]);
                }
                fill += yielded;
                token = next;
                if fill == self.batch {
                    self.probe_pass(
                        tables, plan, node_idx, cover_sub, cur_arity, fill, &mut scratch,
                        colts, bindings, sink, counters,
                    );
                    fill = 0;
                    if yielded == want {
                        continue; // the entry may have more; resume its token
                    }
                }
                if yielded < want {
                    break; // entry exhausted
                }
            }
        }
        if fill > 0 {
            if let Some((open_sub, open_arity, _, _)) = group {
                self.probe_pass(
                    tables, plan, node_idx, open_sub, open_arity, fill, &mut scratch, colts,
                    bindings, sink, counters,
                );
            }
        }
        scratch.pending_len = 0;
        scratch.pending_bindings.clear();
        scratch.pending_cursors.clear();
        scratch.pending_origins.clear();
        scratch.parents.clear();
        scratch.element_origins.clear();
        self.scratch[node_idx] = scratch;
        // Drain the child's sub-batch remainder (full batches cascaded
        // already inside probe_pass's flush check — see its tail).
        if node_idx + 2 < n_nodes && self.scratch[node_idx + 1].pending_len > 0 {
            self.pump(tables, plan, node_idx + 1, colts, bindings, sink, counters);
        }
    }

    #[allow(clippy::too_many_lines)] // the one hot loop; splitting it would
                                     // scatter the batch invariants the comments walk through in order
    fn run_node<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        // The one caller class (docs/perf/ PRD 10): the LAST node — the
        // pipeline pumps every middle node, single-node plans call this
        // directly. Zero-node plans are unrepresentable (validation
        // rule 14 rejects atom-less queries).
        assert!(
            node_idx + 1 == plan.nodes().len(),
            "run_node is the leaf pass; middle nodes pump"
        );
        // The leaf fast paths (docs/perf/ PRD 05): pinned-row elision and
        // the scan-fold pushdown. A `None` decline falls through to the
        // generic batch machinery with no counters fired.
        if self.leaf_single && node_idx + 1 == plan.nodes().len() {
            if let Some(flow) = self.run_leaf_fast(plan, node_idx, colts, bindings, sink, counters)
            {
                return flow;
            }
        }
        counters.node_entry(node_idx);

        // Dynamic cover choice (§4.4): prefer the smallest Exact, else the
        // smallest Estimate — the labels are load-bearing and never
        // compared as the same quantity (post-mortem §40).
        let cover_sub = self.choose_cover(plan, node_idx, colts);
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        let (cover_cursor, cover_level) = self.cursors[cover_occ];
        counters.cover_choice(
            node_idx,
            cover_sub,
            matches!(colts[cover_occ].key_count(cover_cursor), KeyCount::Exact(_)),
        );

        let arity = node.subatoms[cover_sub].vars.len();
        let mut scratch = std::mem::take(&mut self.scratch[node_idx]);

        // Resolve value sources against the runtime cover choice: a var
        // bound by the chosen cover reads the batch key column; everything
        // else reads its (already bound) outer slot.
        let cover_vars = &plan.nodes()[node_idx].subatoms[cover_sub].vars;
        for (sub_idx, subatom) in plan.nodes()[node_idx].subatoms.iter().enumerate() {
            scratch.sources[sub_idx].clear();
            for (i, var) in subatom.vars.iter().enumerate() {
                let source = cover_vars.iter().position(|cv| cv == var).map_or(
                    Source::Slot(self.slot_map[node_idx][sub_idx][i]),
                    Source::Batch,
                );
                scratch.sources[sub_idx].push(source);
            }
        }
        scratch.residual_sources.clear();
        for (residual, lhs_slot, rhs_slot) in &self.residual_slots[node_idx] {
            let resolve = |var: crate::ir::VarId, slot: usize| {
                cover_vars
                    .iter()
                    .position(|cv| *cv == var)
                    .map_or(Source::Slot(slot), Source::Batch)
            };
            scratch.residual_sources.push((
                resolve(residual.lhs, *lhs_slot),
                resolve(residual.rhs, *rhs_slot),
            ));
        }

        let mut token = BatchToken::default();
        let mut flow = Flow::Continue;

        'outer: loop {
            counters.phase_start(node_idx, JoinPhase::Iter);
            let (yielded, next_token) = colts[cover_occ].iter_batch(
                cover_cursor,
                cover_level,
                token,
                &mut scratch.entry_keys,
                &mut scratch.children,
                self.batch,
            );
            counters.phase_end(node_idx, JoinPhase::Iter);
            if yielded == 0 {
                break;
            }
            counters.batch(node_idx, yielded);
            token = next_token;
            scratch.survivors.clear();
            scratch
                .survivors
                .extend(0..u32::try_from(yielded).expect("batch fits u32"));

            // Per sibling: the two-phase probe, then branchless compaction.
            let value_of = |sources: &[Source],
                            entry_keys: &[u64],
                            bindings: &Bindings,
                            entry: usize,
                            i: usize| match sources[i] {
                Source::Batch(word) => entry_keys[entry * arity + word],
                Source::Slot(slot) => bindings.get(slot),
            };
            for sub_idx in 0..plan.nodes()[node_idx].subatoms.len() {
                if sub_idx == cover_sub || scratch.survivors.is_empty() {
                    continue;
                }
                let subatom = &plan.nodes()[node_idx].subatoms[sub_idx];
                let sub_arity = subatom.vars.len();
                let occ = usize::from(subatom.occ.0);
                let (s_cursor, s_level) = self.cursors[occ];
                counters.phase_start(node_idx, JoinPhase::Force);
                colts[occ].ensure_forced(s_cursor, s_level);
                counters.phase_end(node_idx, JoinPhase::Force);

                // Phase 1: gather every probe key and compute every hash —
                // pure ALU, no bucket loads. A pinned sibling
                // (`Cursor::Row`) probes by field equality, never by
                // hash: skip the hash work and its counter, so EXPLAIN's
                // `hashes` counts hashes actually computed for map
                // probes (one branch per sibling per batch).
                let pinned = matches!(s_cursor, Cursor::Row(_));
                counters.phase_start(node_idx, JoinPhase::Hash);
                let n = scratch.survivors.len();
                scratch.hashes.clear();
                scratch.hashes.resize(n, 0);
                // The dominant probe shape — a single batch-sourced key
                // word — takes a match-free specialized loop (docs/perf/
                // PRD 07); everything else takes the general gather.
                let single_batch_word = match scratch.sources[sub_idx].as_slice() {
                    [Source::Batch(word)] if !pinned => Some(*word),
                    _ => None,
                };
                // Alias-hoisted locals (docs/silicon2/07) — see
                // probe_pass; same transform, run_node's sibling shape.
                if let Some(word) = single_batch_word {
                    let survivors = &scratch.survivors[..n];
                    let entry_keys = &scratch.entry_keys[..];
                    let probe_keys = &mut scratch.probe_keys[..n];
                    let hashes = &mut scratch.hashes[..n];
                    for (k, &e) in survivors.iter().enumerate() {
                        let entry = usize::try_from(e).expect("batch fits usize");
                        let key = entry_keys[entry * arity + word];
                        probe_keys[k] = key;
                        counters.probe_hash(node_idx, sub_idx);
                        hashes[k] = crate::exec::colt::hash_key(std::slice::from_ref(&key));
                    }
                } else {
                    let survivors = &scratch.survivors[..n];
                    let entry_keys = &scratch.entry_keys[..];
                    let sources = &scratch.sources[sub_idx];
                    let probe_keys = &mut scratch.probe_keys[..n * sub_arity.max(1)];
                    let hashes = &mut scratch.hashes[..n];
                    for (k, &e) in survivors.iter().enumerate() {
                        let entry = usize::try_from(e).expect("batch fits usize");
                        for i in 0..sub_arity {
                            probe_keys[k * sub_arity + i] =
                                value_of(sources, entry_keys, bindings, entry, i);
                        }
                        if !pinned {
                            counters.probe_hash(node_idx, sub_idx);
                            hashes[k] = crate::exec::colt::hash_key(
                                &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                            );
                        }
                    }
                }
                counters.phase_end(node_idx, JoinPhase::Hash);

                // Phase 1.5 (docs/perf/ PRD 07, re-gated by docs/silicon/
                // 10): the prefetch pass — every bucket the batch will
                // probe gets its ctrl and bucket lines hinted. Gated on
                // RESIDENCY first (an L2-resident map's prefetch is pure
                // loss) and batch width second (tiny batches never
                // amortize the pass).
                if !pinned
                    && scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR
                    && colts[occ].probe_footprint_bytes() > PREFETCH_L2_BUDGET_BYTES
                {
                    crate::obs::event(
                        crate::obs::names::PREFETCH_PASS,
                        crate::obs::Category::Execute,
                        scratch.survivors.len() as u64,
                        colts[occ].probe_footprint_bytes() as u64,
                    );
                    for &hash in &scratch.hashes {
                        colts[occ].prefetch_bucket(s_cursor, hash);
                    }
                }

                // Phase 2: all bucket loads — independent chains the
                // out-of-order window overlaps — then kernel compaction.
                // Alias-hoisted locals (docs/silicon2/07).
                counters.phase_start(node_idx, JoinPhase::Probe);
                scratch.mask.clear();
                scratch.mask.resize(n, 0);
                {
                    let survivors = &scratch.survivors[..n];
                    let probe_keys = &scratch.probe_keys[..n * sub_arity.max(1)];
                    let hashes = &scratch.hashes[..n];
                    let sibling_children = &mut scratch.sibling_children[sub_idx][..];
                    let mask = &mut scratch.mask[..n];
                    let colt = &mut colts[occ];
                    for k in 0..n {
                        let entry = usize::try_from(survivors[k]).expect("batch fits usize");
                        let hit = colt.get_prehashed(
                            s_cursor,
                            s_level,
                            &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                            hashes[k],
                        );
                        counters.probe(node_idx, sub_idx, hit.is_some());
                        sibling_children[entry] = hit.unwrap_or(Cursor::Row(0));
                        mask[k] = u8::from(hit.is_some());
                    }
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
                counters.phase_end(node_idx, JoinPhase::Probe);
            }

            // Residuals run as batch survivor compaction after the probes.
            counters.phase_start(node_idx, JoinPhase::Residual);
            for (r_idx, (lhs_src, rhs_src)) in scratch.residual_sources.iter().enumerate() {
                let op = self.residual_slots[node_idx][r_idx].0.op;
                let n = scratch.survivors.len();
                scratch.mask.clear();
                scratch.mask.resize(n, 0);
                for k in 0..n {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word],
                        Source::Slot(slot) => bindings.get(slot),
                    };
                    let pass = op.compare(&value(lhs_src), &value(rhs_src));
                    counters.residual(node_idx, pass);
                    scratch.mask[k] = u8::from(pass);
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            }
            counters.phase_end(node_idx, JoinPhase::Residual);

            // The leaf (docs/perf/ PRD 01): the last plan node hands its
            // surviving batch to the sink whole. No recursion, no journal,
            // no cursor writes — nothing below reads them — and no
            // binding stores for the leaf's own vars (the batch carries
            // them). `stop_on_skip` folds this node's sink-relevance into
            // the batch call: when the leaf binds nothing sink-relevant,
            // the sink stops at its first emit and the skip unwinds here
            // exactly as the recursive path's absorption arm did.
            if node_idx + 1 == plan.nodes().len() {
                if scratch.survivors.is_empty() {
                    continue;
                }
                counters.phase_start(node_idx, JoinPhase::Descend);
                let batch = LeafBatch {
                    keys: &scratch.entry_keys,
                    arity,
                    survivors: &scratch.survivors,
                    key_slots: &self.slot_map[node_idx][cover_sub],
                    bindings,
                };
                let stop_on_skip = !plan.nodes()[node_idx].sink_relevant && sink.may_skip();
                let batch_flow = sink.emit_batch(&batch, stop_on_skip);
                // EXPLAIN's `emits` counts rows the sink consumed: the
                // whole batch, or exactly one when the first emit's skip
                // stopped it (identical to the recursive path's counts).
                let emitted = if batch_flow == Flow::SkipSuffix {
                    1
                } else {
                    scratch.survivors.len()
                };
                for _ in 0..emitted {
                    counters.emit();
                }
                counters.phase_end(node_idx, JoinPhase::Descend);
                if batch_flow == Flow::SkipSuffix {
                    debug_assert!(
                        sink.may_skip(),
                        "a SkipSuffix crossed a node under a non-skipping sink"
                    );
                    counters.skip(node_idx);
                    flow = Flow::SkipSuffix;
                    break 'outer;
                }
                continue;
            }

            // Middle nodes never reach here (the entry assert): every
            // batch either emitted through the leaf arm above or was
            // empty.
            unreachable!("run_node is the leaf pass; the leaf arm consumed the batch");
        }

        self.scratch[node_idx] = scratch;
        flow
    }

    /// One cross-parent probe pass (docs/perf/ PRD 09): hashes, prefetch,
    /// probes, and residuals run over `fill` elements drawn from many
    /// pending entries; survivors either append to the child's pending
    /// (middle child — flushed when a full batch accumulates) or run the
    /// last node per parent through `run_node`.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::too_many_arguments)]
    fn probe_pass<S: Sink, C: Counters>(
        &mut self,
        tables: &PipeTables,
        plan: &ValidatedPlan,
        node_idx: usize,
        cover_sub: usize,
        arity: usize,
        fill: usize,
        scratch: &mut NodeScratch,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) {
        let n_nodes = plan.nodes().len();
        let slot_count = bindings.slot_count();
        let carried_w = tables.carried[node_idx].len();
        let node = &plan.nodes()[node_idx];
        let cover_occ = usize::from(node.subatoms[cover_sub].occ.0);
        scratch.survivors.clear();
        scratch
            .survivors
            .extend(0..u32::try_from(fill).expect("batch fits u32"));

        // Sibling passes: per-parent Slot reads and per-parent cursors.
        // Instruction diet (docs/silicon/02): value sources resolve once
        // per (pass, subatom) — never a per-element variable search —
        // loop invariants (carried column, start cursor) hoist, and the
        // inner loops write pre-sized buffers by index (a `Vec::push`'s
        // grow branch blocks LICM and unrolling in exactly these loops).
        for sub_idx in 0..node.subatoms.len() {
            if sub_idx == cover_sub || scratch.survivors.is_empty() {
                continue;
            }
            let subatom = &node.subatoms[sub_idx];
            let sub_arity = subatom.vars.len();
            let occ = usize::from(subatom.occ.0);
            let s_level = tables.entry_level[node_idx][occ];
            let cover_vars = &node.subatoms[cover_sub].vars;
            counters.phase_start(node_idx, JoinPhase::Hash);
            scratch.sources[sub_idx].clear();
            for (i, var) in subatom.vars.iter().enumerate() {
                let source = cover_vars.iter().position(|cv| cv == var).map_or(
                    Source::Slot(self.slot_map[node_idx][sub_idx][i]),
                    Source::Batch,
                );
                scratch.sources[sub_idx].push(source);
            }
            let n = scratch.survivors.len();
            scratch.hashes.clear();
            scratch.hashes.resize(n, 0);
            // The dominant shape — one batch-sourced key word — takes a
            // match-free specialized loop, exactly as `run_node`'s
            // sibling pass does.
            let single_batch_word = match scratch.sources[sub_idx].as_slice() {
                [Source::Batch(word)] => Some(*word),
                _ => None,
            };
            // Alias-hoisted locals (docs/silicon2/07, exp 19's follow-up):
            // the loops below interleave reads from some scratch vectors
            // with stores to others — without disjoint pre-loop
            // reborrows, LLVM must reload each Vec's header (ptr/len)
            // every iteration because the stores might alias them
            // (measured 32% of the emulated loop's cost). Disjoint
            // `&mut` field borrows prove non-aliasing; fixed-length
            // slices additionally hoist the bounds checks.
            if let Some(word) = single_batch_word {
                let survivors = &scratch.survivors[..n];
                let entry_keys = &scratch.entry_keys[..];
                let probe_keys = &mut scratch.probe_keys[..n];
                let hashes = &mut scratch.hashes[..n];
                for (k, &e) in survivors.iter().enumerate() {
                    let element = usize::try_from(e).expect("batch fits usize");
                    let key = entry_keys[element * arity + word];
                    probe_keys[k] = key;
                    counters.probe_hash(node_idx, sub_idx);
                    hashes[k] = crate::exec::colt::hash_key(std::slice::from_ref(&key));
                }
            } else {
                let survivors = &scratch.survivors[..n];
                let entry_keys = &scratch.entry_keys[..];
                let parents = &scratch.parents[..];
                let pending_bindings = &scratch.pending_bindings[..];
                let sources = &scratch.sources[sub_idx][..];
                let probe_keys = &mut scratch.probe_keys[..n * sub_arity];
                let hashes = &mut scratch.hashes[..n];
                for (k, &e) in survivors.iter().enumerate() {
                    let element = usize::try_from(e).expect("batch fits usize");
                    let parent = parents[element] as usize;
                    for i in 0..sub_arity {
                        probe_keys[k * sub_arity + i] = match sources[i] {
                            Source::Batch(word) => entry_keys[element * arity + word],
                            Source::Slot(slot) => pending_bindings[parent * slot_count + slot],
                        };
                    }
                    counters.probe_hash(node_idx, sub_idx);
                    hashes[k] = crate::exec::colt::hash_key(
                        &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                    );
                }
            }
            counters.phase_end(node_idx, JoinPhase::Hash);
            let carried = tables.carried_col[node_idx][occ];
            let start_cursor = colts[occ].start();
            // Residency-gated phase 1.5 (docs/silicon/10) — see run_node.
            if scratch.survivors.len() >= PREFETCH_WIDTH_FLOOR
                && colts[occ].probe_footprint_bytes() > PREFETCH_L2_BUDGET_BYTES
            {
                crate::obs::event(
                    crate::obs::names::PREFETCH_PASS,
                    crate::obs::Category::Execute,
                    scratch.survivors.len() as u64,
                    colts[occ].probe_footprint_bytes() as u64,
                );
                for (k, &e) in scratch.survivors.iter().enumerate() {
                    let parent = scratch.parents[e as usize] as usize;
                    let cursor = carried
                        .map_or(start_cursor, |col| {
                            scratch.pending_cursors[parent * carried_w + col]
                        });
                    colts[occ].prefetch_bucket(cursor, scratch.hashes[k]);
                }
            }
            counters.phase_start(node_idx, JoinPhase::Probe);
            scratch.mask.clear();
            scratch.mask.resize(n, 0);
            // The exp-19 shape itself (docs/silicon2/07): reads
            // survivors/parents/pending_cursors/probe_keys/hashes,
            // writes sibling_children/mask — all hoisted to disjoint
            // locals so the stores cannot alias the read headers.
            {
                let survivors = &scratch.survivors[..n];
                let parents = &scratch.parents[..];
                let pending_cursors = &scratch.pending_cursors[..];
                let probe_keys = &scratch.probe_keys[..n * sub_arity];
                let hashes = &scratch.hashes[..n];
                let sibling_children = &mut scratch.sibling_children[sub_idx][..];
                let mask = &mut scratch.mask[..n];
                let colt = &mut colts[occ];
                for k in 0..n {
                    let element = usize::try_from(survivors[k]).expect("batch fits usize");
                    let parent = parents[element] as usize;
                    let cursor = carried
                        .map_or(start_cursor, |col| pending_cursors[parent * carried_w + col]);
                    let hit = colt.get_prehashed(
                        cursor,
                        s_level,
                        &probe_keys[k * sub_arity..(k + 1) * sub_arity],
                        hashes[k],
                    );
                    counters.probe(node_idx, sub_idx, hit.is_some());
                    sibling_children[element] = hit.unwrap_or(Cursor::Row(0));
                    mask[k] = u8::from(hit.is_some());
                }
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
            counters.phase_end(node_idx, JoinPhase::Probe);
            scratch.hashes.clear();
        }

        // Residuals: per-parent Slot reads.
        counters.phase_start(node_idx, JoinPhase::Residual);
        for (residual, lhs_slot, rhs_slot) in &self.residual_slots[node_idx] {
            let cover_vars = &node.subatoms[cover_sub].vars;
            let lhs_word = cover_vars.iter().position(|cv| *cv == residual.lhs);
            let rhs_word = cover_vars.iter().position(|cv| *cv == residual.rhs);
            let n = scratch.survivors.len();
            scratch.mask.clear();
            scratch.mask.resize(n, 0);
            for k in 0..n {
                let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                let parent = scratch.parents[element] as usize;
                let value = |word: Option<usize>, slot: usize| match word {
                    Some(word) => scratch.entry_keys[element * arity + word],
                    None => scratch.pending_bindings[parent * slot_count + slot],
                };
                let pass = residual
                    .op
                    .compare(&value(lhs_word, *lhs_slot), &value(rhs_word, *rhs_slot));
                counters.residual(node_idx, pass);
                scratch.mask[k] = u8::from(pass);
            }
            crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
        }
        counters.phase_end(node_idx, JoinPhase::Residual);

        // Survivor routing. Origins (PRD 10): the absorb node mints one
        // fresh origin per routed survivor — the cancellation unit is
        // exactly "one absorb-element's subtree"; deeper nodes inherit.
        counters.phase_start(node_idx, JoinPhase::Descend);
        let leaf = node_idx + 2 == n_nodes;
        let child_carried = &tables.carried[node_idx + 1];
        let mints_origins = tables.absorb == Some(node_idx);
        for k in 0..scratch.survivors.len() {
            if self.all_cancelled {
                break;
            }
            let element = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
            let parent = scratch.parents[element] as usize;
            let origin = if mints_origins {
                let minted = self.next_origin;
                self.next_origin += 1;
                minted
            } else {
                scratch.element_origins[element]
            };
            // Real origins exist strictly below the absorb node; the
            // seed id above it must never match a minted id.
            if tables.absorb.is_some_and(|a| node_idx > a) && self.origin_cancelled(origin) {
                continue;
            }
            let assemble = |occ: usize| -> Cursor {
                // Advanced at this node: the cover's child or a probed
                // sibling's; otherwise inherited from the parent (or the
                // colt's start when never advanced).
                if occ == cover_occ {
                    return scratch.children[element];
                }
                if let Some(sub_idx) = node
                    .subatoms
                    .iter()
                    .position(|sub| usize::from(sub.occ.0) == occ)
                {
                    debug_assert_ne!(sub_idx, cover_sub, "distinct occs per node");
                    return scratch.sibling_children[sub_idx][element];
                }
                match tables.carried_col[node_idx][occ] {
                    Some(col) => scratch.pending_cursors[parent * carried_w + col],
                    None => colts[occ].start(),
                }
            };
            if leaf {
                // The last node runs per parent through the ordinary
                // machinery: bindings row + cursors restored, then
                // run_node — leaf fast paths, counters, phases and all.
                bindings.load_row(
                    &scratch.pending_bindings[parent * slot_count..(parent + 1) * slot_count],
                );
                for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                    bindings.set(*slot, scratch.entry_keys[element * arity + i]);
                }
                let leaf_node = &plan.nodes()[node_idx + 1];
                for subatom in &leaf_node.subatoms {
                    let occ = usize::from(subatom.occ.0);
                    self.cursors[occ] = (assemble(occ), tables.entry_level[node_idx + 1][occ]);
                }
                let flow = self.run_node(plan, node_idx + 1, colts, bindings, sink, counters);
                if flow == Flow::SkipSuffix {
                    // The leaf skipped (D2): everything descended from
                    // this survivor's origin can only duplicate rows.
                    // The origin is real exactly when this node is at or
                    // below the absorb (minted here or inherited).
                    counters.skip(node_idx);
                    match tables.absorb {
                        Some(a) if node_idx >= a => self.cancel_origin(origin),
                        Some(_) => {}
                        None => self.all_cancelled = true,
                    }
                }
            } else {
                let child = &mut self.scratch[node_idx + 1];
                let start = child.pending_bindings.len();
                child.pending_bindings.extend_from_slice(
                    &scratch.pending_bindings[parent * slot_count..(parent + 1) * slot_count],
                );
                for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                    child.pending_bindings[start + slot] = scratch.entry_keys[element * arity + i];
                }
                for &occ in child_carried {
                    let cursor = assemble(occ);
                    self.scratch[node_idx + 1].pending_cursors.push(cursor);
                }
                self.scratch[node_idx + 1].pending_origins.push(origin);
                self.scratch[node_idx + 1].pending_len += 1;
            }
        }
        counters.phase_end(node_idx, JoinPhase::Descend);
        scratch.parents.clear();
        scratch.element_origins.clear();
        // Cascade at one accumulated batch. Bounded memory: the child
        // holds at most two batches transiently (the 1×batch trigger
        // plus one pass's appends before the next check). The 2×-batch
        // threshold (docs/silicon/14) measured 0.0–0.6% once exp 14
        // priced the per-pass overhead at 11–30 ns — reverted to the
        // simpler contract (docs/silicon2/08).
        if !leaf && self.scratch[node_idx + 1].pending_len >= self.batch {
            self.pump(tables, plan, node_idx + 1, colts, bindings, sink, counters);
        }
    }

    /// The leaf fast paths (docs/perf/ PRD 05). `None` = declined —
    /// multi-position forced nodes the sink cannot scan, sinks without
    /// scan support, byte-column folds — and the generic batch path runs
    /// instead (conservative by construction: correctness never depends
    /// on a fast path firing).
    fn run_leaf_fast<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Option<Flow> {
        let node = &plan.nodes()[node_idx];
        let occ = usize::from(node.subatoms[0].occ.0);
        let (cursor, level) = self.cursors[occ];
        match cursor {
            Cursor::Row(position) => Some(self.run_leaf_pinned(
                plan, node_idx, occ, level, position, colts, bindings, sink, counters,
            )),
            Cursor::Node(_) => self.run_leaf_scan(
                plan, node_idx, occ, level, cursor, colts, bindings, sink, counters,
            ),
        }
    }

    /// The pinned-row arm: a batch of exactly one, with every batch
    /// scaffold skipped — gather, residuals, emit.
    #[allow(clippy::too_many_arguments)] // the run_node context, unpacked
    fn run_leaf_pinned<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        occ: usize,
        level: usize,
        position: u32,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Flow {
        let node = &plan.nodes()[node_idx];
        {
            let key_slots = &self.slot_map[node_idx][0];
            let arity = key_slots.len();
            counters.node_entry(node_idx);
            counters.cover_choice(node_idx, 0, false);
            counters.batch(node_idx, 1);
            counters.phase_start(node_idx, JoinPhase::Descend);
            colts[occ].gather_row(level, position, &mut self.leaf_row[..arity.max(1)]);
            for (idx, (lhs_src, rhs_src)) in self.leaf_residual_sources.iter().enumerate() {
                let value = |src: &Source| match *src {
                    Source::Batch(word) => self.leaf_row[word],
                    Source::Slot(slot) => bindings.get(slot),
                };
                let op = self.residual_slots[node_idx][idx].0.op;
                let pass = op.compare(&value(lhs_src), &value(rhs_src));
                counters.residual(node_idx, pass);
                if !pass {
                    counters.phase_end(node_idx, JoinPhase::Descend);
                    return Flow::Continue;
                }
            }
            let batch = LeafBatch {
                keys: &self.leaf_row,
                arity,
                survivors: &[0],
                key_slots,
                bindings,
            };
            let stop_on_skip = !node.sink_relevant && sink.may_skip();
            let flow = sink.emit_batch(&batch, stop_on_skip);
            counters.emit();
            counters.phase_end(node_idx, JoinPhase::Descend);
            if flow == Flow::SkipSuffix {
                counters.skip(node_idx);
                return Flow::SkipSuffix;
            }
            Flow::Continue
        }
    }

    /// The scan-pushdown arm: positions flow straight from the trie into
    /// the sink's kernels; no key batch exists. Leaf residuals filter
    /// positions per run before the sink sees them; a leaf that could
    /// skip (D2) stays on the batch path.
    #[allow(clippy::too_many_arguments)] // the run_node context, unpacked
    #[allow(clippy::too_many_lines)] // the two measured eval arms are siblings by design
    fn run_leaf_scan<S: Sink, C: Counters>(
        &mut self,
        plan: &ValidatedPlan,
        node_idx: usize,
        occ: usize,
        level: usize,
        cursor: Cursor,
        colts: &mut [Colt],
        bindings: &mut Bindings,
        sink: &mut S,
        counters: &mut C,
    ) -> Option<Flow> {
        let node = &plan.nodes()[node_idx];
        {
            if !colts[occ].suffix_scannable(cursor) || (!node.sink_relevant && sink.may_skip()) {
                return None;
            }
            // Batch-constant residuals (both sides outer) decide the
            // whole leaf at once.
            for (op, lhs, rhs) in &self.leaf_const_residuals {
                if !op.compare(&bindings.get(*lhs), &bindings.get(*rhs)) {
                    counters.node_entry(node_idx);
                    counters.cover_choice(node_idx, 0, false);
                    counters.residual(node_idx, false);
                    return Some(Flow::Continue);
                }
            }
            let scan = LeafScan {
                colt: &colts[occ],
                level,
                key_slots: &self.slot_map[node_idx][0],
                bindings,
            };
            if !sink.begin_scan(&scan) {
                return None;
            }
            counters.node_entry(node_idx);
            counters.cover_choice(node_idx, 0, false);
            counters.phase_start(node_idx, JoinPhase::Descend);
            let n_residuals = self.leaf_scan_residuals.len();
            let mut filtered = std::mem::take(&mut self.scan_filter);
            let drove = scan.colt.for_each_suffix_run(cursor, |run| {
                counters.batch(node_idx, run.len());
                if n_residuals == 0 {
                    sink.scan_run(&scan, run);
                    return;
                }
                // Filter positions through the leaf residuals — run-
                // length-adaptive (see SCAN_HOIST_THRESHOLD): big runs
                // amortize a resolved operand table; small runs resolve
                // per position (both directions measured, both real).
                filtered.clear();
                if run.len() >= SCAN_HOIST_THRESHOLD {
                    assert!(
                        self.leaf_scan_residuals.len() <= MAX_LEAF_RESIDUALS,
                        "leaf residual count exceeds the scan table"
                    );
                    // Option-free prefix table, plain indexed loop
                    // (docs/silicon/08): `array::from_fn` refuses to
                    // inline its element closure (rust-lang/rust#108765)
                    // — this exact table measured ~34 ns/run as a
                    // from_fn-of-Options (eight outlined calls + a 448 B
                    // memcpy, +48 ns/row at fanout runs) vs ~3.4 ns as
                    // straight-line stores. `n_residuals` is the length
                    // prefix; slots past it stay at the placeholder.
                    let placeholder = (
                        crate::ir::CmpOp::Eq,
                        Operand::Const(0),
                        Operand::Const(0),
                    );
                    let mut resolved = [placeholder; MAX_LEAF_RESIDUALS];
                    for (i, (op, lhs, rhs)) in self.leaf_scan_residuals.iter().enumerate() {
                        let side = |src: &Source| match *src {
                            Source::Batch(word) => {
                                Operand::Col(scan.colt.suffix_column(scan.level, word))
                            }
                            Source::Slot(slot) => Operand::Const(bindings.get(slot)),
                        };
                        resolved[i] = (*op, side(lhs), side(rhs));
                    }
                    let mut eval = |position: u32| {
                        for (op, lhs, rhs) in resolved.iter().take(n_residuals) {
                            let value = |operand: &Operand<'_>| match operand {
                                Operand::Col(crate::image::ColumnView::Words(w)) => {
                                    w[position as usize]
                                }
                                Operand::Col(crate::image::ColumnView::Bytes(b)) => {
                                    u64::from(b[position as usize])
                                }
                                Operand::Const(word) => *word,
                            };
                            let pass = op.compare(&value(lhs), &value(rhs));
                            counters.residual(node_idx, pass);
                            if !pass {
                                return false;
                            }
                        }
                        true
                    };
                    push_surviving(run, &mut filtered, &mut eval);
                } else {
                    let mut eval = |position: u32| {
                        for (op, lhs_src, rhs_src) in &self.leaf_scan_residuals {
                            let value = |src: &Source| match *src {
                                Source::Batch(word) => {
                                    match scan.colt.suffix_column(scan.level, word) {
                                        crate::image::ColumnView::Words(w) => w[position as usize],
                                        crate::image::ColumnView::Bytes(b) => {
                                            u64::from(b[position as usize])
                                        }
                                    }
                                }
                                Source::Slot(slot) => bindings.get(slot),
                            };
                            let pass = op.compare(&value(lhs_src), &value(rhs_src));
                            counters.residual(node_idx, pass);
                            if !pass {
                                return false;
                            }
                        }
                        true
                    };
                    push_surviving(run, &mut filtered, &mut eval);
                }
                if !filtered.is_empty() {
                    sink.scan_run(&scan, crate::exec::colt::SuffixRun::Positions(&filtered));
                }
            });
            debug_assert!(drove, "suffix_scannable pre-checked");
            let emitted = sink.end_scan(&scan);
            for _ in 0..emitted {
                counters.emit();
            }
            counters.phase_end(node_idx, JoinPhase::Descend);
            self.scan_filter = filtered;
            Some(Flow::Continue)
        }
    }

    /// Chooses the cover with the fewest keys: smallest `Exact` wins;
    /// otherwise the smallest `Estimate` (v0 rule, 30-execution).
    fn choose_cover(&self, plan: &ValidatedPlan, node_idx: usize, colts: &[Colt]) -> usize {
        let node = &plan.nodes()[node_idx];
        let mut best: Option<(usize, KeyCount)> = None;
        for &cover in &node.covers {
            let sub_idx = usize::from(cover);
            let occ = usize::from(node.subatoms[sub_idx].occ.0);
            let count = colts[occ].key_count(self.cursors[occ].0);
            let better = match &best {
                None => true,
                Some((_, incumbent)) => better_cover(count, *incumbent),
            };
            if better {
                best = Some((sub_idx, count));
            }
        }
        best.expect("validated plans have non-empty cover sets").0
    }
}

/// The magnitude-first cover rule (docs/architecture/30-execution.md): iterating a cover
/// costs O(its keys) plus a probe into every other subatom per key, and
/// both labels are admissible bounds on that cost — an `Estimate`
/// (unforced position count) is exact iteration cost pre-force and an
/// upper bound on post-force keys. So the smaller magnitude wins
/// regardless of label; on a tie, `Exact` wins (it cannot shrink); a
/// full tie keeps the incumbent (lowest subatom index — deterministic).
/// The old rule — "an Exact always displaces an Estimate" — iterated a
/// 500-key forced map while a 7-row param-filtered view sat unforced
/// beside it: the measured wrong-cover in the balance family.
fn better_cover(candidate: KeyCount, incumbent: KeyCount) -> bool {
    let (n, b) = (candidate.magnitude(), incumbent.magnitude());
    n < b
        || (n == b
            && matches!(candidate, KeyCount::Exact(_))
            && matches!(incumbent, KeyCount::Estimate(_)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::image::view::apply;
    use crate::ir::normalize::{NormalizedQuery, OccId, Occurrence, PlacedComparison};
    use crate::ir::{CmpOp, VarId};
    use crate::plan::fj::{binary2fj, factor, validate, ValidatedPlan};
    use crate::plan::planner::JoinOrder;
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, Schema,
        SchemaDescriptor,
    };
    use crate::storage::commit::commit;
    use crate::storage::delta::WriteDelta;
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;
    use std::collections::BTreeSet;
    use std::sync::Arc;

    /// A sink collecting distinct full binding tuples (set semantics).
    #[derive(Default)]
    struct CollectSink {
        rows: BTreeSet<Vec<u64>>,
    }

    impl Sink for CollectSink {
        fn emit(&mut self, bindings: &Bindings) -> Flow {
            let row: Vec<u64> = (0..bindings.slot_count())
                .map(|s| bindings.get(s))
                .collect();
            self.rows.insert(row);
            Flow::Continue
        }

        fn emit_batch(&mut self, batch: &LeafBatch<'_>, stop_on_skip: bool) -> Flow {
            debug_assert!(!stop_on_skip, "CollectSink never skips");
            for &entry in batch.survivors {
                let row: Vec<u64> = (0..batch.bindings.slot_count())
                    .map(|slot| match batch.source_of(slot) {
                        LeafSource::Key(word) => batch.key(entry, word),
                        LeafSource::Outer => batch.bindings.get(slot),
                    })
                    .collect();
                self.rows.insert(row);
            }
            Flow::Continue
        }
    }

    /// Counters recording cover choices for the skew assertion.
    #[derive(Default)]
    struct RecordingCounters {
        cover_choices: Vec<(usize, usize, bool)>,
    }

    impl Counters for RecordingCounters {
        fn node_entry(&mut self, _: usize) {}
        fn batch(&mut self, _: usize, _: usize) {}
        fn cover_choice(&mut self, node: usize, subatom: usize, exact: bool) {
            self.cover_choices.push((node, subatom, exact));
        }
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    /// Builds a schema of binary U64 relations R0..Rn(a, b).
    fn schema(relations: usize) -> Schema {
        SchemaDescriptor {
            relations: (0..relations)
                .map(|r| RelationDescriptor {
                    name: format!("R{r}").into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "a".into(),
                            value_type: crate::schema::ValueType::U64,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "b".into(),
                            value_type: crate::schema::ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![],
                })
                .collect(),
        }
        .validate()
        .expect("valid fixture")
    }

    /// Commits word rows into each relation and returns unfiltered views.
    fn views_of(
        dir: &TempDir,
        schema: &Schema,
        data: &[Vec<(u64, u64)>],
    ) -> Vec<Arc<crate::image::RelationImage>> {
        let env = Environment::create(dir.path(), schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (rel, rows) in data.iter().enumerate() {
            let rel_id = RelationId(u32::try_from(rel).expect("small"));
            for (a, b) in rows {
                let mut bytes = Vec::new();
                encode_fact(
                    &[ValueRef::U64(*a), ValueRef::U64(*b)],
                    schema.relation(rel_id).layout(),
                    &mut bytes,
                );
                delta.insert(&view, rel_id, &bytes).expect("insert");
            }
        }
        drop(view);
        commit(delta, &env).expect("commit");
        let txn = env.read_txn().expect("txn");
        (0..data.len())
            .map(|rel| {
                let rel_id = RelationId(u32::try_from(rel).expect("small"));
                crate::image::build(&txn, schema, rel_id).expect("build")
            })
            .collect()
    }

    /// COLT sources for a plan: schema columns from each occurrence's trie
    /// schema and var-to-field map.
    fn colts_for(plan: &ValidatedPlan, images: &[Arc<crate::image::RelationImage>]) -> Vec<Colt> {
        plan.occurrences()
            .iter()
            .map(|occurrence| {
                let columns: Vec<Vec<usize>> = occurrence
                    .trie_schema
                    .iter()
                    .map(|level| {
                        level
                            .iter()
                            .map(|var| {
                                let (field, _) = occurrence
                                    .vars
                                    .iter()
                                    .find(|(_, v)| v == var)
                                    .expect("plan vars come from the occurrence");
                                usize::from(field.0)
                            })
                            .collect()
                    })
                    .collect();
                Colt::new(
                    apply(
                        &images[usize::try_from(occurrence.relation.0).expect("small")],
                        &[],
                        &[],
                        Vec::new(),
                    ),
                    &[],
                    columns,
                )
            })
            .collect()
    }

    fn occurrence(occ: u16, relation: u32, vars: &[(u16, u16)]) -> Occurrence {
        Occurrence {
            occ_id: OccId(occ),
            relation: RelationId(relation),
            vars: vars.iter().map(|(f, v)| (FieldId(*f), VarId(*v))).collect(),
            filters: vec![],
        }
    }

    fn planned(normalized: &NormalizedQuery, schema: &Schema, order: &[u16]) -> ValidatedPlan {
        planned_with_sinks(normalized, schema, order, &BTreeSet::new())
    }

    /// A plan with explicit sink vars — all-vars sets make every node
    /// sink-relevant, i.e. skip-free: the pipelined executor's shapes
    /// (docs/perf/ PRD 09).
    fn planned_with_sinks(
        normalized: &NormalizedQuery,
        schema: &Schema,
        order: &[u16],
        sinks: &BTreeSet<VarId>,
    ) -> ValidatedPlan {
        let join_order = JoinOrder {
            order: order.iter().map(|o| OccId(*o)).collect(),
            estimates: vec![0; order.len()],
        };
        let mut plan = binary2fj(normalized, &join_order);
        factor(&mut plan);
        validate(&plan, normalized, schema, vec![0; order.len()], sinks).expect("valid plan")
    }

    /// All the query's vars — the skip-free sink set.
    fn all_vars(normalized: &NormalizedQuery) -> BTreeSet<VarId> {
        normalized
            .occurrences
            .iter()
            .flat_map(|o| o.vars.iter().map(|(_, v)| *v))
            .collect()
    }

    fn run(plan: &ValidatedPlan, views: &[Arc<crate::image::RelationImage>]) -> BTreeSet<Vec<u64>> {
        let mut colts = colts_for(plan, views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut executor = Executor::new(plan);
        executor.execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        );
        sink.rows
    }

    /// Counters recording D2 skips (pipeline flavor).
    #[derive(Default)]
    struct SkipCounterRun {
        skips: usize,
    }

    impl Counters for SkipCounterRun {
        fn batch(&mut self, _: usize, _: usize) {}
        fn node_entry(&mut self, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, _: usize, _: usize) {}
        fn probe(&mut self, _: usize, _: usize, _: bool) {}
        fn residual(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {
            self.skips += 1;
        }
    }

    /// The real projection sink, re-exported for pipeline D2 tests.
    use crate::exec::sink::ProjectionSink as ProjectionSinkForTest;

    trait FirstCol {
        fn rows_first_col(&self) -> Vec<u64>;
    }
    impl FirstCol for ProjectionSinkForTest {
        fn rows_first_col(&self) -> Vec<u64> {
            self.rows().map(|r| r[0]).collect()
        }
    }

    /// PRD 10 (docs/perf/): D2 under the pipeline — two parents
    /// interleave in one batch, one parent's suffix skips, and the other
    /// parent's rows all emit. The absorb node sits above a
    /// non-sink-relevant middle node, so cancellation crosses a level.
    #[test]
    fn pipelined_d2_cancels_one_origin_and_spares_the_rest() {
        let dir = TempDir::new("run-pipe-d2");
        let schema = schema(3);
        // R(x, y): two x groups fan out through y; S(y, z) multiplies
        // witnesses; T(z, w) leaf binds nothing projected. Projecting x
        // only: n0 (binds x, y? — order [0,1,2] makes n0 bind x,y) is
        // sink-relevant via x; n1 (z) and n2 (w) are not — a leaf skip
        // cancels one n0-element subtree.
        let r: Vec<(u64, u64)> = vec![(1, 10), (1, 11), (2, 10)];
        let s: Vec<(u64, u64)> = (0..40).map(|i| (10 + (i % 2), i)).collect();
        let t: Vec<(u64, u64)> = (0..40).map(|i| (i, 900 + i)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
            residuals: vec![],
        };
        // Sink vars: x only.
        let sinks: BTreeSet<VarId> = [VarId(0)].into();
        let plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
        assert!(!plan.skip_free(), "the D2 shape");
        for batch in [1usize, 2, 128] {
            let mut executor = Executor::with_batch_size(&plan, batch);
            let mut colts = colts_for(&plan, &views);
            let mut bindings = Bindings::new(plan.slots().len());
            let mut sink = ProjectionSinkForTest::new(vec![plan.slot_of(VarId(0))]);
            let mut counters = SkipCounterRun::default();
            executor.execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);
            let mut rows: Vec<u64> = sink.rows_first_col();
            rows.sort_unstable();
            assert_eq!(rows, vec![1, 2], "batch {batch}: both x groups present");
            assert!(counters.skips > 0, "batch {batch}: skips fired");
        }
    }

    /// PRD 10's randomized differential: subset projections force real
    /// D2 skips through the pipeline — random instances, orders, and
    /// batch sizes against the nested-loop oracle's projected sets.
    /// (This is the harness specified to catch origin-tagging bugs.)
    #[test]
    #[allow(clippy::too_many_lines)] // three shapes, three oracles, one sweep
    fn randomized_subset_projections_match_the_oracle_under_d2() {
        let mut state = 0xBEEF_CAFE_1234_5678u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state >> 33
        };
        let schema = schema(3);
        for case in 0..200u32 {
            let domain = 1 + next() % 6;
            let mut data: Vec<Vec<(u64, u64)>> = Vec::new();
            for _ in 0..3 {
                let rows = 1 + next() % 30;
                let mut rel = Vec::new();
                for _ in 0..rows {
                    rel.push((next() % domain, next() % domain));
                }
                rel.sort_unstable();
                rel.dedup();
                data.push(rel);
            }
            let dir = TempDir::new(&format!("run-d2-diff-{case}"));
            let views = views_of(&dir, &schema, &data);
            let shape = case % 3;
            let occurrences = match shape {
                0 => vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    occurrence(1, 1, &[(0, 1), (1, 2)]),
                ],
                1 => vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    occurrence(1, 1, &[(0, 1), (1, 2)]),
                    occurrence(2, 2, &[(0, 0), (1, 2)]),
                ],
                _ => vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    occurrence(1, 1, &[(0, 1), (1, 2)]),
                    occurrence(2, 2, &[(0, 2), (1, 3)]),
                ],
            };
            let n_vars = if shape == 2 { 4u16 } else { 3 };
            let n = occurrences.len();
            let normalized = NormalizedQuery {
                occurrences,
                residuals: vec![],
            };
            let mut order: Vec<u16> = (0..u16::try_from(n).expect("small")).collect();
            for i in (1..order.len()).rev() {
                let j = usize::try_from(next()).expect("64-bit") % (i + 1);
                order.swap(i, j);
            }
            // Project a random nonempty strict subset of the vars.
            let keep: Vec<VarId> = (0..n_vars).filter(|_| next() % 2 == 0).map(VarId).collect();
            let keep = if keep.is_empty() || keep.len() == usize::from(n_vars) {
                vec![VarId(0)]
            } else {
                keep
            };
            let sinks: BTreeSet<VarId> = keep.iter().copied().collect();
            let plan = planned_with_sinks(&normalized, &schema, &order, &sinks);

            // Oracle: full joins, then project.
            let mut expected: BTreeSet<Vec<u64>> = BTreeSet::new();
            let full = |expected: &mut BTreeSet<Vec<u64>>, vals: &[u64]| {
                expected.insert(keep.iter().map(|v| vals[usize::from(v.0)]).collect());
            };
            match shape {
                0 => {
                    for (a, b) in &data[0] {
                        for (c, d) in &data[1] {
                            if b == c {
                                full(&mut expected, &[*a, *b, *d]);
                            }
                        }
                    }
                }
                1 => {
                    for (a, b) in &data[0] {
                        for (c, d) in &data[1] {
                            for (e, g) in &data[2] {
                                if b == c && a == e && d == g {
                                    full(&mut expected, &[*a, *b, *d]);
                                }
                            }
                        }
                    }
                }
                _ => {
                    for (a, b) in &data[0] {
                        for (c, d) in &data[1] {
                            for (e, g) in &data[2] {
                                if b == c && d == e {
                                    full(&mut expected, &[*a, *b, *d, *g]);
                                }
                            }
                        }
                    }
                }
            }
            for batch in [1usize, 7, 128] {
                let mut executor = Executor::with_batch_size(&plan, batch);
                let mut colts = colts_for(&plan, &views);
                let mut bindings = Bindings::new(plan.slots().len());
                let mut sink =
                    ProjectionSinkForTest::new(keep.iter().map(|v| plan.slot_of(*v)).collect());
                executor.execute(
                    &plan,
                    &mut colts,
                    &mut bindings,
                    &mut sink,
                    &mut NoopCounters,
                );
                let got: BTreeSet<Vec<u64>> = sink.rows().map(<[u64]>::to_vec).collect();
                assert_eq!(
                    got, expected,
                    "case {case} shape {shape} order {order:?} keep {keep:?} batch {batch}"
                );
            }
        }
    }

    /// PRD 09 (docs/perf/): the pipelined executor — dispatched exactly
    /// for skip-free plans with middle nodes — matches the recursive
    /// executor and the nested-loop oracle bit for bit, across batch
    /// sizes that stress fill boundaries (pending exactly at, one under,
    /// and far over the batch), multi-batch expansions with resume
    /// tokens, empty covers, and duplicate-heavy skew.
    #[test]
    fn pipelined_executor_matches_recursive_and_oracle() {
        let _dir = TempDir::new("run-pipeline-equiv");
        let schema = schema(3);
        // Chain shape with heavy fanout at every step; sizes cross the
        // 128 batch on both sides.
        for (n_r, n_s, n_t) in [(127u64, 128, 129), (5, 300, 40), (1, 1, 1), (200, 0, 10)] {
            let r: Vec<(u64, u64)> = (0..n_r).map(|i| (i % 13, i % 7)).collect();
            let s: Vec<(u64, u64)> = (0..n_s).map(|i| (i % 7, i % 11)).collect();
            let t: Vec<(u64, u64)> = (0..n_t).map(|i| (i % 11, i)).collect();
            let mut r = r;
            r.sort_unstable();
            r.dedup();
            let mut s = s;
            s.sort_unstable();
            s.dedup();
            let mut t = t;
            t.sort_unstable();
            t.dedup();
            let dir2 = TempDir::new(&format!("run-pipeline-{n_r}-{n_s}-{n_t}"));
            let views = views_of(&dir2, &schema, &[r.clone(), s.clone(), t.clone()]);
            let normalized = NormalizedQuery {
                occurrences: vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    occurrence(1, 1, &[(0, 1), (1, 2)]),
                    occurrence(2, 2, &[(0, 2), (1, 3)]),
                ],
                residuals: vec![PlacedComparison {
                    op: CmpOp::Ne,
                    lhs: VarId(0),
                    rhs: VarId(3),
                }],
            };
            let sinks = all_vars(&normalized);
            let pipe_plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
            assert!(pipe_plan.skip_free(), "all-vars projections are skip-free");
            let rec_plan = planned(&normalized, &schema, &[0, 1, 2]);
            assert!(!rec_plan.skip_free());

            let mut expected = BTreeSet::new();
            for (rx, ry) in &r {
                for (sy, sz) in &s {
                    for (tz, tw) in &t {
                        if ry == sy && sz == tz && rx != tw {
                            expected.insert(vec![*rx, *ry, *sz, *tw]);
                        }
                    }
                }
            }
            for batch in [1usize, 2, 127, 128, 129, 1024] {
                let mut executor = Executor::with_batch_size(&pipe_plan, batch);
                assert!(executor.pipe.is_some(), "pipeline dispatched");
                let mut colts = colts_for(&pipe_plan, &views);
                let mut bindings = Bindings::new(pipe_plan.slots().len());
                let mut sink = CollectSink::default();
                executor.execute(
                    &pipe_plan,
                    &mut colts,
                    &mut bindings,
                    &mut sink,
                    &mut NoopCounters,
                );
                let got: BTreeSet<Vec<u64>> = sink
                    .rows
                    .iter()
                    .map(|row| {
                        (0..4u16)
                            .map(|v| row[pipe_plan.slot_of(VarId(v))])
                            .collect::<Vec<u64>>()
                    })
                    .collect();
                assert_eq!(got, expected, "sizes ({n_r},{n_s},{n_t}) batch {batch}");
            }
        }
    }

    /// PRD 09's counter-proven batching: a triangle-shaped skip-free plan
    /// whose middle node used to probe once per parent now probes in
    /// cross-parent batches with mean length well above the gate.
    #[test]
    fn pipelined_middle_nodes_probe_in_cross_parent_batches() {
        #[derive(Default)]
        struct ProbeBatches {
            passes: usize,
            probes: usize,
            current: usize,
            node: usize,
        }
        impl Counters for ProbeBatches {
            fn node_entry(&mut self, _: usize) {}
            fn batch(&mut self, _: usize, _: usize) {}
            fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
            fn probe_hash(&mut self, _: usize, _: usize) {}
            fn probe(&mut self, node: usize, _: usize, _: bool) {
                if node == self.node {
                    self.probes += 1;
                    self.current += 1;
                }
            }
            fn residual(&mut self, _: usize, _: bool) {}
            fn emit(&mut self) {}
            fn skip(&mut self, _: usize) {}
            fn phase_start(&mut self, node: usize, phase: JoinPhase) {
                if node == self.node && phase == JoinPhase::Probe {
                    self.current = 0;
                }
            }
            fn phase_end(&mut self, node: usize, phase: JoinPhase) {
                if node == self.node && phase == JoinPhase::Probe && self.current > 0 {
                    self.passes += 1;
                }
            }
        }

        let dir = TempDir::new("run-pipeline-batching");
        let schema = schema(3);
        // R fans out 1000 parents; the middle node probes S per parent —
        // fanout 1 each — the exact starvation shape.
        let r: Vec<(u64, u64)> = (0..1000).map(|i| (i % 4, i)).collect();
        let s: Vec<(u64, u64)> = (0..1000).map(|i| (i, i % 5)).collect();
        let t: Vec<(u64, u64)> = (0..5).map(|i| (i, i)).collect();
        let views = views_of(&dir, &schema, &[r, s, t]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
            residuals: vec![],
        };
        let sinks = all_vars(&normalized);
        let plan = planned_with_sinks(&normalized, &schema, &[0, 1, 2], &sinks);
        assert!(plan.skip_free());
        let mut executor = Executor::new(&plan);
        assert!(executor.pipe.is_some());
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut counters = ProbeBatches {
            node: 1,
            ..Default::default()
        };
        executor.execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);
        assert!(!sink.rows.is_empty());
        assert!(counters.passes > 0);
        let mean = counters.probes / counters.passes;
        assert!(
            mean >= 32,
            "middle-node probes batch across parents: mean {mean} (probes {}, passes {})",
            counters.probes,
            counters.passes
        );

        // The memory bound: pending buffers never exceed two batches.
        for scratch in &executor.scratch {
            assert!(
                scratch.pending_bindings.capacity()
                    <= 2 * BATCH * plan.slots().len() + plan.slots().len()
            );
        }
    }

    /// The clover query over the paper's Fig. 4 instance: only
    /// (x0, a0, b0, c0) joins.
    #[test]
    fn clover_on_the_papers_instance() {
        let dir = TempDir::new("run-clover");
        let schema = schema(3);
        let n = 20u64;
        // R = {(x0,a0)} u {(x1,ai_l), (x2,ai_r)}; S, T rotated (Fig. 4).
        // Encode x0..x3 as 0..3 and the a/b/c values as 100+i / 200+i.
        let mut r = vec![(0, 100)];
        let mut s = vec![(0, 200)];
        let mut t = vec![(0, 300)];
        for i in 1..=n {
            r.push((1, 100 + i));
            r.push((2, 100 + n + i));
            s.push((2, 200 + i));
            s.push((3, 200 + n + i));
            t.push((3, 300 + i));
            t.push((1, 300 + n + i));
        }
        let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

        // Q(x,a,b,c) :- R(x,a), S(x,b), T(x,c).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
                occurrence(2, 2, &[(0, 0), (1, 3)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let results = run(&plan, &views);

        // Naive oracle: triple loop.
        let mut expected = BTreeSet::new();
        for (rx, ra) in &r {
            for (sx, sb) in &s {
                for (tx, tc) in &t {
                    if rx == sx && sx == tx {
                        expected.insert(vec![*rx, *ra, *sb, *tc]);
                    }
                }
            }
        }
        assert_eq!(results, expected);
        assert_eq!(results.len(), 1, "only the center of the clover joins");
    }

    #[test]
    fn chain_query_matches_the_nested_loop_oracle() {
        let dir = TempDir::new("run-chain");
        let schema = schema(3);
        let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i + 1)).collect();
        let s: Vec<(u64, u64)> = (0..10).map(|i| (i + 1, i + 2)).collect();
        let t: Vec<(u64, u64)> = (0..10).map(|i| (i + 2, i + 3)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

        // Q(x,y,z,w) :- R(x,y), S(y,z), T(z,w).
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let results = run(&plan, &views);

        let mut expected = BTreeSet::new();
        for (rx, ry) in &r {
            for (sy, sz) in &s {
                for (tz, tw) in &t {
                    if ry == sy && sz == tz {
                        expected.insert(vec![*rx, *ry, *sz, *tw]);
                    }
                }
            }
        }
        assert_eq!(results, expected);
        assert!(!results.is_empty());
    }

    #[test]
    fn self_join_grandparent() {
        let dir = TempDir::new("run-grandparent");
        let schema = schema(1);
        // OrgParent(child, parent): 0->1->2->3 plus a fork 4->1.
        let edges = vec![(0u64, 1u64), (1, 2), (2, 3), (4, 1)];
        let views = views_of(&dir, &schema, std::slice::from_ref(&edges));

        // Grandparent(c, g) :- OrgParent(c, p), OrgParent(p, g) — two
        // occurrences of relation 0.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 0, &[(0, 1), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        // Both occurrences read relation 0: views vector must be indexed by
        // occurrence, not relation — build colts by occurrence's relation.
        let results = run(&plan, &views);

        let mut expected = BTreeSet::new();
        for (c, p) in &edges {
            for (p2, g) in &edges {
                if p == p2 {
                    expected.insert(vec![*c, *p, *g]);
                }
            }
        }
        assert_eq!(results, expected);
        assert_eq!(results.len(), 3); // 0->1->2, 1->2->3, 4->1->2
    }

    #[test]
    fn triangle_is_wcoj_honest() {
        let dir = TempDir::new("run-triangle");
        let schema = schema(3);
        // R(x,y), S(y,z), T(z,x) over a small dense instance.
        let r: Vec<(u64, u64)> = (0..6).flat_map(|x| (0..6).map(move |y| (x, y))).collect();
        let s: Vec<(u64, u64)> = (0..6).map(|y| (y, (y + 1) % 6)).collect();
        let t: Vec<(u64, u64)> = (0..6).map(|z| (z, (z + 2) % 6)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone(), t.clone()]);

        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 0)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let results = run(&plan, &views);

        let mut expected = BTreeSet::new();
        for (rx, ry) in &r {
            for (sy, sz) in &s {
                for (tz, tx) in &t {
                    if ry == sy && sz == tz && tx == rx {
                        expected.insert(vec![*rx, *ry, *sz]);
                    }
                }
            }
        }
        assert_eq!(results, expected);
        assert!(!results.is_empty());
    }

    #[test]
    fn zero_binding_atom_gates_the_query() {
        let dir = TempDir::new("run-gate");
        let schema = schema(2);
        let r = vec![(1u64, 2u64), (3, 4)];
        // Gate nonempty: results flow; gate empty: nothing.
        for (gate_rows, expect_rows) in [(vec![(9u64, 9u64)], 2usize), (vec![], 0)] {
            let dir2 = TempDir::new(&format!("run-gate-{expect_rows}"));
            let views = views_of(&dir2, &schema, &[r.clone(), gate_rows]);
            let normalized = NormalizedQuery {
                occurrences: vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    Occurrence {
                        occ_id: OccId(1),
                        relation: RelationId(1),
                        vars: vec![],
                        filters: vec![],
                    },
                ],
                residuals: vec![],
            };
            let plan = planned(&normalized, &schema, &[0, 1]);
            let results = run(&plan, &views);
            assert_eq!(results.len(), expect_rows, "gate case {expect_rows}");
        }
        drop(dir);
    }

    #[test]
    fn empty_relations_yield_empty_results() {
        let dir = TempDir::new("run-empty");
        let schema = schema(2);
        let views = views_of(&dir, &schema, &[vec![(1, 2)], vec![]]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        assert!(run(&plan, &views).is_empty());
    }

    #[test]
    fn duplicate_heavy_skew_collapses_to_the_distinct_binding_set() {
        let dir = TempDir::new("run-skew");
        let schema = schema(2);
        // Heavy duplication in the join column (post-collapse the binding
        // set is small).
        let r: Vec<(u64, u64)> = (0..50).map(|i| (i % 2, i % 3)).collect();
        let s: Vec<(u64, u64)> = (0..50).map(|i| (i % 3, i % 5)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let results = run(&plan, &views);
        let mut expected = BTreeSet::new();
        for (ra, rb) in &r {
            for (sa, sb) in &s {
                if rb == sa {
                    expected.insert(vec![*ra, *rb, *sb]);
                }
            }
        }
        assert_eq!(results, expected);
    }

    #[test]
    fn residuals_filter_across_atoms() {
        let dir = TempDir::new("run-residuals");
        let schema = schema(2);
        let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i)).collect();
        let s: Vec<(u64, u64)> = (0..10).map(|i| (i, 9 - i)).collect();
        let views = views_of(&dir, &schema, &[r.clone(), s.clone()]);
        // R(x, a), S(x, b), a < b.
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![PlacedComparison {
                op: CmpOp::Lt,
                lhs: VarId(1),
                rhs: VarId(2),
            }],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let results = run(&plan, &views);
        let mut expected = BTreeSet::new();
        for (rx, ra) in &r {
            for (sx, sb) in &s {
                if rx == sx && ra < sb {
                    expected.insert(vec![*rx, *ra, *sb]);
                }
            }
        }
        assert_eq!(results, expected);
        assert_eq!(results.len(), 5); // i in 0..=4: i < 9-i
    }

    #[test]
    fn dynamic_cover_prefers_the_forced_small_side() {
        let dir = TempDir::new("run-cover-choice");
        let schema = schema(2);
        // R: huge with duplicate x; S: tiny. Node 0 = [R(x), S(x)] via a
        // GJ-style hand plan where both are covers.
        let r: Vec<(u64, u64)> = (0..500).map(|i| (i % 250, i)).collect();
        let s: Vec<(u64, u64)> = vec![(0, 0), (1, 1)];
        let views = views_of(&dir, &schema, &[r, s]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        // Hand-build the GJ plan: [[R(x), S(x)], [R(a)], [S(b)]].
        let plan = crate::plan::fj::FjPlan {
            nodes: vec![
                crate::plan::fj::Node {
                    subatoms: vec![
                        crate::plan::fj::Subatom {
                            occ: OccId(0),
                            vars: vec![VarId(0)],
                        },
                        crate::plan::fj::Subatom {
                            occ: OccId(1),
                            vars: vec![VarId(0)],
                        },
                    ],
                },
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(0),
                        vars: vec![VarId(1)],
                    }],
                },
                crate::plan::fj::Node {
                    subatoms: vec![crate::plan::fj::Subatom {
                        occ: OccId(1),
                        vars: vec![VarId(2)],
                    }],
                },
            ],
        };
        let plan = validate(&plan, &normalized, &schema, vec![0; 3], &BTreeSet::new())
            .expect("valid plan");

        // Pre-force S's root so its Exact(2) beats R's Estimate(500).
        let mut colts = colts_for(&plan, &views);
        let s_root = Colt::root();
        colts[1].get(s_root, 0, &[0]);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut counters = RecordingCounters::default();
        Executor::new(&plan).execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);

        // Node 0's first choice: subatom 1 (S), whose count is Exact.
        let (node, subatom, exact) = counters.cover_choices[0];
        assert_eq!((node, subatom, exact), (0, 1, true));
        assert!(!sink.rows.is_empty());
    }

    /// Regression for the cover-soundness deviation
    /// (docs/architecture/30-execution.md): a subatom carrying an
    /// already-bound variable must never be a runtime-eligible cover. In
    /// the triangle below, node 1 = [S(z), T(x, z)]; with skew, T's tiny
    /// key count would win the dynamic choice, and iterating T(x, z)
    /// rebinds x over R's binding without re-probing R — producing a row
    /// where the correct answer is empty.
    #[test]
    fn covers_never_rebind_an_already_bound_variable() {
        let dir = TempDir::new("run-cover-rebind");
        let schema = schema(3);
        let r = vec![(1, 1)];
        let s: Vec<(u64, u64)> = (0..100).map(|z| (1, z)).collect();
        let t = vec![(2, 5)];
        let views = views_of(&dir, &schema, &[r, s, t]);

        // Q(x,y,z) :- R(x,y), S(y,z), T(x,z), order [R, S, T].
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);

        // The mixed-var subatom T(x, z) must not be listed as a cover of
        // its node (x is bound by node 0).
        for node in plan.nodes() {
            for &cover in &node.covers {
                let vars = &node.subatoms[cover as usize].vars;
                assert_eq!(
                    vars.len(),
                    node.new_vars.len(),
                    "a cover must bind exactly the node's new vars"
                );
            }
        }

        let results = run(&plan, &views);
        assert!(
            results.is_empty(),
            "T binds x=2, R binds x=1: joining them must be empty, got {results:?}"
        );
    }

    #[test]
    fn backtracking_restores_sources_across_sequential_executions() {
        let dir = TempDir::new("run-backtrack");
        let schema = schema(2);
        let r: Vec<(u64, u64)> = (0..20).map(|i| (i % 4, i)).collect();
        let s: Vec<(u64, u64)> = (0..4).map(|i| (i, i * 10)).collect();
        let views = views_of(&dir, &schema, &[r, s]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut executor = Executor::new(&plan);

        let mut first = CollectSink::default();
        executor.execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut first,
            &mut NoopCounters,
        );
        let mut second = CollectSink::default();
        executor.execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut second,
            &mut NoopCounters,
        );
        assert_eq!(first.rows, second.rows);
        assert!(!first.rows.is_empty());
    }
    // ---------- the 30-execution doc: vectorized execution ----------

    /// Runs a plan at a given batch size.
    fn run_batched(
        plan: &ValidatedPlan,
        views: &[Arc<crate::image::RelationImage>],
        batch: usize,
    ) -> BTreeSet<Vec<u64>> {
        let mut colts = colts_for(plan, views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut executor = Executor::with_batch_size(plan, batch);
        executor.execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        );
        sink.rows
    }

    #[test]
    fn results_are_identical_across_batch_sizes() {
        // Skew, empty relations, partial final batches, and batch > row
        // count are all covered by these fixtures x sizes.
        let dir = TempDir::new("run-batch-equality");
        let schema = schema(3);
        let r: Vec<(u64, u64)> = (0..150).map(|i| (i % 7, i % 11)).collect();
        let s: Vec<(u64, u64)> = (0..90).map(|i| (i % 11, i % 5)).collect();
        let t: Vec<(u64, u64)> = (0..40).map(|i| (i % 5, i)).collect();
        let views = views_of(&dir, &schema, &[r, s, t]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 1), (1, 2)]),
                occurrence(2, 2, &[(0, 2), (1, 3)]),
            ],
            residuals: vec![PlacedComparison {
                op: CmpOp::Ne,
                lhs: VarId(0),
                rhs: VarId(3),
            }],
        };
        let plan = planned(&normalized, &schema, &[0, 1, 2]);
        let reference = run_batched(&plan, &views, 1);
        assert!(!reference.is_empty());
        for batch in [2usize, 64, 128, 1024] {
            assert_eq!(
                run_batched(&plan, &views, batch),
                reference,
                "batch size {batch} must match the scalar degenerate case"
            );
        }

        // An empty relation, every batch size.
        let dir2 = TempDir::new("run-batch-empty");
        let views = views_of(&dir2, &schema, &[vec![(1, 2)], vec![], vec![(0, 0)]]);
        for batch in [1usize, 2, 64, 128, 256, 1024] {
            assert!(run_batched(&plan, &views, batch).is_empty());
        }
    }

    /// Counters recording the phase-1/phase-2 event order.
    #[derive(Default)]
    struct PhaseOrderCounters {
        events: Vec<(&'static str, usize, usize)>,
    }

    impl Counters for PhaseOrderCounters {
        fn batch(&mut self, _: usize, _: usize) {}
        fn node_entry(&mut self, _: usize) {}
        fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
        fn probe_hash(&mut self, node: usize, subatom: usize) {
            self.events.push(("hash", node, subatom));
        }
        fn probe(&mut self, node: usize, subatom: usize, _: bool) {
            self.events.push(("probe", node, subatom));
        }
        fn residual(&mut self, _: usize, _: bool) {}
        fn emit(&mut self) {}
        fn skip(&mut self, _: usize) {}
    }

    #[test]
    fn phase_one_hashes_the_whole_batch_before_any_phase_two_probe() {
        let dir = TempDir::new("run-two-phase");
        let schema = schema(2);
        let r: Vec<(u64, u64)> = (0..10).map(|i| (i, i)).collect();
        let s: Vec<(u64, u64)> = (0..10).map(|i| (i, i * 2)).collect();
        let views = views_of(&dir, &schema, &[r, s]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]),
                occurrence(1, 1, &[(0, 0), (1, 2)]),
            ],
            residuals: vec![],
        };
        let plan = planned(&normalized, &schema, &[0, 1]);
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut counters = PhaseOrderCounters::default();
        Executor::with_batch_size(&plan, 128).execute(
            &plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut counters,
        );

        // All 10 root entries fit one batch: every hash of node 0's sibling
        // pass must precede its first probe.
        let first_probe = counters
            .events
            .iter()
            .position(|(kind, node, _)| *kind == "probe" && *node == 0)
            .expect("probes happened");
        let hashes_before = counters.events[..first_probe]
            .iter()
            .filter(|(kind, node, _)| *kind == "hash" && *node == 0)
            .count();
        assert_eq!(
            hashes_before, 10,
            "the entire batch is hashed before the first bucket load"
        );
        assert!(!sink.rows.is_empty());
    }

    /// PRD 05 (docs/hardening): a pinned sibling (`Cursor::Row`) probes
    /// by field equality — phase 1 computes no hash for it, and EXPLAIN's
    /// `hashes` counts only hashes computed for map probes. Probes still
    /// count; results are unchanged.
    #[test]
    fn pinned_siblings_probe_without_hashing() {
        let dir = TempDir::new("run-pinned-hash");
        let schema = schema(3);
        // A(a,b) drives; B and C each have exactly one row per probe key,
        // so both pin to Cursor::Row after node 0. At node 1 both B(c)
        // and C(c) are covers with count 1; the tie keeps the incumbent
        // (B, the lower subatom index), leaving C as the pinned sibling.
        let a_rows: Vec<(u64, u64)> = vec![(1, 10), (2, 20)];
        let b_rows: Vec<(u64, u64)> = vec![(1, 100), (2, 200)];
        let c_rows: Vec<(u64, u64)> = vec![(10, 100), (20, 200)];
        let views = views_of(&dir, &schema, &[a_rows, b_rows, c_rows]);
        let normalized = NormalizedQuery {
            occurrences: vec![
                occurrence(0, 0, &[(0, 0), (1, 1)]), // A(a, b)
                occurrence(1, 1, &[(0, 0), (1, 2)]), // B(a, c)
                occurrence(2, 2, &[(0, 1), (1, 2)]), // C(b, c)
            ],
            residuals: vec![],
        };
        // Hand-built: node 0 probes both B(a) and C(b) — C's second
        // appearance at node 1 is then a probe against its pinned child.
        let plan = crate::plan::fj::FjPlan {
            nodes: vec![
                crate::plan::fj::Node {
                    subatoms: vec![
                        crate::plan::fj::Subatom {
                            occ: OccId(0),
                            vars: vec![VarId(0), VarId(1)],
                        },
                        crate::plan::fj::Subatom {
                            occ: OccId(1),
                            vars: vec![VarId(0)],
                        },
                        crate::plan::fj::Subatom {
                            occ: OccId(2),
                            vars: vec![VarId(1)],
                        },
                    ],
                },
                crate::plan::fj::Node {
                    subatoms: vec![
                        crate::plan::fj::Subatom {
                            occ: OccId(1),
                            vars: vec![VarId(2)],
                        },
                        crate::plan::fj::Subatom {
                            occ: OccId(2),
                            vars: vec![VarId(2)],
                        },
                    ],
                },
            ],
        };
        let plan = validate(&plan, &normalized, &schema, vec![0; 2], &BTreeSet::new())
            .expect("valid plan");
        let mut colts = colts_for(&plan, &views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut counters = PhaseOrderCounters::default();
        Executor::new(&plan).execute(&plan, &mut colts, &mut bindings, &mut sink, &mut counters);

        let count = |kind: &str, node: usize, subatom: usize| {
            counters
                .events
                .iter()
                .filter(|(k, n, s)| *k == kind && *n == node && *s == subatom)
                .count()
        };
        // Node 0's siblings probe root nodes: hashed.
        assert!(count("hash", 0, 1) > 0, "B's root probe hashes");
        assert!(count("hash", 0, 2) > 0, "C's root probe hashes");
        // Node 1's pinned sibling (C, subatom 1): probed, never hashed.
        assert_eq!(count("hash", 1, 1), 0, "pinned probes compute no hash");
        assert_eq!(count("probe", 1, 1), 2, "both entries still probe C");
        // Results unchanged: the two consistent binding triples.
        assert_eq!(
            sink.rows,
            BTreeSet::from([vec![1, 10, 100], vec![2, 20, 200]])
        );
    }

    /// Runs a plan at a given batch size, collecting the full binding set.
    fn run_at(
        plan: &ValidatedPlan,
        views: &[Arc<crate::image::RelationImage>],
        batch: usize,
    ) -> BTreeSet<Vec<u64>> {
        let mut colts = colts_for(plan, views);
        let mut bindings = Bindings::new(plan.slots().len());
        let mut sink = CollectSink::default();
        let mut executor = Executor::with_batch_size(plan, batch);
        executor.execute(
            plan,
            &mut colts,
            &mut bindings,
            &mut sink,
            &mut NoopCounters,
        );
        sink.rows
    }

    /// The magnitude-first cover rule (docs/architecture/30-execution.md), table-tested: the
    /// smaller side wins whatever its label; Exact breaks ties; a full
    /// tie keeps the incumbent.
    #[test]
    fn cover_choice_is_magnitude_first() {
        use KeyCount::{Estimate, Exact};
        // The measured bug: a 7-row unforced view must beat a 500-key
        // forced map.
        assert!(better_cover(Estimate(7), Exact(500)));
        assert!(!better_cover(Exact(500), Estimate(7)));
        // Magnitude wins in both label directions.
        assert!(better_cover(Exact(7), Estimate(500)));
        assert!(!better_cover(Estimate(500), Exact(7)));
        // Equal magnitudes: Exact displaces Estimate, never vice versa,
        // and same-label ties keep the incumbent (deterministic order).
        assert!(better_cover(Exact(9), Estimate(9)));
        assert!(!better_cover(Estimate(9), Exact(9)));
        assert!(!better_cover(Exact(9), Exact(9)));
        assert!(!better_cover(Estimate(9), Estimate(9)));
    }

    /// The randomized differential family (docs/architecture/50-validation.md):
    /// random instances and join orders over three query shapes, the whole
    /// production lowering (binary2fj + factor + validate), compared against
    /// a brute-force nested-loop oracle at batch sizes {1, 7, 128}. This is
    /// the harness that catches plan/executor bugs hand-picked fixtures
    /// miss — the cover-rebind bug needed only mild skew.
    #[test]
    fn randomized_differential_against_the_nested_loop_oracle() {
        // Deterministic LCG (no rand dependency; reproducible failures).
        let mut state = 0x1234_5678_9ABC_DEF0_u64;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            state >> 33
        };

        let schema = schema(3);
        for case in 0..60u32 {
            // Random small instances with skew: values in 0..=domain where
            // a small domain forces duplicates and multi-position chunks.
            let domain = 1 + next() % 8;
            let mut data: Vec<Vec<(u64, u64)>> = Vec::new();
            for _ in 0..3 {
                let rows = 1 + next() % 40;
                let mut rel = Vec::new();
                for _ in 0..rows {
                    rel.push((next() % domain, next() % domain));
                }
                rel.sort_unstable();
                rel.dedup();
                data.push(rel);
            }
            let dir = TempDir::new(&format!("run-differential-{case}"));
            let views = views_of(&dir, &schema, &data);

            // Three shapes over vars x=0, y=1, z=2:
            //   chain:    R0(x,y), R1(y,z)
            //   triangle: R0(x,y), R1(y,z), R2(x,z)
            //   clover:   R0(x,y), R1(x,z) (self-shaped star)
            let shape = case % 3;
            let occurrences = match shape {
                0 => vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    occurrence(1, 1, &[(0, 1), (1, 2)]),
                ],
                1 => vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    occurrence(1, 1, &[(0, 1), (1, 2)]),
                    occurrence(2, 2, &[(0, 0), (1, 2)]),
                ],
                _ => vec![
                    occurrence(0, 0, &[(0, 0), (1, 1)]),
                    occurrence(1, 1, &[(0, 0), (1, 2)]),
                ],
            };
            let n = occurrences.len();
            let normalized = NormalizedQuery {
                occurrences,
                residuals: vec![],
            };
            // Random join order (a permutation drawn by rejection).
            let mut order: Vec<u16> = (0..u16::try_from(n).expect("small")).collect();
            for i in (1..order.len()).rev() {
                let j = usize::try_from(next()).expect("64-bit") % (i + 1);
                order.swap(i, j);
            }
            let plan = planned(&normalized, &schema, &order);

            // The oracle: brute-force nested loops over the shape.
            let mut expected = BTreeSet::new();
            match shape {
                0 => {
                    for (a, b) in &data[0] {
                        for (c, d) in &data[1] {
                            if b == c {
                                expected.insert(vec![*a, *b, *d]);
                            }
                        }
                    }
                }
                1 => {
                    for (a, b) in &data[0] {
                        for (c, d) in &data[1] {
                            for (e, g) in &data[2] {
                                if b == c && a == e && d == g {
                                    expected.insert(vec![*a, *b, *d]);
                                }
                            }
                        }
                    }
                }
                _ => {
                    for (a, b) in &data[0] {
                        for (c, d) in &data[1] {
                            if a == c {
                                expected.insert(vec![*a, *b, *d]);
                            }
                        }
                    }
                }
            }

            for batch in [1usize, 7, 128] {
                // Slot order follows the join order; reorder each row into
                // VarId order before comparing with the oracle.
                let got: BTreeSet<Vec<u64>> = run_at(&plan, &views, batch)
                    .into_iter()
                    .map(|row| {
                        (0..3u16)
                            .map(|v| row[plan.slot_of(VarId(v))])
                            .collect::<Vec<u64>>()
                    })
                    .collect();
                assert_eq!(
                    got, expected,
                    "case {case} shape {shape} order {order:?} batch {batch} domain {domain}"
                );
            }
        }
    }
}
