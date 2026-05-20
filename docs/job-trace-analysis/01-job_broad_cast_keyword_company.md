# RCA: job_broad_cast_keyword_company

**Verdict**

- The measured benchmark result is good: Bumbledb averages 16,090 us versus SQLite at 112,828 us, a 7.012x speedup.
- The large 923 ms Bumbledb prepare is mostly real first-query product waste: the query path builds and retains a whole-database query image before a direct count kernel can run.
- The allocation telemetry is a first Bumbledb execution artifact, but the underlying cache-fill cost is product-visible on cold snapshots: 32.8M allocation calls, 2.597 GB allocated, and 986 MB peak live.
- The measured 30 samples do not include the 899 ms image build. Sample-time waste is mostly inside the direct factorized count kernel, not query-image acquisition.

**Benchmark Numbers**

| Metric | Value |
|---|---:|
| Rows | 1 |
| Runtime | DirectKernel |
| Chosen plan | aggregate_pushdown |
| Plan family | FreeJoinLftj |
| Bumbledb samples | 30 |
| Bumbledb avg | 16,090 us |
| Bumbledb p50 | 16,020 us |
| Bumbledb p95 | 16,980 us |
| Bumbledb min | 15,559 us |
| Bumbledb max | 17,105 us |
| SQLite avg | 112,828 us |
| SQLite p50 | 112,062 us |
| SQLite p95 | 117,435 us |
| SQLite min | 110,093 us |
| SQLite max | 118,235 us |
| SQLite / Bumbledb | 7.012x |
| Bumbledb prepare | 923,197 us |
| SQLite prepare | 110,566 us |
| Bumbledb warmup avg | 17,116 us across 2 runs |
| SQLite warmup avg | 115,214 us across 2 runs |
| Query image built during query | true |

**Timing Breakdown**

| `phase_timing` phase | Time (us) | % of 923,100 us first execution |
|---|---:|---:|
| image | 899,106 | 97.401% |
| execute | 16,352 | 1.771% |
| plan | 7,432 | 0.805% |
| normalize | 52 | 0.006% |
| sink_finish | 31 | 0.003% |
| validate | 27 | 0.003% |
| encode | 11 | 0.001% |
| lftj_build | 0 | 0.000% |
| hash_index | 0 | 0.000% |
| lftj_execute | 0 | 0.000% |
| hash_execute | 0 | 0.000% |
| sink_emit | 0 | 0.000% |
| decode | 0 | 0.000% |

**Trace Span Breakdown**

| Kind | Count | Execute busy (us) | Avg execute (us) | Dominant span | Dominant busy (us) | % of kind execute |
|---|---:|---:|---:|---|---:|---:|
| prepare | 1 | 923,000 | 923,000 | query_image.build | 899,000 | 97.400% |
| warmup | 2 | 34,200 | 17,100 | free_join.dispatch | 33,800 | 98.830% |
| sample | 30 | 482,000 | 16,066.7 | free_join.dispatch | 477,400 | 99.046% |

- Prepare-time waste is the whole-database query image: 899,000 us of 923,000 us in trace spans and 899,106 us of 923,100 us in `phase_timing`.
- Prepare planning is small but non-zero: 7,410 us in spans and 7,432 us in `phase_timing`, mostly `bumbledb.query.plan.stats` at 6,680 us.
- Measured sample-time image overhead is not the problem: 377.22 us across 30 samples, 12.574 us/sample, 0.078% of sample execute busy.
- Measured sample-time non-dispatch overhead is 4,600 us across 30 samples, about 153 us/sample, 0.954% of sample execute busy.
- The 30 measured samples are stable: individual sample spans range from 15,500 us to 17,100 us, matching the result JSON min/max of 15,559 us and 17,105 us.

**Query-Image Relation Breakdown**

| Relation | Busy (us) | % of 899,000 us image build | % of 923,000 us prepare execute |
|---|---:|---:|---:|
| CharName | 320,000 | 35.60% | 34.670% |
| Name | 216,000 | 24.03% | 23.402% |
| PersonInfo | 129,000 | 14.35% | 13.976% |
| CastInfo | 54,900 | 6.11% | 5.948% |
| MovieInfo | 41,900 | 4.66% | 4.540% |
| AkaName | 41,100 | 4.57% | 4.453% |
| CompanyName | 37,100 | 4.13% | 4.020% |
| Title | 29,000 | 3.23% | 3.142% |
| Keyword | 10,800 | 1.20% | 1.170% |
| MovieCompanies | 7,940 | 0.88% | 0.860% |
| MovieKeyword | 5,040 | 0.56% | 0.546% |
| MovieInfoIdx | 4,640 | 0.52% | 0.503% |
| AkaTitle | 441 | 0.05% | 0.048% |
| CompleteCast | 429 | 0.05% | 0.046% |
| MovieLink | 148 | 0.02% | 0.016% |
| CompCastType | 37 | 0.00% | 0.004% |
| CompanyType | 14.2 | 0.00% | 0.002% |
| InfoType | 12.8 | 0.00% | 0.001% |
| KindType | 7.54 | 0.00% | 0.001% |
| LinkType | 5.96 | 0.00% | 0.001% |
| RoleType | 5.58 | 0.00% | 0.001% |

