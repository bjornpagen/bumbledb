# docs-F18: "stays a runtime check" as the last word on `ForeignPreparedQuery`

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F18
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/70-api.md`:
> same-schema/different-environment confusion stays a runtime check
> (`ForeignPreparedQuery`).

## Fix (cites CONTRACT C7)

Per the pinned ruling, this is documented as essential runtime
identity with the horizon representation NAMED: cross-environment is
a process-distinct instance fact; the horizon fix is branding
`PreparedQuery` with the preparing environment (a
generation/instance witness in the type). The sentence states the
essential-complexity ruling and the horizon — "stays a runtime
check" is not the last word.

## Acceptance criteria

- [ ] The passage names (a) why it is essential today and (b) the
      horizon representation; `ForeignPreparedQuery` behavior text
      unchanged.
