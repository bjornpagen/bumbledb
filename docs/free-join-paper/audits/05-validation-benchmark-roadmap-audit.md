# Validation and Benchmark Roadmap Audit - Investigator 5

## Sources Read

- `docs/ROSETTA_STONE.md`, especially set semantics, query semantics, public output, benchmark, golden example, and validation contracts at `docs/ROSETTA_STONE.md:36-47`, `docs/ROSETTA_STONE.md:146-171`, and `docs/ROSETTA_STONE.md:173-203`.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/02-background.tex`, especially full conjunctive query assumptions, binary/GJ baselines, bag-semantics assumptions, and bushy-to-left-deep decomposition.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/03-free-join.tex`, especially GHT, subatoms, plan partitioning, valid plans, covers, build phase, and join phase.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex`, especially `binary2fj`, factoring, COLT, vectorized execution, dynamic cover selection, and factorized output.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/05-eval.tex`, especially the Rust implementation claim, JOB/LSQB setup, COLT/vectorization ablations, factorized-output LSQB result, and bad-plan robustness experiments.
- Query implementation: `crates/bumbledb-lmdb/src/free_join.rs`, `crates/bumbledb-lmdb/src/query/*.rs`, `crates/bumbledb-lmdb/src/query_image.rs`, and query-image submodules.
- Validation assets: `crates/bumbledb-lmdb/src/query_tests*`, `crates/bumbledb-lmdb/src/query_image_tests.rs`, `crates/bumbledb-lmdb/src/storage_tests*`, `crates/bumbledb-core/src/*tests.rs`, and `crates/bumbledb-test-support/tests/*`.
- Benchmark assets: `crates/bumbledb-lmdb/src/benchmark*`, `crates/bumbledb-bench/src/main*.rs`, `crates/bumbledb-bench/src/main/datasets/*`, and `crates/bumbledb-bench/src/open/*`.
- SQLite harnesses: `crates/bumbledb-bench/src/main/sqlite.rs`, `crates/bumbledb-bench/src/main/run.rs`, `crates/bumbledb-test-support/src/sqlite.rs`, and `crates/bumbledb-test-support/tests/sqlite_comparison.rs`.
- Fuzz targets: `fuzz/fuzz_targets/fuzz_encoding_decode.rs`, `fuzz/fuzz_targets/fuzz_storage_ops.rs`, and `fuzz/Cargo.toml`.
- Scripts: `scripts/bench-quick.sh`, `scripts/bench-focused.sh`, `scripts/bench-extreme.sh`, `scripts/bench-job-10k.sh`, `scripts/bench-trace-nonjob.sh`, `scripts/summarize-trace-jsonl.sh`, and validation/check scripts.
- Current test inventory from `cargo test --workspace -- --list`: 178 named tests across workspace crates and integration tests, plus 2 fuzz targets.

## Executive Summary

Current validation is useful for the Rosetta set-engine contract, but it is not sufficient to prove a paper-compliant Free Join implementation.

The strongest current coverage is set-output behavior: `QueryResultSet::new` sorts and deduplicates output facts at `crates/bumbledb-lmdb/src/query/model.rs:166-172`, `EncodedProjectSink` deduplicates projected encoded facts with a `BTreeSet` at `crates/bumbledb-lmdb/src/query/sinks.rs:82-119`, and golden/property tests exercise duplicate witnesses, duplicate insert no-ops, absent delete no-ops, and exact projections.

The strongest benchmark improvement is in `bumbledb-bench`: it materializes Bumbledb results, materializes SQLite projected values, sorts both, and fails on exact value mismatch before timing at `crates/bumbledb-bench/src/main/run.rs:96-124`. This matches the Rosetta benchmark contract. The older `crates/bumbledb-lmdb/src/benchmark/tests.rs:36-45` still compares only counts and should not be treated as a benchmark-correctness exemplar.

