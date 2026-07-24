//! The pipelined Free Join executor (the architecture docs) —
//! vectorized execution is the default and only path;
//! batch size 1 is merely its degenerate setting, never a mode
//! (`docs/architecture/40-execution.md` D4, post-mortem §31).
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
use crate::ir::normalize::{PlacedAllen, PlacedComparison, PlacedDuration, PlacedWordComparison};
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

/// A fused leaf scan: the last node's suffix
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
    /// Emits one complete binding — the key-probe path (single row by
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
    /// every node sink-relevant, so a skip under a
    /// fold is absorbed at the node that produced it — this method
    /// backs the debug tripwire that a skip never *crosses* a node
    /// unless the sink is allowed to skip at all.
    fn skip_capability(&self) -> SkipCapability {
        SkipCapability::Forbidden
    }

    /// Opens a fused leaf scan. `false` — the
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
    /// of rows the scan consumed (introspection's `emits` accounting).
    fn end_scan(&mut self, scan: &LeafScan<'_>) -> u64 {
        let _ = scan;
        unreachable!("end_scan without begin_scan == true");
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
/// (docs/architecture/60-validation.md): the five sequential segments of
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

/// Execution observability seam (40-execution): the normal path
/// instantiates [`NoopCounters`] — zero-sized, compiled to nothing; the
/// introspection entry point (docs/architecture/40-execution.md) instantiates the counting variant.
pub trait Counters {
    fn node_entry(&mut self, node: usize);
    /// One cover batch was drawn (`len` entries) — introspection's "batching
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
    /// One anti-probe ran for a surviving binding (docs/architecture/
    /// 40-execution.md, § anti-probe filters): `hit` means a matching
    /// fact exists in the negated occurrence and the binding is
    /// rejected; a miss survives.
    fn anti_probe(&mut self, node: usize, hit: bool);
    fn emit(&mut self);
    /// Bindings emitted so far (the rule loop's union accounting:
    /// per-rule emitted = the delta across one rule's run; absorbed =
    /// emitted − newly-seen). Zero on uncounted paths — the default is
    /// the honest report of a counter that does not exist.
    fn emits(&self) -> u64 {
        0
    }
    /// A D2 subtree skip propagated through this node.
    fn skip(&mut self, node: usize);
    /// One predicate's frontier rows entering a fixpoint round's delta
    /// image (the driver, `api/prepared/fixpoint.rs`): fires once per
    /// stratum predicate per round ≥ 1, before the round's variants
    /// run. Default no-op — the release path counts nothing.
    #[inline]
    fn fixpoint_delta(&mut self, predicate: u16, rows: u64) {
        let _ = (predicate, rows);
    }
    /// A fixpoint round closed (the driver's union accounting): the
    /// bindings the round's runs emitted and the re-derivations the
    /// spanning seen-sets absorbed. Round 0 is the stratum's
    /// non-recursive rules; its `fixpoint_delta` count is zero. Default
    /// no-op — populated on counted paths only.
    #[inline]
    fn fixpoint_round(&mut self, stratum: u16, emitted: u64, absorbed: u64) {
        let _ = (stratum, emitted, absorbed);
    }
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

/// The trace-mode phase accumulator (docs/architecture/60-validation.md):
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
    /// Bindings emitted (the RULE span's union accounting — trace-mode
    /// only; the release path's [`NoopCounters`] counts nothing).
    emits: u64,
}

/// The release-path counters: every method compiles to nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCounters;

/// The phase accumulator's inert twin (the `trace` feature is off): a
/// ZST with empty bodies, so the execute path's capture branch is
/// written once, `#[cfg]`-free — the obs.rs law. `obs::capturing()` is
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

    /// Flushes nothing (no capture can exist with `trace` off).
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
    fn cover_choice(&mut self, _: usize, _: usize, _: bool) {}
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
    /// Staleness tracking exists only to power the `debug_assert` in
    /// [`Bindings::get`]; the release path pays no epoch store in the
    /// innermost loop.
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

/// Where a value read during batched probing comes from: a word of the
/// current batch's cover keys (varying per element) or an already-bound
/// outer slot (constant across the batch). Word-indexed on both sides:
/// an interval variable occupies two consecutive batch key words exactly
/// as it occupies two consecutive binding slots (the [`crate::ir::
/// normalize::SlotWidth`] layout), so every consumer resolves one
/// `Source` per key **word**, never per variable.
#[derive(Debug, Clone, Copy)]
enum Source {
    Batch(usize),
    Slot(usize),
}

/// Where a probed occurrence's cursor comes from within one pass —
/// resolved once per (pass, occurrence), never a per-element subatom
/// search (the instruction diet). The membership-probe loops and the
/// routing arm consult the same resolution.
#[derive(Debug, Clone, Copy)]
enum CursorSrc {
    /// The cover subatom's child (per element).
    Cover,
    /// A sibling probe's child at this subatom index (per element).
    Sibling(usize),
    /// The element's parent entry's carried column (pipeline only).
    Carried(usize),
    /// A batch-constant cursor, hoisted (a never-advanced start, or the
    /// leaf pass's outer cursor).
    Const(Cursor),
}

/// One whole-value residual compare over a variable's slot words: width
/// 1 is the scalar compare; any wider span — an interval pair or a
/// `bytes<N>` block — compares **word-wise** under `Eq`/`Ne` only
/// (`docs/architecture/20-query-ir.md` — interval-pair predicates travel
/// as Allen mask residuals, point membership as word residuals, and
/// order over multi-word values is a validation-typed refusal, so a wide
/// residual is whole-value identity only).
fn compare_wide(
    op: crate::ir::CmpOp,
    width: usize,
    lhs: impl Fn(usize) -> u64,
    rhs: impl Fn(usize) -> u64,
) -> bool {
    if width == 1 {
        return op.compare(&lhs(0), &rhs(0));
    }
    match op {
        crate::ir::CmpOp::Eq => (0..width).all(|i| lhs(i) == rhs(i)),
        crate::ir::CmpOp::Ne => (0..width).any(|i| lhs(i) != rhs(i)),
        _ => unreachable!("validated: multi-word values admit Eq/Ne only as whole values"),
    }
}

/// Grow-only scratch sizing (the pooled high-water contract): the
/// buffer zero-fills only above its high-water mark, never per pass —
/// `clear` + `resize(n, 0)` re-memset the full window every pass
/// (`_platform_memset`, 3.7% of `meets_chain`) though every element of
/// `[..n]` is written before it is read. Callers confine reads to
/// `[..n]` (the compaction kernel slices internally); the tail above
/// `n` is stale by contract. Shared by both line-parallel passes and
/// the anti-probe — the contract is behavior, not the refused pass
/// extraction.
fn grow_scratch<T: Copy + Default>(v: &mut Vec<T>, n: usize) {
    if v.len() < n {
        v.resize(n, T::default());
    }
}

/// The batch key word offset of `target` inside a cover's variable list
/// — `None` when the variable is not bound by this cover (read its
/// outer slot instead). Word offsets accumulate slot widths, so an
/// interval cover variable's pair lands at `base` and `base + 1`.
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

/// A leaf-scan residual operand, resolved once per residual per hoisted
/// run: a live column view or the outer binding's constant word.
/// Small runs resolve per position; [`SCAN_HOIST_THRESHOLD`]
/// splits the arms.
#[derive(Clone, Copy)]
enum Operand<'a> {
    Col(crate::image::ColumnView<'a>),
    Const(u64),
}

/// Minimum survivors for a phase-1.5 pass — the ONLY prefetch gate:
/// the pass measured at ~12 ns fixed + ~0.3 ns/probe, so a
/// 4-survivor pass amortizes it and smaller ones are pure overhead.
/// The former footprint tier (2 MiB, retuned to 256 KiB) was ablated
/// at the bucket-layout probe floor and measured NOTHING at family
/// level (every family within ±2%, spread −2.9%) —
/// covering an at-floor map costs ~nothing at today's 5.7 ns/probe,
/// and the gate's comparison was the last of its complexity.
/// Re-ablated on the displaced lanes (2026-07-17, twin-binary
/// interleaved A/B, min-of-3 DVFS-normalized p50s, ephemeral) once
/// they made the DRAM/displaced regime measurable: the 256 KiB tier
/// re-armed at both gate sites is decision-free past L2 by
/// construction (the lanes' forced map is ~34 MiB, the tier always
/// passes) and measured so — twin/width-only 1.025/0.985/0.977 on
/// `disp_probe`/`_d24`/`_d96` (mixed signs inside the lane's recorded
/// cross-block wobble) and 0.999/0.998/1.015 on the probe-free
/// stream ladder. NEUTRAL: no regime gives the tier's comparison
/// anything to decide — width-only stays.
const PREFETCH_WIDTH_FLOOR: usize = 4;

/// Per-node reusable scratch: each node's frame is active at most once in
/// the recursion (frames advance strictly by node index), so scratch is
/// indexed by node and allocated once per executor construction. Fields
/// group by lifecycle, marked by the dividers below (named sub-structs
/// were refused: the grouping buys no new invariant — every field is
/// already private to the executor — and would rename every access in
/// the two hot passes for it).
#[derive(Default)]
struct NodeScratch {
    // — Pass scratch: valid within one probe/leaf pass, overwritten per
    //   batch (capacity retained across executions). —
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
    /// Word-residual operand sources, aligned with the node's
    /// `word_residuals` list — one source per side, already offset to
    /// the compared interval word (docs/architecture/20-query-ir.md,
    /// § normalization).
    word_residual_sources: Vec<(Source, Source)>,
    /// Allen-residual operand sources, aligned with the node's
    /// `allen_residuals` list — one source per side at the interval
    /// variable's word **base**; evaluation reads the pair at offsets
    /// 0/1 (batch key words and binding slots lay intervals out
    /// identically — the `SlotWidth` layout).
    allen_sources: Vec<(Source, Source)>,
    /// Measure-residual operand sources, aligned with the node's
    /// `duration_residuals` list: the interval side at its word base
    /// (pair read at offsets 0/1), the u64 side at its single word.
    duration_sources: Vec<(Source, Source)>,
    /// Allen-residual endpoint gather scratch: the four per-survivor
    /// endpoint streams `[a.start | a.end | b.start | b.end]`, each of
    /// the survivor count, gathered per residual pass and classified
    /// whole by the configuration kernel (pooled — capacity retained).
    allen_gather: Vec<u64>,
    /// Allen-residual configuration codes, aligned with the survivor
    /// set (pooled — capacity retained).
    allen_codes: Vec<u8>,
    /// Anti-probe key sources, aligned with the node's anti-probe list
    /// (one inner vec per anti-probe, one source per key **word**) —
    /// resolved per pass against the runtime cover choice, exactly like
    /// `sources`.
    anti_sources: Vec<Vec<Source>>,
    /// Membership-check scratch: one (start column, end column, point
    /// word) triple per point filter of the spec under evaluation,
    /// rebuilt per element (capacity retained).
    point_checks: Vec<(usize, usize, u64)>,
    /// Membership point-word sources, resolved once per (pass, spec)
    /// against the runtime cover choice — the per-element half above
    /// reads through these (capacity retained).
    point_sources: Vec<(usize, usize, Source)>,
    /// Occ-indexed cursor sources for this pass, resolved once per pass
    /// — the membership loops and the routing arm read cursors through
    /// this table instead of re-searching subatoms per element.
    cursor_srcs: Vec<CursorSrc>,
    /// Per-entry survivor mask for the compaction kernel.
    mask: Vec<u8>,
    // — Probe-batch identity (pipeline): per element of the CURRENT
    //   cross-parent batch, aligned with `entry_keys`; cleared at the
    //   end of each `probe_pass`. —
    /// Pipeline probe-batch parent indices: the
    /// pending entry each batch element expanded from.
    parents: Vec<u32>,
    /// Per probe-batch element: the origin (aligned with `parents`).
    element_origins: Vec<u32>,
    // — Pending buffers (pipeline): rows awaiting this node, appended by
    //   the parent node's routing and drained whole by `pump`; live
    //   across passes, reset per execution. —
    /// Pending binding rows awaiting this node, entry-major
    /// (stride = slot count).
    pending_bindings: Vec<u64>,
    /// Pending carried cursors, entry-major (stride = the node's carried
    /// occurrence count).
    pending_cursors: Vec<Cursor>,
    /// Entries in the pending buffers.
    pending_len: usize,
    /// Per pending entry: the D2 origin it descends from — minted at
    /// the absorb node's routing, inherited below.
    pending_origins: Vec<u32>,
}

/// The executor scratch for one plan shape: per-execution cursor state and
/// per-node buffers, sized once at construction. It does not borrow the
/// plan — the same `&ValidatedPlan` is passed to [`Executor::execute`]
/// (the prepared query owns both, the 40-execution doc).
pub struct Executor {
    batch: usize,
    /// Per occurrence: (current cursor, current trie level).
    cursors: Vec<(Cursor, usize)>,
    /// Per subatom slot maps, precomputed: `slot_map[node][subatom][i]` is
    /// the binding slot of that subatom's i-th variable.
    slot_map: Vec<Vec<Vec<usize>>>,
    /// Per residual: (lhs slot, rhs slot, slot width), aligned with each
    /// node's list. Width 2 is an interval pair — `Eq`/`Ne` compare
    /// pairwise over the two slot words (the only operators validation
    /// admits for intervals).
    residual_slots: Vec<Vec<(PlacedComparison, usize, usize, usize)>>,
    /// Per word residual: (lhs slot, rhs slot), aligned with each node's
    /// `word_residuals` — slots already offset to the compared word.
    word_residual_slots: Vec<Vec<(PlacedWordComparison, usize, usize)>>,
    /// Per Allen residual: (residual, lhs base slot, rhs base slot),
    /// aligned with each node's `allen_residuals`.
    allen_residual_slots: Vec<Vec<(PlacedAllen, usize, usize)>>,
    /// Per Allen residual: this execution's resolved mask, aligned with
    /// `allen_residual_slots` — literal masks are fixed at construction;
    /// param masks are rewritten in place by [`Executor::bind_allen_masks`]
    /// before every execution (the executor never sees the param slice
    /// on the hot path).
    allen_masks: Vec<Vec<crate::allen::AllenMask>>,
    /// Per measure residual: (residual, interval base slot, scalar slot),
    /// aligned with each node's `duration_residuals`.
    duration_residual_slots: Vec<Vec<(PlacedDuration, usize, usize)>>,
    /// Per membership probe, aligned with each node's `point_probes`
    /// list ([`PointProbeSpec`]).
    point_probe_slots: Vec<Vec<PointProbeSpec>>,
    /// Per occurrence: some node's membership probe reads this
    /// occurrence's advanced cursor (`PointProbe::occ`), so its
    /// per-position children are semantically live and the zero-arity
    /// cover collapse (`pump`/`run_node`: one entry stands for the whole
    /// suffix) must not fire on it.
    point_probed: Vec<bool>,
    /// Every variable's slot width in words — the word-level source
    /// resolution's lookup (tiny; linear scan).
    var_widths: Vec<(crate::ir::VarId, usize)>,
    /// Per anti-probe, aligned with each node's `anti_probes` list: the
    /// negated occurrence and its probe-key layout, precomputed like
    /// `residual_slots`.
    anti_probe_slots: Vec<Vec<AntiProbeSpec>>,
    scratch: Vec<NodeScratch>,
    /// The leaf fast paths apply when the last node
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
    /// Residual-surviving positions of one scan run (leaf residuals
    /// filter positions before the sink folds them).
    scan_filter: Vec<u32>,
    /// The pipelined executor's shape tables:
    /// `Some` for every multi-node plan — the one executor.
    pipe: Option<PipeTables>,
    /// D2 origin cancellation, epoch-stamped:
    /// `cancelled[origin] == cancel_epoch` marks a dead subtree. Grows
    /// to the per-execution origin high-water and is never cleared.
    cancelled: Vec<u32>,
    cancel_epoch: u32,
    next_origin: u32,
    /// A skip crossed the virtual root: the whole execution is done.
    /// The ONE stop condition every loop granularity checks — set
    /// directly by the root-skip site (a skip is an answer, not an
    /// error) and by [`Executor::poison`] for the typed errors.
    all_cancelled: bool,
    /// The typed early-stop, set-once ([`Executor::poison`]: first
    /// poison wins; two can never coexist because the first breaks
    /// every loop upstream) and drained by `execute` into the typed
    /// error. One sum, not parallel flags: a site cannot set an error
    /// without stopping, and `execute` cannot miss a kind — no `Result`
    /// on the per-tuple path.
    poison: Option<Poison>,
    /// The leaf overlap enumeration's per-execution index cache
    /// (`overlap_leaf.rs`; reset per `execute` — group positions are
    /// only stable within one execution).
    overlap: crate::interval::overlap::OverlapCache,
    /// The current leaf call's matched cover positions, start-ordered
    /// (pooled — capacity retained).
    overlap_hits: Vec<u32>,
    /// The overlap cache-key scratch: cover occurrence + bound prefix
    /// words (pooled).
    overlap_key: Vec<u64>,
}

/// A typed condition that stops the whole execution early — the poison
/// shape: one flag write on the cold path, no `Result` on the per-tuple
/// path. `execute` drains it into the typed error; adding a kind here
/// forces the drain's `match` to answer for it.
enum Poison {
    /// A measure residual reached a ray (`end == MAX`): the offending
    /// interval's two encoded words — the engine's one runtime type
    /// error ([`crate::error::Error::MeasureOfRay`]).
    MeasureOfRay([u64; 2]),
    /// The origin mint space would cross u32 (checked at mint
    /// granularity in `probe_pass`):
    /// [`crate::error::Error::Overflow`].
    OriginOverflow,
}

/// The pipelined executor's static shape tables:
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
    /// The D2 absorb node: the deepest sink-relevant
    /// node — a leaf skip cancels the subtree of one of its elements.
    /// `Some(N-1)` (the leaf itself) means skips never cross a node;
    /// `None` means a skip ends the whole execution. Skips only exist
    /// under sinks carrying `SkipCapability::Licensed`; cancellation is an optimization —
    /// a late cancel re-emits rows the seen-set already holds.
    absorb: Option<usize>,
}

