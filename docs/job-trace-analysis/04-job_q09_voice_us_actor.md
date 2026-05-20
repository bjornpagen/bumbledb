# RCA: job_q09_voice_us_actor

## Verdict

`job_q09_voice_us_actor` is the heaviest measured LFTJ query in this traced run. The benchmark headline samples measure cached LFTJ execution at about 296.7 ms/query, while the first Bumbledb execution pays a separate 682.8 ms cold LFTJ atom-index build. Allocation telemetry is for that first execution, so the 17.25 M allocation calls and 2.75 GB allocated are mostly cold-build allocation, not steady-state sample allocation.

## Benchmark Numbers

| Metric | Value |
|---|---:|
| Rows | 1 |
| Runtime | Lftj |
| Plan family | FreeJoinLftj |
| Chosen plan | aggregate_pushdown |
| Compare mode | materialized |
| Query image built during query | true |
| Bumbledb samples | 30 |
| Bumbledb avg | 296,726 us |
| Bumbledb p50 | 294,560 us |
| Bumbledb p95 | 313,678 us |
| Bumbledb min | 285,017 us |
| Bumbledb max | 329,945 us |
| SQLite samples | 30 |
| SQLite avg | 434,220 us |
| SQLite p50 | 428,932 us |
| SQLite p95 | 476,548 us |
| SQLite min | 402,268 us |
| SQLite max | 486,818 us |
| SQLite/Bumbledb speed ratio | 1.463x |
| Bumbledb prepare | 982,866 us |
| SQLite prepare | 407,874 us |
| Bumbledb warmup avg | 298,523 us |
| SQLite warmup avg | 408,334 us |
| Execution order | prepare + 2 warmups + 30 samples |

## First-Execution Timing

`phase_timing` is from the first Bumbledb execution for this query. `execute` is the parent phase and includes both `lftj_build` and `lftj_execute`.

| Phase | Time | % of first execution total |
|---|---:|---:|
| total | 982,807 us | 100.000% |
| execute parent | 977,726 us | 99.483% |
| lftj_build | 682,770 us | 69.471% |
| lftj_execute | 294,944 us | 30.010% |
| plan | 4,896 us | 0.498% |
| normalize | 49 us | 0.005% |
| image | 28 us | 0.003% |
| sink_finish | 18 us | 0.002% |
| validate | 16 us | 0.002% |
| encode | 12 us | 0.001% |
| hash_index/hash_execute/sink_emit/decode | 0 us | 0.000% |

## Trace Timing

Trace spans separate prepare, warmup, and measured sample time. Warmups and samples reuse cached LFTJ atom tries, so their `lftj.build` spans are only cache/key overhead.

| Span group | Count | Busy time | % of group query.execute | Avg busy |
|---|---:|---:|---:|---:|
| prepare query.execute | 1 | 983,000.0 us | 100.000% | 983,000.0 us |
| prepare free_join.dispatch | 1 | 978,000.0 us | 99.491% | 978,000.0 us |
| prepare lftj.build | 1 | 683,000.0 us | 69.481% | 683,000.0 us |
| prepare lftj.execute | 1 | 295,000.0 us | 30.010% | 295,000.0 us |
| warmup query.execute | 2 | 597,000.0 us | 100.000% | 298,500.0 us |
| warmup lftj.execute | 2 | 597,000.0 us | 100.000% | 298,500.0 us |
| warmup sink.emit | 47,508 | 5,281.9 us | 0.885% | 0.111 us |
| warmup lftj.build | 2 | 52.1 us | 0.009% | 26.1 us |
| sample query.execute | 30 | 8,899,000.0 us | 100.000% | 296,633.3 us |
| sample free_join.dispatch | 30 | 8,894,000.0 us | 99.944% | 296,466.7 us |
| sample lftj.execute | 30 | 8,894,000.0 us | 99.944% | 296,466.7 us |
| sample sink.emit | 712,620 | 78,972.8 us | 0.887% | 0.111 us |
| sample lftj.build | 30 | 645.3 us | 0.007% | 21.5 us |
| sample image | 30 | 551.8 us | 0.006% | 18.4 us |
| sample normalize | 30 | 524.3 us | 0.006% | 17.5 us |
| sample sink.finish | 30 | 378.3 us | 0.004% | 12.6 us |

## Prepare vs Samples

