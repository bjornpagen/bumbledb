# job_movie_link_bridge RCA

**Verdict**

`job_movie_link_bridge` is a tiny direct count kernel wearing a full generic free-join planning costume. The first Bumbledb execution spends 42.7% of `phase_timing` in planning before discovering the direct kernel at execution dispatch. Repeated measured samples avoid the cold plan, but still spend 84.8% of sampled execution busy time inside `bumbledb.query.free_join.dispatch`, because the direct kernel scans every `MovieLink` row and performs 4 prefix-count probes per row.

**Benchmark Numbers**

| Metric | Value |
|---|---:|
| rows | 1 |
| runtime | DirectKernel |
| chosen plan | aggregate_pushdown |
| plan family | FreeJoinLftj |
| compare mode | materialized |
| Bumbledb samples | 30 |
| Bumbledb total | 27,747 us |
| Bumbledb avg | 924 us |
| Bumbledb p50 | 886 us |
| Bumbledb p95 | 1,130 us |
| Bumbledb min | 850 us |
| Bumbledb max | 1,271 us |
| SQLite avg | 22,960 us |
| SQLite p50 | 22,790 us |
| SQLite p95 | 23,675 us |
| SQLite min | 22,063 us |
| SQLite max | 29,562 us |
| SQLite / Bumbledb | 24.848x |
| Bumbledb prepare | 2,304 us |
| SQLite prepare | 23,158 us |
| Bumbledb warmup avg | 1,023 us over 2 runs |
| SQLite warmup avg | 22,622 us over 2 runs |

**Execution Ordering**

| Run kind | Runs | Trace busy total | Avg busy | Notes |
|---|---:|---:|---:|---|
| prepare | 1 | 2,260 us | 2,260 us | First materialized correctness execution |
| warmup | 2 | 1,966 us | 983 us | Plan cache is warm |
| sample | 30 | 27,044 us | 901.467 us | Reported benchmark timing |

The benchmark harness first executes Bumbledb once for correctness and prepare timing, then executes 2 warmups and 30 samples in `crates/bumbledb-bench/src/main.rs:812-883`. The benchmark result records timing and allocation telemetry from that first Bumbledb output at `crates/bumbledb-bench/src/main.rs:990-1047`.

**First-Execution Timing**

`phase_timing` in `job-results.json` and `timing-phases.tsv` is the first Bumbledb execution for this query.

| Phase | Time | % of 2,245 us total | RCA |
|---|---:|---:|---|
| execute | 1,132 us | 50.423% | Direct count kernel work after generic dispatch |
| plan | 959 us | 42.717% | Full generic free-join planning before direct execution |
| sink_finish | 35 us | 1.559% | Final aggregate row materialization |
| image | 34 us | 1.514% | Query image cache lookup, not the 899 ms cold image build seen elsewhere |
| normalize | 18 us | 0.802% | Query normalization |
| validate | 15 us | 0.668% | Input validation |
| encode | 10 us | 0.445% | No input-heavy work |
| lftj_build/hash_index/lftj_execute/hash_execute/sink_emit/decode | 0 us | 0.000% | Not used by this runtime path |

**Trace Timing**

| Span | Kind | Count | Busy total | Avg busy | % of kind execute busy |
|---|---:|---:|---:|---:|---:|
| bumbledb.query.execute | prepare | 1 | 2,260 us | 2,260 us | 100.000% |
| bumbledb.query.free_join.dispatch | prepare | 1 | 1,080 us | 1,080 us | 47.788% |
| bumbledb.query.plan | prepare | 1 | 943 us | 943 us | 41.726% |
| bumbledb.query.plan.optimize_free_join | prepare | 1 | 308 us | 308 us | 13.628% |
| bumbledb.query.plan.variable_order | prepare | 1 | 298 us | 298 us | 13.186% |
| bumbledb.query.plan.stats | prepare | 1 | 229 us | 229 us | 10.133% |
| bumbledb.query.sink.finish | prepare | 1 | 18.7 us | 18.7 us | 0.827% |
| bumbledb.query.image | prepare | 1 | 17.7 us | 17.7 us | 0.783% |
| bumbledb.query.execute | sample | 30 | 27,044 us | 901.467 us | 100.000% |
| bumbledb.query.free_join.dispatch | sample | 30 | 22,923 us | 764.100 us | 84.762% |
| bumbledb.query.sink.finish | sample | 30 | 343.22 us | 11.441 us | 1.269% |
| bumbledb.query.image | sample | 30 | 323.95 us | 10.798 us | 1.198% |
| bumbledb.query.normalize | sample | 30 | 113.74 us | 3.791 us | 0.421% |

