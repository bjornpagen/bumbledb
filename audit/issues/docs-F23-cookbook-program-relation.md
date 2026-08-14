# docs-F23: cookbook recipe 30's example relation is named `Program`

Severity: low
Tree: docs (couples ts/cpp test fixtures)
Status: OPEN
Source: audit/docs.md F23 + wave-2 coupling
Blocked-by: none
Blocks: none

## Bug

`docs/cookbook.md` recipe 30: `relation Program { … }`,
"`Program(grp) -> Program` — one program per group". After the IR
type `Program` was deleted, a worked example named `Program` collides
with the deleted coordinate.

Coupled fixtures that mirror the recipe (rename together):
`ts/test/cookbook.test.ts:1162-1213` (relation `Program`,
`minted.program`, …) and `cpp/tests/cookbook/r30_keyed_read.cc`
(`bdb::relation<"Program", ProgramRow>`, `program_grp_key`, …).

## Fix (cites CONTRACT C7)

Rename the example relation (`Course`, `Track`, or `Offering`) in
the recipe AND both test fixtures in one commit; the law reads "one
row per group".

## Acceptance criteria

- [ ] Grep `Program` over `docs/cookbook.md` returns empty.
- [ ] The coupled ts and cpp cookbook tests renamed and green
      (`pnpm test`, cpp suite); assertions unchanged in strength.
