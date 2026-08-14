# engine-001: interiors live beside `PreparedBody` — one pipeline sum, interiors inside each arm

- **Severity:** high
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F1
- **Depends on:** none (foundation; co-lands with engine-002, engine-015)
- **Conflicts with:** engine-008, engine-009, engine-012, engine-014, engine-023, engine-025, engine-029, engine-031 (same types; land after per INDEX)

## The bug

`crates/bumbledb/src/api/prepared.rs` — `PreparedQuery` carries `interiors: Vec<PreparedInterior>` AND `body: PreparedBody` (`Empty | Rules | Reach`) as sibling fields. Every consumer reconstitutes the missing coordinate from two flags:

```rust
// execute.rs:94
if self.interiors.is_empty() && matches!(self.body, PreparedBody::Empty) {
    return Ok(());
}
// execute.rs:100-107 — the fast lane re-tests both
if self.interiors.is_empty() && matches!(self.body.rules(), [PreparedRule::KeyProbe(...)])
// execute.rs:162
if !self.interiors.is_empty() || matches!(self.body, PreparedBody::Reach(_)) {
// reach.rs:133-134 — run_derived opens by rebuilding the derived count
let derived_count =
    self.interiors.len() + usize::from(matches!(self.body, PreparedBody::Reach(_)));
```

`profile` (`introspect.rs:214,221,263,300`) repeats the same forest with its own predicates.

## Why it's wrong

Two independent fields spell ~8 states; a handful are meaningful, and each call site re-derives which one it is with a different boolean product (Insight 4 — Minsky's three-boolean problem with better names). Interiors sit *beside* the body enum because the body used to be the whole Program; the sidecar is the Program coordinate system surviving the rename (Insight 2).

## The fix

Per `audit/CONTRACT.md §C3`. One pipeline sum, interiors inside each arm:

```rust
enum PreparedPipeline {
    Cq    { interiors: Vec<PreparedInterior>, rules: Vec<PreparedRule> },
    Reach { interiors: Vec<PreparedInterior>, driver: ReachDriver,
            main: Vec<PreparedRule>, rounds_budget: u32 },
}
```

- NO `Empty` variant (engine-023): statically-dead main is `rules: []`, and interiors-only-with-dead-main is `Cq { interiors, rules: vec![] }` — the preamble still runs, the main loop is zero iterations.
- `run_rules` becomes a match on the pipeline; `run_derived` is not a gate in front of the loop, it is the body of each arm's derived phase.
- `visit_rules`/`visit_rules_mut` (`prepared.rs:458+`) walk the arm they matched, not interiors-then-body separately.
- `PreparedBody`, `body.rules()`, `body.rules_mut()` die; main rules are addressed per arm (engine-015 moves main out of the driver).
- Builder (`build.rs:263-270`) constructs the arm directly from the witness sum (engine-016), never `if let Some(mut driver) = rec { driver.main = rules; ... }`.

## Acceptance criteria

- [ ] Gone: `rg -nw 'PreparedBody' crates/bumbledb/src` → no matches; `rg -n 'interiors\.is_empty\(\) &&' crates/bumbledb/src/api` → no matches; `rg -n 'usize::from\(matches!' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: `cargo test -p bumbledb --lib` and `cargo test -p bumbledb --test api --test adversarial_ir` pass with zero assertion edits; observable behavior (answers, error names, budgets) identical.
- [ ] New locks: a unit test (suggested `pipeline_shape.rs` or extend `tests/api.rs`) asserting interiors-only-with-dead-main still emits interior stats (pins engine-023's semantics).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`; conformance corpus unchanged (`lean/conformance/cases` untouched).

## Constraints

- Semantics identical: walls, OPEN refusals, `DerivedBudgetExceeded`/`set_derived_budget`/`DEFAULT_DERIVED_TUPLES`/`DEFAULT_REACH_ROUNDS` names and values locked. No new caps; `MAX_CTES`/`MAX_INTERIORS` stay dead.
- The UNTRUSTED `crates/bumbledb/src/ir.rs::Query` shape does NOT change (CONTRACT §C1); this issue is prepared-side only.
- Co-lands with engine-002 (RecArm) and engine-015 (main out of the driver); coordinate with engine-016 (witness sum input).
