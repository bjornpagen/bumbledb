# docs-029: cookbook says "finished table, not a second SCC"

- **Severity:** low
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** adversarial pass (not in audit/docs.md — F21's pattern, in the cookbook)
- **Depends on:** none (prose; same file as docs-022/023)

## The bug

`docs/cookbook.md:1119` — "finished table, not a second SCC." (the named-interior-of-a-finished-rec teaching, phrased against the condensation coordinate).

## Why it's wrong

Insight 1: same defect as docs-021's README sites — the refused future is defined as "a second SCC", keeping Tarjan condensation as the cookbook reader's frame. The wall is "not a second rec".

## The fix

Per `audit/CONTRACT.md §C7`: "…a finished table, not a second rec." Phrase-align with docs-021's rewrite of the same wall.

## Acceptance criteria

- [ ] Gone: `rg -in 'scc' docs/cookbook.md` → no matches.
- [ ] The recipe's refusal semantics (named interior of a finished rec is OPEN-refused as a second rec) unchanged.

## Constraints

- Prose only.
