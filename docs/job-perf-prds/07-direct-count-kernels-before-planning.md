# PRD 07: Direct Count Kernels Before Generic Planning

## Status

Proposed.

## Motivation

Several JOB queries ultimately run direct count kernels, but only after paying generic free-join planning costs.

Examples from the traced run:

| Query | Final runtime | First-run planning waste |
|---|---|---:|
| `job_broad_movie_info_star` | `DirectKernel` | 2.288 ms plan, 17,350 plan alloc calls, 1.596 MB plan allocation |
| `job_movie_link_bridge` | `DirectKernel` | 959 us plan, 18,247 plan alloc calls, 1.039 MB plan allocation |

Current execution builds/loads image, checks static empty, plans a generic free-join candidate set, then enters `execute_free_join`, where direct count kernels are tried first.

That is backwards. Direct count eligibility can be recognized before generic planning. This PRD moves direct count recognition into an early planning stage and returns compact direct plans.

## Evidence

| Evidence | Anchor |
|---|---|
| Generic planning starts before direct count dispatch | `crates/bumbledb-lmdb/src/query.rs:1516-1554` |
| Direct factorized count is tried inside execution dispatch after planning | `crates/bumbledb-lmdb/src/query.rs:2609-2625`, `2669-2784` |
| Movie link direct count is inside factorized count path | `crates/bumbledb-lmdb/src/query.rs:2786-2886` |
| `plan_query` only sets ordinary direct kernels after optimizer work | `crates/bumbledb-lmdb/src/query.rs:5887-6002` |
| Optimizer builds multiple full candidates before choosing | `crates/bumbledb-lmdb/src/query.rs:6464-6562` |
| `job_movie_link_bridge` report shows generic planning is 42.7% of first execution | `docs/job-trace-analysis/07-job_movie_link_bridge.md:44-58` |
| `job_broad_movie_info_star` report shows direct dispatch is 98.53% of sample busy and planning is first-run artifact | `docs/job-trace-analysis/02-job_broad_movie_info_star.md:85-120` |

## Goals

- Detect supported direct count query shapes before `plan_query`.
- Build compact direct count execution plans without generic `FreeJoinPlan` optimizer candidates.
- Reuse `RelationIndexImage::prefix_count` from PRD 06.
- Preserve existing result rows and counters.
- Keep explain output truthful but compact.
- Delete late direct-count detection from generic execution once the early path is complete.

## Non-Goals

- Do not optimize all direct storage project/range paths in this PRD.
- Do not implement query-image scoping yet.
- Do not implement prefix-count cache yet.
- Do not change aggregate semantics.

## Current Direct Count Shapes

### Factorized Count

Current function: `try_execute_factorized_count` at `query.rs:2669-2784`.

Eligibility:

- No inputs.
- No predicates.
- No atom fields with input/literal terms.
- Output is global `count` with no group vars.
- Central aggregate variable appears in at least two fact indexes.
- Non-central variables have limited degree.
- Each relevant relation has an index leading with the central field.

Current allocation/CPU problems:

- `fact_indexes` vector built during execution.
- Distinct central values stored as `BTreeSet<Vec<u8>>` at `query.rs:2737-2741`.
- Prefix counts done via `.entries_with_prefix(...).count()` at `query.rs:2748-2749`.

### Movie Link Bridge Count

Current function: `try_execute_movie_link_bridge_count` at `query.rs:2786-2886`.

Eligibility:

- Global `count` with no groups.
- Query contains `MovieLink`.
- Query contains at least two `MovieCompanies` atoms.
- Query contains at least two `MovieInfoIdx` atoms.

Current CPU problem:

- Loops every `MovieLink` row.
- For each row, performs four prefix iterator counts at `query.rs:2851-2854`.

## Proposed Architecture

Introduce direct count planning before generic `plan_query`:

```rust
enum DirectCountPlan {
    Factorized(FactorizedCountPlan),
    MovieLinkBridge(MovieLinkBridgeCountPlan),
}

struct FactorizedCountPlan {
    central: usize,
    driver: DirectCountIndexRef,
    fact_indexes: Vec<DirectCountIndexRef>,
}

struct MovieLinkBridgeCountPlan {
    movie_link: RelationId,
    movie_field: FieldId,
    linked_movie_field: FieldId,
    movie_companies_by_movie: DirectCountIndexRef,
    movie_info_idx_by_movie: DirectCountIndexRef,
}

struct DirectCountIndexRef {
    relation: RelationId,
    access: AccessId,
    leading_field: FieldId,
}
```

The plan should store IDs, not relation/index names. Explain output can recover names lazily from schema/image.

## Execution Flow

Current flow:

```text
validate -> normalize -> encode -> direct storage check -> image -> static-empty -> prepared plan -> generic plan_query -> execute_free_join -> try direct count
```

Target flow:

```text
validate -> prepared/normalized query -> encode -> direct storage check -> image -> static-empty -> early direct count plan -> execute direct count -> otherwise generic plan_query
```

The earliest safe point for direct count execution is after query image acquisition because it needs relation index images. PRD 12 later makes this image scoped.

## Implementation Plan

### Step 1: Extract Eligibility Into Planner Functions

Split current execution functions into pure plan builders and executors.

Suggested functions:

```rust
fn plan_factorized_count(image: &QueryImage, query: &NormalizedQuery) -> Result<Option<FactorizedCountPlan>>;
fn plan_movie_link_bridge_count(image: &QueryImage, query: &NormalizedQuery) -> Result<Option<MovieLinkBridgeCountPlan>>;
fn plan_direct_count(image: &QueryImage, query: &NormalizedQuery) -> Result<Option<DirectCountPlan>>;
```

These functions must not mutate `ExecutionPlan` or `PlanCounters`.

### Step 2: Add Compact Execution Path

Add:

```rust
fn execute_direct_count_plan<S: TupleSink>(
    image: &QueryImage,
    txn: &ReadTxn<'_>,
    query: &NormalizedQuery,
    plan: &DirectCountPlan,
    sink: &mut S,
    counters: &mut PlanCounters,
) -> Result<DirectKernelSummary>;
```

Use `RelationIndexImage::prefix_count` from PRD 06.

### Step 3: Build Compact `QueryPlan`

Add a helper to create a minimal plan summary:

```rust
fn direct_count_query_plan(
    query: &NormalizedQuery,
    direct: &DirectCountPlan,
    diagnostics: Diagnostics,
) -> QueryPlan;
```

Set:

- `plan_family = PlanFamily::Direct` or a new `PlanFamily::DirectCount` if useful.
- `runtime_kind = QueryRuntimeKind::DirectKernel`.
- `direct_kernel = Some(DirectKernelSummary { ... })`.
- `free_join.nodes = Vec::new()` or a compact direct node if explain needs one.
- `optimizer.chosen = "direct_count"`.

Do not construct variable estimates, missing index recommendations, full free-join nodes, or candidate traces unless explicitly needed for explain. If tests currently require these fields to exist, update tests to accept compact direct plan shape.

### Step 4: Insert Before `plan_query`

In `execute_query`, after static-empty proof/cache and before `plan_query`:

```rust
if let Some(direct_count) = plan_direct_count(image.as_ref(), &normalized)? {
    return execute_direct_count_query(...);
}
```

`execute_direct_count_query` should:

- create the right sink or specialized count sink from PRD 15 if available,
- time execution under `execute_micros`,
- set allocation phase `execute`,
- finish sink,
- set counters,
- return `QueryOutput`.

If PRD 15 is not implemented yet, use existing `OutputSink` and keep sink overhead.

### Step 5: Remove Late Direct Count From `execute_free_join`

Once early direct count is correct, remove these late calls from generic dispatch:

- `try_execute_factorized_count` from `execute_free_join` at `query.rs:2623-2625`.
- `try_execute_movie_link_bridge_count` nested under factorized count.

If removing immediately is too risky during migration, keep an assertion or fallback temporarily. Long-term there should be one direct count path, not two.

## Direct Factorized Count Heap Fix

While moving direct count, also replace `BTreeSet<Vec<u8>>` central values.

Current:

```rust
let mut central_values = BTreeSet::<Vec<u8>>::new();
for entry in driver.bytes.chunks(driver.encoded_len) {
    if let Some(bytes) = driver.component_bytes(entry, driver_field) {
        central_values.insert(bytes.to_vec());
    }
}
for central_value in central_values { ... }
```

Target:

- Because the chosen driver index is sorted by full entry key and central field is a leading field, iterate entries in order.
- Track previous central value as `EncodedOwned` or fixed stack bytes.
- Skip duplicates without inserting into a tree.

Suggested helper:

```rust
fn for_each_distinct_component(
    index: &RelationIndexImage,
    field: FieldId,
    mut visit: impl FnMut(&[u8]) -> Result<()>,
) -> Result<()>;
```

This helper must not allocate per entry. It may keep one `EncodedOwned` previous value.

## Counters

Preserve or clarify:

- `direct_kernel_probes`: increment for each logical prefix count.
- `direct_kernel_rows`: total factorized counted bindings.
- `factorized_counted_bindings`: total counted bindings.
- `materialized_output_values`: final output values.

If plan family changes from `FreeJoinLftj` to `Direct`, update benchmark expected outputs and docs.

## Acceptance Criteria

- `job_broad_movie_info_star` no longer has generic `bumbledb.query.plan` span in first execution.
- `job_movie_link_bridge` no longer has generic `bumbledb.query.plan` span in first execution.
- Plan allocation calls for those two queries drop by at least 90%.
- Direct factorized count no longer allocates `BTreeSet<Vec<u8>>` central values.
- Results and counters remain correct.
- Generic free-join planning still handles non-direct queries.

## Tests

### Unit Tests

- `plan_factorized_count_accepts_broad_movie_info_star_shape` using a small synthetic schema/query if JOB schema helper is cumbersome.
- `plan_factorized_count_rejects_inputs_literals_predicates`.
- `plan_factorized_count_rejects_grouped_aggregate`.
- `plan_movie_link_bridge_count_accepts_bridge_shape`.
- `for_each_distinct_component_skips_duplicate_leading_values`.
- `execute_direct_count_plan_matches_old_factorized_count` on a small fixture.

### Integration Tests

Run:

```sh
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Plan

Run:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_broad_movie_info_star \
  --query job_movie_link_bridge
```

Gates:

- `job_broad_movie_info_star.allocations.phases.plan.alloc_calls` should collapse from 17,350 to near zero for direct path.
- `job_movie_link_bridge.allocations.phases.plan.alloc_calls` should collapse from 18,247 to near zero for direct path.
- `job_broad_movie_info_star.allocations.phases.execute.alloc_calls` should drop substantially from 35,728 after streaming distinct central values.
- `job_movie_link_bridge.bumbledb.avg_us` should improve or not regress.
- Both queries remain faster than SQLite.

## Risks

- Direct count eligibility must be exact. A false positive is a correctness bug.
- Some direct count shape checks currently rely on relation names like `MovieLink`. That is acceptable for current JOB-specific kernels, but keep the generic factorized path ID-based where possible.
- Explain output will change. Update tests and docs rather than preserving generic plan fiction.
- If direct path bypasses planner stats, diagnostics may show fewer planner stats. That is correct.

## Definition Of Done

- Direct count plans are built before generic free-join planning.
- Late direct count dispatch is removed or marked as temporary fallback with follow-up deletion.
- Direct factorized count central values stream without `BTreeSet<Vec<u8>>`.
- Prefix counting uses PRD 06 API.
- JOB direct-count benchmarks show plan allocation collapse and unchanged results.
