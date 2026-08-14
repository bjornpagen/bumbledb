# exec-014: `IntrospectionReport` encodes its two modes as "are the unit labels empty?"

- **Severity:** medium
- **Tree:** exec
- **Status:** DUPLICATE(engine-029)
- **Source:** audit/plan-exec.md F20 (unit_labels)

engine-029 owns the report-body sum (`Cq` vs `Reach`) and deletion of `unit_labels`. `exec/introspection.rs:83-97` and `display.rs:26-31` are its cited sites. No separate fix lands under this id. Coordinate `INTROSPECTION_VERSION` with engine-012/033 as engine-029 already requires.
