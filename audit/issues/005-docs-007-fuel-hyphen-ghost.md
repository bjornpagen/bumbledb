# docs-007: the wall named "fuel-is-not-denotation" — fuel survives as a hyphenated ghost

- **Severity:** medium
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F7
- **Depends on:** none (prose; same file as docs-001..010)

## The bug

`docs/architecture/20-query-ir.md:208` — "fuel-is-not-denotation (`reachDen` is `lfpS`; the budget is a resource abort)".

## Why it's wrong

The proposition is correct; the NAME keeps the deleted semantic parameter as the wall's identity (Insight 1) — a reader must know what "fuel" was to parse why it isn't the denotation. Dead vocabulary should not survive as the label of its own tombstone.

## The fix

Per `audit/CONTRACT.md §C7`: name the wall for what IS: "Denotation is `reachDen = lfpS`. The derived-tuples / rounds budget is a resource abort (`DerivedBudgetExceeded`) — incompleteness versus `evalQuery`, not a semantic parameter."

## Acceptance criteria

- [ ] Gone: `rg -in 'fuel' docs/architecture/20-query-ir.md` → no matches.
- [ ] `reachDen = lfpS`, `DerivedBudgetExceeded`, and the incompleteness-vs-`evalQuery` claim unchanged.

## Constraints

- Prose only; locked names untouched. `lean/README.md`'s "Fuel is not a Lean semantic parameter" is NOT in scope (audit ruled it the correctly-stated wall).
