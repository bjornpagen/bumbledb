# job_q16_character_title_us RCA

## Verdict

`job_q16_character_title_us` is correctly classified as `StaticEmpty` and returns zero rows, but the measured samples are not zero-cost. The first Bumbledb execution proves emptiness by scanning 1,320 indexed rows and then caches the result. Later measured samples avoid that proof, but still pay per-call overhead for normalization, query-image cache lookup, cache-key construction, empty-plan/result metadata, tracing, read transaction setup, and benchmark timing.

Product waste is the repeated per-query frontend work after the query is known static-empty. Benchmark/first-query artifact is the 491 us proof miss and the allocation telemetry, because allocation stats are captured from the first Bumbledb execution, not from steady-state samples.

## Benchmark Numbers

| metric | value |
|---|---:|
| rows | 0 |
| chosen plan | `static_empty` |
| runtime | `StaticEmpty` |
| plan family | `StaticEmpty` |
| compare mode | `materialized` |
| Bumbledb samples | 30 |
| Bumbledb total | 2,480 us |
| Bumbledb avg | 82 us |
| Bumbledb min / p50 / p95 / max | 76 / 77 / 96 / 175 us |
| SQLite samples | 30 |
| SQLite total | 338,427 us |
| SQLite avg | 11,280 us |
| SQLite min / p50 / p95 / max | 10,569 / 11,129 / 12,962 / 13,048 us |
| SQLite / Bumbledb avg ratio | 137.561x |
| Bumbledb prepare | 676 us |
| SQLite prepare | 11,381 us |
| Bumbledb warmup | 2 samples, 106 us avg |
| SQLite warmup | 2 samples, 11,033 us avg |

## Query Shape

| property | detail |
|---|---|
| Datalog source | `crates/bumbledb-bench/src/open.rs:1059-1073` |
| SQL source | `crates/bumbledb-bench/src/open.rs:1075-1091` |
| Static literals | `CompanyName.country_code = "[us]"`, `Keyword.keyword = "character-name-in-title"` |
| Static predicates | `Title.episode_nr >= 50`, `Title.episode_nr < 100` |
| Inputs | none |
| Result shape | `find count(?movie)` but result is zero rows because SQL/Datalog both use HAVING-style count semantics for no match |

## Prepare-Time Timing

`phase_timing` comes from the first Bumbledb execution carried into the benchmark result. It records named internal phases but does not expose the static-empty proof as its own phase.

| phase_timing phase | time us | pct of 620 us total |
|---|---:|---:|
| validate | 15 | 2.419% |
| normalize | 39 | 6.290% |
| encode | 11 | 1.774% |
| image | 26 | 4.194% |
| plan | 0 | 0.000% |
| lftj_build | 0 | 0.000% |
| hash_index | 0 | 0.000% |
| execute | 0 | 0.000% |
| sink/decode | 0 | 0.000% |
| unclassified, mostly static-empty proof and post-image overhead | 529 | 85.323% |

Trace spans show where the missing prepare time went.

| prepare trace span | count | busy us | pct of prepare execute busy | avg us |
|---|---:|---:|---:|---:|
| `bumbledb.query.execute` | 1 | 623.000 | 100.000% | 623.000 |
| `bumbledb.query.static_empty.prove` | 1 | 491.000 | 78.812% | 491.000 |
| `bumbledb.query.normalize` | 1 | 26.200 | 4.205% | 26.200 |
| `bumbledb.query.image` | 1 | 16.400 | 2.632% | 16.400 |
| `bumbledb.query.encode_inputs` | 1 | 0.584 | 0.094% | 0.584 |
| `bumbledb.query.validate_inputs` | 1 | 0.416 | 0.067% | 0.416 |
| unspanned inside execute | 1 | 88.400 | 14.190% | 88.400 |

Prepare-time waste is dominated by the static-empty proof miss. `491 us / 623 us = 78.8%` of trace prepare busy time and `491 us / 620 us = 79.2%` of `phase_timing.total_us`.

## Sample-Time Timing

Measured Bumbledb samples average 82 us wall-clock. Trace busy time inside `bumbledb.query.execute` totals 1,886 us across 30 samples, or 62.867 us per sample. The gap is benchmark/read-transaction/timing/tracing idle overhead outside the busy span.