| Waste bucket | Prepare | Samples | Interpretation |
|---|---:|---:|---|
| LFTJ build | 683,000.0 us, 69.481% | 645.3 us total, 0.007% | Cold atom-trie construction is the prepare-time cliff and disappears from measured samples after cache fill. |
| LFTJ execute | 295,000.0 us, 30.010% | 8,894,000.0 us total, 99.944% | Steady-state samples are real cached LFTJ traversal. |
| Sink emit | 2,645.0 us, 0.269% | 78,972.8 us total, 0.887% | Many emitted bindings collapse to one aggregate result; per-emit overhead is small but visible. |
| Planning | 4,880.0 us, 0.496% | not repeated materially | First execution plans; later executions hit prepared/image caches. |
| Prepare-only penalty | 686,140 us over sample avg | not applicable | Prepare is 3.31x a sample and 69.8% of prepare wall time is above steady-state average. |

## LFTJ Build and Execute

The cold build does eight atom build attempts. The dominant subphase is scan/filter/copy, not trie sorting.

| Detail | Prepare busy | % of prepare query.execute | Count | Avg busy |
|---|---:|---:|---:|---:|
| lftj.build total | 683,000.0 us | 69.481% | 1 | 683,000.0 us |
| scan_filter_copy | 506,779.5 us | 51.554% | 8 | 63,347.4 us |
| indexed_prefix relation=Name | 123,000.0 us | 12.513% | 1 | 123,000.0 us |
| sorted_trie wrapper | 115,563.9 us | 11.756% | 8 | 14,445.5 us |
| column_image | 53,115.0 us | 5.403% | 8 | 6,639.4 us |
| indexed_prefix relation=CompanyName | 10,300.0 us | 1.048% | 1 | 10,300.0 us |
| lftj.execute | 295,000.0 us | 30.010% | 1 | 295,000.0 us |
| sink.emit | 2,645.0 us | 0.269% | 23,754 | 0.111 us |

| Sorted trie detail | Prepare busy | % of prepare query.execute | Count |
|---|---:|---:|---:|
| relation=13 | 51,800.0 us | 5.270% | 1 |
| relation=8 | 28,600.0 us | 2.909% | 1 |
| relation=9 | 15,500.0 us | 1.577% | 1 |
| relation=11 | 7,870.0 us | 0.801% | 1 |
| relation=10 | 5,460.0 us | 0.555% | 1 |
| relation=15 | 4,680.0 us | 0.476% | 1 |
| relation=1 | 1,460.0 us | 0.149% | 1 |
| relation=7 | 0.834 us | 0.000% | 1 |

Across all 33 Bumbledb executions in the trace for this query, `lftj.execute` consumes 9.786 s, or 93.387% of total `query.execute` busy time. That aggregate view is dominated by the 30 measured samples; the prepare-only build still contributes 683.7 ms, or 6.524% of total traced Bumbledb execution time for this query.

## Allocation Deep Dive

Allocation telemetry is for the first Bumbledb execution. The `execute` phase is a parent and therefore overlaps `lftj_build` and `lftj_execute`.

| Metric | Value |
|---|---:|
| alloc_calls | 17,246,735 |
| dealloc_calls | 17,246,127 |
| realloc_calls | 97,927 |
| bytes_allocated | 2,749,266,511 |
| bytes_deallocated | 2,610,121,232 |
| net_bytes | 139,145,279 |
| current_live_bytes | 139,145,279 |
| peak_live_bytes | 199,048,869 |

| Phase | Alloc calls | % calls | Bytes allocated | % bytes | Net bytes | Peak live |
|---|---:|---:|---:|---:|---:|---:|
| total | 17,246,735 | 100.000% | 2,749,266,511 | 100.000% | 139,145,279 | 199,048,869 |
| execute parent | 17,224,894 | 99.873% | 2,746,716,756 | 99.907% | 139,117,196 | 199,013,379 |
| lftj_build | 16,579,615 | 96.132% | 2,649,330,360 | 96.365% | 139,110,180 | 199,013,227 |
| lftj_execute | 645,250 | 3.741% | 97,382,016 | 3.542% | 6,632 | 6,632 |
| plan | 21,619 | 0.125% | 2,488,214 | 0.091% | 29,795 | 29,795 |
| normalize | 115 | 0.001% | 9,315 | 0.000% | 5,555 | 5,555 |
| query_image | 14 | 0.000% | 24,004 | 0.001% | 0 | 0 |
| encode_inputs | 18 | 0.000% | 3,326 | 0.000% | 0 | 0 |
| validate_inputs | 13 | 0.000% | 1,490 | 0.000% | 0 | 0 |
| sink_finish | 33 | 0.000% | 5,040 | 0.000% | -7,444 | 0 |

Allocation ownership is unambiguous: `lftj_build` owns 16.58 M of 17.25 M calls and 2.649 GB of 2.749 GB allocated. `lftj_execute` is still non-trivial in call count at 645,250 calls and 97.4 MB allocated, but its net live memory is only 6.6 KB, so it is mostly transient per-traversal allocation. The live 139.1 MB and peak 199.0 MB are cold-build artifacts retained by the cached sorted tries and their temporary build products until cleanup.

