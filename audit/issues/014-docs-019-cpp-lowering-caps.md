# docs-019: 75-cpp-lowering — "today's query" + `MAX_CTES`/`MAX_PREDICATES` by negation

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F19
- **Depends on:** none (prose; same file as docs-020)

## The bug

`docs/architecture/75-cpp-lowering.md:430-431` — "Empty `interiors` and `rec: None` is today's query plus two empty fields. Caps: `MAX_RULES = 16` per rule-list (rec pooled); **no `MAX_CTES` / `MAX_PREDICATES`**".

## Why it's wrong

Two findings in one sentence: the "today's query" embedding (docs-004's defect, Insight 3) and deleted cap names taught by negation (docs-005's defect, Insight 1) — in the lowering CONTRACT that SDK implementers treat as normative.

## The fix

Per `audit/CONTRACT.md §C7`: "`Query { interiors, rec: Option<Rec>, head, rules }`. `MAX_RULES = 16` per list (rec pooled). No interior-count cap." Phrase-align with docs-004/docs-015.

## Acceptance criteria

- [ ] Gone: `rg -in "today's query|MAX_CTES|MAX_PREDICATES" docs/architecture/75-cpp-lowering.md` → no matches.
- [ ] `MAX_RULES = 16` and the pooling claim unchanged.

## Constraints

- Prose only. Note sdk-012 deletes the C++ SDK's own invented caps — if it lands first, this chapter's cap section describes the engine's caps only (it already should).
