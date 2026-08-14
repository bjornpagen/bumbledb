# docs-004: "today's query plus two empty fields" — the plain case taught as an embedding

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F4
- **Depends on:** none (prose; same file as docs-001..010; siblings docs-015/docs-019 fix the same phrase in other files)

## The bug

`docs/architecture/20-query-ir.md:57` and `:102` — "a query with empty `interiors` and no rec is today's query plus two empty fields (`lean/Bumbledb/Exec/Reach.lean: evalQuery_plain`)" and "Main is today's query: one head, ≥1 rule, folds, measures, negation. … not an embedding into another type."

## Why it's wrong

Insight 3 (special cases are coordinate artifacts): "today's query" names a PRIOR type and treats the plain case as that type plus two fields — an embedding, exactly what the trailing clause denies. There is one `Query`; empty interiors and `rec: None` is a case of it, and `evalQuery_plain` is that case of `evalQuery`.

## The fix

Per `audit/CONTRACT.md §C7`: "A `Query` with empty `interiors` and `rec: None` is still a `Query`; `evalQuery_plain` is that case, not an embedding of a prior type." Rewrite both sites; keep the `evalQuery_plain` citation.

## Acceptance criteria

- [ ] Gone: `rg -in "today's query" docs/architecture/20-query-ir.md` → no matches.
- [ ] The `evalQuery_plain` citation survives; the "Main is: one head, ≥1 rule, folds, measures, negation" FACTS survive under the new framing.

## Constraints

- Prose only. Coordinate the phrasing with docs-015 (`70-api.md`) and docs-019 (`75-cpp-lowering.md`) so the three files speak one sentence.
