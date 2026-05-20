# RCA: job_q33_linked_series_companies

## Verdict

`job_q33_linked_series_companies` is the traced JOB case where SQLite wins because Bumbledb proves the query is statically empty, but still pays a general query-call pipeline on every measured sample. The actual proof is cheap and cached after prepare; the recurring loss is normalization, query-image/cache plumbing, key construction, diagnostics, result metadata construction, allocation accounting/tracing, and benchmark `env.read` overhead around a zero-work result.

Product waste is real for repeated zero-row static-empty queries: Bumbledb performs per-call normalization, input encoding, image lookup, string-keyed cache lookup, plan/result construction, and allocations even when no execution work exists. Benchmark/first-query artifact is also real: allocation telemetry is from the first Bumbledb execution, where the static-empty proof misses the cache once; measured sample executions do not run the proof.

## Exact Numbers

| metric | Bumbledb | SQLite | note |
|---|---:|---:|---|
| rows | 0 | 0 | `StaticEmpty` returns no rows |
| runtime | `StaticEmpty` | SQLite SQL | Bumbledb chosen plan `static_empty` |
| plan family | `StaticEmpty` | n/a | no LFTJ/hash/direct execution |
| samples | 30 | 30 | after prepare + 2 warmups |
| avg | 91 us | 65 us | SQLite wins |
| min | 81 us | 60 us | benchmark JSON |
| p50 | 85 us | 62 us | benchmark JSON |
| p95 | 134 us | 82 us | benchmark JSON |
| max | 139 us | 84 us | benchmark JSON |
| total sample time | 2,747 us | 1,951 us | benchmark JSON |
| SQLite/Bumbledb ratio | 0.714x | 1.400x Bumbledb/SQLite | Bumbledb is 26 us slower on avg |
| prepare | 200 us | 169 us | Bumbledb prepare is 31 us slower |
| warmup avg | 98 us | 92 us | 2 warmups each |

## Timing Breakdown

`phase_timing` is from the first Bumbledb execution for this query, which is the prepare execution. It shows the query never reaches plan/build/execute phases.

| phase | us | % of phase total | interpretation |
|---|---:|---:|---|
| validate | 16 | 11.348% | no inputs, but still enters validate phase |
| normalize | 51 | 36.170% | clones/builds normalized query metadata and encodes literals |
| encode | 11 | 7.801% | zero inputs, but still builds encoded input container |
| image | 26 | 18.440% | cached image lookup, lock, diagnostics |
| plan | 0 | 0.000% | bypassed by static-empty proof |
| lftj_build | 0 | 0.000% | no join plan built |
| hash_index | 0 | 0.000% | no hash index built |
| execute | 0 | 0.000% | no tuple execution |
| lftj_execute | 0 | 0.000% | no LFTJ execution |
| hash_execute | 0 | 0.000% | no hash execution |
| sink_emit | 0 | 0.000% | no output rows |
| sink_finish | 0 | 0.000% | no aggregate sink finish work recorded |
| decode | 0 | 0.000% | no materialized values decoded |
| phase total | 141 | 100.000% | JSON `phase_timing.total_us` |
| named phase subtotal | 104 | 73.759% | validate + normalize + encode + image |
| uninstrumented first-call remainder | 37 | 26.241% | cache-key construction, static-empty plan/result construction, diagnostics, counters, timers |

Trace spans separate prepare-time waste from measured sample-time waste.

| kind | count | query execute busy total | avg query execute busy | validate avg | normalize avg | encode avg | image avg | prove avg |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| prepare | 1 | 145.0 us | 145.000 us | 0.375 us | 37.000 us | 0.625 us | 16.300 us | 1.040 us |
| warmup | 2 | 152.2 us | 76.100 us | 0.063 us | 6.830 us | 0.104 us | 11.200 us | 0 us |
| sample | 30 | 2,145.7 us | 71.523 us | 0.068 us | 5.858 us | 0.085 us | 9.351 us | 0 us |