The main violation is plan validation. `FreeJoinPlan::validate` only checks dense node IDs and exactly one variable per node at `crates/bumbledb-lmdb/src/free_join.rs:13-28`. The paper requires nodes containing subatoms, atom partitioning, cover existence, and cover-based execution. Current tests even assert singleton-node behavior, for example `crates/bumbledb-lmdb/src/query_tests/basic.rs:611-643`.

There is no validation for COLT laziness, vectorized Free Join execution, binary-plan conversion, conservative factorization, dynamic cover selection, factorized output, JOB/LSQB paper ablations, or robustness under good and bad plans. Existing counters and explain output cannot prove these properties because they do not expose subatoms, covers, GHT schemas, COLT force events, batch sizes, or factorization moves.

## Paper Evaluation Requirements

- The paper evaluates a Rust Free Join implementation that receives optimized binary plans, converts binary plans to Free Join plans, optimizes the Free Join plans, then runs them with COLT and vectorized execution: `tex/05-eval.tex:9-13`.
- Paper baselines are binary hash join in DuckDB, a Rust Generic Join baseline, and Kuzu for LSQB: `tex/05-eval.tex:20-29`.
- The three evaluation questions are performance versus binary join and GJ, impact of COLT/vectorization, and sensitivity to optimizer quality: `tex/05-eval.tex:30-35`.
- Paper datasets are JOB and LSQB. JOB has 113 acyclic queries with about 8 joins per query. LSQB includes cyclic and acyclic queries. Paper excludes 5 empty JOB queries and uses the first 5 LSQB queries: `tex/05-eval.tex:139-152`.
- Paper measurements are single-threaded in main memory and exclude selection and aggregation time because the goal is join-algorithm performance: `tex/05-eval.tex:154-164`.
- COLT ablation compares simple trie, simple lazy trie, and COLT with default batch size 1000: `tex/05-eval.tex:282-291`.
- Vectorization ablation compares batch sizes 1, 10, 100, and 1000: `tex/05-eval.tex:294-308`.
- Robustness evaluation compares good plans against intentionally bad cardinality estimates: `tex/05-eval.tex:311-337`.
- The optimization section requires `binary2fj`, conservative `factor`, COLT lazy trie construction, vectorized execution, dynamic cover choice, and factorized output: `tex/04-optimizations.tex:31-149`, `tex/04-optimizations.tex:163-345`, `tex/04-optimizations.tex:371-451`.
- Rosetta adapts the paper by requiring set semantics, no SQL surface, no bag output, exact SQLite `SELECT DISTINCT` value equality before benchmarks matter, and duplicate-free `QueryResultSet`: `docs/ROSETTA_STONE.md:36-47`, `docs/ROSETTA_STONE.md:146-178`.

## Current Validation Coverage

