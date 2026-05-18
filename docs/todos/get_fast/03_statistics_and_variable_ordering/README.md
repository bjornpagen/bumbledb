# 03: Statistics And Variable Ordering

**Goal**
- Replace syntactic atom ordering with a statistics-backed variable-order optimizer for the WCOJ executor.

Stage 01 gives the engine the right execution primitive. This stage teaches it to choose the right variable order.

**Thesis**
- WCOJ performance depends on variable order and available trie constraints.
- Relation row counts are not enough.
- The optimizer needs prefix cardinalities, distinct counts, fanout estimates, and selectivity estimates over encoded components.
- For cyclic joins, cost should reason about hypergraph structure and intermediate candidate domains, not only binary join order.

**Hard Cut**
- Delete purely syntactic variable ordering as the default.
- Do not reintroduce relation-at-a-time join ordering.
- Do not choose indexes independently of variable order.
- Do not add statistics that are disconnected from optimizer decisions.

**Statistics To Store Or Compute**
- Relation row count.
- Index entry count.
- Distinct count per leading prefix component.
- Distinct count for common two-component prefixes.
- Fanout distribution for prefix to next component.
- Min/max encoded values for range-indexed components.
- Optional equi-depth samples for high-cardinality scalar domains.

Because schemas are hardcoded and ETL is explicit, bulk-load can compute stronger stats than a generic mutable OLTP system.

**Optimizer Inputs**
- Typed query hypergraph.
- Available index permutations.
- Input and literal constraints.
- Comparison predicates and range constraints.
- Relation and prefix stats.
- Projection and aggregation shape.

**Cost Model Direction**
- Estimate candidate domain size per variable.
- Estimate constraint intersection cost per variable level.
- Penalize variable orders that force unconstrained scans.
- Penalize variable orders that require unavailable index permutations.
- Prefer orders that bind high-selectivity literals and inputs early.
- Prefer orders that close cycles early when this lowers the AGM-style bound.

This does not need a perfect global optimum. It needs to avoid obvious disasters and serve the trie executor.

**Hypergraph Reasoning**
- Model each atom as a hyperedge over variables.
- Track which atoms become active as variables are bound.
- Track when an atom can become an existence check rather than a candidate generator.
- Use worst-case bound thinking for cyclic queries.
- Prefer variable orders where every new variable is constrained by existing bound context and at least one selective trie stream.

**Implementation Steps**
- Add persisted or rebuildable stats metadata keyed by schema fingerprint and index layout.
- Compute stats during bulk load.
- Update stats on writes only if needed for current benchmark scope; otherwise document stale/ETL-only stats explicitly.
- Add `StatsProvider` at the query planner boundary.
- Replace `variable_order` with costed variable-order selection.
- Add optimizer explain output: candidate estimates, chosen order, rejected alternatives, required indexes.
- Add deterministic tie-breaking.

**Benchmark Targets**
- `triangle_count`: choose an order that reduces repeated `EdgeBC` work and closes the cycle through trie intersections.
- `red_boat_sailors`: start from `color` or a selective boat domain once the physical index exists.
- `tag_lookup_join`: start from `tag` once the physical index exists.
- `supplier_nation_orders`: start from `nation` once the physical index exists.
- `revenue_by_customer_range`: weigh `nation` against `ship_date` range using stats rather than source-order atoms.

**Passing Criteria**
- Explain output contains variable order estimates and actual counters.
- Optimizer rejects unavailable ideal orders by naming missing index permutations.
- Costed order is deterministic.
- No query falls back to the old atom-order planner.
- Broad benchmark plans are explainable from stats rather than incidental source order.

**Design Trap To Avoid**
- Do not build a classic binary join optimizer and call it done. This executor is variable-at-a-time; the optimizer must be variable-at-a-time.
