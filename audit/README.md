# audit/ — the representation-finish campaign

The Program → Query cutover landed (main `fef913b6`). This folder
judged the cut against one law — representation over control flow.
Verdict: **the cut renamed Program but kept its coordinate system.**
Recursion is still an `Option`/bool/length-pun instead of a sum,
proofs are validated then discarded, two denotations and two builders
survive, and the docs still teach the deleted language.

Fixes are product-code commits that cite issue ids. Do not assign
`DUPLICATE` or `WONTFIX` ids. Ledger: `issues/INDEX.md`.

## Files

| File | What it is |
|---|---|
| `00-representation-is-the-essence.md` | Doctrine. Required reading. SPOV 1–3, Insights 1–16. |
| `CONTRACT.md` | Pinned decisions C1–C8. The only authority a fix implements. Proposed C9 (sealed schema sums) is recommended — pin it before schema fanout. Do not mint a CONTRACT C10 for corruption variants (`capacity-laws` already uses C10 for rays). |
| `lean.md` / `engine.md` / `sdks.md` / `docs.md` | Wave-1 dumps (18 / 40 / 21 / 28). Historical; issues/ is the work list. |
| `lean-rest.md` / `sdk-rest.md` / `plan-exec.md` / `storage-schema.md` / `bench.md` | Wave-2 dumps. Historical; issues/ is the work list. |
| `issues/` | One file per finding (188). OPEN files are `NNN-id-slug.md` so `ls audit/issues` is the work queue. Ids (`lean-019`, `engine-001`, …) stay stable. DUPLICATE/WONTFIX are `9xx-…` after the OPEN sequence. |
| `issues/INDEX.md` | Work-order table (topological) plus status, dependencies, tree appendix. |

## Gate (before the first product-code fix)

- [x] Every wave-1 finding maps to one issue file (counts 18/40/21/28).
- [x] Wave-2 dumps exploded (lean-rest 4, sdk-rest 7, plan-exec 24, storage-schema 25, bench 12).
- [x] Duplicates are stubs (`DUPLICATE(id)`); C5 refusals and essential identities are `WONTFIX` or `OPEN (scoped)`.
- [x] INDEX.md ids match filenames; dependencies are a DAG; every OPEN issue has a seq (`ls audit/issues` order).
- [x] Final-pass validation landed: Fix rewrites; four new findings (sdk-030, docs-030, exec-017, schema-011); lean-018 demoted DUPLICATE(lean-001). Ledger is DAG-complete; first product-code fix is still `lean-019`.
- [ ] Fixer confirms the issue's Fix section cites CONTRACT.md C1–C8 (pin C9 in CONTRACT before schema fanout; do not invent a C10 — `capacity-laws` already uses C10 for rays).
- [ ] Acceptance criteria are mechanical (the issue file's checkboxes).

## Fanout (see INDEX.md work order)

Work queue: `ls audit/issues` and the INDEX **Work order (topological)** table. Start at the top and go down — no waves to think about. First fix is seq **001 = lean-019** (census token; un-reds `scripts/lean.sh`). Co-landing clusters are adjacent (one commit). `9xx-` files are DUPLICATE/WONTFIX — do not assign them.

Each fix commit names its issue ids and flips them to `FIXED(<sha>)` in the issue file and in INDEX.md.

## Invariants that never move

1. **C1:** hostile boundary (`ir.rs::Query`, corpus JSON, C ABI `bdb_query`, TS wire type) stays shape-unchanged. 268 cases do not regenerate.
2. **C5 (R-DENSE):** Lean ids stay dense `Nat`s; environments stay total. Fin-telescope/`Vector` rewrites are refused. Dual coordinates still die.
3. Denotations, walls, OPEN refusals, ledger, budget values, locked names (`DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`) unchanged.
4. Green = `scripts/lean.sh` + `scripts/check.sh` plus the issue's tree-local suite. Assertions are never weakened.