| Area | Current coverage | Key references |
| --- | --- | --- |
| Test inventory | Workspace exposes 178 named tests from `cargo test --workspace -- --list`; this is a broad but mostly unit/integration inventory, not a Free Join proof suite. | `cargo test --workspace -- --list` |
| Set-result sink | Encoded projection facts are inserted into a `BTreeSet`; final result set sorts and deduplicates. | `crates/bumbledb-lmdb/src/query/sinks.rs:82-119`, `crates/bumbledb-lmdb/src/query/model.rs:166-172` |
| Duplicate projection tests | Unit and golden tests cover duplicate projected facts and duplicate witnesses. | `crates/bumbledb-lmdb/src/query_tests/basic.rs:34-47`, `crates/bumbledb-test-support/tests/golden_examples.rs:135-162` |
| Storage set semantics | Duplicate insert no-op, absent delete no-op, exact delete, FK restrict, and guard behavior are tested. | `crates/bumbledb-lmdb/src/storage_tests/lifecycle.rs:137-185`, `crates/bumbledb-lmdb/src/storage_tests/constraints.rs:3-55` |
| Property tests | Bulk loads and insert/delete holder sequences compare against a reference set model. | `crates/bumbledb-test-support/tests/property_and_differential.rs:35-90` |
| Reference evaluator | In-memory set reference evaluator deduplicates base facts and projected outputs. | `crates/bumbledb-test-support/src/reference.rs:21-70`, `crates/bumbledb-test-support/src/reference.rs:231-241` |
| LMDB differential tests | A small LMDB-vs-reference test covers two ledger queries only. | `crates/bumbledb-lmdb/src/query_tests/differential.rs:3-48` |
| Golden families | Manifest requires ledger, sailors, joinstress, TPC-H subset, IMDb/JOB subset, Lahman subset, and LDBC subset. | `crates/bumbledb-test-support/src/golden.rs:22-32`, `crates/bumbledb-test-support/tests/golden_examples.rs:23-37` |
| SQLite correctness tests | One ledger SQLite comparison uses `SELECT DISTINCT` and exact value mapping. | `crates/bumbledb-test-support/tests/sqlite_comparison.rs:45-65` |
| Benchmark exact equality | Main benchmark runner compares exact projected SQLite values against Bumbledb values before timing. | `crates/bumbledb-bench/src/main/run.rs:96-124`, `crates/bumbledb-bench/src/main/sqlite.rs:15-73` |
| Benchmark helper tests | Helper tests catch value mismatch and duplicate projection mismatch. | `crates/bumbledb-bench/src/main_tests.rs:3-17` |
| Benchmark datasets | Synthetic ledger, sailors, joinstress, and TPC-H subsets are present; open IMDB, JOB, TPC-H, Lahman, and LDBC importers exist. | `crates/bumbledb-bench/src/main/datasets.rs:19-469`, `crates/bumbledb-bench/src/open.rs:74-107` |
| Benchmark SQL distinctness | Main benchmark SQL strings generally use `SELECT DISTINCT`. | `crates/bumbledb-bench/src/main/datasets.rs:60-90`, `crates/bumbledb-bench/src/open/job_query_list.rs:9-170` |
| Performance gates | A small set of query gates exists for latency, SQLite ratio, LFTJ next calls, and materialized values. | `crates/bumbledb-bench/src/main/result.rs:101-169`, `crates/bumbledb-bench/src/main/result.rs:171-248` |
| Fuzzing | Fuzzing covers primitive decode robustness and single-relation insert/delete count consistency. | `fuzz/fuzz_targets/fuzz_encoding_decode.rs:9-18`, `fuzz/fuzz_targets/fuzz_storage_ops.rs:9-47` |
| Explain output | Explain prints variable order, timing, allocation, cache stats, singleton `free_join_node`, and counters. | `crates/bumbledb-lmdb/src/query/explain.rs:3-140` |
| Scripts | `bench-quick.sh` runs workspace tests, clippy, fuzz check, and a benchmark run; focused/extreme/job scripts run benchmark variants. | `scripts/bench-quick.sh:1-7`, `scripts/bench-focused.sh:1-15`, `scripts/bench-job-10k.sh:1-14` |

## Gaps/Violations

