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
| `CONTRACT.md` | Pinned decisions C1–C8. The only authority a fix implements. |
| `lean.md` / `engine.md` / `sdks.md` / `docs.md` | Wave-1 dumps (18 / 40 / 21 / 28). Historical; issues/ is the work list. |
| `issues/` | One file per finding. Naming: `lean-001`, `engine-001`, `sdk-001`, `docs-001`. |
| `issues/INDEX.md` | Fanout ledger: status, dependencies, waves, clusters. |

## Gate (before the first product-code fix)

- [x] Every wave-1 finding maps to one issue file (counts 18/40/21/28).
- [x] Duplicates are stubs (`DUPLICATE(id)`); C5 refusals are `WONTFIX` or `OPEN (scoped)`.
- [x] INDEX.md ids match filenames; dependencies are a DAG; every OPEN issue has a wave.
- [ ] Fixer confirms the issue's Fix section cites CONTRACT.md C1–C8.
- [ ] Acceptance criteria are mechanical (the issue file's checkboxes).

## Fanout (see INDEX.md for clusters)

- **Wave 0:** `lean-019` (census token; un-reds `scripts/lean.sh`).
- **Wave 1:** the sums — lean-001+002 ∥ engine-001+002+015+023 ∥ engine-005+006 ∥ sdk-001+002. Docs with no code deps may start.
- **Wave 2:** everything that collapses once the sums exist.
- **Wave 3:** sdk-018 compile-fail suite; docs-002/017 after engine-041; docs-012/027 after lean-008.

Each fix commit names its issue ids and flips them to `FIXED(<sha>)` in the issue file and in INDEX.md.

## Invariants that never move

1. **C1:** hostile boundary (`ir.rs::Query`, corpus JSON, C ABI `bdb_query`, TS wire type) stays shape-unchanged. 268 cases do not regenerate.
2. **C5 (R-DENSE):** Lean ids stay dense `Nat`s; environments stay total. Fin-telescope/`Vector` rewrites are refused. Dual coordinates still die.
3. Denotations, walls, OPEN refusals, ledger, budget values, locked names (`DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`, `DEFAULT_REACH_ROUNDS`) unchanged.
4. Green = `scripts/lean.sh` + `scripts/check.sh` plus the issue's tree-local suite. Assertions are never weakened.