/// One anti-probe resolved for execution (docs/architecture/
/// 40-execution.md, § anti-probe filters): the negated occurrence's
/// index and its probe-key parts — the occurrence's single trie level in
/// binding order, each variable with its first binding slot and its
/// [`crate::ir::normalize::SlotWidth`] word count.
struct AntiProbeSpec {
    occ: usize,
    /// Per key variable: (variable, first binding slot, width in words).
    parts: Vec<(crate::ir::VarId, usize, usize)>,
    /// Total probe-key words (the occurrence's `key_widths[0]`); zero for
    /// the emptiness-gate form.
    key_words: usize,
    /// The negated occurrence's var-sourced membership filters, per
    /// filter: (start column, end column, point variable, point slot).
    /// Evaluated inside the probe: a binding is rejected only if a fact
    /// matching the keys **also** satisfies every membership — the
    /// existential reading over the negated occurrence's facts
    /// (docs/architecture/20-query-ir.md, § param sets / membership).
    point_parts: Vec<(usize, usize, crate::ir::VarId, usize)>,
}

/// One membership probe resolved for execution ([`crate::plan::fj::
/// PointProbe`]): the positive occurrence whose remaining positions are
/// scanned, and per filter the interval field's column pair with the
/// bound point variable's slot. A binding survives iff one position
/// satisfies **every** part (the conjunction quantifies over one fact).
struct PointProbeSpec {
    occ: usize,
    /// Per filter: (start column, end column, point variable, point slot).
    parts: Vec<(usize, usize, crate::ir::VarId, usize)>,
}

/// The single-subatom-leaf precompute: everything
/// the leaf fast paths would otherwise re-derive per node entry.
struct LeafPrecompute {
    single: bool,
    residual_sources: Vec<(Source, Source)>,
    scan_residuals: Vec<(crate::ir::CmpOp, Source, Source)>,
    const_residuals: Vec<(crate::ir::CmpOp, usize, usize)>,
    row: Vec<u64>,
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
