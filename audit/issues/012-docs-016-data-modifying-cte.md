# docs-016: write idioms taught as "data-modifying CTEs"

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F16
- **Depends on:** none (prose; same file as docs-015/017/18; sibling docs-022 fixes the cookbook copy)

## The bug

`docs/architecture/70-api.md:663` — "everything SQL spells with data-modifying CTEs — must read on a snapshot first"; `:711` — "the data-modifying-CTE shapes with the premises witnessed instead of locked."

## Why it's wrong

CTE is not a bumbledb word (Insight 1): the idioms are host writes (insert-select, update-where) over prepared queries, and naming them by SQL's coordinate teaches SQL's model as ours.

## The fix

Per `audit/CONTRACT.md §C7`: "Insert-select / update-where: query on a snapshot, then `write_from` witnessing the snapshot." SQL's data-modifying `WITH` may appear at most as an explicitly-external analogy ("what SQL spells with…") — prefer deleting it; it must not be the NAME of the idiom.

## Acceptance criteria

- [ ] Gone: `rg -in 'data-modifying' docs/architecture/70-api.md` → no matches (or only inside an explicit "SQL calls this…" aside if the fixer keeps the analogy — prefer none).
- [ ] `write_from` / snapshot-witness semantics unchanged.

## Constraints

- Prose only; align wording with docs-022 (cookbook).
