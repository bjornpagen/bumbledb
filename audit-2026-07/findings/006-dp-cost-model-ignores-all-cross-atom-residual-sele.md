## DP cost model never consults cross-atom residual selectivity — Allen-joined atom pairs price as pure Cartesian products

category: perf | severity: high | verdict: CONFIRMED | finder: engine:plan-ir

### Summary

The join-order DP's per-step estimator reads only occurrence row counts, key coverage, and equality-join fanout. The four cross-atom residual lists that normalization produces — `residuals`, `word_residuals`, `allen_residuals`, `duration_residuals` — are never consulted anywhere under `src/plan/planner/`. Two interval atoms related only by an Allen mask (the canonical temporal overlap join; interval `Eq`/`Ne` also canonicalize into `allen_residuals` per the `NormalizedQuery` doc comment) share no join variables, so the estimator returns a full cross product with no credit for the mask's keep fraction. The honest fraction already exists one file over: `allen_keep` (popcount/13, justified by the JEPD partition with "no workload assumption needed") in `plan/selectivity.rs` — but it applies only to same-atom filters. The inflated estimates then flow into two real consumers: sink presizing and introspection's est/actual honesty numbers.

### Evidence (all verified against the code)

- `crates/bumbledb/src/plan/planner/estimate.rs:16-35` — the entire per-step model: `join_vars == 0` → `prefix_est.saturating_mul(r.rows)`; else key coverage → `prefix_est`; else min per-var fanout. Nothing else.
- `crates/bumbledb/src/plan/planner/densify.rs:71-76` — `OccInfo` carries `rows`, `vars`, `var_distincts`, `key_var_sets` only; no residual data ever reaches the DP.
- `crates/bumbledb/src/plan/planner/plan.rs:63` — `estimate(prev.est, mask_vars[...], &occs, last)`; `normalized` is in scope (line 18) but only its occurrences are used. `grep -rn 'residual\|Allen' src/plan/planner/` returns zero hits.
- `crates/bumbledb/src/ir/normalize.rs:325-340` — `NormalizedQuery`'s four cross-atom residual lists; lines 333-335 confirm "interval `Eq`/`Ne` comparisons canonicalize here too."
- `crates/bumbledb/src/plan/fj/validate.rs:289-325` — every residual kind attaches to the earliest node binding both sides, where execution genuinely compacts survivors (40-execution.md, vectorized-execution section: "residuals run as batch survivor compaction after the probes"). The shrinkage is real at runtime; the DP just never sees it.
- `crates/bumbledb/src/plan/selectivity.rs:36-51` — `allen_keep`: literal mask keeps popcount/13 ("the mask's measure in the coordinate system, no workload assumption needed"); param masks take `RANGE_KEEP_DEN` (line 34). Applied only inside `occurrence_estimate` over `occurrence.filters` (same-atom position, lines 180-186).
- `crates/bumbledb/src/api/prepared/build.rs:479, 485` — `plan.estimates().last().copied().unwrap_or(0).min(1 << 21)` sizes the shared sink; an Allen-connected pair's cross-product estimate slams the 2M cap.
- Bench-lane reality: `crates/bumbledb-bench/src/scenarios/temporal.rs:211-235` (t3, `mixed_mask`) — two `Span` atoms whose only shared position is a **param** (`key = ?0`), so they share no variables; the sole connection is `Allen(u, v, DURING ∪ MEETS)`. This is exactly the `join_vars == 0` shape: priced as est(A)·est(B) while the true output is ~2/13 of that. t2 (`overlap_join`, lines 155-182) shares a key variable but its INTERSECTS residual is likewise unpriced on top of the fanout.

### Spec check (docs/architecture/40-execution.md, "Join cardinality estimator, written down")

The doc records the cross-product rule as a decision: "Neither: estimate = |L| × |R| — **no estimate exists, so pessimism**, which pushes non-key joins last; that is the correct behavior, not a modeling failure." The code follows the doc, so this is not code/spec divergence — it is a gap in both, and the doc's rationale is now false by the repo's own later doctrine: for a literal Allen mask an estimate **does** exist (popcount/13, the JEPD measure), documented and shipped at `selectivity.rs:36-42` for the identical predicate in same-atom position. The estimator section prices no residual class at the join step at all.

### Failure scenario / bench impact

Two corrections to the original claim, then the surviving impact:

1. The DP is not literally position-blind for cross products — placing one early inflates every later prefix estimate, so the summed cost does push disconnected atoms late. The defect is that it does so on residual-blind numbers: a popcount-1 Allen pair (true keep 1/13) is deferred exactly like a genuine Cartesian product, and among orders that all contain the pair the relative costs of binding both residual sides early vs. late are wrong by 13/popcount compounded through every subsequent step. This distorts order choice for ≥3-atom temporal queries (A(iv1), B(iv2), C(x) with Allen(iv1, m, iv2) and B–C equality: the step joining the Allen pair is priced |·|·|B| regardless of the mask).
2. On the shipped two-atom temporal lanes order choice is trivial, but the two downstream consumers are live today: (a) `output_hint` presizes the sink from the inflated last estimate — t3's cross-product estimate over-allocates by ~6.5× (13/2) up to the 2M-row cap; (b) introspection's per-node est/actual (40-execution.md § plan introspection) is structurally wrong by 13/popcount (or 4× for range/PointIn/measure residual classes) on every residual-bearing plan, which poisons exactly the honesty numbers the repo uses to diagnose estimator drift (the estimate.rs doc comment's own "misled introspection by 12,703x" precedent).

### Suggested fix

Make residuals data the DP reads (representation over control flow): at `densify` time, translate each cross-atom residual into `(var_bitset, keep_num, keep_den)` on a new field of the planner input — literal Allen masks get popcount/13, param masks and PointIn/order/duration get the existing `RANGE_KEEP_DEN` class, `Ne` gets 1/1 (all constants already defined in `plan/selectivity.rs`). In `estimate()`, after the fanout product, apply the keep fraction of every residual whose bitset becomes fully covered by `prefix_vars | r.vars` but was not covered by `prefix_vars` — the exact moment `fj/validate.rs`'s earliest-bound-node rule fires it at runtime. Clamp to `[1, ..]` per the selectivity.rs discipline. Update 40-execution.md's estimator section: the "no estimate exists" clause no longer holds for residual-connected joins.
