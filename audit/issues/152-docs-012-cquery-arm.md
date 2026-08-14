# docs-012: validation chapter teaches a "CQuery arm" beside the reach arm

- **Severity:** high
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F12
- **Depends on:** lean-008 (one decoder, `CQuery` deleted) — this doc must describe the post-fix corpus truthfully

## The bug

`docs/architecture/60-validation.md:107` — "the CQuery arm (`seeded-*.json`) is unchanged."

## Why it's wrong

Two languages for one corpus (Insight 2): the architecture's type is `Query`; teaching a "CQuery arm" versus a "reach arm" presents the conformance lane as two query types, which is exactly the dual representation lean-008 deletes. Seeded cases ARE queries with empty interiors and no rec.

## The fix

Per `audit/CONTRACT.md §C7` + §C8: "Seeded cases are `Query` values (`interiors = []`, `rec = none`) in `seeded-*.json`. Reach cases are `Query` values with interiors / rec in `reach-*.json`. One type, one decoder." (Corpus files themselves are FROZEN — lean-008's constraint — so the sentence describes spellings of one type, not two types.)

## Acceptance criteria

- [ ] Gone: `rg -n 'CQuery' docs/architecture/60-validation.md` → no matches.
- [ ] The 246-seeded/22-reach counts and case-file names unchanged.

## Constraints

- Blocked by lean-008 (otherwise the doc describes code that doesn't exist yet). Prose only; corpus untouched.
