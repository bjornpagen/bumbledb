# docs-009: "the former named-head sneak" — history in a present-tense grammar rule

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F9
- **Depends on:** none (prose; same file as docs-001..010)

## The bug

`docs/architecture/20-query-ir.md:1082` — "A named head without either keyword is a compile error (the former named-head sneak)."

## Why it's wrong

"Former sneak" is history riding a normative sentence (Insight 1): the rule needs no origin story, and the parenthetical implies a reader should know the pre-cut behavior to understand the current one.

## The fix

Per `audit/CONTRACT.md §C7`: "A named head requires `interior` or `recursive`. Bare rules are the main query." Drop the parenthetical.

## Acceptance criteria

- [ ] Gone: `rg -n 'former named-head sneak|the former' docs/architecture/20-query-ir.md` → no matches on this sentence.
- [ ] The compile-error claim and the `interior mid(x) | Edge(src: x);` example unchanged.

## Constraints

- Prose only.