| ID | Severity | Gap or violation | Evidence and impact |
| --- | --- | --- | --- |
| V-01 | P0 | Current `FreeJoinPlan` is not paper Free Join and cannot validate paper plan shapes. | `FreeJoinPlan` has only singleton `bind_vars` nodes and output at `crates/bumbledb-lmdb/src/free_join.rs:3-44`. Validation rejects multi-variable nodes at `crates/bumbledb-lmdb/src/free_join.rs:21-24`. No subatoms, covers, relation occurrences, or atom partitions can be checked. |
| V-02 | P0 | Tests codify the wrong plan shape. | `crates/bumbledb-lmdb/src/query_tests/basic.rs:635-643` asserts every cyclic triangle node binds one variable. A paper-compliant implementation needs tests that allow multi-variable cover nodes and reject only invalid cover/partition shapes. |
| V-03 | P0 | No binary-to-Free-Join conversion validation exists. | `build_free_join_plan` simply converts a variable order into one node per variable at `crates/bumbledb-lmdb/src/query/planner.rs:197-211`. There are no tests for `binary2fj`, bushy decomposition, or binary-plan equivalence. |
| V-04 | P0 | No cover/factorization correctness validation exists. | There is no representation of a cover or subatom to move. Therefore the paper's factoring invariant at `tex/04-optimizations.tex:115-149` cannot be tested. |
| V-05 | P0 | Legacy LMDB benchmark test compares counts only. | `crates/bumbledb-lmdb/src/benchmark/tests.rs:36-45` stores Bumbledb count and SQLite count and asserts count equality. This violates Rosetta if used as a benchmark correctness pattern. |
| V-06 | P1 | Current exact benchmark equality is not guarded by a full end-to-end negative test. | Helper tests catch mismatched sorted vectors at `crates/bumbledb-bench/src/main_tests.rs:3-17`, but there is no `run_dataset` fixture that intentionally produces equal-count/different-value mismatch and proves the runner fails. |
| V-07 | P1 | COLT laziness cannot be validated because COLT is absent. | `lftj_lazy_access_slices` counts durable access slices in `crates/bumbledb-lmdb/src/query/lftj_access.rs:28-42`, not COLT offset vectors, `force()`, maps, or lazy GHT children. |
| V-08 | P1 | Vectorization correctness cannot be validated because vectorized Free Join is absent. | Runtime binds one scalar variable candidate and recurses immediately at `crates/bumbledb-lmdb/src/query/lftj_runtime.rs:181-203`. There is no `iter_batch`, batch probe, survivor compaction, or batch-size setting. |
| V-09 | P1 | Current `emit_project_batch` name is misleading. | `OutputSink::emit_project_batch` pushes one binding and returns true at `crates/bumbledb-lmdb/src/query/sinks.rs:70-79`; it is not vectorized execution. |
| V-10 | P1 | Differential coverage is too narrow for a join engine proof. | Property tests use deterministic ledger query families and small generated facts at `crates/bumbledb-test-support/tests/property_and_differential.rs:35-90`. They do not generate arbitrary conjunctive queries, self-join aliases, no-useful-index atoms, plan variants, or vectorization modes. |
| V-11 | P1 | Query fuzzing does not exercise planner or executor correctness. | Fuzz targets cover primitive decoders and one single-field storage operation sequence only. No fuzz target generates typed queries and compares Bumbledb against `ReferenceDb` or SQLite. |
| V-12 | P1 | Benchmark datasets are not paper-complete. | There is no LSQB dataset or LSQB first-five query suite. JOB harness has 8 selected queries at `crates/bumbledb-bench/src/open/job_query_list.rs:3-170`, not JOB's full 113 query workload from `tex/05-eval.tex:139-140`. |
| V-13 | P1 | Paper ablations are missing. | There are no benchmark modes for simple trie vs SLT vs COLT, batch size 1/10/100/1000, factored vs unfactored Free Join, static vs dynamic cover, materialized vs factorized output, or good vs bad plan quality. |
| V-14 | P1 | Timed SQLite samples do not materialize exact values. | Exact SQLite values are materialized once, but timed SQLite samples use `sqlite_count` at `crates/bumbledb-bench/src/main/run.rs:113-149` and `crates/bumbledb-bench/src/main/sqlite.rs:3-13`. The report marks `sqlite_materialized_facts` true at `crates/bumbledb-bench/src/main/result.rs:49`, which is measurement-label drift. |
| V-15 | P1 | Metrics cannot prove Free Join, COLT, vectorization, or factorization behavior. | `PlanCounters` has LFTJ and projection counters at `crates/bumbledb-lmdb/src/query/metrics.rs:174-241`, but no cover, subatom, COLT force, batch, factorization, or factorized-output counters. |
| V-16 | P1 | `trie_intersections` appears unused. | Counter is defined at `crates/bumbledb-lmdb/src/query/metrics.rs:189-190` and printed at `crates/bumbledb-lmdb/src/query/explain.rs:98-101`, but the audited execution path does not increment it. |
| V-17 | P1 | Explain output is not an audit artifact for paper compliance. | `QueryPlan::explain` prints only singleton `free_join_node id=... bind_vars=...` at `crates/bumbledb-lmdb/src/query/explain.rs:77-84`. It omits subatoms, partitions, covers, chosen cover, GHT schemas, access source, COLT state, vector batch size, and factorization. |
| V-18 | P1 | Performance gates are too small and ad hoc for paper claims. | Gates cover a handful of queries at `crates/bumbledb-bench/src/main/result.rs:171-248`, are loose for many synthetic cases, and fail the process only with `--fail-gates` at `crates/bumbledb-bench/src/main.rs:122-130`. |
| V-19 | P1 | Regression fixtures do not include paper examples as permanent plan/output fixtures. | Golden examples cover domain subsets at `crates/bumbledb-test-support/tests/golden_examples.rs:23-385`, but not clover/sand-dollar skew, binary-derived chain plan, factorized clover plan, COLT lazy examples, vectorized batches, or robustness plans. |
| V-20 | P2 | Open-data ETL can hide null/decimal correctness drift. | Open import helpers convert missing values to `0` or empty strings at `crates/bumbledb-bench/src/open.rs:109-123` and `crates/bumbledb-bench/src/open/csv_readers.rs:42-48`; decimal/rating parsing goes through `f64` at `crates/bumbledb-bench/src/open.rs:129-139`. |
| V-21 | P2 | Scripts do not encode the full validation contract. | `bench-quick.sh` runs tests, clippy, fuzz check, and benchmark, but focused/extreme/job scripts only run benchmarks. No script runs exact plan-shape, COLT, vectorization, ablation, and gate suites because those suites do not yet exist. |

