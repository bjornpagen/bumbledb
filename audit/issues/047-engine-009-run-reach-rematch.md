# engine-009: `run_reach` re-matches `PreparedBody::Reach` four times because the layout won't split its borrows

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED(472b23ef)
- **Source:** audit/engine.md F9
- **Depends on:** engine-001 (the Reach arm owning its pieces is the fix)

## The bug

`crates/bumbledb/src/api/prepared/reach.rs:292-450` — the function is dispatched only on Reach, asserts it, then re-proves it every time it needs the driver mutably:

```rust
let PreparedBody::Reach(_) = &self.body else {
    unreachable!("run_reach is dispatched only on PreparedBody::Reach")
};                                                    // line 300
...
let PreparedBody::Reach(driver) = &mut self.body else {
    unreachable!("matched above")
};                                                    // line 310 (reset)
...
let PreparedBody::Reach(driver) = &mut self.body else {
    unreachable!("matched above")
};                                                    // line 344 (round 0)
...
loop {
    let PreparedBody::Reach(driver) = &mut self.body else {
        unreachable!("matched above")
    };                                                // line 374 (every iteration)
```

The cause is engine-001's layout: `self.interiors`, `self.derived`, `self.body` are sibling fields, so a `&mut ReachDriver` and `&mut DerivedScratch` cannot coexist without re-entering the enum each time the borrow lapses.

## Why it's wrong

Control flow compensating for a layout that will not split (Insight 5): four identical discriminant checks with `unreachable!("matched above")` are the code admitting the type keeps forgetting what the function proved at entry. Each re-match is a site where a future edit can silently pair the wrong pieces.

## The fix

Per `audit/CONTRACT.md §C3`: with engine-001's `PreparedPipeline::Reach { interiors, driver, main, rounds_budget }`, `run_reach` receives the destructured arm ONCE:

```rust
fn run_reach<C: Counters>(
    driver: &mut ReachDriver,
    derived: &mut DerivedImages,     // engine-013's unified scratch
    ctx: RunCtx<'_>, ...
) -> Result<bool>
```

called from the pipeline match that already holds `Reach { driver, .. }` — the borrow splits at the match site, all four inner re-matches delete. `ReachDriver` owns rec sink + rec scratch (it already does); it does NOT own main (engine-015).

## Acceptance criteria

- [x] Gone: `rg -n 'matched above' crates/bumbledb/src/api/prepared/reach.rs` → no matches. `run_reach` is a free function taking `&mut ReachDriver`; the pipeline match at `run_derived` splits the borrow once.
- [x] Unchanged tests: all reach/budget tests pass UNCHANGED (landed with engine-001).
- [x] Green: `cargo test -p bumbledb --lib` 1055 passed after the pipeline cluster.

## Constraints

- Semantics identical: round structure, watermark discipline, budget checks byte-identical (`DerivedBudgetExceeded { rounds, tuples }` payloads unchanged).
- Lands after engine-001.
