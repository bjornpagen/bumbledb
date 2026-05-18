# 03: Query Observability Data Model

**Goal**
- Add first-class query phase timing, runtime kind, node summaries, and allocation-summary fields to executed `QueryPlan`s.
- Keep timing coarse enough to avoid distorting tiny selective queries.

**Current Code Evidence**
- `ReadTxn::execute_query` currently performs validate, normalize, encode inputs, QueryImage acquisition, planning, Free Join execution, result column construction, and sink finish in one function.
- `execute_free_join` dispatches all-HashProbe plans to `execute_hash_probe`; other plans fall back to `execute_lftj`.
- `PlanCounters` captures structural events but not elapsed phase time.
- `QueryImageCacheDiagnostics` and `PlannerStatsCacheDiagnostics` expose cache totals, but not per-query phase timing.
- `OutputSink`, `EncodedProjectSink`, and `AggregateSink` are the right places to attribute sink finish/decode work.

**Required Data Types**
- Add `QueryTimings` to `QueryPlan`.
- Add `QueryRuntimeKind` with at least `Lftj`, `HashProbe`, and `MixedFallback`; reserve `DirectKernel` for `performance_kill_list/05_direct_selective_query_kernels.md`.
- Add `QueryAllocationStats` or equivalent fields to `QueryPlan`, defaulting to zero when allocation profiling is disabled.
- Add per-node timing/counter summaries at node granularity, not per candidate and not per row.
- Keep all timing values in microseconds for benchmark table consistency.

**Minimum QueryTimings Fields**
```rust
pub struct QueryTimings {
    pub total_micros: u128,
    pub validate_inputs_micros: u128,
    pub normalize_micros: u128,
    pub encode_inputs_micros: u128,
    pub query_image_micros: u128,
    pub plan_micros: u128,
    pub lftj_build_micros: u128,
    pub hash_index_micros: u128,
    pub execute_micros: u128,
    pub lftj_execute_micros: u128,
    pub hash_execute_micros: u128,
    pub sink_emit_micros: u128,
    pub sink_finish_micros: u128,
    pub decode_micros: u128,
}
```

**Minimum Allocation Fields**
```rust
pub struct QueryAllocationStats {
    pub enabled: bool,
    pub alloc_calls: u64,
    pub dealloc_calls: u64,
    pub realloc_calls: u64,
    pub bytes_allocated: u64,
    pub bytes_deallocated: u64,
    pub net_bytes: i128,
    pub peak_live_bytes: u64,
}
```

**Instrumentation Semantics**
- `total_micros` is inclusive from the beginning of `execute_query` through completed `QueryOutput` construction.
- Top-level phase fields are exclusive where practical, but this must be documented in explain output.
- `execute_micros` is inclusive runtime execution after planning and before sink finish.
- `lftj_build_micros` covers atom plan/trie preparation inside `build_lftj_atom_plans`.
- `hash_index_micros` covers hash index cache lookup/build preparation inside `build_hash_atom_indexes`.
- `sink_finish_micros` covers final projection/aggregation materialization, sorting, and decode during finish.
- `decode_micros` may be zero in the first implementation if per-decode timing would distort tiny queries; if omitted, document why and add a follow-up note.
- `sink_emit_micros` should not require an `Instant::now()` per emitted row in default mode unless benchmark overhead is proven negligible.

**Required Spans**
- `bumbledb.query.validate_inputs`
- `bumbledb.query.normalize`
- `bumbledb.query.encode_inputs`
- `bumbledb.query.image`
- `bumbledb.query.plan`
- `bumbledb.query.plan.stats`
- `bumbledb.query.plan.variable_order`
- `bumbledb.query.plan.optimize_free_join`
- `bumbledb.query.free_join.dispatch`
- `bumbledb.query.hash.build_indexes`
- `bumbledb.query.hash.execute`
- `bumbledb.query.lftj.build`
- `bumbledb.query.lftj.execute`
- `bumbledb.query.sink.emit`
- `bumbledb.query.sink.finish`
- `bumbledb.sorted_trie.build`
- `bumbledb.hash_trie.build`

**Span Field Rules**
- Span fields may include counts, relation IDs, atom IDs, variable counts, node IDs, implementation kind, cache hit/miss booleans, and elapsed micros.
- Span fields must not include raw values, input values, literal bytes, strings from user data, row payloads, interned text, or result rows.
- Query text should not be emitted in spans by default.

**Explain Output Requirements**
- `QueryPlan::explain()` must render runtime kind.
- `QueryPlan::explain()` must render a `timings:` section.
- `QueryPlan::explain()` must render allocation summary fields, even when disabled and all zero.
- Existing counters must stay visible.

**Tests**
- Default `QueryTimings` values are zero.
- A nontrivial query populates `total_micros`, `query_image_micros`, `plan_micros`, `execute_micros`, and `sink_finish_micros` with nonnegative values.
- LFTJ plans set runtime kind to `Lftj` or fallback kind as designed.
- All-hash plans set runtime kind to `HashProbe`.
- Explain output includes timing and allocation sections.
- Tests do not assert exact timing values.

**Passing Requirements**
- Every `QueryOutput.plan` includes populated timing and allocation-summary fields.
- Timing instrumentation has no measurable disabled-tracing regression above 5% on focused scale-10000 smoke.
- Existing counter gates remain intact.
- No raw query values or row payloads are emitted through tracing fields.

**Stop Conditions**
- Stop if timing requires `Instant` calls inside per-candidate LFTJ recursion by default.
- Stop if adding runtime kind creates a second public query engine abstraction.
- Stop if explain output becomes too large for benchmark use; summarize and move detail into markdown tables.
