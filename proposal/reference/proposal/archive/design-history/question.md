> Historical design research. The [active proposal](../../proposal.md) supersedes recommendations here.

# The brief

## Framing

"The Allen interval algebra of probabilistic data structures."

Allen's algebra is a small, finite, *qualitative* relation algebra: thirteen
jointly exhaustive, pairwise disjoint relations between intervals, closed under
converse, intersection, and composition, with a composition table that is the
whole theory. The engine already owns it for time: `AllenMask`, `classify`,
`CmpOp::Allen`, the intersection/difference segment operators, `Pack`, interval
width as a capacity weight.

The question is what the analogous *canonical, closed, small* structure is for
uncertainty, such that the engine can own it the way it owns time:

1. **The type.** What is the first-class value? A point probability, an interval
   `[l, u]`, an evidence pair `(r, s)`, a belief/plausibility pair, a vector of
   intervals over a closed roster, a convex set of distributions?
2. **The connectives.** What are `and`, `or`, `not` on that type, and under
   which dependence assumption? Which of them are exact, which are sound bounds,
   which are incoherent when iterated?
3. **The order.** How do two uncertain values compare? Is Allen literally the
   comparison (interval dominance), or is it a bilattice (truth order and
   knowledge order), or both?
4. **The folds.** What are the aggregates over a group: sum, mean, hull, meet,
   fusion, expectation? Which are exact via Choquet, which need LP?
5. **The laws.** What does the judge check at admission? Coherence (avoiding
   sure loss), reachability, normalization?
6. **The propagation.** How does uncertainty flow through Free Join, negation,
   and linear recursion? Is there a semiring, or is it lineage first and
   evaluation second?
7. **The boundary.** How does a Noul, a Choice distribution, or a Score become
   this type honestly? Calibration, confidence-to-width, prediction sets.

## Engine constraints the answer must respect

Taken from the current engine (1.3.1), not negotiable without a separate decision:

- Relations are sets. Admission judges the proposed final state deterministically
  and exactly. **Nothing nondeterministic enters the judge.** Any external model
  call is upstream of `ChangeSet`, never inside the engine.
- `ValueType` is a flat structural enum; illegal states are unrepresentable
  rather than rejected. Canonical row bytes are big-endian, tagged, exact.
- Existing intervals are **half-open and nonempty** by construction. A degenerate
  point interval `[p, p]` is unrepresentable today; a probability type cannot
  reuse `interval<f64>` and must be its own `ValueType`.
- `F64` is canonical binary64, NaN-free, `-0 → +0`, with `±∞` as ray endpoints.
  Aggregates over `f64` are deterministic with one final rounding. `mulDiv` is an
  exact wide product with an explicit rounding mode.
- Closed rosters have declaration-order axiom indices, but the SDK refuses to let
  a closed reference enter arithmetic: "declaration order is an accident, not
  semantics." Any ordered scale must carry its rank as a payload column.
- Queries are pure-data IR: rule-scoped variables, DNF-normalized condition
  trees, Free Join over COLT tries built from columnar images. Images are
  structure-of-arrays with `Byte`, `Word`, `WordPair`, `Words{n}` column kinds.
  A 16-byte value is a `WordPair` and every existing kernel shape applies.
- Recursion is linear (`rec`), projection-only heads, one SCC, stratified
  negation.
- Capacity laws already sum a numeric field or an interval width against a
  window; containment and mirrors laws already bind a field to a roster.
- Free Join, the differential oracle, and the naive model are the correctness net
  for any executor change.

## Candidate answers under adjudication

- **A. Probability intervals** (de Campos/Huete/Moral 1994; Weichselberger 2000).
  Type `[l, u]`; roster type = vector of reachable intervals; connectives =
  Fréchet bounds; coherence = avoiding sure loss + reachability; expectation via
  Choquet (2-monotone). Cheap, closed-form, 2|X| parameters.
- **B. Opinions / evidence pairs** (Jøsang; Walley's IDM). Type `(r, s)` or
  `(b, d, u, a)`; interval derived as `[b, b+u]`; fusion is evidence addition
  (a monoid); connectives approximate the Beta product. Bayesian-native, exact
  fusion, approximate logic.
- **C. Semiring annotation** (Green et al. 2007; Datalog°). Every fact carries a
  tag in a commutative semiring; Free Join propagates tags. Exact and convergent
  for absorptive semirings (Boolean, tropical, Viterbi, Łukasiewicz-max, fuzzy
  min-max); probability is *not* a semiring; intervals are *not* a semiring.
- **D. Lineage then evaluation** (Suciu et al.; AMC; credal SDDs). Free Join
  computes Boolean provenance; a separate pass evaluates it: exact WMC under
  independence, LP under unknown dependence, credal circuits for interval leaves.
  Tight, and #P-hard in general; safe queries are PTIME.
- **E. Bilattice / trilattice of intervals** (Ginsberg, Fitting, Lakshmanan &
  Sadri). Truth order, knowledge order, precision order; fixpoints in the
  knowledge direction; recursion terminates for positive-correlation modes.

The synthesis argues these are layers, not rivals, and says which layer owns what.
