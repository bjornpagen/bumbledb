# docs-F20: lowering contract denies the deleted Program layout instead of stating Query

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F20
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/75-cpp-lowering.md`:
> There is no output-last predicate slot and no `output =
> recs.length`.
> interiors then rec then main — no output-last predicate slot

## Fix (cites CONTRACT C7)

Describe `Query` field-for-field: wire shape is
`QueryIr { interiors, rec, head, rules }`; evaluation order is
interiors, optional rec, main; main is `head` + `rules`, not an
output index into a predicate table. Drop the denials.

## Acceptance criteria

- [ ] Grep `output-last|recs\.length` over
      `docs/architecture/75-cpp-lowering.md` returns empty.