Prepare-time waste is the 943 us planning span, especially 308 us optimizing generic free-join candidates, 298 us choosing variable order, and 229 us collecting planner stats. This is not required to run the final direct kernel.

Measured sample-time waste is different: there is no sampled plan span, but every sample still pays about 901 us total and about 764 us in direct dispatch. That is real repeated product work unless this exact query gets a stronger static direct plan or cached result strategy.

**Direct-Kernel Dispatch**

| Metric | First execution value | Meaning |
|---|---:|---|
| direct kernel target | movie_link_bridge_count | Specialized count path selected in execution dispatch |
| direct_kernel_probes | 4,080 | 4 prefix-count probes per `MovieLink` row |
| implied MovieLink rows | 1,020 | 4,080 / 4 |
| direct_kernel_rows | 149,301 | Factorized counted bindings, not output rows |
| factorized count output | 1 row | Final `COUNT` result |
| direct_kernel_predicates | 0 | No residual predicates in kernel |
| materialized_output_values | 1 | Only the final count value is materialized |

The runtime path is visible in `crates/bumbledb-lmdb/src/query.rs:2609-2630`: `execute_free_join` enters `bumbledb.query.free_join.dispatch`, tries factorized count first, then direct kernels, then hash/LFTJ fallback. The `job_movie_link_bridge` shape is handled inside `try_execute_movie_link_bridge_count` at `crates/bumbledb-lmdb/src/query.rs:2786-2886`.

The direct kernel loops `movie_link.row_count` rows and fetches `movie` and `linked_movie` from the image at `crates/bumbledb-lmdb/src/query.rs:2842-2850`. For every row it runs four `entries_with_prefix(...).count()` calls against `MovieCompanies(movie)` and `MovieInfoIdx(movie)` at `crates/bumbledb-lmdb/src/query.rs:2851-2855`, then multiplies those counts into the aggregate at `crates/bumbledb-lmdb/src/query.rs:2856-2864`. This explains why repeated samples are still about 0.9 ms: the query is tiny at output, but not zero-work internally.

**Allocation Deep Dive**

Allocation telemetry is from the first Bumbledb execution.

| Phase | Alloc calls | Realloc calls | Bytes allocated | Net bytes | Peak live | % calls | % bytes |
|---|---:|---:|---:|---:|---:|---:|---:|
| total | 18,504 | 2,129 | 1,114,047 | 36,819 | 36,819 | 100.000% | 100.000% |
| plan | 18,247 | 2,069 | 1,039,289 | 28,930 | 28,930 | 98.611% | 93.290% |
| query_image | 14 | 14 | 24,004 | 0 | 0 | 0.076% | 2.155% |
| normalize | 127 | 8 | 11,580 | 7,540 | 7,540 | 0.686% | 1.039% |
| execute | 23 | 5 | 10,951 | 7,615 | 7,615 | 0.124% | 0.983% |
| sink_finish | 33 | 9 | 5,040 | -7,444 | 0 | 0.178% | 0.452% |
| encode_inputs | 18 | 5 | 3,326 | 0 | 0 | 0.097% | 0.299% |
| validate_inputs | 13 | 4 | 1,490 | 0 | 0 | 0.070% | 0.134% |

Planning owns allocation calls and bytes: 98.611% of allocation calls and 93.290% of bytes allocated. Execution is not allocation-heavy: only 23 alloc calls and 10,951 bytes, even though execution is 50.4% of first-run time and 84.8% of sampled time. The allocation problem is overwhelmingly planner churn, not direct-kernel row scanning.

**Code-Level RCA**

