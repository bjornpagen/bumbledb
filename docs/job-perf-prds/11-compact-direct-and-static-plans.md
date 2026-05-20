# PRD 11: Compact Direct And Static Plans

## Status

Proposed.

## Motivation

Direct count and static-empty queries do not need a full `ExecutionPlan` shaped like a generic free-join plan. Today they still pay for generic plan metadata, plan cloning, diagnostics, result columns, and explain scaffolding.

This PRD introduces compact physical plans for:

- Static-empty zero-row results.
- Direct count kernels.
- Direct storage project/range kernels where applicable.

The goal is to stop pretending these paths are free-join plans. The codebase is unstable; we should break the internal plan shape now and make it honest.

## Evidence

| Current behavior | Anchor |
|---|---|
| `ExecutionPlan` always carries `variable_order_ids`, `relation_atoms`, `comparisons`, optional direct kernel, and full `QueryPlan` summary | `crates/bumbledb-lmdb/src/query.rs:1147-1154` |
| Cache hit clones the full `ExecutionPlan` | `crates/bumbledb-lmdb/src/query.rs:1157-1177` |
| Static-empty plan creates full `QueryPlan` metadata and clones output | `crates/bumbledb-lmdb/src/query.rs:2261-2292` |
| Result columns clone variable names on return | `crates/bumbledb-lmdb/src/query.rs:7469-7485` |
| Direct kernels are represented as `direct_kernel: Option<DirectKernelPlan>` inside generic `ExecutionPlan` | `crates/bumbledb-lmdb/src/query.rs:1151-1153`, `5991-5998` |
| `job_movie_link_bridge` first run spends 18,247 plan allocation calls before direct execution | `docs/job-trace-analysis/07-job_movie_link_bridge.md:97-113` |
| q33 cached static-empty residual is mostly metadata/cache/result overhead | `docs/job-trace-analysis/08-job_q33_linked_series_companies.md:58-68` |

## Goals

- Introduce an explicit physical plan enum.
- Avoid building/cloning `FreeJoinPlan` for direct/static paths.
- Store compact direct/static plan templates in caches.
- Keep explain output truthful and minimal.
- Preserve existing counters/timings/allocations where meaningful.

## Non-Goals

- Do not optimize LFTJ traversal in this PRD.
- Do not implement query-image scoped loading here.
- Do not preserve old explain output shape for direct/static paths if it is misleading.

## Proposed Plan Shape

Replace the current one-size-fits-all `ExecutionPlan` with:

```rust
enum PhysicalPlan {
    StaticEmpty(StaticEmptyPlan),
    DirectCount(DirectCountPlan),
    DirectStorage(DirectStoragePlan),
    FreeJoin(FreeJoinExecutionPlan),
}
```

Where current `ExecutionPlan` becomes something like:

```rust
struct FreeJoinExecutionPlan {
    variable_order_ids: Box<[usize]>,
    relation_atoms: Box<[NormAtom]>,
    comparisons: Box<[NormPredicate]>,
    summary_template: FreeJoinPlanSummaryTemplate,
}
```

Do not keep direct/static as fields inside free-join execution.

## QueryPlan Output

`QueryPlan` is currently a public diagnostic struct. It can remain broad, but it should be constructed from compact templates at the boundary.

Recommended split:

```rust
struct QueryPlanTemplate {
    plan_family: PlanFamily,
    runtime_kind: QueryRuntimeKind,
    output: OutputPlan,
    result_columns: Arc<[ResultColumn]>,
    explain: PlanExplainTemplate,
}
```

At execution finish, create `QueryPlan` by adding:

- timings,
- allocations,
- counters,
- current cache diagnostics when requested,
- runtime-specific summaries.

This avoids cloning full plan diagnostics every cached execution.

## Static Empty Plan

Static-empty plan should store:

- `shape_key`,
- `result_columns`,
- `plan_family = StaticEmpty`,
- `runtime_kind = StaticEmpty`,
- optional proof counters from cold miss,
- no free-join nodes,
- no optimizer candidates except a compact single selected candidate if explain requires it.

Current `static_empty_plan` builds:

- `variable_order: Vec::new()`
- `variable_estimates: Vec::new()`
- `missing_indexes: Vec::new()`
- `optimizer: OptimizerTrace { chosen: "static_empty".to_owned(), candidates: vec![...] }`
- `free_join: FreeJoinPlan { nodes: Vec::new(), output: query.output.clone(), ... }`

