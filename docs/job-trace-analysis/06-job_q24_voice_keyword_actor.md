# RCA: job_q24_voice_keyword_actor

## Verdict

`job_q24_voice_keyword_actor` is fast in steady state but pays a very large cold LFTJ atom-index build on the first Bumbledb execution. The product waste is real in the temporary LFTJ build path because it materializes filtered atom rows as heap-owned byte vectors, converts them into column images, then sorts/builds tries. The benchmark headline samples mostly measure cached LFTJ execution, while the allocation telemetry is explicitly the first execution and is therefore dominated by cold-build allocation.

## Benchmark Numbers

| Metric | Value |
|---|---:|
| Rows | 12 |
| Runtime | Lftj |
| Plan family | FreeJoinLftj |
| Bumbledb samples | 30 |
| Bumbledb avg | 2,689 us |
| Bumbledb p50 | 2,672 us |
| Bumbledb p95 | 2,754 us |
| Bumbledb min | 2,603 us |
| Bumbledb max | 3,260 us |
| SQLite avg | 385,657 us |
| SQLite p50 | 385,555 us |
| SQLite p95 | 408,680 us |
| SQLite min | 351,452 us |
| SQLite max | 409,482 us |
| SQLite/Bumbledb speed ratio | 143.42x |
| Bumbledb prepare | 37,288 us |
| SQLite prepare | 367,299 us |
| Warmup order | prepare + 2 warmups + 30 samples |
| Bumbledb warmup avg | 2,989 us |
| SQLite warmup avg | 365,657 us |

## Phase Timing

`phase_timing` is from the first Bumbledb execution for this query, which is also the execution used for allocation telemetry.

| Phase | Time | % of first execution total |
|---|---:|---:|
| total | 37,225 us | 100.000% |
| execute parent | 36,361 us | 97.679% |
| lftj_build | 33,592 us | 90.240% |
| lftj_execute | 2,756 us | 7.404% |
| plan | 649 us | 1.743% |
| normalize | 50 us | 0.134% |
| sink_finish | 43 us | 0.116% |
| image | 31 us | 0.083% |
| validate | 16 us | 0.043% |
| encode | 11 us | 0.030% |
| sink_emit | 0 us | 0.000% |
| hash_index/hash_execute/decode | 0 us | 0.000% |

Trace spans split the same execution order into prepare, warmups, and measured samples.

| Span group | Count | Busy time | % of group query.execute | Avg busy |
|---|---:|---:|---:|---:|
| prepare query.execute | 1 | 37,200.0 us | 100.000% | 37,200.0 us |
| prepare free_join.dispatch | 1 | 36,400.0 us | 97.849% | 36,400.0 us |
| prepare lftj.build | 1 | 33,600.0 us | 90.323% | 33,600.0 us |
| prepare lftj.execute | 1 | 2,750.0 us | 7.392% | 2,750.0 us |
| warmup query.execute | 2 | 5,880.0 us | 100.000% | 2,940.0 us |
| warmup lftj.execute | 2 | 5,410.0 us | 92.007% | 2,705.0 us |
| warmup lftj.build | 2 | 61.2 us | 1.041% | 30.6 us |
| sample query.execute | 30 | 80,010.0 us | 100.000% | 2,667.0 us |
| sample free_join.dispatch | 30 | 75,930.0 us | 94.901% | 2,531.0 us |
| sample lftj.execute | 30 | 74,780.0 us | 93.463% | 2,492.7 us |
| sample sink.emit | 6,570 | 1,552.1 us | 1.940% | 0.236 us |
| sample lftj.build | 30 | 498.6 us | 0.623% | 16.62 us |
| sample sink.finish | 30 | 337.6 us | 0.422% | 11.25 us |
| sample image | 30 | 274.6 us | 0.343% | 9.15 us |

## Prepare-Time Waste vs Sample-Time Waste

