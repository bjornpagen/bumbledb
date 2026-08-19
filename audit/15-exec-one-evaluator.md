# 15 — The predicate walk is shared; the interpreters are not

- **Status:** OPEN (verified 2026-08-19 17:10 EDT — the shared walk is
  referenced from five-plus modules; the tree is hot).
- **Severity:** should-fix.
- **Supersedes:** EXEC-01; carries EXEC-02 as its tail.

## Principle

`proposals/exec-representation.md`: one predicate evaluator, callers supply
an `Operands` provider; `FilterPredicate` matched exhaustively in **at most
two** modules (the evaluator and the planner's selectivity reader). Sharing
`holds()` as a subroutine while keeping per-site wrappers is still N
interpreters with a common helper — a new predicate kind still costs edits
at every entry.

## Evidence

- `Operands` + `holds()` exist (`image/view/eval.rs`); entered from
  `image/view/apply.rs`, `exec/dispatch/key_probe_fact.rs`,
  `exec/verdict.rs`, `plan/ground/evaluate.rs`, `api/prepared/bind.rs`,
  plus `plan/selectivity.rs` as the reader.
- The `Placed*` family is gone (landed); the consolidation of the entry
  points is the remaining half.

## The fix

1. One evaluator module owns the walk **and** the entry: callers construct
   their `Operands` (image columns, fact bytes, binding slots) and call the
   one entry; no site keeps its own predicate loop or per-kind dispatch.
2. `plan/selectivity.rs` remains the one other exhaustive
   `FilterPredicate` match (it reads statistics, not truth values).
3. **Tail (EXEC-02, later):** `NodeScratch`'s residual-source copies beside
   `NodePrecompute` — precompute owns residual metadata; scratch holds only
   transient state. Do not touch the recorded kind-grouped batching
   representation or the `NodeScratch` extraction refusal.

## Acceptance

- Exhaustive `FilterPredicate` matches exist in exactly two modules
  (evaluator, selectivity) — grep gate.
- Scenario lanes byte-identical; kernel purity signature untouched.
