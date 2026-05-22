# job_broad_movie_info_star RCA

Historical note: this trace report predates the set-native v4 rewrite. References to factorized count, hash probe, direct-count dispatch, or line-number anchors describe removed legacy systems.

## Verdict

- The measured sample-time cost is real product execution cost: `305.670 ms` of `310.230 ms` sample busy time is inside `bumbledb.query.free_join.dispatch`, which dispatches to the direct count kernel, so `98.53%` of measured sample time is the count/probe loop.
- The biggest allocation byte source is first-query planning, not steady-state sample execution: `plan` owns `1,595,918 B` (`64.01%`) and `17,350` calls (`32.55%`) in the first Bumbledb execution.
- The biggest allocation call source is direct-kernel execution: `execute` owns `35,728` calls (`67.03%`) and `834,104 B` (`33.46%`) during the first execution, mostly from the factorized count path materializing central keys with `BTreeSet<Vec<u8>>`.
- Benchmark prepare is not a pure prepare phase. `bumbledb-bench` executes the query once before warmups/samples at `crates/bumbledb-bench/src/main.rs:812-831`, then runs warmups and samples at `crates/bumbledb-bench/src/main.rs:846-888`.
- Product waste to fix first: direct-kernel heap churn and probe-loop overhead. Benchmark/first-query artifact to treat separately: planner stats, optimizer candidate construction, and prepared-plan insertion on the first execution.

## Exact Numbers

| metric | value |
|---|---:|
| rows | 1 |
| chosen plan | `aggregate_pushdown` |
| runtime | `DirectKernel` |
| plan family | `FreeJoinLftj` |
| compare mode | `materialized` |
| Bumbledb samples | 30 |
| Bumbledb avg | 10,372 us |
| Bumbledb p50 | 10,281 us |
| Bumbledb p95 | 11,084 us |
| Bumbledb min | 9,961 us |
| Bumbledb max | 11,658 us |
| SQLite samples | 30 |
| SQLite avg | 5,300,376 us |
| SQLite p50 | 5,296,854 us |
| SQLite p95 | 5,346,890 us |
| SQLite min | 5,242,101 us |
| SQLite max | 5,375,989 us |
| SQLite / Bumbledb avg ratio | 511.027x |
| Bumbledb prepare | 12,828 us |
| SQLite prepare | 5,337,185 us |
| Bumbledb warmup | 2 samples, 11,109 us avg |
| SQLite warmup | 2 samples, 5,310,914 us avg |

## Query Shape

- Source query is a broad star count over `?movie` with 10 atoms and no inputs or predicates, defined at `crates/bumbledb-bench/src/open.rs:963-996`.
- The central count variable appears in `Title`, `CastInfo`, `MovieCompanies`, `MovieKeyword`, `MovieInfo`, and `MovieInfoIdx`; dimension atoms validate role/company/keyword/info-type IDs.
- Historical legacy detail: this shape matched a deleted direct count-kernel family that required no inputs, no predicates, aggregate count, no groups, and at least two fact indexes.

## Timing: Phase Timing

First Bumbledb execution phase timings from `phase_timing`:

| phase | us | pct of total |
|---|---:|---:|
| total | 12,773 | 100.000% |
| execute | 10,345 | 80.991% |
| plan | 2,288 | 17.913% |
| image | 33 | 0.258% |
| sink_finish | 22 | 0.172% |
| normalize | 17 | 0.133% |
| validate | 15 | 0.117% |
| encode | 11 | 0.086% |
| lftj_build | 0 | 0.000% |
| hash_index | 0 | 0.000% |
| lftj_execute | 0 | 0.000% |
| hash_execute | 0 | 0.000% |
| sink_emit | 0 | 0.000% |
| decode | 0 | 0.000% |

## Timing: Trace Spans

All 33 Bumbledb executions are `1 prepare + 2 warmups + 30 samples`.

