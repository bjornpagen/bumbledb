# docs-F28: "never `idb`" — retired name taught by negation

Severity: med
Tree: docs (lean/conformance)
Status: OPEN
Source: audit/docs.md F28
Blocked-by: none
Blocks: none

## Bug

`lean/conformance/README.md`:
> Atoms on this arm are `edb` / `interior` (never `idb`, never a
> stored `relation` key).

## Fix (cites CONTRACT C7)

Speak: atoms are `edb` / `interior`; `FieldId` on an interior atom
addresses a derived head position. (After lean-M2, `relation` is
the seeded spelling of the same source — say that, do not deny
`idb`.)

## Acceptance criteria

- [ ] Grep `(?i)\bidb\b` over `lean/conformance/README.md` returns
      empty.
- [ ] `bash scripts/lean.sh` green.
