# job_q01_top_production RCA

## Verdict

`job_q01_top_production` returns zero rows and is classified as `StaticEmpty`, but it still pays two different costs:

- First execution/prepare cost: a 3.27 ms static-empty proof scans 28,235 index/row entries before proving emptiness, and this proof owns almost all first-execution allocations while being missing from the named allocation phase table.
- Measured sample cost: the proof is cached, but every sample still revalidates, rebuilds a normalized query, encodes an empty input vector, looks up the query image, formats a debug-string cache key, checks an `RwLock<BTreeSet<String>>`, builds a fresh static-empty `QueryPlan`, and allocates result metadata.
- Product waste: repeated normalization/cache-key/result-plan construction on an already proven static-empty query is real product overhead; the 3.27 ms proof and 31k first-execution allocation calls are mostly benchmark first-query/cache-warming artifacts, but the proof implementation is still product code and can hurt cold executions.

## Benchmark Numbers

| Metric | Value |
|---|---:|
| Rows | 0 |
| Chosen plan | `static_empty` |
| Runtime | `StaticEmpty` |
| Plan family | `StaticEmpty` |
| Compare mode | `materialized` |
| Bumbledb samples | 30 |
| Bumbledb total | 2,504 us |
| Bumbledb avg | 83 us |
| Bumbledb min | 72 us |
| Bumbledb p50 | 80 us |
| Bumbledb p95 | 117 us |
| Bumbledb max | 129 us |
| SQLite samples | 30 |
| SQLite total | 271,935 us |
| SQLite avg | 9,064 us |
| SQLite min | 8,614 us |
| SQLite p50 | 8,905 us |
| SQLite p95 | 9,865 us |
| SQLite max | 10,942 us |
| SQLite / Bumbledb avg | 109.205x |
| Bumbledb prepare | 3,446 us |
| SQLite prepare | 9,360 us |
| Bumbledb warmup | 2 samples, 113 us avg |
| SQLite warmup | 2 samples, 8,669 us avg |

## Query Shape

Source: `crates/bumbledb-bench/src/open.rs:998-1021`.

```datalog
find count(?movie)
where
  CompanyType(id: ?company_type, kind: "production companies")
  InfoType(id: ?info_type, info: "top 250 rank")
  MovieCompanies(movie: ?movie, company_type: ?company_type)
  MovieInfoIdx(movie: ?movie, info_type: ?info_type)
  Title(id: ?movie)
```

- The query has no inputs and two static string literals.
- The zero-row proof is not a normal execution result; it is a specialized static-empty proof over literal atoms and JOB-specific intersections.
- Counters from the first Bumbledb execution: `static_empty_atoms_checked=6`, `static_empty_rows_scanned=28235`, `static_empty_cache_hits=0`, `static_empty_cache_misses=1`, and all normal execution counters are zero.

## Timing Breakdown: First Execution Phase Timing

`phase_timing` in `job-results.json` and `timing-phases.tsv` is attached to the first Bumbledb execution, which the benchmark reports as prepare.

| Phase | Time | Percent of 3,392 us total |
|---|---:|---:|
| validate | 15 us | 0.442% |
| normalize | 41 us | 1.209% |
| encode | 11 us | 0.324% |
| image | 26 us | 0.767% |
| plan | 0 us | 0.000% |
| lftj_build | 0 us | 0.000% |
| hash_index | 0 us | 0.000% |
| execute | 0 us | 0.000% |
| lftj_execute | 0 us | 0.000% |
| hash_execute | 0 us | 0.000% |
| sink_emit | 0 us | 0.000% |
| sink_finish | 0 us | 0.000% |
| decode | 0 us | 0.000% |
| Unattributed static-empty gap | 3,299 us | 97.258% |

- The named first-execution phases account for only 93 us of 3,392 us.
- The 3,299 us gap corresponds to the static-empty proof path, which has a trace span but no `phase_timing` field.
- This is prepare-time waste, not measured sample-time waste.

