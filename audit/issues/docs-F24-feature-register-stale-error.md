# docs-F24: feature register cites the deleted `AggregateInteriorPredicate`

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F24
Blocked-by: none
Blocks: none

## Bug

`docs/feature-register.md`:
> refused today by name (`AggregateInteriorPredicate`).

The living ledger names an error that no longer exists; the current
refusal is `AggregateInInterior`.

## Fix (cites CONTRACT C7)

Speak: `AggregateInInterior` / `MeasureInInterior` — folds and
measure finds are legal only at the main head.

## Acceptance criteria

- [ ] Grep `AggregateInteriorPredicate` over `docs/` returns empty;
      the named errors grep to real variants in `crates/…/error.rs`.
