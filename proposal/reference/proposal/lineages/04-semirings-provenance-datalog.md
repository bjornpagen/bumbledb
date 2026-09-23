# 04 — Semirings, provenance, and Datalog over semirings

**Historical lineage notes.** Current execution and operator decisions are in
[revision 0.5's adjudication](../research/algebra-resolution.md). Provenance
homomorphisms do not make arbitrary probability observations homomorphisms.

Candidate C. The propagation layer. Bumbledb's Free Join is from Suciu's group
(Wang, Willsey, Suciu 2023); Datalog° is Abo Khamis, Ngo, Pichler, Suciu, Wang.
This lineage is the engine's own family tree.

## K-relations (Green, Karvounarakis, Tannen, PODS 2007)

- **Definition 3.1**: "A K-relation over a finite set of attributes U is a
  function R : U-Tup → K such that its support supp(R) = {t | R(t) ≠ 0} is
  finite." Union/projection combine tags with `+`; join combines with `·`.
- **Definition 3.2** lifts `RA⁺` to K-relations. Set semantics is the Boolean
  semiring `(𝔹, ∨, ∧)`; bags are `(ℕ, +, ·)`; c-tables are `PosBool(B)`
  ("Boolean expressions over some set B of variables which are positive"); event
  tables are `(𝒫(Ω), ∪, ∩)`. "These four structures are examples of commutative
  semirings." Proposition 3.4: the RA identities hold **iff** K is a commutative
  semiring — the semiring axioms are forced by wanting join to distribute over
  union and be associative/commutative.
- **Theorem 4.3** (the universal property): provenance polynomials `ℕ[X]` are
  the free commutative semiring on the tuple variables; an assignment into another
  commutative semiring extends uniquely to a homomorphism evaluating the
  polynomial. "Query evaluation commutes with
  semiring homomorphisms."
- **Datalog**: tags are sums over (possibly infinitely many) derivation trees, so
  K must be **ω-continuous** ("The most general such semiring ... is the
  commutative ω-continuous semiring of formal power series with variables from X
  and coefficients from ℕ∞"). `PosBool(B)` with B finite "is in fact a
  distributive lattice ... hence ω-continuous." Definition 5.5 / Theorem 5.6:
  least fixpoint of the immediate-consequence operator over `Kⁿ`.
- A witness-set provenance carrier uses collections of finite variable sets,
  with union of collections and pairwise union of witnesses for multiplication.
  Its identities are the empty collection and the singleton empty witness.
  Writing `(𝒫(X), ∪, ∪)` would not give the required distinct zero/unit behavior.
  Applicable provenance quotients and lineage are coarser homomorphic images.

**Key statement (Senellart 2017; repeated in arXiv:2310.16472):** "there is no
semiring for which the probabilities of certain queries coincide ... the reason
is that probabilities are not truth functional, while provenance is. However,
query probabilities can be computed using positive Boolean provenance, which is
captured by a semiring."

## Codd's theorem over semirings (Badia, Kolaitis, Noguera, PODS 2025; arXiv:2501.16543)

- Difference needs a **monus**: `b ∸ a = ⋀{c | a + c ≥ b}`, "well defined for
  naturally ordered semirings with some additional properties." Semirings with
  monus include "the Boolean semiring, the bag semiring, the tropical semiring on
  the natural numbers, and every complete bounded distributive lattice (hence,
  the fuzzy semiring)."
- Relational algebra over semirings needs a **support** operation `s(a) = 1 if
  a ≠ 0 else 0`, not expressible from the five basic operations "for relations
  over the fuzzy semiring."
- Division is **not** expressible from the five basic operations over positive
  semirings, "even for bag databases." Two versions of Codd's theorem result.
- Setting: `K = (K, +, ·, ∸, s, 0, 1)` zero-sum-free commutative semiring with
  monus and support. BRA ≡ domain-independent BRC with a restricted negation.

## FAQ (Abo Khamis, Ngo, Rudra, PODS 2016 best paper; arXiv:1504.04044)

- One problem: `φ(x_f) = ⊕^{(f+1)} … ⊕^{(n)} ⊗_S ψ_S(x_S)` — aggregate variables
  out of a product of factors over a semiring (or several, one per variable).
  Instances: conjunctive queries and Datalog (Boolean), `#CQ` (counting),
  marginal inference and MAP in graphical models (sum-product, max-product),
  matrix chain multiplication, DFT, quantified CQs.
- **InsideOut**: variable elimination with indicator projections, complexity
  governed by the **FAQ-width** (a fractional hypertree width of the variable
  ordering), leveraging Grohe–Marx fractional edge covers and worst-case optimal
  join analysis.
- Relevance: FAQ gives a broader factor/elimination framework. Native Free Join
  matches bindings and its sinks implement selected aggregates; it is not already
  an arbitrary semiring or mixed-aggregate executor. Adding Event programs must
  preserve complete-binding errors, union of all witnesses, and justified
  elimination order. Probability contraction needs actual joint factors beyond
  the ordinary row join. The analogy motivates integration; it does not prove it.

## Datalog° — convergence over semirings (Abo Khamis, Ngo, Pichler, Suciu, Wang; PODS 2022, JACM 2024; arXiv:2105.14435)

### Definitions

- **Pre-semiring**: `(S, ⊕, 0)` commutative monoid, `(S, ⊗, 1)` monoid, `⊗`
  distributes over `⊕`. **Semiring** adds absorption `x ⊗ 0 = 0`.
- **POPS** (partially ordered pre-semiring): plus a poset `⊑` with `⊕, ⊗`
  monotone and a least element `⊥`. **Core semiring** `P_{⊕⊥} = {x ⊕ ⊥}`.
- **p-stable**: `u^{(p)} = 1 ⊕ u ⊕ … ⊕ u^p = u^{(p+1)}`. **Stable**: p exists per
  element. **Absorptive = 0-stable**: `1 ⊕ u = 1`, equivalently `a ⊕ (a ⊗ b) = a`.
- **Theorem 1.2**: "Every datalog° program converges iff the semiring P_{⊕⊥} is
  stable"; converges in a number of steps depending only on `|ADom(I)|` iff
  p-stable; "If P_{⊕⊥} is 0-stable, then every datalog° program converges in N
  steps" (N = number of ground IDB tuples), i.e. polynomial time.
- **Dioid** (§6): "a semiring for which ⊕ is idempotent." Proposition 6.1: a
  dioid is naturally ordered by `a ⊑ b iff a ⊕ b = b` and `⊕ = ∨`.
  **Complete distributive dioid** (Def. 6.2): the natural order is a complete
  distributive lattice. Difference `b ⊖ a = ⋀{c | a ⊕ c ⊒ b}`. **Theorem 6.4**:
  semi-naive evaluation is correct for datalog° over any complete distributive
  dioid. Examples given: `(2^U, ∪, ∩)`, `Trop⁺ = (ℝ₊∪{∞}, min, +)`, `ℕ∪{∞}`.
  Not dioids: `Trop⁺_p`, `Trop⁺_{≤η}`, `ℝ_⊥`.
- **THREE** `= ({⊥,0,1}, ∨, ∧, 0, 1, ≤_k)`: "Kleene 3-valued logic with knowledge
  order," core `{⊥,1} ≅ 𝔹`, "Used for negation under Fitting's semantics." This
  is Fitting's `{0,1}`-restricted interval bilattice (lineage 03).
- The reals: "R is not naturally ordered"; Lemma 2.8: any POPS extension of ℝ
  "is not a semiring"; lifted reals `ℝ_⊥` have trivial core. **Sum-product over
  probabilities has no convergence guarantee** — proof trees can be infinite on
  cyclic data, and even ignoring convergence "probabilities are not truth
  functional."

### Follow-ups

- Convergence rate for linear datalog° over stable semirings (Im, Moseley, Ngo,
  Pruhs, ICDT 2024; arXiv:2311.17664) and polynomial-time convergence in general
  (ACM 2024).
- **Circuits and formulas for Datalog over semirings** (Fan, Koutris, Roy, PODS
  2025; arXiv:2504.08914): "We focus on absorptive semirings as these guarantee
  the existence of a polynomial-size circuit." Depth dichotomy `Θ(log m)` vs
  `Θ(log² m)`. Transitive-closure provenance needs super-polynomial *formulas*
  (Karchmer–Wigderson) but polynomial *circuits* (Deutch et al. 2014) over
  absorptive semirings.
- **Revisiting semiring provenance for Datalog** (Bourgaux et al., KR 2022):
  derivation-based and model-based provenance semantics coincide for
  "a commutative absorptive ω-continuous semiring"; "If K is not absorptive,
  there exists Σ, (D,K,λ) and α" where they differ. Absorptive is the robustness
  condition for recursion with provenance; `PosBool(X)` is the reference case.

## Scallop (Huang et al. NeurIPS 2021; Li, Huang, Naik 2023, arXiv:2304.04812)

A Datalog engine in Rust (45k LoC) with pluggable provenance, the closest
existing system to "tags through a Datalog join engine."

- **Provenance structure**: 7-tuple `(T, 0, 1, ⊕, ⊗, ⊖, ⊜)`: "the 5-tuple
  (T,0,1,⊕,⊗) should form a semiring"; for fixpoints "it must also be
  absorptive, i.e., t₁ ⊕ (t₁ ⊗ t₂) = t₁"; `⊖` negation with `⊖0 = 1, ⊖1 = 0`;
  `⊜` saturation "serves as a customizable stopping mechanism for fixed-point
  iteration." External interface `(I, O, τ, ρ)` tags inputs and recovers outputs.
- Built-ins: **max-min-prob** `([0,1], 0, 1, max, min, 1−x, ==)` — "The tags do
  not represent true probabilities but are merely an approximation"; **add-mult-
  prob** `(clamp(t₁+t₂), t₁·t₂)` — saturation "always returns true to avoid
  non-termination," so "less suitable for complex recursive programs";
  **top-k proofs** — DNF of proof sets capped at k, `∨_k` = top-k of union,
  `∧_k` = top-k of non-conflicting unions, recovered by `WMC(t, Γ)`; "dtkp is
  often the best performing one, and setting k=3 is usually a good choice."
  Differentiable variants use dual numbers.
- **Negation**: stratified; tagged difference keeps `t₁ ⊗ (⊖t₂)` "so information
  is not lost." **Aggregation**: sum over subsets ("worlds") of `⊗` of positive
  tags times `⊗` of negated complements — "inherently exponential if we
  enumerate all worlds," specialized per provenance (counting under mmp is
  `O(n log n)`).
- Reading: Scallop is the existence proof that tags ride a Datalog join engine
  cleanly, and the cautionary tale that "probabilistic" tags that are not
  absorptive either fail to terminate or are admitted approximations.

## Algebraic model counting (Kimmig, Van den Broeck, De Raedt 2012/2017; arXiv:1211.4475)

- `AMC(T, α) = ⊕_{I ∈ M(T)} ⊗_{l ∈ I} α(l)` over a commutative semiring with a
  literal labeling `α`. Generalizes SAT, #SAT, WMC, PROB, MPE, sensitivity
  (polynomials), gradient (dual numbers), shortest/widest path, fuzzy (max,min),
  k-weighted, OBDD construction, why-provenance.
- Three properties decide which circuit suffices: **idempotent ⊕**;
  **neutral** `α(v) ⊕ α(¬v) = e⊗`; **consistency-preserving**
  `α(v) ⊗ α(¬v) = e⊕`. Theorems 2–7: sd-DNNF always sound; s-DNNF needs
  idempotent ⊕; d-DNNF needs neutral; DNNF needs both; NNF variants need
  consistency-preserving ⊗ as well. PROB is neutral but not idempotent → d-DNNF.
- "any NNF can be smoothed in polytime preserving determinism and
  decomposability." Compiled circuit "can be re-used for several queries, or
  with a different α." No mention of intervals.
- Derkinderen et al. 2024 survey (arXiv:2402.13782) catalogs the PLP semirings
  (Boolean, probability, Viterbi, gradient, WMI, expected utility) and notes
  BetaProbLog replaces point facts with Beta distributions for "epistemic
  uncertainty"; still no interval semiring.

## Why probability intervals are not a semiring

Three independent statements of the same obstruction:

1. **Interval arithmetic is sub-distributive.** Hardouin et al. (LAA 2009,
   arXiv:1306.1136): "in the traditional interval arithmetic, multiplication of
   intervals is not distributive with respect to addition of intervals, while
   idempotent interval arithmetic keeps this distributivity."
2. **Inclusion–exclusion.** Yorgey (2016, network reliability): "there is
   similarly an appropriate semiring for the network reliability problem ... I
   do not know of one" — combining two paths needs the joint-success term, which
   a scalar `⊕` cannot express. The upper Fréchet disjunction `min(1, a+b)` and
   the independent disjunction `a + b − ab` do not distribute with `min` or `·`.
3. **Truth-functionality.** Probabilities depend on the *joint* distribution;
   provenance is compositional. Dubois & Prade 2001 say the same of any belief
   measure.

### What *is* a semiring on `[0,1]`

Checking the three lower-bound projections of the Fréchet/Frank family:

| Dependence | `⊕` (disjunction lower) | `⊗` (conjunction lower) | Semiring? | Idempotent ⊕ / absorptive |
| --- | --- | --- | --- | --- |
| unknown (Łukasiewicz) | `max` | `max(0, a+b−1)` | **yes** (Łukasiewicz semiring) | yes / yes (0-stable) |
| independent (product) | `max` | `a·b` | **yes** (Viterbi) | yes / yes |
| positive correlation | `max` | `min` | **yes** (fuzzy/Gödel, a bounded distributive lattice) | yes / yes |

Distributivity check for Łukasiewicz: `T_L(a, max(b,c)) = max(0, a + max(b,c) − 1)
= max(max(0,a+b−1), max(0,a+c−1))`. Absorption: `max(a, T_L(a,b)) = a` since
`T_L(a,b) ≤ a`. Natural order is `≤` on `[0,1]`, a complete distributive
lattice, so all three are **complete distributive dioids** in the Datalog° sense:
every datalog° program converges in `N` steps and semi-naive evaluation applies.
`max` is a sound lower bound for a disjunction under every dependence
assumption (P(A∨B) ≥ max(P(A),P(B))).

The **upper-bound** projections:

| Dependence | `⊕` (disjunction upper) | `⊗` (conjunction upper) | Semiring? |
| --- | --- | --- | --- |
| unknown | `min(1, a+b)` | `min` | **no** (`min` does not distribute over bounded sum) |
| independent | `a + b − ab` | `a·b` | **no** (`a(b+c−bc) ≠ ab + ac − a²bc`) |
| positive correlation | `max` | `min` | **yes** (same fuzzy semiring) |

So: **lower bounds propagate as dioids under every dependence mode; upper
bounds propagate as a semiring only under comonotonicity.** This is exactly
possibility theory's asymmetry (necessity is min-decomposable, possibility is
max-decomposable) and matches Lukasiewicz's "lower bounds in linear time, upper
bounds by LP" and Lakshmanan–Sadri's "polynomial iff disjunction is positive
correlation."

