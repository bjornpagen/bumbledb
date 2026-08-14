# docs-F14: "the whole cte-list" residue in the translator paragraph

Severity: low
Tree: docs
Status: OPEN
Source: audit/docs.md F14
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/60-validation.md`:
> it emits SQL `WITH [RECURSIVE]` then the whole cte-list because
> that is what SQLite speaks

## Fix (cites CONTRACT C7)

Speak: the translator emits SQLite `WITH RECURSIVE` — SQLite's
spelling of interiors + rec + main. That SQL is not a field in the
IR. "cte-list" does not become a bumbledb noun.

## Acceptance criteria

- [ ] Grep `(?i)\bcte\b` over `docs/architecture/60-validation.md`
      returns empty.