## Required New Test Suites

- Formal Free Join plan validation suite: valid clover binary plan, clover Generic Join plan, factorized clover plan, triangle plan, chain `binary2fj` plan, self-join aliases, and invalid cases for missing partition, duplicated partition, duplicate relation occurrence in one node, missing cover, unavailable probe variables, and out-of-range variables.
- Set-semantics correctness suite: duplicate base facts, duplicate witnesses, existential variables, self-joins, projection-only duplicates, empty outputs, exact delete/reinsert, and equality of all output variants as duplicate-free sets.
- Exact-result differential suite: randomly generate small schemas, set-valued facts, positive conjunctive typed queries, literals, inputs, comparisons, omitted fields, and self-joins; compare Bumbledb against `ReferenceDb` and a generated SQLite `SELECT DISTINCT` oracle.
- SQLite harness negative suite: create end-to-end benchmark fixtures where counts match but values differ, where SQLite has duplicate projected rows, and where SQL accidentally omits `DISTINCT`; the runner must fail before timing.
- COLT laziness suite: initial offset vector, suffix `iter()` without force, non-suffix `iter()` with force, `get(tuple)` force, repeated `get()` no extra force, untouched relation no build, empty relation, forced-level counts, and exact output equality against eager trie.
- Vectorization correctness suite: batch size 1 equals scalar, batch sizes 10/100/1000 equal scalar, failed probes compact survivors correctly, final partial batch works, empty batch paths work, and counters report batch inputs/survivors/probes.
- Cover and factorization suite: `binary2fj` output golden tests, conservative `factor` golden tests, unfactored/factored equivalence, dynamic cover choice under asymmetric cardinalities, cover choice under prefixes, and invalid factor moves rejected.
- Factorized-output suite: duplicate witnesses and large Cartesian products can be represented internally without expansion, final materialized `QueryResultSet` remains exact, and compression/expansion counters are correct.
- Regression fixture suite: permanent exact outputs and expected explain fragments for ledger, sailors, joinstress, TPC-H subset, IMDb/JOB subset, Lahman subset, LDBC subset, clover/sand-dollar, triangle, chain, star, and empty-result queries.
- Query validation suite: duplicate fields in one atom, repeated variables in one atom according to the chosen policy, malformed public typed IR, missing indexes requiring scan/COLT fallback, type mismatches, serial-domain mismatches, and enum-domain mismatches.
- Fuzz suite: query-IR differential fuzzing, operation-sequence storage-plus-query fuzzing, Free Join plan validator fuzzing, vectorized-vs-scalar fuzzing, COLT force/lookup fuzzing, and SQLite SQL-generation oracle fuzzing for small schemas.

