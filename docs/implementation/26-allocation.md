# PRD 26 — Allocation Discipline and the Counting Allocator

Authority: `docs/architecture/30-execution.md` (the allocation contract + CI gate
protocol), `00-product.md` (success criterion 3).

## Purpose

Make the zero-allocation contract mechanically checkable, and bring the hot path into
compliance.

## Technical direction

- `alloc_counter` module (test-support feature-gated, `#[cfg(feature =
  "alloc-counter")]`): a global-allocator wrapper counting allocations/deallocations
  with `fn reset()`, `fn count() -> u64` — thread-naive by design; the protocol is
  single-threaded (doc rule).
- An in-crate integration test implementing the doc's gate protocol as a *unit-level*
  contract of `PreparedQuery::execute` (this is the module's own contract, not an e2e
  suite): fixture schema + committed data; prepare; N=8 warmup executions over a
  fixed param set; reset counter; M=8 measured executions with caller-provided
  buffer; assert count == 0. Run for: a join query, an aggregate query, a guard-probe
  query, across batch sizes.
- **The compliance work**: fix every allocation the counter finds in the measured
  window — expected offenders: hidden `Vec` growth in probe scratch, sink map
  rehashing (size maps to high-water at prepare/warmup, rehash allowed only during
  warmup), `HashMap` default hasher state, format!/error paths on non-error
  executions, iterator adapters that box. Each fix stays within existing designs
  (arena reuse, presizing) — **no new caching layers, no unsafe** to chase zeros.
- Document (module comment) exactly which executions are sanctioned to allocate:
  first-after-prepare, first-after-commit (rebuild), buffer growth.

## Non-goals

CI wiring (human-owned). Benchmarks. Cross-thread attribution.

## Passing criteria

- The gate test passes: zero allocations across measured executions for all three
  query shapes and all batch sizes.
- Warmup-phase allocation is *finite and convergent*: two consecutive warmup rounds
  allocate, then the third is zero (asserted).
- No public API changed; global commands green with and without the feature.