| span | count | busy us | avg busy us | pct of execute busy |
|---|---:|---:|---:|---:|
| `bumbledb.query.execute` | 33 | 345,130.00 | 10,458.49 | 100.000% |
| `bumbledb.query.free_join.dispatch` | 33 | 337,670.00 | 10,232.42 | 97.838% |
| `bumbledb.query.plan` | 1 | 2,270.00 | 2,270.00 | 0.658% |
| `bumbledb.query.plan.stats` | 1 | 1,720.00 | 1,720.00 | 0.498% |
| `bumbledb.query.image` | 33 | 442.95 | 13.42 | 0.128% |
| `bumbledb.query.sink.finish` | 33 | 440.40 | 13.35 | 0.128% |
| `bumbledb.query.plan.optimize_free_join` | 1 | 250.00 | 250.00 | 0.072% |
| `bumbledb.query.plan.variable_order` | 1 | 223.00 | 223.00 | 0.065% |
| `bumbledb.query.normalize` | 33 | 162.76 | 4.93 | 0.047% |
| `bumbledb.query.aggregate` | 33 | 30.82 | 0.93 | 0.009% |
| `bumbledb.query.static_empty.prove` | 33 | 15.58 | 0.47 | 0.005% |
| `bumbledb.query.encode_inputs` | 33 | 14.12 | 0.43 | 0.004% |
| `bumbledb.query.validate_inputs` | 33 | 5.00 | 0.15 | 0.001% |

## Prepare-Time Waste

| prepare span | busy us | pct of prepare execute |
|---|---:|---:|
| `bumbledb.query.execute` | 12,800.00 | 100.000% |
| `bumbledb.query.free_join.dispatch` | 10,300.00 | 80.469% |
| `bumbledb.query.plan` | 2,270.00 | 17.734% |
| `bumbledb.query.plan.stats` | 1,720.00 | 13.438% |
| `bumbledb.query.plan.optimize_free_join` | 250.00 | 1.953% |
| `bumbledb.query.plan.variable_order` | 223.00 | 1.742% |
| `bumbledb.query.image` | 17.20 | 0.134% |
| `bumbledb.query.sink.finish` | 12.20 | 0.095% |
| `bumbledb.query.normalize` | 6.42 | 0.050% |

- Prepare-time one-shot planning cost is `2.270 ms`, or `17.734%` of prepare execution busy time.
- Planner stats are `1.720 ms`, or `75.77%` of plan time and `13.438%` of prepare execution busy time.
- Prepare also includes `10.300 ms` of real direct-kernel execution; that is not planning waste.
- The prepared-plan cache path is in `ReadTxn::execute_query`: cache lookup and insertion happen at `crates/bumbledb-lmdb/src/query.rs:1516-1554`, and cached plans are cloned/reset through `ExecutionPlan::instantiate` at `crates/bumbledb-lmdb/src/query.rs:1157-1177`.

## Sample-Time Waste

| sample span | count | busy us | avg busy us | pct of sample execute |
|---|---:|---:|---:|---:|
| `bumbledb.query.execute` | 30 | 310,230.00 | 10,341.00 | 100.000% |
| `bumbledb.query.free_join.dispatch` | 30 | 305,670.00 | 10,189.00 | 98.530% |
| `bumbledb.query.sink.finish` | 30 | 394.80 | 13.16 | 0.127% |
| `bumbledb.query.image` | 30 | 388.95 | 12.97 | 0.125% |
| `bumbledb.query.normalize` | 30 | 142.21 | 4.74 | 0.046% |
| `bumbledb.query.aggregate` | 30 | 27.19 | 0.91 | 0.009% |
| `bumbledb.query.static_empty.prove` | 30 | 13.83 | 0.46 | 0.004% |
| `bumbledb.query.encode_inputs` | 30 | 9.46 | 0.32 | 0.003% |
| `bumbledb.query.validate_inputs` | 30 | 3.63 | 0.12 | 0.001% |

- Measured sample-time non-dispatch overhead is `4,560 us` total, or `152 us` per sample, or `1.47%` of sample execution busy time.
- There is no sample-time planner span; sample iterations reuse the prepared plan.
- `image` is a cheap cache acquisition during samples: `12.97 us` avg and `0.125%` of sample execution busy time.

