## Ring-closing joins price the closing atom by its single best column, ignoring the conjunction of join variables

category: perf | severity: medium | verdict: PLAUSIBLE | finder: engine:plan-ir

### Summary

The planner's per-step cardinality model (`estimate`) prices a new occurrence's fanout as the **minimum** over per-join-variable `rows / distinct` values. When the occurrence shares multiple variables with the prefix — the closing atom of every cyclic/ring query — this uses only the single most selective column and ignores that the atom is constrained on all of them simultaneously. The repo's own pinned test shows the closing node of a 3-cycle estimated at 576 against an actual of 192 (best one-column fanout 3 vs true pair fanout 1). The mechanism is real and the misestimate enters DP plan choice, not just introspection. However, this is a **recorded and ruled-on** limitation, and the suggested independence-product fix trades the current pessimism for optimism under column correlation, against the codebase's written doctrine — so this stands as a plausible design/perf finding, not a confirmed defect.

### Evidence

- `crates/bumbledb/src/plan/planner/estimate.rs:25-35` — the general arm:
  ```rust
  let fanout = r.var_distincts.iter()
      .filter(|(bit, _)| bit & join_vars != 0)
      .map(|(_, distinct)| (r.rows / (*distinct).clamp(1, r.rows.max(1))).max(1))
      .min()
      .unwrap_or_else(|| r.rows.max(1));
  prefix_est.saturating_mul(fanout)
  ```
  Only the `key_var_sets` fast path (line 22) composes multiple variables, and only for exact key coverage. No pair/compound distinct statistic exists.
- `crates/bumbledb/src/plan/selectivity.rs:884-918` — test `cyclic_estimate_diagnosis_is_p3_not_a_domain_or_range_defect` pins `(24,24), (192,192), (576,192)` on the full-head 3-cycle, with the message "the closing two-variable probe uses its best one-column fanout 3 instead of pair fanout 1". Verified against the fixture (selectivity.rs:800-828): CYCLE_C has 24 rows, distinct(z)=8, distinct(x)=3 → min fanout 3; independence product 24/(8·3)=1 = actual.
- `docs/architecture/40-execution.md:665-666` — the DP is exhaustive, left-deep, and minimizes the **sum of prefix estimates**; the closing term differs per candidate closing atom, so the error participates in order selection.
- `docs/architecture/40-execution.md:1040-1059` — the estimator record classifies this exact behavior as P3 ("closing two-variable independence error"), cites the pinned test, and rules: "Cyclic estimates are not governed by a fixed factor: they order the exhaustive DP while the WCOJ execution bounds the chosen plan's damage... **No histograms or new tuning rung are earned by this diagnosis.**"
- `docs/free-join-paper/arXiv-2301.10841v2/tex/05-eval.tex` (robustness experiment, "modifying its cardinality estimator to always return 1") — the executed algorithm family degrades under bad estimates chiefly via bushy plans materializing large intermediates; bumbledb's DP is left-deep only, so the paper's worst failure mode is structurally excluded, supporting the doc's "bounds the damage" position.
- `crates/bumbledb-bench/src/families/read.rs:401-441` — the actual ring lane (`triangle`) is a Posting **self-join** on (account, instrument), (instrument, entry), (entry, account). Every closing candidate is the same relation; under the independence product each closer's fanout would clamp to 1 (distinct(account)·distinct(instrument) etc. exceed rows), so the claimed "DP can now distinguish the right closer" benefit does not obviously materialize on this lane — the discrimination shifts to prefix terms, which may or may not change the chosen order.

### Bench impact

Every 3+-atom cyclic query's final DP term is inflated by (second-best single-column fanout / true multi-column fanout) — 3x on the pinned fixture, unbounded under skew. Because the inflation factor differs per candidate closing atom, the DP can in principle prefer an order with larger true intermediates because its closer's *best single column* looks tighter. No concrete mis-chosen ring plan or bench regression was demonstrated; the doc's recorded 4761.9x triangle introspection factor is an execution-work ratio dominated by D2 cancellation semantics (the narrow-head test executes 576/24), not purely this estimator error.

### Suggested fix

If acted on, compose per-variable constraints multiplicatively under independence — `fanout ≈ rows / Π clamped distincts`, clamped to `[1, rows]` — in the general arm only (the `key_var_sets` path already handles exact coverage, including compound keys). This uses zero new statistics, so it is arguably outside the doc's "no histograms or new tuning rung" ruling. But note the trade: `min` is the pessimistic composition and the product underestimates under column correlation (plausible in the bench triangle's self-join), which collides with the codebase's stated doctrine at estimate.rs:31-33 ("optimism without evidence is how plans go wrong") and 40-execution.md:658. A middle path consistent with the doctrine: keep `min` as the bound but record the product as a tie-break, or revisit only if a real ring lane is shown to pick a dominated order. Any change must re-pin `cyclic_estimate_diagnosis_is_p3_not_a_domain_or_range_defect` and the 40-execution.md estimator record together.
