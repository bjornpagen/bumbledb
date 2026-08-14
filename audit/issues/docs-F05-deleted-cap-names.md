# docs-F05: deleted cap names taught by negation (IR chapter)

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F5
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> There is no `MAX_CTES` / `MAX_INTERIORS` / `TooManyCtes`.
> `InteriorIdOverflow` (… there is no `TooManyCtes`)

## Fix (cites CONTRACT C7)

No history, no retired names. Speak: derived-table count is `u32`
width (`InteriorIdOverflow`). There is no interior-count product cap.
Do not name CTE errors.

## Acceptance criteria

- [ ] Grep `MAX_CTES|MAX_INTERIORS|TooManyCtes` over
      `docs/architecture/` returns empty.
- [ ] `bash scripts/lean.sh` green.
