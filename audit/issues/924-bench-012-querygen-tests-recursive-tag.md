# bench-012: `querygen/tests.rs` still asserts interiors-only rows under `RecursiveVariant`

- **Severity:** low
- **Tree:** bench
- **Status:** DUPLICATE(engine-020)
- **Source:** audit/bench.md F12

engine-020 owns the coverage-label split (`InteriorsDag` / `InteriorsAntiJoin` / `ManyInteriors` reported as interiors, not recursive). The tests at `querygen/tests.rs:490-491,649-678` are that issue's tree. **Do not rename the `RecursiveVariant` Debug variants** — `reach-*.json` provenance embeds `"{variant:?}"` (C1). No residual edit under this id.