## Required Benchmark Changes

- Replace or remove the count-only `crates/bumbledb-lmdb/src/benchmark` comparison path; all benchmark correctness paths must compare exact projected values, not just counts.
- Add a benchmark lint that every SQLite correctness query uses `SELECT DISTINCT` and does not use `COUNT(`, `GROUP BY`, `LEFT JOIN`, `OUTER JOIN`, or null-sensitive predicates unless a separate Rosetta-approved feature exists.
- Add a materialized-SQLite timing mode or relabel current SQLite timings as row-iteration timings after exact-value correctness, because `sqlite_count` does not materialize projected values.
- Add JOB-full and JOB-sample modes that distinguish the current 8 selected JOB queries from a future full 113-query suite.
- Add LSQB or a documented Bumbledb-compatible LSQB subset to match the paper's cyclic/acyclic benchmark coverage.
- Add paper microbenchmarks: sand-dollar/clover skew, triangle, chain, star, non-skew cyclic query, output-heavy LSQB-like query, and bad-plan robustness fixture.
- Add ablation dimensions: binary-like Free Join, factored Free Join, GJ/LFTJ baseline, simple trie, simple lazy trie, COLT, batch size 1/10/100/1000, static cover, dynamic cover, materialized output, and factorized output.
- Add performance gates only after exact correctness passes; gates should include wall-clock thresholds, SQLite ratio where meaningful, LFTJ/FJ mechanical counters, materialized values, COLT force counts, batch survivor ratios, and factorized-output expansion savings.
- Store benchmark run metadata: scale, dataset source, open-limit, query list, plan mode, batch size, cover mode, output mode, git commit if available, hardware label, and whether gates were enforced.
- Make `--fail-gates` part of the CI/validation script once gates are stable; keep local exploratory scripts able to run without failing on experimental gates.
- Fix open dataset ETL before treating open benchmarks as correctness evidence: no silent null sentinels unless documented as real domain values, and no floating-point decimal/rating parsing for exact persisted values.

## Metrics/Trace Changes

- Plan-shape metrics: `fj_nodes`, `fj_subatoms`, `fj_atom_partitions`, `fj_cover_candidates`, `fj_invalid_plan_rejections`, `fj_binary_plan_nodes`, `fj_bushy_subplans`, `fj_factor_attempts`, and `fj_factor_moves`.
- Runtime cover metrics: `fj_node_entries`, `fj_cover_choices`, `fj_cover_exact_key_counts`, `fj_cover_estimated_key_counts`, `fj_cover_switches`, `fj_probe_calls`, `fj_probe_misses`, and `fj_probe_survivors`.
- COLT metrics: `colt_nodes_created`, `colt_nodes_forced`, `colt_hash_maps_built`, `colt_offset_vectors_scanned`, `colt_offsets_total`, `colt_get_calls`, `colt_get_misses`, `colt_iter_calls`, and `colt_force_micros`.
- Vectorization metrics: `fj_batches`, `fj_batch_size_config`, `fj_batch_input_tuples`, `fj_batch_probe_calls`, `fj_batch_failed_tuples`, `fj_batch_survivor_tuples`, `fj_batch_max_size`, and `fj_batch_avg_size`.
- Output metrics: `projection_duplicate_witnesses`, `encoded_project_facts_seen`, `encoded_project_facts_inserted`, `factorized_output_nodes`, `factorized_output_edges`, `factorized_output_logical_facts`, `factorized_output_materialized_facts`, and `factorized_expansions_saved`.
- Correctness metrics: exact result fingerprint for Bumbledb, exact result fingerprint for SQLite, result columns, result facts, duplicate projected SQLite rows observed before `DISTINCT`, and correctness mode.
- Explain output must show formal plan before and after factorization, atom occurrence IDs, subatoms, partition coverage, covers, chosen cover policy, GHT/COLT schema per atom, source kind, access accelerator used, vector batch size, output mode, and per-node estimates/observed counters.
- Trace spans should be added for plan validation, binary-to-Free-Join conversion, factorization, query-image/base-image build, COLT force, dynamic cover choice, vectorized probe batch, sink deduplication, SQLite correctness materialization, and benchmark gate evaluation.
- Existing `trie_intersections` should either be incremented at the actual intersection point or removed/renamed so explain output does not print dead counters.

