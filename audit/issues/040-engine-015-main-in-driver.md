# engine-015: main rules are stuffed into `ReachDriver` after construction — main has two homes

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED
- **Source:** audit/engine.md F15
- **Depends on:** engine-001 (co-lands: the Reach arm carries `main` beside the driver)

## The bug

`crates/bumbledb/src/api/prepared/build.rs` — the driver is built with a placeholder, then mutated:

```rust
// build.rs:443-446 — prepare_reach returns
Ok(super::reach::ReachDriver {
    base, rec: rec_rules,
    main: Vec::new(),           // stuffed later
    ...
})
// build.rs:263-265 — prepare_witnessed
let body = if let Some(mut driver) = rec {
    driver.main = rules;
    PreparedBody::Reach(Box::new(driver))
}
```

So main lives on `PreparedBody::Rules` OR on `ReachDriver.main` (`reach.rs:40-42` — "Main/answer rules, run after the rec closes"), never both, selected by the Option; `PreparedBody::rules()` reaches into the driver to find it.

## Why it's wrong

One thing, two homes, and a construct-then-mutate window in which `ReachDriver { main: Vec::new() }` is a real value that means something false (Insight 8: make the object valid at construction). Interiors weren't stolen this way — only main — because the body enum needed a place to put it; that asymmetry is the sidecar layout (engine-001) leaking into the driver.

## The fix

Per `audit/CONTRACT.md §C3`: main lives on the pipeline arm, always — `PreparedPipeline::Reach { interiors, driver, main, rounds_budget }`. `ReachDriver` is base arms + rec arms (`Vec<RecArm>` per engine-002) + rec sink/scratch/field_types/units — no `main` field. The `driver.main = rules` mutation and the `main`-through-driver accessor path delete; `run_rules`' post-rec main loop reads the arm's `main` directly.

## Acceptance criteria

- [x] Gone: `rg -n 'main:' crates/bumbledb/src/api/prepared/reach.rs` → no `main` field on `ReachDriver`; `rg -n 'driver\.main' crates/bumbledb/src` → no matches.
- [x] Unchanged tests: all reach tests green unchanged (main still runs strictly after the rec closes — order pinned by existing differential tests).
- [x] Green: `cargo test -p bumbledb --lib` pass (reach tests included).

## Constraints

- Execution order identical (interiors → rec fixpoint → main). Co-lands with engine-001.
