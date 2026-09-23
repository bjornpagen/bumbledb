> Historical design research. The [active proposal](../../proposal.md) supersedes recommendations here.

# Synthesis — what the algebra is

**Historical synthesis, superseded as the working design.** See [the active proposal](/Users/bjorn/Documents/bumbledb/proposal/proposal.md), [the claim-by-claim review](/Users/bjorn/Documents/bumbledb/proposal/review-astra.md), and [the decision record](/Users/bjorn/Documents/bumbledb/proposal/decisions.md). The text below is retained so its original claims and subsequent corrections remain traceable.

Draft. Working answer to `question.md`, built from the seven lineage digests.
Claims marked **[derived]** are ours, checked against the cited results but not
found stated in the literature in that form.

## The one-paragraph answer

The type is a **closed subinterval of the unit line, `[l, u]`, degenerate
allowed**, read as belief and plausibility. Its algebra is **Fitting's interval
bilattice**: a truth order (componentwise, `≤_t`) and a knowledge order (reverse
inclusion, `≤_k`), with negation `[1−u, 1−l]` swapping nothing but the line.
Kleene's three-valued logic is this type restricted to `{0, 1}`, so the engine's
existing negation semantics is a sub-logic of it. Two values compare
**qualitatively by Allen's thirteen relations** on closed intervals: dominance
is `PRECEDES`, refinement is `DURING`, indecision is any overlap. Connectives
are the **Frank family**, with unknown dependence spelled as the pair
`[Łukasiewicz, min]` and independence as product; every connective is exact for
one operation and a sound bound when iterated. A **roster** carries a reachable
interval vector governed by two closed-form laws (avoiding sure loss,
reachability) that induce a **2-monotone capacity**, so lower and upper
expectations over ranks are exact by **Choquet** in `O(n log n)`. Through joins
and recursion, **lower bounds propagate as complete distributive dioids** under
every dependence mode (`max` with Łukasiewicz, product, or min), converging in
`N` steps with semi-naive evaluation; **upper bounds propagate as a semiring
only under comonotonicity**. Exact upper bounds need **Boolean lineage** then
evaluation: componentwise on read-once lineage (tight), credal circuits
otherwise (exact marginals, one LP per OR node), Boole–Hailperin LP under
unknown dependence (NP-hard). The **evidence pair `(r, s)`** is the boundary
primitive: it fuses by addition and maps to the interval by Walley's IDM, which
is Jøsang's `[b, b+u]`; Venn–Abers is the warranted interval for a black-box
scorer; conformal sets are the warranted set-valued answer for a Choice.

That is the whole structure. Everything below is the argument and the cuts.

## Why this is "the Allen of probability"

Allen's algebra has three properties worth copying: it is **qualitative** (13
relations, no metric), it is **closed** (composition of relations yields a
relation), and it is **complete** for what it describes (any two intervals stand
in exactly one basic relation).

- **Qualitative.** Two beliefs `[l₁,u₁]`, `[l₂,u₂]` stand in exactly one of
  Allen's 13 relations on closed intervals. `PRECEDES`/`PRECEDED-BY` is
  interval dominance (decide). `DURING`/`CONTAINS` is refinement (one source
  knows more). `EQUALS` is agreement. `OVERLAPS`/`STARTS`/`FINISHES` and their
  converses are indecision with a direction. `MEETS` on closed intervals is a
  single shared point, which is the decision boundary `u₁ = l₂` exactly. The
  engine's `AllenMask`, `classify`, `CmpOp::Allen` apply with a closed-endpoint
  classifier. **[derived]** from Allen 1983 plus the closed-interval type.
- **Closed.** Fréchet connectives, hull, meet, negation, and Choquet expectation
  all return values of the same type. The two roster laws are preserved by every
  Lakshmanan–Sadri mode (their Theorems 4.2–4.3).
- **Complete.** For a Boolean proposition the interval **is** the credal set:
  nothing is lost. This is the single fact that makes the type honest rather
  than a heuristic. It fails above two outcomes, which is where the roster laws
  and Choquet take over as the tight-box theory (lineage 01).

The three-valued embedding is the second reason. Fitting: "we only consider the
two probabilities 0 and 1 ... [0,0], [0,1], [1,1] ... identified with false, ⊥,
and true ... what we get is the structure of Kleene's three-valued logic."
Datalog° uses that exact structure (THREE, knowledge order) for Fitting-style
negation. So `prob` is not a new logic bolted on; the engine's Boolean world is
its `{0,1}` fragment.

## The layers, and which lineage owns each

