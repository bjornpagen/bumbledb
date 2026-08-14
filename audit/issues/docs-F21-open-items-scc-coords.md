# docs-F21: OPEN items located in SCC/Tarjan/predicate-table coordinates

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F21
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/README.md`:
> **Mutual-linear** (one SCC, several names, …) … Refused this cut so
> Tarjan / k-variants / multi-pred scratch stay gone. … not a
> resurrection of a predicate table. … still not a second SCC. …
> Refused this cut so `NegationInRec` covers the whole SCC

## Fix (cites CONTRACT C7)

OPEN items are present-tense product. Speak: mutual-linear = several
names, each rule ≤1 rec atom — a new IR, not `Option<Rec>`; a named
interior of a finished rec is not a second rec; `NegationInRec`
covers the one rec. The refusals themselves DO NOT change.

## Acceptance criteria

- [ ] Grep `SCC|Tarjan|predicate table|k-variant` over
      `docs/architecture/README.md` returns empty.
- [ ] The OPEN refusal list is unchanged in content.
