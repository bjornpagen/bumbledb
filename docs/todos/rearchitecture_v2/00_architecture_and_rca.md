# 00: Architecture And RCA

**Goal**
- Define the new architecture and record the root-cause analysis of the current implementation without preserving backwards-compatible internals.

**Problem Statement**
- The current engine is often one order of magnitude slower than SQLite on generated benchmarks.
- The current WCOJ implementation is logically variable-oriented but physically still scan/materialization-oriented.
- The database needs a real query runtime, not direct execution over durable LMDB indexes.

**Current Architecture Facts**
- LMDB stores current rows, covering index keys, dictionary entries, metadata counters, and history.
- Query execution uses encoded index scans opened through `ReadTxn::scan_encoded_index_prefix`.
- Query execution collects candidate variable values into `BTreeSet<EncodedValue>` and intersects sets.
- Query planning chooses variable order and access paths but does not own a Free Join plan IR.
- Explicit indexes exist, but query execution still repeats prefix scans and materializes candidate sets.

**Root Causes To Eliminate**
- Repeated LMDB prefix iterator construction in variable recursion.
- Materialized candidate domains via `BTreeSet`.
- Lack of reusable trie cursor state across variable depths.
- Lack of `next_distinct(depth)` and `seek(depth, target)` over trie levels.
- Lack of prefix/section cardinality and fanout stats.
- Physical query representation based on covering LMDB keys instead of query-optimized relation images.
- One-size variable-at-a-time executor without a plan IR capable of binary/probe/hybrid nodes.
- Decoding and materialization counters exist, but execution still materializes too many output-side values.

**New Architecture**
```text
LMDB durable snapshot
  -> QueryImage cache keyed by (schema_fingerprint, tx_id)
    -> RelationImage per relation
      -> ColumnImage per field
      -> SortedTrieIndex per sorted physical access order
      -> HashTrieIndex per probe-heavy physical access order
      -> RelationStats and prefix/fanout stats
    -> FreeJoinPlan
      -> SortedLeapfrog nodes
      -> HashProbe nodes
      -> Hybrid nodes
      -> AggregateSink nodes
```

**Module Target**
```text
crates/bumbledb-lmdb/src/
  durable.rs
  dictionary.rs
  query_image.rs
  relation_image.rs
  column_image.rs
  sorted_trie.rs
  hash_trie.rs
  trie_iter.rs
  leapfrog.rs
  free_join_plan.rs
  free_join_executor.rs
  aggregate_sink.rs
  optimizer.rs
  explain.rs
```

**Target Rust Facade**
```rust
pub struct QueryRuntime {
    cache: QueryImageCache,
}

impl QueryRuntime {
    pub fn execute(
        &self,
        txn: &ReadTxn<'_>,
        schema: &StorageSchema,
        query: &TypedQuery,
        inputs: &InputBindings,
    ) -> Result<QueryOutput> {
        let image = self.cache.get_or_build(txn, schema)?;
        let normalized = NormalizedQuery::from_typed(schema, query, inputs)?;
        let plan = Optimizer::new(&image).plan(&normalized)?;
        FreeJoinExecutor::new(&image, &plan).execute(&normalized)
    }
}
```

**Design Principles**
- LMDB is the durable source of truth, not the hot join iterator structure.
- QueryImage is immutable per snapshot and can be shared by concurrent read transactions.
- Execution operates on encoded columns, row ids, sorted trie indexes, hash tries, and row ranges.
- Free Join is the single physical plan language.
- LFTJ and binary-like joins are node implementations in one executor, not separate engines.
- Aggregation is a sink inside the plan when possible, not always a post-processing step.

**Passing Criteria**
- This PRD suite exists and is linked from `docs/todos/README.md`.
- The implementation roadmap has no old-executor fallback milestone.
- Every later PRD names code modules, public/internal structs, invariants, and passing criteria.
- The current benchmark gap is framed as a physical-runtime problem, not as a missing index-only problem.

**Non-Goals**
- Do not preserve current internal query executor APIs.
- Do not maintain LMDB prefix scans as the long-term query hot path.
- Do not preserve covering LMDB index keys as the primary execution representation.
- Do not add migration compatibility code for old experimental storage/query internals.
