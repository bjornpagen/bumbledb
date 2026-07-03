//! The one tracing mechanism (docs/benchmarks/02-trace-core.md):
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
    // Read path (docs/benchmarks/03). Args noted as (a0, a1); `-` = unused.

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

    /// Image found in the shared cache. (relation id, -)
    pub const CACHE_HIT: &str = "cache_hit";
    /// A full image decode. (relation id, rows)
    pub const IMAGE_BUILD: &str = "image_build";
    /// Lost the insert race; adopted the winner's image. (relation id, -)
    pub const CACHE_ADOPT: &str = "cache_adopt";
    /// Old-generation reader built without caching. (relation id, -)
    pub const CACHE_QUERY_LOCAL: &str = "cache_query_local";
    /// One COLT node forced. (positions ingested, distinct keys)
    pub const COLT_FORCE: &str = "colt_force";
}

#[cfg(feature = "trace")]
mod imp {
    use super::{Category, TraceEvent};
    use std::cell::RefCell;
    use std::sync::OnceLock;
    use std::time::Instant;

    thread_local! {
        static BUFFER: RefCell<Option<Vec<TraceEvent>>> = const { RefCell::new(None) };
    }

    fn anchor() -> Instant {
        static ANCHOR: OnceLock<Instant> = OnceLock::new();
        *ANCHOR.get_or_init(Instant::now)
    }

    pub(super) fn now_ns() -> u64 {
        u64::try_from(anchor().elapsed().as_nanos()).expect("process uptime fits u64 ns")
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
                    dur_ns: now_ns() - live.start_ns,
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
mod tests {
    use super::*;

    #[test]
    fn nested_spans_record_containment_in_drop_order() {
        start_capture();
        {
            let mut outer = span("outer", Category::Execute);
            std::hint::black_box(1 + 1);
            {
                let _inner = span_args("inner", Category::Execute, 7, 9);
                std::hint::black_box(2 + 2);
            }
            outer.set_args(42, 0);
        }
        let events = finish_capture();
        assert_eq!(events.len(), 2);
        // Drop order: inner lands first.
        let (inner, outer) = (&events[0], &events[1]);
        assert_eq!(inner.name, "inner");
        assert_eq!(outer.name, "outer");
        assert_eq!((inner.a0, inner.a1), (7, 9));
        assert_eq!(outer.a0, 42, "set_args landed");
        assert!(outer.start_ns <= inner.start_ns);
        assert!(inner.start_ns + inner.dur_ns <= outer.start_ns + outer.dur_ns);
    }

    #[test]
    fn point_events_record_zero_duration_and_args() {
        start_capture();
        event("tick", Category::Cache, 3, 4);
        let events = finish_capture();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].dur_ns, 0);
        assert_eq!((events[0].a0, events[0].a1), (3, 4));
    }

    #[test]
    fn nothing_records_outside_capture() {
        {
            let _span = span("ghost", Category::Execute);
            event("ghost-event", Category::Execute, 0, 0);
        }
        assert!(!capturing());
        start_capture();
        let events = finish_capture();
        assert!(events.is_empty());
    }

    #[test]
    fn sequential_captures_are_independent() {
        start_capture();
        event("first", Category::Harness, 0, 0);
        let a = finish_capture();
        start_capture();
        event("second", Category::Harness, 0, 0);
        let b = finish_capture();
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a[0].name, "first");
        assert_eq!(b[0].name, "second");
    }
}

#[cfg(all(test, not(feature = "trace")))]
mod off_tests {
    #[test]
    fn the_guard_is_a_zst_when_off() {
        assert_eq!(std::mem::size_of::<super::SpanGuard>(), 0);
    }
}
