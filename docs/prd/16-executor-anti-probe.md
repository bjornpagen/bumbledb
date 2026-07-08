# PRD 16 — Executor: anti-probes

**Depends on:** 15.
**Modules:** `crates/bumbledb/src/exec/run/` (node loop), `crates/bumbledb/src/exec/colt/`, `crates/bumbledb/src/exec/explain/`.
**Authority:** `docs/architecture/40-execution.md` (§ inputs from normalization — anti-probe bullet; § vectorized execution — compaction).

## Goal

Negated atoms execute: at their attached node, each surviving binding probes the
negated occurrence; a hit rejects the binding. Batched, branchless, on the residual
compaction machinery.

## Technical direction

1. **Evaluation point:** the node loop's residual step gains a sibling step: after
   residual compaction, for each anti-probe attached to this node, probe and
   compact (anti-probe hits are removed — the inverted polarity of a positive
   probe miss; reuse the survivor-compaction cursor-write, inverting the keep
   condition).
2. **The probe:** the negated occurrence owns a COLT over its relation's image
   (or filtered view — its own literal/param/set/membership bindings became its
   filter list in PRD 13, evaluated as an ordinary filtered view). Probe with the
   binding's values for the occurrence's bound fields, in the trie schema's level
   order; any leaf reachable ⇒ hit. Two-phase batching applies exactly as for
   positive probes (phase 1 hash the batch, phase 2 issue loads) — route through
   the existing batch probe path rather than writing a scalar loop; the only new
   code is the inverted compaction predicate and the "existence, not
   continuation" early-out (an anti-probe never descends past confirmation —
   `get` to the last level, never `iter`).
3. **View memo / generation discipline:** the negated occurrence participates in
   the prepared query's view-memo LRU like any occurrence (it has a view and a
   COLT; memoized per (generation, resolved filters)). No special casing.
4. **EXPLAIN:** per-node anti-probe selectivity (probed vs rejected counts) joins
   the counted execution report; the `Counters` trait gains the two counters
   (Noop impl stays zero-sized).
5. **The shared-primitive note:** commit's judgment probes (PRDs 07–09) run
   against LMDB guards, not COLT — the sharing is the *semantic* ("no fact
   matches") and the compaction machinery, not a common function. Add a comment
   at the anti-probe entry point cross-referencing `commit/judgment.rs` and
   `40-execution.md`'s "one mechanism, two callers" sentence so the next reader
   doesn't hunt for a nonexistent shared module.

## Out of scope

Param-set probes and membership kernels (17); sinks (18).

## Passing criteria

- `[shape]` Anti-probe evaluation reuses the batch probe path and the compaction
  cursor-write; no per-tuple `if` on probe results exists in the hot loop.
- `[shape]` The early-out: anti-probes call `get`-style confirmation, never
  leaf iteration.
- `[test]` Correctness family (in the executor's correctness test style):
  postings-without-tag over constructed data (some tagged, some not, some
  multiply-tagged — multiplicity must not resurrect a rejected binding);
  negated atom with a literal binding (rejects only matching-kind facts);
  zero-binding negated atom as emptiness gate (nonempty relation ⇒ empty result;
  empty relation ⇒ passthrough); negation under an aggregate (fold domain
  excludes rejected bindings); the outer-join idiom pair returns
  complementary sets (their union sizes sum to |A ⋈ B| + |A − πB|).
- `[test]` Batch-size equality: results identical across batch sizes
  1/2/64/256/partial on every fixture above (the existing equality harness).
