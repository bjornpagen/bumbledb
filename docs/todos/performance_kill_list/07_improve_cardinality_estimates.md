# 07: Improve Cardinality Estimates

**Goal**
- Replace local-only variable estimates with propagated per-node cardinality estimates and actual-vs-estimated reporting.

**Problem**
Current `NodeRowEstimate.estimated_rows` is local candidate count per parent, while `actual_rows` is cumulative accepted rows across all invocations. This mixes units.

Examples:
- `tag_lookup_join`: node 2 estimates `1`, actual `10000` because one account lookup happens per posting.
- `red_boat_sailors`: `sailor` estimates `20`, actual `16660` because fanout multiplies by boat count.
- `triangle_count`: downstream local estimates `3`, actual hundreds/thousands depending scale.

**Required Design**
Extend node estimates with explicit units:

```rust
estimated_invocations
estimated_local_candidates
estimated_local_accepted
estimated_cumulative_rows
actual_invocations
actual_candidates
actual_rows
estimate_reason
```

Add `binding_rows` to `PlanEstimates` so global aggregate `output_rows=1` does not hide pre-aggregate binding work.

**Propagation Rules**
- First node `estimated_invocations=1`.
- Node N invocations equal previous node cumulative rows.
- Cumulative rows equal invocations times local accepted fanout.
- Unique/primary full-key lookup caps local accepted rows at 1.
- Equality literals use heavy hitters when present, otherwise rows/distinct.
- Range predicates use min/max when available.
- Multi-subatom intersections use conservative overlap bounded by smallest stream.

**Actuals**
Executor should count:
- node visits/invocations
- actual candidates before predicates
- actual accepted rows after predicates

Explain should print error ratios such as `under=200.00x`, `over=0.50x`, or `exact`.

**Benchmark Output**
Add markdown table:

```markdown
## Cardinality Estimates
| dataset | query | node | variable | est invocations | est local | est cumulative | actual invocations | actual candidates | actual rows | error |
```

Main table should include estimated/actual bindings and output rows.

**Implementation Steps**
1. Extend `NodeRowEstimate` and `PlanEstimates`.
2. Add `estimate_node_cardinalities` after variable ordering.
3. Feed propagated estimates into candidate costing.
4. Update executor actual counters per node.
5. Update explain and benchmark markdown.
6. Add estimate quality gate notes.

**Tests**
- Single FK chain multiplies downstream lookup count by upstream cardinality.
- `tag_lookup_join` estimates account lookups near posting fanout.
- `red_boat_sailors` models reserve fanout from selected boats.
- `triangle_count` uses cumulative downstream estimates.
- Markdown emits cardinality table.

**Acceptance Criteria**
- `PlanEstimates` separates `binding_rows` and `output_rows`.
- `NodeRowEstimate` no longer compares local estimates to cumulative actuals.
- Scale-2000 benchmark markdown includes cardinality estimate table.
- Focused queries no longer have silent >10x underestimates without a gate note.
- Existing correctness and benchmark gates pass.

**Risks**
- Uniform assumptions fail on correlated data.
- Better estimates can change plan selection before all runtimes are optimized.
- Range selectivity from min/max is crude without histograms.
