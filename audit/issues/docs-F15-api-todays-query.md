# docs-F15: "today's query plus two empty fields" on the embedding API

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F15
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/70-api.md`:
> A query with empty interiors and no rec prepares as today's query
> plus two empty fields

## Fix (cites CONTRACT C7)

Speak: `Db::prepare(&Query)` — empty interiors and `rec: None` is an
ordinary `Query` (the `evalQuery` plain case).

## Acceptance criteria

- [ ] Grep `today's query` over `docs/architecture/70-api.md`
      returns empty.
