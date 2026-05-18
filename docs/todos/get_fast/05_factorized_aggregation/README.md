# 05: Factorized Aggregation

**Goal**
- Move aggregation from post-join materialization into the variable-order execution pipeline where possible.

This stage is inspired by FAQ/InsideOut and factorized query processing. It should reduce work for aggregate-heavy queries without backing away from WCOJ.

**Thesis**
- Many aggregates do not need every complete binding materialized as a row.
- Once variables not required for grouping are below an aggregate boundary, the executor can combine contributions earlier.
- Count and sum should exploit factorization when relation constraints allow it.
- Aggregation should operate over encoded bindings and decode only aggregate operands that require logical arithmetic.

**Hard Cut**
- Do not restore decoded binding vectors as the default aggregation input.
- Do not add aggregate-specific old executor branches.
- Do not implement one-off aggregate shortcuts that bypass the WCOJ plan model.

**Aggregation Classes**
- `count`: can often count combinations without materializing all lower variables.
- `sum`: needs decoded numeric operands but can still aggregate before full projection materialization.
- `min` and `max`: can use encoded order only where encoding matches logical order.
- grouped aggregates: group keys should remain encoded until output.

**Planning Requirements**
- Identify group variables.
- Identify aggregate operand variables.
- Identify variables that are existential below the grouping boundary.
- Determine whether lower subplans can produce multiplicities instead of bindings.
- Keep exact Datalog set semantics clear; avoid bag semantics leaks.

**Execution Direction**
- Add an aggregate sink interface to the WCOJ executor.
- Allow variable levels to emit partial aggregate updates when all required group and aggregate variables are bound.
- For `count`, consider multiplicity summaries from trie cardinalities when all remaining constraints are independent enough.
- For `sum`, decode only the aggregate operand value and group key values if needed for state storage.
- Deduplicate encoded group keys before final decoding.

**Implementation Steps**
- Replace `project_aggregates` over decoded bindings with encoded aggregate states.
- Add aggregate planning metadata to `JoinHypergraph`.
- Add encoded group-key storage.
- Add numeric decode helpers for aggregate operands.
- Add correctness tests for count, sum, min, max, grouping, duplicates, and empty results.
- Add benchmark counters for aggregate updates, aggregate groups, decoded aggregate operands, and avoided bindings.

**Benchmark Targets**
- `joinstress/triangle_count`: count triangles without materializing unnecessary projected rows.
- `ledger/balances_by_instrument`: aggregate by instrument with minimal decode.
- `tpch/revenue_by_customer_range`: aggregate revenue by customer with encoded grouping and numeric-only decode.

**Passing Criteria**
- Aggregate output matches existing semantics.
- Aggregates do not require a decoded full binding vector.
- Explain output reports aggregate sink behavior and avoided materialization.
- Aggregate-heavy benchmarks show lower allocation and decode counts.

**Design Trap To Avoid**
- Do not sacrifice semantic clarity for clever aggregate shortcuts. Factorization is useful only when exact set semantics remain obvious and tested.
