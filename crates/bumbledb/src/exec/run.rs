//! The recursive Free Join executor (the architecture docs + 21) — vectorized execution
//! is the default and only path; batch size 1 is merely its degenerate
//! setting, never a mode (`docs/architecture/30-execution.md` D4,
//! post-mortem §31; paper §3.3 Fig. 5, §4.3).
//!
//! Everything is a monomorphized generic — no `dyn` anywhere in the hot
//! path. Per node entry: choose the cover by labeled key count, iterate it
//! in batches, two-phase-probe each sibling (phase 1 computes every hash —
//! pure ALU; phase 2 issues all bucket loads — independent chains the
//! out-of-order window overlaps), compact survivors branchlessly, evaluate residuals as
//! batch compaction, then recurse per surviving element with the scalar
//! journal discipline — except at the last node, where the surviving
//! batch goes to the sink whole (docs/perf/ PRD 01: no recursion, no
//! journal, no per-row binding stores at the leaf).
//!
//! Honest caveat, stated (D4): deep in the plan the batch source is the
//! current subtrie, whose fanout on FK walks is often 1-10 — large batches
//! are reliably available only at the root; cross-node-entry accumulation
//! is future work, not assumed.

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
/// Small runs skip the table — measured on spread (~1.4 positions/run),
/// building it cost +48 ns/row; measured on range (one 100k-position
/// run), skipping it cost ~10 µs. [`SCAN_HOIST_THRESHOLD`] splits them.
enum Operand<'a> {
    Col(crate::image::ColumnView<'a>),
    Const(u64),
}

