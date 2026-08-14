# docs-F07: the wall named after the deleted fuel parameter

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F7
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> fuel-is-not-denotation (`reachDen` is `lfpS`; the budget is a
> resource abort)

## Fix (cites CONTRACT C7)

Speak: denotation is `reachDen = lfpS`. The derived-tuples / rounds
budget is a resource abort (`DerivedBudgetExceeded`), incompleteness
versus `evalQuery`, not a semantic parameter. Drop the hyphenated
fuel ghost.

## Acceptance criteria

- [ ] Grep `fuel-is-not-denotation` over `docs/` returns empty.
- [ ] `DerivedBudgetExceeded` named as-is; `bash scripts/lean.sh`
      green.
