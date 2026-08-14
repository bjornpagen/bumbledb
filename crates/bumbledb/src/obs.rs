//! The one tracing mechanism (docs/architecture/60-validation.md):
//! nanosecond spans and point events recorded into a thread-local buffer
//! during explicit capture, drained by tooling — Chrome-trace export and
//! flame summaries are this seam plus names.
//!
//! **Zero-cost when off** (docs/architecture/00-product.md: no always-on
//! instrumentation in release paths): under default features every
//! function here is an inline empty body and [`SpanGuard`] is a ZST with
//! no `Drop`; instrumented call sites are written once, `#[cfg]`-free.
//!
//! Recording allocates (the capture buffer grows): sanctioned only
//! because capture is never enabled inside a measured allocation window —
//! the gate never calls [`start_capture`], and the bench harness treats
//! trace capture and allocation windows as mutually exclusive run modes.

/// Event categories — coarse lanes for trace visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Prepare,
    Execute,
    Storage,
    Commit,
    Image,
    Cache,
    Harness,
    /// Executor phase accumulators (docs/architecture/60-validation.md):
    /// synthetic point events carrying `(total_ns, calls)` per
    /// (node, phase), flushed once per traced execution — never real
    /// spans, so flame containment math must exclude them.
    Phase,
}

impl Category {
    /// The category's stable label (Chrome-trace `cat` field).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Execute => "execute",
            Self::Storage => "storage",
            Self::Commit => "commit",
            Self::Image => "image",
            Self::Cache => "cache",
            Self::Harness => "harness",
            Self::Phase => "phase",
        }
    }
}

/// One recorded span or point event (`dur_ns == 0` ⇒ point event). The
/// two payload args' meanings are defined per name in [`names`].
/// The time fields are nanoseconds in every drained event; inside a
/// live capture buffer they hold raw anchor-relative ticks until
/// [`finish_capture`] converts once per event, off the measured
/// windows (the `PhaseTimers` discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceEvent {
    pub name: &'static str,
    pub cat: Category,
    pub start_ns: u64,
    pub dur_ns: u64,
    pub a0: u64,
    pub a1: u64,
}

/// The instrumentation-point name registry: every span/event name lives
/// here so call sites cannot typo-drift. Arg meanings are documented per
/// constant; consumers (the trace exporter, tests) match on these.
pub mod names {
    // Read path (docs/architecture/60-validation.md). Args noted as (a0, a1); `-` = unused.

    /// The whole prepare pipeline. (-, -)
    pub const PREPARE: &str = "prepare";
    /// IR validation. (-, -)
    pub const VALIDATE: &str = "validate";

    // Validation's interior (`ir/validate/validate.rs`), lit under
    // [`VALIDATE`]. Pass granularity — one span per roster pass, never
    // per rule or per candidate; rule work rides the pass span's args.

    /// One rule-set lowering — shape roster, DNF distribution, collapse
    /// (`lower_rules`): once per interior, once for the rec pool, once
    /// for main. (lowered rules
    /// produced, -)
    pub const VALIDATE_LOWER: &str = "validate_lower";
    /// The signature-sealing pass — declaration-order interior sealing
    /// (one span over interior count, never a chaotic loop).
    /// (interiors sealed, -)
    pub const VALIDATE_SEAL: &str = "validate_seal";
    /// The strict per-rule roster pass — every lowered rule through the
    /// typing fixpoint with all signatures anchored: one span per
    /// validation, never per rule. (rules validated, -)
    pub const VALIDATE_RULES: &str = "validate_rules";
    /// Normalization. (-, -)
    pub const NORMALIZE: &str = "normalize";
    /// One rule's comparison placement (`ir/normalize/place_comparisons.rs`)
    /// — the cross-atom residual routing, under [`NORMALIZE`], one per
    /// normalized rule (ray probes included). (cross-atom residuals placed,
    /// -)
    pub const PLACE_COMPARISONS: &str = "place_comparisons";
    /// One rule's statically-empty constant fold (`ir/normalize/fold.rs`),
    /// under [`NORMALIZE`], one per normalized rule. (1 rule dead on
    /// constants / 0 live, -)
    pub const NORMALIZE_FOLD: &str = "normalize_fold";
    /// Key-probe-vs-join classification. (-, -)
    pub const CLASSIFY: &str = "classify";
    /// Statistics reads. (occurrences measured concretely, -)
    pub const STATS: &str = "stats";
    /// The exhaustive left-deep DP. (-, -)
    pub const PLAN_DP: &str = "plan_dp";
    /// binary2fj + factor + plan validation. (-, -)
    pub const LOWER: &str = "lower";
    /// COLT construction at prepare. (-, -)
    pub const BUILD_COLTS: &str = "build_colts";