Anchor: `query.rs:2261-2292`.

Most of this can be static or borrowed from prepared shape.

## Direct Count Plan

Direct count plan should store IDs and access IDs only:

- relation IDs,
- field IDs,
- index access IDs,
- aggregate output variable,
- direct kernel kind.

It should not store:

- variable order estimates,
- optimizer candidates,
- generic free-join nodes,
- missing index recommendations from generic planner.

If a direct count plan needs missing indexes, direct planner should produce direct-specific missing-index errors/recommendations.

## Direct Storage Plan

Current direct storage plan machinery exists around:

- direct storage project try path at `query.rs:1433-1444`,
- direct kernel planning around `query.rs:5991-5998`,
- direct execution helpers around `query.rs:2295-2607`, `3000+` depending on current code.

Fold direct storage into `PhysicalPlan::DirectStorage` where feasible.

## Implementation Plan

### Step 1: Add `PhysicalPlan` Enum

Introduce the enum internally. Initially only new direct/static paths need to use it. Generic path can wrap old `ExecutionPlan` as `PhysicalPlan::FreeJoin`.

### Step 2: Update Prepared Plan Cache Value

Prepared cache currently stores `ExecutionPlan` in `PreparedPlanCache`.

Anchor: `crates/bumbledb-lmdb/src/query_image.rs:189-200`, plus cache internals later in same file.

Change cache value to `PhysicalPlan` or `Arc<PhysicalPlanTemplate>`.

Do not clone full plans on hit. Store immutable plan templates and instantiate only per-execution counters/timings.

### Step 3: Static Empty Uses Compact Plan

Replace `static_empty_plan` with compact static template construction.

The return path should no longer call `result_columns(&normalized)` on every hit if prepared result columns exist.

### Step 4: Direct Count Uses Compact Plan

After PRD 07, direct count plan construction should return `PhysicalPlan::DirectCount` directly.

Execution should match on physical plan:

```rust
match plan {
    PhysicalPlan::StaticEmpty(plan) => execute_static_empty(...),
    PhysicalPlan::DirectCount(plan) => execute_direct_count(...),
    PhysicalPlan::DirectStorage(plan) => execute_direct_storage(...),
    PhysicalPlan::FreeJoin(plan) => execute_free_join(...),
}
```

### Step 5: Explain Output Cleanup

Update `QueryOutput::explain` and `QueryPlan::explain` around `query.rs:225-350` so compact plans render honestly.

For static-empty:

```text
plan_family: StaticEmpty
runtime_kind: StaticEmpty
static_empty: cache_hit=true proof_rows_scanned=0
```

For direct count:

```text
plan_family: Direct
runtime_kind: DirectKernel
direct_kernel: factorized_count steps=...
```

Do not fabricate a free-join variable order for direct/static paths.

## Acceptance Criteria

- Static-empty cached hits do not clone full `QueryPlan`/`FreeJoinPlan` structures.
- Direct count paths do not build or clone generic `ExecutionPlan`.
- Prepared cache stores compact immutable plan templates.
- Explain output remains useful and truthful.
- Tests are updated for new plan shape.

## Tests

### Unit Tests

- Static-empty plan template returns zero rows and has `PlanFamily::StaticEmpty`.
- Static-empty cache hit does not allocate free-join nodes.
- Direct count plan has no free-join nodes and correct direct summary.
- Prepared cache hit reuses same plan template without cloning full plan.
- Explain output for static/direct includes required fields.

### Integration Tests

Run:

```sh
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Gates

For q33 after PRD 09/11:

- Cached static-empty sample time should fall below SQLite.
- Allocations should no longer be dominated by static plan/result metadata.

For direct counts after PRD 07/11:

- First-run plan allocation calls should be near zero for direct count paths.
- `job_movie_link_bridge` first-run `plan_us` should disappear or be renamed direct plan construction and be far below 959 us.

## Risks

- Broad `QueryPlan` public diagnostic struct may still force allocations. If necessary, break it too and use an enum diagnostic shape.
- Tests may assume free-join fields are always present. Update tests; do not keep fake fields for compatibility.
- Direct/static paths still need accurate counters and gates. Do not drop counters just to reduce allocations.

## Definition Of Done

- Direct/static plans have their own compact internal plan shape.
- Generic free-join plan is no longer the container for all runtimes.
- Prepared cache stores compact templates.
- q33 and direct-count benchmarks show reduced plan/result metadata overhead.
