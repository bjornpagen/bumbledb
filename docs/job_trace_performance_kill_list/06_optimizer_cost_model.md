# 06 Optimizer Build-Cost Model And Statistics Fixes

Priority: P1

Primary affected queries:

- `job_q16_character_title_us`: cost model did not anticipate `501,066` cold hash build rows.
- `job_q24_voice_keyword_actor`: cost model did not anticipate `317,617` cold hash build rows.
- `job_q33_linked_series_companies`: cost model did not anticipate `220,043` cold hash build rows.
- `job_broad_cast_keyword_company`: optimizer selects a plan named `hash_probe`, but runtime falls back to LFTJ.
- `job_broad_movie_info_star`: late fanout underestimated, causing `47,034` bindings.

## Problem

The optimizer's cost model prices logical candidate work, but it misses physical build costs that dominate traced runtime. It estimates hash build cost from candidate rows, not from relation rows or index materialization rows. It also treats an `aggregate_pushdown` candidate as cheap even though the runtime still enumerates complete bindings.

The result is a misleading trace and bad cold-plan choices:

- HashProbe plans look cheap but build hundreds of thousands of rows.
- Mixed hash plans look cheap but run as LFTJ fallback.
- Aggregate pushdown looks like a special plan but uses ordinary LFTJ leaf enumeration.

## Trace Evidence

| Query | Chosen Plan | Estimated Problem | Actual Cold Cost |
|---|---|---|---:|
| `job_q16_character_title_us` | `hash_probe` | tiny candidate counts | `501,066` hash build rows, `96.6ms` |
| `job_q24_voice_keyword_actor` | `hash_probe` | dies after `2` probes | `317,617` hash build rows, `47.8ms` |
| `job_q33_linked_series_companies` | `hash_probe` | dies after `5` probes | `220,043` hash build rows, `26.1ms` |
| `job_broad_cast_keyword_company` | `hash_probe` | mixed vector priced as hash | runtime `MixedFallback`, hash counters zero |
| `job_broad_movie_info_star` | `aggregate_pushdown` | pushdown priced cheap | `47,034` bindings enumerated |

## Current Technical Cause

Hash build cost is estimated from variable candidates:

`crates/bumbledb-lmdb/src/query.rs:3773-3830`

```rust
for (cost, implementation) in variable_costs.iter().zip(implementations) {
    let mut variable_ops = cost.estimated_candidates.max(1);
    match implementation {
        NodeImpl::SortedLeapfrog => {
            variable_ops = variable_ops.saturating_mul(if cyclic { 1 } else { 3 });
        }
        NodeImpl::HashProbe => {
            hash_probe_rows = hash_probe_rows.saturating_add(cost.estimated_candidates.max(1));
            hash_build_rows = hash_build_rows.saturating_add(cost.estimated_candidates.max(1));
        }
        ...
    }
    iterator_ops = iterator_ops.saturating_add(variable_ops);
}
```

Candidate cost then divides `hash_build_rows` by `64`:

`crates/bumbledb-lmdb/src/query.rs:3675-3681`

```rust
let cost = CostKey {
    estimated_micros: estimates
        .iterator_ops
        .saturating_add(estimates.hash_probe_rows)
        .saturating_add(estimates.hash_build_rows / 64)
        .saturating_add(estimates.materialized_values),
```

Actual build rows are counted elsewhere from full relation cardinality:

`crates/bumbledb-lmdb/src/query.rs:1469-1476`

```rust
if !cached.hit {
    plan.summary.counters.hash_index_builds += 1;
    plan.summary.counters.hash_index_build_rows = plan
        .summary
        .counters
        .hash_index_build_rows
        .saturating_add(relation.row_count as u64);
}
```

The optimizer knows relation row counts through stats, but it does not use them for hash build cost.

Planner stats are exact and expensive on first relation use:

`crates/bumbledb-lmdb/src/planner_stats.rs:119-159`

```rust
for field in &relation.fields {
    fields.insert(
        field.name.clone(),
        OptimizerFieldStats::build(relation, field.id)?,
    );
}

for path in schema.access_paths(&relation.name)? {
    ...
    let trie = SortedTrieIndex::build(
        relation,
        IndexSpec::new(format!("{}_stats", path.index_name), field_ids),
    )?;
```

Field stats scan each row and allocate frequency maps:

`crates/bumbledb-lmdb/src/planner_stats.rs:183-191`

```rust
let mut frequencies = BTreeMap::<EncodedOwned, usize>::new();
for row in 0..relation.row_count {
    let value = relation
        .encoded(RowId(row as u32), field)
        .map(EncodedOwned::from_ref)
        .ok_or_else(|| Error::internal("missing optimizer field value"))?;
    *frequencies.entry(value).or_insert(0) += 1;
}
```

## Desired End State