| Cause | Evidence | Effect |
|---|---|---|
| Direct count is detected too late | `execute_query` validates, normalizes, encodes, gets image, checks static-empty, then calls `plan_query` at `crates/bumbledb-lmdb/src/query.rs:1397-1552`; only after planning does `execute_free_join` call `try_execute_factorized_count` at `crates/bumbledb-lmdb/src/query.rs:2609-2625` | First execution pays 943-959 us planning for a direct-count path that does not need the generic plan |
| Planner builds full generic candidate machinery | `plan_query` collects stats, chooses variable order, builds node rows, missing-index recommendations, and optimizes candidates at `crates/bumbledb-lmdb/src/query.rs:5887-6002` | Planning is 42.7% of first execution and 93.3% of allocated bytes |
| Variable ordering allocates and sorts repeatedly | `choose_variable_order` creates `BTreeSet`s, allocates a candidate `Vec` each iteration, sorts by a key containing cloned variable names at `crates/bumbledb-lmdb/src/query.rs:6038-6077` | 298 us prepare span and allocator churn for a plan that will not run |
| Planner stats allocate per relation | `PlannerStats::collect` inserts cloned relation names and cached stats into a `BTreeMap` at `crates/bumbledb-lmdb/src/query.rs:1180-1205`; relation stats build field/index maps at `crates/bumbledb-lmdb/src/planner_stats.rs:145-175` | 229 us prepare span and part of the 1.039 MB planner allocations |
| Optimizer builds four candidate plans | `optimize_free_join_plan` builds `pure_lftj`, `hash_probe`, `hybrid`, and `aggregate_pushdown` candidates, sorts them, then clones the chosen plan at `crates/bumbledb-lmdb/src/query.rs:6464-6563` | 308 us prepare span and avoidable `Vec`/`String`/plan-node churn |
| Direct runtime performs real probe work every sample | `try_execute_movie_link_bridge_count` scans all `MovieLink` rows and performs 4 prefix-counts per row at `crates/bumbledb-lmdb/src/query.rs:2842-2855` | 764 us average sampled dispatch time is product work, not just tracing overhead |
| Direct path still uses generic aggregate sink | `OutputSink::new` creates `AggregateSink` for aggregate outputs at `crates/bumbledb-lmdb/src/query.rs:7563-7568`; count ranges go through `AggregateSink` and a `BTreeMap` group store at `crates/bumbledb-lmdb/src/query.rs:7750-7785` | Small but measurable sink/finish overhead: 11.4 us average sampled sink finish and 5,040 first-run finish allocation bytes |
| Prepared-plan cache keys allocate debug strings | `prepared_plan_cache_key` hashes `format!("{query:?}")` and converts the BLAKE3 hash to hex `String` at `crates/bumbledb-lmdb/src/query.rs:1771-1775` | Repeated non-plan overhead remains in every execution before cache lookup |
| Cached plan instantiation clones the whole execution plan | `ExecutionPlan::instantiate` starts with `let mut plan = self.clone()` at `crates/bumbledb-lmdb/src/query.rs:1156-1176` | Warmup/sample executions avoid cold planning but still clone plan structures before dispatch |
| Benchmark prepare includes correctness materialization | The harness runs `execute_query` once, clones materialized output in materialized mode, then runs warmups/samples at `crates/bumbledb-bench/src/main.rs:812-883` | First-run prepare and allocation metrics mix product first-query work with benchmark correctness setup |

**Allocation-Killing Rampage**

