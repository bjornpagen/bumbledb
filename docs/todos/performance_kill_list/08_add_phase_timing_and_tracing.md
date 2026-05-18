# 08: Add Phase Timing And Tracing

**Goal**
- Add fine-grained phase timing so future performance work can be attributed to image build, stats, planning, atom-index construction, LFTJ execution, sink emit, and sink finish.

**Problem**
The trace currently exposes only coarse `free join query planned` and `free join query executed` events. We inferred setup vs execution by timestamp subtraction, but cannot split:
- QueryImage build inside `execute_query`
- stats collection
- variable ordering
- optimizer candidate selection
- per-atom LFTJ plan construction
- sorted trie builds
- traversal
- sink emit/finish

**Required Data Model**
Add `QueryTimings` to `QueryPlan`:

```rust
QueryPhaseTimings {
  total_micros,
  validate_inputs_micros,
  normalize_micros,
  encode_inputs_micros,
  query_image_micros,
  plan_micros,
  lftj_build_micros,
  lftj_execute_micros,
  sink_finish_micros,
}
```

Add planner, LFTJ build, LFTJ node, and sink sub-timing structs.

**Required Spans**
- `bumbledb.query.validate_inputs`
- `bumbledb.query.normalize`
- `bumbledb.query.encode_inputs`
- `bumbledb.query.plan.stats`
- `bumbledb.query.plan.variable_order`
- `bumbledb.query.plan.optimize_free_join`
- `bumbledb.query.lftj.build`
- `bumbledb.query.lftj.build_atom`
- `bumbledb.query.lftj.execute`
- `bumbledb.query.lftj.node`
- `bumbledb.query.sink.emit`
- `bumbledb.query.sink.finish`
- `bumbledb.sorted_trie.build`

No raw values, strings, bytes, or row payloads in span fields.

**Counters To Add**
- stats fields/indexes/trie builds
- LFTJ atom plans built
- LFTJ atom source/included rows
- sink emit calls
- project rows before/after dedup
- aggregate encoded/decoded updates

**Benchmark Markdown**
Add sections:
- `## Phase Timing`
- `## LFTJ Build Timing`
- `## LFTJ Node Timing`

Interpretation guide:
- high image us => QueryImage/segment build bottleneck
- high stats us => optimizer stats/trie stats bottleneck
- high LFTJ build us => per-query atom trie construction bottleneck
- high LFTJ exec us with trie seeks/key reads => intersection bottleneck
- high sink time => dedup/aggregation/decode bottleneck

**Implementation Steps**
1. Add timing structs to query plan.
2. Time top-level phases in `execute_query`.
3. Time planner sub-phases.
4. Time `build_lftj_atom_plan` row filtering, column build, and trie build.
5. Time LFTJ node traversal at summary granularity, not per candidate.
6. Time sink emit/finish/decode/update.
7. Render timings in explain and benchmark markdown.
8. Update tracing docs.

**Tests**
- Timings default to zero.
- Nontrivial query populates total/plan/build/execute sections.
- Explain renders timing sections.
- Markdown renders timing tables.
- Trace subscriber test verifies key span names if feasible.

**Acceptance Criteria**
- Every `QueryOutput.plan` includes populated `QueryTimings`.
- Benchmark markdown includes phase timing tables.
- Trace-enabled benchmark emits phase/operator summaries without per-candidate spam.
- Existing counter gates remain intact.
- Disabled tracing introduces no material benchmark regression beyond timing noise.

**Risks**
- Too many `Instant` calls in recursion can distort tiny queries; measure coarse blocks only.
- Inclusive/exclusive timing must be documented.
- Tests must not assert exact timing values.
