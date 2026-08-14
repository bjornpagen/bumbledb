# docs-006: "not a Tarjan condensation" — the judge taught by what it isn't

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F6
- **Depends on:** lean-002 (`recLinear` dies; linearity is structural on `LinearRec` — do not cite `recLinear` as the surviving judge)

## The bug

`docs/architecture/20-query-ir.md:116` — "The rec roster (`lean/Bumbledb/Query/Syntax.lean: Query.recLinear`) is the judge, not a Tarjan condensation:"

## Why it's wrong

Same negation-of-retired-coordinate as docs-005 (Insight 1): Tarjan is the deleted stratification machinery, and naming it in the normative sentence keeps it as the reference frame the new judge is measured against.

## The fix

Per `audit/CONTRACT.md §C7` + §C4 (`recLinear` dies): "The one rec's well-formedness is structural (exactly one positive self-atom per rec arm in the typed rec; nonempty base and step)." Drop the Tarjan clause. Do **not** keep `Query.recLinear` as the destination name — C7 wants the successor, not the dying Lean identifier. `NegationInRec` and the well-formedness *content* stay.

## Acceptance criteria

- [ ] Gone: `rg -in 'tarjan|recLinear' docs/architecture/20-query-ir.md` → no matches.
- [ ] The well-formedness content (one positive self-atom per rec arm, `NegationInRec`) unchanged.

## Constraints

- Prose only.