Caveat from lineage 06: dioid propagation of the Łukasiewicz lower bound is
*sound* but the bounds it assigns to overlapping sub-conjunctions are not
jointly coherent (Gilio–Sanfilippo Example 7), and repeated leaves are
double-penalized because `T_L` is not idempotent. Min and product do not have
the coherence problem; min is also `⊗`-idempotent.

### The 2026 pearl's interval semiring (Liell-Cock & Staton, ICFP 2026)

```haskell
data IntervalS = IntervalS !Double !Double
instance Semiring IntervalS where
  zero = IntervalS 0 0; one = IntervalS 1 1
  IntervalS a b .+. IntervalS c d = IntervalS (a + c) (b + d)
  IntervalS a b .*. IntervalS c d = IntervalS (a * c) (b * d)
```

This *is* a semiring (componentwise product of two arithmetic semirings on
`ℝ≥0`), used for WMC on a compiled BDD: "a single-pass, linear-time sound outer
approximation of the credal set bounds." Not tight because "the knowledge that
w⊥ + w⊤ = 1 is lost. This is the dependency problem of interval arithmetic" and
"each BDD guard is processed separately, so the same Knightian variable can be
evaluated differently." Proposed exact alternative: "convex polyhedra over the
Knightian variables, computing the exact credal set in a single pass," with
exponential monomial growth.

