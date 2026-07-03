# PRD 01 — Allocation observability v2: bytes, live, peak

Authority: `docs/architecture/00-product.md` success criterion 3, `30-execution.md`
allocation contract; the existing `crates/bumbledb/src/alloc_counter.rs`.

## Purpose

The counting allocator counts events. Data-driven memory work needs **bytes**: total
allocated/freed per window, live bytes, and peak live bytes — so the report can say
"this family's warm path holds X bytes of scratch and peaked at Y during warmup".

## Technical direction

- Extend `alloc_counter` (same feature flag `alloc-counter`, same single sanctioned
  `#[allow(unsafe_code)]` module):
  - Atomics: `ALLOCS`, `DEALLOCS` (existing), plus `ALLOC_BYTES`, `DEALLOC_BYTES`
    (window-relative, reset by `reset()`), and `LIVE_BYTES`, `PEAK_LIVE_BYTES`
    (absolute — **not** reset by `reset()`; `reset_peak()` sets peak to current
    live). All `Ordering::Relaxed` except the peak update.
  - `alloc`: add `layout.size()` to `ALLOC_BYTES` and `LIVE_BYTES`, then update peak
    with a compare-exchange loop: `loop { let p = PEAK.load(); if live <= p ||
    PEAK.compare_exchange_weak(p, live, ..).is_ok() { break } }`.
  - `dealloc`: add to `DEALLOC_BYTES`, subtract from `LIVE_BYTES` (`fetch_sub`).
  - `realloc`: account as dealloc(layout.size()) + alloc(new_size) — one alloc event
    and one dealloc event? **No**: keep the existing event semantics (realloc counts
    as one allocation event, zero dealloc events — the gate's contract today) and
    account bytes as `ALLOC_BYTES += new_size`, `DEALLOC_BYTES += layout.size()`,
    `LIVE_BYTES += new_size - old_size` (signed-safe: add then sub as two ops).
    Document this asymmetry in the module docs — events answer "did the warm path
    touch the allocator", bytes answer "how much".
  - `pub struct AllocSnapshot { pub allocs: u64, pub deallocs: u64, pub alloc_bytes:
    u64, pub dealloc_bytes: u64, pub live_bytes: u64, pub peak_live_bytes: u64 }`
    with `pub fn snapshot() -> AllocSnapshot`; keep the existing `count()` /
    `dealloc_count()` fns delegating (their reader is the gate).
- The gate (`tests/alloc_gate.rs`) additionally asserts `alloc_bytes == 0 &&
  dealloc_bytes == 0` over measured windows — byte-level tightening of the same
  contract.

## Non-goals

RSS/OS-level memory (deferred; heap peak is the in-envelope truth). Per-callsite
attribution (that is what traces + windows are for).

## Passing criteria

- Unit tests (feature on, single-threaded #[test]s in the module): allocating a
  `Vec::with_capacity(4096)` of u8 moves `alloc_bytes` by ≥4096 and `live_bytes` up
  then down on drop; peak observes a transient spike (alloc 2 MB, drop, alloc 1 KB —
  peak ≥ 2 MB); `reset()` zeroes window counters but not live/peak; realloc
  accounting test (Vec growth: alloc_bytes grows, live equals final capacity ±).
- The release-mode allocation gate passes with the new byte assertions.
- Default-feature builds contain none of this (compile check).
