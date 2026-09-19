# Retained named-source process theory

Research from draft 0.3. The [representation proposal](../proposal.md) owns the
active Event design; kernels here construct its source laws.

# Event proposal and named-source process research

**Active carrier revision: [Event: a region of possible worlds](../event-surface.md).**
An event field denotes a region of admissible worlds, optionally measured under
captured normalized source laws. This replaces
both the first-class-kernel surface and the later weight-and-capacity surface.
Section 9 reflects that revision. Sections 1–8 and 10–11 retain draft 0.3's
process mathematics and acceptance obligations; they are not a complete definition
of the new event field. In particular, process-law equality is not event equality.
Source/region integration, scope alignment, and persistence are now specified in
[the active representation](../representation.md) and [source-law.md](../source-law.md).
Revision 0.5 additionally specifies finite structural closure in
[world-relations.md](../world-relations.md); the limits on unrestricted stochastic
recursion below do not exclude that separately sealed operator.

**Draft 0.3 process semantics, retained before implementation.** A reusable
experiment is an open probability kernel with named unknown laws and exact
constraints. Its finite sampling fragment has polynomial normal forms; its exact
constraint envelope is semialgebraic. Finite credal polytopes remain useful
information views. These constructions supply laws and event probabilities;
they no longer define the public event carrier by themselves.

This replaces draft 0.2's conditional recommendation. The two substantive questions are resolved: **support repeated draws sharing an unknown law; do not require a finite qualitative relation table**. The [literature resolution](../literature-resolution.md) gives the papers, versions, theorem boundaries, and reasons. The [law registry](../laws.md) separates cited results, derived laws, and exact example checks. Earlier drafts are retained in [the archive](../archive/draft-0.2/proposal.md).

## 1. What the type should make possible

A coin has an unknown rate `p`. Draw it twice, keeping the rate fixed. Observe that the results differ. The first result is now exactly fair:

```text
P(HT | p) = p(1-p) = P(TH | p)
P(first=H | results differ, p) = 1/2, whenever 0<p<1.
```

This is an identity of experiments, valid without estimating `p`. If we copy one result instead, the observation is impossible. If we draw from two unrelated unknown rates, fairness is no longer guaranteed. If we query how often the experiment succeeds, the answer is `2p(1-p)`, so the filtered experiment is not simply interchangeable with an unconditional fair coin.

These distinctions should follow from the type's ordinary composition, copying, observation, and equality rules. They should not require separate confidence conventions or an application-specific inference escape hatch.

The analogy to Allen is the relationship between a denotation, its equations, and its execution. Allen's endpoint-order theory yields thirteen atoms and the existing machine lookup. Here named sampling yields polynomial identities; constraint composition yields exact quantified formulas. The finite-table shape belongs to Allen's particular theory. We inherit the demand that the mathematics own the implementation.

## 2. The object and its identities

There are two kinds of coordinates:

- **Outcome variables** range over finite, nonempty, law-bound carriers. They identify results of experiments or ordinary finite facts.
- **Source parameters** describe unknown laws. A Bernoulli source has a real coordinate `p∈[0,1]`; a categorical source has a vector in a finite simplex. Finite selectors can be represented by `s∈{0,1}`. A scope can constrain several sources jointly.

A reusable model has a finite source interface `Γ`, a finite input frame `X`, a finite output frame `Y`, and a denotation

```text
M = (Γ, D, K : X ⇝ Y)

D ⊆ Θ_Γ                         admitted assignments to source parameters
K_θ(y | x) ≥ 0
Σ_y K_θ(y | x) ≤ 1               for every θ∈D and x∈X.
```

`Θ_Γ` is the product of the declared parameter carriers. `K_θ` is one substochastic matrix for each assignment `θ`. Its missing mass records failed observations. A total model has every row sum equal to one. A state has input frame `1={()}`. The zero-output-variable frame is also `1`; it is not the empty set of outcomes.

**Finite exact description.** `D` and the graph of `K` must be definable by finite Boolean combinations of polynomial equalities and inequalities with rational coefficients. They are semialgebraic. Quantified real formulas are permitted as an intermediate description and have equivalent quantifier-free descriptions. The graph must specify exactly one whole matrix for each `θ∈D`; all matrix rows must satisfy the inequalities above.