Note the difference: this semiring is sound on a **d-DNNF** (decomposable,
deterministic) circuit where sum-product is exact for point weights; it is not
sound as a join-propagation semiring on raw proof trees because sum over
non-disjoint derivations overcounts. It belongs to lineage 05's regime, not to
tag propagation.

## What this lineage contributes to the current decision

1. FAQ motivates a common factor/elimination vocabulary, but native Free Join
   needs explicit typed Event stages and source contractions. This is actual
   integration work, not merely exchanging two arithmetic operators.
2. Datalog° supplies convergence results under its stated stability hypotheses.
   Those results are not a universal prohibition on other recursive semantics
   or a proof that arbitrary exact probability programs terminate. Revision 0.5
   uses separately sealed finite structural fixed points with an explicit order.
3. The dependence-bound algebras above have their own assumptions and information
   loss. They remain alternative observation/approximation theories, not the
   selected Event carrier or a replacement for the retained joint law.
4. Boolean provenance can retain overlapping derivations before measurement.
   Probability is generally **not** a semiring homomorphism: P(A union B) needs
   overlap, and P(A intersection B) cannot use marginal multiplication without
   independence. A valid compiled contraction supplies the required structure.
5. Monus/support results apply to the cited K-relational signatures. The active
   Event design has set-relative complement and distinct typed relational
   operators; it does not inherit arbitrary negation/recursion laws by naming
   its operations after a semiring.
