# 07 — Dependency-driven join elimination (the chase)

**Kind:** planner feature — the largest item, and the thesis made executable: a
rewrite Postgres wants and structurally cannot have. Postgres rejected FK-implied
inner-join removal repeatedly for one reason — constraints can be deferred, so an
FK is not trustworthy at plan or execution time. This engine deleted the
objection: dependencies are judged once per commit against the final state
(`30-dependencies.md`), there are no deferral modes, and *every readable snapshot
satisfies every statement unconditionally*. Containment-implied atom elimination
is therefore always sound here. The classical name is the **chase**; this is its
minimal, workload-shaped slice.

**This is the only item in the folder with real regression risk. After it lands,
run the full two-oracle verify before anything stacks on top.**

## The rewrite

At normalization/plan time, a positive atom occurrence `B` is **removable** when:

1. There is an accepted containment `A(X | φ) <= B(Y | ψ)` (or the relevant
   direction of an `==`), and the query joins `A` to `B` exactly on X→Y (every
   join variable pairs a source-projection position with its target position);
2. The query's use of `B` is exhausted by that join: no `B` field outside Y is
   projected, filtered, compared in residuals, **or referenced by any other
   occurrence — positive or negated, including anti-probe bindings and membership
   points**; `B` carries no selections beyond ψ; and the `A` occurrence's own
   filters include φ — **literal-subset checked, never inferred** (a structural
   comparison of (field, literal) sets; if φ is not literally present on the `A`
   side, no elimination);
3. All of `B`'s variables are either the join variables (unified with `A`'s) or
   dead in the sense of condition 2.

Then `B` contributes nothing: existence by containment; **uniqueness because the
acceptance gate requires Y to be a key of `B`** — and key-ness is load-bearing
twice: it also makes every non-Y field of the unique match functionally
determined, so a variable bound only on `B`'s non-key fields takes exactly one
value per binding and cannot multiply aggregate binding sets. Delete the
occurrence; the result set is bit-identical under both sinks.

**Interval refusal (v0):** elimination is refused when any paired position is
interval-typed — pointwise coverage is not 1:1 fact-to-fact (a source span can be
covered by several target segments), so binding multiplicity needs its own proof.
Recorded as the item's OPEN sub-question, trigger "a census query that would
benefit."

Workload instances (the census shapes): existence walks (joining the parent only
to prove the reference resolves — the containment *is* that proof), and DU
header/sidecar `==` pairs (a query over
`Grading(id | kind == Deterministic) == DeterministicGrading(grading)` touching
fields of only one side drops the other side entirely, in either direction — the
highest-frequency instance in ledger-shaped schemas).

## Fit with existing machinery

The same move `plan/provably_distinct.rs` already makes — a plan-time proof
derived from schema statements, carried as a validated flag — applied one stage
earlier, to occurrence existence rather than binding distinctness. Home: after
normalization (occurrences + filters explicit), before stats/DP, as a fixpoint
(removing one atom can make another removable; chains `A<=B<=C` are real).
EXPLAIN reports eliminated occurrences and the licensing statement id
(mechanism-names-its-reader: the reader is EXPLAIN plus the DP, which sees a
smaller problem).

D2 skip-suffix already short-circuits *executions* of these joins after a first
witness; this is the static completion — the join never runs, never forces a
trie, never occupies a DP bit. And skip-suffix is illegal under aggregate sinks,
while elimination is sink-independent (the atom provably changes no binding).

**Decision (to record in `40-execution.md` when built):** chase-based occurrence
elimination under accepted statements. **Alternative:** leave it to D2
skip-suffix dynamics. **Why it loses:** per-binding probes, larger DP, and
aggregate-sink illegality. **Reverses if:** measured plan-time cost of the
fixpoint exceeds its execution savings on the ledger suite (implausible at ≤20
occurrences).

## Acceptance

- Differential oracle: randomized queries with eliminable atoms produce identical
  results with the rewrite on and off (test-only build flag; no runtime mode
  ships). The naive model needs **no change** — it computes the unrewritten query,
  which *is* the differential test.
- The full two-oracle verify run green immediately after landing, before further
  items.
- Ledger-family benchmark: measurable win on existence-walk and one-sided DU
  families; no family regresses.
- EXPLAIN shows eliminated occurrences with statement ids; the querygen gains
  eliminable-atom shapes so the class stays covered under randomization.

## Doc amendments (rule 5)

`40-execution.md` gains the elimination pass (placement, conditions, the interval
refusal); `30-dependencies.md` gains one sentence noting statements license
planner rewrites, with the pointer.
