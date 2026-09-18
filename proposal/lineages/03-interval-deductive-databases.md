# 03 — Interval probabilities inside logic and deductive databases

The lineage that already tried to put `[l, u]` into a Datalog-shaped engine.
1965–2003. Most of what the engine would need to decide was decided here once,
then forgotten.

## Boole → Hailperin → Nilsson: bounds are a linear program

- Boole (1854) posed "the general problem": given probabilities of some events,
  bound the probability of a Boolean function of them. Hailperin (1965, *Amer.
  Math. Monthly*) proved best-possible inequalities; Hailperin (1976/1986)
  "observed that Boole's probabilistic logic can be given a linear programming
  model." Nilsson (1986, *Artif. Intell.* 28) rediscovered it for AI.
- The model: variables `p(w)` for each of the `2^n` possible worlds, `Σ p(w) = 1`,
  each known probability a linear constraint `Σ_{w⊨φ} p(w) = x_φ`; the tight
  bounds on a target `ψ` are `min` and `max` of `Σ_{w⊨ψ} p(w)`. This is
  **probabilistic satisfiability (PSAT)**, NP-complete even where classical SAT
  is in P (Georgakopoulos, Kavvadias, Papadimitriou 1988). Column generation is
  the standard solver (Jaumard, Hansen, Poggi de Aragão 1991; Hansen & Jaumard
  2000 survey; Finger & De Bona 2011, 2015 phase transition).
- Conditional extensions become PSPACE-complete when constraints combine
  distinct conditional events. Imprecise (interval) inputs are handled by the
  same LP (Hansen et al. ISIPTA'99; Walley, Pelessoni, Vicig 2004).

**This is the tightness oracle.** Every "compositional" interval rule below is
an approximation of this LP; the LP is exact and exponential.

## Lukasiewicz — conditional constraints (JAIR 1999; TOCL 2001)

- Constraint `(H|G)[u₁,u₂]` with semantics
  `Pr ⊨ (H|G)[u₁,u₂] iff u₁·Pr(G) ≤ Pr(GH) ≤ u₂·Pr(G)`. Note `Pr(G)=0` always
  satisfies it.
- **Tight logical consequence**: `KB ⊨_tight (H|G)[u₁,u₂] iff u₁ = inf u, u₂ = sup u`
  over all models with `Pr(G) > 0`; the set `u` "is a closed interval in the real
  line." Globally complete deduction = computing tight answers.
- **Theorem 2.1**: computing the tight answer is NP-hard *even for constraints
  over basic events only* (reduction from graph 3-colorability). So restricting
  the language to atoms does not buy tractability.
- Tractable case: **conditional constraint trees** (undirected trees, basic
  events as nodes, bidirectional constraints as edges). Point probabilities:
  tight lower and upper bounds in linear time. Interval probabilities: "greatest
  lower bounds can be deduced in the same way, in linear time"; least upper
  bounds need nonlinear programs "transformed into equivalent linear programs,"
  polynomial. **Lower bounds are easier than upper bounds** — a pattern that
  recurs in lineage 04.
- Related local-rule work: Frisch & Haddawy 1994 (anytime deduction),
  Thöne/Güntzer/Kießling, Amarger/Dubois/Prade 1991 — "sound but globally
  incomplete" inference rules.

## Ng & Subrahmanian (Inf. & Comput. 1992); ProbView (Lakshmanan, Leone, Ross, Subrahmanian, TODS 1997)

- Ng & Subrahmanian: annotated logic programs where truth values are
  probability intervals; "the connectives cannot be interpreted
  truth-functionally when truth values are regarded as probabilities" and
  "negation-free definite-clause-like sentences can be inconsistent when
  interpreted probabilistically." Fixpoint semantics; stable semantics 1995.
- **ProbView**: "we characterize, using postulates, whole classes of strategies
  for conjunction, disjunction, and negation." Probabilities are intervals
  *because* dependence is unknown. Materialized probabilistic views maintained
  algorithmically. This is the first relational system to make the dependence
  assumption a **per-operation parameter** rather than a global axiom.

## Lakshmanan & Sadri — a theory of probabilistic deductive databases (TPLP 2001; arXiv:cs/0312043)

The most complete prior design. Read this before designing anything.

### The type

A **confidence level** is a pair of closed intervals `⟨[α,β], [γ,δ]⟩`: belief
bounded by `[α,β]`, doubt by `[γ,δ]`. "Doubt is not necessarily the
truth-functional complement of belief." Semantics: three worlds (true/false/
unknown) with `α ≤ w₁ ≤ β`, `γ ≤ w₀ ≤ δ`, `Σ w_i = 1`.

