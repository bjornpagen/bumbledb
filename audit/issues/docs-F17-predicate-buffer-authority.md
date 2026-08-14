# docs-F17: `PreparedQuery::predicate()` taught as "the predicate the query defines"

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F17
Blocked-by: eng-F41
Blocks: none

## Bug

`docs/architecture/70-api.md`:
> column metadata via `PreparedQuery::predicate()` — the predicate
> the query defines (`20-query-ir.md` § the query shape) is the
> **buffer-typing authority**

## Fix (cites CONTRACT C7, C3 amendment)

After eng-F41 renames the accessor: speak "column metadata via
`PreparedQuery::signature()` — the sealed main signature (answer
columns + folds) is the buffer-typing authority." No "predicate" as
the noun for main.

## Acceptance criteria

- [ ] Grep `predicate` over `docs/architecture/70-api.md` returns
      empty.
- [ ] Cited method name matches the code (census-adjacent honesty:
      grep the symbol in `crates/`).
