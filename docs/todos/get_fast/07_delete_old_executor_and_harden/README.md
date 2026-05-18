# 07: Delete Old Executor And Harden

**Goal**
- Remove architectural leftovers, simplify the query layer, and harden the replacement executor.

This is the cleanup stage that prevents a permanent half-migration.

**Thesis**
- A fast engine with two execution models becomes a slow codebase.
- After WCOJ is working, old abstractions should be deleted aggressively.
- Tests should protect behavior, not implementation nostalgia.

**Hard Cut**
- Delete old atom-recursive planner/executor code.
- Delete compatibility helpers used only by the old executor.
- Delete tests that assert old relation-atom plan order.
- Delete explain fields that describe the old plan as primary output.
- Delete unused scan APIs if they exist only for old query execution.

**Code To Inspect For Removal**
- `execute_atoms` and recursive atom execution helpers.
- `ChosenAccess` if it no longer represents trie access planning.
- `open_scan` if it exists only to return decoded scan rows.
- `match_atom` over decoded `Row`.
- `PlanCounters` fields that no longer describe the executor.
- Relation-atom debug spans and atom-completion events.

**API Simplification**
- Keep public logical APIs only where they are still product-shaped.
- Do not preserve internal names for downstream compatibility.
- Rename plan/counter types to match the new executor.
- Keep the facade clean: users should not see WCOJ internals unless they ask for explain/debug output.

**Correctness Hardening**
- Golden tests for all benchmark query results.
- Property tests comparing WCOJ output to SQLite on generated datasets.
- Fuzz parser/typechecker remains active.
- Add query executor fuzzing if practical with bounded schemas and generated rows.
- Add tests for encoded equality, encoded ordering, `Id`/`Ref` normalization, and dictionary-backed values.

**Performance Hardening**
- Allocation checks for broad joins where feasible.
- Decode-count checks for queries that should stay encoded.
- Counter sanity tests for broad joins.
- Regression thresholds in docs or benchmark harness, even if not yet CI-enforced.

**Documentation Updates**
- Update `docs/ROSETTA_STONE.md` with the new query architecture.
- Update `docs/ERRORS_AND_TRACING.md` span/counter names.
- Update `docs/BENCHMARKS.md` with new baseline numbers and interpretation.
- Update older stage docs if they describe the old executor as current.

**Passing Criteria**
- Search finds no old nested-loop executor implementation.
- Search finds no query hot-path full-row decode.
- Tests and clippy pass.
- Benchmark docs reflect the new architecture and latest numbers.
- The codebase has one planner/executor story.

**Design Trap To Avoid**
- Do not leave old code around because it is useful for comparison. Git history is the comparison path.
