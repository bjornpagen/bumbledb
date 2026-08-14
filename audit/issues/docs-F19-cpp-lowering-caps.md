# docs-F19: cpp-lowering contract — "today's query" + deleted cap names

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F19
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/75-cpp-lowering.md`:
> Empty `interiors` and `rec: None` is today's query plus two empty
> fields. Caps: `MAX_RULES = 16` per rule-list (rec pooled); **no
> `MAX_CTES` / `MAX_PREDICATES`**

## Fix (cites CONTRACT C7)

Speak: `Query { interiors, rec: Option<Rec>, head, rules }`.
`MAX_RULES = 16` per list (rec pooled). No interior-count cap. No
retired names by negation.

## Acceptance criteria

- [ ] Grep `today's query|MAX_CTES|MAX_PREDICATES` over
      `docs/architecture/75-cpp-lowering.md` returns empty.
