//! The one tracing mechanism:
//! nanosecond spans and point events recorded into a thread-local buffer
//! during explicit capture, drained by tooling — Chrome-trace export and
//! flame summaries are this seam plus names.
//! **Zero-cost when off**: under default features every
//! Recording allocates (the capture buffer grows): sanctioned only
//! no `Drop`; instrumented call sites are written once, `#[cfg]`-free.

mod point;

pub use point::{TraceArgs, TracePoint, names};

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

    Phase,
}

impl Category {
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

/// One recorded span or point event. Payloads are [`TraceArgs`] — unused
/// is [`TraceArgs::None`], not `0`. Time fields are nanoseconds in every
/// drained event; inside a live capture buffer they hold raw
/// anchor-relative ticks until [`finish_capture`] converts once per
/// event, off the measured windows (the `PhaseTimers` discipline).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEvent {
    Span {
        point: TracePoint,
        start_ns: u64,
        dur_ns: u64,
        args: TraceArgs,
    },
    Point {
        point: TracePoint,
        start_ns: u64,
        args: TraceArgs,
    },
}

impl TraceEvent {
    #[must_use]
    pub const fn point(self) -> TracePoint {
        match self {
            Self::Span { point, .. } | Self::Point { point, .. } => point,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.point().label()
    }

    #[must_use]
    pub const fn cat(self) -> Category {
        self.point().category()
    }

    #[must_use]
    pub const fn start_ns(self) -> u64 {
        match self {
            Self::Span { start_ns, .. } | Self::Point { start_ns, .. } => start_ns,
        }
    }

    #[must_use]
    pub const fn dur_ns(self) -> u64 {
        match self {
            Self::Span { dur_ns, .. } => dur_ns,
            Self::Point { .. } => 0,
        }
    }

    #[must_use]
    pub const fn args(self) -> TraceArgs {
        match self {
            Self::Span { args, .. } | Self::Point { args, .. } => args,
        }
    }

    #[must_use]
    pub const fn a0(self) -> u64 {
        self.args().a0()
    }

    #[must_use]
    pub const fn a1(self) -> u64 {
        self.args().a1()
    }
}

/// The trace-mode fast clock, under the measured cost model: a raw
/// `cntvct_el0` read costs 0.30 ns (1/cycle — the instrument is free;
/// the 24 MHz / 41.67 ns tick granularity is the real limit), and
/// an unfenced closing stamp can read up to ~50 ns early (bounded by
/// backend scheduler occupancy, not the ROB). Stamp policy:
/// - **Accumulated attribution** (`PhaseTimers`) uses raw [`ticks`] at
/// both ends — measured inflation ≤ 2–3% at 10 ns phases; any fence
/// costs more than it fixes (`isb` stamps measured +164%).
#[cfg(feature = "trace")]
pub mod fastclock;

#[cfg(feature = "trace")]
mod imp {
    use super::{TraceEvent, fastclock};
    use std::cell::RefCell;
    use std::sync::OnceLock;

    thread_local! {
        static BUFFER: RefCell<Option<Vec<TraceEvent>>> = const { RefCell::new(None) };
    }

    fn anchor_ticks() -> u64 {
        static ANCHOR: OnceLock<u64> = OnceLock::new();
        *ANCHOR.get_or_init(fastclock::ticks)
    }

    /// first stamp the anchor would otherwise be read after the stamp

    pub(super) fn now_ticks() -> u64 {
        let anchor = anchor_ticks();
        fastclock::ticks().wrapping_sub(anchor)
    }

    pub(super) fn now_ticks_ss() -> u64 {
        let anchor = anchor_ticks();
        fastclock::ticks_ss().wrapping_sub(anchor)
    }

    pub(super) fn capturing() -> bool {
        BUFFER.with(|b| b.borrow().is_some())
    }

    pub(super) fn start_capture() {
        BUFFER.with(|b| {
            b.borrow_mut()
                .get_or_insert_with(|| Vec::with_capacity(4096));
        });
    }

