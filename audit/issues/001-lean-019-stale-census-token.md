# lean-019: Bridge.lean cites deleted `translate/program.rs` — the census gate is RED on main

- **Severity:** high
- **Tree:** lean (Bridge/census)
- **Status:** FIXED(93f7640b)
- **Source:** adversarial pass (not in audit/lean.md)
- **Depends on:** none — **fix FIRST**: every other issue's green definition includes `./scripts/lean.sh`, whose census battery cannot pass until this lands.

## The bug

`bash scripts/spec-census.sh` fails on main today:

```
spec-census: FAIL — path 'crates/bumbledb-bench/src/translate/program.rs' (token 'translate_query (crates/bumbledb-bench/src/translate/program.rs)') does not exist
```

The stale token:

```568:571:lean/Bumbledb/Bridge.lean
  .row @Query.evalLinearReach_eq_lfp `Bumbledb.Query.evalLinearReach_eq_lfp
    "The executable reach lists exactly reachDen."
    "translate_query (crates/bumbledb-bench/src/translate/program.rs)"
    "lean/conformance/cases",
```

`crates/bumbledb-bench/src/translate/` contains `builder.rs`, `reach.rs`, `query.rs`, `goldens.rs`, `tests.rs`, `types.rs` — no `program.rs`. `translate_query` lives in `translate/reach.rs`.

## Why it's wrong

The cutover renamed the translator module and did not move the Bridge mechanism token with it — a cross-tree citation that no longer parses is drift already delivered (Insight 1), and a red gate on main means every "green" claim in this campaign is currently unverifiable (the census exists precisely so citations cannot rot silently).

## The fix

Per `audit/CONTRACT.md §C8` (tokens move WITH renames): retarget the one token to `"translate_query (crates/bumbledb-bench/src/translate/reach.rs)"`. Verify with `rg -w translate_query crates/bumbledb-bench/src/translate/reach.rs` before committing. If engine-021 later moves/renames the translator entry, the token moves again in THAT commit.

## Acceptance criteria

- [x] `bash scripts/spec-census.sh` exits 0.
- [x] `./scripts/lean.sh` exits 0 (build + battery + census + conformance + comparator).
- [x] Gone: `rg -n 'translate/program\.rs' lean crates docs` → no matches.
- [x] No other token, ledger row, theorem, or assertion touched (one-line diff).

## Constraints

- Semantics identical; do NOT weaken the census or exclude the row. No Program vocabulary (the fix DELETES a `program.rs` mention). Locked names untouched.
