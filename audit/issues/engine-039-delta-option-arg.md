# engine-039: `prepare_rule_variant`'s `delta: Option<OccId>` is a boolean with an id stuffed in

- **Severity:** low
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F39
- **Depends on:** engine-007 (lands as part of its rec-arm prepare split), engine-002

## The bug

`build.rs:642-674` — `prepare_rule_variant(..., delta: Option<OccId>)`: `None` = not a delta plan (every CQ/interior/base caller), `Some(id)` = mark that occurrence with the delta floor (rec-arm caller, `build.rs:414-423`). `prepare_rule` is a wrapper passing `None`. The distinct illegal state: `Some(id)` naming an occurrence the rule doesn't have is representable and would silently mis-floor the plan.

## Why it's wrong

Option-as-flag with a payload that can dangle (Insight 5): which function you're preparing (an ordinary rule vs. a rec arm) is static knowledge at every call site, so encoding it as a runtime argument admits mismatched states and forces the shared body to branch.

## The fix

Per `audit/CONTRACT.md §C3`: two entries — `prepare_rule(...) -> PreparedRule` for CQ/interior/base/main, and `prepare_rec_arm(..., delta: OccId) -> RecArm` for rec arms (returning engine-002's type, which also deletes the `let PreparedRule::FreeJoin(fj) = prepared else { unreachable! }` unwrap at `build.rs:424-426`). The floor choice is applied where the arm's occurrence is marked (engine-017/018's role data), not threaded as an argument past every non-rec caller. Shared plumbing stays in a private helper both entries call — the Option does not survive in any signature.

## Acceptance criteria

- [ ] Gone: `rg -n 'delta: Option<OccId>' crates/bumbledb/src` → no matches; `rg -n 'prepare_rule_variant' crates/bumbledb/src` → no matches (renamed/split).
- [ ] Unchanged tests: all green; plans identical (floors applied to the same occurrences).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands inside engine-007's change (one fixer). Plan outputs byte-identical.
