# docs-008: "There is no separate program renderer" — denial that teaches the deleted noun

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F8
- **Depends on:** none (prose; same file as docs-001..010; the renderer STRINGS quoted nearby are engine-033's)

## The bug

`docs/architecture/20-query-ir.md:969-970` — "There is no separate program renderer."

## Why it's wrong

Same negation pattern (Insight 1): a present-tense renderer section mentions the Program renderer even to deny it, so the reader learns the deleted architecture as the thing this one is not.

## The fix

Per `audit/CONTRACT.md §C7`: delete the sentence; the section already says what IS: "`ir::render` prints a `Query`: interiors, optional rec, then bare main rules — total, and golden-pinned."

## Acceptance criteria

- [ ] Gone: `rg -n 'program renderer' docs/architecture/20-query-ir.md` → no matches.
- [ ] The golden-pinned/total claims unchanged.

## Constraints

- Prose only. The quoted output strings `interior p{id}` / `recursive p{id}` at `:968` change when engine-033 lands — this doc line updates in ENGINE-033's change, not here (avoid a doc that contradicts shipping output).