The optimizer should price physical work close enough that it does not select plans that are only cheap on paper.

Cost estimates should include:

- Whether required hash/sorted indexes are already cached.
- Cold build rows for each distinct hash/sorted index request.
- Whether a chosen mixed plan is actually executable without fallback.
- Whether aggregate pushdown truly reduces binding enumeration.
- Relation row count and prefix selectivity for index materialization.

## Proposed Technical Solution

### Distinct Physical Index Request Modeling

During candidate construction, compute the set of physical index requests the candidate would need:

```rust
struct PhysicalIndexRequest {
    kind: IndexRuntimeKind,
    relation: RelationId,
    access: AccessId,
    fields: Vec<FieldId>,
    leaf_mode: Option<LeafMode>,
    expected_rows_to_build: u64,
    cache_key: String,
    cache_hit: bool,
}
```

Candidate estimates should charge each distinct request once, not once per variable.

### Hash Build Cost

For current eager HashProbe, `expected_rows_to_build = relation.row_count` for each distinct hash request that is not cached.

After lazy HashProbe lands, cost should be branch-sensitive:

- Required before first probe: only first node driver/probe indexes.
- Expected downstream indexes: probability-weighted by estimated survival.

Start simple:

```text
hash_build_cost = sum_uncached_hash_request_rows / HASH_BUILD_ROWS_PER_MICRO
```

Use observed trace to calibrate initial constants:

- `job_q16`: `501,066` rows in `96.6ms`, about `5,187 rows/ms`.
- `job_q24`: `317,617` rows in `47.8ms`, about `6,645 rows/ms`.
- `job_q33`: `220,043` rows in `26.1ms`, about `8,431 rows/ms`.

A conservative constant around `5,000 rows/ms` is safer than current `hash_build_rows / 64` based on candidate rows.

### LFTJ Build Cost

For LFTJ candidates, estimate cold temp trie build work:

```text
lftj_build_rows = sum(source_relation_rows for each uncached atom trie request)
lftj_sort_rows = sum(retained_rows * log2(retained_rows))
```

Use cached status from `QueryImage.sorted_trie_cache` if available. Add a cache probe API that checks keys without building.

### Runtime Executability Cost

If a candidate contains mixed `HashProbe` and `SortedLeapfrog` nodes and there is no mixed executor, it must not be costed as hash-probe. Options:

- Reject candidate.
- Rename candidate `mixed_fallback` and cost it as LFTJ.
- After mixed executor lands, cost it as mixed.

### Aggregate Pushdown Cost

Only discount aggregate enumeration if the selected plan has a real pushdown executor.

Until [`05_count_aggregate_pushdown.md`](05_count_aggregate_pushdown.md) lands, `aggregate_pushdown` should be either removed or costed identically to pure LFTJ.

### Planner Stats Cost

Exact planner stats construction should be visible in plan cost. Add cheap relation metadata to `QueryImage` during image build:

- Row count.
- Field min/max when already available from column chunks.
- Approximate distinct where durable segment stats exist.
- Access path row counts and prefix distinct from durable index segment descriptors.

Defer exact heavy stats until needed.

## Implementation Plan

1. Add `PhysicalIndexRequest` extraction for each candidate.
2. Add cache-hit check APIs for `QueryImage` sorted/hash trie caches.
3. Change `PlanEstimates` to include `hash_build_rows_cold`, `sorted_build_rows_cold`, and `build_micros_estimate`.
4. Charge hash build by relation rows, not candidate rows.
5. Reject or relabel mixed hash candidates until mixed executor exists.
6. Remove aggregate discount until real aggregate pushdown exists.
7. Add trace diagnostics showing estimated vs actual build rows.

## Tests

- A hash plan requiring a `200,000` row build must be costed above a direct/sorted plan that avoids the build when runtime work is tiny.
- Mixed plan without mixed executor is not selected as `hash_probe`.
- `aggregate_pushdown` does not claim lower cost unless pushdown is executable.
- Estimated hash build rows match `hash_index_build_rows` within a known bound for cold runs.

## Acceptance Criteria

- `job_q33` no longer chooses a plan whose cold cost is dominated by a `200,000` row hash build unless it is still faster by measured trace.
- Plan trace includes estimated physical build rows and actual build rows.
- Mixed fallback is no longer selected under a `hash_probe` label.
- Cost estimates become directionally predictive for cold vs warm runs.

## Risks

- Overpricing cold builds could choose worse steady-state plans when caches are warm.
- We may need separate cold and warm cost modes.
- Cache-hit checks must not mutate or build.

## Rollout Plan

1. Add diagnostics-only physical request extraction.
2. Compare estimated requests with trace counters.
3. Change hash build cost.
4. Add cold/warm cost modes.
5. Re-run JOB and calibrate constants.
