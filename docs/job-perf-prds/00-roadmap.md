# JOB Performance PRD Roadmap

## Purpose

This directory is the implementation roadmap for eliminating the largest CPU and allocation waste found in the fully traced JOB benchmark run.

The work is intentionally ambitious and breaking. The codebase is unstable. Do not preserve compatibility with current internal query/image/plan data shapes if keeping them would leave tech debt. Prefer replacing bad representations early over layering adapters around them.

## Source Artifacts

| Artifact | Path |
|---|---|
| Trace analysis overview | `docs/job-trace-analysis/00-overview.md` |
| Per-query reports | `docs/job-trace-analysis/01-job_broad_cast_keyword_company.md` through `08-job_q33_linked_series_companies.md` |
| Benchmark results | `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-results.json` |
| Raw trace summary | `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-trace-summary.txt` |
| Raw trace | `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-trace.jsonl` |

## Product Constraints

- Bumbledb is an embedded, typed, schemaful, LMDB-backed Datalog database for highly normalized join-heavy application data.
- LMDB remains the only storage backend.
- SQL, JSON/document storage, nullable values, migrations, server mode, and vector search are out of scope.
- We are free to break internal storage/query/image/plan APIs now.
- We should not retain compatibility shims for old experimental query-image or plan shapes.
- Correctness beats microbenchmarks, but the current representation creates allocation cliffs large enough to hide real algorithmic behavior.

## Core Finding

The biggest performance problem is not a single slow algorithm. It is a bad encoded-value ownership model repeated in several subsystems.

| Subsystem | Current bad representation | Evidence |
|---|---|---|
| Query image segment decode | Flat fixed-width segment bytes become `Vec<Vec<u8>>`, then become typed arrays | `job_broad_cast_keyword_company`: 32.72M query-image alloc calls, 2.591 GB allocated, 984.7 MB live |
| LFTJ atom build | Atom rows become `Vec<Vec<Vec<u8>>>`, then `Vec<Vec<u8>>`, then `ColumnImage`, then `SortedTrieIndex` | q09: 16.58M `lftj_build` alloc calls, 2.649 GB allocated; q24: 920k calls, 107.7 MB |
| Atom extraction | Per retained row uses `BTreeMap<usize, Vec<u8>>` and repeated `to_vec()` | q09/q24 `scan_filter_copy` dominates cold LFTJ build time |
| Direct factorized count | Distinct central keys stored as `BTreeSet<Vec<u8>>` | `job_broad_movie_info_star`: 35,728 execute alloc calls in first execution |
| Static-empty cache | Cached empty still rebuilds normalized query and string key | q33 loses to SQLite, 91 us vs 65 us |

## Implementation Order

| Order | PRD | Why this order |
|---:|---|---|
| 01 | `01-measurement-and-allocation-contract.md` | Lock down what cold/warm/sample metrics mean before changing execution shape. |
| 02 | `02-encoded-column-builder-substrate.md` | Build the substrate that replaces `Vec<Vec<u8>>` in both query images and LFTJ. |
| 03 | `03-query-image-flat-segment-decoding.md` | Kill the largest single allocation cliff with the new builders. |
| 04 | `04-lftj-atom-column-builders.md` | Apply the same representation fix to cold LFTJ builds. |
| 05 | `05-lftj-indexed-prefix-streaming.md` | Remove the nested indexed-prefix intermediate that still allocates after PRD 04. |
| 06 | `06-relation-index-prefix-count-api.md` | Add exact prefix counts for direct kernels and static proofs. |
| 07 | `07-direct-count-kernels-before-planning.md` | Stop planning generic free joins before direct count kernels. |
| 08 | `08-structural-query-cache-keys.md` | Replace debug-string keys required by prepared/static/LFTJ cache cleanup. |
| 09 | `09-static-empty-zero-row-fast-path.md` | Make q33 and all cached static-empty queries truly near-zero work. |
| 10 | `10-prepared-normalized-query-reuse.md` | Stop rebuilding normalized owned query metadata for repeated typed queries. |
| 11 | `11-compact-direct-and-static-plans.md` | Replace full `ExecutionPlan`/`QueryPlan` clones for simple direct/static paths. |
| 12 | `12-query-image-scoped-loading.md` | Build only relation/index/column scopes required by the query. |
| 13 | `13-lftj-durable-index-trie-source.md` | Use durable sorted index images directly when they already satisfy LFTJ order. |
| 14 | `14-lftj-zero-alloc-traversal.md` | Remove steady-state key/value cloning in LFTJ traversal. |
| 15 | `15-sink-specialization.md` | Specialize global count and tiny project/dedup sinks. |
| 16 | `16-planner-compact-ids-and-lazy-candidates.md` | Remove planner String/BTreeMap/candidate overbuilding after fast paths are in place. |
| 17 | `17-benchmark-gates-and-rollout.md` | Define final benchmark gates and rollout sequencing. |

## Non-Negotiable Design Direction

- Fixed-width encoded values should be represented as `[u8; 1]`, `[u8; 8]`, or `[u8; 16]` as early as possible.
- A column of fixed-width encoded values should be one typed vector, never a vector of heap byte vectors.
- Query/image/cache keys should be structural fixed-size keys, never `Debug` strings.
- Direct count and static-empty paths should not build generic free-join plans.
- Repeated execution of the same typed query should not clone relation names, field names, variable names, value types, atoms, predicates, and outputs on every call.
- Cold query images may remain owned and `Arc`-cached. Do not store LMDB-borrowed slices in cache objects unless the cache model is rewritten to pin read transactions. The immediate plan is owned typed vectors, not borrowed cached images.