This is an exact mathematical class, not an approximate solver format. Parameters range over real numbers, including irrational probabilities. Rational coefficients make descriptions finite; they do not impose a rational grid on unknown laws. Open parameter regions are allowed. Empty `D` is mathematical conflict; stored assessments must have nonempty `D`.

The basic sampling constructors produce **polynomial** entries. The broader semialgebraic envelope provides closure for exact constraint elimination, algebraic restrictions, and normalized views. We do not claim that every admitted semialgebraic kernel is expressible by a finite program made only of coin flips. A declarative exact kernel law is also a valid constructor.

### Identity is an interface contract

| Identity | What sharing means |
| --- | --- |
| Outcome carrier | Two coordinates use the same vocabulary; they need not be the same variable |
| Outcome variable | Two expressions refer to the same result; `A and A` is `A` |
| Named source | Repeated invocations use the same unknown law; they need not return the same result |
| Source allocation | A new captured source is created; it can subsequently be shared by several invocations |
| Assessment/context | Application identity, provenance, subject, and evidence scope |
| Model value | Extensional domain and kernel equality on an explicitly aligned interface |

A model template has indexed source ports. A query binding connects those ports to explicit source identities. Instantiating a fresh template freshens its allocated sources once; invoking an already bound model retains its captured sources and creates fresh outcome variables. Copying a computed outcome creates a deterministic diagonal. No operation infers one of these meanings from equal numerical values.

The complete binding travels with a bound model value. Projecting away unrelated assessment columns must not sever it. An unbound template is an explicit mode requiring binding before use. Formal port renaming may preserve template equality; it does not identify two distinct captured source identifiers. Comparing bound kernels pointwise requires those source interfaces to be explicitly aligned first.

Distinct unknown names do **not** themselves assert statistical independence. Their joint feasible region can constrain them together. The sampling product below explicitly states independence of new draws **conditional on the entire source assignment**. A probability distribution over sources, when wanted, is a separate declared prior.

The existing [law typing](../../ts/src/law.ts) supplies outcome carriers through positional containment, mirrors, capacity, and closed generators. Keys alone do not unify domains. [Query variables](../../ts/src/query/scope.ts) supply the precedent for explicit fresh identity. Source ports add a new kind of binding; they do not replace those rules with nominal domain names.

## 3. The compositional algebra

All formulas below lift parameters to the explicitly aligned union of source scopes. Shared names use one coordinate. Source-domain constraints are conjoined. A contradictory conjunction produces `Conflict`, not a zero-likelihood experiment.

### Sequential and independent composition

```text
(L ∘ K)_θ(z | x) = Σ_y K_θ(y | x) L_θ(z | y)

(K ⊗ L)_θ(y,v | x,u) = K_θ(y | x) L_θ(v | u).
```

Sequential composition sums over the distinct intermediate outcomes. Tensor composition makes the draws conditionally independent at fixed `θ`. Associativity, units, symmetry of independent composition, and the interchange law follow from finite sums and products. They remain true when a source appears in both operands: that parameter stays fixed in every term.

An outcome-dependent unknown policy is possible, but its dependence must be expressed by an indexed law table or explicit kernel. The evaluation order must not silently permit an uncertain choice to adapt to earlier draws. This is the important difference from an adaptive nondeterministic scheduler.

### Deterministic structure and observation

A total function `f:X→Y` denotes `K(y|x)=1` when `y=f(x)` and zero otherwise. This includes ordinary finite expressions, tuple construction, outcome renaming, and projection. In particular:

```text
copy(x) = (x,x)
discard(x) = ()
observe_E(x) = x with weight 1_E(x)
```

A comparator observes equality and returns the agreed value. Boolean connectives act on outcome variables inside such expressions. There is no universal t-norm pretending to infer a joint law from two unbound probability numbers.

Copy has the associative, commutative diagonal laws. **Sampling does not commute with copy.** Drawing once and copying differs from drawing twice. Discard is harmless after a total kernel, but discarding the result of a filtering kernel retains its success effect. Thus an optimizer can erase an unused total draw when the retained interface permits it; it cannot erase an unused observation.

A bounded likelihood `λ:X→[0,1]` is a diagonal filter with entry `λ(x)`. Two such filters multiply. Observing the same already computed Boolean event twice is idempotent because `1_E²=1_E`; performing two distinct trials with the same observed result generally squares a likelihood and adds evidence.

### Chance and unresolved alternatives

For a fixed common source domain, known chance is pointwise mixture:

