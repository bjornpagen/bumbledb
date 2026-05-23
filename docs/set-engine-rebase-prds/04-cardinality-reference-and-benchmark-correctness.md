# PRD 04: Cardinality, Reference, And Benchmark Correctness

## 01. Status

Not started.

## 02. Severity

High correctness and validation reliability.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must understand the difference between aggregate values and result-set cardinality.

The implementer must update tests before trusting benchmark timing output.

The implementer must not use benchmark timing as evidence until correctness gates are strengthened.

The implementer must not change public semantics to match broken benchmark behavior.

## 04. Dependency Order

PRD 03 must be complete before aggregate benchmark expectations are final.

This PRD must be complete before PRD 09 and PRD 10 performance work.

This PRD must be complete before PRD 16 benchmark gates.

Storage PRDs may proceed in parallel only if benchmark correctness files are not modified.

## 05. Problem Statement

The engine now has a distinct API for result-set cardinality.

That API must never be confused with aggregate count values.

The reference evaluator must be exact for every supported aggregate type.

The benchmark harness must not time one path while validating another incompatible path.

Several current issues undermine those requirements.

Static-empty global count materialized output returns one result fact containing zero.

Static-empty cardinality-only output currently returns zero result facts.

The reference evaluator can treat unsupported aggregate states as count.

Benchmark `compare-mode facts` times cardinality execution but does not prove parity for every output shape.

