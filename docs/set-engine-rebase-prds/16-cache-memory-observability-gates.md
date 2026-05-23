# PRD 16: Cache, Memory, Observability, And Benchmark Gates

## 01. Status

Not started.

## 02. Severity

High operational safety and performance governance.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer should complete PRDs 01 through 15 first.

The implementer must not add new unbounded caches.

The implementer must add deterministic tests for accounting.

The implementer must keep exact correctness gates mandatory for benchmarks.

## 04. Dependency Order

This is the final PRD in the suite.

PRD 08 provides query-image byte accounting foundations.

PRD 09 provides projection work counters.

PRD 10 provides aggregate event counters.

PRD 13 provides lazy GHT counters.

PRD 14 provides vectorization counters.

PRD 15 provides optimizer trace and plan cost dimensions.

## 05. Problem Statement

The engine has multiple snapshot-local and query-shape caches.

Several are unbounded.

Diagnostics expose hit/miss counts but not enough memory accounting.

Benchmarks report timings but do not yet enforce all set-engine waste gates.

After the rebase, performance must be governed by hard counters.

Those counters must expose result-set work versus witness work.

Cache memory must have explicit budgets or explicit bounded policies.

Benchmark output must make regressions obvious.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/query_image.rs`.
- `crates/bumbledb-lmdb/src/sorted_trie.rs`.
- Free Join lazy access code.
- `crates/bumbledb-bench/src/main.rs`.
- `crates/bumbledb-bench/src/open.rs`.
- `crates/bumbledb-test-support` tests.

Relevant current regions:

- `query.rs:292-344` for prepared normalized query cache.
- `query_image.rs:533-605` for prepared plan cache.
- `query_image.rs:1239-1319` for query image cache.
- `query.rs:1290-1339` for plan counters.
- Benchmark JSON and renderer code in `bumbledb-bench/src/main.rs`.

## 07. Current Behavior

Query image cache stores images keyed by schema, tx id, and scope.

Prepared plan cache stores plans keyed by query shape.

Prepared query stores normalized query cache by transaction ID.

Planner caches store reusable planning outcomes.

Sorted trie and lazy access caches exist under query image internals.

Caches expose hit and miss counters.

Caches generally do not expose byte budgets.

Caches generally do not evict by memory budget.

Benchmark output includes many timings and counters but not all new set-engine work dimensions.

## 08. Target Behavior

Every major cache has item count diagnostics.

Every major cache has byte count diagnostics where practical.

Every major cache has an explicit budget or documented bounded lifecycle.

Eviction counters exist when eviction is implemented.

High-water memory counters exist where practical.

Benchmarks report set-engine counters.

Focused benchmark gates fail when witness work regresses beyond thresholds.

Correctness remains mandatory before timing.

## 09. Cache Inventory

Query image cache.

Prepared plan cache.

Prepared normalized query cache.

Static empty fast cache.

Planner proof-like cache, if reintroduced through Free Join.

Sorted trie cache.

Lazy access cache.

Planner stats cache.

Any lazy GHT/COLT cache from PRD 13.

Any vectorized execution scratch cache from PRD 14 if retained across queries.

## 10. Required Cache Metrics

Current item count.

Current estimated bytes.

High-water item count.

High-water estimated bytes.

Cache hits.

Cache misses.

Cache inserts.

Cache evictions.

Evicted bytes.

Build time where applicable.

## 11. Budget Policy

Each cache must have one of three policies.

Policy one: explicit byte budget with eviction.

Policy two: explicit item budget with eviction and byte diagnostics.

Policy three: bounded lifecycle that guarantees release by transaction ID or object drop.

Unbounded global growth is not allowed.

If a cache cannot estimate bytes exactly, it must use a conservative estimate.

If a cache has no eviction, the bounded lifecycle must be documented and tested.

## 12. Eviction Plan

Prefer LRU or generation-based eviction keyed by schema and tx id.

Evict old snapshots before current snapshot entries when possible.

Never evict data currently borrowed by an active query image.

Use `Arc` ownership carefully.

Eviction must not invalidate active readers.

Eviction must not change query results.

Eviction must update diagnostics deterministically.

Tests must use small budgets to force eviction.

## 13. Set-Engine Observability Counters

Completed bindings.

Projected result facts considered.

Projected result facts inserted.

Projected duplicate result facts avoided.

Early projection attempts.

Early projection successes.

Aggregate domain events attempted.

Aggregate domain events applied.

Aggregate duplicate domain events avoided.

Existential semijoin probes.

Lazy GHT nodes forced.

Lazy GHT bytes copied.

Vector batches.

Vector probe keys.

Scalar fallbacks.

## 14. Benchmark JSON Requirements

Include all set-engine counters.

Include cache byte diagnostics.

Include cache eviction diagnostics.

Include optimizer selected cover information from PRD 15.

Include query image scoped field counts.

Include query image scoped access counts.

Include cardinality parity fields from PRD 04.

Include correctness mode.

Include aggregate domain description when applicable.

Do not remove existing fields without migration note in benchmark docs.

## 15. Focused Gate Requirements

Add a projection duplicate-witness gate.

Add an aggregate duplicate-domain gate.

Add a clover-like factoring gate.

Add a lazy GHT avoided-build gate.

Add a vectorized batch gate.

Each gate must validate exact result values first.

Each gate must assert counter thresholds.

Thresholds must be documented in the benchmark code or docs.

Thresholds must be deterministic for the fixture.

## 16. Required Cache Tests

Query image cache reports current bytes.

Query image cache reports high-water bytes.

Prepared plan cache reports current items and evictions if budgeted.

Planner proof-like cache reports items and evictions if reintroduced through Free Join.

Sorted trie cache reports bytes.

Lazy access cache reports bytes.

Small test budget forces eviction.

Evicted cache entry can be rebuilt correctly.

Active image is not invalidated by eviction.

## 17. Required Benchmark Tests

JSON renderer includes new fields.

Markdown renderer includes important gate counters.

Focused gates fail when a counter exceeds threshold in a test helper.

Focused gates pass on expected fixture.

Correctness mismatch still aborts timing.

Cardinality parity mismatch still aborts timing.

Aggregate SQL mismatch still aborts timing.

## 18. Required Documentation Updates

Update benchmark contract docs.

Update query observability docs if present.

Update `ROSETTA_STONE.md` validation section if global gates change.

Document cache budget defaults.

Document how to tune budgets if any knobs are public.

Document that correctness gates precede timing gates.

## 19. Stale-Term Gate

Run stale-term grep over source and normative docs.

The generated PRD suite may mention historical context only through approved wording.

No removed public model terms may reappear in code.

No removed scalar-count terminology may reappear.

No removed all-field access terminology may reappear.

If a legacy word appears in third-party paper quotes, do not put those quotes in normative docs.

## 20. Passing Criteria

Every major cache has budget or bounded lifecycle documentation.

Every major cache has item diagnostics.

Every major cache has byte diagnostics or documented reason it cannot.

Eviction tests pass for caches with budgets.

Benchmark JSON exposes set-engine counters.

Focused gates validate exact results and enforce counter thresholds.

Correctness failures prevent timing reports.

The global validation gate passes.

The query-focused validation gate passes.

Stale-term gate passes.

## 21. Failure Modes

Leaving a global cache unbounded is a failure.

Reporting hits and misses without memory diagnostics is a failure.

Evicting active reader data is a failure.

Benchmark timing after correctness failure is a failure.

Benchmark gates without exact value validation are a failure.

Counters that cannot distinguish result-set work from witness work are insufficient.

Adding nondeterministic thresholds is a failure.

## 22. Non-Goals

Do not add server metrics endpoints.

Do not add multi-threaded execution.

Do not add approximate memory accounting when exact accounting is easy.

Do not add external telemetry dependencies.

Do not change public query result APIs.

Do not implement new join algorithms.

## 23. Completion Notes

Record final benchmark gates in docs.

Record final cache budgets in docs.

Keep focused fixtures permanent.

This PRD completes the enforceable set-engine rebase process.