```text
mix_w(K,L)_θ = w K_θ + (1-w) L_θ,       0≤w≤1.
```

It obeys barycentric symmetry, associativity, and idempotence. Source sharing is retained. For `0<w<1`, both component domains must hold. Endpoint constructors select the used operand and omit the unused operand's constraints.

Unresolved alternatives use an explicit **nonrandom selector source**. For `choose(K,L)`, introduce `s∈{0,1}`, use `K` when `s=1` and `L` when `s=0`, and guard their respective domain constraints by that selector. Reusing the resulting bound model retains `s`; allocating a new selector permits a fresh choice. Discarding the selector into a law view gives set union, not an automatic convex hull.

A known random branch and an unresolved branch therefore have different equations under reuse. Sampling a fair coin and reusing an unknown choice between the two certain outcomes are already distinguishable by two draws. Convex pooling remains available as an explicitly named information operation in §6.

## 4. The polynomial core and its normal forms

The Bernoulli source constructor is

```text
coin(p) : 1 ⇝ {H,T}
coin(p) = (p, 1-p).
```

The categorical constructor reads a simplex vector. Finite deterministic branching, sampling, mixture, observation, and marginalization generate polynomial kernel entries. A Bernoulli path with `h` heads and `t` tails from one source has factor `p^h(1-p)^t`. Different sources contribute different factors. Copying a result contributes no new sampling factor.

For `n` draws from the same Bernoulli source, the count basis is

```text
B[n,k](p) = choose(n,k) p^k(1-p)^(n-k).
Σ_k B[n,k](p) = 1.
```

Exchangeability groups equal-count paths. Degree elevation follows from multiplication by `p+(1-p)=1`:

```text
B[n,k] = (n+1-k)/(n+1) B[n+1,k]
       + (k+1)/(n+1) B[n+1,k+1].
```

The multivariate version is a product of these bases, or a multinomial basis for a categorical source. These coordinates explain the algebra: repeating a source increases its degree, and forgetting an unused trial is degree elevation in reverse.

Staton et al.'s Beta–Bernoulli paper, Proposition 7 and Theorem 9, supplies a complete equation theory for its stated sampling language. At a fixed degree the Bernstein coefficients are unique. For an unconstrained Bernoulli parameter cube, reduced expanded monomial coefficients give a degree-independent canonical polynomial representation. For a categorical simplex one can eliminate a chosen final coordinate using its normalization equation and then expand.

Extra source constraints change equality. For example `p²=p` on `p∈{0,1}`, but not on the full interval. Coefficient comparison alone does not decide equality under arbitrary constraints; the exact logical judgment in §8 does. No cited theorem is being presented as a complete finite rewrite list for the entire semialgebraic extension.

### Explicit priors and finite Bayesian updating

Unknown `p∈[0,1]` means no prior has been supplied. It does not mean `p` is uniformly random. A finite number of observed heads under that model can exclude zero-likelihood parameters; it does not justify inventing a concentrated posterior over rates.

The proposal includes a restricted exact prior constructor for polynomial experiments:

```text
withBeta(a,b,p,K) = integrate K over one shared p ~ Beta(a,b)
E[p^h(1-p)^t] = (a)_h (b)_t / (a+b)_(h+t),
```

where `(a)_h` is the rising factorial and `a,b` are fixed positive rationals. `K` must be polynomial in the bound `p`; its domain must allow the full `p∈[0,1]` independently of the remaining source assignment. Bind only after composing every experiment intended to share that prior allocation. Apply observations to the unnormalized polynomial first, then integrate, then normalize.

This constructor takes an explicit finite polynomial representation in the bound parameter, checked against the kernel graph when necessary. It does not infer an integration rule from an arbitrary semialgebraic formula.

This produces exact rational moments and captures conjugacy: after `h` heads and `t` tails the same process has `Beta(a+h,b+t)` posterior. Allocating one uniform Beta process and drawing twice gives `P(HH)=1/3`; allocating two independent uniform Beta processes gives `1/4`.

This constructor has an explicit mathematical domain. It is not arbitrary integration of semialgebraic functions, arbitrary truncated priors, or an unbounded recursive sampler. Those operations can introduce functions and constants outside the chosen exact class. No promise of that closure is made.

## 5. Evidence and posterior views

Close a filtering kernel by supplying its input model, obtaining unnormalized weights `w_θ(y)`. Define

