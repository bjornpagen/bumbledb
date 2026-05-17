# 06: Planner, Executor, And Aggregation

**Goal**
- Execute typed positive Datalog over current indexes using access-path-aware plans, joins, filters, projection, and v0 aggregation.

**Why This Stage Exists**
- This is the central database thesis: Datalog-shaped joins over sorted covering indexes.
- The first implementation should be correct and explainable before becoming clever.

**Concrete Work**
- Enumerate available access paths for each relation atom.
- Implement variable-ordering heuristics from the Rosetta Stone.
- Choose an index per relation atom based on constants, inputs, already-bound variables, and range predicates.
- Implement simple direct indexed walks for highly selective paths.
- Implement a basic trie/cursor multiway join path for many-way joins.
- Push comparison filters to the earliest point where their variables are bound.
- Execute projection with Datalog set semantics.
- Implement `count`, `sum`, `min`, and `max` aggregations.
- Implement overflow behavior for integer and decimal aggregation.
- Add plan counters needed for explain output, even if explain rendering is finalized in the next stage.
- Add correctness tests comparing query output against direct storage reads or a simple in-memory reference evaluator.

**Out Of Scope**
- Recursive rules.
- Stratified negation.
- Hash join as a first-class operator.
- Query plan caching.
- Compile-time query macros.
- As-of query execution.
- Ordered output and limit.
- Spill-to-LMDB temporary relations.

**Passing Criteria**
- Single-relation queries execute correctly.
- Two-relation joins execute correctly.
- Many-relation joins execute correctly on the benchmark-style schema.
- Equality joins over typed refs use ref indexes when available.
- Range predicates use range indexes when available.
- Queries still return correct results when falling back to primary scans.
- Projection returns set semantics.
- Aggregation returns correct grouped results.
- Decimal and integer aggregation overflow is detected.
- Plans are deterministic enough for golden tests unless stats intentionally change them.
- The executor does not decode full tuples earlier than needed for common indexed paths.

**Notes**
- Correctness beats theoretical optimality in the first executor.
- WCOJ capability is required, but not every query must use it.
- If the trie join abstraction becomes too large, ship direct indexed joins first and keep the WCOJ work scoped to the smallest useful many-way case.
