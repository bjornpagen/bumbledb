# plan-004: `fj/validate.rs` claims a sealed ValidatedQuery has no Interior occurrence

- **Severity:** medium
- **Tree:** plan
- **Status:** DUPLICATE(engine-030)
- **Source:** audit/plan-exec.md F16

engine-030 owns deletion of the dead `normalize()` and the false "no Interior occurrence" sentence. engine-011 owns the same sentence as a false invariant. `plan/fj/validate.rs:198-217` is a cited site of both; no separate fix lands under this id. The `#[cfg(test)]` `validate` wrapper's honest doc ("EDB-only fixtures pass no derived signatures") is engine-030's fix.
