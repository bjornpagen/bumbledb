# 01: Cache Planner Stats Per Query Image

**Goal**
- Stop rebuilding relation, field, and access-path planner statistics for every query.
- Make planner stats snapshot-scoped and reusable through `QueryImage`.

**Trace Evidence**
Setup residual after subtracting visible QueryImage build time is the largest observed bucket:

| Query | Setup | QueryImage | Approx Stats/Planning |
|---|---:|---:|---:|
| `ledger/postings_for_holder_range` | `26.1ms` | `8.1ms` | `18.0ms` |
| `ledger/tag_lookup_join` | `31.0ms` | `8.1ms` | `23.0ms` |
| `sailors/sailor_range_reserves` | `20.3ms` | `4.1ms` | `16.2ms` |
| `tpch/supplier_nation_orders` | `39.2ms` | `7.4ms` | `31.8ms` |

Across the 10 traced queries, approximate setup is `268.3ms`; image build accounts for `62.9ms`; planner/stat work accounts for roughly `205.4ms`.

**Current Code Path**
- `ReadTxn::execute_query` calls `plan_query` for every query.
- `plan_query` calls `PlannerStats::collect`.
- `PlannerStats::collect` calls `OptimizerRelationStats::build` for each relation in the query.
- `OptimizerRelationStats::build` scans every field and builds `SortedTrieIndex` instances for every access path just to obtain stats.
- `SortedTrieIndex::build` sorts row IDs and builds trie levels, then the temporary trie is discarded.

Relevant files:
- `crates/bumbledb-lmdb/src/query.rs`
- `crates/bumbledb-lmdb/src/query_image.rs`
- `crates/bumbledb-lmdb/src/sorted_trie.rs`

**Required Design**
- Move planner statistics into a snapshot-scoped cache owned by `QueryImage`.
- Key cached relation stats by `RelationId` inside the `QueryImageKey` boundary.
- `PlannerStats::collect` must become a cheap projection over cached relation stats.
- Initial cached stats may use the current expensive builders on first miss; repeated query execution on the same `QueryImage` must hit cache.

Proposed internal structure:

```rust
pub(crate) struct PlannerStatsCache {
    relations: RwLock<BTreeMap<RelationId, Arc<OptimizerRelationStats>>>,
}

pub(crate) struct PlannerStats {
    relations: BTreeMap<String, Arc<OptimizerRelationStats>>,
}
```

**Implementation Steps**
1. Move `PlannerStats`, `OptimizerRelationStats`, `OptimizerFieldStats`, and `OptimizerIndexStats` into a `planner_stats.rs` module.
2. Add a `PlannerStatsCache` to `QueryImage`.
3. Add `QueryImage::planner_relation_stats(schema, relation_id)` crate-private API.
4. Change `PlannerStats::collect` to fetch relation stats from the image cache.
5. Keep current stat semantics initially: row count, field distinct/min/max/heavy hitters, index distinct/fanout/max fanout.
6. Add diagnostics: cache hits, misses/builds, build micros, cached relation count.
7. Surface diagnostics in explain and benchmark markdown.

**Tests**
- Repeated same-query same-snapshot execution builds stats once and then hits cache.
- Two queries sharing `Posting` reuse `Posting` stats.
- A write commit creates a new `QueryImageKey` and fresh stats cache.
- Variable order and query output remain deterministic.
- Concurrent readers may share cached stats without panics or stale snapshot leaks.

**Acceptance Criteria**
- `PlannerStats::collect` does not call `SortedTrieIndex::build` directly.
- The second execution of the same query on the same image performs zero planner relation stats builds.
- Stats are never reused across different schema fingerprints or tx IDs.
- Benchmark markdown proves planner stats hits/misses.
- Focused scale-10000 hot-cache queries improve materially, especially tiny selective queries.

**Risks**
- Interior mutability inside `QueryImage` must remain logically immutable per snapshot.
- Cold concurrent readers may duplicate one build unless per-entry `OnceLock` is used.
- Stats caching can increase memory retention; lazy relation stats are preferred.