| sample trace span | count | total busy us | pct of sample execute busy | avg busy us |
|---|---:|---:|---:|---:|
| `bumbledb.query.execute` | 30 | 1,886.000 | 100.000% | 62.867 |
| `bumbledb.query.image` | 30 | 263.970 | 13.996% | 8.799 |
| `bumbledb.query.normalize` | 30 | 112.430 | 5.961% | 3.748 |
| `bumbledb.query.encode_inputs` | 30 | 2.418 | 0.128% | 0.081 |
| `bumbledb.query.validate_inputs` | 30 | 1.749 | 0.093% | 0.058 |
| unspanned inside execute | 30 | 1,505.433 | 79.822% | 50.181 |

There is no `bumbledb.query.static_empty.prove` span in warmup or sample rows. The proof cache is effective, but the static-empty fast path is reached only after repeated frontend work.

## Execution Order And Cache Effect

| run kind | count | total busy us | avg busy us | visible effect |
|---|---:|---:|---:|---|
| prepare | 1 | 623.000 | 623.000 | proof miss, 491 us proof span |
| warmup | 2 | 151.800 | 75.900 | proof cached, still repeats normalize/image |
| sample | 30 | 1,886.000 | 62.867 | proof cached, steady-state overhead only |

Static-empty proof counters from the first Bumbledb execution:

| counter | value |
|---|---:|
| `static_empty_atoms_checked` | 7 |
| `static_empty_rows_scanned` | 1,320 |
| `static_empty_cache_hits` | 0 |
| `static_empty_cache_misses` | 1 |
| cursor seeks / rows scanned by execution | 0 / 0 |
| materialized output values | 0 |

The cache effect is visible as a one-time proof span in prepare and no proof span in the 2 warmups or 30 samples. The first warmup is 86.0 us busy and the second is 65.8 us busy, so removing the 491 us proof collapses the call from 623 us prepare to roughly the same order as measured samples.

## Static-Empty Proof Breakdown

The proof is not a physical join. It is a pre-planning shortcut in `execute_query`.

| proof step | source | behavior |
|---|---|---|
| Generic literal-atom scan | `crates/bumbledb-lmdb/src/query.rs:1783-1818` | For atoms with literal/input fields, scan relation rows until a matching row exists; if any literal atom has no matching row, return empty. |
| Specialized keyword/company/title proof | `crates/bumbledb-lmdb/src/query.rs:1964-2110` | For `Keyword` + `MovieKeyword` + `Title` + `MovieCompanies` + `CompanyName`, use leading-field indexes to find candidate movies for the keyword, apply title predicates, and prove no matching `[us]` company. |
| Cache insert | `crates/bumbledb-lmdb/src/query.rs:1489-1492` | On empty proof, insert the prepared cache key into the image-local static-empty cache. |
| Cached fast return | `crates/bumbledb-lmdb/src/query.rs:1456-1478` | On later calls, return `StaticEmpty` without proof, planning, LFTJ build, execution, sink finish, or decode. |

The key point: the cached fast return is after validation, normalization, input encoding, query-image acquisition, diagnostics, and cache-key construction.

## Allocation Deep Dive

Allocation telemetry is from the first Bumbledb execution for this query. It is therefore a prepare/proof-miss profile, not a steady-state sample profile.

| allocation metric | value |
|---|---:|
| alloc calls | 169 |
| dealloc calls | 89 |
| realloc calls | 44 |
| bytes allocated | 47,347 |
| bytes deallocated | 41,725 |
| net bytes | 5,622 |
| current live bytes | 5,622 |
| peak live bytes | 5,622 |

| phase | alloc calls | pct calls | bytes allocated | pct bytes | net bytes | peak live bytes |
|---|---:|---:|---:|---:|---:|---:|
| validate_inputs | 13 | 7.692% | 1,490 | 3.147% | 0 | 0 |
| normalize | 104 | 61.538% | 9,156 | 19.338% | 5,406 | 5,406 |
| encode_inputs | 18 | 10.651% | 3,326 | 7.025% | 0 | 0 |
| query_image | 14 | 8.284% | 24,004 | 50.698% | 0 | 0 |
| plan | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| lftj_build | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| hash_index | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| execute | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| sink_finish | 0 | 0.000% | 0 | 0.000% | 0 | 0 |
| unassigned total overhead | 20 | 11.834% | 9,371 | 19.792% | 216 | 216 |

