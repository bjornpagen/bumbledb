# docs-F08: "There is no separate program renderer"

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F8
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> There is no separate program renderer.

## Fix (cites CONTRACT C7)

Present tense, no denial of deleted types. Speak: `ir::render`
prints a `Query`: interiors, optional rec, then bare main rules.

## Acceptance criteria

- [ ] Grep `program renderer` over `docs/` returns empty.
- [ ] `bash scripts/lean.sh` green.