## Timing Breakdown: Trace Spans

Trace spans give the missing split between first proof and cached samples.

| Kind | Span | Count | Busy | Percent of kind execute busy | Avg busy |
|---|---:|---:|---:|---:|---:|
| prepare | `bumbledb.query.execute` | 1 | 3,400.0 us | 100.000% | 3,400.0 us |
| prepare | `bumbledb.query.static_empty.prove` | 1 | 3,270.0 us | 96.176% | 3,270.0 us |
| prepare | `bumbledb.query.normalize` | 1 | 29.1 us | 0.856% | 29.1 us |
| prepare | `bumbledb.query.image` | 1 | 16.1 us | 0.474% | 16.1 us |
| prepare | `bumbledb.query.encode_inputs` | 1 | 0.583 us | 0.017% | 0.583 us |
| prepare | `bumbledb.query.validate_inputs` | 1 | 0.500 us | 0.015% | 0.500 us |
| warmup | `bumbledb.query.execute` | 2 | 156.0 us | 100.000% | 78.0 us |
| warmup | `bumbledb.query.image` | 2 | 25.7 us | 16.474% | 12.85 us |
| warmup | `bumbledb.query.normalize` | 2 | 12.54 us | 8.038% | 6.27 us |
| warmup | `bumbledb.query.validate_inputs` | 2 | 0.333 us | 0.213% | 0.166 us |
| warmup | `bumbledb.query.encode_inputs` | 2 | 0.209 us | 0.134% | 0.105 us |
| sample | `bumbledb.query.execute` | 30 | 1,852.6 us | 100.000% | 61.753 us |
| sample | `bumbledb.query.image` | 30 | 261.28 us | 14.103% | 8.709 us |
| sample | `bumbledb.query.normalize` | 30 | 86.67 us | 4.678% | 2.889 us |
| sample | `bumbledb.query.encode_inputs` | 30 | 2.211 us | 0.119% | 0.074 us |
| sample | `bumbledb.query.validate_inputs` | 30 | 1.541 us | 0.083% | 0.051 us |

- Prepare is dominated by `static_empty.prove`: 3,270 us of 3,400 us trace busy.
- Samples do not show `static_empty.prove`; the cache is working after the first miss.
- Sample named spans account for 351.702 us of 1,852.6 us, leaving 1,500.898 us, or 81.017%, in wrapper/static-empty-cache work not broken into child spans.
- Per sample, image lookup is the largest visible child at 8.709 us avg, followed by normalize at 2.889 us avg.

## Static-Empty Proof And Cache Effect

| Evidence | Value |
|---|---:|
| First execution static-empty cache hits | 0 |
| First execution static-empty cache misses | 1 |
| First execution static-empty atoms checked | 6 |
| First execution static-empty rows scanned | 28,235 |
| Prepare proof span | 3,270.0 us |
| Warmup proof spans | 0 visible for this query |
| Sample proof spans | 0 visible for this query |

- `ReadTxn::execute_query` validates, normalizes, encodes inputs, acquires the query image, computes `prepared_plan_cache_key`, then checks `image.static_empty_cached` at `crates/bumbledb-lmdb/src/query.rs:1397-1459`.
- On the first miss, `static_literal_atoms_prove_empty` runs only when `normalized.inputs.is_empty()` at `crates/bumbledb-lmdb/src/query.rs:1480-1488`.
- If proof succeeds, the code inserts the static-empty cache key and returns zero rows at `crates/bumbledb-lmdb/src/query.rs:1489-1514`.
- The count-only path duplicates the same sequence at `crates/bumbledb-lmdb/src/query.rs:1610-1688`.
- The cache effect is visible: prepare has one `static_empty.prove` span; warmups and samples do not. However, the cache is reached too late to skip normalization, empty input encoding, image lookup, cache-key formatting, static plan construction, and result-column allocation.

## Allocation Deep Dive

Allocation telemetry corresponds to the first Bumbledb execution for this query.