    // The DP planner's interior (docs/architecture/40-execution.md, § the
    // planner), lit under [`PLAN_DP`]. Pass granularity — never a span per
    // subset-DP candidate (the mask loop is O(2ⁿ·n), the doctrine's
    // per-tuple line); the candidate work is a single counted point event.

    /// Densifying participating occurrences + Allen residuals into the
    /// DP's bitset form. (participating occurrences, cross-atom Allen
    /// residuals densified)
    pub const PLAN_DENSIFY: &str = "plan_densify";
    /// The exhaustive left-deep subset DP's table-fill pass, over every
    /// mask of popcount ≥ 2. (subproblems filled, `(last, prev)` candidate
    /// pairs evaluated) — the second arg is the pruned-candidate COUNT the
    /// doctrine allows in place of a per-candidate event.
    pub const PLAN_FILL: &str = "plan_fill";
    /// One planner row-count read (docs/architecture/50-storage.md § the
    /// planner's `S` read) — an ordinary relation's stored `S` counter, a
    /// closed relation's sealed-extension length. Fires on the plan path
    /// (per participating EDB occurrence, and per unconditional-containment
    /// target inside the distinct ladder) and the staleness path (per
    /// pin). (relation id, rows) — a storage read, not a per-tuple label.
    pub const RELATION_ROWS: &str = "relation_rows";
    /// One resolution of the per-field distinct-count ladder
    /// (`plan/selectivity.rs`), one per (occurrence, field) at prepare —
    /// the rung that fired rides `a0`: `0` a single-field key (⇒ rows),
    /// `1` a resident image's exact count, `2` a containment target bound,
    /// `3` the documented floor. (rung, distinct count)
    pub const DISTINCT_LADDER: &str = "distinct_ladder";

    /// One prepared execution. (answers, -)
    pub const EXECUTE: &str = "execute";
    /// One rule of the loop, under the execute span — the index rides in
    /// the name (`RULE[index]`; the validation cap `crate::ir::MAX_RULES`
    /// = 16 bounds it). (bindings emitted, absorbed by the spanning
    /// seen-set) — both zero on uncounted paths (the release executor
    /// counts nothing; trace and introspection runs count).
    pub const RULE: [&str; 16] = [
        "rule_0", "rule_1", "rule_2", "rule_3", "rule_4", "rule_5", "rule_6", "rule_7", "rule_8",
        "rule_9", "rule_10", "rule_11", "rule_12", "rule_13", "rule_14", "rule_15",
    ];
    // The cap and the table move together, or the rule loop's span
    // lookup would panic on a legal query.
    const _: () = assert!(crate::ir::MAX_RULES == RULE.len());
    /// The interior preamble under the execute span. (interior count,
    /// derived tuples emitted across interiors)
    pub const INTERIORS: &str = "interiors";
    /// The rec least-fixpoint under the execute span. (rounds run,
    /// derived tuples at close)
    pub const REACH: &str = "reach";
    /// One reach round under the REACH span — round 0 is the base
    /// arms; the round index is the span's position under REACH
    /// (rounds are budget-bounded, not cap-bounded, so no name table
    /// exists). (bindings emitted, absorbed by the spanning seen-set.)
    pub const FIXPOINT_ROUND: &str = "fixpoint_round";
    /// Parameter binding. (-, -)
    pub const BIND_PARAMS: &str = "bind_params";
    /// Filter-constant resolution. (-, -)
    pub const RESOLVE_FILTERS: &str = "resolve_filters";
    /// The per-occurrence view loop. (-, -)
    pub const VIEWS: &str = "views";
    /// One occurrence's view rebuild. (occurrence index, survivors)
    pub const VIEW_BUILD: &str = "view_build";
    /// The warm memo fast path fired. (occurrence index, -)
    pub const VIEW_MEMO_HIT: &str = "view_memo_hit";
    /// The occurrence-dedup path fired: this occurrence's rebuild
    /// cloned a same-shaped sibling occurrence's bound state — view and
    /// forced root — instead of re-scanning the image and re-forcing
    /// (docs/architecture/40-execution.md). (occurrence index, canonical
    /// occurrence index)
    pub const VIEW_DEDUP: &str = "view_dedup";
    /// The Free Join executor. (-, -)
    pub const JOIN: &str = "join";
    /// Sink finalization into the result buffer. (-, -)
    pub const FINALIZE: &str = "finalize";
    /// The key-probe access path. (1 hit / 0 miss, -)
    pub const KEY_PROBE: &str = "key_probe";
    /// One snapshot point read (`Snapshot::get` / `get_dyn` /
    /// `get_dyn_into`) — the formerly wholly dark keyed-get surface,
    /// spanned whole at the API boundary. (1 hit / 0 miss, -)
    pub const POINT_READ: &str = "point_read";
    /// The whole selection-probe loop, batched over the occurrences —
    /// the lazy selection forces run inside it, so the span keeps that
    /// cost from masquerading as rule self-time
    /// (docs/architecture/40-execution.md § introspection).
    /// (occurrences probed, 1 all hit / 0 short-circuited empty)
    pub const SELECTIONS: &str = "selections";
    /// One occurrence's selection-level probe (docs/architecture/40-execution.md).
    /// (occurrence index, 1 hit / 0 miss)
    pub const SELECT_PROBE: &str = "select_probe";