| Priority | Kill item | Concrete change | Expected effect | Risk |
|---:|---|---|---|---|
| 1 | Move static direct count detection before generic planning | Add an early direct-count planner after normalize/encode/image and before `plan_query`; recognize `movie_link_bridge_count` and return a minimal `ExecutionPlan`/`QueryPlan` with no free-join candidate generation | Remove nearly all 959 us first-run plan time and most of the 18,247 plan alloc calls for this shape | Medium: must preserve explain/diagnostics and avoid bypassing predicates/unsupported aggregates |
| 2 | Introduce static direct plans | Represent direct count plans as compact enums keyed by relation/field IDs instead of cloning full `ExecutionPlan`, `FreeJoinPlan`, node rows, optimizer traces, and variable estimates | Cuts first-run and cached-run clone/allocation overhead; makes repeated dispatch start closer to the kernel loop | Medium: touches plan cache and explain output |
| 3 | Replace `format!("{query:?}")` cache keys | Hash normalized query fields structurally into BLAKE3 without building a debug `String` or hex `String`; use `[u8; 32]` or a compact key type in prepared/static caches | Reduces every execution’s pre-cache allocation and CPU overhead | Low/medium: key stability tests needed |
| 4 | Avoid `BTreeSet`/`Vec` churn in variable ordering | Use bitsets or small fixed arrays for remaining/bound variables; avoid per-iteration candidate `Vec` sort and cloned variable-name tie breakers | Reduces planner allocation calls and the 298 us variable-order span for all queries | Medium: ordering stability must remain deterministic |
| 5 | Avoid building unused optimizer candidates for direct-capable aggregate counts | Check direct/factorized count eligibility before `optimize_free_join_plan`; for direct matches, do not build `pure_lftj`, `hash_probe`, `hybrid`, or `aggregate_pushdown` candidates | Removes the 308 us optimize span for direct-count shapes | Medium: cost model no longer arbitrates if direct eligibility is too broad |
| 6 | Stack query bindings for direct kernels | Use a stack/small fixed binding frame for count-only direct kernels and avoid `EncodedBinding::new(query.vars.len())` where no variable materialization is needed; current binding stores `SmallVec<[Option<EncodedValue>; 8]>` at `crates/bumbledb-lmdb/src/query.rs:1114-1126` | Small direct execution allocation reduction; helpful in hot samples | Low: direct count uses empty aggregate key and can often avoid binding entirely |
| 7 | Specialize aggregate count output | For global `COUNT`, bypass `AggregateSink`/`BTreeMap` and produce one `Vec<Value>` directly from `total`; avoid `emit_count_range` through generic aggregate state | Removes most sink finish and aggregate sink overhead for this query | Low/medium: must preserve materialized output counters and explain counters |
| 8 | Cache per-movie prefix counts | Build or cache arrays/maps for `MovieCompanies(movie)` count and `MovieInfoIdx(movie)` count, then direct kernel does 2 array lookups per movie instead of 4 binary-search prefix counts per bridge row | Reduces the recurring 764 us sampled dispatch work substantially | Medium/high: memory and invalidation policy; may be JOB-specific unless generalized as relation-prefix count cache |
| 9 | Add relation-index count API | Extend `RelationIndexImage` with a lower/upper-bound count helper so `entries_with_prefix(...).count()` avoids iterator overhead while still using sorted index bytes | Reduces recurring dispatch CPU without changing semantics | Low/medium: careful bounds testing needed |
| 10 | Keep planner stats as compact relation-id maps | Replace `BTreeMap<String, Arc<...>>` in `PlannerStats` with relation-id indexed storage and avoid cloned names | Reduces planner allocation bytes for all generic plans | Medium: broad planner refactor |

**Product Waste vs Harness Artifact**

| Category | Waste | Why |
|---|---|---|
| Product first-query waste | 943-959 us generic planning and 18,247 plan alloc calls | Direct-count eligibility is discovered after generic planning, so a real first user query pays this unless a plan is already cached |
| Product repeated-query waste | 764.1 us average sampled dispatch | The direct kernel really scans 1,020 `MovieLink` rows and does 4,080 prefix-count probes per execution |
| Product small waste | 10.8 us image lookup, 3.8 us normalize, 11.4 us sink finish per sample | Cache lookup, normalization, and generic aggregate materialization remain in the hot path |
| Benchmark/first-query artifact | Prepare allocation telemetry and first prepare time | The harness intentionally records allocation from the first correctness execution and then samples warm plan-cache executions |
| Benchmark artifact, but useful signal | Materialized compare mode clones first output in `crates/bumbledb-bench/src/main.rs:815-817` | The output has only 1 row, so this is not the main issue here |

**Short Verdict**

The biggest avoidable first-run cost is generic planning for a direct kernel. The biggest recurring cost is not allocation but direct dispatch CPU: 4,080 prefix-count probes per sample. The benchmark harness amplifies the first-query planning/allocation story, but the 0.9 ms sampled runtime is real product work until the direct plan and prefix-count path are specialized further.
