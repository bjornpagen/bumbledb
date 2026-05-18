# 06: Tracing And Benchmark Gates

**Goal**
- Make the new executor observable and lock performance wins behind benchmark gates.

This stage is not generic logging. It is instrumentation for the encoded trie architecture.

**Thesis**
- The old traces were noisy because they mirrored recursive atom execution.
- The new traces should summarize variable-level work and trie intersection behavior.
- Performance regressions should be caught by generated benchmarks before adding new features.

**Hard Cut**
- Do not preserve atom-recursive debug spans.
- Do not emit per-candidate trace data at debug level.
- Do not accept performance-sensitive refactors without benchmark comparison.

**Counters To Add**
- Variables planned.
- Variable levels entered.
- Candidate values produced per variable.
- Trie seeks per variable and per index.
- Trie advances per variable and per index.
- Intersection rounds.
- Prefix existence checks.
- Encoded comparisons evaluated and failed.
- Decoded comparisons evaluated and failed.
- Values decoded by type.
- Dictionary reverse lookups by kind.
- Projection rows before and after deduplication.
- Aggregate updates and groups.

**Trace Shape**
- One debug event for query planning summary.
- One debug event per variable level summary after execution.
- One debug event for projection or aggregate summary.
- Trace-level events only for focused single-query debugging.

**Explain Shape**
- Variable order and rationale.
- Physical trie constraints per variable.
- Estimated versus actual candidate counts.
- Missing/rejected index permutations if applicable.
- Decode/materialization counts.
- Output row counts.

**Benchmark Gates**
- Keep the full generated suite at `scale=2000 repeats=10` as a comparison point.
- Add focused single-query benchmark commands for each broad query.
- Track untraced timings as the primary performance metric.
- Track traced counters as the diagnostic metric.
- Store benchmark interpretation in `docs/BENCHMARKS.md` after each major get-fast stage.

**Initial Performance Targets**
- `triangle_count`: reduce nested seek-equivalent work by an order of magnitude.
- `tag_lookup_join`: eliminate full `PostingTag` primary scan once physical index stage lands.
- `red_boat_sailors`: eliminate full `Boat` primary scan once physical index stage lands.
- `supplier_nation_orders`: eliminate full `Supplier` primary scan once physical index stage lands.
- Selective point/range queries should not regress by more than a small constant factor while the new executor stabilizes.

**Implementation Steps**
- Replace `PlanCounters` with WCOJ-oriented counters.
- Update `QueryPlan::explain` to variable-order format.
- Add trace summary emission after query execution.
- Update benchmark parser expectations if tests inspect explain output.
- Add benchmark documentation snapshots after each major refactor.

**Passing Criteria**
- `RUST_LOG=bumbledb_lmdb=debug` gives readable summaries for broad joins.
- Debug traces are no longer dominated by recursive atom completion spam.
- Benchmark output clearly shows WCOJ counters.
- Performance gates are documented and reproducible.

**Design Trap To Avoid**
- Do not optimize for pretty traces. Optimize the executor, then make the important work visible.