    /// Image found in the shared cache. (relation id, -)
    pub const CACHE_HIT: &str = "cache_hit";
    /// A full image decode. (relation id, slab bytes)
    pub const IMAGE_BUILD: &str = "image_build";
    /// An append-path image extension: the base's columns copied, only
    /// the tail rows decoded (docs/architecture/50-storage.md § the
    /// image cache). (relation id, slab bytes)
    pub const IMAGE_APPEND: &str = "image_append";
    /// The exact per-column distinct counting pass inside an image
    /// build/append/synthesis (`image/distinct.rs`) — batch granularity,
    /// one span per image; on the append arm the rows counted are the
    /// tail alone (the persisted state's payoff). (columns counted, rows
    /// inserted)
    pub const IMAGE_DISTINCTS: &str = "image_distincts";
    /// The columnar fact decode inside an image build/append/synthesis —
    /// the batch decode that hid inside [`IMAGE_BUILD`] / [`IMAGE_APPEND`]:
    /// one sequential scan's worth of per-fact decode into the column slabs,
    /// one span per build (append decodes only the tail rows). Batch
    /// granularity — the per-fact kernel underneath is never spanned.
    /// (rows decoded, fact width in bytes)
    pub const DECODE_BATCH: &str = "decode_batch";
    /// An untouched relation's image carried forward to the reader's
    /// generation — the same Arc, re-keyed. (relation id, -)
    pub const CACHE_CARRY: &str = "cache_carry";
    /// Lost the insert race; adopted the winner's image. (relation id, -)
    pub const CACHE_ADOPT: &str = "cache_adopt";
    /// Old-generation reader built without caching. (relation id, -)
    pub const CACHE_QUERY_LOCAL: &str = "cache_query_local";
    /// One COLT node forced. (positions ingested, distinct keys)
    pub const COLT_FORCE: &str = "colt_force";
    /// One dictionary resolution in finalize — fires per *distinct*
    /// intern per PREPARED QUERY LIFETIME: the resolve memo's persistent
    /// arena tier caches the (word → text) pair forever, sound because
    /// the dictionary is append-only
    /// (docs/architecture/40-execution.md). (intern word, byte length)
    pub const DICT_RESOLVE: &str = "dict_resolve";
    /// One scalar String param bind served from the per-slot word memo
    /// — the dictionary descent skipped; sound because the append-only
    /// dictionary makes a resolved (text → word) pair final, and a MISS
    /// never memoizes (docs/architecture/40-execution.md). (param
    /// index, word)
    pub const PARAM_WORD_MEMO: &str = "param_word_memo";
    /// A `str` literal latched: the dictionary is append-only, so its
    /// resolved word rewrites the plan template once, permanently —
    /// fires once per distinct literal over the prepared query's
    /// lifetime (docs/architecture/40-execution.md § the literal
    /// latch). (latched word, -)
    pub const LITERAL_LATCH: &str = "literal_latch";