## Suggested Implementation Sequence

1. Stop treating the current singleton-variable LFTJ plan as paper Free Join in tests and explain. Add a compatibility label or rename internally before adding more validation around the wrong abstraction.
2. Add end-to-end benchmark negative tests for exact value mismatch and duplicate SQLite projected rows. Replace the LMDB count-only benchmark assertion with exact value equality.
3. Add formal Free Join IR and validator tests without changing execution: atom occurrence, subatom, node, cover candidates, partition validation, and invalid-plan rejection.
4. Add paper example fixtures for clover/sand-dollar, triangle, chain, and star with exact set outputs and expected valid plan shapes.
5. Add `binary2fj` and conservative `factor` as pure plan transformations with golden tests and no runtime dependency.
6. Add a richer differential query generator against `ReferenceDb`, then extend it to SQLite `SELECT DISTINCT` for small all-integer schemas.
7. Implement a scan/COLT source abstraction with laziness counters and prove it against eager/reference output before optimizing with durable access slices.
8. Implement scalar node-and-cover Free Join execution and compare unfactored, factored, and singleton-GJ plans for exact set equality.
9. Add vectorized execution with batch size 1 as the baseline, then prove batch sizes 10/100/1000 match exactly and expose batch counters.
10. Add factorized-output internals while preserving public `QueryResultSet` materialization and exact equality.
11. Expand benchmarks to paper-shaped datasets and ablations, then establish gates from correctness-first runs.
12. Update scripts so the standard validation path runs formatting, clippy, workspace tests, fuzz checks, exact benchmark correctness, plan-shape fixtures, and performance gates.

## Open Questions

- Is the product goal to implement paper-compliant Free Join, or should the Rosetta contract be revised to say the current retained executor is LFTJ/GJ with lazy durable access slices?
- What should provide the starting binary plan in a no-SQL, typed-IR product: internal deterministic atom order, an internal cost optimizer, or a test-only injected binary plan?
- Should Bumbledb import/adapt LSQB, or should a documented synthetic cyclic/acyclic substitute become the paper-compliance benchmark?
- Should timed SQLite samples materialize exact projected values every time, or is a separate exact-value correctness pass plus row-iteration timing acceptable if labeled honestly?
- What hardware and variance policy should performance gates use so they are useful but not flaky?
- Should same-atom repeated variables be rejected as invalid IR or lowered to same-fact equality predicates?
- Are open-data null sentinels acceptable for benchmarks if documented, or must optional source fields become absent facts in separate relations?
- Should `--fail-gates` become default in `bench-quick.sh` once gates stabilize?
- What should public explain guarantee: human-readable diagnostics only, or stable golden-plan text/JSON suitable for regression fixtures?
- Can factorized output remain purely internal while public `QueryOutput` always returns materialized duplicate-free result sets?
