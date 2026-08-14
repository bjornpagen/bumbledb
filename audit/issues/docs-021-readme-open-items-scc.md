# docs-021: architecture README's OPEN items still locate the design in SCC/Tarjan/predicate-table coordinates

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F21
- **Depends on:** none (prose; parallel-safe — own file)

## The bug

`docs/architecture/README.md:82-103`:

> **Mutual-linear** (one SCC, several names, each rule ≤1 rec atom). … even/odd encodes as one linear predicate with a parity column. Refused this cut so Tarjan / k-variants / multi-pred scratch stay gone. … not a resurrection of a predicate table.
> … still not a second SCC.
> Refused this cut so `NegationInRec` covers the whole SCC

Also `docs/architecture/README.md:230` — "caps are product decisions; no rule-program" (retired vocabulary by negation, same file).

## Why it's wrong

OPEN items are present-tense product surface (Insight 1): they define the refused futures in the deleted coordinates (SCC, Tarjan, predicate table), so the roadmap itself keeps the old model as the measuring frame. The refused future is `List Rec` / several names — not "a second SCC".

## The fix

Per `audit/CONTRACT.md §C7`: "Mutual-linear: several names, each rule ≤1 rec atom — a new IR (`List Rec` or named recs), not `Option<Rec>`. even/odd encodes as one linear rec with a parity column. Named interior of a finished rec is not a second rec. `NegationInRec` covers the one rec." Delete Tarjan / predicate-table / SCC framing (the "stay gone" claims can survive as "the condensation machinery stays deleted" ONCE, if the OPEN item genuinely needs the refusal recorded — prefer the positive statement).

## Acceptance criteria

- [ ] Gone: `rg -in 'SCC|tarjan|predicate table|linear predicate' docs/architecture/README.md` → no matches.
- [ ] The OPEN/refused rulings themselves (what is refused this cut) semantically unchanged.

## Constraints

- Prose only. Do not silently change any refusal's SCOPE — the walls are locked (mutual/nonlinear/stacked/named-interior-of-finished-rec stay OPEN refusals).