The phase owner for allocation calls is normalization: 104 calls, 61.5% of all calls, and 5,406 of 5,622 live bytes. The phase owner for allocated bytes is query-image lookup/diagnostics: 24,004 bytes, 50.7% of allocated bytes, all transient.

The unassigned allocation bucket is real overhead outside named allocation phases. It includes work after image acquisition and before return, such as debug-format cache-key construction, static-empty cache insertion, static-empty plan metadata, and result-column construction.

## Code-Level RCA

| source | RCA |
|---|---|
| `crates/bumbledb-bench/src/main.rs:812-833` | The benchmark always performs a first Bumbledb materialized execution and a SQLite execution as prepare. This is where the static-empty proof miss and allocation telemetry are captured. |
| `crates/bumbledb-bench/src/main.rs:846-888` | Warmups and samples execute the same query again 2 + 30 times. Samples are not measuring proof work, but they still measure all pre-cache overhead inside `execute_query`. |
| `crates/bumbledb-lmdb/src/query.rs:1397-1431` | `execute_query` validates, normalizes, and encodes inputs before it can check the static-empty cache. For this no-input query, `encode_inputs` still runs and the tracing span around it still allocates. |
| `crates/bumbledb-lmdb/src/query.rs:1447-1456` | Query-image acquisition and diagnostics happen before static-empty cache lookup. Steady samples spend 14.0% of traced busy time here despite doing no execution work. |
| `crates/bumbledb-lmdb/src/query.rs:1457-1478` | The static-empty cache hit path is late. It still requires a prepared cache key, cache lookup, static plan construction, result column construction, and empty row vector return. |
| `crates/bumbledb-lmdb/src/query.rs:1480-1514` | The proof miss path runs only for no-input queries, inserts a static-empty key, and returns before planning/execution. That explains zero `plan`, `execute`, `lftj_build`, `hash_index`, and `sink_finish` timing/allocation. |
| `crates/bumbledb-lmdb/src/query.rs:1771-1774` | The cache key is `format!("{query:?}")` hashed by BLAKE3 and converted to hex `String`. This is avoidable string/debug churn on every sample. |
| `crates/bumbledb-lmdb/src/query_image.rs:95-102` | `QueryImage` stores caches in `BTreeMap<String, ...>` and `BTreeSet<String>`, forcing ordered tree and heap-string key costs for hot lookup paths. |
| `crates/bumbledb-lmdb/src/query_image.rs:202-215` | Static-empty cache lookup/insert uses `RwLock<BTreeSet<String>>`. Hits are cheap compared with proof, but still require a lock and string lookup after normalization. |
| `crates/bumbledb-lmdb/src/query_image.rs:887-904` | Query-image `get_or_build` does an `RwLock` read, `BTreeMap` lookup, and `Arc` clone on every execution. This is visible in every sample. |
| `crates/bumbledb-lmdb/src/query_image.rs:925-932` | Query-image diagnostics take another read lock to count cached images and load counters. This is recorded before the static-empty return. |
| `crates/bumbledb-lmdb/src/query.rs:7236-7297` | Normalization rebuilds `Vec` structures for vars, inputs, atoms, predicates, output, and find terms on every call. This owns 61.5% of first-execution allocation calls. |
| `crates/bumbledb-lmdb/src/query.rs:7300-7318` | Atom normalization clones relation names, field names, value types, and fields into new vectors. This is product waste for repeated execution of a typed query. |
| `crates/bumbledb-lmdb/src/query.rs:7363-7379` | Literal normalization encodes literals through temporary `Vec<u8>` before converting to fixed-width `EncodedOwned`. Static literals are re-encoded every call. |
| `crates/bumbledb-lmdb/src/query.rs:2261-2292` | `static_empty_plan` allocates metadata even when no rows can be emitted: `"static_empty".to_owned()`, empty `Vec`s, cloned output plan, counters, timings, and free-join summary. |
| `crates/bumbledb-lmdb/src/query.rs:7469-7485` | Result columns are rebuilt and variable names cloned for the empty output on every static-empty return. |

## Product Waste Vs Artifact

