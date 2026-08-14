# docs-013: "a program whose every disjunct vanishes" in the validation chapter

- **Severity:** high
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F13
- **Depends on:** none (prose; same file as docs-012/014 — one fixer may take 60-validation.md)

## The bug

`docs/architecture/60-validation.md:206` — "a program whose every disjunct vanishes is the empty union"; also `:799` "**multi-rule programs** at arm" and `:858` "multi-rule programs replayed engine-vs-naive".

## Why it's wrong

`Program` is deleted (Insight 1); this is the empty-main-rule-set case of a `Query`, and the algebra-family sentences describe rule-lists of queries.

## The fix

Per `audit/CONTRACT.md §C7`: "a query whose every main disjunct vanishes is the empty union (`EmptyRuleSet`)"; ":799/:858 → multi-rule queries". Sweep the rest of the file for "program"-naming-a-query in the same pass.

## Acceptance criteria

- [ ] Gone: `rg -inw 'program|programs' docs/architecture/60-validation.md` → no matches naming a query.
- [ ] `EmptyRuleSet` and all algebra/lock names unchanged.

## Constraints

- Prose only. Note: engine-023 proposes deleting the `PreparedBody::Empty` VARIANT while keeping the statically-empty BEHAVIOR — the doc sentence describes behavior and stays true either way.