```text
z(θ) = Σ_y w_θ(y)
D+ = {θ∈D : z(θ)>0}
posterior_θ(y) = w_θ(y)/z(θ),             θ∈D+.
```

The posterior view retains `D+`, the normalized law, and the evidence function `z`. Its graph can be expressed without a division operator: `z>0` and `z q_y=w_y` for every coordinate. It is semialgebraic. Zero-likelihood parameter assignments are excluded from that posterior; if all assignments have zero likelihood, return `ImpossibleObservation`.

The reusable object remains the unnormalized kernel. Normalizing separate input rows and replacing the original kernel is invalid: rows with different success probabilities carry information about the input. A concrete falsifier appears in §7 and in the law registry. Even a global positive rescaling is observable when evidence mass can be queried. Consequently our kernel equality is stricter than the projective equality used by the conditioned-circuit completeness paper.

A robust probability result is the infimum and supremum over `D+` of the posterior event probability. A Bayesian prior over parameters instead weights their contributions by `z` before final normalization; the restricted Beta constructor is one exact way to specify that. Merely retaining an unknown parameter does not invent a prior over it.

No closure is silently taken. Posterior families can be open, so bounds report whether their endpoints are attained. A rational input description can produce algebraic rather than rational bounds. `ImpossibleObservation`, an inconsistent source domain, and a feasible zero-weight row are different cases.

## 6. Containment and information views

There are two connected information languages. Both are relational, with explicit coordinate scope.

**Source constraints.** A formula `Φ(θ)` states which unknown law assignments are possible. Combine assertions by conjunction, retain alternatives by disjunction, and forget inaccessible parameters by existential quantification. A source implication is ordinary logical implication over the declared real parameter carriers. Independent-looking source names do not change these rules.

**Outcome-law views.** For a total state, its exact image is

```text
laws(M) = {q∈Δ_Y : ∃θ∈D, q=K_θ}.
```

For a posterior use `D+` and its normalized graph. This is a semialgebraic set of joint distributions; it need not be convex or closed. The view deliberately loses correlations with the forgotten source interface. It is a typed information value, not a replacement for a still-bound reusable model.

If later sampling from a law view is desired, `fromLaws(A)` explicitly allocates one new categorical law `q∈A` and samples it. That law can then be reused. It does not reconnect to source identities forgotten by `laws(M)`.

For views over outcome scopes `S,T`, the information operations are

```text
combine(A_S,B_T)
  = {q∈Δ_(S∪T) : marginal_S(q)∈A and marginal_T(q)∈B}
project_T(A_S) = {marginal_T(q) : q∈A}
union(A,B) = A∪B
convexify(A) = conv(A).
mixViews_w(A,B) = {w a+(1-w)b : a∈A, b∈B},  0<w<1.
```

Combination retains every compatible coupling; it makes no independence assertion. On one scope it is intersection. It is associative, commutative, and idempotent, with the same scoped elimination law proved in draft 0.2. These facts do not require convexity. Projection, union, finite mixtures, and convexification remain semialgebraic; Carathéodory's theorem bounds how many points a convex-hull witness requires in each fixed finite dimension.

The endpoint view mixtures select their used operand. Setwise mixture permits separate choices from its two views; it is not pointwise mixture with shared source ports. For nonconvex `A`, `mixViews_w(A,A)` need not equal `A`. Information union/intersection/forgetting use Boolean logic and existential quantification; stochastic composition uses multiplication and summation. Keeping these two kinds of elimination distinct is the connection between the probability algebra and the relational language.

For two outcomes, `interval(l,u)` is the exact law region `l≤q_H≤u`. A finite rational polytope is another exact subcase. The old `pool=conv(union)` is now named **convexPool**. Its convex-semilattice equations and unique extreme-point basis still apply to finite polytopes. General unresolved alternatives use `union`; neither operation is silently substituted for the other.

### Refinement and observations

View refinement is actual inclusion `A⊆B`. On a common source/input/output interface, process refinement is graph inclusion:

```text
M refines N  iff D_M⊆D_N and K_M(θ)=K_N(θ) for every θ∈D_M.
```

This means restricting uncertainty while preserving the process at each retained source assignment. It does not treat a smaller evidence probability as “more knowledge.” Output-law inclusion and pointwise process refinement are different typed judgments.

For a nonempty normalized law view `A` and a total rational payoff `f`,

