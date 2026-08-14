# docs-028: conformance README teaches atom keys by negating `idb`

- **Severity:** medium
- **Tree:** docs (lean README)
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F28
- **Depends on:** none (prose; same file as docs-027 — land together)

## The bug

`lean/conformance/README.md` (reach-case atom section) — "Atoms on this arm are `edb` / `interior` (never `idb`, never a stored `relation` key)."

## Why it's wrong

Correct keys, taught by negating the deleted one (Insight 1): `idb` re-enters the corpus documentation through its own denial. (The "never a stored `relation` key" half also collides with lean-008's ruling that `relation` IS the seeded spelling of the EDB source — the sentence must not imply `relation` is illegal in the corpus at large.)

## The fix

Per `audit/CONTRACT.md §C7`: "Atoms are `edb` / `interior`. `FieldId` on an interior atom addresses a derived head position." State what the keys ARE; drop both negations (docs-027's rewrite states where the `relation` spelling lives).

## Acceptance criteria

- [ ] Gone: `rg -inw 'idb' lean/conformance/README.md` → no matches.
- [ ] The positive key documentation matches the frozen corpus files (spot-check one `reach-*.json`).

## Constraints

- Prose only; land with docs-027.
