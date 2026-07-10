# PRD 07 — Rule execution: one head, one sink

**Depends on:** 05.
**Modules:** `crates/bumbledb/src/exec/run/` (the rule loop), `exec/sink/`
(cross-rule dedup), `api/prepared/` (plan list, view memo), `obs.rs` (per-rule
spans).
**Authority:** `40-execution.md` (sinks, allocation contract, view memo).
**Representation move:** union is not an operator — it is what one sink hearing
several rules *means* under set semantics. No merge node, no concat-then-dedup
pass: the sink's existing seen-set machinery, spanning rules, is the entire
implementation of ∪.

## Context (decided shape)

- Execution runs the prepared query's rules **sequentially** (they share pools
  and the binding-slot scratch; inter-rule parallelism is inter-query
  parallelism's job and stays a non-goal) into **one sink**.
- **Projection sink:** the dedup seen-set spans rules — that is the union.
- **Aggregate sink:** the fold domain is the union of head-projected bindings,
  so the binding seen-set spans rules too; the `distinct_bindings` elision flag
  becomes per-query-shape: rules provably pairwise-disjoint (PRD 08) and each
  internally distinct ⇒ elide; otherwise the seen-set stays. Correct first,
  elided when proven.
- **First-witness suffix skip (D2)** stays per-rule and legal under the
  projection sink only, unchanged: a later rule re-deriving the same head fact
  is absorbed by the seen-set.
- **View memo:** occurrences of one relation in different rules share the image
  Arc by construction (same cache) and share memoized filtered views when their
  resolved filters coincide — the memo key already says so; no new machinery,
  verify and test it.
- Binding-slot layout is per-rule (scopes are per-rule); the head projection
  maps rule slots → head positions, computed at plan validation.
- EXPLAIN reports per-rule node stats plus head-level union stats (emitted vs
  absorbed-by-seen-set per rule).
- The allocation contract is unchanged in kind: scratch is per-prepared-query,
  high-water across all rules.

## Technical direction

1. `api/prepared`: `ExecPlan` holds `Vec<ValidatedPlan>` + one sink config +
   the per-rule head projections.
2. `exec/run`: the rule loop; sink reset happens once per execution, not per
   rule (the spanning is the point).
3. Sinks: seen-set keys are head-projected binding tuples (they already are —
   confirm the key is head-shaped, not rule-slot-shaped, and fix if not: the
   *representation* of the dedup key must be rule-independent).
4. `obs`: `RULE` span (index, emitted, absorbed) under the existing execute
   span.

## Passing criteria

- `[test]` Two rules with overlapping results: the union has no duplicates;
  the same query as two separate executions concatenated by the host *does*
  (the negative control that proves the seen-set spans rules).
- `[test]` Aggregates over overlapping rules: `Count` counts the union (the
  duplicate binding folds once); with provably-disjoint rules and PRD 08
  landed, the elided path returns identical results (differential).
- `[test]` Params bind once and reach all rules; the alloc gate passes with a
  multi-rule prepared query in the measured window.
- `[shape]` No merge/concat/dedup pass exists outside the sinks; `grep` for a
  union-shaped executor node finds nothing.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`40-execution.md`: the rule loop, sink spanning, D2's per-rule statement, the
EXPLAIN additions, allocation-contract wording ("across all rules").
