# PRD 16: Planner Compact IDs And Lazy Candidates

## Status

Proposed.

## Motivation

After direct/static paths are moved out of generic planning, remaining free-join queries still pay avoidable planner allocation churn.

Current planner code is string-heavy and overbuilds candidate plans:

- `BTreeMap<String, ...>` for relation stats.
- Cloned relation/field/index names in access estimates.
- `BTreeSet` and `Vec` candidate churn for variable ordering.
- Full `FreeJoinPlan` built for every optimizer candidate.
- Chosen candidate cloned after sorting.
- Tie breaker strings built from `format!` and `join`.

This is not the first priority because image/LFTJ/direct/static were larger cliffs, but after PRDs 01-15 it becomes the right cleanup.

## Evidence

| Current behavior | Anchor |
|---|---|
| Planner stats wrapper maps relation names to stats | `crates/bumbledb-lmdb/src/query.rs:1180-1205` |
| Planner stats itself now caches by `RelationId`, but relation stats store field/index maps by `String` | `crates/bumbledb-lmdb/src/planner_stats.rs:133-174` |
| Field stats sample uses `BTreeMap<EncodedOwned, usize>` | `crates/bumbledb-lmdb/src/planner_stats.rs:191-231` |
| Variable order uses `BTreeSet`, candidate `Vec`, sort, and variable-name clone tie breaker | `crates/bumbledb-lmdb/src/query.rs:6038-6077` |
| Access estimates clone relation/index labels and produce reason strings | `crates/bumbledb-lmdb/src/query.rs:6255-6288` |
| Optimizer builds all candidate plans | `crates/bumbledb-lmdb/src/query.rs:6464-6562` |
| Candidate cost tie breaker formats implementation strings | `crates/bumbledb-lmdb/src/query.rs:6578-6634` |
| `build_free_join_plan` allocates nodes/subatoms/vars for every candidate | `crates/bumbledb-lmdb/src/query.rs:6649-6709` |
| `job_movie_link_bridge` plan allocation was 18,247 calls before PRD 07 | `docs/job-trace-analysis/07-job_movie_link_bridge.md:97-113` |

## Goals

- Replace planner hot structures with compact relation/field/access IDs.
- Estimate candidates first and build only the chosen `FreeJoinPlan`.
- Remove string tie-breaker allocations from hot planning.
- Keep explain output available by resolving IDs to names at render time.
- Reduce planner allocation calls and bytes for generic free-join queries.

## Non-Goals

- Do not change query semantics.
- Do not change the high-level optimizer search space unless a candidate is proven redundant by tests.
- Do not preserve old string-heavy planner internals.

## Proposed Planner Shape

### PlannerStats

Replace this wrapper:

```rust
struct PlannerStats {
    relations: BTreeMap<String, Arc<OptimizerRelationStats>>,
}
```

with:

```rust
struct PlannerStats {
    relations: BTreeMap<RelationId, Arc<OptimizerRelationStats>>,
}
```

or a dense `Vec<Option<Arc<...>>>` if relation IDs are dense and scoped images can handle missing entries explicitly.

### OptimizerRelationStats

Current:

```rust
pub(crate) struct OptimizerRelationStats {
    pub rows: usize,
    pub fields: BTreeMap<String, OptimizerFieldStats>,
    pub indexes: BTreeMap<String, OptimizerIndexStats>,
}
```

Target:

```rust
pub(crate) struct OptimizerRelationStats {
    pub relation: RelationId,
    pub rows: usize,
    pub fields: Box<[Option<OptimizerFieldStats>]>,
    pub indexes: BTreeMap<AccessId, OptimizerIndexStats>,
}
```

If `Box<[Option<_>]>` is awkward, use `Vec<Option<_>>`. The key point is field IDs, not field names.

### AccessEstimate

Current `AccessEstimate` stores relation/index labels as strings. Replace with IDs:

```rust
struct AccessEstimate {
    relation: RelationId,
    access: AccessId,
    estimated_rows: u64,
    prefix_len: usize,
    current_is_next: bool,
    distinct: usize,
    avg_fanout: u64,
    max_fanout: usize,
    variable_distinct: usize,
    has_min: bool,
    has_max: bool,
    heavy_hitters: usize,
}
```

Explain strings can be generated later from schema/layouts.

## Lazy Candidate Planning

Current optimizer builds full plan for each candidate:

```rust
candidates.push(build_plan_candidate("pure_lftj", ...)?);
candidates.push(build_plan_candidate("hash_probe", ...)?);
candidates.push(build_plan_candidate("hybrid", ...)?);
if has_aggregate(query) { candidates.push(build_plan_candidate("aggregate_pushdown", ...)?); }
candidates.sort_by_key(...);
let plan = candidates.iter().find(...).plan.clone();
```

Target:

```rust
let candidate_specs = estimate_candidate_specs(...);
let chosen_spec = candidate_specs.iter().min_by_key(|spec| spec.cost).unwrap();
let plan = build_free_join_plan(..., chosen_spec.implementations, chosen_spec.estimates)?;
let trace = optimizer_trace_from_specs(candidate_specs, chosen_spec.name);
```

Candidate trace should store names, implementation list, family, and cost. It does not need full plan nodes for rejected candidates.

## Tie Breaker Without Strings

Replace:

```rust
tie_breaker: format!("{}:{}", name, implementations.iter().map(|implementation| format!("{implementation:?}")).collect::<Vec<_>>().join(","))
```

with a compact deterministic tuple:

```rust
struct CostKey {
    estimated_micros: u64,
    setup_micros: u64,
    memory_bytes: usize,
    materialization_penalty: u64,
    candidate_rank: u8,
    implementation_mask: u64,
}
```

This changes public `CostKey`, which is fine. Explain rendering can format candidate name separately.

## Variable Ordering Without BTreeSet Sort Churn

Current `choose_variable_order` repeatedly:

- stores remaining in `BTreeSet`,
- builds a candidate `Vec`,
- sorts it,
- clones variable name for tie break.

Replace with:

- `Vec<bool>` or bitset for remaining/bound,
- single pass to find best variable by comparing cost tuple,
- tie-break by variable ID or precomputed stable name ordinal, not cloned `String`.

Preserve deterministic output. If old variable-name tie break matters, precompute name order once as integer ordinals.

## Implementation Plan

### Step 1: Compact CostKey

Change `CostKey` and all tests/renderers. Remove `tie_breaker: String`.

### Step 2: Lazy Candidate Specs

Introduce:

```rust
struct CandidateSpec {
    name: CandidateName,
    family: PlanFamily,
    implementations: Box<[NodeImpl]>,
    estimates: PlanEstimates,
    cost: CostKey,
}
```

Use enum for candidate names:

```rust
enum CandidateName { PureLftj, HashProbe, Hybrid, AggregatePushdown }
```

Render to string only in explain/JSON.

### Step 3: Build Only Chosen Plan

Change `optimize_free_join_plan` to estimate all candidates but call `build_free_join_plan` once.

### Step 4: Compact PlannerStats Wrapper

Change `PlannerStats` in `query.rs` from string-keyed map to relation ID keyed map.

### Step 5: Compact Relation Stats

Change `OptimizerRelationStats` field/index lookup to IDs/access IDs. Update all callers:

- `stats.relation_rows`
- `stats.field_stats`
- `stats.index_stats`
- `estimate_atom_variable_access`

### Step 6: Variable Order Rewrite

Replace BTreeSet/candidate-sort loop with single-pass best selection.

## Acceptance Criteria

- Optimizer builds one `FreeJoinPlan`, not one per candidate.
- `CostKey` does not allocate strings.
- Variable ordering does not allocate/sort a candidate vector each depth.
- Planner stats use relation/field/access IDs internally.
- Explain output remains stable enough for humans and tests.

## Tests

### Unit Tests

- Existing optimizer choice tests still choose the same candidate.
- Candidate trace contains all expected candidates.
- Chosen plan matches old plan for representative queries.
- Variable order matches old order for tie-heavy queries or changes only where documented.
- Planner stats lookup by ID returns same estimates as old string lookup.

### Integration Tests

Run:

```sh
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Gates

Run generic queries after direct/static paths are out of planner:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q09_voice_us_actor \
  --query job_q24_voice_keyword_actor
```

Expected:

- First-run `plan` allocation calls drop materially.
- Candidate trace still present.
- q09/q24 runtime unchanged or improved.

Run non-JOB too because planner affects synthetic workloads:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob
```

## Risks

- Variable-order tie breaks may change. That can change runtime and benchmark numbers. If changed, verify correctness and update expectations.
- ID-based stats must handle scoped images with missing relations. Return explicit error if missing required stats.
- Explain rendering may need names; keep schema/image references available at render construction time.

## Definition Of Done

- Planner internals are ID-based and string-light.
- Optimizer builds only the chosen plan.
- Cost keys do not contain heap strings.
- Generic planner allocation drops without correctness regressions.