## Code-Level RCA

| Finding | Source |
|---|---|
| LFTJ execution explicitly times and alloc-profiles build before execute, then runs `LftjExecutor` with a fresh `EncodedBinding`. | `crates/bumbledb-lmdb/src/query.rs:4864`, `query.rs:4884`, `query.rs:4913`, `query.rs:4932`, `query.rs:4949` |
| Atom tries are cached on the query image. A miss calls the build closure, wraps the result in `Arc`, and inserts it into a `BTreeMap` cache; this explains why warmups/samples no longer rebuild. | `crates/bumbledb-lmdb/src/query.rs:5430`, `query.rs:5443`, `query.rs:5447`, `crates/bumbledb-lmdb/src/query_image.rs:218`, `query_image.rs:243`, `query_image.rs:264` |
| `build_lftj_sorted_trie` creates a temporary schema and `raw_columns = vec![Vec::<Vec<u8>>::new(); variables.len()]`, so every retained atom value is stored as a heap-owned byte vector inside another heap vector. | `crates/bumbledb-lmdb/src/query.rs:5488`, `query.rs:5495`, `query.rs:5505` |
| The scan/filter/copy phase pushes copied byte vectors into `raw_columns` for both indexed-prefix and full-scan paths. This maps directly to the 506.8 ms `scan_filter_copy` span and 96.1% allocation-call owner. | `crates/bumbledb-lmdb/src/query.rs:5512`, `query.rs:5515`, `query.rs:5517`, `query.rs:5524`, `query.rs:5534` |
| Indexed prefix lookup reduces scanned rows but still materializes all matching rows into `IndexedLftjAtomValues { rows: Vec<Vec<Vec<u8>>> }` before column construction. | `crates/bumbledb-lmdb/src/query.rs:5599`, `query.rs:5604`, `query.rs:5659`, `query.rs:5667`, `query.rs:5672` |
| `atom_index_entry_values` and `atom_row_values` allocate `BTreeMap<usize, Vec<u8>>`, copy field bytes with `to_vec`, clone them again into ordered output vectors, and repeat this per row. | `crates/bumbledb-lmdb/src/query.rs:5687`, `query.rs:5688`, `query.rs:5701`, `query.rs:5720`, `query.rs:5835`, `query.rs:5843`, `query.rs:5856`, `query.rs:5875` |
| `ColumnImage::from_query_image_bytes` consumes `Vec<Vec<u8>>`, converts each small byte vector into fixed arrays, and collects a new typed vector, creating a second conversion pass after scan/filter/copy. | `crates/bumbledb-lmdb/src/query_image.rs:742`, `query_image.rs:750`, `query_image.rs:755`, `query_image.rs:762` |
| `SortedTrieIndex::build` allocates row order, sorts by repeated encoded-column comparisons, then builds key/range/parent vectors for every trie level. This explains the 115.6 ms sorted-trie wrapper and relation-specific trie spans. | `crates/bumbledb-lmdb/src/sorted_trie.rs:83`, `sorted_trie.rs:92`, `sorted_trie.rs:96`, `sorted_trie.rs:108`, `sorted_trie.rs:379`, `sorted_trie.rs:387` |
| Cached LFTJ execution still clones owned keys from borrowed trie references for leapfrog comparisons and bindings; this contributes to 645k transient execute allocations. | `crates/bumbledb-lmdb/src/query.rs:5172`, `query.rs:5198`, `query.rs:5310`, `query.rs:5325`, `query.rs:5367`, `query.rs:5403`, `query.rs:5409` |
| Project/aggregate sink overhead is low in percent but high-frequency: prepare emits 23,754 bindings and samples emit 712,620 bindings, all through `sink.emit` spans. | `crates/bumbledb-lmdb/src/query.rs:5147`, `query.rs:7487`, `query.rs:7572`, `query.rs:7790`, `query.rs:7801` |
| Aggregate output stores groups in `BTreeMap<SmallEncodedRow, Vec<AggregateState>>`; this avoids materializing every emitted row but still allocates and tree-searches group keys. | `crates/bumbledb-lmdb/src/query.rs:7751`, `query.rs:7766`, `query.rs:7780`, `query.rs:7801`, `query.rs:7870` |
| The benchmark harness first runs a real Bumbledb query for prepare/validation, then runs warmups, then samples. The first run fills caches and owns the allocation telemetry. | `crates/bumbledb-bench/src/main.rs:812`, `main.rs:815`, `main.rs:832`, `main.rs:846`, `main.rs:868`, `main.rs:944` |
| Benchmark result serialization records first-execution `timings` and `allocations` from `output.plan`, not an aggregate over the 30 measured samples. | `crates/bumbledb-bench/src/main.rs:990`, `main.rs:1027`, `main.rs:1028`, `main.rs:1415`, `main.rs:1436` |

