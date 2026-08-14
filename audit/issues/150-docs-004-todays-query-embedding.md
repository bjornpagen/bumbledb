# docs-004: "today's query plus two empty fields" — the plain case taught as an embedding

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F4
- **Depends on:** lean-001 (the Query sum deletes `evalQuery_plain` / `Query.Plain`; this doc must not cite them as the destination)

## The bug

`docs/architecture/20-query-ir.md` teaches the plain case as an embedding of a prior type, three sites:

- `:55-58` — "The single-rule query is the degenerate case and embeds the conjunctive query unchanged (`Query::single`); a query with empty `interiors` and no rec is today's query plus two empty fields (`… evalQuery_plain`)."
- `:98-103` — "Main is today's query: one head, ≥1 rule, folds, measures, negation. … A query with empty `interiors` and no rec is today's query plus two empty fields … — not an embedding into another type."
- `:1098` — "`evalQuery_plain` is that sentence as a theorem."

## Why it's wrong

Insight 3 (special cases are coordinate artifacts): "today's query" names a PRIOR type and treats the plain case as that type plus two fields — an embedding, exactly what the trailing clause denies. There is one `Query`; empty interiors and `rec: None` is a case of it, and `evalQuery_plain` is that case of `evalQuery`.

## The fix

Per `audit/CONTRACT.md §C7` + §C4 (`evalQuery_plain` / `Query.Plain` die with the Lean sum): "A `Query` with empty `interiors` and `rec: None` is still a `Query`; that is a constructor case of `evalQuery`, not an embedding of a prior type." `Query::single` may stay as a host constructor; the sentence must not call it an embedding of "the conjunctive query." Drop every `evalQuery_plain` citation in this file — do not keep it as the destination name. Main's facts (one head, ≥1 rule, folds, measures, negation) survive under the new framing.

## Acceptance criteria

- [ ] Gone: `rg -in "today's query|evalQuery_plain|embeds the conjunctive" docs/architecture/20-query-ir.md` → no matches.
- [ ] The "Main is: one head, ≥1 rule, folds, measures, negation" FACTS survive under the new framing. Cite `evalQuery`, not a deleted lemma.

## Constraints

- Prose only. Coordinate the phrasing with docs-015 (`70-api.md`) and docs-019 (`75-cpp-lowering.md`) so the three files speak one sentence.