/// Run length at which hoisting operand/column tables pays for itself.
const SCAN_HOIST_THRESHOLD: usize = 32;

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
    /// Undo journal: (occurrence index, previous cursor, previous level).
    journal: Vec<(usize, Cursor, usize)>,
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
                    journal: Vec::new(),
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
        self.run_node(plan, 0, colts, bindings, sink, counters);
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
        // Zero-node plans are unrepresentable (validation rule 14 rejects
        // atom-less queries; binary2fj makes a node per occurrence), and
        // the leaf batch path emits at the last real node — so every call
        // lands on a real node.
        assert!(
            node_idx < plan.nodes().len(),
            "the leaf batch path emits at the last node"
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
                scratch.hashes.clear();
                // The dominant probe shape — a single batch-sourced key
                // word — takes a match-free specialized loop (docs/perf/
                // PRD 07); everything else takes the general gather.
                let single_batch_word = match scratch.sources[sub_idx].as_slice() {
                    [Source::Batch(word)] if !pinned => Some(*word),
                    _ => None,
                };
                if let Some(word) = single_batch_word {
                    for (k, &e) in scratch.survivors.iter().enumerate() {
                        let entry = usize::try_from(e).expect("batch fits usize");
                        let key = scratch.entry_keys[entry * arity + word];
                        scratch.probe_keys[k] = key;
                        counters.probe_hash(node_idx, sub_idx);
                        scratch
                            .hashes
                            .push(crate::exec::colt::hash_key(std::slice::from_ref(&key)));
                    }
                } else {
                    for (k, &e) in scratch.survivors.iter().enumerate() {
                        let entry = usize::try_from(e).expect("batch fits usize");
                        for i in 0..sub_arity {
                            scratch.probe_keys[k * sub_arity + i] = value_of(
                                &scratch.sources[sub_idx],
                                &scratch.entry_keys,
                                bindings,
                                entry,
                                i,
                            );
                        }
                        if pinned {
                            scratch.hashes.push(0);
                        } else {
                            counters.probe_hash(node_idx, sub_idx);
                            scratch.hashes.push(crate::exec::colt::hash_key(
                                &scratch.probe_keys[k * sub_arity..(k + 1) * sub_arity],
                            ));
                        }
                    }
                }
                counters.phase_end(node_idx, JoinPhase::Hash);

                // Phase 1.5 (docs/perf/ PRD 07): the prefetch pass — every
                // bucket the batch will probe gets its ctrl and bucket
                // lines hinted while the OoO window is still free. Gated:
                // tiny batches gain nothing and pay the loop.
                if !pinned && scratch.survivors.len() >= 16 {
                    for &hash in &scratch.hashes {
                        colts[occ].prefetch_bucket(s_cursor, hash);
                    }
                }

                // Phase 2: all bucket loads — independent chains the
                // out-of-order window overlaps — then kernel compaction.
                counters.phase_start(node_idx, JoinPhase::Probe);
                scratch.mask.clear();
                for k in 0..scratch.survivors.len() {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let hit = colts[occ].get_prehashed(
                        s_cursor,
                        s_level,
                        &scratch.probe_keys[k * sub_arity..(k + 1) * sub_arity],
                        scratch.hashes[k],
                    );
                    counters.probe(node_idx, sub_idx, hit.is_some());
                    scratch.sibling_children[sub_idx][entry] = hit.unwrap_or(Cursor::Row(0));
                    scratch.mask.push(u8::from(hit.is_some()));
                }
                crate::exec::kernel::compact_u32_by_mask(&mut scratch.survivors, &scratch.mask);
                counters.phase_end(node_idx, JoinPhase::Probe);
            }

            // Residuals run as batch survivor compaction after the probes.
            counters.phase_start(node_idx, JoinPhase::Residual);
            for (r_idx, (lhs_src, rhs_src)) in scratch.residual_sources.iter().enumerate() {
                let op = self.residual_slots[node_idx][r_idx].0.op;
                scratch.mask.clear();
                for k in 0..scratch.survivors.len() {
                    let e = scratch.survivors[k];
                    let entry = usize::try_from(e).expect("batch fits usize");
                    let value = |src: &Source| match *src {
                        Source::Batch(word) => scratch.entry_keys[entry * arity + word],
                        Source::Slot(slot) => bindings.get(slot),
                    };
                    let pass = op.compare(&value(lhs_src), &value(rhs_src));
                    counters.residual(node_idx, pass);
                    scratch.mask.push(u8::from(pass));
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

            // Save the node-entry cursors once per batch: every entry's
            // recursion restores them, so they are identical for each
            // surviving element — the old per-entry journal rebuild paid
            // a push/drain cycle per tuple in the innermost loop. The
            // journal save is descend bookkeeping: timed as Descend.
            counters.phase_start(node_idx, JoinPhase::Descend);
            scratch.journal.clear();
            scratch.journal.push((cover_occ, cover_cursor, cover_level));
            for (sub_idx, subatom) in plan.nodes()[node_idx].subatoms.iter().enumerate() {
                if sub_idx == cover_sub {
                    continue;
                }
                let occ = usize::from(subatom.occ.0);
                let (cursor, level) = self.cursors[occ];
                scratch.journal.push((occ, cursor, level));
            }

            // Recurse per surviving element (paper §4.3: batch within a
            // node, recurse per tuple) with the scalar journal discipline.
            for k in 0..scratch.survivors.len() {
                let entry = usize::try_from(scratch.survivors[k]).expect("batch fits usize");
                for (i, slot) in self.slot_map[node_idx][cover_sub].iter().enumerate() {
                    bindings.set(*slot, scratch.entry_keys[entry * arity + i]);
                }
                self.cursors[cover_occ] = (scratch.children[entry], cover_level + 1);
                let mut journal_idx = 1;
                for (sub_idx, _) in plan.nodes()[node_idx].subatoms.iter().enumerate() {
                    if sub_idx == cover_sub {
                        continue;
                    }
                    let (occ, _, level) = scratch.journal[journal_idx];
                    journal_idx += 1;
                    self.cursors[occ] = (scratch.sibling_children[sub_idx][entry], level + 1);
                }

                flow = self.run_node(plan, node_idx + 1, colts, bindings, sink, counters);

                for &(occ, cursor, level) in scratch.journal.iter().rev() {
                    self.cursors[occ] = (cursor, level);
                }

                if flow == Flow::SkipSuffix {
                    if plan.nodes()[node_idx].sink_relevant {
                        // This node binds a sink-relevant variable: absorb
                        // the skip — later entries change the output.
                        // Under an aggregate plan *every* variable is
                        // sink-relevant (hardening PRD 05; sink.rs's
                        // AggregateSink notes the same coupling), so a
                        // skip can never travel past its producing node.
                        flow = Flow::Continue;
                    } else {
                        // The suffix from here binds nothing sink-relevant:
                        // propagate the unwind (D2) — reachable only for
                        // sinks that skip at all (the projection sink).
                        debug_assert!(
                            sink.may_skip(),
                            "a SkipSuffix crossed a node under a non-skipping sink"
                        );
                        counters.skip(node_idx);
                        counters.phase_end(node_idx, JoinPhase::Descend);
                        break 'outer;
                    }
                }
            }
            counters.phase_end(node_idx, JoinPhase::Descend);
        }

        self.scratch[node_idx] = scratch;
        flow
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
                    let resolved: [Option<(crate::ir::CmpOp, Operand<'_>, Operand<'_>)>;
                        MAX_LEAF_RESIDUALS] = std::array::from_fn(|i| {
                        self.leaf_scan_residuals.get(i).map(|(op, lhs, rhs)| {
                            let side = |src: &Source| match *src {
                                Source::Batch(word) => {
                                    Operand::Col(scan.colt.suffix_column(scan.level, word))
                                }
                                Source::Slot(slot) => Operand::Const(bindings.get(slot)),
                            };
                            (*op, side(lhs), side(rhs))
                        })
                    });
                    let mut eval = |position: u32| {
                        for spec in resolved.iter().take(n_residuals) {
                            let (op, lhs, rhs) = spec.as_ref().expect("resolved up to len");
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
        let join_order = JoinOrder {
            order: order.iter().map(|o| OccId(*o)).collect(),
            estimates: vec![0; order.len()],
        };
        let mut plan = binary2fj(normalized, &join_order);
        factor(&mut plan);
        validate(
            &plan,
            normalized,
            schema,
            vec![0; order.len()],
            &BTreeSet::new(),
        )
        .expect("valid plan")
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
