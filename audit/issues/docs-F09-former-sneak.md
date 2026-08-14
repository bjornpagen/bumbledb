# docs-F09: "(the former named-head sneak)" — history in a present-tense rule

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F9
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/20-query-ir.md`:
> A named head without either keyword is a compile error (the former
> named-head sneak).

## Fix (cites CONTRACT C7)

Speak: a named head requires `interior` or `recursive`; bare rules
are the main query.

## Acceptance criteria

- [ ] Grep `former named-head sneak|former` over
      `docs/architecture/20-query-ir.md` returns no history framing
      on this rule.
