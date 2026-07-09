# 07 — Dependency-driven join elimination (the chase)

**Kind:** planner feature — the largest Postgres-transfer item, and the one where
this engine's semantics permit a rewrite Postgres wants and structurally cannot
have.

## Context

Postgres cannot remove an inner join implied by a foreign key. It was proposed on
-hackers repeatedly and rejected every time for one reason: an FK is not guaranteed
to hold at planning or execution time — constraints can be deferred, and
mid-transaction states can violate them, so the rewrite is unsound in general.
All Postgres ships is provably-unique LEFT JOIN removal and (v18)
self-join elimination — both keyed off *unique* constraints, neither touching the
FK-implied inner-join case that ORMs and views generate constantly.

**This engine deleted the objection.** Dependencies are judged once per commit
against the final state (`30-dependencies.md`); there are no deferral modes and no
observable violating state — *every readable snapshot satisfies every statement
unconditionally*. Therefore containment-implied atom elimination is always sound
here. The classical name for this rewrite family is the **chase** (semantic query
optimization under INDs + FDs); this item is its minimal, workload-shaped slice.

## The rewrite

At normalization/plan time, a positive atom occurrence `B` is **removable** when:

1. There is an accepted containment `A(X | φ) <= B(Y | ψ)` (or the relevant
   direction of an `==`), and the query joins `A` to `B` exactly on X→Y (every join
   variable pairs a source-projection position with its target position);
2. The query's use of `B` is exhausted by that join: no `B` field outside Y is
   projected, filtered, compared in residuals, or bound by another occurrence's
   variable; `B` carries no selections beyond ψ, and the query's selection on the
   join implies φ on the `A` side (φ, ψ literal-checked, not inferred);
3. `B`'s occurrence is positive (negated occurrences never qualify), and dropping
   it does not orphan a variable another part of the query needs (all of `B`'s
   variables are either the join variables or dead).

Then `B` contributes nothing: the containment proves every surviving `A` binding
has exactly the `B` match the join would find (existence by containment; uniqueness
because the acceptance gate requires Y to be a key of `B` —
`30-dependencies.md`, probe-ability). Delete the occurrence; the result set is
bit-identical.

Workload instances (the census shapes):

- **Existence walks:** joining the parent only to prove the reference resolves —
  the containment *is* that proof.
- **DU header/sidecar `==` pairs:** a query over
  `Grading(id | kind == Deterministic) == DeterministicGrading(grading)` touching
  fields of only one side can drop the other side entirely — in either direction,
  because `==` gives both containments. This is the highest-frequency instance in
  ledger-shaped schemas.

## Fit with existing machinery

This is the same move `plan/provably_distinct.rs` already makes — a plan-time proof
derived from schema statements, carried as a validated flag — applied one stage
earlier, to occurrence existence rather than binding distinctness. Natural home:
after normalization (occurrences + filters are explicit), before stats/DP, as a
fixpoint (removing one atom can make another removable). EXPLAIN should report
eliminated occurrences and the licensing statement id (mechanism-names-its-reader:
the reader is EXPLAIN plus the DP, which sees a smaller problem).

Interval positions: v0 should **refuse** elimination when any paired position is
interval-typed — pointwise coverage means the join is not 1:1 fact-to-fact (a
source span can be covered by several target segments), so binding multiplicity
under set semantics needs its own proof. Record as the item's OPEN sub-question
with trigger "a census query that would benefit."

D2 skip-suffix already short-circuits *executions* of these joins after a first
witness; this item is the static completion — the join never runs, never forces a
trie, never occupies a DP bit.

**Decision (to record in `40-execution.md` when built):** chase-based occurrence
elimination under accepted statements. **Alternative:** leave it to D2 skip-suffix
dynamics. **Why it loses:** skip-suffix still pays per-binding probes and plans a
larger DP; and it is illegal under aggregate sinks, while elimination is
sink-independent (the atom provably changes no binding). **Reverses if:** measured
plan-time cost of the fixpoint exceeds its execution savings on the ledger suite
(implausible at ≤20 occurrences).

## Acceptance

- Differential oracle: randomized queries with eliminable atoms produce identical
  results with the rewrite on and off (the off switch can be a test-only build
  flag; no runtime mode ships).
- The naive model needs no change (it computes the unrewritten query — that *is*
  the differential test).
- Ledger-family benchmark: measurable win on existence-walk and one-sided DU
  families; no family regresses.
- EXPLAIN shows eliminated occurrences with statement ids.

## Doc amendments (rule 5)

`40-execution.md` gains the elimination pass (placement, conditions, the interval
refusal); `30-dependencies.md` gains one sentence noting statements license planner
rewrites, with the pointer.