| measured sample component | total | avg | % of sample query busy |
|---|---:|---:|---:|
| `bumbledb.query.execute` | 2,145.700 us | 71.523 us | 100.000% |
| `bumbledb.query.image` | 280.530 us | 9.351 us | 13.074% |
| `bumbledb.query.normalize` | 175.750 us | 5.858 us | 8.191% |
| `bumbledb.query.encode_inputs` | 2.543 us | 0.085 us | 0.119% |
| `bumbledb.query.validate_inputs` | 2.038 us | 0.068 us | 0.095% |
| named sample subtotal | 460.861 us | 15.362 us | 21.479% |
| residual inside query span | 1,684.839 us | 56.161 us | 78.521% |

The benchmark-level sample gap is larger than the query-span gap: Bumbledb benchmark avg is 91 us vs SQLite 65 us, a 26 us loss. The traced Bumbledb query span averages 71.523 us, only 6.523 us over SQLite avg; the remaining about 19.477 us is outside the `bumbledb.query.execute` busy span or measurement/tracing envelope, primarily benchmark `env.read` transaction wrapping and timer/harness overhead.

## Static-Empty Proof

| proof counter | value | meaning |
|---|---:|---|
| `static_empty_atoms_checked` | 4 | literal/input-bearing atoms considered on the first execution |
| `static_empty_rows_scanned` | 23 | relation rows scanned before proving empty |
| `static_empty_cache_misses` | 1 | first execution had to prove and insert |
| `static_empty_cache_hits` | 0 | allocation telemetry is first execution only |
| `cursor_seeks` | 0 | no normal execution cursor work |
| `rows_scanned` | 0 | no executor row scan work |
| `direct_kernel_probes` | 0 | direct kernel not used |
| `materialized_output_values` | 0 | zero rows, zero values |

The q33 Datalog has 12 clauses, 8 variables, no inputs, and several static literals: `CompanyName(country_code: "[us]")`, two `KindType(kind: "tv series")` atoms, `LinkType(link: "sequel")`, and `Title` year predicates. Source clause order is in `crates/bumbledb-bench/src/open.rs:1169-1185`.

The first execution enters `bumbledb.query.static_empty.prove` once and spends 1.04 us there. Warmup and sample executions have no `static_empty.prove` span, so the cache effect is visible: the proof itself is removed after prepare, but the repeated call still spends about 71.5 us inside `execute_query` and about 91 us at the benchmark level.

The proof code first scans literal/input-bearing atoms in normalized clause order and returns immediately when one has no matching row. That is implemented at `crates/bumbledb-lmdb/src/query.rs:1783-1841`; the per-row literal comparison path is at `crates/bumbledb-lmdb/src/query.rs:2236-2258`. The trace and counters do not expose per-atom row counts, but 4 checked atoms and 23 scanned rows are consistent with cheap matches for the early literals followed by an empty literal atom before planning.

## Allocation Deep Dive

Allocation telemetry corresponds to the first Bumbledb execution for q33, not the cached sample executions.

| metric | value |
|---|---:|
| alloc calls | 207 |
| dealloc calls | 97 |
| realloc calls | 48 |
| bytes allocated | 57,369 |
| bytes deallocated | 49,943 |
| net bytes | 7,426 |
| current live bytes | 7,426 |
| peak live bytes | 7,426 |

| phase | alloc calls | % calls | bytes allocated | % bytes | net bytes | % net |
|---|---:|---:|---:|---:|---:|---:|
| validate_inputs | 13 | 6.280% | 1,490 | 2.597% | 0 | 0.000% |
| normalize | 142 | 68.599% | 11,498 | 20.042% | 7,210 | 97.091% |
| encode_inputs | 18 | 8.696% | 3,326 | 5.798% | 0 | 0.000% |
| query_image | 14 | 6.763% | 24,004 | 41.841% | 0 | 0.000% |
| plan | 0 | 0.000% | 0 | 0.000% | 0 | 0.000% |
| lftj_build | 0 | 0.000% | 0 | 0.000% | 0 | 0.000% |
| hash_index | 0 | 0.000% | 0 | 0.000% | 0 | 0.000% |
| execute | 0 | 0.000% | 0 | 0.000% | 0 | 0.000% |
| lftj_execute | 0 | 0.000% | 0 | 0.000% | 0 | 0.000% |
| hash_execute | 0 | 0.000% | 0 | 0.000% | 0 | 0.000% |
| sink_finish | 0 | 0.000% | 0 | 0.000% | 0 | 0.000% |
| instrumented subtotal | 187 | 90.338% | 40,318 | 70.278% | 7,210 | 97.091% |
| unassigned total remainder | 20 | 9.662% | 17,051 | 29.722% | 216 | 2.909% |
| total | 207 | 100.000% | 57,369 | 100.000% | 7,426 | 100.000% |

