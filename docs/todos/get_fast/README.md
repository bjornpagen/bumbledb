# Get Fast Mission

This mission replaces the current query execution architecture with an encoded trie, variable-at-a-time, worst-case-optimal join engine.

The objective is not to tune the current nested-loop executor. The objective is to remove it.

**Architectural Bet**
- The foundational performance problem is relation-tuple-at-a-time execution.
- The replacement model is variable-at-a-time execution over encoded trie views of LMDB covering indexes.
- The engine should keep values encoded until a value must be returned, aggregated, or passed to a comparison that cannot run on encoded bytes.
- Indexes, statistics, tracing, and aggregation should be designed around this execution model.

**Non-Negotiables**
- No compatibility path for the recursive nested-loop executor.
- No runtime switch between old and new engines.
- No duplicated planner paths.
- No broad optimization pass that preserves full-row decode as the default query primitive.
- No blind index proliferation before the variable-order executor states which permutations it needs.
- No performance claim without benchmark evidence.

**Priority Order**
- `01_encoded_trie_wcoj_executor`: replace relation-at-a-time recursion with encoded trie WCOJ.
- `02_encoded_bindings_and_late_materialization`: make encoded values and field-demand analysis the default execution representation.
- `03_statistics_and_variable_ordering`: choose variable orders from cardinalities, fanout, and selectivity.
- `04_index_permutations_and_access_layouts`: add only the physical permutations the new executor needs.
- `05_factorized_aggregation`: aggregate inside the variable-order pipeline where possible.
- `06_tracing_and_benchmark_gates`: add operator-level summaries and benchmark gates for the new architecture.
- `07_delete_old_executor_and_harden`: remove leftovers, simplify APIs, and harden the replacement.

**Benchmark Baseline**
- `joinstress/triangle_count`: `64.65ms`, `24001` seeks, `34000` rows scanned.
- `ledger/tag_lookup_join`: `11.97ms`, `2001` seeks, `8000` rows scanned.
- `sailors/red_boat_sailors`: `11.52ms`, `3487` seeks, `7140` rows scanned.
- `sailors/high_rating_red_boats`: `11.36ms`, `3487` seeks, `7140` rows scanned.
- `tpch/supplier_nation_orders`: `6.60ms`, `1431` seeks, `4288` rows scanned.
- `tpch/revenue_by_customer_range`: `6.02ms`, `801` seeks, `4000` rows scanned.

**Success Bar**
- Broad joins stop scaling with nested cursor reopen counts.
- Cyclic joins use trie intersections, not repeated relation scans.
- Entity lookups avoid full-row decode unless fields are demanded.
- Explain output reports variable-order work, trie seeks, intersections, encoded candidates, and decode counts.
- The codebase has one query executor.
