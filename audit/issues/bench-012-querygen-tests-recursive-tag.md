# bench-012: `querygen/tests.rs` still asserts interiors-only rows under `RecursiveVariant`

- **Severity:** low
- **Tree:** bench
- **Status:** DUPLICATE(engine-020)
- **Source:** audit/bench.md F12

engine-020's acceptance already requires `InteriorsDag` / `InteriorsAntiJoin` / `ManyInteriors` to leave `RecursiveVariant`, and the coverage tests at `querygen/tests.rs:490-491,649-678` are that issue's tree. No residual edit under this id.
