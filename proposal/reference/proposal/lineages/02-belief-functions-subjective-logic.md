# 02 — Belief functions, subjective logic, and the evidence representation

The Bayesian-native candidate. The value is *evidence*; belief, disbelief,
uncertainty, and the interval are all derived.

## Dempster–Shafer in one paragraph

A basic belief assignment `m` over subsets of a frame; belief `Bel(A) = Σ_{B⊆A} m(B)`,
plausibility `Pl(A) = 1 − Bel(¬A)`. `[Bel(A), Pl(A)]` is an interval. Belief
functions are **∞-monotone capacities** (Destercke et al.; Cuzzolin), hence a
special case of 2-monotone capacities, hence Choquet-integrable, hence a special
case of coherent lower previsions. Combination is Dempster's rule (normalized
conjunctive), which is for **independent sources about the same frame**, not for
one source's beliefs across independent frames — Jøsang is explicit that these
are different operations.

## Jøsang's opinion calculus ("Belief Calculus", arXiv:cs/0606029; book 2016)

### The type

Binomial opinion on `x` with complement `x̄`:

```
ω_x = (b_x, d_x, u_x, a_x)        b + d + u = 1            (Additivity, eq. 8)
E(x) = b_x + a_x · u_x                                    (Expectation, eq. 9)
```

`b` belief, `d` disbelief, `u` uncertainty, `a` base rate (prior). "Although an
opinion has 4 parameters, it only has 3 degrees of freedom."

Multinomial opinion over K classes: `ω = (b, u, a)`, `Σ b_i + u = 1`.

The interval reading: `[b_x, b_x + u_x]` is the belief/plausibility interval;
`u_x` is its width; `a_x` picks the point inside it that `E` reports.
Vacuous opinion `u = 1` is `[0, 1]`. Dogmatic opinion `u = 0` is a point.

### The bijection to Beta / Dirichlet (eqs. 12–15, W = 2 convention)

```
α = r + 2a,  β = s + 2(1 − a)                r positive evidence, s negative
E(P) = (r + 2a) / (r + s + 2)
r = 2 b_x / u_x,   s = 2 d_x / u_x,   a = a_x
b_x = r / (r + s + 2),  d_x = s / (r + s + 2),  u_x = 2 / (r + s + 2)
```

Example: `ω = (0.7, 0.1, 0.2, 0.5) ↔ beta(8, 2)`. For a K-roster the same map
gives a Dirichlet with `α_i = r_i + W a_i`. Evidential deep learning (lineage 07)
emits exactly this with `W = K`, `a_i = 1/K`.

**Consequence.** The evidence pair `(r, s)` (or vector `r`) is a sufficient
statistic for the interval. Interval width `u = W / (r + s + W)` is a pure
function of total evidence. This is identical to Walley's IDM predictive
interval `[n/(N+s), (n+s)/(N+s)]` with `W = s`.

### Operators (Belief Calculus §5–6)

- **Negation**: `b_¬x = d_x, d_¬x = b_x, u_¬x = u_x, a_¬x = 1 − a_x`. Exact.
- **Addition** (disjoint x, y): `b = b_x + b_y`, `a = a_x + a_y`,
  `u = (a_x u_x + a_y u_y)/(a_x + a_y)`, `d = [a_x(d_x − b_y) + a_y(d_y − b_x)]/(a_x + a_y)`.
- **Multiplication (AND)** of independent x, y:
  `d = d_x + d_y − d_x d_y`,
  `b = b_x b_y + [(1−a_x) a_y b_x u_y + a_x (1−a_y) u_x b_y] / (1 − a_x a_y)`,
  `u = u_x u_y + [(1−a_y) b_x u_y + (1−a_x) u_x b_y] / (1 − a_x a_y)`,
  `a = a_x a_y`.
- **Comultiplication (OR)**: dual under `b↔d, a↔1−a` (De Morgan holds).
- **Division / codivision**: inverses with preconditions on base rates.
- **Cumulative fusion** (two independent sources, same proposition): add
  evidence. In `(r, s)` form it is literally `(r₁ + r₂, s₁ + s₂)`. This is a
  commutative monoid. **Averaging fusion** (dependent sources): average evidence.
  Weighted fusion, belief-constraint fusion (Dempster's rule generalized):
  van der Heijden et al. (arXiv:1805.01388, FUSION 2018) give multi-source,
  associative definitions and correct earlier formulas.

### Exactness

Multiplication is an approximation of the true Beta-product: "Non-Bayesian
coarsenings will cause the product and coproduct of opinions to deviate from the
analytically correct product and coproduct," though "the magnitude of this
deviation is always small." What is preserved exactly is the expectation:
`E(opinion expression) = probability expression` (eq. 42). Operators may yield
illegal values, handled by "clipping" while preserving `E` and `a`.

**Distributivity fails** (eq. 41): `ω_{x∧(y∨z)} ≠ ω_{(x∧y)∨(x∧z)}` "because if
x, y and z are independent, then x∧y and x∧z are not generally independent."
No statement on associativity. So the opinion algebra is **not a semiring**
either; it is a calculus.

## Where the evidence representation is strong

- Fusion of independent judges is exact, associative, commutative, and
  monotone in total evidence: a fold over a group is a vector sum.
- Width has a semantic: total evidence. `[0,1]` is zero evidence, a point is
  infinite evidence. This is what "confidence" *should* mean.
- Emitted natively by evidential networks; identical to IDM statistics.
- Base rates give a principled way to report one number when forced (`E`).

## Where it is weak

- Logical connectives are approximate; no coherence theory for iterated
  conjunction; independence is assumed, not chosen.
- The type has a hyperparameter (`W`, or `s`) baked in. Two stores with
  different `W` disagree on width for the same counts.
- Comparison of two opinions is not canonical (compare `E`? compare intervals?).
- For a K-roster it commits to a Dirichlet, a specific shape of credal set,
  whereas de Campos intervals commit only to a box.

## Cuzzolin's position (arXiv:2104.06839)

Argues for **random sets / belief functions** as the compromise: "strong
evidence that observations are inherently set-valued provides support for the
theory of random sets." Rejects "no single best model." Notes IP's generality
has had "limited impact" and that "generality, after all, is not the real
thing." His own hierarchy places belief functions as ∞-monotone capacities,
below 2-monotone capacities, below lower/upper probabilities, below coherent
lower previsions ≈ credal sets. Also: "Dempster updating yields probability
intervals contained within those from Bayesian updating."

## What this lineage decides

The **evidence pair/vector is the right boundary and fusion primitive**. It is
not the right connective algebra. Whatever the stored type is, it must be
derivable from `(r, s, W)` by the IDM/opinion map, and the fold for "two judges
on one proposition" is evidence addition.
