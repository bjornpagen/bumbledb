# docs-F01: "hand-written multi-rule programs" in the IR chapter

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F1
Blocked-by: none (wave 3)
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> **Hand-written multi-rule programs keep the head-projection law**

## Fix (cites CONTRACT C7)

Speak: "Hand-written multi-rule **queries** keep the head-projection
law" (or "hand-written main rule-lists"). A hand-written rule-list is
a `Query` — main, one `Interior`, or the rec pool.

## Acceptance criteria

- [ ] Grep `multi-rule program` over `docs/architecture/20-query-ir.md`
      returns empty.
- [ ] Surrounding claim content unchanged; `bash scripts/lean.sh`
      green (census citations intact).
