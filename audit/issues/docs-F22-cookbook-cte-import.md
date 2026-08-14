# docs-F22: cookbook insert-select taught as "the data-modifying CTE"

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F22
Blocked-by: none
Blocks: none

## Bug

`docs/cookbook.md`:
> **Insert-select**: query source answers, insert the derived facts —
> the data-modifying CTE with its premises witnessed instead of
> locked.

## Fix (cites CONTRACT C7)

Speak: insert-select — query source answers, insert the derived
facts, `write_from` witnessing the snapshot. No CTE as the idiom's
name.

## Acceptance criteria

- [ ] Grep `(?i)\bcte` over `docs/cookbook.md` returns empty.
