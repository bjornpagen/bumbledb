# docs-015: 70-api teaches prepare of the plain case as "today's query plus two empty fields"

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F15
- **Depends on:** none (prose; same file as docs-016/017/018 — one fixer may take 70-api.md)

## The bug

`docs/architecture/70-api.md:509` — "A query with empty interiors and no rec prepares as today's query plus two empty fields".

## Why it's wrong

docs-004's embedding framing, now on the embedding API (Insight 3): "today's query" names a prior type and makes the plain case an embedding of it. One `Query`; the plain case is a case.

## The fix

Per `audit/CONTRACT.md §C7`: "`Db::prepare(&Query)` — empty interiors and `rec: None` is an ordinary `Query` (`evalQuery_plain`)." Match docs-004's chosen sentence.

## Acceptance criteria

- [ ] Gone: `rg -in "today's query" docs/architecture/70-api.md` → no matches.
- [ ] Prepare-API facts unchanged.

## Constraints

- Prose only; phrase-align with docs-004 and docs-019.