## Allocation-Killing Rampage

| Priority | Kill item | Concrete change | Expected effect | Risk |
|---:|---|---|---|---|
| 1 | Kill temporary `Vec<Vec<u8>>` atom rows | Replace `raw_columns: Vec<Vec<Vec<u8>>>` with width-specialized appenders for `[u8; 1]`, `[u8; 8]`, and `[u8; 16]`; write retained values directly into typed column buffers during scan. | Directly targets the 16.58 M-call, 2.649 GB `lftj_build` owner; should remove one heap allocation per retained value and the `ColumnImage` conversion pass. | Medium: width/type handling and duplicate-variable checks must remain exact. |
| 2 | Remove per-row `BTreeMap<usize, Vec<u8>>` churn | Replace `BTreeMap` in `atom_index_entry_values` and `atom_row_values` with `SmallVec<[Option<EncodedOwned>; 8]>` or a dense slot array keyed by variable ordinal. | Cuts tree-node allocation, per-row comparisons through map lookup, and repeated `Vec<u8>` clones in the hottest scan/copy path. | Low/medium: variable ordering is already dense but duplicate-field semantics need targeted tests. |
| 3 | Stream indexed-prefix results into builders | Change `indexed_lftj_atom_values` from returning `Vec<Vec<Vec<u8>>>` to a callback/visitor that appends accepted index entries directly into column builders. | Removes the largest avoidable intermediate for the two visible indexed-prefix spans, including the 123 ms `Name` prefix scan. | Medium: helper API and error propagation become streaming. |
| 4 | Use direct prefix source iterators when index order matches LFTJ order | If the durable relation index already has the bound prefix plus variable order needed by an atom, create a trie/linear iterator over the index prefix range instead of copying rows and rebuilding a sorted trie. | Potentially eliminates most cold `scan_filter_copy`, `column_image`, and `sorted_trie` work for indexed atoms. | High: needs a correct duplicate-collapsing trie interface over existing index entries. |
| 5 | Reuse sorted trie build scratch | Allocate reusable scratch for `order`, `TrieLevel.keys`, `TrieLevel.ranges`, `TrieLevel.parent`, and column builders within a scoped build context; pre-size from retained row estimates. | Reduces reallocations, peak live bytes, and allocator pressure during unavoidable cold builds. | Medium: concurrent query-image cache builds must not share mutable scratch unsafely. |
| 6 | Borrow keys through leapfrog instead of cloning on every key read | Keep `EncodedRef` or width-specialized copied scalar in `LeapfrogState` comparisons and bind only when descending. Avoid `EncodedOwned::from_ref` on every `key_owned_opt`. | Attacks the 645k call/97.4 MB transient `lftj_execute` allocation bucket and steady-state sample cost. | Medium: lifetimes across iterator movement are tricky; copied scalar enum may be safer than borrowed refs. |
| 7 | Specialize aggregate sink for single-group/count-heavy output | For one-row aggregate outputs, avoid `BTreeMap<SmallEncodedRow, Vec<AggregateState>>` and update a single state directly when group vars are empty or fixed. | Reduces high-frequency `sink.emit` overhead; sample sink emits cost only 0.887%, so this is secondary. | Low/medium: must preserve grouped aggregate behavior when group vars exist. |
| 8 | Replace string cache keys with typed keys | Replace `lftj_atom_cache_key` string/hex construction with an ordered typed key containing relation, field IDs, term tags, and encoded literals/inputs. | Cuts cached-build overhead in warmups/samples and avoids cold key-string allocation. | Medium: touches cache map key type and diagnostics. |

## Product vs Artifact

- Real product waste: temporary atom rows are copied into `Vec<Vec<u8>>`, converted into typed column images, then sorted into new tries. This is real cold-query latency and memory pressure whenever the query image lacks the atom trie.
- Real product waste: cached LFTJ traversal is genuinely expensive for this query; samples spend 8.894 s of 8.899 s in `lftj.execute` across 30 measured runs.
- Real product waste: `sink.emit` runs 712,620 times across samples, but it is under 1% of sample busy time and is not the primary bottleneck.
- Benchmark/first-query artifact: allocation telemetry reflects the first execution, so `lftj_build` owns 96.1% of allocation calls and 96.4% of bytes even though measured samples do not rebuild.
- Benchmark/first-query artifact: Bumbledb prepare is 982.9 ms because it includes the 682.8 ms cold build; the measured average is 296.7 ms after cache warmup.
- Not benchmark-only: the cold build is still product waste for first execution, cache misses, changed query images, and workloads with many distinct atom shapes or input literals.
