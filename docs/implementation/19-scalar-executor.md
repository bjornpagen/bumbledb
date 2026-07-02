# PRD 19 — The Scalar Executor

Authority: `docs/architecture/30-execution.md` (execution core, dynamic covers,
residual step, Counters genericity), paper §3.3 Fig. 5.

## Purpose

The recursive Free Join executor, scalar form (batch size 1) — the semantics reference
that PRD 21's vectorized path must match exactly.

## Technical direction

- `exec::run`. `execute<C: Counters, S: Sink>(plan: &ValidatedPlan, sources:
  &mut [ColtRoot], binding: &mut Bindings, sink: &mut S, counters: &mut C)`.
  **Everything is a monomorphized generic — no `dyn` anywhere** (doc rule).
- `Counters` trait defined here with a `NoopCounters` zero-sized impl (every method
  `#[inline] {}`); the counting impl arrives in PRD 24. Counter calls at: node entry,
  cover choice, probe, residual eval, sink emit.
- `Bindings`: dense VarId-indexed `[u64]` slot array + a set/unset epoch discipline
  (unset = not bound; use an epoch word per slot rather than Option — branch-light).
- Node loop, per the doc: choose cover from the node's cover set by
  `key_count()` (smallest Exact, else smallest Estimate — PRD 18 labels); iterate the
  chosen cover (`(key words, child)` pairs); write binding slots; probe sibling
  subatoms (`get` with key words gathered from slots); replace each occurrence's
  current-node ref (an undo journal: `Vec<(occ, prev NodeRef)>` per depth, arena
  scratch); evaluate the node's residual comparisons on slots; recurse; restore on
  backtrack.
- Zero-binding occurrences and nullary relations: covered by the plan formalism
  (empty var sets) — must fall out, not be special-cased; write the test first.
- Early-exit signal: `Sink::emit(&Bindings) -> Flow` where
  `Flow::{Continue, SkipSuffix}` — the sink-driven subtree skip hook (the projection
  sink returns SkipSuffix per its rules in PRD 20; this PRD implements the unwind:
  on SkipSuffix, return from the current node level up to the node that bound the
  last projection-relevant... **no** — keep exactly the doc's rule: SkipSuffix
  propagates up through nodes that bound no sink-relevant variable; the plan
  precomputes per-node "sink-relevant" bits from the find-var set. Implement precisely
  that and nothing cleverer).

## Non-goals

Sinks beyond the trait + a test CountingSink (PRD 20 ships the real ones). Batching.
Access-path dispatch (PRD 23).

## Passing criteria

- Unit tests against a naive nested-loop oracle written in the test (10-line
  interpreter over decoded facts): equality of binding sets on — the clover fixture
  (paper Fig. 4 data), a chain query, a self-join (grandparent over OrgParent), a
  triangle (cyclic), zero-binding atom gating, empty relations, duplicate-heavy skew
  (the paper's clover instance where factoring matters); residuals filter across
  atoms correctly; dynamic cover picks the forced small side on a constructed skew
  fixture (assert via a test Counters recording choices); backtrack restores sources
  (run two queries sequentially on shared roots).
- Global commands green.
