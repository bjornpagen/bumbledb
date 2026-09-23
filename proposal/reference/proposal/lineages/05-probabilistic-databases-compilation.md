# 05 — Probabilistic databases, lineage, knowledge compilation, credal circuits

Candidate D. The exact layer: compute Boolean provenance, then evaluate it.
Where tightness lives and where #P lives.

## The possible-worlds model (Suciu, Olteanu, Ré, Koch 2011; Van den Broeck & Suciu 2017)

- A probabilistic database is a distribution over possible worlds. Compact
  representations: **tuple-independent (TI)** tables, **block-independent-
  disjoint (BID)** tables ("TI databases are BID databases where each block has
  exactly one tuple"), **c-tables / pc-tables** for correlations (Imielinski &
  Lipski; Abiteboul, Kanellakis, Grahne 1991). Systems: MystiQ, Trio (ULDBs),
  MayBMS/SPROUT, MCDB.
- Two evaluation strategies. **Extensional**: "the entire probabilistic
  inference can be pushed into the database engine ... The relational queries
  that can be evaluated this way are called safe queries." **Intensional**: "the
  probabilistic inference is performed over a propositional formula called
  lineage expression: every relational query can be evaluated this way, but the
  data complexity ... can be #P-hard."

## The dichotomy (Dalvi & Suciu, PODS 2007 arXiv:cs/0612102; JACM 2012)

- **Hierarchical** (Def. 1.2): "for any two variables x, y, either sg(x) ∩ sg(y)
  = ∅, or sg(x) ⊆ sg(y), or sg(y) ⊆ sg(x)." `R(x), S(x,y)` is hierarchical;
  `R(x), S(x,y), T(y)` is not.
- **Theorem 1.3** (no self-joins): "If q is hierarchical, then it is in PTIME.
  If q is not hierarchical then it is #P-hard." The PTIME algorithm is a
  recurrence: pick a maximal variable per connected component, multiply
  independent components, use `1 − Π(1 − …)` for the existential over a
  variable — the **independent-join / independent-project** rules.
- With self-joins, **inversions** (unifying subgoals with `x ⊐ y` and `x' ⊏ y'`)
  make some hierarchical queries #P-hard (`H_k`); "If q is hierarchical and has
  no inversions, then it is in PTIME."
- **JACM 2012**: for every union of conjunctive queries, PQE is either in PTIME
  (safe) or #P-hard (unsafe), decidable in PTIME. Kenig & Suciu (arXiv:2008.00896):
  unsafe UCQs stay #P-hard "even if the probabilities are restricted to
  {0, 1/2, 1}." Fink & Olteanu (TODS 2016) extend to negation; Ré & Suciu to
  HAVING queries (trichotomy); Amarilli & Ceylan (ICDT 2020) to
  homomorphism-closed queries.

## Lineage and knowledge compilation (Jha & Suciu, ICDT 2011, ToCS 2013)

- Compile the lineage into a tractable circuit: "four target languages of
  strictly increasing expressive power: Read-Once Boolean formulae, OBDD, FBDD
  and d-DNNF. ... these queries can also be evaluated in PTIME over
  probabilistic databases."
- Hierarchical queries without self-joins have **read-once** lineage; inversion-
  free UCQs have polynomial OBDDs; queries with inversions lack even tractable
  d-SDNNF (Bova & Szeider).
- **Read-once is the property that makes interval evaluation exact.** In a
  read-once positive formula every variable appears once, so the probability is
  monotone in each input probability and the extremes are attained at endpoints:
  evaluating the safe plan on `[l_i, u_i]` componentwise gives tight bounds under
  strong independence, with no dependency problem. With stratified negation each
  variable is monotone or antitone, so swap endpoints under `¬`. For non-read-once
  lineage, componentwise evaluation is sound but loose.

## Credal semantics for probabilistic logic programs (Lukasiewicz 2005; Cozman & Mauá JAIR 2017, arXiv:1701.09000)

- Program = logic program plus probabilistic facts `α :: A` (point-valued
  rationals). "A probability model ... (i) every interpretation with positive
  probability is a stable model of P ∪ PF↓θ for the total choice θ ... (ii) the
  probability of each total choice θ is the product of the probabilities for all
  individual choices." "The set of all probability models for a plp is the
  semantics of the program" — a **credal set**. For stratified programs the
  stable and well-founded models coincide and the semantics "is exactly Sato's
  distribution semantics."
- Inference returns lower and upper probabilities. Corollary 15: the credal
  semantics "is a closed and convex set of probability measures" that dominates
  "an infinitely monotone Choquet capacity" — so credal-semantics bounds are
  belief-function-shaped, hence Choquet-integrable.
- Complexity: propositional acyclic PP-complete; bounded-arity acyclic without
  negation PP^NP; unbounded arity PEXP; cycles and negation climb "the counting
  hierarchy, up to PP^{NP^{NP}}."
- The defense of sets: "What are the best bounds on probabilities that one can
  safely assume, taking into account only the given rules, facts, and
  assessments?" and "why should we insist on singling out a distribution over
  colorings when no preference over them is expressed?"
- Solvers: PASOCS (approximate, arXiv:2105.10908), dPASP (exact lower/upper via
  Cozman & Mauá 2020, arXiv:2308.02944). Credal extension of ICL
  (arXiv:1806.08298); Logical Credal Networks (arXiv:2109.12240).

## Credal sentential decision diagrams (Antonucci, Facchini, Mattei ISIPTA 2019; Mattei et al. IJAR 2020, arXiv:2008.08524)