| Metric | Value |
|---|---:|
| Allocation profiling enabled | true |
| Alloc calls | 31,492 |
| Dealloc calls | 31,435 |
| Realloc calls | 44 |
| Bytes allocated | 1,218,878 |
| Bytes deallocated | 1,214,951 |
| Net bytes | 3,927 |
| Current live bytes | 3,927 |
| Peak live bytes | 3,927 |

| Phase | Alloc calls | Bytes allocated | Net bytes | Call % | Byte % |
|---|---:|---:|---:|---:|---:|
| validate_inputs | 13 | 1,490 | 0 | 0.041% | 0.122% |
| normalize | 78 | 7,175 | 3,431 | 0.248% | 0.589% |
| encode_inputs | 18 | 3,326 | 0 | 0.057% | 0.273% |
| query_image | 14 | 24,004 | 0 | 0.044% | 1.969% |
| plan | 0 | 0 | 0 | 0.000% | 0.000% |
| lftj_build | 0 | 0 | 0 | 0.000% | 0.000% |
| hash_index | 0 | 0 | 0 | 0.000% | 0.000% |
| execute | 0 | 0 | 0 | 0.000% | 0.000% |
| sink_finish | 0 | 0 | 0 | 0.000% | 0.000% |
| Unattributed static-empty/cache gap | 31,369 | 1,182,883 | 496 | 99.609% | 97.047% |

- Named phases explain only 123 allocation calls and 35,995 allocated bytes.
- The unattributed gap owns 99.609% of allocation calls and 97.047% of bytes despite `execute=0`, `plan=0`, and zero output rows.
- This gap is located between query-image acquisition and the static-empty return, which includes `prepared_plan_cache_key`, `static_literal_atoms_prove_empty`, static-empty cache insertion, static plan construction, and result column construction.
- The largest named phase by calls is `normalize` with 78 calls; the largest named phase by bytes is `query_image` with 24,004 bytes. Neither explains the first-execution allocation storm.

## Code-Level RCA

### Benchmark Harness

- `crates/bumbledb-bench/src/main.rs:807-833` parses/typechecks once per selected query, builds `InputBindings`, runs one Bumbledb execution, and records that duration as `bumbledb_prepare`.
- `crates/bumbledb-bench/src/main.rs:846-883` then runs two warmups and 30 measured Bumbledb samples; this is why the trace order is prepare + 2 warmups + 30 samples.
- `crates/bumbledb-bench/src/main.rs:944-951` stores every sample duration in a `Vec<Duration>`, but that harness allocation is outside the Bumbledb plan allocation telemetry.
- `crates/bumbledb-bench/src/main.rs:967-976` prepares the SQLite statement on every SQLite count call, so SQLite avg is not directly comparable to a prepared-statement reuse mode.

### Late Static-Empty Cache Check

- `crates/bumbledb-lmdb/src/query.rs:1397-1454` always runs validate, normalize, encode inputs, and query-image lookup before checking the static-empty cache.
- `crates/bumbledb-lmdb/src/query.rs:1456-1478` checks the cache and returns zero rows, but only after `prepared_plan_cache_key(&normalized)` has been computed.
- `crates/bumbledb-lmdb/src/query.rs:1771-1774` computes the cache key by `format!("{query:?}")`, hashing the formatted string, converting the digest to hex, and converting that to `String`. This is avoidable `String` churn on every cached static-empty sample.
- `crates/bumbledb-lmdb/src/query_image.rs:202-215` stores static-empty keys in `Arc<RwLock<BTreeSet<String>>>`, so every sample takes a read lock and does string-tree lookup.

### Expensive Cold Proof

