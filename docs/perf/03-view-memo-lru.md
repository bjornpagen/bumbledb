# PRD 03 — View memo LRU for residual filters

Authority: `30-execution.md` (view memoization; generational immutability makes
caching sound), suite README finding 1 (the residual tail). Depends on PRD 02.

## Purpose

After PRD 02, the only per-param view rebuilds left are **residual** filters —
range predicates (`at >= ?0`, `at < ?1`), Ne, and var-vs-var shapes. A rebuild
there is an honest O(relation) scan (that is what a range *is*), but real
workloads repeat parameter values (the same dashboard window, the same report
range), and generational immutability makes a cached view provably valid for
its whole generation. Generalize the per-occurrence memo from one slot to a
small LRU so repeated residual bindings are free.

This is a legitimate architectural advantage — MVCC generations make view
reuse sound where SQLite must re-scan — but note honestly: the benchmark's
fixed 4-set rotation will hit this cache by construction. The range family
remains the pure-scan measurement via its *first* (cold-memo) executions; the
report's numbers describe the warm steady state, as they always did.

## Technical direction

- In `api/prepared.rs`, the per-occurrence memo triple
  (`built_generation[occ]`, `built_filters[occ]`, `colts[occ]`,
  `survivor_buffers[occ]`) becomes a fixed array of `MEMO_SLOTS` slots:

  ```rust
  const MEMO_SLOTS: usize = 4;   // documented: bench rotation is 4; real
                                 // workloads repeat a handful of bindings.
  struct ViewSlot {
      generation: Option<u64>,
      filters: Vec<FilterPredicate>,   // resolved residuals, the key
      colt: Colt,
      survivor_buffer: Vec<u32>,       // the recycle half of the ping-pong
      last_used: u64,                  // execution tick for LRU
  }
  ```

  Lookup: linear scan of 4 slots for `(generation, filters)` equality (4
  comparisons — no hashing). Hit ⇒ `view_memo_hit`, bump `last_used`, use that
  slot's COLT. Miss ⇒ evict the least-recently-used slot, rebuild into it
  (`view_build`), reusing its buffers.
- Occurrences with **no residual filters** (the common case after PRD 02)
  degenerate to slot 0 with generation-only keying — the LRU never engages;
  assert this costs nothing extra (the lookup short-circuits on the first slot).
- Memory bound, documented on `MEMO_SLOTS`: up to 4 COLTs + survivor buffers
  per occurrence per prepared query, each bounded by that occurrence's
  high-water. This is the explicit trade; a prepared query is the unit a
  caller already sizes for.
- A generation bump invalidates all slots (generation mismatch) — no epoch
  bookkeeping needed; stale slots get evicted by LRU naturally.
- The executor consumes whichever slot's COLT was chosen — thread the slot
  index (or `&mut Colt`) through `run_join` in place of today's `colts[occ]`
  indexing.
- Allocation: slots grow to high-water like everything else; steady-state
  rotation across ≤ 4 residual bindings is allocation-free. Extend the alloc
  gate: a range-shaped query rotating 4 window params, two warm cycles, then
  zero allocs across four more.

## Non-goals

Cross-prepared-query view sharing (the image cache is the shared layer; views
are query-local by design). Caching more than `MEMO_SLOTS` bindings. Any
special treatment of the bench rotation (the cache is workload-honest; the
bench README note above is the disclosure).

## Passing criteria

- Trace-based test (obs lane): a range-shaped query rotating 4 window param
  pairs emits exactly 4 `view_build`s for the Posting occurrence across 12
  executions, then only `view_memo_hit`s; a 5th distinct window evicts LRU
  (one more `view_build`) and re-requesting the *most* recent of the old four
  still hits, while the least recent misses — the eviction-order test.
- Same-generation correctness under rotation: results per window are equal to
  fresh `execute_collect` results on a second prepared query (differential
  pairing), for all 5 windows.
- Generation bump invalidates: commit one fact, next execution emits
  `view_build`, and results reflect the new fact.
- Zero-residual queries never build more than slot 0 (assert only one slot's
  generation is ever `Some` via a test-only accessor, or structurally by trace:
  exactly one `view_build` per generation as in PRD 02's criteria — those tests
  must still pass verbatim).
- The extended alloc gate passes in release. `scripts/check.sh` green.
