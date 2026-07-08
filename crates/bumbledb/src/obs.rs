//! The one tracing mechanism (docs/architecture/50-validation.md):
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
    /// Executor phase accumulators (docs/architecture/50-validation.md):
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
    // Read path (docs/architecture/50-validation.md). Args noted as (a0, a1); `-` = unused.

    /// The whole prepare pipeline. (-, -)
    pub const PREPARE: &str = "prepare";
    /// IR validation. (-, -)
    pub const VALIDATE: &str = "validate";
    /// Normalization. (-, -)
    pub const NORMALIZE: &str = "normalize";
    /// Guard-vs-join classification. (-, -)
    pub const CLASSIFY: &str = "classify";
    /// Statistics reads. (occurrences measured concretely, -)
    pub const STATS: &str = "stats";
    /// The exhaustive left-deep DP. (-, -)
    pub const PLAN_DP: &str = "plan_dp";
    /// binary2fj + factor + plan validation. (-, -)
    pub const LOWER: &str = "lower";
    /// COLT construction at prepare. (-, -)
    pub const BUILD_COLTS: &str = "build_colts";

    /// One prepared execution. (result rows, -)
    pub const EXECUTE: &str = "execute";
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
    /// The Free Join executor. (-, -)
    pub const JOIN: &str = "join";
    /// Sink finalization into the result buffer. (-, -)
    pub const FINALIZE: &str = "finalize";
    /// The guard-probe access path. (1 hit / 0 miss, -)
    pub const GUARD_PROBE: &str = "guard_probe";
    /// One occurrence's selection-level probe (docs/architecture/30-execution.md).
    /// (occurrence index, 1 hit / 0 miss)
    pub const SELECT_PROBE: &str = "select_probe";

    /// Image found in the shared cache. (relation id, -)
    pub const CACHE_HIT: &str = "cache_hit";
    /// A full image decode. (relation id, slab bytes)
    pub const IMAGE_BUILD: &str = "image_build";
    /// Lost the insert race; adopted the winner's image. (relation id, -)
    pub const CACHE_ADOPT: &str = "cache_adopt";
    /// Old-generation reader built without caching. (relation id, -)
    pub const CACHE_QUERY_LOCAL: &str = "cache_query_local";
    /// One COLT node forced. (positions ingested, distinct keys)
    pub const COLT_FORCE: &str = "colt_force";
    /// One dictionary resolution in finalize — fires per *distinct*
    /// intern per finalize (docs/architecture/30-execution.md). (intern word, byte length)
    pub const DICT_RESOLVE: &str = "dict_resolve";

    // Write path (docs/architecture/50-validation.md).

    /// One state-changing commit. (1 changed / 0 no-op, -)
    pub const COMMIT: &str = "commit";
    /// A commit that netted to nothing. (-, -)
    pub const COMMIT_NOOP: &str = "commit_noop";
    /// Phase 1. (facts deleted, -)
    pub const APPLY_DELETES: &str = "apply_deletes";
    /// Phase 2. (facts inserted, -)
    pub const APPLY_INSERTS: &str = "apply_inserts";
    /// Phase 3a. (deduped forward probes, -)
    pub const FK_FORWARD: &str = "fk_forward";
    /// Phase 3b. (guards scanned, -)
    pub const FK_RESTRICT: &str = "fk_restrict";
    /// Phase 4. (pending interns flushed, -)
    pub const COUNTERS_FLUSH: &str = "counters_flush";
    /// Phase 5: the LMDB commit alone — the fsync-bound number. (-, -)
    pub const LMDB_COMMIT: &str = "lmdb_commit";
    /// One `bulk_load` chunk. (facts submitted, facts changed)
    pub const BULK_CHUNK: &str = "bulk_chunk";
    /// One `Db::write`, closure plus commit. (1 committed / 0 aborted, -)
    pub const WRITE_TXN: &str = "write_txn";

    // Harness (docs/architecture/50-validation.md, 17): tool overhead, honestly visible
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
    /// descend, force.
    pub const JOIN_PHASE: [[&str; 9]; 6] = [
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
    ];

    /// One sink-map rehash inside a measured execution. (new capacity, arity)
    pub const WORDMAP_GROW: &str = "wordmap_grow";

    /// One residency-gated phase-1.5 prefetch pass ran (docs/silicon/10).
    /// (survivors hinted, probed colt's forced footprint in bytes)
    pub const PREFETCH_PASS: &str = "prefetch_pass";
}

/// The trace-mode fast clock, under the measured cost model
/// (docs/silicon/01-timer-discipline.md, bumblebench exp 11): a raw
/// `cntvct_el0` read costs 0.30 ns (1/cycle — the instrument is free;
/// the 24 MHz / 41.67 ns tick granularity is the real constraint), and
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
    use super::{fastclock, Category, TraceEvent};
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

    /// The opening stamp: raw ticks (0.30 ns; an early-read slide on an
    /// opening stamp only lengthens the span, bounded by ~50 ns). The
    /// anchor resolves FIRST — on the very first stamp the anchor would
    /// otherwise be read after the stamp and sit ahead of it.
    pub(super) fn now_ns() -> u64 {
        let anchor = anchor_ticks();
        fastclock::ticks_to_ns(fastclock::ticks().wrapping_sub(anchor))
    }

    /// The closing stamp: self-synchronized (docs/silicon/01) — a raw
    /// closing stamp can read up to ~50 ns early, which is −83% on a
    /// 28 ns span; `CNTVCTSS` cannot slide.
    pub(super) fn now_ns_ss() -> u64 {
        let anchor = anchor_ticks();
        fastclock::ticks_to_ns(fastclock::ticks_ss().wrapping_sub(anchor))
    }

    pub(super) fn capturing() -> bool {
        BUFFER.with(|b| b.borrow().is_some())
    }

    pub(super) fn start_capture() {
        BUFFER.with(|b| {
            let mut slot = b.borrow_mut();
            debug_assert!(slot.is_none(), "nested start_capture");
            *slot = Some(Vec::with_capacity(4096));
        });
    }

    pub(super) fn finish_capture() -> Vec<TraceEvent> {
        BUFFER.with(|b| b.borrow_mut().take().unwrap_or_default())
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
        pub start_ns: u64,
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
                record(TraceEvent {
                    name: live.name,
                    cat: live.cat,
                    start_ns: live.start_ns,
                    dur_ns: now_ns_ss().saturating_sub(live.start_ns),
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

/// Begins capturing on this thread. Nested capture is a programmer error
/// (debug-asserted).
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
                start_ns: imp::now_ns(),
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
        let now = imp::now_ns();
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
    pub fn set_args(&mut self, _a0: u64, _a1: u64) {}

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
pub fn span(_name: &'static str, _cat: Category) -> SpanGuard {
    SpanGuard
}

/// Opens a span with args (inert: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
#[must_use]
pub fn span_args(_name: &'static str, _cat: Category, _a0: u64, _a1: u64) -> SpanGuard {
    SpanGuard
}

/// Records a point event (no-op: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
pub fn event(_name: &'static str, _cat: Category, _a0: u64, _a1: u64) {}

#[cfg(all(test, feature = "trace"))]
mod tests;

#[cfg(all(test, not(feature = "trace")))]
mod off_tests;
