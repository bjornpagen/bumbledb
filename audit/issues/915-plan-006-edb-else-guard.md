# plan-006: plan-side `source.edb() else` / `edb()?` forest (Interior as not-EDB)

- **Severity:** medium
- **Tree:** plan
- **Status:** DUPLICATE(engine-017)
- **Source:** audit/plan-exec.md F18

engine-017 owns the accepted half: past normalize, bind/floor kind is data on the occurrence (`Finished` / `RecDelta` / `RecAcc`), never `edb().is_none()`. Its grep (`edb().is_none()`, `edb()?` in plan/selectivity, densify, evaluate, provably_distinct, provably_disjoint, dispatch/classify) covers these sites. Boundary `AtomSource` stays C1. The panicking `PlanOccurrence::relation()` helper is the distinct OPEN issue plan-003.