- **Consistent**: `α ≤ β, γ ≤ δ, α + γ ≤ 1`.
- **Reduced**: additionally `α + δ ≤ 1` and `β + γ ≤ 1` (Props. 4.1–4.2).

These are exactly de Campos's non-emptiness and reachability conditions on a
three-element roster {true, false, unknown}. Same laws, different decade.

### The connectives ("modes", Theorem 4.1)

Each mode is "characterized by a system of constraints" over nine joint worlds,
with confidences "found by extremizing certain objective functions" — i.e. each
mode is a *closed-form solution of the Boole/Hailperin LP* under a dependence
assumption:

| Mode | Conjunction belief | Conjunction doubt |
| --- | --- | --- |
| Ignorance (Fréchet) | `[max(0, α₁+α₂−1), min(β₁,β₂)]` | `[max(γ₁,γ₂), min(1, δ₁+δ₂)]` |
| Independence | `[α₁α₂, β₁β₂]` | `[1−(1−γ₁)(1−γ₂), 1−(1−δ₁)(1−δ₂)]` |
| Positive correlation | `[min α, min β]` | `[max γ, max δ]` |
| Negative correlation | `[max(0, α₁+α₂−1), max(0, β₁+β₂−1)]` | `[min(1, γ₁+γ₂), min(1, δ₁+δ₂)]` |
| Mutual exclusion | `[0, 0]` | disjunction belief `[α₁+α₂, β₁+β₂]` |

Disjunction is dual. Negation "simply swaps belief and doubt." Theorems
4.2–4.3: **all modes preserve consistent and reduced levels.** Positive
correlation is identical to the lattice operations `⊗_t / ⊕_t`.

### The order: a trilattice

Three orders, "interlaced" (Lemma 3.1):
- **truth** `≤_t`: belief up, doubt down;
- **knowledge** `≤_k`: both up;
- **precision** `≤_p`: intervals narrow.

Extremes: `⊤_t = ⟨[1,1],[0,0]⟩`, `⊥_t = ⟨[0,0],[1,1]⟩`, `⊤_k = ⟨[1,1],[1,1]⟩`
(inconsistent), `⊥_p = ⟨[0,1],[0,1]⟩` (total ignorance). Fitting's fourth order
would make a quadri-lattice; set aside.

### Rules and fixpoints

A p-rule `(r; μ_r, μ_p)`: `μ_r` conjoins body subgoals with the rule's
confidence, `μ_p` disjoins derivations of the head. Rules for one predicate
share `μ_p`. Example: `(prognosis(X,D) ← high-risk(X,D); ign, pc)`.

- `T_P` is "monotone and continuous" (Theorem 5.1) in `≤_t`, preserves
  consistency; iteration starts at `⊥_t`, joins with `⊕_t`; lfp = `⊗_t` of
  satisfying valuations (Theorem 5.2), mirroring van Emden–Kowalski.
- **Termination**: "the closure ordinal of T_P can be as high as ω in general
  (but no more)" — some programs "do not even terminate on some input
  databases." **Theorem 7.2**: when recursive predicates use **positive
  correlation as the disjunction mode**, "its least fixpoint can be computed in
  time polynomial in the database size." The proof needs disjunctive
  derivation trees, not the classical argument.