| Waste bucket | Prepare | Samples | Interpretation |
|---|---:|---:|---|
| LFTJ build | 33,600.0 us, 90.323% | 498.6 us total, 0.623% | Cold sorted-trie construction is the prepare-time cliff; sample cost is mostly cache lookup/key overhead. |
| LFTJ execute | 2,750.0 us, 7.392% | 74,780.0 us total, 93.463% | Steady-state samples are real LFTJ traversal cost. |
| Sink emit | 53.0 us, 0.142% | 1,552.1 us total, 1.940% | There are 219 emits per execution, 12 unique output rows after projection/dedup. |
| Planning | 623.0 us, 1.675% | not repeated in samples | First execution plans; samples hit prepared plan/image caches. |

## LFTJ Build and Execute Breakdown

| Detail | Prepare busy | % of prepare query.execute | Count | Avg busy |
|---|---:|---:|---:|---:|
| lftj.build total | 33,600.0 us | 90.323% | 1 | 33,600.0 us |
| scan_filter_copy | 25,735.0 us | 69.180% | 3 | 8,578.3 us |
| sorted_trie wrapper | 6,146.7 us | 16.523% | 3 | 2,048.9 us |
| sorted_trie relation=18 | 5,700.0 us | 15.323% | 1 | 5,700.0 us |
| sorted_trie relation=10 | 399.0 us | 1.073% | 1 | 399.0 us |
| sorted_trie relation=4 | 0.75 us | 0.002% | 1 | 0.75 us |
| column_image | 1,504.2 us | 4.043% | 3 | 501.4 us |
| indexed_prefix relation=Keyword | 1.42 us | 0.004% | 1 | 1.42 us |
| lftj.execute | 2,750.0 us | 7.392% | 1 | 2,750.0 us |
| sink.emit | 53.0 us | 0.142% | 219 | 0.242 us |

The first execution builds three LFTJ atom tries. One large relation dominates trie sort/build time: relation `18` consumes 5.7 ms of sorted-trie build time, or 92.7% of the 6.15 ms `sorted_trie` wrapper total. The biggest build subphase is not sort, but `scan_filter_copy`: 25.7 ms and 69.2% of prepare query execution.

## Allocation Deep Dive

| Metric | Value |
|---|---:|
| alloc_calls | 942,537 |
| dealloc_calls | 942,008 |
| realloc_calls | 2,960 |
| bytes_allocated | 109,457,119 |
| bytes_deallocated | 104,389,060 |
| net_bytes | 5,068,059 |
| current_live_bytes | 5,068,059 |
| peak_live_bytes | 5,068,059 |

| Phase | Alloc calls | % calls | Bytes allocated | % bytes | Net bytes | Peak live |
|---|---:|---:|---:|---:|---:|---:|
| total | 942,537 | 100.000% | 109,457,119 | 100.000% | 5,068,059 | 5,068,059 |
| execute parent | 926,532 | 98.302% | 108,612,731 | 99.229% | 5,058,854 | 5,058,854 |
| lftj_build | 920,431 | 97.655% | 107,683,203 | 98.379% | 5,037,878 | 5,037,878 |
| lftj_execute | 6,070 | 0.644% | 924,940 | 0.845% | 20,768 | 20,768 |
| plan | 15,758 | 1.672% | 779,246 | 0.712% | 22,362 | 22,362 |
| normalize | 131 | 0.014% | 11,010 | 0.010% | 6,758 | 6,758 |
| sink_finish | 44 | 0.005% | 7,020 | 0.006% | -20,018 | 0 |
| query_image | 14 | 0.001% | 24,004 | 0.022% | 0 | 0 |
| encode_inputs | 18 | 0.002% | 3,326 | 0.003% | 0 | 0 |
| validate_inputs | 13 | 0.001% | 1,490 | 0.001% | 0 | 0 |

Allocation ownership is unambiguous: `lftj_build` owns 920,431 of 942,537 allocation calls and 107.7 MB of 109.5 MB allocated. `execute` is a parent phase, so its 98.3% call share mostly wraps `lftj_build`; the incremental steady-state `lftj_execute` allocation is only 0.644% of calls and 0.845% of bytes.

