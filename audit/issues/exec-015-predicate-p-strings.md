# exec-015: display still prints `predicate p{id}` / `interior p{}` / strata section

- **Severity:** low
- **Tree:** exec
- **Status:** DUPLICATE(engine-033)
- **Source:** audit/plan-exec.md F20 (display strings)

engine-033 owns the diagnostic strings (`display.rs:87,153,186`; render's `interior p{id}` / `recursive p{id}`). No separate fix lands under this id. One `INTROSPECTION_VERSION` bump shared with engine-012/029.