- `crates/bumbledb-lmdb/src/query.rs:1783-1840` runs `static_literal_atoms_prove_empty`; it scans literal-constrained atoms row by row, then tries JOB-specific proof helpers.
- `crates/bumbledb-lmdb/src/query.rs:1807-1814` linearly scans `0..relation.row_count` for each literal atom until a match is found or the atom is proven empty.
- `crates/bumbledb-lmdb/src/query.rs:1843-1961` builds the company/info intersection proof and allocates a `BTreeSet` of movie ids.
- `crates/bumbledb-lmdb/src/query.rs:1921-1936` inserts `movie.to_vec()` into `BTreeSet`, which explains why a zero-output query can still allocate heavily during proof.
- `crates/bumbledb-lmdb/src/query.rs:1964-2110` contains another JOB-specific proof path for keyword/movie/company/title emptiness. It is not the apparent winner here, but it has the same cold-proof shape.

### Normalization Churn

- `crates/bumbledb-lmdb/src/query.rs:7236-7297` rebuilds `NormalizedQuery` on every execution.
- `crates/bumbledb-lmdb/src/query.rs:7241-7258` clones variable names, input names, and value types into new `Vec`s.
- `crates/bumbledb-lmdb/src/query.rs:7259-7268` builds atom and predicate vectors from scratch.
- `crates/bumbledb-lmdb/src/query.rs:7300-7318` clones relation names, field names, field value types, and allocates per-atom field vectors.
- `crates/bumbledb-lmdb/src/query.rs:7321-7367` encodes literals during normalization, so static string literals are re-encoded each execution.
- `crates/bumbledb-lmdb/src/query.rs:7399-7424` encodes inputs into a fresh `Vec`, even when this query has zero inputs.

### Static-Empty Result Construction

- `crates/bumbledb-lmdb/src/query.rs:2261-2292` constructs a fresh `QueryPlan` for the static-empty result, including new `Vec`s and `query.output.clone()`.
- `crates/bumbledb-lmdb/src/query.rs:1474-1478` and `crates/bumbledb-lmdb/src/query.rs:1510-1514` allocate `columns: result_columns(&normalized)` and `rows: Vec::new()` for the materialized path.
- `crates/bumbledb-lmdb/src/query.rs:7469-7485` builds result columns by cloning the projected variable name. For this aggregate query, it still clones the aggregate variable name even though zero rows are returned.

### Cache Structures

- `crates/bumbledb-lmdb/src/query_image.rs:91-103` keeps `relation_by_name`, prepared plans, static-empty cache, sorted trie cache, and hash trie cache as `BTreeMap`/`BTreeSet` string-keyed structures.
- `crates/bumbledb-lmdb/src/query_image.rs:189-199` prepared plan caching also uses `String` keys; static-empty and prepared-plan caches are separate even though static-empty is effectively a prepared result.
- `crates/bumbledb-lmdb/src/query_image.rs:862-921` query-image lookup uses a `RwLock<BTreeMap<QueryImageKey, Arc<QueryImage>>>`. The sample trace shows this lookup is the largest visible cached-path child span.

## Waste Classification

| Waste source | Evidence | Classification |
|---|---|---|
| Cold static-empty proof | 3,270 us, 28,235 rows scanned, 99.609% allocation-call gap on first execution | Benchmark first-query artifact and real cold-product risk |
| Late cache check | Cache hit avoids proof but samples still average 83 us benchmark / 61.753 us trace busy | Real product waste |
| Debug-string cache key | `format!("{query:?}")` every execution | Real product waste |
| Normalization and literal encoding per execution | Sample normalize 2.889 us avg; first normalize 78 calls/7,175 bytes | Real product waste |
| Query image lookup per execution | Sample image 8.709 us avg; largest visible sample child span | Real product waste unless snapshot changes frequently |
| `BTreeSet<Vec<u8>>` proof intersection | `movie.to_vec()` inserts in cold proof | Mostly first-query artifact; real cold-product risk |
| SQLite statement prepare per sample | Harness prepares SQLite count each call | Benchmark harness artifact for SQLite side |

## Allocation-Killing Rampage

