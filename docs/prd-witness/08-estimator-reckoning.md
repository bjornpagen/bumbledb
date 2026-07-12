# PRD 08 — The estimator reckoning: the 3.3× pin vs the 4761× report

**Depends on:** baseline only.
**Modules:** `crates/bumbledb/src/plan/selectivity.rs`,
`crates/bumbledb/src/plan/planner/` (`estimate.rs`, `plan.rs`),
`api/prepared/staleness.rs` (the comment that cites the ≥4× re-prepare
convention; there is no threshold constant),
`docs/architecture/40-execution.md` (the pinned claim),
`docs/architecture/70-api.md` (the host convention).
**Authority:** the doc-law ("when code and these docs disagree, one of
them is wrong and the repo is broken") — this is the audit's one item in
that class: `40-execution.md` pins "measured worst est/actual across the
ledger families: ≤ 3.3×"; the regenerated reports show triangle
**4761.9**, conflict_free 691.2, meets_chain 511.0, conflict_pairs 289.0
(consistent across runs). The re-prepare-at-≥4× host convention derives
from the dead pin.
**Representation move:** a pinned number is a claim with a derivation;
when the world contradicts it, the fix is to find WHICH premise broke —
not to quietly re-pin. The candidate premises are enumerable and each is
testable in a unit fixture.

## Context (decided shape)

The estimate path (read before writing anything): per-node fanout =
`rows / distinct(join field)` with key coverage pinning fanout to 1; the
distinct ladder = key-exact → resident-image exact (`ImageCache::peek`) →
schema bounds (containment domains, bool) → documented keep-fraction
floors (`plan/selectivity.rs`); folded ranges charge `RANGE_KEEP_DEN`
once per field (the CT 10 change). The campaign touched three inputs the
old 3.3× never saw:

- **P1 — closed-vocabulary domains**: reference fields now carry
  containments into 3-row closed relations; the containment-domain bound
  (`containment domains` rung) can clamp a distinct estimate to ≤3 where
  the pre-campaign schema had an unbounded enum column. A 3-row domain
  under a `rows/distinct` fanout inflates estimates by orders where the
  TRUE distinct is large (or deflates — derive the direction on the
  triangle family's actual statements).
- **P2 — the fold's count-once change**: constant ranges now multiply the
  keep-fraction once per field, not per filter — correct for folded
  conjunctions, but verify the triangle/conflict families' predicates
  weren't relying on the old double-count to accidentally cancel error.
- **P3 — cyclic-join independence**: triangle is the WCOJ-honesty family;
  independence-assumption estimates on cycles are classically off by
  orders. Was the old 3.3× measured over a population that excluded
  triangle, or with a different triangle query? (`git log` the family and
  the pin; the pin's derivation must name its population.)

Decided outcomes by diagnosis:
- If P1/P2 expose a defect (an estimate rung misapplied): fix the rung in
  `selectivity.rs`, with a unit fixture reproducing the family's shape at
  small scale and asserting the estimate within a stated factor of the
  true cardinality.
- If P3 (inherent cyclic error, old pin mispopulated): the pin is
  rewritten honestly — worst est/actual reported per family CLASS
  (acyclic vs cyclic), with the cyclic number carried as-is and the doc
  stating why cyclic estimates are not governed (WCOJ plans bound the
  damage; the estimate orders the DP, it does not gate correctness).
- Either way, the **≥4× re-prepare convention is re-derived or deleted**:
  if worst-case honest est/actual is thousands, a 4× staleness heuristic
  is noise. Decide from the fixed/explained distribution: either a
  per-family-class threshold with a stated derivation, or deletion of the
  convention (staleness signaling reverts to generation-age only) — the
  70-api text and the `staleness.rs` commentary move together with the
  derivation written beside the generation-age signal.

## Technical direction

1. Build the diagnosis fixture FIRST (this is the weaker-model rail):
   a unit test in `plan/selectivity.rs`'s test module (or
   `plan/planner/tests`) that constructs the triangle family's schema
   shape at toy scale (three relations, cyclic join, known cardinalities,
   a closed vocabulary on one edge), runs `prepare`, and prints/pins the
   per-node estimates via the structured stats. This makes P1/P2/P3
   distinguishable by reading one test's output, no bench run needed.
2. Chase the number: instrument nothing new — the structured stats
   already carry `estimates`; the fixture reads them.
3. Apply the outcome per the decided shapes. Any `selectivity.rs` fix
   carries its own positive/negative unit tests (the estimate-rung
   discipline already in that file's tests).
4. Rewrite the `40-execution.md` pin with the new derivation and
   population statement; rewrite or delete the 70-api convention and
   `staleness.rs` in lockstep. No number appears without its population
   named next to it — that's the sentence the old pin was missing.
5. Do NOT tune the estimator beyond the diagnosed defect (no new rungs,
   no histograms — the no-histograms refusal stands).

## Passing criteria

- `[test]` The diagnosis fixture exists, is committed, and its assertion
  message names which premise (P1/P2/P3) the numbers demonstrated.
- `[test]` If a defect was fixed: the fixture asserts the corrected
  estimate within its stated factor; a near-miss negative fixture pins
  the rung's boundary.
- `[shape]` `40-execution.md` carries no unqualified "≤ 3.3×"; every
  est/actual claim names its population and derivation date.
- `[shape]` `staleness.rs` and `70-api.md` agree with each other and
  with the new derivation (or the convention is gone from both).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

As above — the pin and the convention ARE doc amendments riding the
diagnosis code.
