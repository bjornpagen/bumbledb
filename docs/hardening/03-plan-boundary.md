# PRD 03 — The plan boundary rejects what the executor cannot run

Findings fixed (docs/audit/plan.md): **MEDIUM** "validate() accepts plans that
silently drop a zero-variable occurrence (nonemptiness gate)"; **LOW**
"Subatoms referencing an occurrence outside the normalized query are not
rejected — executor panics instead"; the check_selections-tautology and
slot-comment NOTEs.

## Purpose

`ValidatedPlan` exists because `FjPlan` is plain data anyone can construct —
the boundary's contract is that everything it seals, the executor computes
correctly. Two holes: the partition check is vacuous for empty var sets (a
plan omitting a gate occurrence validates, and with the gate relation empty
the executor returns all of R instead of the empty set — wrong results on a
validated plan), and an out-of-range `OccId` sails through to an index panic
in the executor. Close the boundary so the seal means what it says.

## Technical direction

- **Every occurrence appears.** `plan/fj.rs::validate` (`:289-308`): during
  the partition pass, count per-occurrence subatom appearances (one
  `Vec<u32>` indexed by dense occ position, or fold into the existing loop).
  After the pass: any occurrence with zero appearances →
  `PlanError::MissingOccurrence { occ }`. Zero-var occurrences (gates) are
  legal *only* as an empty-vars subatom in some node — exactly what
  `binary2fj` emits. The audit's degenerate extreme (all-gate query, empty
  plan `FjPlan { nodes: [] }`) is rejected by the same check.
- **Every subatom resolves.** In the same pass (or `derive_nodes`): a subatom
  whose `occ` is not among `normalized.occurrences` →
  `PlanError::UnknownOccurrence { node, occ }` — a typed rejection where
  today `run.rs` panics on `colts[occ]`.
- **The tautology, made honest.** `check_selections` inside `validate` checks
  occurrences `validate` itself just constructed via `split_filters` — it
  cannot fire there. Keep the function (its unit test constructs bad
  occurrences directly, and the executor-side twin is a debug_assert), but
  demote the `validate`-internal call to `debug_assert!(check_selections(..).is_ok())`
  with a comment naming the real producer (hand-built `PlanOccurrence`s in
  tests) — the audit's "honest form."
- **Comment fixes in the same file:** the slot-layout comment
  (`fj.rs:359` — "node order, then subatom order" → "node order, then `VarId`
  order within a node"); keep the factor()-vs-Fig.8 NOTE's reasoning as a
  permanent comment above `factor` if not already sufficient (the audit
  called the existing comment adequate — verify and leave it).

## Non-goals

Changing what plans the production pipeline produces (binary2fj/factor emit
every occurrence already — these are boundary checks, not pipeline changes);
relaxing the covers-equal-new-vars deviation (audited sound); estimator work.

## Passing criteria

- The audit's exact scenario as a test: normalized query = one bound
  occurrence + one zero-binding gate occurrence; hand-built plan containing
  only the bound occurrence → `Err(MissingOccurrence { occ: gate })`. The
  all-gates/empty-plan degenerate → the same error, never `Ok`.
- Unknown-occ test: a hand-built subatom with `OccId(99)` →
  `Err(UnknownOccurrence { .. })`, no panic.
- Positive control: every existing fj/planner/executor test passes verbatim —
  in particular the full pipeline over gate-carrying queries (querygen's
  Gated shape via the bench differential) still validates and runs.
- The demoted check_selections call is a debug_assert with the naming comment;
  its direct unit test still exercises the error variant.
- `scripts/check.sh` green.