- Positive-correlation disjunction is `max` — idempotent `⊕`. This is the same
  condition Datalog° later isolates as stability/absorption (lineage 04). Sum-
  like disjunctions (independence `a+b−ab`, ignorance `min(1,a+b)`) are what
  break convergence.

## Bilattices and the interval order (Ginsberg 1988; Fitting 1991, 2002, 2020)

- Ginsberg's product: `L₁ ⊙ L₂` on pairs `⟨belief, doubt⟩` with
  `⟨a,b⟩ ≤_t ⟨c,d⟩ iff a ≤ c ∧ d ≤ b` and `⟨a,b⟩ ≤_k ⟨c,d⟩ iff a ≤ c ∧ b ≤ d`.
  "L₁ ⊙ L₂ will always be a pre-bilattice that satisfies the interlacing
  conditions"; distributive if both are. Negation `¬⟨a,b⟩ = ⟨b,a⟩`; conflation
  `−⟨a,b⟩ = ⟨−b,−a⟩` given an order-reversing involution.
- Fitting, *Bilattices and the semantics of logic programming* (JLP 1991): "A
  fixed-point semantics is developed for logic programming, allowing any
  bilattice as the space of truth values ... including those involving
  confidence factors." "The most natural 'direction' in which to evaluate a
  least fixed point is the information or knowledge direction; it turns out that
  negation poses none of the familiar problems then."
- **Fitting, *Kleene's Logic, Generalized*, §3 — the probability-interval
  bilattice verbatim:**

  > "Suppose we are attempting to assign, not classical truth values, but
  > probability estimates to formulas ... we may only be able to say the value
  > of a formula lies in the sub-interval [a, b] ... Suppose, then, that we take
  > as our truth values such closed intervals: B = {[a, b] | 0 ≤ a ≤ b ≤ 1}."

  Orders: "[a, b] ≤_k [c, d] if [c, d] ⊆ [a, b]" (Sandewall) and
  "[a, b] ≤_t [c, d] if a ≤ c and b ≤ d" (Scott). "≤_t yielding a complete
  lattice, and ≤_k a complete semi-lattice."

  > "Suppose we make the most extreme restriction: we only consider the two
  > probabilities 0 and 1. In this case there are three intervals: [0,0] = {0},
  > [0,1] = {0,1}, and [1,1] = {1}, which are identified with false, ⊥, and true
  > respectively. In fact, it is easy to see that what we get is the structure
  > of Kleene's three-valued logic."

  With `{0, ½, 1}`: six values, recovering Garcia & Moussavi's six-valued logic.

- **Approximation Fixpoint Theory** (Denecker, Marek, Truszczyński 2000):
  elements `(x,y)` of `L²` denote intervals `[x,y]`; precision order
  `(x,y) ≤_p (x',y') iff x ≤ x' ∧ y' ≤ y` (reverse inclusion). Generalizes
  Ginsberg/Fitting/Baral–Subrahmanian; the standard framework for stable and
  well-founded semantics of logic programs with negation.

## What this lineage decides

1. **`[l,u]` on the unit line with `≤_t` and `≤_k` is a known, worked-out
   bilattice (Fitting), and Kleene's `{false, ⊥, true}` is its `{0,1}`
   restriction.** The engine's existing three-valued negation semantics is a
   sub-logic of the proposed type.
2. **Dependence is a per-operation mode**, closed-form per mode, each mode a
   solved LP; all modes preserve consistency and reachability. ProbView and
   Lakshmanan–Sadri both landed here independently.
3. **Recursion converges iff disjunction is idempotent (positive correlation /
   `max`).** With additive disjunctions the closure ordinal is ω.
4. **Tightness is an LP and NP-hard even over atoms.** Anything compositional
   is a bound.
5. **Lower bounds are structurally easier than upper bounds** (Lukasiewicz's
   trees; recurs in lineage 04).
