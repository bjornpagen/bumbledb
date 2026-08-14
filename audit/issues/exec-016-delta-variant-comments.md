# exec-016: introspection still narrates delta-variants / `stats.strata`

- **Severity:** high
- **Tree:** exec
- **Status:** DUPLICATE(engine-007)
- **Source:** audit/plan-exec.md F20 (delta-variant / strata comments)

engine-007 owns deletion of `DeltaVariant` and the "delta variants" / `stats.strata` / "per-stratum" prose at `exec/introspection.rs:66-94` and `display.rs:23`. engine-011 absorbs leftover Program wording; engine-029 absorbs the unit-labels mode bit. No separate fix lands under this id.