## Current Code Hotspots

| Hotspot | Code anchor | Problem |
|---|---|---|
| Query-image per-cell decode | `crates/bumbledb-lmdb/src/query_image.rs:773-781` | `chunks_exact(width).map(|chunk| chunk.to_vec()).collect::<Vec<_>>()` |
| Query-image fallback current-index build | `crates/bumbledb-lmdb/src/query_image.rs:1124-1164` | `Vec<Vec<u8>>` raw columns plus clone into `ColumnImage` |
| Whole-schema image build | `crates/bumbledb-lmdb/src/query_image.rs:948-979` | Always iterates every relation in the schema |
| Segment bytes copy | `crates/bumbledb-lmdb/src/storage.rs:1392-1397` | Copies LMDB value bytes into `Vec<u8>` before decoding |
| Late static-empty path | `crates/bumbledb-lmdb/src/query.rs:1397-1514` | Validate/normalize/encode/image/key before static-empty cache hit |
| Debug string key | `crates/bumbledb-lmdb/src/query.rs:1771-1774` | Hashes `format!("{query:?}")` then returns hex `String` |
| Direct factorized count | `crates/bumbledb-lmdb/src/query.rs:2669-2784` | Uses `BTreeSet<Vec<u8>>` and iterator `.count()` probes |
| Movie link direct count | `crates/bumbledb-lmdb/src/query.rs:2786-2886` | Four prefix iterator counts per `MovieLink` row |
| LFTJ build entry point | `crates/bumbledb-lmdb/src/query.rs:4864-4956` | Build phase allocates before execution |
| LFTJ raw columns | `crates/bumbledb-lmdb/src/query.rs:5488-5559` | `Vec<Vec<u8>>` per atom variable column |
| LFTJ indexed prefix rows | `crates/bumbledb-lmdb/src/query.rs:5599-5678` | Returns `Vec<Vec<Vec<u8>>>` |
| LFTJ atom extraction | `crates/bumbledb-lmdb/src/query.rs:5681-5729`, `5835-5884` | Per-row `BTreeMap<usize, Vec<u8>>` |
| Planner variable order | `crates/bumbledb-lmdb/src/query.rs:6038-6077` | `BTreeSet`, candidate `Vec`, string-clone tie breaker |
| Optimizer candidates | `crates/bumbledb-lmdb/src/query.rs:6464-6562` | Builds all full candidate plans, then clones chosen |
| Candidate tie breaker | `crates/bumbledb-lmdb/src/query.rs:6578-6634` | Builds `Vec<String>` and joined string |
| Aggregate sink | `crates/bumbledb-lmdb/src/query.rs:7750-7881` | `BTreeMap<SmallEncodedRow, Vec<AggregateState>>` even for global count |
| Benchmark prepare semantics | `crates/bumbledb-bench/src/main.rs:812-888` | First query execution is called prepare; samples are warm |

## Expected Final Shape

The intended post-roadmap shape is:

- Storage segments contain flat fixed-width column and index bytes.
- Query images decode segment columns directly into typed fixed-width vectors.
- LFTJ atom builds append directly into typed fixed-width builders.
- Durable sorted index images expose prefix range and prefix count primitives.
- Direct count kernels operate before generic free-join planning and use exact prefix counts.
- Static-empty cached queries return via a compact zero-row path before normalization where possible.
- Prepared queries own structural fingerprints and reusable normalized shape.
- Generic free-join planning is only used for queries that actually need it.
- Output sinks specialize global count and tiny dedup cases.

## Minimum Global Gates

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- JOB subset benchmark after each PRD that touches query execution.
- Full practical JOB benchmark after every major group: PRDs 03, 05, 09, 12, 17.
- Non-JOB preset after PRDs that alter generic planner/sink semantics.

## Benchmark Gates From Trace Baseline

| Query | Current traced pain | Gate target after relevant PRDs |
|---|---|---|
| `job_broad_cast_keyword_company` | 899.1 ms image build, 32.72M query-image alloc calls | Query-image alloc calls drop by at least 95% after PRD 03; cold image time materially lower |
| `job_q09_voice_us_actor` | 682.8 ms LFTJ build, 16.58M build alloc calls | LFTJ build alloc calls drop by at least 80% after PRD 05 |
| `job_q24_voice_keyword_actor` | 33.6 ms LFTJ build, 920k build alloc calls | LFTJ build alloc calls drop by at least 80% after PRD 05 |
| `job_broad_movie_info_star` | 35,728 execute alloc calls, 98.53% sample dispatch | Direct execute allocs collapse after PRD 06/07 |
| `job_movie_link_bridge` | 4,080 prefix probes per run and 42.7% first-run planning | Prefix-count API lowers dispatch CPU; early direct planning removes generic plan allocation |
| `job_q33_linked_series_companies` | Bumbledb 91 us vs SQLite 65 us | Bumbledb cached static-empty should beat SQLite after PRD 09 |

## Work Discipline

- Implement one PRD at a time.
- Delete obsolete representations instead of supporting both old and new paths indefinitely.
- Add tests close to the changed subsystem before broad integration tests.
- Keep benchmark output honest: do not hide cold costs by prewarming unless cold/warm metrics are reported separately.
- Update these PRDs if code reality changes during implementation.
