# PRD 15: Optimizer Cover Cost And Direct Kernel Consolidation

## 01. Status

Not started.

## 02. Severity

High optimizer architecture.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRDs 11 through 14 first.

The implementer must not add uncosted direct paths.

The implementer must not leave fake optimizer candidates.

The implementer must keep optimizer traces deterministic.

## 04. Dependency Order

PRD 11 is mandatory.

PRD 12 is mandatory.

PRD 13 is mandatory.

PRD 14 is strongly recommended.

PRD 10 is mandatory if aggregate candidates are costed.

PRD 16 depends on final optimizer counters and benchmark gates.

## 05. Problem Statement

The optimizer does not yet cost the full Free Join design space.

It chooses variable order.

It does not choose covers per node.

It does not cost lazy build work accurately.

It does not cost result-set work separately from witness work.

Direct kernels can bypass the optimizer.

The direct materialized path has fake cost.

The aggregate pushdown candidate can be fake.

Planner stats can overstate distinctness.

These issues prevent reliable performance work.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/free_join.rs`.
- `crates/bumbledb-lmdb/src/planner_stats.rs`.
- `crates/bumbledb-lmdb/src/query_access.rs`.
- `crates/bumbledb-lmdb/src/hash_trie.rs`.
- `crates/bumbledb-lmdb/src/query_image.rs`.

Relevant current regions:

- `query.rs:3977-4031` for direct materialized preselection and fake cost.
- `query.rs:6719-7008` for variable cost estimation.
- `query.rs:7207-7445` for candidate generation and estimates.
- `planner_stats.rs:182-280` for field and access estimates.
- `query_access.rs:7-32` for narrow access abstraction.
- `hash_trie.rs:216-218` and `hash_trie.rs:343-349` for recursive count.

## 07. Current Behavior

The optimizer generates candidates such as pure LFTJ.

Direct materialized execution can be selected before normal proof and planning.

Direct materialized candidate records estimated micros as one.

Aggregate pushdown candidate can appear without a distinct implementation.

Variable ordering uses sampled field stats and simple access stats.

One-field non-unique access paths can be estimated as unique due to forced final depth distinct count.

Hash trie prefix counts recursively sum subtrees.

Plan costs do not distinguish result-set work from full binding work.

## 08. Target Behavior

All physical strategies compete as optimizer candidates.

Direct paths are Free Join node implementations or removed.

Every selected candidate has a real implementation.

Every candidate has a real cost model, even if approximate.

Cover choice is explicit.

Lazy build cost is modeled.

Vectorized execution cost is modeled if PRD 14 is complete.

Set-output work is modeled separately from witness work.

Optimizer trace explains selection and rejection.

## 09. Research Context

Free Join's strength is exploring the space between binary-style plans and WCOJ-style plans.

That requires choosing covers and probe groupings.

Generic Join typically iterates the smallest key set.

Binary plans often avoid building on the left input.

COLT changes this tradeoff by making some builds lazy.

Set projection and aggregate domains further change the cost model because full witness output is not the target.

Bumbledb needs a cost model centered on result-set and domain-event work.

## 10. Desired Cost Dimensions

Estimated cover key count.

Estimated probe count.

Estimated lazy node forces.

Estimated eager build facts.

Estimated copied bytes.

Estimated projected result facts.

Estimated aggregate domain events.

Estimated completed bindings.

Estimated predicate evaluations.

Estimated memory bytes.

Estimated cache reuse probability if available.

## 11. Cover Enumeration Plan

For each Free Join node, enumerate valid cover choices.

A cover must bind the node's new variables.

A cover must be backed by an available access source.

A cover can be a relation scan, compact access image, lazy GHT node, sorted trie, or hash source.

For each cover, enumerate required probes.

Reject covers that require unavailable variables.

Record cover candidates in optimizer trace.

Choose cover by stable cost key.

## 12. Direct Kernel Consolidation Plan

Inventory every direct kernel.

Map each direct kernel to a Free Join node implementation if possible.

Delete direct kernels that duplicate Free Join behavior without unique value.

If a direct kernel remains, it must enter optimizer candidate generation normally.

No direct kernel may be selected before static proof and normal candidate costing unless explicitly proven safe and costed.

Remove fake direct cost of one microsecond.

Update tests that assert direct runtime kind.

## 13. Aggregate Candidate Plan

If PRD 10 implemented real aggregate-domain execution, add a real aggregate candidate.

If not, remove aggregate pushdown candidate entirely.

Do not leave a candidate that shares implementation with pure LFTJ and differs only by name.

Aggregate candidate cost must include domain-event estimates.

Aggregate candidate cost must not estimate full witness output as required work when early events suffice.

## 14. Planner Stats Plan

Rename sampled stats so they are not documented as exact.

Compute actual prefix distinct counts from compact access images when available.

Use conservative estimates when only samples are available.

Do not force final depth distinct count to fact count for non-unique one-field access paths.

Track heavy hitters if useful but do not overfit initial implementation.

Add subtree counts to hash trie nodes to make prefix count cheap.

Expose stats provenance in diagnostics: exact, sampled, conservative.

## 15. Cost Key Plan

Extend `CostKey` only if needed.

Keep ordering deterministic.

Include setup cost.

Include expected execution work.

Include memory bytes.

Include materialization penalty.

Include witness-work penalty for set-output queries.

Include candidate rank only as tie-breaker, not fake preference.

Do not hide missing implementation behind low cost.

## 16. Required Optimizer Trace

List candidate family.

List node implementations.

List selected covers.

List estimated cover key counts.

List estimated probes.

List estimated result facts or domain events.

List estimated completed bindings.

List estimated memory bytes.

List rejection reason.

Trace must be stable across runs.

## 17. Required Tests

Direct-prefix/range shape appears as normal candidate.

Direct-prefix/range candidate can win when cheap.

Pure LFTJ can win when appropriate.

Lazy GHT candidate can win when it avoids build work.

Aggregate candidate is absent unless it is real.

Non-unique one-field access is not estimated as unique without exact evidence.

Cover candidates are shown in trace.

Plan selection is deterministic under tie.

## 18. Required Stats Tests

Sampled field stats are marked sampled.

Exact access prefix stats are used when compact access image exists.

Conservative stats are used when exact stats are absent.

Hash trie prefix count uses stored subtree counts after implementation.

Non-unique access path estimate reflects duplicates.

Heavy hitter tests remain deterministic if present.

## 19. Required Benchmark Tests

Run focused clover-like query and verify factored/lazy plan selection.

Run acyclic chain query and verify optimizer does not force WCOJ-style plan unnecessarily.

Run cyclic query and verify LFTJ or Free Join intersection path remains available.

Run projection duplicate-witness query and verify witness-work penalty affects plan.

Run aggregate-domain duplicate-witness query and verify domain-event cost affects plan.

Exact correctness must pass before timing.

## 20. Passing Criteria

No direct kernel bypasses optimizer costing.

No fake aggregate pushdown candidate remains.

Cover choices are explicit and traced.

Cost model distinguishes result-set/domain work from completed binding work.

Planner stats no longer claim sampled values are exact.

Non-unique access estimates are not forced unique.

The global validation gate passes.

The query-focused validation gate passes.

## 21. Failure Modes

Keeping fake cost for direct materialized path is a failure.

Leaving aggregate pushdown as a name-only candidate is a failure.

Selecting unimplemented node variants is a failure.

Using candidate rank to hide bad estimates is a failure.

Regressing deterministic optimizer traces is a failure.

Optimizing for full witness work on projection-only queries is a failure.

## 22. Non-Goals

Do not add external optimizer dependencies.

Do not add runtime SQL planning.

Do not add approximate cardinality estimators that affect correctness.

Do not add parallel execution.

Do not change storage layout.

Do not change public APIs.

## 23. Completion Notes

Document cost dimensions in planner docs.

Document any remaining direct kernels and why they remain.

Keep optimizer trace tests permanent.

This PRD makes the engine capable of choosing among real set-native Free Join strategies.
