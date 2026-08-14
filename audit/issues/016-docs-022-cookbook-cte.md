# docs-022: cookbook teaches insert-select as "the data-modifying CTE"

- **Severity:** medium
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F22
- **Depends on:** none (prose; same file as docs-023)

## The bug

`docs/cookbook.md:967` — "**Insert-select**: query source answers, insert the derived facts — the data-modifying CTE with its premises witnessed instead of locked."

## Why it's wrong

docs-016's CTE import, in the cookbook's write-idiom teaching (Insight 1): the recipe reader learns SQL's coordinate as the idiom's name.

## The fix

Per `audit/CONTRACT.md §C7`: "Insert-select: query source answers, insert the derived facts, `write_from` witnessing the snapshot." Drop the CTE clause; phrase-align with docs-016.

## Acceptance criteria

- [ ] Gone: `rg -in 'data-modifying' docs/cookbook.md` → no matches.
- [ ] The recipe's code and semantics unchanged.

## Constraints

- Prose only.
