# docs-005: deleted cap names taught by negation in the IR roster

- **Severity:** medium
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F5
- **Depends on:** none (prose; same file as docs-001..010)

## The bug

`docs/architecture/20-query-ir.md:111` — "There is no `MAX_CTES` / `MAX_INTERIORS` / `TooManyCtes`." and the roster line "`InteriorIdOverflow` (derived-table count does not fit `u32` — id-width, not a product 16; there is no `TooManyCtes`)".

## Why it's wrong

Teaching a name by negation keeps it teachable (Insight 1): every reader now knows `MAX_CTES` and `TooManyCtes`, and CTE re-enters as a bumbledb noun through its own obituary. The regression PIN belongs in the test suite (`tests/adversarial_ir.rs` has it — engine-035 keeps it); the architecture speaks present tense.

## The fix

Per `audit/CONTRACT.md §C7`: "Derived-table count is `u32` width (`InteriorIdOverflow`). There is no interior-count product cap." Delete the retired names from both sites.

## Acceptance criteria

- [ ] Gone: `rg -n 'MAX_CTES|TooManyCtes|MAX_INTERIORS' docs/architecture/20-query-ir.md` → no matches.
- [ ] `InteriorIdOverflow` and the id-width framing unchanged.

## Constraints

- Prose only. The adversarial test pinning `TooManyCtes`'s absence is code and stays (engine-035).
