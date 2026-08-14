# err-006: "program" vocabulary in `error.rs` / `obs.rs` / prepare glue

- **Severity:** low
- **Tree:** err
- **Status:** OPEN
- **Source:** audit/storage-schema.md F22
- **Depends on:** none
- **Conflicts with:** none

## The bug

C7: no `program` on live data. These trees have no live Program/Idb/stratum fields. Comments still teach the deleted word:

- `error.rs:603-604` — "hand-written 2+-rule program"
- `obs.rs:155-156` — "legal program"
- `api/db/prepare.rs:14-15` — "A query whose `rec` is `None` never enters the reach driver" (restates Query's `Option<Rec>` as the API's explanation of reach)

## Why it's wrong

Insight 1 — leftover vocabulary steers the next reader toward the old coordinate. No control-flow diversion (unlike engine-011's `stats.strata`), so this is low.

## The fix

Per `audit/CONTRACT.md` §C7. "query" / "rule set." Prepare glue: after engine-001, "a Reach pipeline runs the rec; a Cq pipeline does not" — not `rec is None`. Until engine-001 lands, say "a query without a rec never enters the reach driver" without naming the Option.

## Acceptance criteria

- [ ] Gone: `rg -in 'program' crates/bumbledb/src/error.rs crates/bumbledb/src/obs.rs crates/bumbledb/src/api/db/prepare.rs` → no Query-IR "program" (keep "programmer-invariant").
- [ ] Gone: `rg -n 'rec is \`None\`' crates/bumbledb/src/api/db/prepare.rs`.
- [ ] No product-code behavior change.
- [ ] Green: `./scripts/check.sh` (comment-only).

## Constraints

- Prose only. `CountAcrossRules` variant name and semantics locked. Do not touch Query IR types (C1).