Normalize owns allocation calls and retained bytes despite zero execution work: 142 calls, 68.599% of all calls, and 7,210 net bytes. Query-image owns allocated bytes: 24,004 bytes, 41.841% of total allocated bytes, but returns them all before phase end. Plan/build/execute phases own no calls and no bytes.

## Code-Level RCA

`execute_query` runs a full front half before static-empty return. It validates, normalizes, encodes inputs, tries direct storage, gets the query image, computes a cache key, checks static-empty cache, and only then returns an empty result at `crates/bumbledb-lmdb/src/query.rs:1383-1514`.

Validation and input encoding are structurally unavoidable for dynamic-input queries, but q33 has no inputs. The code still enters timed/allocation phases at `crates/bumbledb-lmdb/src/query.rs:1397-1431`; `validate_inputs` loops over `query.inputs` at `crates/bumbledb-lmdb/src/query.rs:7173-7181`; `encode_inputs` still constructs and collects a `Vec` at `crates/bumbledb-lmdb/src/query.rs:7399-7423`.

Normalization is allocation-heavy because normalized metadata is rebuilt per call. `NormalizedQuery` stores `Vec`s and owned `String`s at `crates/bumbledb-lmdb/src/query.rs:74-87`; `NormVar`, `NormInput`, `NormAtom`, and `NormAtomField` retain cloned names at `crates/bumbledb-lmdb/src/query.rs:90-134`. `normalize_query` clones variable/input names, value types, atoms, predicates, find terms, and output at `crates/bumbledb-lmdb/src/query.rs:7236-7297`; `normalize_atom` clones field names, relation names, and value types at `crates/bumbledb-lmdb/src/query.rs:7300-7319`.

Literal encoding happens during normalization even though the query has no runtime inputs. `normalize_term`, `normalize_predicate`, `encode_literal`, and `encode_owned_value` convert literals into owned encoded forms at `crates/bumbledb-lmdb/src/query.rs:7321-7379`. For q33, this includes string literals and year predicate constants on every call.

The query image is cached, but lookup still costs time and allocation-accounting noise. `QueryImageCache::get_or_build` computes a cache key, reads `last_committed_tx_id`, takes an `RwLock` read lock over a `BTreeMap`, clones an `Arc`, and updates atomics at `crates/bumbledb-lmdb/src/query_image.rs:887-905`. Diagnostics take another read lock at `crates/bumbledb-lmdb/src/query_image.rs:925-932`.

The static-empty cache key is expensive for a zero-row fast path. `prepared_plan_cache_key` formats the entire `NormalizedQuery` with `format!("{query:?}")`, hashes those bytes, converts the digest to hex, and returns a new `String` at `crates/bumbledb-lmdb/src/query.rs:1771-1774`. This happens before static-empty cache lookup in `execute_query` at `crates/bumbledb-lmdb/src/query.rs:1456-1459`, so cached static-empty samples still pay the debug-format/query-image/hash/string path.

The static-empty cache is also string and tree based. `QueryImage` stores `static_empty_queries: Arc<RwLock<BTreeSet<String>>>` at `crates/bumbledb-lmdb/src/query_image.rs:93-102`; cache check and insert take locks and use string keys at `crates/bumbledb-lmdb/src/query_image.rs:202-215`.

Static-empty result construction allocates diagnostics and metadata rather than reusing a zero-row prepared result. `static_empty_plan` creates many empty `Vec`s, allocates `"static_empty".to_owned()`, and clones `query.output` at `crates/bumbledb-lmdb/src/query.rs:2261-2292`. Return paths then build `result_columns(&normalized)` and `Vec::new()` rows at `crates/bumbledb-lmdb/src/query.rs:1474-1478` and `crates/bumbledb-lmdb/src/query.rs:1510-1514`; `result_columns` clones output variable names into a new `Vec<ResultColumn>` at `crates/bumbledb-lmdb/src/query.rs:7469-7485`.