    // Write path (docs/architecture/60-validation.md).

    /// One state-changing commit. (1 changed / 0 no-op, -)
    pub const COMMIT: &str = "commit";
    /// A commit that netted to nothing. (-, -)
    pub const COMMIT_NOOP: &str = "commit_noop";
    /// Phase 1. (facts deleted, -)
    pub const APPLY_DELETES: &str = "apply_deletes";
    /// Phase 2. (facts inserted, -)
    pub const APPLY_INSERTS: &str = "apply_inserts";
    /// Phase 3, containment source side. (satisfying source probes, -)
    pub const JUDGMENT_SOURCE: &str = "judgment_source";
    /// Phase 3, containment target side. (disestablished determinants scanned, -)
    pub const JUDGMENT_TARGET: &str = "judgment_target";
    /// Phase 3, capacity statements. (touched parents judged, -)
    pub const JUDGMENT_CAPACITIES: &str = "judgment_capacities";
    /// Phase 4. (pending interns flushed, -)
    pub const COUNTERS_FLUSH: &str = "counters_flush";
    /// Phase 5: the LMDB commit alone — the fsync-bound number. (-, -)
    pub const LMDB_COMMIT: &str = "lmdb_commit";
    /// One bounded retry of the durability boundary after a transient
    /// commit-sync failure (`docs/architecture/50-storage.md` § write
    /// path, phase 5) — never silent. (retry number, OS errno)
    pub const COMMIT_SYNC_RETRY: &str = "commit_sync_retry";
    /// One `bulk_load` chunk. (facts submitted, facts changed)
    pub const BULK_CHUNK: &str = "bulk_chunk";
    /// `Db::compact`'s durability chain completed: the copied file, its
    /// dirent in `dest`, and `dest`'s own dirent in the parent directory
    /// all fsynced — fires only after the last sync succeeds, so its
    /// presence pins that the parent-dirent sync path executed.
    /// (directory fsyncs performed, -)
    pub const COMPACT_DURABLE: &str = "compact_durable";
    /// `Db::create`'s birth dirent chain completed: the store directory
    /// and its parent fsynced after the initialize commit (finding 022 —
    /// LMDB fsyncs file contents, never a directory), so a create-time
    /// power loss cannot lose the whole store. Fires only after the last
    /// sync succeeds. (directory fsyncs performed, -)
    pub const CREATE_DURABLE: &str = "create_durable";
    /// One `Db::write`, closure plus commit. (1 committed / 0 aborted, -)
    pub const WRITE_TXN: &str = "write_txn";

    // Verification path (`verify_store.rs`): the O(store) integrity sweep,
    // formerly wholly dark. One outer span and one span per namespace
    // pass — pass granularity, never a span per swept entry — each
    // carrying the findings it raised, so a desync localizes to its pass.

    /// The whole `Db::verify_store` sweep. (findings raised, -)
    pub const VERIFY_STORE: &str = "verify_store";
    /// The `F` (fact) namespace pass, plus the `S`-counter reconciliation
    /// inputs it tallies. (findings raised, -)
    pub const VERIFY_FACTS: &str = "verify_facts";
    /// The `M` (membership/idempotence) namespace pass. (findings, -)
    pub const VERIFY_MEMBERSHIP: &str = "verify_membership";
    /// The `U` (determinant) namespace pass, incl. pointwise
    /// disjointness. (findings, -)
    pub const VERIFY_DETERMINANTS: &str = "verify_determinants";
    /// The `R` (reverse-edge) namespace pass. (findings, -)
    pub const VERIFY_REVERSE: &str = "verify_reverse";
    /// The `Q` (fresh marks) namespace pass. (findings, -)
    pub const VERIFY_MARKS: &str = "verify_marks";
    /// The `S`-counter-vs-`F`-scan reconciliation pass. (findings, -)
    pub const VERIFY_COUNTERS: &str = "verify_counters";
    /// The fresh-field ratchet-law pass. (findings, -)
    pub const VERIFY_FRESH: &str = "verify_fresh";
    /// The dictionary liveness / dangling-id statistic pass. (findings, -)
    pub const VERIFY_DICT: &str = "verify_dict";

    // Harness (docs/architecture/60-validation.md, 17): tool overhead, honestly visible
    // inside the same trace, separated by tid at export.

