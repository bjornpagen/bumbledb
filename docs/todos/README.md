# Todo Roadmap

This folder turns `docs/ROSETTA_STONE.md` into implementation stages.

The Rosetta Stone remains normative. These todo documents are the execution plan.

**Rules**
- Keep stages broad enough to produce useful vertical slices.
- Do not split tasks into tiny tickets unless a stage becomes unmanageable.
- A stage is complete only when its passing criteria are met.
- If design pressure contradicts the Rosetta Stone, update the Rosetta Stone first.
- Do not pull deferred features forward unless they unblock a current-stage passing criterion.
- Prefer one working, tested path over several partial abstractions.

**Stage Order**
- `01_project_skeleton_and_lmdb_foundation.md`
- `02_schema_types_and_encoding.md`
- `03_storage_write_path_and_constraints.md`
- `04_read_snapshots_and_access_paths.md`
- `05_datalog_frontend_and_typechecker.md`
- `06_planner_executor_and_aggregation.md`
- `07_observability_testing_and_benchmarks.md`
- `08_bulk_etl_backup_and_hardening.md`
- `09_deferred_features.md`
- `10_comprehensive_testing_and_hardening.md`
- `11_errors_and_tracing_foundation.md`

**Get Fast Mission**
- `get_fast/README.md`
- `get_fast/01_encoded_trie_wcoj_executor/README.md`
- `get_fast/02_encoded_bindings_and_late_materialization/README.md`
- `get_fast/03_statistics_and_variable_ordering/README.md`
- `get_fast/04_index_permutations_and_access_layouts/README.md`
- `get_fast/05_factorized_aggregation/README.md`
- `get_fast/06_tracing_and_benchmark_gates/README.md`
- `get_fast/07_delete_old_executor_and_harden/README.md`

The get-fast mission supersedes incremental tuning of the current query executor. Its core directive is to replace relation-at-a-time recursive execution with encoded trie/WCOJ execution and remove the old path rather than maintain dual engines.

**Rearchitecture V2 PRD Suite**
- `rearchitecture_v2/README.md`
- `rearchitecture_v2/00_architecture_and_rca.md`
- `rearchitecture_v2/01_query_image.md`
- `rearchitecture_v2/02_columnar_relation_image.md`
- `rearchitecture_v2/03_sorted_trie_index.md`
- `rearchitecture_v2/04_leapfrog_triejoin_executor.md`
- `rearchitecture_v2/05_free_join_plan_ir.md`
- `rearchitecture_v2/06_hash_trie_and_hybrid_nodes.md`
- `rearchitecture_v2/07_factorized_projection_and_aggregation.md`
- `rearchitecture_v2/08_optimizer_and_statistics.md`
- `rearchitecture_v2/09_durable_segments_and_snapshots.md`
- `rearchitecture_v2/10_benchmark_gates_and_testing.md`
- `rearchitecture_v2/11_cutover_and_code_deletion.md`
- `rearchitecture_v2/12_query_normalization_and_runtime_specialization.md`
- `rearchitecture_v2/13_dependency_graph_and_migration_plan.md`

The rearchitecture suite is the successor to the get-fast experiment. It treats LMDB as durable storage and moves hot query execution to snapshot-local QueryImages, specialized sorted/hash tries, Free Join plans, LFTJ, hybrid probes, and factorized aggregation.

**Trace-Backed Performance Kill List**
- `performance_kill_list/README.md`
- `performance_kill_list/01_cache_planner_stats.md`
- `performance_kill_list/02_cache_query_image_indexes.md`
- `performance_kill_list/03_route_queries_through_query_image_cache.md`
- `performance_kill_list/04_real_hash_probe_runtime.md`
- `performance_kill_list/05_direct_selective_query_kernels.md`
- `performance_kill_list/06_optimize_lftj_inner_loop.md`
- `performance_kill_list/07_improve_cardinality_estimates.md`
- `performance_kill_list/08_add_phase_timing_and_tracing.md`

This kill list is derived from scale-10000 trace evidence after the v2 cutover. It is ordered by observed time impact and is the next performance execution plan.

**Observability, Lints, And Allocation Hardening**
- `observability_lints_allocation_hardening/README.md`
- `observability_lints_allocation_hardening/00_baseline_inventory_and_guardrails.md`
- `observability_lints_allocation_hardening/00_baseline_results.md`
- `observability_lints_allocation_hardening/01_workspace_lints_and_clippy_policy.md`
- `observability_lints_allocation_hardening/01_workspace_lints_results.md`
- `observability_lints_allocation_hardening/02_panic_unwrap_and_smell_cleanup.md`
- `observability_lints_allocation_hardening/02_panic_unwrap_results.md`
- `observability_lints_allocation_hardening/03_query_observability_data_model.md`
- `observability_lints_allocation_hardening/03_query_observability_results.md`
- `observability_lints_allocation_hardening/04_tracing_and_profiling_ux.md`
- `observability_lints_allocation_hardening/04_tracing_profiling_ux_results.md`
- `observability_lints_allocation_hardening/05_allocation_recording_and_heap_observability.md`
- `observability_lints_allocation_hardening/05_allocation_observability_results.md`
- `observability_lints_allocation_hardening/06_stack_gat_and_hot_path_allocation_cleanup.md`
- `observability_lints_allocation_hardening/07_verification_and_handoff.md`

This interstitial suite runs after `performance_kill_list/04_real_hash_probe_runtime.md` and before `performance_kill_list/05_direct_selective_query_kernels.md`. It hardens compiler/linter policy, panic cleanup, phase timing, profiling UX, allocation recording, and first-pass stack/GAT allocation cleanup so the next performance PRDs are trace-backed and heap-observable.

**Completion Philosophy**
- Each stage should leave the project in a coherent state.
- Tests should pass at every stage boundary.
- Documentation should match behavior at every stage boundary.
- Performance work should be measured, not guessed.
- The first implementation can be simple, but it should not violate the core architecture.
