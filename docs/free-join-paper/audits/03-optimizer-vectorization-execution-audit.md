# Optimizer, Vectorization, and Execution Audit

## Sources Read

- `docs/ROSETTA_STONE.md:1-236`, especially the v4 product contract, set semantics, query execution contract, duplicate-free result-set output, and benchmark correctness contract.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/02-background.tex:65-100`, binary and bushy plan decomposition into left-deep plans.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/03-free-join.tex:177-608`, Free Join plan/subatom/covers/GHT execution and dynamic-cover motivation.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/04-optimizations.tex:31-479`, binary-plan conversion, conservative factoring, COLT, vectorized execution, dynamic covers, and factorized output discussion.
- `docs/free-join-paper/arXiv-2301.10841v2/tex/05-eval.tex:9-337`, implementation claims, COLT/vectorization ablations, materialization limitations, and factorized output evaluation.
- `crates/bumbledb-lmdb/src/free_join.rs`, `query/planner.rs`, `query/planner_scoring.rs`, `query/lftj_runtime.rs`, `query/lftj_iter.rs`, `query/lftj_leapfrog.rs`, `query/lftj_access.rs`, `query/lftj_prefix.rs`, `query/hash.rs`, `query/metrics.rs`, `query/explain.rs`, and `query/sinks.rs`.
- `crates/bumbledb-lmdb/src/planner_stats.rs`, `query_image/cache.rs`, `query_image/builder.rs`, `query_image/access.rs`, `query_image/scope.rs`, and `query_image/types.rs`.
- `crates/bumbledb-lmdb/src/benchmark*.rs`, `crates/bumbledb-bench/src/main/*.rs`, `crates/bumbledb-bench/src/main/datasets/*.rs`, `crates/bumbledb-bench/src/open/job_query_*.rs`, and `scripts/bench-*.sh`.

## Executive Summary

Current Bumbledb does not implement the Free Join optimizer/execution pipeline described in the paper. It implements a typed-query LFTJ executor with a planner-chosen variable order, durable sorted index images, and encoded projection deduplication. That is useful and correct for the current result-set contract, but it is not the paper's Free Join plan model.

The largest gap is structural: `FreeJoinPlan` has only ordered nodes that each bind exactly one variable, while the paper's Free Join plan is a list of nodes containing relation subatoms, with per-node covers, a partitioning of every atom, and the ability to sit anywhere between binary join and Generic Join. Because the plan IR lacks subatoms and covers, the implementation cannot express binary-plan-to-Free-Join conversion, conservative factoring, bushy decomposition, dynamic cover selection, vectorized batched probing, or factorized output.

The required fix is not incremental tuning. It is a breaking refactor of the planner IR, query-image/cache boundary, runtime iterator API, execution loop, and explain/counters. The current variable-order LFTJ can be retained as a baseline or as the degenerate Generic Join mode, but it cannot be the primary representation if the goal is paper-faithful Free Join.

## Paper Requirements

- Binary optimizer input: The paper starts from an optimized binary plan, decomposes a bushy plan into left-deep plans, converts each left-deep plan to Free Join, then optimizes the Free Join plan (`tex/04-optimizations.tex:31-40`; bushy decomposition in `tex/02-background.tex:81-91`).
- Binary-to-Free-Join conversion: `binary2fj` creates Free Join nodes where each relation is split into subatoms based on variables available at that node (`tex/04-optimizations.tex:49-71`).
- Valid Free Join plan: A plan node is a list of subatoms; across nodes, subatoms partition each query atom; each node must have no duplicate relation and at least one cover containing all newly bound variables (`tex/03-free-join.tex:193-220` and `tex/03-free-join.tex:288-305`).
- Factoring: Optimization moves eligible lookup subatoms from node `i` to node `i - 1`, only when variables are already available, the previous node does not already contain that relation, and lookup order is preserved conservatively (`tex/04-optimizations.tex:115-149`).
- GHT/COLT access model: Build phase constructs generalized hash tries from the Free Join plan schema; COLT lazily materializes hash-map levels from column vectors only when lookup/iteration requires it (`tex/03-free-join.tex:383-438`; `tex/04-optimizations.tex:163-345`).
- Dynamic cover selection: For each node, find all covers and at runtime iterate over the cover with the fewest keys; with COLT vectors, vector length is an estimate when exact key count would force materialization (`tex/04-optimizations.tex:435-451`).
- Vectorized execution: Replace scalar `iter` with `iter_batch(batch_size)`, run probes for a batch before recursing, remove failed tuples from the batch, then recurse per surviving tuple (`tex/04-optimizations.tex:371-417`).
- Factorized output: The paper explicitly connects trie/factorized representation to output compression and says factorized output was implemented for LSQB output-heavy cases (`tex/04-optimizations.tex:426-433`; `tex/05-eval.tex:254-260`).
- Correctness constraints from Bumbledb: Bumbledb v4 is set-semantic, duplicate-free at projection, and query output is `QueryOutput { result: QueryResultSet }` (`docs/ROSETTA_STONE.md:36-46`, `docs/ROSETTA_STONE.md:146-171`). Benchmarks must compare exact values to SQLite `SELECT DISTINCT`, not only counts (`docs/ROSETTA_STONE.md:173-178`).

## Current Implementation

- Plan IR is LFTJ-shaped, not Free Join-shaped. `FreeJoinPlan` contains only `nodes` and `output`; `PlanNode` contains only `id` and `bind_vars` (`crates/bumbledb-lmdb/src/free_join.rs:3-38`). Validation rejects any node that does not bind exactly one variable (`crates/bumbledb-lmdb/src/free_join.rs:13-27`).
- Planning chooses a variable order directly from normalized atoms and predicates, not from a binary plan. `plan_query` calls `choose_variable_order`, attaches predicate depths, and then `build_free_join_plan` (`crates/bumbledb-lmdb/src/query/planner.rs:3-58`).
- `build_free_join_plan` creates one node per variable in the selected variable order and does not include atoms, subatoms, covers, relation order, access choice, or factoring state (`crates/bumbledb-lmdb/src/query/planner.rs:197-211`).
- The scoring model estimates variable order with relation/index stats and simple constraint counts. The ordering key prioritizes physical field position, candidate estimate, static/bound constraints, relation count, degree, and variable name (`crates/bumbledb-lmdb/src/query/planner_scoring.rs:3-26`).
- Statistics are cheap and mostly one-dimensional. Field stats sample at most 4096 leading rows (`crates/bumbledb-lmdb/src/planner_stats.rs:10`, `crates/bumbledb-lmdb/src/planner_stats.rs:172-194`). Index stats are derived from field distinct estimates and relation fact count, not exact multi-column prefix cardinalities or skew histograms (`crates/bumbledb-lmdb/src/planner_stats.rs:197-263`).
- Query images are acquired before planning and scoped from normalized atoms, not from the chosen Free Join plan (`crates/bumbledb-lmdb/src/query/api.rs:53-71`; `crates/bumbledb-lmdb/src/query/hash.rs:3-51`). This makes access-image selection plan-insensitive.
- Query image building scans durable access paths and copies every included index key into memory (`crates/bumbledb-lmdb/src/query_image/builder.rs:164-231`). This is not COLT lazy materialization from base columns; it is eager snapshot imaging of selected durable sorted indexes.
- Runtime extracts a single variable order from `FreeJoinPlan` and errors if any node binds more than one variable (`crates/bumbledb-lmdb/src/query/lftj_runtime.rs:97-107`).
- Runtime builds one `LftjAtomPlan` per normalized atom. Each atom's variables are simply all atom variables that appear in plan order (`crates/bumbledb-lmdb/src/query/lftj_access.rs:16-48`, `crates/bumbledb-lmdb/src/query/lftj_access.rs:342-348`). There is no per-node subatom partition.
- `LazyAccessSlice` chooses the smallest durable index slice that can expose the atom variables in variable-order sequence (`crates/bumbledb-lmdb/src/query/lftj_access.rs:50-89`). Filtering is by static/literal fields not already covered by the prefix (`crates/bumbledb-lmdb/src/query/lftj_access.rs:217-266`).
- `LazyAccessIter` groups sorted index entries by field value and supports scalar `open`, `up`, `key`, `next`, and `seek` (`crates/bumbledb-lmdb/src/query/lftj_iter.rs:93-292`). There is no `iter_batch`, batched seek, batched lookup, or vectorized probe API.
- Leapfrog execution intersects all atom participants for the current variable. `LeapfrogState` sorts iterators by current key, not by cover cardinality or trie key count (`crates/bumbledb-lmdb/src/query/lftj_leapfrog.rs:54-92`).
- Recursive execution binds exactly one variable per depth, evaluates ready comparisons, and recurses immediately for each scalar candidate (`crates/bumbledb-lmdb/src/query/lftj_runtime.rs:133-210`). There is no batch before recursion.
- Projection materialization is encoded and duplicate-free but fully materialized. `EncodedProjectSink` inserts projected encoded facts into a `BTreeSet` and decodes the final set during `finish` (`crates/bumbledb-lmdb/src/query/sinks.rs:82-160`). This satisfies the result-set contract but is not factorized output.
- Explain output reports only variable order, phase timings, allocation summaries, query-image/planner cache diagnostics, one-variable `free_join_node` records, and coarse counters (`crates/bumbledb-lmdb/src/query/explain.rs:3-140`). It does not show atoms, subatoms, covers, access paths, factoring moves, batch size, or dynamic choices.
- The benchmark harness in `bumbledb-bench` does compare exact Bumbledb result values against SQLite result values before timing (`crates/bumbledb-bench/src/main/run.rs:96-124`), which matches Rosetta. The older `bumbledb-lmdb` benchmark test compares only counts (`crates/bumbledb-lmdb/src/benchmark/tests.rs:36-47`).
- Benchmark reporting contains optimizer visibility gaps: `query_image_scope` reports `full_schema` for any non-empty query-image usage even though the engine now builds scoped images (`crates/bumbledb-bench/src/main/result.rs:73-82`).

## Violations

- P0: There is no binary-plan-to-Free-Join conversion. The current planner never accepts or constructs binary join trees, left-deep relation sequences, or decomposed bushy subplans. It goes straight from typed IR to variable order (`planner.rs:3-58`, `planner.rs:197-211`).
- P0: The plan IR cannot express Free Join. A node cannot contain subatoms, covers, relation IDs, or multiple newly bound variables because validation requires exactly one bound variable (`free_join.rs:13-27`).
- P0: Factoring is impossible in the current representation. There is no subatom to move between nodes, no same-relation-in-previous-node check, no lookup-order preservation, and no factoring diagnostics.
- P0: Runtime is LFTJ, not Free Join. It opens all relation participants for one variable and runs scalar leapfrog intersections (`lftj_runtime.rs:164-203`), rather than executing plan nodes with one cover iterator and probe subatoms.
- P0: Dynamic cover selection is missing. The code has no concept of `cover(phi)`, no per-node cover set, and no runtime choice by fewest keys. Sorting by current key in `LeapfrogState` is not cover selection (`lftj_leapfrog.rs:68-92`).
- P1: Vectorized execution is missing. There is no `iter_batch(batch_size)` in `LinearIter` or `TrieIter` (`sorted_trie.rs:43-61`), and recursion happens immediately per scalar candidate (`lftj_runtime.rs:181-203`). The `emit_project_batch` name is misleading because it pushes one binding at a time (`sinks.rs:70-79`).
- P1: Batch probes are missing. `LazyAccessIter` supports scalar `seek` only (`lftj_iter.rs:226-253`); there is no API to lookup or probe a batch of tuple keys into one relation/trie.
- P1: COLT is not implemented. Current access is over eager in-memory durable sorted index images (`query_image/builder.rs:164-231`) with lazy grouping over sorted bytes (`lftj_iter.rs:93-292`). It lacks column-offset leaves, lazy hash-map forcing, per-level materialization counters, and build-time elimination for untouched levels.
- P1: Factorized output is missing. Projection enumerates complete bindings, inserts projected facts into a `BTreeSet`, and decodes materialized result facts (`sinks.rs:96-160`). Duplicate witnesses are deduplicated after enumeration instead of avoiding expansion through factorized output.
- P1: Bushy decomposition and materialized intermediate plans are missing. The paper requires decomposing bushy binary plans into left-deep subplans with materialized intermediates (`tex/02-background.tex:81-91`), but current planning has no binary tree or materialized intermediate relation path.
- P1: Plan/cache ordering is backwards for a Free Join optimizer. Query images are scoped and built before plan construction (`api.rs:53-71`), so the optimizer cannot first choose covers/subatoms/access schema and then request exactly the needed columns/indexes.
- P2: Statistics are insufficient for Free Join choices. The planner has no exact prefix cardinality by bound prefix, no per-node cover key counts, no skew/fanout histograms, no dynamic feedback, and no cost model for build versus probe versus materialization.
- P2: Existing counters are incomplete and partially misleading. `PlanCounters::trie_intersections` exists (`metrics.rs:189-190`) and is printed (`explain.rs:98-101`) but the audited execution path does not increment it. There are no counters for covers considered/chosen, factoring moves, batch sizes, batch survivors, COLT force events, or factorized-output compression.
- P2: Explain output cannot audit paper-required behavior. It omits relation order, subatoms, covers, access paths, per-node stats, chosen cover at runtime, vectorization settings, and output mode (`explain.rs:77-139`).
- P2: Benchmark coverage is not paper-shaped. The harness has correctness checks and some joinstress/JOB queries, but no ablation switches for converted binary plan versus factored plan, static cover versus dynamic cover, scalar versus vectorized, materialized versus factorized output, or good versus bad binary plans.

## Required Breaking Changes

- P0: Replace `FreeJoinPlan` with a real Free Join physical IR. It needs `nodes: Vec<FreeJoinNode>`, each node needs `subatoms: Vec<PlanSubatom>`, `available_vars`, `new_vars`, `covers`, optional chosen/default cover, probe order, and access schema. `PlanSubatom` needs atom ID, relation instance ID, variable list, field mapping, and access candidate IDs.
- P0: Remove the invariant that every node binds exactly one variable. Keep a validation rule matching the paper: subatoms partition each atom, no duplicate relation in a node, and at least one cover contains `vs(phi) - avs(phi)`.
- P0: Add a binary plan IR and conversion boundary. The optimizer should produce or accept `BinaryJoinPlan` with leaf atoms and join nodes, decompose bushy plans into left-deep segments, and convert each segment with the paper's `binary2fj` algorithm.
- P0: Implement conservative factoring as a physical-plan rewrite over subatoms. It must preserve lookup order and record each attempted/successful move for explain and tests.
- P0: Rewrite execution around Free Join nodes. For each node, pick a cover, iterate cover tuples, probe other subatoms using available/new variables, replace participating tries/subtries, then recurse to the next node.
- P1: Split the current LFTJ executor into a baseline engine behind a trait or mode. It can remain as `GenericJoin/LFTJ` for tests and fallback, but `execute_free_join` should dispatch to the new node/subatom executor for real Free Join plans.
- P1: Redesign the query-image boundary. Either plan before building the image and request plan-required columns/indexes, or support lazy per-relation/per-index image acquisition during plan finalization/execution. The current pre-plan image scope will cause over- or under-loading once covers and access schemas are plan-dependent.
- P1: Add COLT-like relation access. For Bumbledb's LMDB/set architecture this likely means column-backed offset vectors plus lazily forced per-level maps or sorted-group indexes. Durable sorted access images can be one implementation, but the API must expose `key_count_estimate`, `iter`, `iter_batch`, `lookup`, `lookup_batch`, and `force` diagnostics.
- P1: Add dynamic cover selection. At runtime each node should evaluate all covers and choose by exact key count where available or estimate where forcing would be too expensive.
- P1: Add vectorized node execution. Implement batch iteration of cover tuples, batch projection of keys for each probe subatom, batch lookup/seek, survivor compaction, and controlled recursion after probing.
- P1: Add a factorized-output path behind the existing `QueryResultSet` contract. Internal sinks may count or compact factorized output, but final public output must still produce duplicate-free `QueryResultSet` when materialization is requested.
- P2: Replace planner statistics with prefix-aware and skew-aware stats. At minimum store relation cardinality, per-access exact entry count, per-prefix distinct counts for declared access prefixes, sampled or exact fanout histograms, and output/materialization estimates.
- P2: Make explain output a first-class physical-plan audit. If a future implementation claims Free Join, explain must prove it by showing subatoms, covers, selected cover policy, access schemas, and factoring/vectorization/factorized-output settings.

## Implementation Sequence

1. Freeze current behavior with regression tests for typed IR set semantics, duplicate projection, range filters, query-image cache, planner-stats cache, and current LFTJ fallback.
2. Introduce new Free Join plan structs beside the existing one and update validation to implement the paper's valid-plan rules.
3. Build a binary plan IR and a deterministic left-deep conversion path from current typed query atom order, then add a separate hook for a future cost-based binary optimizer.
4. Implement `binary2fj` and unit-test it on the clover and chain examples from `tex/04-optimizations.tex:73-80`.
5. Implement conservative factoring and unit-test that it produces the optimized clover plan from `[[R(x,a),S(x)],[S(b),T(x)],[T(c)]]` to `[[R(x,a),S(x),T(x)],[S(b)],[T(c)]]`.
6. Refactor query-image planning so the physical plan chooses required relation fields and access schemas before the image/cache build, or introduce lazy plan-driven image slices.
7. Add a node/subatom runtime in scalar form first, using the current sorted durable access slices as the initial `GhtAccess` backend.
8. Add dynamic cover selection to the scalar runtime and expose chosen covers in counters/explain.
9. Add batch APIs and implement vectorized execution with batch size configuration, defaulting to a conservative value and allowing batch size 1 as a scalar baseline.
10. Add a COLT-compatible backend or adapt durable sorted images behind a COLT-like API with lazy force/key-count instrumentation.
11. Add factorized-output/counting sinks for projection-heavy queries while preserving final `QueryResultSet` materialization semantics.
12. Replace benchmark gates and reports with plan-aware ablations, then use JOB/open datasets and joinstress to validate optimizer robustness.

## New Counters/Explain Output

- Plan counters: `fj_nodes`, `fj_subatoms`, `fj_covers_total`, `fj_factoring_attempts`, `fj_factoring_moves`, `fj_binary_plan_nodes`, `fj_bushy_subplans`, `fj_materialized_intermediates`.
- Runtime counters: `fj_node_entries`, `fj_cover_choices`, `fj_cover_key_count_estimates`, `fj_cover_exact_key_counts`, `fj_cover_switches`, `fj_probe_calls`, `fj_probe_failures`, `fj_probe_survivors`.
- Vectorization counters: `fj_batches`, `fj_batch_input_tuples`, `fj_batch_survivor_tuples`, `fj_batch_probe_calls`, `fj_batch_probe_failures`, `fj_batch_max_size`, `fj_batch_avg_size`.
- COLT/access counters: `colt_nodes_forced`, `colt_offsets_scanned`, `colt_hash_maps_built`, `colt_force_micros`, `colt_lookup_calls`, `colt_lookup_misses`, `durable_access_bytes_loaded`, `access_key_count_estimate_calls`.
- Output counters: `factorized_output_nodes`, `factorized_output_edges`, `factorized_output_logical_facts`, `factorized_output_materialized_facts`, `factorized_output_expansions_saved`, `projection_duplicate_witnesses`.
- Explain additions: binary plan source, decomposed left-deep subplans, converted Free Join plan before factoring, factoring diff, final plan nodes with subatoms and covers, access schema per relation, dynamic cover policy, vector batch size, output mode, per-node estimated and observed cardinalities.

## New Tests/Benchmarks

- Unit tests for valid and invalid Free Join plans: duplicate relation in node, missing cover, non-partitioned atom, repeated subatom variable, and multi-variable cover node.
- Golden conversion tests for `binary2fj`: clover, chain, triangle, star, self-join aliasing, static/literal fields, and projection-only existential variables.
- Factoring tests: clover improvement, no movement when variables unavailable, no movement across same relation in previous node, and conservative stop when an earlier subatom in the same node cannot move.
- Bushy decomposition tests: `(R join S) join (T join U)` produces two left-deep subplans and explicit materialization boundary.
- Dynamic cover tests: triangle with asymmetric cardinalities chooses the smaller cover at each depth and changes choice under different prefixes.
- Vectorization tests: batch size 1 matches scalar, batch size >1 matches scalar, probe failure compaction preserves results, and counters report batch survivors.
- COLT tests: untouched left-most relation does not force hash maps, lookup forces only required levels, key-count estimate avoids force where required, and force counters match expected levels.
- Factorized-output tests: duplicate witnesses do not multiply projected output, count-only output does not expand Cartesian products, final materialized `QueryResultSet` is identical to scalar materialization.
- Differential tests against the existing reference evaluator and SQLite `SELECT DISTINCT` for clover skew, triangle, chain, star, JOB subset, Lahman subset, LDBC subset, and range-filter joins.
- Benchmark ablations: binary-like Free Join without factoring, factored Free Join, static cover, dynamic cover, scalar, vectorized batch 10/100/1000, materialized output, factorized output, durable sorted access, COLT access.
- Robustness benchmarks: good versus intentionally bad binary plans, including bushy plans that force large materialized intermediates.

## Open Questions

- What produces the starting binary plan in Bumbledb, given the Rosetta Stone forbids SQL/server mode and the typed query IR is unstable? Options are an internal cost-based binary optimizer over typed IR, a deterministic atom-order binary plan for now, or an explicit internal plan injection API for tests/benchmarks.
- How much of the paper's DuckDB-driven plan flow should be retained when Bumbledb's product target is embedded typed Rust queries rather than SQL?
- Should durable LMDB access images remain the primary access backend, or should COLT be implemented over column images with optional durable-index acceleration?
- How should range predicates participate in Free Join covers and `binary2fj`, since the paper mainly describes equality conjunctive joins and Bumbledb supports comparison predicates evaluated at binding depths?
- Can factorized output be exposed internally only, with public `QueryOutput` unchanged, or should the API grow a separate count/factorized output mode?
- What cache key should represent plan-dependent image scopes once dynamic cover selection and lazy access forcing can differ across executions with the same query shape and different inputs?
