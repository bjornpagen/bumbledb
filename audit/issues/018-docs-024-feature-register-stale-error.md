# docs-024: feature register cites the old error name `AggregateInteriorPredicate`

- **Severity:** high
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F24
- **Depends on:** none (prose; same file as docs-025/026 — one fixer may take feature-register.md)

## The bug

`docs/feature-register.md:23` — "refused today by name (`AggregateInteriorPredicate`). Not a feature — a doctrine reversal."

## Why it's wrong

The feature register is the LIVING product ledger, and it names a refusal that no longer exists under that name (Insight 1 — drift already delivered): the current refusal is `AggregateInInterior` (interior/rec heads project bound variables). `AggregateInteriorPredicate` is the old predicate-table error; a reader greps for it and finds nothing (verify: `rg -nw 'AggregateInteriorPredicate' crates/bumbledb/src` → empty; `rg -nw 'AggregateInInterior' crates/bumbledb/src/error.rs` → the real one).

## The fix

Per `audit/CONTRACT.md §C7`: "refused today by name (`AggregateInInterior`; measure finds likewise `MeasureInInterior`) — folds and measure finds are legal only at the main head."

## Acceptance criteria

- [ ] Gone: `rg -n 'AggregateInteriorPredicate' docs/feature-register.md` → no matches (the `docs/research/` hits are archival and stay).
- [ ] The cited names exist in code: `rg -nw 'AggregateInInterior|MeasureInInterior' crates/bumbledb/src/error.rs` → both found.

## Constraints

- Prose only; the refusal's semantics unchanged.
