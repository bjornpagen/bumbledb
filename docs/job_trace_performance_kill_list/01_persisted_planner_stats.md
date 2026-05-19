# 01 Persist Or Cheaply Derive Planner Stats

Priority: P0

## Problem

Cold planning is still the largest remaining prepare cost for broad JOB queries. After the first kill-list round, repeated sample executions are mostly plan-cache hits, but the first execution still builds exact planner stats by scanning full relation images and constructing sorted tries for access paths.

The post-kill trace shows this most clearly:

| Query | Prepare | `plan.stats` | Prepare Share |
|---|---:|---:|---:|
| `job_broad_cast_keyword_company` | `278ms` | `219ms` | `78.8%` |
| `job_q09_voice_us_actor` | `247ms` | `168ms` | `68.0%` |
| `job_broad_movie_info_star` | `51ms` | `17.6ms` | `34.4%` |

This cost is paid before meaningful execution work starts and is still orders of magnitude above SQLite prepare for affected queries.

## Technical Cause

`PlannerStats::collect` requests relation stats for every relation touched by the query:

`crates/bumbledb-lmdb/src/query.rs:1080-1097`

```rust
for atom in atoms {
    if relations.contains_key(&atom.relation_name) {
        continue;
    }
    let relation = image
        .relations()
        .get(atom.relation.0 as usize)
        .ok_or_else(|| Error::unknown_relation(&atom.relation_name))?;
    relations.insert(
        atom.relation_name.clone(),
        image.planner_relation_stats(schema, relation)?,
    );
}
```

On cache miss, `OptimizerRelationStats::build` scans every field and builds a `SortedTrieIndex` for every access path:

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
    indexes.insert(...);
}
```

Field stats build exact frequency maps by cloning encoded values for every row:

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

Sorted trie stats sort row IDs by encoded field bytes:

`crates/bumbledb-lmdb/src/sorted_trie.rs:92-108`

```rust
let mut order = (0..relation.row_count)
    .map(|row| RowId(row as u32))
    .collect::<Vec<_>>();

order.sort_by(|left, right| {
    for field in &spec.fields {
        let left = relation.encoded_bytes(*left, *field).unwrap_or(&[]);
        let right = relation.encoded_bytes(*right, *field).unwrap_or(&[]);
        ...
    }
});
```

## Required Solution

Move planner stats away from full exact build-on-first-query. The planner should obtain stats from cheap query-image metadata or durable segment/index metadata, and exact stats should become opt-in or lazy per field/path.

### Phase 1: Demand-Driven Stats

Replace `OptimizerRelationStats::build(schema, relation)` with demand-driven stats access:

```rust
struct OptimizerRelationStats {
    rows: usize,
    fields: RwLock<BTreeMap<FieldId, Arc<OptimizerFieldStats>>>,
    indexes: RwLock<BTreeMap<AccessId, Arc<OptimizerIndexStats>>>,
}
```

Planner variable estimation should request only the field and index stats it actually needs. Do not build stats for unrelated fields or access paths.

### Phase 2: Use Segment Index Stats

`IndexSegmentDescriptor` already carries `IndexStatsSummary` for durable segment indexes. Build `OptimizerIndexStats` from segment descriptors where possible instead of rebuilding `SortedTrieIndex` only to get `distinct_by_depth`, `avg_fanout_by_depth`, and `max_fanout_by_depth`.

### Phase 3: Approximate Heavy Hitters

Exact heavy hitters require full frequency maps. Replace first-use exact heavy hitters with:

- min/max from encoded column metadata or one pass only for constrained fields,
- approximate top values only when the optimizer needs them,
- or no heavy hitters in cold planning, with a later background/exact cache.

## Implementation Plan

1. Add lazy field/index stat methods to `PlannerStatsCache` and `OptimizerRelationStats`.
2. Change `estimate_atom_variable_access` to request stats per path and variable field, not all paths/fields.
3. Teach `OptimizerIndexStats` to derive from durable/query-image segment stats.
4. Add diagnostics for `field_stats_built`, `index_stats_built`, `stats_from_segments`, `stats_exact_scans`.
5. Keep exact legacy build behind a fallback path for missing segment stats.

## Strict Passing Criteria

- `job_broad_cast_keyword_company` prepare drops from `~278ms` to `<50ms`.
- `job_q09_voice_us_actor` prepare drops from `~247ms` to `<60ms`.
- `plan.stats` for both broad/q09 drops by at least `90%` in traced runs.
- No query result changes across the full JOB suite.
- Planner diagnostics show no full relation stats build for fields/access paths not used by a query.
- Existing planner-stats cache tests still pass, updated to assert lazy stats behavior.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb planner_stats --all-targets
cargo test -p bumbledb-bench --all-targets
cargo check --workspace --all-targets --all-features
cargo run -p bumbledb-bench --release -- --dataset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --scale 10000 --warmup 2 --repeats 10 --format json
```

## Risks

- Approximate stats may alter plan choices and regress hot execution.
- Segment-level stats may be insufficient after multiple writes/segments unless merged carefully.
- Lazy stats must not introduce lock contention in repeated queries.