## Direct-Kernel Dispatch

| counter/span | value |
|---|---:|
| direct kernel probes | 37,151 |
| direct kernel rows | 54,075,120 |
| direct kernel predicates | 0 |
| materialized output values | 1 |
| cursor seeks | 0 |
| rows scanned | 0 |
| dictionary reverse lookups | 0 |
| trace dispatch busy, all executions | 337,670 us |
| trace dispatch pct, all executions | 97.838% |
| trace dispatch busy, samples | 305,670 us |
| trace dispatch pct, samples | 98.530% |

- Historical legacy detail: dispatch entered `execute_free_join` and tried a deleted count-kernel family before ordinary direct-kernel, deleted hash-probe, mixed, or LFTJ execution.
- This query uses the factorized count special case, not ordinary tuple materialization. The code builds fact indexes, picks the smallest driver by encoded byte size, deduplicates central movie IDs, probes each fact index by prefix, multiplies counts, and emits one aggregate count at `crates/bumbledb-lmdb/src/query.rs:2698-2783`.
- `direct_kernel_rows = 54,075,120` is the computed factorized cardinality, not materialized row output. The output remains one row.
- `direct_kernel_probes = 37,151` reflects prefix-count probes across central values and fact indexes, with early exits when a count is zero.

## Allocation Summary

Allocation telemetry corresponds to the first Bumbledb execution for this query.

| metric | value |
|---|---:|
| alloc calls | 53,301 |
| dealloc calls | 52,741 |
| realloc calls | 1,727 |
| bytes allocated | 2,493,228 |
| bytes deallocated | 2,456,337 |
| net bytes | 36,891 |
| current live bytes | 36,891 |
| peak live bytes | 36,891 |

| phase | alloc calls | pct calls | bytes allocated | pct bytes | net bytes | peak live |
|---|---:|---:|---:|---:|---:|---:|
| total | 53,301 | 100.000% | 2,493,228 | 100.000% | 36,891 | 36,891 |
| execute | 35,728 | 67.031% | 834,104 | 33.455% | 7,608 | 7,608 |
| plan | 17,350 | 32.551% | 1,595,918 | 64.010% | 29,610 | 29,610 |
| normalize | 116 | 0.218% | 10,980 | 0.440% | 6,940 | 6,940 |
| sink_finish | 33 | 0.062% | 5,040 | 0.202% | -7,444 | 0 |
| encode_inputs | 18 | 0.034% | 3,326 | 0.133% | 0 | 0 |
| query_image | 14 | 0.026% | 24,004 | 0.963% | 0 | 0 |
| validate_inputs | 13 | 0.024% | 1,490 | 0.060% | 0 | 0 |
| lftj_build | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| hash_index | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| lftj_execute | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| hash_execute | 0 | 0.000% | 0 | 0.000% | 0 | 0 |

- Allocation calls are owned by execution: `35,728 / 53,301 = 67.03%`.
- Allocation bytes are owned by planning: `1,595,918 / 2,493,228 = 64.01%`.
- Live memory after the first execution is mostly planner/cache product: `plan` net is `29,610 B`; `execute` net is `7,608 B`; `normalize` net is `6,940 B`; `sink_finish` frees `7,444 B` net.
- Size-class distribution is small-object heavy: `40,990 / 53,301 = 76.90%` of allocation calls are in the smallest reported class, and the first two classes account for `46,315 / 53,301 = 86.89%`.

## Code-Level RCA