- The top three relations, CharName, Name, and PersonInfo, consume 665,000 us, 73.97% of image build time and 72.05% of prepare execute time.
- The query image touches 21 relations even though the final runtime is a direct count kernel.
- The broad image scope, not the measured direct kernel, is the RCA for prepare latency.

**Allocation Deep Dive**

| Phase | Alloc calls | % calls | Bytes allocated | % bytes | Net bytes | Peak live bytes |
|---|---:|---:|---:|---:|---:|---:|
| total | 32,806,517 | 100.000% | 2,596,590,703 | 100.000% | 984,767,632 | 986,141,983 |
| query_image | 32,721,565 | 99.741% | 2,591,326,860 | 99.797% | 984,714,090 | 984,724,370 |
| execute | 57,325 | 0.175% | 1,618,952 | 0.062% | 7,608 | 1,374,552 |
| plan | 27,427 | 0.084% | 3,615,473 | 0.139% | 47,917 | 227,143 |
| normalize | 107 | 0.000% | 8,876 | 0.000% | 5,284 | 5,284 |
| sink_finish | 33 | 0.000% | 5,040 | 0.000% | -7,444 | 0 |
| encode_inputs | 18 | 0.000% | 3,326 | 0.000% | 0 | 0 |
| validate_inputs | 13 | 0.000% | 1,490 | 0.000% | 0 | 0 |

- Allocation ownership is unambiguous: `query_image` owns 99.741% of allocation calls and 99.797% of allocated bytes.
- The first execution retains nearly all live memory in the image: query_image net bytes are 984,714,090 out of total net bytes 984,767,632.
- The size-class vector starts with 32,783,276 allocations in the smallest bucket, consistent with per-cell `Vec<u8>` churn during column conversion.
- Execution allocation is comparatively small but still worth removing from the hot sample path: 57,325 calls and 1.619 MB allocated on the first execution.

**Code-Level RCA**

- The benchmark defines "prepare" as the first full Bumbledb query execution before warmups and samples. `crates/bumbledb-bench/src/main.rs:812-816` runs `txn.execute_query(...)` and records that elapsed time as `bumble_prepare`; `crates/bumbledb-bench/src/main.rs:846-883` then runs 2 warmups and 30 measured samples.
- Percentiles are computed only over the sample vector. `crates/bumbledb-bench/src/main.rs:638-655` sorts sample durations and computes avg/min/p50/p95/max, so the 923 ms cold image build is excluded from Bumbledb avg/p50/p95.
- Query execution always acquires a query image after direct-storage prechecks. `crates/bumbledb-lmdb/src/query.rs:1447-1454` wraps `self.query_images.get_or_build(self, schema)` in the `bumbledb.query.image` span and allocation phase.
- The query image cache is keyed only by schema fingerprint and storage transaction id. `crates/bumbledb-lmdb/src/query_image.rs:885-921` returns a cached `Arc<QueryImage>` on hit, otherwise builds one and stores it in a `BTreeMap`.
- The image builder is whole-schema, not query-scoped. `crates/bumbledb-lmdb/src/query_image.rs:949-979` iterates every relation in `schema.descriptor().relations`, which matches the 21 relation spans and explains why unrelated relations dominate this query.
- Segment-backed relation image build reads every field column and every durable index for a relation. `crates/bumbledb-lmdb/src/query_image.rs:1026-1041` loads each column with `txn.segment_bytes`, and `crates/bumbledb-lmdb/src/query_image.rs:1042-1101` loads every index byte vector into `RelationIndexImage`.
- `segment_bytes` clones LMDB value bytes into owned heap memory. `crates/bumbledb-lmdb/src/storage.rs:1392-1397` calls `map(ToOwned::to_owned)`, so each segment column and index is copied out of LMDB into the image.
- Column conversion creates avoidable per-cell heap objects. `crates/bumbledb-lmdb/src/query_image.rs:773-781` splits a flat segment byte vector into `Vec<Vec<u8>>`, then `crates/bumbledb-lmdb/src/query_image.rs:742-768` converts those per-cell vectors into typed arrays and collects another `Vec`; this is the direct source of tens of millions of tiny allocations.
- The fallback path is even more allocation-heavy if a segment is missing. `crates/bumbledb-lmdb/src/query_image.rs:1124-1185` scans the current primary index, pushes `bytes.to_vec()` for every field of every row, then clones raw columns again into `ColumnImage::from_bytes`.
- The measured runtime is direct because `execute_free_join` first attempts factorized count. `crates/bumbledb-lmdb/src/query.rs:2609-2625` runs `try_execute_factorized_count`, marks `runtime_kind = DirectKernel`, and returns without LFTJ execution when it succeeds.
- The factorized count kernel still allocates and sorts central values every execution. `crates/bumbledb-lmdb/src/query.rs:2735-2741` builds a `BTreeSet<Vec<u8>>` from the driver index, and `crates/bumbledb-lmdb/src/query.rs:2744-2750` counts matching index prefixes for each central value.
- Count output is mostly efficient once the kernel has a total. `crates/bumbledb-lmdb/src/query.rs:2776-2782` emits a count range, and `crates/bumbledb-lmdb/src/query.rs:7779-7785` applies that count directly to aggregate state.
- Sink finish still allocates ordinary result vectors. `crates/bumbledb-lmdb/src/query.rs:7832-7859` builds `Vec<Vec<Value>>` and sorts it, but for this query it only materializes 1 output value and is not the bottleneck.