The exact algorithm for **interval leaves on a compiled circuit**.

- A PSDD is a logic circuit whose OR (decision) gates carry probabilities;
  a **CSDD** "allows for replacing the local probabilities with (so-called
  credal) sets of mass functions," specified by "a finite number of linear
  constraints" (in the examples, IDM-derived probability intervals such as
  `θ₁ ∈ [31/101, 32/101]`). For Boolean variables "the number of extreme points
  of the convex closure of a CS cannot be more than two."
- **Algorithm 3 (lower probability of evidence)**: terminals `π̲(n) ← P̲_n(e)`;
  decision nodes `π̲(n) ← min_{θ ∈ 𝕂_n} Σ_i π̲(p_i) · π̲(s_i) · θ_i` — **products at
  AND elements, one LP per OR node** over the node's credal set. "The
  optimizations with respect to the CSs of the terminal nodes can be done
  independently of the others," bottom-up in topological order. For interval
  constraints on a simplex the LP is a greedy fill (fractional knapsack).
- **Exactness**: marginals (Alg. 3) are exact "with no restrictions on the
  topology of the CSDD"; conditionals and MAP-robustness are exact only for
  singly connected circuits, otherwise "a conservative (outer) approximation";
  exactness is decidable at no extra cost by checking whether shared credal sets
  chose consistent extreme points.
- **Independence**: **strong independence** — "stochastic independence is
  satisfied for each extreme point of the convex closure." The CSDD's strong
  extension is "the convex hull of the set of joint PMFs induced by the
  collection of its compatible PSDDs."
- **Complexity**: polynomial in circuit size, "a single linear programming task
  for each CS."
- Inspired by credal sum-product networks (Mauá, Cozman et al. 2017). Retraces
  the BN → credal network step for circuits.

## Imprecise probabilistic programming, precisely (Liell-Cock & Staton, ICFP 2026; arXiv:2607.20801)

The most recent statement of the exact-vs-bounded trade-off for interval leaves.

- Credal set = "a closed, convex set of distributions," identified with its
  vertices; a **Knightian** choice is "a BDD variable whose weight is left free
  rather than fixed." Type-level names grade a monad (`Imp g a`, graded by finite
  sets of names under disjoint union) to "restore commutativity when sets of
  names are disjoint" — the convex powerset monad is not commutative.
- "no change to the standard BDD compilation and weighted model counting
  pipeline." WMC is polymorphic in a `Semiring`: reals (exact enumeration over
  `2^|g|` extreme valuations, then min/max), dual numbers (gradient ascent/
  descent on the multilinear WMC; single-variable optima at extremes, jointly
  "a non-convex problem in general"), and **`IntervalS`** (componentwise
  bounds, one pass, "sound (but not necessarily tight)").
- Conditioning: per-valuation local renormalization, "no global
  renormalization" — Walley's natural extension; infeasible valuations
  discarded.
- Independence by naming: "Choices with different names are independent."
  Bind = disjoint union of names.
- Exact enumeration is exponential and "unavoidable since tight bounds are
  #P-hard in general (Mauá & Cozman)."

## A domain-theoretic foundation (arXiv:2604.09272, April 2026)

- Value domain `𝕀[0,1]`: "non-empty closed intervals of the unit interval
  ordered by reverse inclusion." Credal sets: "the space of non-empty compact
  subsets ... ordered by reverse inclusion," convex sub-domain with polytope
  basis. Envelope `Ê(K)(O₁,O₂) = [inf σ(O₁), 1 − inf σ(O₂)]` = `[P̲, P̄]`,
  "correspond to the Choquet integral of indicator functions."
- Conditional probability is an interval with sound and complete rules
  (Theorem 5.1); Bayes updating is sharp on intervals (Theorem 6.4). Under
  conditional independence "lower endpoints multiply" while the upper endpoint
  "is only bounded via Fréchet: ≤ min{1−σ(U₂|W₁), 1−σ(V₂|W₁)}."
- "All operations extend Scott-continuously to spaces of credal sets, ensuring
  that finite approximations converge." Fixpoints via Hutchinson operators on
  the probabilistic power domain. Interval images of credal sets are outer
  approximations only.

## What this lineage decides

1. **Boolean lineage in Free Join, evaluation afterwards** is the exact route.
   Three evaluation regimes by what is known about dependence:
   - **strong independence, point leaves**: WMC on a d-DNNF; PTIME iff the query
     is safe, else compile or accept #P.
   - **strong independence, interval leaves**: read-once lineage → componentwise
     evaluation is tight; general lineage → credal circuit (exact marginals, one
     LP per OR node, polynomial in circuit size) or `IntervalS` (sound, one pass,
     loose).
   - **unknown dependence**: Boole/Hailperin/Nilsson LP over worlds (NP-hard);
     Fréchet pairwise as the sound bound.
2. **Safe queries make the interval type exact for free.** Dalvi–Suciu safety
   is a static property of the query; the engine can decide it at prepare time
   and choose the regime.
3. **Recursion is #P even for points** (network reliability); only bounds
   (lineage 04's dioids) are on the table for `rec`.
4. **Conditioning is an interval with sharp rules**; Bayes with intervals is
   solved (domain-theoretic paper Thm 6.4; Weichselberger's generalized Bayes).
5. The credal semantics defense is the philosophy the engine should adopt for
   its refusals: report the best bounds that follow, never invent a point.