| Layer | Owner | What is exact | What is a bound | Out of scope |
| --- | --- | --- | --- | --- |
| 0. Boundary | 07, 02 | IDM/opinion map from evidence `(r,s,W)` to `[l,u]`; Venn–Abers `[p0,p1]` for binary; conformal sets for rosters | TypeSafe `confidence` → width has no warrant; store it as `f64` | Sampling |
| 1. Type | 01, 03 | `[l,u]` closed on unit line; roster = reachable interval vector; ASL + reachability laws; closed-form reachability correction | Interval vector is the bounding box of the true credal set for K ≥ 3 | General credal polytopes as stored values |
| 2. Order | 03 | `≤_t` (componentwise), `≤_k` (reverse inclusion); Allen classification; hull = `∨_k`, meet = `∧_k` (empty meet is conflict) | — | Quadri-lattice |
| 3. Connectives | 06, 03 | `not`; each single `and`/`or` under a declared Frank mode; positive-correlation min/max; product | Iterated Łukasiewicz (sound, not jointly coherent); any iterated bound | Compositional truth-functional probability (does not exist) |
| 4. Folds | 01, 02 | Choquet lower/upper expectation over a ranked roster; hull; meet; evidence-sum fusion; `Count`; componentwise `Sum` of lowers as lower bound | `Sum` of intervals is exact only under independence/read-once | Averaging fusion (semi-associative) |
| 5. Propagation | 04, 05 | Lower bounds via dioid tags in Free Join and `rec` (0-stable, N steps, semi-naive); upper bounds under comonotone via fuzzy semiring; **safe queries**: componentwise interval evaluation on read-once lineage is tight; credal circuits: exact marginals in circuit-size polynomial time | `IntervalS` WMC (sound, loose); Fréchet through join trees | Exact probabilities of recursive queries (#P); tight bounds under unknown dependence (LP, NP-hard) |
| 6. Laws | 01 | ASL `Σl ≤ 1 ≤ Σu`; reachability `l_i + Σ_{j≠i} u_j ≥ 1`, `u_i + Σ_{j≠i} l_j ≤ 1`; conflict on empty meet | — | Coherence of arbitrary conditional assessments (PSAT) |

## Where the dividing lines fall, precisely

### Lower bounds are easy; upper bounds are hard

This shows up independently in four places and is the structural fact of the
whole field:

1. Lukasiewicz 1999: in conditional constraint trees, greatest lower bounds
   "in linear time," least upper bounds by LP.
2. Lakshmanan & Sadri 2001: recursion has polynomial data complexity iff the
   disjunction mode is positive correlation (`max`).
3. Datalog°: convergence iff the core semiring is stable; `max`-disjunction
   dioids are 0-stable.
4. **[derived]**: the lower projections `(max, T_L)`, `(max, ·)`, `(max, min)`
   are complete distributive dioids; the upper projections `(min(1,a+b), min)`
   and `(a+b−ab, ·)` are not semirings (distributivity fails), only `(max, min)`
   is.

The reason is inclusion–exclusion: a disjunction's upper bound needs the joint
term, and no scalar `⊕` carries it (Yorgey's network-reliability observation;
Senellart's "probabilities are not truth functional, while provenance is").
The lower bound never needs the joint term because `max` is a valid lower bound
for `P(A ∨ B)` under every dependence structure.

Consequence for the engine: **tags on facts can carry a lower bound through
Free Join and `rec` soundly and convergently. Upper bounds through a join must
either assume comonotonicity or go through lineage.**

### Read-once lineage makes intervals exact **[derived]**

Dalvi–Suciu: hierarchical queries without self-joins are safe and have
read-once lineage (Jha & Suciu). A read-once positive Boolean formula is
monotone in each variable and each variable appears once, so under strong
independence the probability is a monotone function of each `p_i` and its
extremes are at the endpoints: evaluating the safe plan on `[l_i, u_i]`
componentwise (swapping endpoints under stratified negation) returns the exact
lower and upper probabilities. The dependency problem of interval arithmetic
(Liell-Cock & Staton: "the knowledge that w⊥ + w⊤ = 1 is lost") cannot arise
because no variable appears twice. For unsafe queries the same evaluation is
sound but loose, and the credal-circuit LP (Mattei et al., exact for marginals
on any topology) is the exact fallback.

Safety is a static property of the query. The planner can decide the regime at
`prepare`.

### Coherence of iterated bounds

Gilio & Sanfilippo Example 7: assigning Łukasiewicz to every sub-conjunction of
`(0.5, 0.6, 0.7)` yields `(0.1, 0.2, 0.3, 0)`, which is incoherent. Minimum and
product do not have this defect. So the unknown-dependence lower bound is a
**per-result** guarantee, not a **joint** one: every stored bound is sound; the
set of stored bounds need not be simultaneously attainable. Report it as a
bound. Never re-feed derived bounds as if they were assessments without saying
so.

### What the evidence representation buys and costs

`(r, s, W)` fuses exactly by addition, has width `W/(r+s+W)` with a semantic
(total evidence), and is what evidential networks emit. Its connectives are
approximate (Jøsang: "deviate from the analytically correct product") and it
fixes a hyperparameter `W`. The interval `[l,u]` has exact connectives-as-bounds
and no hyperparameter but no canonical fusion beyond hull/meet. **[derived]**
Storing evidence and deriving the interval on read gives both: fusion on
evidence, logic on intervals. Whether the engine stores evidence, intervals, or
both is an open decision below.

## Mapping onto the engine

Non-binding sketch of where each layer lands, to check the answer is
implementable within the constraints in `question.md`.

- **Theory crate**: `ValueType::Prob` (flat variant), checked `Prob { lo, hi }`
  with `0 ≤ lo ≤ hi ≤ 1`, degenerate allowed; the pure algebra (`not`, Frank
  modes, hull, meet, Allen classify on closed endpoints, Choquet over a ranked
  slice, the two roster laws, the reachability correction).
- **Representation**: fixed-point `u64` pair with `2⁶³ = 1`. Łukasiewicz is
  saturating add/sub; `min`/`max` are integer compares; product is the existing
  exact wide `mulDiv` with a new round-up mode for the upper endpoint; Choquet is
  sums of `rank × Δl` via `mulDiv`. Boundary conversion from JSON floats rounds
  once, lower down and upper up. Binary64 pairs would need outward rounding
  after every op and `1 − u` is inexact below one half. **[decision]**
- **Encoding/canonical**: 16 bytes, big-endian, own tag. **Image**: `WordPair`
  column, the interval precedent; every existing kernel shape applies.
- **IR**: `CmpOp::Allen` accepts `prob` pairs with the closed classifier; new
  `ScalarExpr` ops for `not` and the Frank connectives with an explicit mode;
  `FindTerm` folds `Hull`, `Meet`, `LowerExpectation`/`UpperExpectation` over
  `(rank, prob)`; `Sum`/`Min`/`Max`/`Count` componentwise where sound.
- **Plan/exec**: (a) a tag lane: a `prob` field designated as the fact's weight
  propagates through Free Join with a chosen dioid, and through `rec` with
  saturation on the natural order (Datalog° Theorem 6.4 semi-naive); (b) a
  lineage lane: `PosBool` tags through Free Join for safe queries, componentwise
  evaluation at the sink; credal-circuit fallback later. Safety decided at
  prepare.
- **Judge**: the two roster laws as capacity-shaped statements over a closed
  discriminator; conflict-on-empty-meet as a refusal kind.
- **Both macros, both SDKs, bridge marshal, log codec tags, naive oracle in
  bench, cookbook recipes.**

The engine never calls a model. TypeSafe, evidential networks, Venn–Abers, and
conformal are all upstream of `ChangeSet`.

## What is deliberately not in the answer

- **General credal sets as stored values.** Polytopes over K ≥ 3 outcomes.
  The interval vector plus laws is the tight box; the domain-theoretic paper
  confirms images "need not be an interval." Revisit if a real workload needs
  vertices.
- **p-boxes / probabilistic arithmetic.** The distributional lift of the same
  idea (Williamson–Downs, Ferson). Belongs with uncertain `f64` measurements,
  not with uncertain propositions.
- **Sampling semantics** (Uncertain<T>). Nondeterministic; two ideas transfer
  (shared-source tracking = lineage; conditionals as threshold tests with a
  third outcome = the interval trichotomy) and the mechanism does not.
- **Exact probabilities through recursion.** #P-hard (network reliability);
  Datalog° over sum-product does not converge. `rec` gets dioid bounds only.
- **Tight bounds under unknown dependence for compound events.** PSAT, NP-hard
  even over atoms (Lukasiewicz Theorem 2.1). Fréchet per operation is the
  offer.
- **Averaging/weighted fusion, Dempster's rule.** Fusion policies beyond hull,
  meet, and evidence sum are application code.

## Open decisions

1. **Representation**: fixed-point `u64` pair (recommended) vs binary64 pair.
2. **Unmarked `and`/`or`**: unknown dependence (Fréchet) as the default with
   independence spelled out, or the reverse. Recommended: unmarked is unknown
   dependence; the engine should never silently assume independence it was not
   told.
3. **Stored primitive**: interval only; evidence only with derived interval; or
   both with a declared `W`. Recommended: interval as the value type, evidence
   as an ordinary pair of `u64` fields the application owns, with an SDK helper
   for the IDM map.
4. **Tag lane scope**: lower-bound dioid propagation through `rec` in the first
   cut, or intervals through non-recursive safe queries only.
5. **Roster laws**: judged always when a `prob` field sits under a closed
   discriminator, or only when a statement asks.
