# PRD 13 — Harness core: protocol, percentiles, windows, cold

Authority: `00-product.md` (medians; warm gated, cold reported; p99 ≤ 10 ms
budget), `30-execution.md` (the warmup/measure protocol shape, as in the
allocation gate), README rule 5.

## Purpose

The one measurement engine: warmup → measured samples → exact percentiles, with
optional allocation windows and a precisely defined cold protocol. Everything the
report prints comes from here.

## Technical direction

- `harness::Protocol { warmups: u32 /* 32 */, samples: u32 /* 256 */ }` —
  per-family overridable (writes use fewer, PRD 15).
- `harness::Stats { min, p50, p90, p95, p99, max, mean_ns }` from
  `fn stats(samples: &mut Vec<u64>) -> Stats` — sort, **nearest-rank** percentiles
  (`idx = ceil(p/100 × n) - 1`), document the method in the doc comment (numbers
  must be reproducible by hand).
- `harness::measure<F: FnMut() -> Result<u64>>(proto, f) -> Result<Measurement>`
  where `f` runs one sample and returns a checksum-ish u64 (row count) that the
  harness black-boxes via `std::hint::black_box` and sums into
  `Measurement { stats: Stats, work: u64 }` — the anti-dead-code contract: every
  runner drains its rows and returns the count.
- Timing: `Instant::now()` around exactly the call, `as_nanos() as u64`.
- Param rotation: `harness::Rotation` wraps a fixed `Vec<Vec<Value>>` and yields
  round-robin — the gate-style fixed set; misses included where a family says so.
- **Allocation window mode** (feature `alloc-counter` on the bench build): after
  warmups, `alloc_counter::reset()`; after samples, capture
  `AllocSnapshot` deltas into `Measurement.alloc: Option<AllocSnapshot>`. The
  harness refuses (`Err`) to run alloc-window and trace-capture in the same
  invocation (README rule: mutually exclusive modes).
- **Trace mode**: `harness::traced_sample(f) -> (u64, Vec<TraceEvent>)` wraps one
  additional post-measurement sample in `obs::start/finish_capture` — traces
  never contaminate the measured samples.
- **Cold protocol**, defined exactly: `harness::measure_cold(db, proto, touch, f)`
  — per sample: call `touch()` (commits one fact to a scratch relation, bumping
  the generation and evicting the cache), then time `f()` once. `warmups = 2`,
  `samples = 16` default for cold. The scratch relation is `Tag` with a unique
  seeded label per touch (never colliding with corpus labels: prefix
  `"__touch_"`).
- All engine-side comparisons in `f` closures use the caller's prepared
  statements/queries — the harness owns time, not queries.

## Non-goals

Statistical inference, outlier rejection, confidence intervals (medians +
percentiles of honest samples, per the doc). Multi-threaded load generation.

## Passing criteria

- Unit tests: `stats` against hand-computed vectors (n = 1, 2, 100; known p50/p99
  by nearest-rank); rotation order determinism; `measure` calls f exactly
  warmups+samples times and sums work correctly (counting closure); alloc-window
  mode returns a snapshot and the exclusive-mode refusal errors; `measure_cold`'s
  touch runs before every sample (instrumented closure) and generations strictly
  increase across samples (via `Db::generation`).
- `scripts/check.sh` green (bench crate builds with and without the
  `alloc-counter`/`trace` features of the engine — add a bench feature
  passthrough: `[features] obs = ["bumbledb/trace", "bumbledb/alloc-counter"]`).
