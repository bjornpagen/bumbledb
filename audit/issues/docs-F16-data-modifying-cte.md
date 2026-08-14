# docs-F16: "data-modifying CTEs" as the name of the write idioms (API chapter)

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F16
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/70-api.md`:
> everything SQL spells with data-modifying CTEs — must read on a
> snapshot first
> the data-modifying-CTE shapes with the premises witnessed instead
> of locked.

## Fix (cites CONTRACT C7)

Speak: insert-select / update-where — query on a snapshot, then
`write_from`. SQL's data-modifying `WITH` may appear as a translator
analogy at most, never as the idiom's name.

## Acceptance criteria

- [ ] Grep `(?i)\bcte` over `docs/architecture/70-api.md` returns
      empty.
