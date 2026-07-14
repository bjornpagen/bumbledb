# PRD 09 — The lifecycle: transactions, judgment, witnesses, ETL

**Depends on:** 03 (`holds`), 04 (answers as reads).
**Modules:** `lean/Bumbledb/Txn.lean`, `Countermodels.lean`.
**Authority:** `70-api.md`'s transaction semantics (builds their
replacement); the FinalStateView seam (the constitution made judgment's
input a type — this PRD gives the type its theorems); the maintenance
protocol (constitution PRD 20's three witness classes); the migration
recipe's three laws.
**Representation move:** Level 2 — the lifecycle as a state machine
with invariance theorems. The Quint plan, absorbed.

## Context (decided shape)

Definitions:
- `State` = a theory-modeling instance (`holds T I` — 03); `Delta` =
  insert/delete fact multiop as a SET pair (adds, removes);
  `apply : State → Delta → Instance` (final state, order-free by
  construction — the point).
- `commit : State → Delta → Result State Violations` — judge
  `apply`'s result against the theory: accept iff `holds`, else the
  COMPLETE violation set (03's per-statement violation predicate,
  collected — the violations-refactor's spec).
- `Snapshot` = a State + a generation tag; `writeWitnessed` — the
  optimistic protocol: derive from a snapshot, commit iff the
  generation is unmoved, else `GenerationMoved` (a conflict, distinct
  from `Violations` by TYPE — the two failure kinds are different
  constructors, which IS the theorem "generation conflict ≠ dependency
  failure").
- `scanLoad : State → Theory → (Fact → Option Fact) → Result State _`
  — the ETL loop abstractly: export every fact, transform, bulk-judge
  under the target theory.

Theorems:
1. `final_state_judgment_order_free` — any two op sequences with equal
   `apply` results receive identical verdicts (insert/delete order
   inside a transaction cannot change validity; the FinalStateView
   law; Bridge: `judgment::FinalStateView`, the sole judge input).
2. `committed_states_model` — every committed state satisfies `holds`
   (the "free lunches" law: queries may assume every dependency; the
   induction over commit).
3. `rejection_is_complete` — a rejected delta's violation set contains
   EVERY violated statement of the final state (the citation-refactor
   spec; Bridge: `Violations`, sorted+deduped+nonempty).
4. `witness_conflict_distinct` — `writeWitnessed` never converts a
   generation move into a violation or vice versa (by construction —
   state it anyway; it is the API's contract sentence).
5. `snapshot_reads_one_state` — every read (04's answers) is a
   function of one `State` (the signature-level fact, stated).
6. `derived_soundness_vs_freshness` — the maintenance protocol's
   division of authority: a containment-constrained derived relation
   is SOUND in every committed state (from 2), while freshness is not
   a property of any committed state (countermodel: a committed state
   with a stale-but-sound derived fact — the host-discipline gap,
   formal).
7. `etl_identity` — `scanLoad` with the identity transform into the
   SAME theory reproduces the state; with a transform into a new
   theory, the loaded state `holds` the NEW theory or the load
   rejects (the migration recipe's third law: "a migration that lands
   is already valid").
8. Countermodel: `per_op_judgment_wrong` — a delta that is valid as a
   final state but transiently violates mid-sequence (delete parent,
   delete child vs child, parent) — why judgment is final-state and
   per-operation checking would reject valid transactions.

## Technical direction

Keep the machine tiny: no interleaving, no concurrency beyond the
generation tag (single-writer is the engine's law — model the
PROTOCOL, not threads). `Result` as a two-constructor sum. Item 3
reuses 03's violation predicates; do not redefine. The crash/
durability axis is REFUSED here (covenant refusals) — one module-doc
sentence points at the crashpoint estate.

## Passing criteria

- `[shape]` All eight items checked; zero sorry/axioms;
  `scripts/lean.sh` 0.
- `[shape]` `Violations` and `GenerationMoved` are distinct
  constructors of the commit result (grep the type).
- `[shape]` The per-op countermodel present — the FinalStateView
  seam's formal justification.
- `[gate]` CI green.

## Doc amendments

None yet — PRD 12 thins `70-api.md`'s semantic prose against these
names.
