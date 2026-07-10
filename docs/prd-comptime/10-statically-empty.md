# PRD 10 — Statically empty: predicate folding at normalize

**Depends on:** baseline only (independent; 07's empty-set fold routes into
this PRD's plan shape when both land — whichever lands second wires the
route).
**Modules:** `ir/normalize/` (the folding pass), `api/prepared/build.rs`
(the empty plan), `api/stats.rs` (EXPLAIN), `exec/dispatch` (plan kinds).
**Authority:** `20-query-ir.md` (normalization), the staging audit item 5.
**Representation move:** the database analog of comptime-unreachable. A
query whose predicates are mutually unsatisfiable is stage-2-known to
denote the empty set; today it plans, binds views, builds images, and
executes to produce nothing. After this PRD it becomes `ExecPlan::Empty` —
a plan whose execution touches nothing and whose EXPLAIN says why.

## Context (decided shape)

A **range-summary fold** per (occurrence, field/slot) runs at the end of
normalization, over that occurrence's own filter list plus single-variable
residuals confined to one occurrence (cross-occurrence residuals are NOT
folded — their satisfiability depends on data):

1. **Conjunction of order filters** on one u64/i64 slot folds to a single
   `[lo, hi]` summary (`x > 5 ∧ x >= 7 ∧ x < 20` → `[8, 19]`); the folded
   summary REPLACES the constituent filters (fewer residuals = fewer
   keep-fraction multiplications and fewer kernel passes — the fold is also
   the selectivity fix for double-counted ranges, noted in the estimator).
2. **Contradictions**, each detected on constants only, each producing a
   *statically-empty verdict for the rule*: empty range summary
   (`lo > hi`); `Eq` to two distinct constants on one slot; `Eq` constant
   outside the range summary; membership set that is empty after
   sentinel-trim intersected with an `Eq` constant not in it; an Allen
   literal-vs-literal predicate that `classify` refutes (both operands
   constant intervals); a point-membership of a constant point in a
   constant interval that fails.
3. **`Ne` and param-bearing predicates never fold** (params are stage-3;
   `Ne` prunes nothing statically). Interval variables fold via their
   two-slot summaries independently (start slot, end slot) — no cross-slot
   reasoning v0 (recorded; the constructor invariant `s < e` is data, not
   plan knowledge).
4. **Rule death and the empty plan:** a statically-empty rule is marked
   (sibling of `Role::Eliminated` at the RULE level — `Rule::dead: bool`);
   a query whose rules are all dead prepares to `ExecPlan::Empty`.
   `execute` on an empty plan: bind params (errors still surface — a
   vacuous mask param must still be rejected), touch no images, run no
   join, emit nothing, finalize an empty buffer. EXPLAIN prints the
   contradiction that killed each dead rule (`statically empty: x ∈ [8,19]
   ∧ x == 3`), via `ir::render`'s predicate printer.
- **Semantics guard:** folding is set-preserving by construction
  (conjunction reassociation over one slot's total order); the differential
  suite covers it for free once the generator draws contradiction-bearing
  queries — extend the generator with a contradiction knob (this PRD owns
  that one generator arm; it is not an oracle PRD, it is one drawing rule).

## Technical direction

1. `ir/normalize/fold.rs` (new): the per-slot summary accumulator
   (`RangeSummary { lo: Bound, hi: Bound, eq: Option<word>, ne_count }` over
   encoded words — the sign-flip encoding makes one unsigned comparison
   domain serve both integer types, the config-kernel precedent); one
   function per contradiction rule, unit-tested in isolation (the chase
   conditions' naming discipline).
2. Emission: the summary lowers back to at most two order filters + one Eq
   per slot (the existing `FilterPredicate` shapes — no new kernel, no new
   filter kind).
3. `build.rs`: the all-rules-dead branch producing `ExecPlan::Empty`;
   `dispatch` gains the arm; stats surface gains the per-rule death line.
4. Estimator touch: folded summaries count ONCE in `occurrence_estimate`
   (delete the per-constituent keep-fraction multiplication for folded
   slots — cite the selectivity function at its current home).

## Passing criteria

- `[test]` Each contradiction rule: one positive (kills the rule) and one
  near-miss negative (survives) unit test — twelve tests minimum.
- `[test]` Fold-preservation: randomized single-slot filter sets, folded vs
  unfolded execution over a fixture corpus, byte-identical results (the
  test-only unfolded switch, chase-off-switch precedent).
- `[test]` Multi-rule: one dead rule + one live rule executes the live one
  only (obs: one rule span); all-dead prepares to `ExecPlan::Empty`,
  executes to empty, binds params first (a vacuous mask param still errors).
- `[test]` The generator's contradiction knob produces queries that both
  engines/oracles agree are empty (differential, small corpus).
- `[shape]` EXPLAIN names the killing predicate per dead rule; no image
  build or view bind occurs under `ExecPlan::Empty` (obs counters).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`20-query-ir.md`: normalization gains the fold and the statically-empty
verdict; the estimator note (folded slots count once). `40-execution.md`:
the plan-kinds list gains `Empty`.