## Code-Level RCA

| Finding | Source |
|---|---|
| Pure LFTJ execution explicitly times build separately from execute, then records build allocations before running `LftjExecutor`. | `crates/bumbledb-lmdb/src/query.rs:4864`, `query.rs:4884`, `query.rs:4913`, `query.rs:4932`, `query.rs:4949` |
| LFTJ atom plans are cached by query image, so the huge build is cold-only when cache misses and later executions mostly reuse sorted tries. | `crates/bumbledb-lmdb/src/query.rs:5430`, `query.rs:5443`, `crates/bumbledb-lmdb/src/query_image.rs:218`, `query_image.rs:243`, `query_image.rs:264` |
| `build_lftj_sorted_trie` creates `fields: Vec<FieldImage>` and `raw_columns = vec![Vec::<Vec<u8>>::new(); variables.len()]`, which makes the temporary representation per column a heap vector of heap vectors. | `crates/bumbledb-lmdb/src/query.rs:5488`, `query.rs:5495`, `query.rs:5505` |
| The scan path pushes an owned `Vec<u8>` per retained variable value into `raw_columns`; this is the measured `scan_filter_copy` cliff. | `crates/bumbledb-lmdb/src/query.rs:5512`, `query.rs:5515`, `query.rs:5517`, `query.rs:5524`, `query.rs:5534` |
| Indexed prefix lookup can avoid full relation scans, but still materializes `rows: Vec<Vec<Vec<u8>>>` and copies field bytes through `atom_index_entry_values`. | `crates/bumbledb-lmdb/src/query.rs:5599`, `query.rs:5604`, `query.rs:5659`, `query.rs:5667`, `query.rs:5672`, `query.rs:5687` |
| Both indexed and non-indexed atom row extraction use `BTreeMap<usize, Vec<u8>>`, causing tree churn and byte copies per row before producing an ordered `Vec<Vec<u8>>`. | `crates/bumbledb-lmdb/src/query.rs:5688`, `query.rs:5701`, `query.rs:5720`, `query.rs:5842`, `query.rs:5856`, `query.rs:5875` |
| `ColumnImage::from_query_image_bytes` consumes `Vec<Vec<u8>>`, copies each byte vector into fixed arrays, then collects a new typed `Vec`, so column build pays another conversion pass. | `crates/bumbledb-lmdb/src/query_image.rs:742`, `query_image.rs:750`, `query_image.rs:755`, `query_image.rs:762` |
| `SortedTrieIndex::build` allocates a full row-order vector, sorts by repeated column byte comparisons, then builds per-level key/range/parent vectors. | `crates/bumbledb-lmdb/src/sorted_trie.rs:83`, `sorted_trie.rs:92`, `sorted_trie.rs:96`, `sorted_trie.rs:108`, `sorted_trie.rs:379`, `sorted_trie.rs:387` |
| LFTJ execution allocates iterator stacks with `Vec`, participant lists with `SmallVec`, and repeatedly clones owned keys from trie refs into the binding path. | `crates/bumbledb-lmdb/src/sorted_trie.rs:121`, `sorted_trie.rs:236`, `crates/bumbledb-lmdb/src/query.rs:4958`, `query.rs:5172`, `query.rs:5198`, `query.rs:5403` |
| Project output emits every yielded binding into a `BTreeSet<SmallEncodedRow>`, so 219 emitted bindings become 12 rows after dedup. This is steady-state overhead but only 1.94% of sample busy time. | `crates/bumbledb-lmdb/src/query.rs:5147`, `query.rs:7617`, `query.rs:7640`, `query.rs:7645`, `query.rs:7657` |
| The benchmark prepare execution is a real timed query before warmups and samples; it materializes rows, validates against SQLite, then samples run after caches are warm. | `crates/bumbledb-bench/src/main.rs:812`, `main.rs:815`, `main.rs:832`, `main.rs:846`, `main.rs:868`, `main.rs:944` |

