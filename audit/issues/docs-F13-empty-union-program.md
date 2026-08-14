# docs-F13: "a program whose every disjunct vanishes"

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F13
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/60-validation.md`:
> a program whose every disjunct vanishes is the empty union

## Fix (cites CONTRACT C7)

Speak: a query whose every main disjunct vanishes is the empty union
(`EmptyRuleSet`).

## Acceptance criteria

- [ ] Grep `(?i)\bprogram\b` over `docs/architecture/60-validation.md`
      returns empty.