- Benchmark harness first runs `execute_query` and treats that elapsed time as Bumbledb prepare at `crates/bumbledb-bench/src/main.rs:812-831`; warmups and samples then call `execute_query` again at `crates/bumbledb-bench/src/main.rs:846-888`.
- Query execution alloc/timing phases are explicitly bracketed in `ReadTxn::execute_query`: image acquisition at `crates/bumbledb-lmdb/src/query.rs:1447-1454`, plan/cache at `crates/bumbledb-lmdb/src/query.rs:1516-1554`, execution at `crates/bumbledb-lmdb/src/query.rs:1558-1570`, and sink finish at `crates/bumbledb-lmdb/src/query.rs:1572-1587`.
- Planner stats collection builds a per-query `BTreeMap<String, Arc<OptimizerRelationStats>>` and clones relation names at `crates/bumbledb-lmdb/src/query.rs:1180-1205`.
- Relation stats build uses `BTreeMap<String, OptimizerFieldStats>` and `BTreeMap<String, OptimizerIndexStats>` at `crates/bumbledb-lmdb/src/planner_stats.rs:145-174`, and field sampling allocates a `BTreeMap<EncodedOwned, usize>` plus heavy-hitter `Vec` at `crates/bumbledb-lmdb/src/planner_stats.rs:191-225`.
- Variable order selection rebuilds and sorts a `Vec<VariableCost>` for every variable depth, with `BTreeSet` remaining/bound sets and string cloning in the sort key at `crates/bumbledb-lmdb/src/query.rs:6038-6077`.
- Optimizer candidate construction builds four full plans for aggregate queries (`pure_lftj`, `hash_probe`, `hybrid`, `aggregate_pushdown`), sorts candidates, clones the chosen plan, and converts candidates to public trace records at `crates/bumbledb-lmdb/src/query.rs:6464-6562`.
- `build_plan_candidate` creates a `tie_breaker` string by formatting each implementation into a `Vec<String>` and joining it at `crates/bumbledb-lmdb/src/query.rs:6578-6634`.
- `build_free_join_plan` allocates `Vec` nodes, `Vec` subatoms, `Vec` fields, `Vec` vars, and repeated payload demand for every variable and candidate at `crates/bumbledb-lmdb/src/query.rs:6649-6709`.
- Direct count execution builds `central_values = BTreeSet::<Vec<u8>>::new()` and inserts `bytes.to_vec()` for each driver entry at `crates/bumbledb-lmdb/src/query.rs:2735-2741`; this directly explains the high first-execution execute call count and small-object allocation profile.
- Direct count then loops central values and fact indexes, issuing `entries_with_prefix(...).count()` probes and multiplying counts at `crates/bumbledb-lmdb/src/query.rs:2744-2761`; this explains the `37,151` direct probes and `10.189 ms` sample dispatch average.
- The count result is emitted through the generic aggregate sink using `sink.emit_count_range` and an `EncodedBinding::new(query.vars.len())` at `crates/bumbledb-lmdb/src/query.rs:2776-2781`.
- Generic aggregate output still stores one group in `BTreeMap<SmallEncodedRow, Vec<AggregateState>>`, initializes aggregate states with a `Vec`, decodes on finish, and sorts rows at `crates/bumbledb-lmdb/src/query.rs:7750-7859`.
- Query-image cache and hash/sorted-trie caches use `RwLock<BTreeMap<...>>` and `Arc` lookups/inserts at `crates/bumbledb-lmdb/src/query_image.rs:93-103`, `crates/bumbledb-lmdb/src/query_image.rs:218-307`, and `crates/bumbledb-lmdb/src/query_image.rs:862-921`; image lookup is cheap for this query but still visible as `12.97 us` per sample.

## Allocation-Killing Rampage