| Priority | Change | Expected effect | Risk |
|---:|---|---|---|
| 1 | Cache a prepared static-empty result before normalization where possible: key by typed query identity plus schema/tx id, or store a `StaticEmptyPlan` alongside parsed/typechecked query state. | Removes repeated normalize, literal encoding, empty input vec, debug cache key, static plan construction, and result column allocation from cached samples; targets most of the 81.017% unclassified sample busy time. | Medium: cache invalidation must include schema fingerprint and visible tx id; input-bearing queries need a separate key or must stay out. |
| 2 | Replace `prepared_plan_cache_key` debug formatting with a structural hash over stable IDs, or precompute the key once after parse/typecheck. | Removes per-execution `format!("{query:?}")`, hex string allocation, and `String` churn; reduces cached static-empty overhead without changing plan semantics. | Low to medium: must ensure hash covers literals, predicates, output, and relation/field IDs. |
| 3 | Add a no-row fast path that returns a static/borrowed zero-row count or minimal `QueryCountOutput`/`QueryOutput` without constructing full `QueryPlan` details unless explain/profile is requested. | Cuts result-column cloning, empty plan vectors, and diagnostics allocation on zero-row cached hits; directly targets sample-time overhead. | Medium: benchmark/reporting currently expects plan diagnostics, timings, allocations, and counters. |
| 4 | Move static-empty cache lookup earlier for no-input typed queries by using a cache key derivable from `TypedQuery` plus schema fingerprint before `normalize_query`. | Skips normalization and input encoding on hits; sample normalize span should approach zero. | Medium: typed-query identity must be stable and must not bypass schema validation after schema changes. |
| 5 | Use stack/static normalization for small queries: `SmallVec` for vars/inputs/atoms/predicates/find/fields and borrowed `&str`/IDs instead of cloned `String`s. | Reduces first and sample normalization allocations; for this query targets 78 calls/7,175 bytes first-run named normalization and 2.889 us/sample. | Medium: lifetime plumbing and public debug/explain ownership expectations. |
| 6 | Represent zero inputs as a shared empty `EncodedInputs` or `SmallVec`, and skip `encode_inputs` entirely when `query.inputs.is_empty()`. | Removes 18 calls/3,326 bytes on first execution and the tiny per-sample encode span. | Low: no semantic change for no-input queries. |
| 7 | Rewrite cold proof intersection to avoid `BTreeSet<Vec<u8>>`: use sorted index merge, borrowed encoded refs, or fixed-width `[u8; 8]` keys in a reusable scratch set. | Attacks the 31,369-call/1,182,883-byte unattributed cold allocation gap and 3.27 ms proof. | Medium: proof correctness over encoded component widths must be maintained. |
| 8 | Use index prefix existence checks for literal atoms instead of row-by-row scans in `static_literal_atoms_prove_empty`. | Reduces 28,235 scanned rows and cold proof time; may make cold static-empty proof cheap enough to keep. | Medium: requires robust field-to-index selection and fallback for unindexed literals. |
| 9 | Unify static-empty cache with prepared plan cache using compact hash keys and `HashMap`/`FxHashMap`-style lookup instead of `RwLock<BTreeSet<String>>`. | Reduces lock/tree/string overhead on every cached hit. | Medium: dependency and determinism choices; concurrent access behavior must remain safe. |
| 10 | Reuse encoded literals and normalized query artifacts from the benchmark harness after parse/typecheck. | Removes repeated product work in benchmark mode and reveals the true cached static-empty floor. | High for product API if exposed globally; low if limited to bench/prepared-query API. |

## Top Root Causes

| Rank | Cause | Quantitative backing |
|---:|---|---|
| 1 | Cold proof is outside named phases and scans/allocates heavily | 3,270 us proof span; 28,235 rows scanned; 31,369 unattributed alloc calls; 1,182,883 unattributed bytes |
| 2 | Static-empty cache is checked too late and keyed expensively | Samples still average 83 us benchmark / 61.753 us trace busy with zero proof spans |
| 3 | Per-execution normalization and result-plan construction churn | Sample normalize 2.889 us avg; first-run normalize 78 calls/7,175 bytes; static-empty result constructs fresh plan/columns |