    pub(super) fn finish_capture() -> Vec<TraceEvent> {
        let mut events = BUFFER.with(|b| b.borrow_mut().take().unwrap_or_default());

        for event in &mut events {
            match event {
                TraceEvent::Span {
                    start_ns, dur_ns, ..
                } => {
                    *start_ns = fastclock::ticks_to_ns(*start_ns);
                    *dur_ns = fastclock::ticks_to_ns(*dur_ns);
                }
                TraceEvent::Point { start_ns, .. } => {
                    *start_ns = fastclock::ticks_to_ns(*start_ns);
                }
            }
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
        pub point: super::TracePoint,
        pub start_ticks: u64,
        pub args: super::TraceArgs,
    }

    impl SpanGuard {
        pub fn set_args(&mut self, args: super::TraceArgs) {
            if let Some(live) = &mut self.live {
                live.args = args;
            }
        }

        pub fn set_count(&mut self, n: u64) {
            self.set_args(super::TraceArgs::Count(n));
        }

        pub fn set_pair(&mut self, a0: u64, a1: u64) {
            self.set_args(super::TraceArgs::Pair(a0, a1));
        }

        pub fn set_flag(&mut self, flag: bool) {
            self.set_args(super::TraceArgs::Flag(flag));
        }

        /// spelled for call sites that would otherwise `drop()` a guard
        /// that is a Drop-less ZST when the feature is off.
        pub fn end(self) {}
    }

    impl Drop for SpanGuard {
        fn drop(&mut self) {
            if let Some(live) = self.live.take() {
                record(TraceEvent::Span {
                    point: live.point,
                    start_ns: live.start_ticks,
                    dur_ns: now_ticks_ss().saturating_sub(live.start_ticks),
                    args: live.args,
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

/// Opens a span; the event records when the guard drops. Payload starts
/// as [`TraceArgs::None`] until [`SpanGuard::set_args`].
#[cfg(feature = "trace")]
#[must_use]
pub fn span(point: TracePoint) -> SpanGuard {
    span_args(point, TraceArgs::None)
}

/// Opens a span with a payload known at entry.
#[cfg(feature = "trace")]
#[must_use]
pub fn span_args(point: TracePoint, args: TraceArgs) -> SpanGuard {
    if imp::capturing() {
        SpanGuard {
            live: Some(imp::Live {
                point,
                start_ticks: imp::now_ticks(),
                args,
            }),
        }
    } else {
        SpanGuard { live: None }
    }
}

/// Records a point event (duration zero).
#[cfg(feature = "trace")]
pub fn event(point: TracePoint, args: TraceArgs) {
    if imp::capturing() {
        let now = imp::now_ticks();
        imp::record(TraceEvent::Point {
            point,
            start_ns: now,
            args,
        });
    }
}

/// A live span (inert: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
pub struct SpanGuard;

#[cfg(not(feature = "trace"))]
impl SpanGuard {
    #[inline]
    pub fn set_args(&mut self, _: TraceArgs) {}

    #[inline]
    pub fn set_count(&mut self, _: u64) {}

    #[inline]
    pub fn set_pair(&mut self, _: u64, _: u64) {}

    #[inline]
    pub fn set_flag(&mut self, _: bool) {}

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
pub fn span(_: TracePoint) -> SpanGuard {
    SpanGuard
}

/// Opens a span with args (inert: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
#[must_use]
pub fn span_args(_: TracePoint, _: TraceArgs) -> SpanGuard {
    SpanGuard
}

/// Records a point event (no-op: the `trace` feature is off).
#[cfg(not(feature = "trace"))]
#[inline]
pub fn event(_: TracePoint, _: TraceArgs) {}

#[cfg(all(test, feature = "trace"))]
mod tests;

#[cfg(all(test, not(feature = "trace")))]
mod off_tests;