| priority | change | expected effect | risk |
|---:|---|---|---|
| 1 | Historical legacy proposal for the deleted count-kernel family: replace copied central values with a zero-copy streaming distinct iterator. | Removes most of `execute` allocation calls: target `35,728` calls and `834,104 B`; should also reduce sample dispatch time by avoiding tree insertion and `Vec<u8>` copies. | Medium: must preserve distinct central movie semantics when driver entries repeat. |
| 2 | Add a direct aggregate-count output path for ungrouped `count` that writes the final `u64` result directly, bypassing `AggregateSink`, `BTreeMap`, `SmallEncodedRow`, `Vec<AggregateState>`, row sort, and generic decode. | Removes most sink/aggregate overhead: `394.8 us` sample sink finish, `27.19 us` sample aggregate span, and `5,040 B` first-execution sink allocations; keeps output at one scalar. | Low: narrow to ungrouped `count`, already semantically special-cased by factorized count. |
| 3 | Cache or precompute planner relation stats across the image before benchmarked queries, and make the benchmark optionally warm prepared plans before measurement. | Moves or removes `1,720 us` stats time and much of `1,595,918 B` plan allocation from first-query telemetry; does not change sample time because samples already hit the prepared plan. | Medium: changes benchmark accounting and cache lifecycle, not query semantics. |
| 4 | Replace planner `BTreeMap<String, ...>` structures with relation/field/index IDs and dense `Vec`/`SmallVec` storage during planning. | Targets the `plan` allocation owner: `17,350` calls and `1.596 MB`; avoids relation/field/index string clones in stats and access labels. | Medium: broad planner refactor and explain output must still recover names. |
| 5 | Stop rebuilding full candidate plans for every optimizer candidate; estimate all candidates first, build only the chosen `FreeJoinPlan`, and keep trace candidates as compact metadata. | Cuts plan allocations from `build_free_join_plan` repeated four times for aggregate queries and removes a chosen-plan clone. | Medium: optimizer trace must remain equivalent. |
| 6 | Remove `tie_breaker` string churn in `build_plan_candidate`; use enum/discriminant keys or static labels instead of `format!`, `Vec<String>`, and `join`. | Reduces small allocation calls in planning; helps the smallest size classes that account for `76.90%` of alloc calls. | Low: stable ordering must remain deterministic. |
| 7 | Rework `choose_variable_order` to use fixed-size arrays/bitsets for remaining/bound variables and select min without allocating/sorting a candidate `Vec` at every depth. | Cuts planner small-object churn in `bumbledb.query.plan.variable_order` (`223 us` prepare span) and reduces `BTreeSet` overhead. | Low to medium: ordering tie-breaks must be preserved. |
| 8 | Make direct-kernel prefix construction borrow encoded bytes where possible instead of cloning `EncodedOwned` into `SmallEncodedPrefix` for every probe. | Reduces per-probe heap/copy pressure in ordinary direct kernels and any future factorized path that needs prefixes. | Medium: lifetimes and mixed literal/input/binding ownership are tricky. |
| 9 | Keep query normalization names as borrowed/interned IDs after typecheck instead of cloning strings into `NormVar`, `NormInput`, `NormAtom`, and `NormAtomField`. | Reduces `normalize` net `6,940 B` and repeated per-sample normalization time `4.74 us`. | Medium: IR ownership and diagnostics change. |
| 10 | Expose a count-only benchmark mode for this report class so materialized one-row aggregate output is not forced during performance samples. | Would avoid generic materialized output overhead, though current output is only one row and not the dominant cost. | Low: benchmark comparability changes; keep materialized mode as default. |

## Product Waste vs Artifact

| source | classification | quantitative evidence |
|---|---|---|
| Direct factorized count probe loop | Real product sample-time cost | `305,670 us / 310,230 us = 98.53%` of sample busy time in dispatch; `37,151` probes; `54,075,120` factorized rows. |
| Direct count central-key materialization | Real product first-execution allocation, likely sample-time CPU cost too | `execute` owns `35,728` alloc calls (`67.03%`) and code uses `BTreeSet<Vec<u8>>` plus `to_vec()` at `query.rs:2735-2741`. |
| Planner stats and candidate construction | First-query artifact for steady-state samples, product cost for uncached ad hoc queries | `plan` owns `1,595,918 B` (`64.01%`) and `2.270 ms`; samples have no planner span. |
| Query image acquisition | Mostly neither for this query | `image` is `33 us` in phase timing and `12.97 us` avg in samples; no large query-image build span is present for this query. |
| Generic aggregate sink finish | Small real product waste | `sink_finish` is `394.8 us` total over samples (`13.16 us` avg) and `5,040 B` first-execution allocations. |
