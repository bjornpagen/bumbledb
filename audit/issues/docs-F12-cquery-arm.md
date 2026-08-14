# docs-F12: validation chapter teaches a "CQuery arm"

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F12
Blocked-by: lean-M2 (the one-decoder fact this describes)
Blocks: none

## Bug

`docs/architecture/60-validation.md`:
> the CQuery arm (`seeded-*.json`) is unchanged.

## Fix (cites CONTRACT C7, C4)

Speak: seeded cases are `Query` values (`interiors = []`,
`rec = none`) in `seeded-*.json`; reach cases are `Query` values
with interiors/rec in `reach-*.json`. One type, one decoder
(lean-M2). Corpus files unchanged.

## Acceptance criteria

- [ ] Grep `CQuery` over `docs/architecture/` returns empty.
- [ ] `bash scripts/lean.sh` green.
