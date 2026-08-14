# docs-025: feature register says "zero stratification impact"

- **Severity:** high
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F25
- **Depends on:** lean-002 (`recLinear` is not the destination name in the rewrite)

## The bug

`docs/feature-register.md:26` — "…`AggregateSink::finalize_into`, zero stratification impact, no new Lean axioms".

## Why it's wrong

Stratum is a deleted coordinate (Insight 1): the living ledger measures a feature against machinery that no longer exists. What the sentence means is that weak-form HAVING does not touch the rec roster.

## The fix

Per `audit/CONTRACT.md §C7`: "no change to `NegationInRec` / the one linear rec; no new Lean axioms." Do **not** keep `recLinear` (C4 / lean-002 deletes it; C7 wants the successor).

## Acceptance criteria

- [ ] Gone: `rg -in 'stratif' docs/feature-register.md` → no matches.
- [ ] The no-new-axioms claim and the feature's row unchanged otherwise.

## Constraints

- Prose only.
