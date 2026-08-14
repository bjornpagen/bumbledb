# docs-F06: "the judge, not a Tarjan condensation"

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F6
Blocked-by: lean-H2 (successor name for recLinear)
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> The rec roster (`lean/…: Query.recLinear`) is the judge, not a
> Tarjan condensation

## Fix (cites CONTRACT C7, C8)

Speak present tense: the typed `LinearRec` (post-lean-H2) is the
well-formedness of the one linear rec (exactly one positive
self-atom per rec arm, no negation). Drop Tarjan. Citation moves to
the successor declaration.

## Acceptance criteria

- [ ] Grep `Tarjan` over `docs/architecture/20-query-ir.md` returns
      empty.
- [ ] Cited declaration resolves (census green).