**Allocation-Killing Rampage**

| Priority | Kill item | Concrete change | Expected effect | Risk |
|---:|---|---|---|---|
| 1 | Remove per-cell `Vec<Vec<u8>>` conversion in segment image build | Add `ColumnImage::from_flat_segment_bytes(field, width, bytes)` that converts `Vec<u8>` directly into `Vec<[u8; 1]>`, `Vec<[u8; 8]>`, or `Vec<[u8; 16]>` without `chunk.to_vec()` per row | Kills the dominant tiny allocation source; should remove most of the 32.7M query_image allocation calls and reduce build time substantially | Medium: needs careful width/alignment validation and tests for bool/u64/intern-id/decimal columns |
| 2 | Make query image relation-scoped | Derive required relation IDs from normalized atoms and direct-kernel needs, then build only those relation images instead of all schema relations | For this query, can avoid loading large unrelated relations such as CharName, Name, PersonInfo, AkaName, and Title if they are not required by the count kernel; potential prepare reduction approaching the 74% held by top three relations when irrelevant | High: planner stats, static-empty proof, and generic free-join planning currently assume a complete image |
| 3 | Add a direct-kernel planning path before full image acquisition | Detect factorized count candidates from the normalized query and schema, then request only durable index images required by `try_execute_factorized_count` | Turns broad-count queries into index-only cold execution instead of whole-database image build; removes the largest first-query artifact for this class | High: direct kernel currently reads `RelationImage.indexes()`, so a partial image or separate index-image API is needed |
| 4 | Stream unique central values without `BTreeSet<Vec<u8>>` | Because the driver index is sorted, iterate encoded entries in order and skip duplicate central value slices instead of inserting copied keys into `BTreeSet<Vec<u8>>` | Reduces sample-time allocation and CPU in the 16 ms direct kernel path; attacks the measured hot path, not just prepare | Medium: must preserve uniqueness when index entry ordering includes prefix bytes and covering components |
| 5 | Borrow or mmap segment bytes for immutable images | Store segment column/index bytes as `Arc<[u8]>`, LMDB-backed borrowed slices scoped to the read txn, or an owned shared slab instead of cloning every segment value into independent `Vec<u8>` | Reduces 2.591 GB allocated bytes and 984 MB live image footprint; improves cold image build memory pressure | High: lifetime and snapshot safety around LMDB values need a clear ownership model |
| 6 | Avoid `BTreeMap<String, ...>` in hot metadata caches | Replace relation/name lookup and prepared/static/hash-trie cache keys with interned relation IDs and compact key structs where possible | Small-to-medium reduction in plan/cache allocation churn; improves lookup locality | Low to medium: APIs currently expose string names heavily |
| 7 | Make aggregate count-only output path fully stack/static | For single `count` aggregate with no groups, bypass `BTreeMap<SmallEncodedRow, Vec<AggregateState>>` and return a one-row scalar directly | Removes small execute/sink allocation and simplifies direct count result materialization | Low: narrow specialization with clear semantics |
| 8 | Pre-size image metadata vectors from descriptors | Use field counts, index counts, and row counts to allocate exact capacities in relation/image builders | Removes reallocations and improves cold build smoothness | Low: mechanical optimization |
| 9 | Cache direct-kernel central-value summaries | Cache per-index distinct leading-key counts or compact prefix ranges in the query image | Reduces repeated 16 ms sample work for repeated identical snapshots | Medium: invalidation is tied to query image key but memory footprint must be controlled |

**Product Waste vs Harness Artifact**

- Product waste: the first query on a snapshot builds a full query image, reads all relation columns/indexes, allocates 2.591 GB, and retains 984.7 MB before executing a query that ultimately uses a direct factorized count kernel.
- Product waste: the image build is relation-broad; the top three relation image spans alone cost 665 ms even though the benchmark result row is a scalar count.
- Product waste: sample execution rebuilds a `BTreeSet<Vec<u8>>` of central values on every run, contributing to the 16 ms measured Bumbledb average.
- Harness artifact: allocation telemetry corresponds to the first Bumbledb execution, so the 32.8M calls and 986 MB peak do not describe each measured sample.
- Harness artifact: "prepare" is a full first query execution in the bench harness, not a separate prepare-only API.
- Not a harness artifact: users hitting a cold query image after load or after a new storage tx id will pay the same image-building cost unless it is prewarmed or made relation-scoped.
