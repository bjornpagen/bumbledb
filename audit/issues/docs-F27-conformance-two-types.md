# docs-F27: conformance README teaches two query types (`CQuery` vs `Query`)

Severity: high
Tree: docs (lean/conformance)
Status: OPEN
Source: audit/docs.md F27
Blocked-by: lean-M2 (the decoder unification it must describe)
Blocks: none

## Bug

`lean/conformance/README.md`:
> A reach case carries a Query with `interiors` / `rec` / main
> `rules` … instead of a CQuery.

## Fix (cites CONTRACT C7, C4)

Speak: every case carries a `Query`. Plain cases have empty
`interiors` and `rec: null`; reach cases fill those fields. One
decoder (lean-M2). The corpus files do not change.

## Acceptance criteria

- [ ] Grep `CQuery` over `lean/conformance/README.md` returns empty
      (or only as the find-shape type if lean-M2 keeps it for
      aggregate heads, described as such).
- [ ] `bash scripts/lean.sh` green.