```text
lower(A,f) = inf_(q∈A) Σ_y q(y) f(y)
upper(A,f) = -lower(A,-f).
```

These observations preserve the earlier robust-decision interpretation: `lower(A,u_a-u_b)>0` proves strict preference. But all linear payoff bounds identify only a closed convex hull. They no longer define equality of arbitrary nonconvex or open views, and certainly not equality of reusable named models. Exact membership and polynomial constraints preserve the additional distinctions.

Ordinary finite relations still embed as `Face(R)`, the laws supported on `R`. Natural join and projection become compatible coupling and marginal image. `Face(R∪Q)=convexPool(Face(R),Face(Q))`; it generally differs from plain set union of the two faces. This probability algebra does not silently change ordinary database rows into probabilistic existence events.

## 7. Acceptance examples

### Shared unknown law, copied result, and fresh laws

The following is proposed declarative notation, not an existing SDK:

```text
p = source(binaryLaw, interval(0,1))
x = draw(p)
y = draw(p)                    # fresh result, same source
m = model({x,y})
r = posterior(observe(m, x != y))

probability(r, x=H)            # exactly [1/2,1/2]
evidence(r)                   # 2p(1-p)
```

The joint law is `(p²,p(1-p),p(1-p),(1-p)²)` in order `HH,HT,TH,TT`. The success region is `0<p<1`. With `p∈[1/5,4/5]`, success probability ranges from `8/25` to `1/2`, and the posterior remains exactly fair. Over the full interval the lower success probability is zero; no uniform bound on retries is implied. This proposal specifies one finite trial and its conditioned result, not an implementation of an infinite retry loop.

Replace `y=draw(p)` by `y=copy(x)`: disagreement has zero weight everywhere. Replace it by `y=draw(q)` for a fresh unconstrained source: the posterior is

```text
p(1-q) / (p(1-q)+(1-p)q),
```

whose feasible range is `[0,1]`. Shared source and shared outcome identity are both necessary parts of the language.

### A distinction that a convex hull destroys

Let `A` permit only the two laws `δ_H,δ_T`; let `B=convexify(A)` permit all Bernoulli laws. Every single-draw linear payoff bound agrees for `A` and `B`. Allocate one law from each view and draw twice from that same law. In `A`, disagreement is always impossible. In `B`, it can have probability `1/2`.

Even retaining the full output hull of a shared-rate experiment does not retain its original rate identity for composition with another experiment. Its two-draw hull also has infinitely many extreme points: in coordinates `m=P(first=H)`, `s=P(HH)`, it is `m²≤s≤m`. Each lower-bound point is extreme by strict convexity of the parabola. A finite polynomial expression represents the experiment exactly; a finite vertex list cannot.

### Evidence must survive composition

Take an input bit with `P(H)=1/10`. A kernel always returns `ok`, succeeding with probability `1` on `H` and `1/2` on `T`. Retain the input bit alongside the result. After observing success:

```text
P(H | success) = (1/10) / (1/10 + (9/10)(1/2)) = 2/11.
```

Both separately normalized output rows are just `certain(ok)`. Replacing the kernel by those rows before applying the prior gives `1/10`, a different answer. Equality must preserve the unnormalized matrix, and input closing must precede posterior normalization.

### Facts need not be experiments

Given only `P(A)=4/5` and `P(B)=7/10`, `combine` gives the Fréchet family with `P(A and B)∈[1/2,7/10]`. It does not choose the independent value `14/25`. Adding the ordinary support constraint `A⇒B` conflicts, since it requires `P(A)≤P(B)`.

The same marginal distributions can instead occur in an explicitly constructed independent experiment. Its product semantics is valid because the constructor declares that sampling structure. Row joins alone do not.

### Decisions use coupled payoffs

For `P(first)∈[1/5,4/5]`, let action `a` pay `(10,100)` and `b` pay `(0,101)`. Their expectation intervals overlap, but the difference payoff `(10,-1)` has lower expectation `6/5`. The model proves that `a` is strictly preferable under every allowed law. Comparing separately computed intervals would lose the common uncertainty.

The earlier three-outcome example where **every event interval agrees but a general payoff separates the models** remains valid (`laws.md`, X05). Thus intervals fail even as complete views of all convex information, before the additional process distinctions arise.

## 8. Exact judgments and result types

Admission judges the proposed final state deterministically. It performs no random draws, model calls, or calibration. A source/model description must have a finite well-typed interface and satisfy:

