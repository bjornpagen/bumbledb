# engine-038: conformance JSON dropped the Program fields; engine stats did not

- **Severity:** low
- **Tree:** engine
- **Status:** DUPLICATE(engine-012)
- **Source:** audit/engine.md F38

The observation — `bumbledb-bench/src/conformance/reach.rs:7-11` boasts "No `predicates` / `output` / `strata` / `idb`" while `ExecutionStats` keeps `reach: Option` and the introspection comments keep `strata` (`stats.rs:48-52`, `exec/introspection.rs:90`) — is the same split-brain engine-012 (structure) and engine-007/011 (the `stats.strata` prose) already own. Aligning `ExecutionStats` with the pipeline sum IS the alignment with the JSON's tables. No residual edit.