    /// One harness-timed sample around the runner closure. (-, -)
    pub const SAMPLE: &str = "sample";
    /// One cold-protocol touch commit. (-, -)
    pub const TOUCH: &str = "touch";

    // Executor phase accumulators (Category::Phase): per (node, phase)
    // point events, (total_ns, calls). Node indices past the table cap
    // share the overflow name — attribution, not identification.

    /// Phase-name table: `JOIN_PHASE[phase][min(node, 8)]`. Phase order
    /// matches `exec::run::JoinPhase`: iter, hash, probe, residual,
    /// descend, force, gather.
    pub const JOIN_PHASE: [[&str; 9]; 7] = [
        [
            "jp_iter_n0",
            "jp_iter_n1",
            "jp_iter_n2",
            "jp_iter_n3",
            "jp_iter_n4",
            "jp_iter_n5",
            "jp_iter_n6",
            "jp_iter_n7",
            "jp_iter_nX",
        ],
        [
            "jp_hash_n0",
            "jp_hash_n1",
            "jp_hash_n2",
            "jp_hash_n3",
            "jp_hash_n4",
            "jp_hash_n5",
            "jp_hash_n6",
            "jp_hash_n7",
            "jp_hash_nX",
        ],
        [
            "jp_probe_n0",
            "jp_probe_n1",
            "jp_probe_n2",
            "jp_probe_n3",
            "jp_probe_n4",
            "jp_probe_n5",
            "jp_probe_n6",
            "jp_probe_n7",
            "jp_probe_nX",
        ],
        [
            "jp_residual_n0",
            "jp_residual_n1",
            "jp_residual_n2",
            "jp_residual_n3",
            "jp_residual_n4",
            "jp_residual_n5",
            "jp_residual_n6",
            "jp_residual_n7",
            "jp_residual_nX",
        ],
        [
            "jp_descend_n0",
            "jp_descend_n1",
            "jp_descend_n2",
            "jp_descend_n3",
            "jp_descend_n4",
            "jp_descend_n5",
            "jp_descend_n6",
            "jp_descend_n7",
            "jp_descend_nX",
        ],
        [
            "jp_force_n0",
            "jp_force_n1",
            "jp_force_n2",
            "jp_force_n3",
            "jp_force_n4",
            "jp_force_n5",
            "jp_force_n6",
            "jp_force_n7",
            "jp_force_nX",
        ],
        [
            "jp_gather_n0",
            "jp_gather_n1",
            "jp_gather_n2",
            "jp_gather_n3",
            "jp_gather_n4",
            "jp_gather_n5",
            "jp_gather_n6",
            "jp_gather_n7",
            "jp_gather_nX",
        ],
    ];
    // The executor's phase attribution and this table move together, or
    // `PhaseTimers::flush` would index past a row on a legal plan — the
    // RULE/MAX_RULES precedent: the enum's declaration order is the row
    // order, its count the row count, the node cap the column count.
    #[cfg(feature = "trace")]
    const _: () = {
        assert!(JOIN_PHASE.len() == crate::exec::run::JoinPhase::COUNT);
        assert!(JOIN_PHASE[0].len() == crate::exec::run::PHASE_NODE_CAP + 1);
    };

    /// One sink-map rehash inside a measured execution. (new capacity, arity)
    pub const WORDMAP_GROW: &str = "wordmap_grow";

    /// One residency-gated phase-1.5 prefetch pass ran.
    /// (survivors hinted, probed colt's forced footprint in bytes)
    pub const PREFETCH_PASS: &str = "prefetch_pass";

    /// One predicate-scan kernel invocation over a whole image column
    /// (`exec/kernel/filter.rs`) — the `std::simd` survivor scans, lit at
    /// the batch entry, never per lane. Fires once per kernel-shaped
    /// filter the view-build path dispatches. (lanes scanned, survivors)
    pub const KERNEL_FILTER: &str = "kernel_filter";

    /// One Allen configuration-kernel dense scan over a whole interval
    /// column pair (`exec/kernel/allen.rs` — the filter-position
    /// compositions the view-build path dispatches), lit at the batch
    /// entry like [`KERNEL_FILTER`], never per lane. The join loop's
    /// code/membership batches (`allen_code_batch` /
    /// `allen_filter_batch`) stay dark deliberately: they run inside
    /// the probe loop, whose attribution is the per-(node, phase)
    /// residual accumulator. (lanes scanned, survivors)
    pub const KERNEL_ALLEN: &str = "kernel_allen";
}