| category | evidence | classification |
|---|---|---|
| 491 us static proof | prepare span only, 1 cache miss, no sample proof spans | First-query artifact unless the workload constantly executes never-seen static-empty query shapes. |
| 1,320 proof rows scanned | first execution counters only | First-query artifact with current cache, but still product waste if static-empty proof is repeatedly rediscovered across snapshots/processes. |
| normalize every sample | 112.43 us across samples, 5.961% of sample busy, 61.5% of first-execution alloc calls | Real product waste. |
| query-image lookup every sample | 263.97 us across samples, 13.996% of sample busy, 50.7% of first-execution allocated bytes | Real product waste on hot static-empty path. |
| unspanned sample overhead | 1,505.433 us across samples, 79.822% of sample busy | Mixed product and benchmark overhead: cache-key construction, locks, static plan/result metadata, tracing, read transaction, and timing closure cost. |
| allocation profile | first execution has cache miss, insert, proof, and plan metadata | Mostly first-query artifact, but normalization/image/key/metadata allocations point to real steady-state risks. |

## Allocation-Killing Rampage

| priority | kill item | concrete change | expected effect | risk |
|---:|---|---|---|---|
| 1 | Move static-empty cache before full normalization | Add a typed-query/static-literal fingerprint during parse/typecheck or benchmark query setup; check `QueryImage.static_empty_queries` using that key before rebuilding `NormalizedQuery`; for no-input static-empty hits, return cached empty count/output metadata. | Removes most sample `normalize` time and its 104 first-execution allocation calls from cached static-empty hits. | Medium: key must include schema fingerprint, relation/field IDs, literals, predicates, output shape, and input absence to avoid false empty. |
| 2 | Replace `format!("{query:?}")` cache keys | Implement structural hashing over `NormalizedQuery` into a fixed `[u8; 32]` or `blake3::Hash`; store static-empty/prepared-plan caches by fixed key instead of `String`. | Cuts unassigned allocations and CPU on every execution; avoids debug string churn and hex allocation. | Low-medium: cache diagnostics and map key types need updates. |
| 3 | Add a no-row static-empty fast output path | Cache `QueryPlan`/`ResultColumn` metadata for static-empty results, or return a static count-only/materialized empty object without cloning output and variable names each call. | Removes empty-plan/result-column allocation and part of the 50 us/sample unspanned overhead. | Low: output metadata must remain correct for materialized explain output. |
| 4 | Use `HashSet`/fixed-key maps for hot caches | Replace `BTreeSet<String>` static-empty cache and `BTreeMap<String, Arc<ExecutionPlan>>` prepared-plan cache with hash maps keyed by fixed hash. | Reduces lock-held comparison cost and heap-string ordering overhead. | Low-medium: deterministic iteration is lost, but cache iteration does not appear semantically required. |
| 5 | Split diagnostics from hot path | Avoid calling `query_images.diagnostics()`, `planner_stats_diagnostics()`, and `prepared_plan_diagnostics()` on every static-empty hit unless explain/profile output is requested. | Reduces read locks and counter/size reads during samples. | Medium: current explain output expects diagnostics in `QueryPlan`. |
| 6 | Stack/static normalization for typed queries | Precompute reusable normalized structure for each `TypedQuery`, with borrowed/static names and pre-encoded static literals. Runtime should only bind/encode dynamic inputs. | Eliminates repeated `Vec`, `String`, `ValueType`, and literal-encoding churn for all repeated benchmarks, not just static-empty. | Medium-high: lifetime/API changes around `TypedQuery`, `NormalizedQuery`, and schema snapshot validity. |
| 7 | Reusable encoded inputs | For no-input queries, use a shared empty `EncodedInputs`; for input queries, reuse a small buffer across executions where the same typed query is sampled repeatedly. | Removes `encode_inputs` allocations and trivial spans; small effect here but broad cleanup. | Low: must avoid retaining references to mutable input buffers beyond execution. |
| 8 | Persist static-empty proof across image snapshots | Store static-empty proofs by schema/query key plus data-version constraints or relation/index versions. | Converts future first executions after snapshot changes from proof scans into cache hits when safe. | High: invalidation must be exact, because stale empty proofs are correctness bugs. |

## Bottom Line

The engine is beating SQLite by 137.561x because it avoids execution entirely after proving emptiness. The remaining 82 us/sample is dominated by frontend/cache/metadata overhead, not join work. The highest-return fixes are earlier static-empty caching, fixed structural cache keys, and a no-row fast output path.
