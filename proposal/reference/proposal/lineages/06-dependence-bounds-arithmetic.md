# 06 — Dependence, Fréchet bounds, t-norms, probabilistic arithmetic

The connectives. What `and`/`or`/`not` can mean on uncertain values when you do
or do not know how the underlying propositions are correlated.

## Boole → Fréchet → Hoeffding: the bounds

For events with marginals `x_i = P(A_i)` and **no information about dependence**:

```
conjunction:  max(0, Σ x_i − (n−1))  ≤  P(⋀A_i)  ≤  min_i x_i
disjunction:  max_i x_i              ≤  P(⋁A_i)  ≤  min(1, Σ x_i)
negation:     P(¬A) = 1 − P(A)                     (exact)
```

Boole posed the problem (1854); Hailperin (1965, "Best possible inequalities
for the probability of a logical function of events") proved sharpness and
(1976) gave the LP model; Fréchet (1935) and Hoeffding (1940) stated the
copula form `max(u+v−1, 0) ≤ C(u,v) ≤ min(u,v)`, the distributions of perfectly
negatively and positively dependent variables.

Lifted to intervals `[l_i, u_i]` these are monotone in each argument, so the
interval conjunction under unknown dependence is

```
[l₁,u₁] ∧ [l₂,u₂] = [ max(0, l₁+l₂−1), min(u₁,u₂) ]
[l₁,u₁] ∨ [l₂,u₂] = [ max(l₁,l₂),      min(1, u₁+u₂) ]
¬[l,u]            = [ 1−u, 1−l ]
```

Under **independence** (product copula):

```
∧ⁱ = [ l₁l₂,  u₁u₂ ]
∨ⁱ = [ 1−(1−l₁)(1−l₂),  1−(1−u₁)(1−u₂) ]
```

Under **positive correlation** (comonotone): `∧ = [min l, min u]`, `∨ = [max l, max u]`.
Under **negative correlation** (countermonotone): `∧ = [max(0,l₁+l₂−1), max(0,u₁+u₂−1)]`.
(Lakshmanan & Sadri Theorem 4.1 lists all of these as "modes"; see lineage 03.)

## Fréchet bounds are the extreme t-norms (Gilio & Sanfilippo, arXiv:2010.14382)

- Łukasiewicz t-norm `T_L(x,y) = max(0, x+y−1)` **is** the Fréchet lower bound.
  Minimum t-norm `T_M = min` **is** the Fréchet upper bound. Product `T_P = xy`
  is independence. The dual t-conorms give the disjunction bounds.
- **Frank t-norms** `T_λ` interpolate: `λ=0` minimum, `λ=1` product,
  `λ=+∞` Łukasiewicz; `T_L ≤ T_λ ≤ T_M`. Theorem 10: under logical independence
  the coherent set of `(x₁..xₙ, x₁…ₙ)` is exactly `x₁…ₙ ∈ [T_L(x), T_M(x)]`.
  Every Frank t-norm is "the prevision of a conjunction" for some dependence
  structure. **The dependence knob is one real parameter λ.**
- Among *all* t-norms the lower bound is the drastic t-norm, not Łukasiewicz;
  Łukasiewicz is the lower bound among **copulas**, which is why it is the
  probability bound (Bonissone 1987: T₁=Łukasiewicz "appropriate to perform the
  intersection of lower probability bounds," T₃=min for upper bounds).

### The coherence trap (Gilio & Sanfilippo §6)

Assigning the t-norm value to **every** sub-conjunction of three events:
"is coherent for every (x₁,x₂,x₃) ∈ [0,1]³ when T_λ is the minimum t-norm, or
the product t-norm," but for Łukasiewicz "coherence is not assured and hence it
may happen that the Frank t-norm of three conditional events is not a
conjunction." Concretely, `(x₁,x₂,x₃, T_L pairs, T_L triple) = (0.5, 0.6, 0.7,
0.1, 0.2, 0.3, 0)` is **not coherent**.

**Implication for the engine.** The Fréchet lower bound is a sound, sharp bound
for *one* conjunction taken alone. Iterating it pairwise through a join tree
gives a sound but not tight bound, and the *set* of intermediate bounds it
produces need not be jointly coherent. Tightness for a compound event under
unknown dependence needs the LP over possible worlds (lineage 03). Min and
product do not have this problem: they are coherent when pushed through
sub-conjunctions.

Dubois & Prade (2001, "Possibility theory, probability theory and
multiple-valued logics: a clarification"): degrees of truth are compositional,
degrees of belief "cannot be so"; any belief representation "where
compositionality is taken for granted is bound to at worst collapse to a
Boolean truth assignment and at best to a poorly expressive tool." The interval
algebra is compositional *as a bound*, never as a truth value.

## Probabilistic arithmetic (Williamson & Downs, IJAR 1990; Williamson thesis 1987)

The same idea for arithmetic on random variables rather than logic on events.

- "Dependency bounds are lower and upper bounds on the distribution of a
  function of random variables that contain the true distribution even when
  nothing is known of the dependence of the random variables. They are based on
  the Fréchet inequalities."
- Sharp bounds for sum/difference/product/quotient of two RVs with fixed
  marginals: Frank, Nelsen & Schweizer 1987 ("Best-possible bounds for the
  distribution of a sum — a problem of Kolmogorov") for sums; Williamson & Downs
  extend to the four operations. Subtlety: "sharp bounds on the distribution of
  the treatment effect are not reached at the Fréchet-Hoeffding lower and upper
  bounds for the joint distribution" — the extreme copulas are not always the
  extremizers of a *function* of the variables.
- Numerical representation: discretized quantile functions; convolution
  with the extreme copulas via `sup`/`inf` convolutions (triangle functions).
  Berleant & Goodman-Strauss (Reliable Computing) do the same with intervals.

## P-boxes and Dempster–Shafer structures (Ferson et al., SAND2002-4015)

- A p-box is a pair of CDFs `[F̲, F̄]`; a DS structure on the reals is a set of
  intervals with masses; "the Dempster-Shafer structure associated with a p-box
  [is] that approximation produced by using a discretization with 100
  equiprobable thin rectangles." Every belief function fixes a unique p-box;
  a p-box corresponds to an equivalence class of belief functions.
- Five ways to construct them from data/knowledge; aggregation methods for
  agreement and conflict: envelope (hull), intersection (meet), mixture,
  Dempster's rule, "convolutive average," "horizontal average," logarithmic
  pool. The companion SAND2004-3072 treats dependence in propagation.
- "Working with p-boxes also allows, via so-called probabilistic arithmetic,
  for very efficient numerical methods" (Destercke et al.).
- Destercke/Dubois/Chojnacki: generalized p-boxes are special random sets and
  pairs of possibility distributions; ordinary p-boxes are incomparable with
  probability intervals.

**Reading for the engine.** p-boxes are the *distributional* generalization of
the interval type: an interval bounds one probability; a p-box bounds a whole
CDF. Out of scope for a first cut, but it is where `interval<f64>`-valued
uncertain *measurements* would go, and probabilistic arithmetic is what
`Compute` over uncertain numbers would be.

## Uncertain<T> (Bornholt, Mytkowicz, McKinley, ASPLOS 2014)

The PL precedent for "uncertainty as a first-class type."

- Three **uncertainty bugs**: "(1) Using estimates as facts ignores random error
  in estimates. (2) Computation compounds that error. (3) Boolean questions on
  probabilistic data induce false positives and negatives."
- Semantics: `Uncertain<T>` "encapsulates a random variable"; operators build a
  **Bayesian network** of computations lazily ("nodes represent random variables
  and edges represent conditional dependences"); leaves are expert-specified
  distributions; evaluation samples. Static single assignment tracks shared
  leaves so `A = Y + X; B = A + X` gets the *correct* dependent network, not two
  independent copies of X (Fig. 8).
- Conditionals become **hypothesis tests**: `if (Speed > 4)` is
  `Pr[Speed > 4] > 0.5` implicitly, or explicit `(Speed < 4).Pr(0.9)`; evaluated
  by sequential sampling with a ternary outcome ("neither branch may be true
  because the runtime may not be able to reject the null hypothesis").
- Leaves "assumes that leaf nodes are independent," overridable by experts.

**Reading for the engine.** Sampling is out (nondeterminism), but two ideas
transfer exactly: (a) *tracking shared sources* so the same uncertain fact used
twice in a join is one variable, which is precisely what Boolean lineage does in
lineage 05; (b) *conditionals as threshold tests with a third outcome*, which is
the `act / refuse / escalate` trichotomy an interval gives for free.

## What this lineage decides

1. **`not` is exact.** `and`/`or` are exact only under a declared dependence
   mode; under unknown dependence they are sharp bounds for one operation and
   sound-but-loose when iterated.
2. **The dependence knob is a Frank parameter λ ∈ {0 (min), 1 (product),
   ∞ (Łukasiewicz)}** at minimum, and could be continuous. Unknown dependence
   is the pair `[Łukasiewicz, min]` on lower/upper.
3. **Iterated Łukasiewicz is not coherent**; iterated min and product are. Any
   design that pushes Fréchet through a join tree must label the result "sound
   bound," never "the probability."
4. p-boxes and probabilistic arithmetic are the same construction one level up
   (distributions instead of events). Park them.