/// The trace-mode fast clock, under the measured cost model: a raw
/// `cntvct_el0` read costs 0.30 ns (1/cycle — the instrument is free;
/// the 24 MHz / 41.67 ns tick granularity is the real limit), and
/// an unfenced closing stamp can read up to ~50 ns early (bounded by
/// backend scheduler occupancy, not the ROB). Stamp policy:
///
/// - **Accumulated attribution** (`PhaseTimers`) uses raw [`ticks`] at
///   both ends — measured inflation ≤ 2–3% at 10 ns phases; any fence
///   costs more than it fixes (`isb` stamps measured +164%).
/// - **Single-shot spans** close with [`ticks_ss`] (`CNTVCTSS_EL0`,
///   `FEAT_ECV` — present on M2+): self-synchronized, slide-proof, 4.6 ns
///   worst case — half the price of `isb` (9.4 ns), and the only honest
///   way to time one sub-500 ns region.
#[cfg(feature = "trace")]
pub mod fastclock;

#[cfg(feature = "trace")]
mod imp {
    use super::{Category, TraceEvent, fastclock};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    thread_local! {
        static BUFFER: RefCell<Option<Vec<TraceEvent>>> = const { RefCell::new(None) };
    }

    /// The process tick anchor: trace timestamps are ns since the first
    /// stamp, from the same counter `PhaseTimers` accumulates — one
    /// timeline, coherent across spans and phase events.
    fn anchor_ticks() -> u64 {
        static ANCHOR: OnceLock<u64> = OnceLock::new();
        *ANCHOR.get_or_init(fastclock::ticks)
    }

    /// The opening stamp: raw anchor-relative ticks (0.30 ns; an
    /// early-read slide on an opening stamp only lengthens the span,
    /// bounded by ~50 ns). The anchor resolves FIRST — on the very
    /// first stamp the anchor would otherwise be read after the stamp
    /// and sit ahead of it. Ticks, not ns: `ticks_to_ns`'s u128 divide
    /// by the runtime `cntfrq` is a `__udivti3` libcall that would land
    /// inside every enclosing span's measured window — events carry raw
    /// ticks and convert once at drain, the `PhaseTimers` discipline.
    pub(super) fn now_ticks() -> u64 {
        let anchor = anchor_ticks();
        fastclock::ticks().wrapping_sub(anchor)
    }

    /// The closing stamp: self-synchronized — a raw
    /// closing stamp can read up to ~50 ns early, which is −83% on a
    /// 28 ns span; `CNTVCTSS` cannot slide. Raw anchor-relative ticks,
    /// as [`now_ticks`].
    pub(super) fn now_ticks_ss() -> u64 {
        let anchor = anchor_ticks();
        fastclock::ticks_ss().wrapping_sub(anchor)
    }

    pub(super) fn capturing() -> bool {
        BUFFER.with(|b| b.borrow().is_some())
    }

    pub(super) fn start_capture() {
        BUFFER.with(|b| {
            // Idempotent by representation: a nested (or unwound-over)
            // start extends the live capture, never destroys it — the
            // silent mid-run timeline reset was the one way this seam
            // could lie by omission.
            b.borrow_mut()
                .get_or_insert_with(|| Vec::with_capacity(4096));
        });
    }

    pub(super) fn finish_capture() -> Vec<TraceEvent> {
        let mut events = BUFFER.with(|b| b.borrow_mut().take().unwrap_or_default());
        // The one tick→ns conversion site, off every measured window:
        // in-buffer events carry raw anchor-relative ticks in the two
        // time fields until the capture ends.
        for event in &mut events {
            event.start_ns = fastclock::ticks_to_ns(event.start_ns);
            event.dur_ns = fastclock::ticks_to_ns(event.dur_ns);
        }
        events
    }

    pub(super) fn record(event: TraceEvent) {
        BUFFER.with(|b| {
            if let Some(buffer) = b.borrow_mut().as_mut() {
                buffer.push(event);
            }
        });
    }

    /// A live span: records one [`TraceEvent`] on drop, if capturing.
    pub struct SpanGuard {
        pub(super) live: Option<Live>,
    }

    pub(super) struct Live {
        pub name: &'static str,
        pub cat: Category,
        pub start_ticks: u64,
        pub a0: u64,
        pub a1: u64,
    }

