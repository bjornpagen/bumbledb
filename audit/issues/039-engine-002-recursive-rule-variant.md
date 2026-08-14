# engine-002: `PreparedRule::Recursive` is legal in every rule list; `unreachable!` is the typechecker

- **Severity:** high
- **Tree:** engine
- **Status:** FIXED
- **Source:** audit/engine.md F2
- **Depends on:** engine-001 (co-lands; `RecArm` lives in the Reach arm's driver)
- **Conflicts with:** engine-007, engine-025, engine-026 (same enum; they land with or after this)

## The bug

`crates/bumbledb/src/api/prepared.rs` — `PreparedRule` is `FreeJoin | KeyProbe | Recursive`, so a Recursive rule is representable in interiors, base arms, main, and `Rules` bodies. Every walk carries a panic arm:

```rust
// execute.rs:299-301
PreparedRule::Recursive(_) => {
    unreachable!("recursive rules run under the reach driver, never the main rule loop")
}
```

Same at `introspect.rs:53-55, 73-75, 100-102, 312-314, 343-345` (five more `unreachable!`s). Worst, `reach.rs:417-419` silently *skips* the illegal state instead of refusing it:

```rust
let PreparedRule::Recursive(rule) = &driver.rec[rule_idx] else {
    continue;
};
```

## Why it's wrong

A type-code plus a forest of guards (Insight 4): the variant is admitted everywhere and forbidden almost everywhere, so the invariant lives in seven scattered control-flow sites — six panics and one silent `continue` that would drop a rec arm's derivations if construction ever drifted. The essential fact (one delta occurrence per rec arm) belongs on the rec-arm slot's *type*, not on a tag every list can carry (Insight 5).

## The fix

Per `audit/CONTRACT.md §C3` ("Rules"):

```rust
enum PreparedRule { FreeJoin(FreeJoinRule), KeyProbe(KeyProbeRule) }
struct RecArm { delta: OccId, rule: FreeJoinRule }   // only ReachDriver.rec: Vec<RecArm>
```

- Interiors, base arms, main, and Cq rules are `Vec<PreparedRule>`; `ReachDriver.rec` is `Vec<RecArm>`.
- Every `Recursive` match arm — the six `unreachable!`s and the `continue` — deletes. `run_reach`'s rec loop iterates `&driver.rec` and reads `arm.delta` / `arm.rule` directly.
- `RecursiveRule` and `DeltaVariant` die here or in engine-007 (one change; engine-007 owns the variant-vocabulary sweep).
- Builder: `prepare_reach` (`build.rs:427-429`) constructs `RecArm { delta, rule: fj }` — the `let PreparedRule::FreeJoin(fj) = prepared else { unreachable! }` unwrap becomes the return type of a rec-arm-specific prepare (engine-039).

## Acceptance criteria

- [x] Unrepresentable: `rg -nw 'PreparedRule::Recursive|RecursiveRule' crates/bumbledb/src` → no matches; `rg -c 'unreachable!' crates/bumbledb/src/api/prepared` decreases by ≥6 with none of the survivors mentioning recursive rules.
- [x] Unchanged tests: `cargo test -p bumbledb` green with zero assertion edits; the 22 reach conformance cases and `bumbledb-bench` differential recursive tests unchanged and green.
- [x] New locks: none required — the deleted variant is the lock (a Recursive-in-main state can no longer be constructed to test).
- [x] Green: `cargo test -p bumbledb --lib` pass. Rec-arm `unreachable!`/`continue` gone; `Bridge.lean` `DeltaVariant` token moved to `RecArm` (C8).

## Constraints

- Semantics identical; nonlinear rec stays an OPEN refusal at validate (`NonlinearRecArm` — do not touch).
- Co-lands with engine-001; `Bridge.lean` mechanism tokens naming `DeltaVariant` move with engine-007's rename (`./scripts/lean.sh` census must stay green).
