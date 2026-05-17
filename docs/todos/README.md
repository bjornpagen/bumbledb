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

**Completion Philosophy**
- Each stage should leave the project in a coherent state.
- Tests should pass at every stage boundary.
- Documentation should match behavior at every stage boundary.
- Performance work should be measured, not guessed.
- The first implementation can be simple, but it should not violate the core architecture.
