# docs-006: "not a Tarjan condensation" — the judge taught by what it isn't

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F6
- **Depends on:** none (prose; same file as docs-001..010)

## The bug

`docs/architecture/20-query-ir.md:116` — "The rec roster (`lean/Bumbledb/Query/Syntax.lean: Query.recLinear`) is the judge, not a Tarjan condensation:"

## Why it's wrong

Same negation-of-retired-coordinate as docs-005 (Insight 1): Tarjan is the deleted stratification machinery, and naming it in the normative sentence keeps it as the reference frame the new judge is measured against.

## The fix

Per `audit/CONTRACT.md §C7`: "`Query.recLinear` (`lean/Bumbledb/Query/Syntax.lean`) is the well-formedness of the one linear rec (exactly one positive self-atom per rec arm, …)." Drop the Tarjan clause.

## Acceptance criteria

- [ ] Gone: `rg -in 'tarjan' docs/architecture/20-query-ir.md` → no matches.
- [ ] The `recLinear` citation and the well-formedness content unchanged.

## Constraints

- Prose only.
