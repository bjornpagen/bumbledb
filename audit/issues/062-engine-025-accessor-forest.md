# engine-025: `visit_rules` and the `PreparedRule` accessor forest re-match Recursive to unwrap `.variant.rule`

- **Severity:** medium
- **Tree:** engine
- **Status:** FIXED(472b23ef)
- **Source:** audit/engine.md F25
- **Depends on:** engine-001, engine-002 (this is their acceptance surface on `prepared.rs:458-569`)

## The bug

`crates/bumbledb/src/api/prepared.rs:458-569` — `visit_rules`/`visit_rules_mut` match Empty/Rules/Reach and walk `interiors` as a separate loop (the sidecar again), and every `PreparedRule` accessor (`finds`, `slot_count`, `distinct_witness`, `dedup_spans`, `pinned`) carries a Recursive arm forwarding through the ghost wrapper:

```rust
PreparedRule::Recursive(rule) => rule.variant.rule.finds(),   // and four siblings
```

with comments narrating the dead k-wide design ("Variants project one head", "variant 0 speaks", "every variant shares the slot layout"). `distinct_witness` on Recursive is `None` *by policy comment* — a branch where the RecArm type simply wouldn't have the field.

## Why it's wrong

engine-002's tag-plus-guards pattern in miniature, at the read surface: five accessors × one ghost arm each, plus a visitor that hand-splices the sidecar. Policy-by-match-arm (`Recursive => None`) is a decision the type should make unwritable rather than the accessor make repeatedly (Insight 4).

## The fix

Rides engine-001 + engine-002; this issue is the checklist for the read surface:

- Accessors exist on `FreeJoinRule`/`KeyProbeRule` (and the two-variant `PreparedRule` where genuinely shared); `RecArm` exposes `.rule: FreeJoinRule` — callers needing a rec arm's finds say `arm.rule.finds` explicitly. No accessor has a Recursive arm because the variant does not exist.
- `visit_rules`/`visit_rules_mut` become per-arm iteration on the pipeline sum: `Cq` visits interior rules + main rules; `Reach` visits interior rules + base + rec arms' rules + main. One definition of "every rule of this query", written once.
- The "variant … speaks" comments delete (engine-007's sweep).

## Acceptance criteria

- [ ] Gone: `rg -n 'variant\.rule' crates/bumbledb/src` → no matches; `rg -n 'Recursive\(' crates/bumbledb/src/api/prepared.rs` → no matches; `rg -in 'variant 0 speaks|any variant speaks|shares the slot layout' crates/bumbledb/src` → no matches.
- [ ] Unchanged tests: `pending_literal_note`, slot-sizing (`Bindings::new` max computation), and every introspection test green UNCHANGED — the visitor must reach exactly the same rule set (base + rec + interiors + main).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Rides engine-001/002 — do not fork the enum change; this file exists so the fixer of 001/002 has the read-surface checklist and the INDEX can assign it as their verification step.
