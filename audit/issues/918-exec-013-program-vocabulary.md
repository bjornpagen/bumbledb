# exec-013: Program / strata vocabulary in exec below prepare

- **Severity:** high
- **Tree:** exec
- **Status:** DUPLICATE(engine-011)
- **Source:** audit/plan-exec.md F20 (exec half)

engine-011 is the load-bearing sweep (false invariants, `stats.strata`, `idb_*`, "program" naming a Query). Its grep covers `exec/wordmap/clear.rs:47` ("non-recursive program cannot observe it"), `exec/sink.rs:7,31`, `exec/sink/aggregate/fold_row.rs:108`, `exec/dispatch/execute_key_probe.rs:11`, `exec/sink/projection/new.rs:74`, `exec/introspection.rs:15,66-94,114`, `exec/introspection/into_stats.rs:10`, `exec/introspection/counting_counters.rs:29`. Structured leftovers of that prose are engine-029 (unit_labels, exec-014), engine-033 (`predicate p{}`, exec-015), engine-007 (delta-variant / strata comments, exec-016). No separate fix lands under this id.
