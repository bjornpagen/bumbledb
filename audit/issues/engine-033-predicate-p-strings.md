# engine-033: render/display still print `interior p{id}` / `predicate p{id}` / `recursive p{id}`

- **Severity:** low
- **Tree:** engine
- **Status:** OPEN
- **Source:** audit/engine.md F33
- **Depends on:** engine-012/engine-029 (share the single `INTROSPECTION_VERSION` bump)

## The bug

`ir/render.rs:148,158` writes `interior p{id}` and `recursive p{id}` (rec as "predicate pN" via the len pun); `exec/introspection/display.rs:152-153` prints Interior sources as Datalog predicates:

```rust
crate::ir::AtomSource::Interior(pred) => format!("predicate p{}", pred.0),
```

and `display.rs:87` prints `interior p{}: {} emits`.

## Why it's wrong

The diagnostic surface is where users learn the model's names; printing `predicate p3` for a derived table teaches the deleted Datalog/IDB vocabulary on every EXPLAIN (Insight 1). The type is Interior/Rec/Query; the strings are Program.

## The fix

Per `audit/CONTRACT.md §C3` vocabulary: `interior {id}`, `rec` (the one rec needs no number in display; render may keep the dense id for round-tripping — `interior {id}` / `rec`), `main`. Concretely: render's rule prefixes `interior {id}` / `rec`; display's source label `interior {id}` (or `rec` when the stored id equals the witness's rec id — read the stored value per engine-028, never re-derive); stats line `interior {}: {} emits`.

Every rendered-output test/snapshot containing the old strings updates in the SAME change, and `INTROSPECTION_VERSION` increments — coordinate with engine-012/029 so the version moves exactly once. `rendered_query()`'s output is also the introspection header (`introspect.rs:110`) — one string authority, one change.

## Acceptance criteria

- [ ] Gone: `rg -n '"predicate p|interior p\{|recursive p\{' crates/bumbledb/src` → no matches.
- [ ] Honest snapshots: every snapshot/test-string change in this commit is a mechanical old-string→new-string rewrite (no numeric or structural assertion touched).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- One version bump shared with engine-012/029. Diagnostic-surface only; the IR and answers unchanged.
