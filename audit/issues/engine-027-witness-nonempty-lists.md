# engine-027: witness rule lists are `Vec`s the constructor promises nonempty; downstream indexes `[0]` on faith

- **Severity:** medium
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F27
- **Depends on:** engine-005 (same witness types), engine-004 (same nonempty carrier decision)

## The bug

The hostile IR correctly represents empty main/interiors/finds (`ir.rs:417-466`) and validation refuses them by name (`EmptyRuleSet`, `EmptyInterior`, `EmptyFinds` — `validate.rs:48-50, 141-144, 379-387`). But the witness keeps plain `Vec<LoweredRule>` on `ValidatedMain`/`ValidatedInterior`, and downstream derives the predicate from the first element:

```rust
Predicate::derive(&lowered[0], &typings[0])
```

— an index that is total only because a constructor elsewhere refused emptiness.

## Why it's wrong

Same King gap as engine-004/005, milder: the established fact ("nonempty") is not in the type, so the `[0]` sites are latent panics guarded by module discipline rather than by construction (Insight 6). Alternatively viewed: the predicate is *derived* from element 0 at multiple points instead of stored once when it was first derived (Insight 9).

## The fix

Per `audit/CONTRACT.md §C3`, either arm is acceptable; prefer the second (smaller):

1. Nonempty carrier on `ValidatedMain`/`ValidatedInterior` (same type used for engine-004's rec arms), making `[0]` reads `first()`; or
2. Store the derived `Predicate` on the witness struct at sealing (it is already computed there) and delete the downstream re-derivations from `[0]` — the nonemptiness fact then has no remaining consumer beyond the roster refusal.

Boundary types stay open; `EmptyRuleSet`/`EmptyInterior`/`EmptyFinds` names and triggers locked.

## Acceptance criteria

- [ ] Gone: `rg -n '\[0\]' crates/bumbledb/src/ir/validate.rs crates/bumbledb/src/api/prepared/build.rs` → no faith-based first-element reads of witness rule lists (survivors must be justified by a local nonempty type).
- [ ] Unchanged tests: empties-refusal adversarial tests pass UNCHANGED.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Lands with the engine-004/005 witness change (one fixer, one wave).