```text
exists θ : D(θ)                                      nonempty assessment
forall θ : D(θ) implies exists exactly one W : G(θ,W)  total functional graph
forall θ,W : D(θ) and G(θ,W) implies
             W_xy >= 0 and sum_y W_xy <= 1            valid finite kernel
```

A total-model annotation strengthens row sums to equality. Graph construction and source constraints use exact rational descriptions. Constructor-local malformed values must be unrepresentable after decoding; global nonemptiness and schema implications are final-state judgments. Trusted construction does not exempt later cross-row source constraints from those judgments.

All these are first-order statements over real closed fields. For equal aligned interfaces:

```text
equivalent(M,N)
  iff forall θ,W : (D_M(θ) and G_M(θ,W))
                  iff (D_N(θ) and G_N(θ,W)).
```

View containment uses the same implication judgment on its defining formulas. Feasibility, functional graph validation, and equality therefore have complete decision specifications by quantifier elimination. This is our derived correctness argument, not a claim that an engine implementation or a full rewrite-completeness proof already exists.

Threshold queries can avoid explicit optimization. For a posterior bound `P(E)≥a`, prove that no feasible assignment has `z>0` and `sum_E w < a z`. Optimization additionally determines the tight endpoint and whether it is attained. A nonempty rationally defined semialgebraic feasible set has a real-algebraic witness. Finite bound endpoints are algebraic, but witnesses need not be rational. The old LP-only certificate contract is therefore restricted to the linear subcase.

| Result | Meaning |
| --- | --- |
| `InvalidDescription` | Ill-typed coordinates, malformed constants, nonfunctional graph, or invalid mass |
| `ScopeMismatch` | Incompatible carriers or unresolved source/outcome alignment |
| `Conflict` | Well-formed information has no common source assignment or allowed law |
| `ImpossibleObservation` | A feasible model has zero evidence weight everywhere for the requested posterior |
| `MissingAssessment` | Application data is absent; no model operation was performed |
| `Bounds(lower,upper,attainment)` | Exact rational/algebraic endpoints, with witnesses when attained |
| `ResourceLimit` | The exact judgment did not finish; neither success nor infeasibility |

A zero kernel over a nonempty source domain is a valid experiment that never passes its observations. It is different from conflict, whose domain is empty. A zero-output-variable filtered model is a scalar evidence function and must be retained. Today's face projection API rejects empty projections; this unit/effect case needs an explicit new IR representation.

## 9. Public surface

The active field is Event. See [the current proposal](../proposal.md) and
[event surface](../event-surface.md). This retained document specifies source
processes, not an alternative field API.

## 10. Retained process representation and execution obligations

For the retained process theory, the semantic payload is an immutable,
structurally typed object with four components: interface including captured
source bindings, source-domain formula, kernel graph, and mode. Polynomial
coefficient tables and finite-polytope bases are exact specialized coordinates.
An event value additionally needs its region predicate and a normalized ambient
joint context. The process payload alone is insufficient for event identity.
Application provenance remains in separate relations.

[The existing ValueType](../../crates/bumbledb-theory/src/schema.rs) is a flat structural enum. The new family must remain a structural leaf with a checked descriptor and owned payload. It cannot smuggle a general recursive host-language type or a process-local pointer into row identity. The exact byte layout, image ownership, and log encoding require a subsequent concrete interface design; this proposal does not pretend a symbolic model is already an existing 16-byte `WordPair`.

### Canonicality has two distinct obligations

For the polynomial fragment, reference canonical coefficients are specified in §4. Finite rational polytopes retain their unique extreme basis. For the full semialgebraic envelope, equality is equality of graphs and domains; it is decidable, but neither an input formula nor an arbitrary cylindrical decomposition is automatically canonical.

Persistence must supply a deterministic canonical representation or a rigorously specified mechanism meeting the engine's extensional row-equality and stable-byte invariants. Raw syntax hashing does not meet that contract. Decidability and the finite description alphabet imply an effective canonical representative exists: enumerate descriptions in a fixed length-lexicographic order and select the first equivalent one. This is an existence argument, **not the proposed storage algorithm or a natural geometric normal form**. A satisfactory concrete format remains a required design deliverable before implementation. It is not a reason to remove shared-law semantics.

