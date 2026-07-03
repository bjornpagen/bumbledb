# PRD 02 — Trace core: spans, events, capture, the `trace` feature

Authority: `docs/architecture/00-product.md` (no always-on instrumentation in release
paths; `30-execution.md` observability section), README rule 5 (zero-cost off).

## Purpose

The one tracing mechanism: nanosecond spans and point events, recorded into a
thread-local buffer during explicit capture, drained by tooling. Everything later
(read/write instrumentation, Chrome traces, flame summaries) is this seam plus names.

## Technical direction

- New feature `trace` on `bumbledb`. New module `src/obs.rs` (`pub mod obs` — it is
  a documented, feature-gated public surface; note it in `60-api.md`'s observability
  paragraph, added by this PRD).
- Types (feature on):
  - `#[derive(Clone, Copy, Debug)] pub struct TraceEvent { pub name: &'static str,
    pub cat: Category, pub start_ns: u64, pub dur_ns: u64, pub a0: u64, pub a1: u64 }`
    — `dur_ns == 0` means a point event. Two u64 payload args, meaning defined per
    name (rows, bytes, hit/miss as 1/0…). **Names are `&'static str` only** — no
    allocation to record.
  - `pub enum Category { Prepare, Execute, Storage, Commit, Image, Cache, Harness }`.
  - Clock: `fn now_ns() -> u64` — `Instant` delta against a `OnceLock<Instant>`
    process anchor. Monotonic by construction.
- API (feature on):
  - `pub fn start_capture()` / `pub fn finish_capture() -> Vec<TraceEvent>` — a
    thread-local `RefCell<Option<Vec<TraceEvent>>>`; `start` installs an empty vec
    (capacity 4096), `finish` takes it. Nested `start` while capturing is a
    programmer error (debug_assert).
  - `pub fn span(name: &'static str, cat: Category) -> SpanGuard` — records
    `start_ns` now; `Drop for SpanGuard` pushes the completed event. When not
    capturing, `span` reads one thread-local flag and returns an inert guard.
  - `pub fn span_args(name, cat, a0, a1) -> SpanGuard` and `pub fn event(name, cat,
    a0, a1)`. SpanGuard exposes `set_args(a0, a1)` for values known only at scope end.
  - `pub fn capturing() -> bool`.
- Feature **off**: the whole module still exists with the same signatures;
  `TraceEvent`/`Category` compile; `span`/`event` are `#[inline(always)]` empty;
  `SpanGuard` is a unit struct (ZST, no Drop). `finish_capture` returns an empty vec.
  This keeps call sites cfg-free — instrumented code never writes `#[cfg]`.
- Buffer growth allocates: **sanctioned only because capture is never enabled inside
  a measured allocation window** — the gate never calls `start_capture`. State this
  in module docs; the harness (PRD 13) enforces trace-capture and alloc-window as
  mutually exclusive modes of a run.
- Names registry: `pub mod names` in `obs.rs` — every instrumentation point's name
  constant lives here (`pub const EXECUTE: &str = "execute";` …), added by the PRDs
  that instrument. No string literals at instrumentation sites: typo-proof.

## Non-goals

Cross-thread trace merging (single-threaded queries; the harness records its own
thread). Sampling profilers, PMU counters (humans run Instruments when they want
those).

## Passing criteria

- Unit tests (feature on): nested spans record correct containment
  (`outer.start <= inner.start`, `inner end <= outer end`) and reverse-drop order;
  `event` records dur 0 and args; nothing records outside capture; two sequential
  captures are independent; `set_args` lands.
- Feature-off compile test in CI-shape: `cargo check -p bumbledb` (default features)
  plus a `#[cfg(not(feature = "trace"))]` test asserting
  `size_of::<obs::SpanGuard>() == 0`.
- Release-mode allocation gate green (it does not capture). `60-api.md` gains the
  observability paragraph. `scripts/check.sh` green.
