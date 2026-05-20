# PRD 09: Static-Empty Zero-Row Fast Path

## Status

Proposed.

## Motivation

Static-empty queries should be the cheapest queries in the system. They prove that no result exists and then should return zero rows with almost no work on repeated execution.

Today they are not cheap enough. In the traced JOB run:

| Query | Runtime | Bumbledb avg | SQLite avg | Result |
|---|---|---:|---:|---|
| `job_q01_top_production` | `StaticEmpty` | 83 us | 9,064 us | Bumbledb wins massively |
| `job_q16_character_title_us` | `StaticEmpty` | 82 us | 11,280 us | Bumbledb wins massively |
| `job_q33_linked_series_companies` | `StaticEmpty` | 91 us | 65 us | SQLite wins |

q33 is the important signal. Bumbledb has already cached the static-empty proof, but it still pays the general query pipeline.

## Evidence

| Evidence | Anchor |
|---|---|
| `execute_query` validates, normalizes, encodes, gets image, computes key, then checks static-empty cache | `crates/bumbledb-lmdb/src/query.rs:1397-1459` |
| Static-empty cache hit returns empty rows only after plan/result metadata construction | `crates/bumbledb-lmdb/src/query.rs:1459-1478` |
| First miss runs static proof and inserts cache | `crates/bumbledb-lmdb/src/query.rs:1480-1514` |
| Count-only path duplicates static-empty pipeline | `crates/bumbledb-lmdb/src/query.rs:1610-1688` |
| Static-empty proof scans literal atoms and JOB-specific intersections | `crates/bumbledb-lmdb/src/query.rs:1783-2110` |
| Cache key is currently debug-string based | `crates/bumbledb-lmdb/src/query.rs:1771-1774` |
| Static plan allocates fresh metadata | `crates/bumbledb-lmdb/src/query.rs:2261-2292` |
| Result columns clone names every return | `crates/bumbledb-lmdb/src/query.rs:7469-7485` |
| q33 report shows sample named spans only 21.5% and residual 78.5% | `docs/job-trace-analysis/08-job_q33_linked_series_companies.md:50-68` |

## Goals

- Make cached static-empty no-input queries return before normalization when possible.
- Replace per-call zero-row plan/result metadata allocation with cached compact metadata.
- Keep proof correctness exact and snapshot-scoped.
- Make q33 faster than SQLite in practical JOB benchmark.
- Keep materialized and count-only behavior consistent.

## Non-Goals

- Do not implement negation or general contradiction reasoning.
- Do not persist static-empty proofs across writes unless invalidation is exact.
- Do not add backwards-compatible old static-empty cache paths.

## Current Static-Empty Flow

Current cached-hit flow:

```text
validate_inputs
normalize_query
encode_inputs
try_execute_direct_storage_project
query_images.get_or_build
query_images.diagnostics
prepared_plan_cache_key
image.static_empty_cached
static_empty_plan
result_columns
return QueryOutput { rows: Vec::new() }
```

Target cached-hit flow for no-input prepared static-empty query:

```text
compute or reuse QueryShapeKey
read snapshot tx id
static-empty result cache lookup
return cached zero-row output metadata
```

The initial implementation can still acquire `QueryImage` before lookup if simpler, but the end-state should allow an environment-level snapshot-scoped static-empty result cache.

## Required Data Structures

### Static Empty Result Cache

Introduce a cache keyed by:

```text
(schema_fingerprint, tx_id, QueryShapeKey)
```

The cache value should include:

```rust
struct StaticEmptyResultEntry {
    columns: Arc<[ResultColumn]>,
    plan_template: Arc<StaticEmptyPlanTemplate>,
}
```

The plan template should store immutable metadata that does not vary per execution:

- output columns,
- plan family,
- optimizer chosen = `static_empty`,
- empty free-join shape,
- maybe relation/field names if explain needs them.

Per-execution timing/allocation/counters still need to be filled into a fresh `QueryPlan` summary, but that fresh summary should be compact and avoid cloning large vectors.

### Query Shape Key

This PRD depends on PRD 08. Do not use `String` debug keys.

### Prepared No-Input Shape

This PRD can use `TypedQuery` directly at first, but should be designed to combine with PRD 10. For the earliest lookup before normalization, we need a key derivable without encoding literals if possible.

Two-stage approach:

1. Runtime `QueryShapeKey` after normalization using encoded literals, image-local static cache. Easier and still removes string key and metadata churn.
2. Prepared shape key before normalization, environment-level snapshot cache. This gets the full q33 win.

Because the user wants ambitious breaking changes and no tech debt, implement both if feasible in one PRD. If split, document the exact remaining late work and do it immediately in PRD 10.

## Implementation Plan

### Step 1: Compact Static Empty Return Helper

Create a helper:

```rust
fn finish_static_empty_output(
    query: &NormalizedQuery,
    timings: QueryTimings,
    allocations: QueryAllocationStats,
    counters: PlanCounters,
    diagnostics: QueryDiagnostics,
    total_start: Instant,
    total_alloc_start: AllocationSnapshot,
) -> QueryOutput;
```

Then replace duplicate static-empty return blocks at:

- `query.rs:1459-1478`
- `query.rs:1489-1514`
- count-only equivalents at `query.rs:1637-1688`

This first cleanup reduces duplication and makes the next fast path easy.

### Step 2: Cache Static Empty Result Metadata

After a proof succeeds, store:

- `QueryShapeKey`
- result columns
- static plan template

The current image-local `static_empty_queries` stores only a key set. Replace it with a result cache or point to the environment-level cache.

Current field:

```rust
static_empty_queries: Arc<RwLock<BTreeSet<String>>>
```

Target field if still image-local:

```rust
static_empty_results: Arc<RwLock<BTreeMap<QueryShapeKey, Arc<StaticEmptyResultEntry>>>>
```

### Step 3: Early No-Input Cache Check

Before `normalize_query`, check if the query is eligible for early static-empty cache:

- no runtime inputs,
- typed query has only deterministic literals and comparisons,
- query shape key can be derived structurally from `TypedQuery` and schema fingerprint,
- storage snapshot tx id is known.

Anchor for early insertion point: immediately after validation or even before validation in `execute_query` at `query.rs:1397`.

Because no-input queries cannot fail missing input validation, it is safe to skip `validate_inputs` on a cached hit if the typed query was already produced by the typechecker.

For input-bearing queries, stay on the normal path until input value keys are implemented.

### Step 4: Snapshot-Scoped Cache Location

Preferred ambitious design:

- Move static-empty result cache to `Environment` or `QueryImageCache`, keyed by `{schema, tx_id, query_shape}`.
- This lets cached hit avoid building/acquiring a full `QueryImage` just to find out it is empty.

Current `Environment` stores `query_images: QueryImageCache` at `crates/bumbledb-lmdb/src/lib.rs:86-91`.

Add a sibling cache or include static-empty in a broader `QueryRuntimeCaches` object.

Do not put environment-level cache values that depend on borrowed transaction data.

### Step 5: Rewrite Proof To Insert Into New Cache

On a proof miss:

- Run existing static proof.
- If empty, construct static-empty result entry.
- Insert into snapshot-scoped cache.
- Return using cached entry.

Proof code can remain as-is initially; PRD 06 prefix-count APIs can be used opportunistically.

### Step 6: Remove Old Static Empty Set

Delete old string-key set. Do not keep both.

## Count-Only Behavior

`execute_query_count_only` currently duplicates large parts of `execute_query`.

For static-empty:

- Count-only should hit the same static-empty result/proof cache.
- It can return `QueryCountOutput { rows: 0, plan }` without materialized columns.
- The plan should indicate `StaticEmpty` and cache hit/miss counters.

Do not let materialized and count-only static-empty paths diverge.

## Tests

### Unit Tests

- Static-empty miss inserts result cache.
- Static-empty second execution hits before proof.
- No-input cached static-empty hit does not call normalization. If direct instrumentation is hard, assert timing/counter behavior or add a test-only counter.
- Input-bearing static-empty queries do not use no-input fast path unless keyed by input value.
- Cache key includes tx id: after writing new data in a new transaction, old static-empty result is not reused.
- Materialized static-empty output has same columns and zero rows as before.
- Count-only static-empty output has row count zero and `StaticEmpty` plan.

### Existing Tests

Run:

```sh
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Plan

Run q01, q16, q33:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q01_top_production \
  --query job_q16_character_title_us \
  --query job_q33_linked_series_companies
```

Gates:

- q33 Bumbledb avg must beat SQLite avg. Current traced: 91 us vs 65 us.
- q01/q16 must not regress materially.
- Cached sample spans should show near-zero or absent normalize/encode/image work if early environment cache is implemented.
- Static-empty proof should still run exactly once on cold miss.

## Risks

- False static-empty cache hits are catastrophic. Cache key must include schema fingerprint and tx id.
- If a proof depends on dictionary IDs, the tx id scoping protects it. Do not reuse across snapshots unless dictionary/data versions are included.
- Explain diagnostics may be less detailed on ultra-fast cache hits. Provide truthful compact diagnostics rather than forcing slow cache-lock reads.
- Skipping validation is only safe for no-input typed queries. Runtime input validation must remain for input-bearing queries.

## Definition Of Done

- Cached no-input static-empty hits bypass normalization and proof.
- Old `BTreeSet<String>` static-empty cache is gone.
- Materialized and count-only static-empty paths share one cache/proof mechanism.
- q33 beats SQLite in practical JOB preset.