    impl SpanGuard {
        /// Sets the payload args (for values known only at scope end).
        pub fn set_args(&mut self, a0: u64, a1: u64) {
            if let Some(live) = &mut self.live {
                live.a0 = a0;
                live.a1 = a1;
            }
        }

        /// Ends the span now (records the event). Equivalent to dropping,
        /// spelled for call sites that would otherwise `drop()` a guard
        /// that is a Drop-less ZST when the feature is off.
        pub fn end(self) {}
    }

    impl Drop for SpanGuard {
        fn drop(&mut self) {
            if let Some(live) = self.live.take() {
                // Tick-valued time fields until the drain converts.
                record(TraceEvent {
                    name: live.name,
                    cat: live.cat,
                    start_ns: live.start_ticks,
                    dur_ns: now_ticks_ss().saturating_sub(live.start_ticks),
                    a0: live.a0,
                    a1: live.a1,
                });
            }
        }
    }
}

#[cfg(feature = "trace")]
pub use imp::SpanGuard;

/// Whether this thread is currently capturing.
#[cfg(feature = "trace")]
#[must_use]
pub fn capturing() -> bool {
    imp::capturing()
}

/// Begins capturing on this thread. Idempotent: a nested start extends
/// the live capture (it never resets the timeline mid-run — recorded
/// events are destroyed by nothing but [`finish_capture`]'s drain).
#[cfg(feature = "trace")]
pub fn start_capture() {
    imp::start_capture();
}

/// Ends capture, returning every recorded event (empty if not capturing).
#[cfg(feature = "trace")]
#[must_use]
pub fn finish_capture() -> Vec<TraceEvent> {
    imp::finish_capture()
}

/// Opens a span; the event records when the guard drops.
#[cfg(feature = "trace")]
#[must_use]
pub fn span(name: &'static str, cat: Category) -> SpanGuard {
    span_args(name, cat, 0, 0)
}

/// Opens a span with payload args.
#[cfg(feature = "trace")]
#[must_use]
pub fn span_args(name: &'static str, cat: Category, a0: u64, a1: u64) -> SpanGuard {
    if imp::capturing() {
        SpanGuard {
            live: Some(imp::Live {
                name,
                cat,
                start_ticks: imp::now_ticks(),
                a0,
                a1,
            }),
        }
    } else {
        SpanGuard { live: None }
    }
}

/// Records a point event (duration zero).
#[cfg(feature = "trace")]
pub fn event(name: &'static str, cat: Category, a0: u64, a1: u64) {
    if imp::capturing() {
        // Tick-valued time fields until the drain converts.
        let now = imp::now_ticks();
        imp::record(TraceEvent {
            name,
            cat,
            start_ns: now,
            dur_ns: 0,
            a0,
            a1,
        });
    }
}

// ---------------------------------------------------------------------
// Feature off: identical signatures, empty bodies, ZST guard — call
// sites never write #[cfg].
// ---------------------------------------------------------------------

/// A live span (inert: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
pub struct SpanGuard;

#[cfg(not(feature = "trace"))]
impl SpanGuard {
    /// Sets the payload args (no-op: the `trace` feature is off).
    #[inline]
    pub fn set_args(&mut self, _: u64, _: u64) {}

    /// Ends the span (no-op: the `trace` feature is off).
    #[inline]
    pub fn end(self) {}
}

/// Whether this thread is currently capturing (never, feature off).
#[cfg(not(feature = "trace"))]
#[inline]
#[must_use]
pub fn capturing() -> bool {
    false
}

/// Begins capturing (no-op: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
pub fn start_capture() {}

/// Ends capture (always empty: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
#[must_use]
pub fn finish_capture() -> Vec<TraceEvent> {
    Vec::new()
}

/// Opens a span (inert: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
#[must_use]
pub fn span(_: &'static str, _: Category) -> SpanGuard {
    SpanGuard
}

/// Opens a span with args (inert: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
#[must_use]
pub fn span_args(_: &'static str, _: Category, _: u64, _: u64) -> SpanGuard {
    SpanGuard
}

/// Records a point event (no-op: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
pub fn event(_: &'static str, _: Category, _: u64, _: u64) {}

#[cfg(all(test, feature = "trace"))]
mod tests;

#[cfg(all(test, not(feature = "trace")))]
mod off_tests;