Benchmark SQL aggregate references must match explicit set-domain semantics.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-test-support/src/reference.rs`.
- `crates/bumbledb-bench/src/main.rs`.
- `crates/bumbledb-bench/src/open.rs` if benchmark SQL is stored there.

Relevant current regions:

- `query.rs:2305-2408` for cardinality static-empty returns.
- `query.rs:3825-3859` for materialized static-empty output facts.
- `reference.rs:367-378` for aggregate state creation fallback.
- `bumbledb-bench/src/main.rs:920-982` for benchmark correctness and timing setup.

## 07. Current Semantics

`QueryOutput` returns a materialized result set.

`QueryResultSet.cardinality()` returns number of result facts.

`QueryResultCardinality.cardinality` must mean number of result facts.

A global aggregate count query over empty input returns one result fact.

That one result fact contains the aggregate value zero.

Therefore its result-set cardinality is one.

The aggregate value is zero.

The result-set cardinality is one.

Those values must not be confused.

## 08. Concrete Static-Empty Failure

Query: `find_count_domain([x])` over an empty relation.

Materialized execution returns result facts `[[0]]`.

Materialized result-set cardinality is `1`.

Cardinality-only static-empty path returns `0` today.

That is incorrect because it reports zero result facts.

The correct cardinality-only result is `1`.

The value inside the result fact remains zero.

Cardinality-only API does not return that value.

## 09. Reference Evaluator Failure

The reference evaluator has aggregate states.

It supports count, signed integer sum, decimal sum, min, and max.

It currently falls through to count for unsupported aggregate/type combinations.

The engine supports unsigned integer sum.

The reference evaluator can treat unsigned integer sum as count.

This can make differential tests pass or fail for the wrong reason.

Unsupported aggregate/type combinations must be hard errors.

Every supported engine aggregate/type combination must be implemented explicitly in the reference evaluator.

## 10. Benchmark Harness Failure

Benchmark materialized correctness compares Bumbledb values against SQLite values.

That is good.

In `compare-mode facts`, benchmark timing can use cardinality execution.

The harness then wraps the cardinality plan with the materialized result for reporting.

If cardinality execution is wrong but materialized execution is right, timing can still be reported.

That hides correctness bugs in the path being timed.

The harness must prove cardinality parity before accepting timing.

## 11. SQL Aggregate Risk

SQLite defaults are not Bumbledb's explicit set-domain semantics.

Plain SQL aggregate over a join can aggregate duplicate witnesses.

Bumbledb aggregate over an explicit domain must aggregate once per domain key.

Every aggregate benchmark SQL reference must express the same domain.

This usually requires a domain-distinct subquery.

For count domains, SQL should count distinct domain keys.

For sums over domains, SQL should select one measure per distinct valid domain key.

For min and max over domains, SQL should apply over domain-determined values.

Benchmark SQL must be audited query by query.

## 12. Research Context

Set engines must validate exact result values before performance numbers matter.

Free Join performance claims depend on query semantics being identical across algorithms.

If a cardinality path is timed while a materialized path is validated, the benchmark no longer validates the timed algorithm.

If SQL references use ordinary join multiplicity, they can punish correct set-domain execution or bless incorrect witness aggregation.

This PRD makes the benchmark harness a guardrail for the remaining rebase.

## 13. Desired Invariants

Materialized and cardinality-only execution agree on result-set cardinality.

Aggregate values are not confused with result-set cardinality.

Static-empty shortcuts preserve the output-shape cardinality contract.

Reference evaluator implements every supported aggregate explicitly.

Reference evaluator errors on unsupported aggregate/type combinations.

Benchmark timing paths are correctness-checked before timing is reported.

SQLite references mirror explicit Bumbledb aggregate domains.

Benchmark JSON records enough information to audit the correctness mode.

## 14. Engine Implementation Plan

Add `empty_output_cardinality(output: &OutputPlan) -> usize`.

Use this helper in static-empty cardinality cache-hit path.

Use this helper in static-empty cardinality proof path.

Use this helper anywhere else static-empty cardinality is constructed.

For global count plans, helper returns one.

For projection output, helper returns zero.

For grouped aggregate output over empty input, helper returns zero.

For global non-count aggregate output, follow documented semantics and test it explicitly.

Do not change materialized `empty_output_facts` unless a test proves it is wrong.

## 15. Reference Implementation Plan

Add `SumU64(u64)` to reference aggregate state.

Initialize `SumU64` for `AggregateFunction::Sum` with `ValueType::U64`.

Implement checked addition for unsigned integer sum.

Return aggregate overflow error on `u64` overflow.

Remove the fallback that returns count for unsupported aggregate/type pairs.

Replace fallback with explicit error.

Add tests for every supported aggregate/type combination.

Add tests for unsupported combinations if they can be constructed.

## 16. Benchmark Implementation Plan

During benchmark setup, always materialize once for correctness.

When `compare-mode facts` is selected, execute cardinality once before timing samples.

Compare cardinality-only result to materialized result-set cardinality.

Reject the benchmark run if they differ.

Record `cardinality_parity_checked` in output.

Record `materialized_result_cardinality` in output.

Record `cardinality_result_cardinality` when available.

Keep exact SQLite value comparison mandatory.

Keep SQLite result-count comparison mandatory.

Do not report timing samples after any correctness mismatch.

## 17. SQL Reference Plan

Audit every benchmark query with aggregate output.

Identify Bumbledb aggregate domain variables.

Write SQL that constructs the same domain set.

For `count_domain`, use distinct domain key count.

For `count_distinct`, use distinct value count.

For `sum`, use a domain-distinct subquery before summing.

For `min`, use a domain-distinct subquery before min if joins can duplicate domain keys.

For `max`, use a domain-distinct subquery before max if joins can duplicate domain keys.

Document the domain in benchmark metadata or comments.

Add fixture data with duplicate witnesses to prove SQL is correct.

## 18. Required Engine Tests

Static-empty global count materialized result has one result fact.

Static-empty global count cardinality-only result reports one.

Static-empty projection materialized result has zero result facts.

Static-empty projection cardinality-only result reports zero.

Static-empty grouped aggregate materialized result has zero result facts.

Static-empty grouped aggregate cardinality-only result reports zero.

Static-empty cache-hit path and proof path are both covered.

Prepared cardinality path is covered.

Non-prepared cardinality path is covered.

## 19. Required Reference Tests

Reference `sum(u64)` returns the arithmetic sum.

Reference `sum(u64)` detects overflow.

Reference `sum(i64)` remains correct.

Reference `sum(decimal)` remains correct.

Reference `count_domain` remains correct.

Reference `count_distinct` remains correct.

Reference `min` remains correct.

Reference `max` remains correct.

Unsupported aggregate/type combinations error explicitly.

## 20. Required Benchmark Tests

Benchmark helper rejects cardinality parity mismatch.

Benchmark JSON includes cardinality parity fields.

Benchmark aggregate SQL test includes duplicate existential witnesses.

Benchmark aggregate SQL result matches Bumbledb set-domain result.

`compare-mode facts` remains supported for non-aggregate projection queries.

`compare-mode facts` remains supported for aggregate queries only after parity check.

## 21. Required Output Fields

Add or verify `correctness_mode`.

Add `cardinality_parity_checked`.

Add `materialized_result_cardinality`.

Add `cardinality_result_cardinality` when the cardinality path is used.

Add aggregate domain description for aggregate benchmark queries if practical.

Do not remove existing timing fields.

Do not remove existing plan diagnostics.

## 22. Passing Criteria

Materialized and cardinality-only output cardinalities match for all tested output shapes.

Static-empty global count cardinality is fixed.

Reference evaluator implements unsigned integer sum correctly.

Reference evaluator has no silent aggregate fallback.

Benchmark timing path is correctness-checked before timing is accepted.

Aggregate SQL references mirror explicit domains.

The global validation gate passes.

The query-focused validation gate passes.

## 23. Failure Modes

Returning aggregate value from cardinality-only API is a failure.

Returning zero result facts for global count over empty input is a failure.

Keeping a reference aggregate fallback to count is a failure.

Timing cardinality execution without parity check is a failure.

Using plain SQL aggregate over duplicate witnesses is a failure.

Changing Bumbledb semantics to match SQLite defaults is a failure.

Suppressing benchmark correctness errors to report timing is a failure.

## 24. Non-Goals

Do not optimize cardinality execution.

Do not implement aggregate pushdown.

Do not add new benchmark datasets.

Do not change public aggregate function names.

Do not change result-set ordering.

Do not change storage layout.

## 25. Completion Notes

Update benchmark docs if output JSON fields change.

Keep parity tests permanent.

Keep duplicate-witness SQL fixtures permanent.

This PRD is a validation foundation for all later performance work.
