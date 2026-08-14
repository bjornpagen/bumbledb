# docs-014: "then the whole cte-list" — CTE creeping back as the emitted shape

- **Severity:** low
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F14
- **Depends on:** none (prose; same file as docs-012/013)

## The bug

`docs/architecture/60-validation.md:98` — "it emits SQL `WITH [RECURSIVE]` then the whole cte-list because that is what SQLite speaks". The paragraph correctly frames the translator as lossy and external; the residual is "cte-list" as the name of what WE emit.

## Why it's wrong

SQLite's spelling can be named as SQL without importing CTE as a bumbledb noun (Insight 1): "cte-list" invites reading the IR as a CTE list — the exact coordinate the paragraph exists to deny.

## The fix

Per `audit/CONTRACT.md §C7`: "The translator emits SQLite `WITH RECURSIVE` (SQLite's spelling of interiors + rec + main). That SQL is not a grammar for the language and not a field in the IR."

## Acceptance criteria

- [ ] Gone: `rg -in 'cte-list' docs/architecture/60-validation.md` → no matches.
- [ ] The lossy-translator framing (audit-approved) unchanged.

## Constraints

- Prose only. `WITH RECURSIVE` as SQLite's name is fine — CTE as OUR noun is what dies.