Exact rationals use reduced numerator/positive denominator. Decimal input denotes its exact finite-decimal value. Explicit binary64 import denotes its exact binary64 rational. Derived real-algebraic numbers use a square-free defining polynomial, an isolating rational interval, and a specified real-root identity, normalized for the chosen wire format. Internal rounding must not change equality. Display enclosures can be rounded with stated direction.

### Execution follows the denotation

The planner may use matrix associativity, tensor symmetry, deterministic map fusion, Bernstein degree elevation, source-constraint implication, and scope-correct elimination. It may not duplicate a sample, freshen a captured source, erase an observation, adapt an unknown source to evaluation order, or convexify a live process interface.

Pure polynomial identities have coefficient certificates. Linear fragments admit LP certificates. General polynomial constraints require exact real-algebraic reasoning and independently checkable judgments appropriate to that theory. An approximate solver may propose a candidate; exact checking owns acceptance. A feasible witness alone does not prove an optimum or universal containment.

Free Join continues to operate on ordinary relations. Event construction and
probability evaluation require explicit typed operations; joins do not acquire a
universal probability multiplication annotation. The event revision gives Boolean
region operations a first-class role. Event identity and source-rate identity
remain different: reusing an event is idempotent, while repeated sampling from
one rate creates new outcomes and higher powers in the probability polynomial.

Follow the Allen development pattern after the interface is specified: scalar semantics, exact reference cases, differential and naive oracles, planner-equivalence checks, canonical persistence cases, and emitted-code checks for the actual kernels. No performance target selects the denotation.

## 11. Boundaries now chosen

The core owns finite law-bound outcomes, named reusable unknown laws, finite composition, exact constraints, evidence effects, and guarded posterior views. The restricted Beta moment constructor gives a principled finite Bayesian case. This settles the shared-parameter requirement without claiming an arbitrary continuous-probability programming system.

Existing relational recursion remains unchanged. A finite reusable model can describe every finite prefix of repeated sampling by a family of finite compositions; that is different from providing a general fixed-point operator. Infinite retries, unrestricted model-valued recursion, and limiting inference need separate fixed-point semantics. The finite semialgebraic carrier is not closed under arbitrary countable limits.

Ordinary negation remains stratified absence of rows. Event negation complements a finite event. Logical complement of a law constraint is available within its ambient parameter space, but does not mean the complement of a probability and need not produce a valid nonempty assessment.

Causal interventions are not inferred from an associative kernel factorization. Adaptive uncertain policies need explicit dependencies. Arbitrary continuous prior integration is outside the closed exact operations; its omission is a mathematical expressibility boundary, not a performance optimization.

### The TypeSafe/jev import boundary

A Noul point maps to a precise binary law; Choice supplies a normalized law on a bound roster; Score additionally needs an explicit rank or utility payoff. Preserve the original supplied values and provenance. Exact imports reject nonnormalized vectors unless an explicit upstream normalization transform has been requested and recorded.

Source `confidence` does not by itself determine a probability interval, Beta counts, or a prior over source rates. Independent evaluation of questions does not imply statistical independence of their propositions. Repeated invocations of a predictor do not automatically establish iid draws sharing a stable latent law. These are distinct assumptions to declare in the imported model.

Calibration, model calls, and empirical fitting stay upstream of admission. The database checks what the adopted model implies; it does not certify the calibration of a source by accepting its finite description. The retained [documentation evidence](../review-evidence/README.md) and [original review](../review-astra.md) support these import rules.

## 12. The proposal's current position

The [event revision](../event-surface.md) now owns the public denotation: regions
under normalized laws, with pointwise FD/INDs expressing disjointness and coverage.
The [literature resolution](../literature-resolution.md) remains the research basis
for reusable sources and experiments. Its rejection of kernel simplifications
does not prove that a kernel should itself be the stored event field.

The earlier shared-parameter examples, evidence retention, and distinction between
law and outcome identity remain acceptance requirements. The new event carrier
requires a complete source/region descriptor, scope/equality contracts, the
concrete empty-value API, constant-face IR, and canonical persistence. These are
substantive integration obligations, not merely syntax or byte-layout choices.

Before engine changes, reconcile those contracts in an end-to-end schema,
transaction, query, and persistence example. The [event checks](../event-checks.py),
[process checks](../process-checks.py), and retained [polytope checks](../spec-checks.py)
support their stated finite examples. They do not constitute native engine tests
or formal verification of the combined proposal.
