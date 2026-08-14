# docs-002: the IR chapter teaches main as "one anonymous predicate"

- **Severity:** high
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F2
- **Depends on:** engine-041 (the `Predicate` → `Signature` type rename; this doc cites the type by name)

## The bug

`docs/architecture/20-query-ir.md:60-70`:

> **Main defines one anonymous predicate; rules derive it.** … sealed in the witness (`ir/validate`'s `Predicate`) … The predicate is anonymous and engine-internal

## Why it's wrong

Predicate-as-query-head is the old Program coordinate — each predicate in a rule program (Insight 1). Main is the query's ANSWER SHAPE: `head` + `rules`; the sealed object is the main signature (arity, types, folds), not a Datalog predicate. The doc's word is currently propped up by the engine type's leftover name (engine-041).

## The fix

Per `audit/CONTRACT.md §C7`: "Main owns the answer signature (arity, types, folds), sealed once at validation. Interiors and the rec are derived tables addressed by `InteriorId`." Cite the renamed type (`ir/validate`'s `Signature` — after engine-041). Do not call main a predicate anywhere in the section.

## Acceptance criteria

- [ ] Gone: `rg -n 'anonymous predicate|The predicate is' docs/architecture/20-query-ir.md` → no matches.
- [ ] The type citation matches the code post-engine-041 (`rg -n "ir/validate.*Predicate" docs/architecture` → no matches).
- [ ] Sealing-time claims (once at validation, buffer-typing role) unchanged.

## Constraints

- Blocked by engine-041 (land after, so the cited symbol exists). Prose only; no Program vocabulary.
