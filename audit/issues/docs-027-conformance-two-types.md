# docs-027: conformance README teaches two query types in the third oracle's corpus

- **Severity:** high
- **Tree:** docs (lean README)
- **Status:** OPEN
- **Source:** audit/docs.md F27
- **Depends on:** lean-008 (one decoder; `CQuery`/`plainQuery` deleted) — this README describes the decode pipeline

## The bug

`lean/conformance/README.md:297` — "A reach case carries a Query with `interiors` / `rec` / main `rules` (`Bumbledb/Query/Syntax.lean`) instead of a CQuery." The README presents the interchange as two types: `CQuery` for `seeded-*.json`, `Query` for `reach-*.json`.

## Why it's wrong

Dual representation taught as the corpus's shape (Insight 2): the architecture is one `Query`, and "instead of a CQuery" teaches the split as current. The split is real TODAY only because of lean-008's defect; once the one decoder lands, this README would be false — and it is the document conformance-case authors read.

## The fix

Per `audit/CONTRACT.md §C7` + §C8, after lean-008: "Every case carries a `Query`. Plain cases have empty `interiors` and `rec: null` (their atoms use the `relation` spelling of the EDB source). Reach cases fill those fields (atoms spell `edb`/`interior`). One type, one decoder." Delete every `CQuery`/`plainQuery` mention.

## Acceptance criteria

- [ ] Gone: `rg -n 'CQuery|plainQuery' lean/conformance/README.md` → no matches.
- [ ] Case counts (246 seeded + 22 reach = 268) and JSON key documentation match the frozen corpus exactly.
- [ ] `./scripts/lean.sh` green (README changes can't break it, but the fixer confirms the described pipeline is the shipped one).

## Constraints

- Blocked by lean-008. Corpus frozen — the README describes, never prescribes regeneration.