## Allocation-Killing Rampage

| Priority | Kill item | Concrete change | Expected effect | Risk |
|---:|---|---|---|---|
| 1 | Delete temporary `Vec<Vec<u8>>` atom rows | Replace `raw_columns: Vec<Vec<Vec<u8>>>` with width-specialized column builders that append `[u8; 1]`, `[u8; 8]`, or `[u8; 16]` directly while scanning. | Attacks the 920k-call/107.7 MB `lftj_build` owner; should remove one heap allocation per retained value plus the column conversion pass. | Medium: needs careful width/type handling and comparison semantics. |
| 2 | Remove per-row `BTreeMap<usize, Vec<u8>>` in atom extraction | Use `SmallVec<[Option<EncodedOwned>; N]>` or a fixed slot vector indexed by dense variable ordinal; compare duplicates in-place and emit ordered slots without tree lookup. | Cuts tree-node allocation/churn in `atom_row_values` and `atom_index_entry_values`; reduces `scan_filter_copy` time and tiny allocation calls. | Low/medium: variable ordinals are already dense in `variables`; duplicate-variable checks must remain exact. |
| 3 | Stream indexed-prefix rows directly into column builders | Change `indexed_lftj_atom_values` from returning `Vec<Vec<Vec<u8>>>` to invoking a row visitor or builder callback over `RelationIndexPrefixIter`. | Avoids retaining all indexed rows as an intermediate nested vector; makes indexed-prefix path actually low-allocation. | Medium: changes helper API and error flow. |
| 4 | Reuse sorted trie build buffers inside query image | Keep scratch `order`, `TrieLevel` vectors, and column builders scoped to one build or cache object; pre-size from row-count estimates. | Reduces reallocations and peak temporary memory during cold LFTJ builds. | Medium: cache/scratch ownership must not leak across concurrent readers. |
| 5 | Add direct prefix source usage for LFTJ atoms | When durable index order already matches LFTJ variable order after bound prefix filtering, build an iterator over index entries instead of copying to a temporary relation and rebuilding a sorted trie. | Can eliminate most cold build work for atoms served by existing sorted indexes; directly targets `scan_filter_copy` plus sorted-trie build. | High: needs a trie/linear iterator over durable index prefix ranges with correct duplicate collapse. |
| 6 | Specialize project sink for tiny outputs | Replace `BTreeSet<SmallEncodedRow>` with a small linear set or sorted small buffer until row count crosses a threshold. | Reduces sample `sink.emit` overhead for 219 emits/12 outputs; likely small but steady-state. | Low: must preserve dedup and output ordering behavior. |
| 7 | Convert cache keys away from string construction | Replace `lftj_atom_cache_key` string/hex building with a typed key struct containing relation, fields, term tags, and encoded values. | Reduces cold and cached-build overhead; helps the 0.623% sample `lftj.build` cache-check path. | Medium: requires deriving stable `Ord`/`Hash` and cache map changes. |

## Product Waste vs Benchmark Artifact

- Real product waste: `build_lftj_sorted_trie` copies retained atom values into nested `Vec<Vec<u8>>`, creates typed columns from those vectors, and builds temporary sorted tries. This is algorithmic product overhead whenever the query image lacks a cached LFTJ atom trie.
- Real product waste: project sink dedup via `BTreeSet<SmallEncodedRow>` emits 219 bindings for 12 output rows; this is small for this query but steady-state and visible in samples.
- Benchmark/first-query artifact: the 942,537 allocation calls and 109.5 MB allocated are from the first Bumbledb execution, not the 30 measured sample executions.
- Benchmark/first-query artifact: `prepare` includes the cold 33.6 ms LFTJ build; samples average 2.689 ms with cached sorted tries.
- Not a current issue: SQLite remains 143.42x slower on sample avg, so this RCA is about Bumbledb allocation and cold-query latency rather than benchmark failure.
