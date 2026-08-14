# docs-F25: "zero stratification impact" in the living ledger

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F25
Blocked-by: lean-H2 (successor name)
Blocks: none

## Bug

`docs/feature-register.md`:
> zero stratification impact, no new Lean axioms

## Fix (cites CONTRACT C7)

Speak: no change to the one linear rec's well-formedness
(post-lean-H2 name) / `NegationInRec`; no new Lean axioms. Stratum
is a deleted coordinate.

## Acceptance criteria

- [ ] Grep `stratif` over `docs/feature-register.md` returns empty;
      cited Lean names resolve (census).
