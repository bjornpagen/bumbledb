# docs-001: "Hand-written multi-rule programs" in the normative IR chapter

- **Severity:** high
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F1
- **Depends on:** none (prose; parallel-safe within `docs/architecture/20-query-ir.md` — coordinate with docs-002..010 which edit the same file; one fixer may take all of 20-query-ir)

## The bug

`docs/architecture/20-query-ir.md:298`:

> **Hand-written multi-rule programs keep the head-projection law**

## Why it's wrong

`Program` is the deleted IR type; a present-tense architecture chapter teaching "program" as the name of a rule-list keeps the old coordinate system alive in the exact document new contributors read first (Insight 1: the doc IS a representation, and it currently represents the deleted system). A hand-written rule-list is a `Query`'s main (or an `Interior`, or the rec pool).

## The fix

Per `audit/CONTRACT.md §C7`: rewrite as "Hand-written multi-rule **queries** keep the head-projection law" (or "hand-written main rule-lists" where the sentence is specifically about main). Sweep the whole sentence's paragraph for agreeing pronouns.

## Acceptance criteria

- [ ] Gone: `rg -in 'multi-rule program' docs/architecture/20-query-ir.md` → no matches.
- [ ] The surrounding technical claims (head-projection law, its lock names) are UNCHANGED — vocabulary only.
- [ ] No code changes in this issue.

## Constraints

- Prose only; semantics of the documented law untouched. No Program vocabulary anywhere in the replacement.