The benchmark harness runs exactly the expected ordering: parse/typecheck once, Bumbledb prepare once, SQLite prepare once, 2 warmups each, then 30 samples each at `crates/bumbledb-bench/src/main.rs:806-888`. SQLite is not getting a free preprepared statement: `sqlite_count` calls `conn.prepare(sql)` every time at `crates/bumbledb-bench/src/main.rs:967-976`. That makes the SQLite win meaningful: even with per-call SQL prepare, SQLite averages 65 us.

## Kill List

| priority | change | expected effect | risk |
|---:|---|---|---|
| 1 | Cache a prepared static-empty result keyed by typed query identity before normalization, containing plan family/runtime, result columns, and immutable zero-row output metadata. | Removes most recurring 91 us sample cost: skip normalize, encode, query-image diagnostics, cache-key debug formatting, static-empty plan construction, and result column cloning on hits. | Medium: cache invalidation must include schema fingerprint and storage tx id; diagnostics must remain correct. |
| 2 | Replace `prepared_plan_cache_key(&NormalizedQuery)` debug-format + BLAKE3 + hex `String` with a stable structural fingerprint computed during parse/typecheck or normalization, preferably fixed-size `[u8; 32]` or `u64/u128`. | Removes hidden residual cost in cached samples and avoids `String` allocation/churn on every query call. | Medium: key stability/collision discipline; must distinguish literals, predicates, output, and schema-dependent relation/field IDs. |
| 3 | Add a no-input fast path that bypasses `validate_inputs` and `encode_inputs`, and stores empty encoded inputs as a static singleton. | Kills 31 first-call allocation calls and all recurring validate/encode span overhead for no-input JOB queries. | Low: only applies when `query.inputs.is_empty()`; error behavior unchanged for queries with inputs. |
| 4 | Make normalized query metadata reusable or borrowed: store relation/field names as IDs or `Arc<str>`/borrowed descriptors, pre-size vectors, and avoid cloning `String`/`ValueType` per execution. | Attacks the largest allocation caller: normalize currently owns 68.599% of allocation calls and 97.091% of net bytes. | Medium: touches planner/executor assumptions that use owned diagnostic names. |
| 5 | Replace `static_empty_queries: RwLock<BTreeSet<String>>` with a hash/fingerprint set or per-prepared-plan state, and combine lookup with prepared-plan cache. | Removes string/tree/lock overhead after the first proof; makes static-empty cache hits O(1) without allocation. | Medium: concurrent cache correctness and diagnostics need care. |
| 6 | Reuse a static empty row vector/result object for static-empty materialized queries, and make `static_empty_plan` use static strings/slices for empty vectors where possible. | Removes plan/result allocation noise that currently appears in uninstrumented allocation/time remainder. | Low to medium: public result ownership may require `Arc` or `Cow`-style representation. |
| 7 | Avoid query-image lookup for known static-empty prepared queries when the cache key includes the read snapshot tx id or is invalidated on write. | Removes the recurring 9.351 us sample `query.image` span, 13.074% of query busy time. | Medium-high: static emptiness depends on data snapshot, so invalidation must be exact. |
| 8 | Reduce diagnostics work on fast-path hits, especially `query_image_cache`, planner stats, and prepared-plan diagnostics locks. | Cuts residual cached-hit time and lock traffic. | Low if diagnostics can be sampled/lazy; medium if tests assert exact counters. |

## Product vs Harness

Real product waste: the cached static-empty path still averages 71.523 us inside `bumbledb.query.execute`, with 15.362 us in named recurring phases and 56.161 us residual query-span work, despite zero cursor seeks, zero executor rows scanned, zero direct-kernel probes, and zero materialized output values.

Real benchmark/harness artifact: first-execution allocation telemetry includes a static-empty cache miss and proof insert; cached samples do not run `static_empty.prove`. The benchmark-level 91 us includes about 19.477 us beyond the traced query span, plausibly `env.read` transaction wrapping, timer overhead, and trace/harness envelope.

The SQLite win is still valid for this benchmark. SQLite prepares and executes its SQL in each sample and averages 65 us, while Bumbledb has already cached the static-empty proof and still averages 91 us at the benchmark level.
