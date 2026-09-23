# Law registry and falsifiers

Companion to [the proposal](/Users/bjorn/Documents/bumbledb/proposal/proposal.md). Status: draft 0.2. This registry specifies semantic equalities, not an implemented optimizer rule set. Unless stated otherwise, frames are finite and nonempty, coefficients are rational, hulls are real convex hulls, and model arguments are nonempty rational polytopes. `∅` denotes conflict where the completion is explicitly used.

Evidence labels: **cited** means the identified primary theorem; **derived** means a proof argument given here; **checked** means a finite exact example in `spec-checks.py`. Finite checks supplement proofs rather than replacing them. No machine-checked general proof is claimed.

## P01 — Closure and finite normal form

**Derived; cited for the unique-basis theorem.** A bounded rational polyhedron has finitely many rational vertices, and a rational polytope has a finite rational half-space/equality description. Inside a simplex, intersection and inverse image under marginalization remain bounded rational polyhedra. Linear images remain rational polytopes. Therefore combination, projection, pooling, fixed rational mixture, and deterministic pushforward are closed, including explicit conflict.

If `K=conv(V)` and `L=conv(W)`, then

```text
pool(K,L) = conv(V∪W)
mix_w(K,L) = conv{w v+(1−w)u : v∈V, u∈W},   0<w<1
push_f(K) = conv{f_*v : v∈V}.
```

For mixture, expand `p=Σα_i v_i`, `q=Σβ_j u_j` using product weights `α_iβ_j`. Conversely each candidate belongs to the convex Minkowski sum. Each vertex of a finitely generated hull belongs to every generating set, so the set of extreme points is its unique minimal generator set. Bonchi–Sokolova–Vignudelli, Theorem 1, is the general unique-base result; see [the local PDF](/Users/bjorn/Documents/bumbledb/proposal/papers/bonchi-sokolova-vignudelli-2021-convex-semilattices-unique-bases.pdf).

Canonical rational coordinates use reduced fractions. A canonical model encoding additionally fixes frame alignment and vertex order. The theorem does not make a raw generator list or a raw constraint list canonical.

## P02 — Complete rational convex-choice theory

**Cited for real weights; derived specialization for rational weights.** The nonempty fragment has operations `⊔=pool` and `+_w=mix_w`, with:

```text
K⊔L = L⊔K
(K⊔L)⊔M = K⊔(L⊔M)
K⊔K = K

K+_wK = K
K+_wL = L+_(1−w)K
(K+_pL)+_qM = K+_(pq)(L+_[q(1−p)/(1−pq)]M),  0<p,q<1
(K⊔L)+_wM = (K+_wM)⊔(L+_wM),                0<w<1.
```

Endpoints are `K+_0L=L` and `K+_1L=K`. Other boundary instances are reduced through those equations, avoiding division by zero. These are axiom schemas indexed by rational weights, not a finite list of probability constants.

Theorem 3 of Bonchi–Sokolova–Vignudelli presents the free convex semilattice with real weights. The proposed rational specialization follows constructively:

1. Distribute mixtures through finite pools. Every term becomes a pool of barycentric terms over outcomes.
2. Barycentric normalization identifies each such term with its rational distribution; zero coefficients are omitted. Standard associative regrouping uses only addition, multiplication, and division by nonzero sums of rational coefficients.
3. Semilattice order is monotone under mixture by distributivity. If `A` is the pool of generators and `v_i≤A`, any rational convex combination of the `v_i` is at most `A`, because a mixture of repeated `A` equals `A`. Pooling that combination into `A` changes nothing.
4. A rational point in the real hull of rational generators has rational barycentric weights: the nonempty rational feasibility polytope for those weights has a rational vertex. Thus all nonextreme rational generators can be removed using step 3.
5. Equal polytopes have the same unique extreme generators. Both terms reduce to their pool, with the same barycentric normal forms, so equal denotations are equationally equal. Soundness of each axiom follows from its set interpretation.

This is the proof obligation's proposed resolution, available for mathematical review. It does not claim a rational-specific theorem statement in the source paper. Nor does it establish completeness for the signature extended by intersection, conditioning, conflict, or recursion.

## P03 — Information combination

**Derived; checked for small examples.** Write `K⋈L=combine(K,L)` and `π_T=project_T`.

```text
K_S⋈L_T = L_T⋈K_S
(K_S⋈L_T)⋈M_U = K_S⋈(L_T⋈M_U)
K_S⋈K_S = K_S
K_S⋈L_S = K_S∩L_S
π_R(π_T(K_S)) = π_R(K_S),                    R⊆T⊆S
K_S⋈π_T(K_S) = K_S,                          T⊆S
π_S(K_S⋈L_T) = K_S⋈π_(S∩T)(L_T).
```

Associativity follows because either side is exactly the distributions whose three specified marginals satisfy the three input constraints. Nested projection is composition of marginal maps. Absorption holds because every distribution in `K` already satisfies every marginal consequence of `K`.

For elimination, one inclusion follows by marginalizing a feasible joint distribution. For the other, let `p_S∈K` and choose `q_T∈L` with the same overlap marginal `r`. Glue them on each overlap value `c` of positive mass:

```text
h(a,b,c) = p(a,c) q(b,c) / r(c).
```

For `r(c)=0`, use zero joint mass. The result has marginals `p` and `q`. This establishes the other inclusion without an independence assertion in the output: the proof constructs one extension to show existence; `combine` retains all extensions.

Combination is monotone in each operand under inclusion. Projection is monotone. Conflict is absorbing on the union scope. Combining with vacuity introduces unconstrained new axes; it is equality to the original model only when those axes were already present. The zero-scope vacuous model is the global typed unit.

The labeled information-algebra connection is supported by Kohlas–Casanova–Zaffalon, Theorem 3, §7, and Theorem 12 in [the local PDF](/Users/bjorn/Documents/bumbledb/proposal/papers/casanova-kohlas-zaffalon-2021-information-algebras-coherent-gambles.pdf). Direct finite proofs above own the exact definitions in this proposal, including marginal coupling semantics.

## P04 — Maps, relations, and variable identity

**Derived; cited for the choice homomorphism.**

```text
push_id(K) = K
push_g(push_f(K)) = push_(g∘f)(K)
push_f(K⊔L) = push_f(K)⊔push_f(L)
push_f(K+_wL) = push_f(K)+_wpush_f(L)
push_f(K∩L) ⊆ push_f(K)∩push_f(L).
```

The last inclusion becomes equality for injective `f` on the common frame. Simultaneous bijective axis/outcome renaming preserves all well-typed information operations. Scoped maps give computed results fresh identities and copy any retained axes unchanged. A noninjective axis substitution is not a rename: identifying two variables requires the diagonal equality constraint and then projection.

For ordinary finite relations, `Face(R)=conv{δ_x:x∈R}` consists exactly of distributions supported on `R`. Therefore

```text
Face(R natural-join Q) = Face(R)⋈Face(Q)
Face(project_T R) = π_T(Face(R))
Face(R∪Q) = Face(R)⊔Face(Q)                 same scope.
```

Lemma 6 of Bonchi–Sokolova–Vignudelli establishes the choice homomorphism; Example 4 shows why images of minimal bases may cease to be minimal.

## P05 — Dual observations and refinement

**Derived from finite convex separation; checked on examples.** For nonempty `K`, define `ℓ_K(f)=min_(p∈K)p·f` and `u_K(f)=−ℓ_K(−f)`.

```text
ℓ_K(c·1) = c
ℓ_K(f+c·1) = ℓ_K(f)+c
ℓ_K(a f) = aℓ_K(f),                           a≥0
ℓ_K(f+g) ≥ ℓ_K(f)+ℓ_K(g)
f≤g pointwise implies ℓ_K(f)≤ℓ_K(g)

ℓ_(K⊔L)(f) = min(ℓ_K(f),ℓ_L(f))
ℓ_(K+_wL)(f) = wℓ_K(f)+(1−w)ℓ_L(f)
ℓ_(push_g K)(f) = ℓ_K(f∘g)
K⊆L iff ℓ_K(f)≥ℓ_L(f) for all rational payoffs f.
```

Inclusion gives the forward refinement implication by minimizing over a smaller set. If inclusion fails, a rational separating hyperplane for rational polytopes supplies a payoff refuting the inequality. The full real-payoff version follows from ordinary separation as well. Because the target algebra `(R,min,weighted mean)` satisfies the choice axioms, assigning outcome payoffs extends uniquely to the lower-expectation homomorphism.

For an event `E`, `P_lower(E)=1−P_upper(Eᶜ)`. Expectation extrema are attained by vertices. Identical event intervals need not identify models; X05 gives a counterexample. Linearity of lower expectation is generally false: the minimizing distribution can differ between payoffs.

## P06 — Conditioning closure and sequential evidence

**Derived; checked including zero-evidence vertices.** Let `K=conv{v_i}`, let `z_i=Σ_xλ(x)v_i(x)`, and keep indices with `z_i>0`. Write `r_i=λv_i/z_i`. Then

```text
condition_λ(K) = conv{r_i : z_i>0}.
```

For a prior mixture `p=Σα_i v_i` with positive evidence, its posterior has weights `β_i=α_i z_i / Σ_jα_j z_j`. Conversely, given any convex posterior weights `β_i`, choose prior weights proportional to `β_i/z_i` on the positive-evidence vertices. That constructs the desired posterior exactly. Zero-evidence vertices add no new posterior points. Rational inputs give rational output vertices.

If all `z_i=0`, conditioning is impossible. Positive rescaling of `λ` changes nothing. Repeated fixed likelihoods compose by pointwise multiplication, provided the resulting observation is possible. Event conditioning is idempotent after a successful first conditioning.

`condition_λ(pool(K,L))` is the pool of the feasible component posterior sets; for this mathematical equality an impossible component contributes an empty set. At the diagnostic API layer, a wholly impossible observation still reports `ImpossibleObservation` rather than an ordinary model. No fixed-weight mixture law follows; X03 refutes it.

## P07 — Empty completion

**Derived from definitions.**

```text
K⋈∅ = ∅                 with union scope
K⊔∅ = K
push_f(∅) = ∅
π_T(∅_S) = ∅_T
K+_w∅ = ∅               0<w<1
K+_0L = L; K+_1L = K    including empty unused branches.
```

The extended-real lower expectation of conflict is `+∞`, so formulas with zero times infinity require endpoint handling rather than floating-point arithmetic. The public API propagates diagnostics and never uses these formal values for a decision.

## Counterexamples: forbidden general rewrites

### X01 — Intersection does not distribute over pooling

Let `K` be the fair-coin point and `H,T` the two Dirac points. Then `K∩(H⊔T)=K` but `(K∩H)⊔(K∩T)=∅`. Treating the convex-set lattice as Boolean or distributive is invalid. **Checked.**

### X02 — Arbitrary projection pushdown hides conflict

On binary axes `A,B`, let `K={(1/2,0,0,1/2)}` and `L={(0,1/2,1/2,0)}`. Their intersection is empty, but both projections onto `A` are the fair point. Thus `π_A(K∩L)≠π_A(K)∩π_A(L)`. Use P03's scoped elimination law, not unrestricted projection distribution. **Checked.**

### X03 — Conditioning is not refinement or fixed-weight mixing

For the precise distribution `(1/2,1/2)`, conditioning on the first outcome gives `(1,0)`; intersecting with its certain-support face is empty.

For `K={(3/4,1/4,0)}`, `L={(1/4,0,3/4)}`, and evidence `E={second,third}`, the posterior of their equal mixture is `(0,1/4,3/4)`. The equal mixture of their posteriors is `(0,1/2,1/2)`. In general fixed evidence masses `α,β` replace prior mixture weight `w` with `wα/(wα+(1−w)β)`. With uncertain masses, weights and posterior choices can remain coupled. **Checked.**

### X04 — Singleton bounds do not identify a model

`conv{(.5,.5,0,0),(0,0,.5,.5)}` and `conv{(.5,0,.5,0),(0,.5,0,.5)}` have the same coordinate intervals `[0,.5]`. The first two outcomes have event interval `[0,1]` in the first model and `[.5,.5]` in the second. **Checked.**

### X05 — Even every event bound does not identify a model

Condition the four-outcome box from proposal §6 on its first three outcomes. Its posterior `K` has six vertices: the permutations of `(.5,.25,.25)` and `(.4,.4,.2)`. The coordinate enclosure `L` has six vertices, the permutations of `(.5,.3,.2)`.

All eight subset-event intervals agree, since on three outcomes each proper nonempty event is a singleton or a singleton complement. Every point of `K` satisfies `q₁≤2q₂`, so `upper(K,(1,−2,0))=0`. The point `(.5,.2,.3)` belongs to `L` and gives `.1`, its maximum. **Checked against an independent half-space vertex enumeration.**

There is also a direct half-space derivation of the posterior: eliminate the evidence mass `z` from `1/10≤zq_i≤1/5` for each of the three coordinates. The result is `q_i≤2q_j` for every ordered pair. The induced feasible `z` already lies in `[3/10,3/5]`, meeting the fourth coordinate's bounds. The checker enumerates this separate half-space description and recovers the same six vertices.

### X06 — Convex strong product does not retain a factorization promise

The strong product of two vacuous binary models contains all four joint Dirac points, so its convex hull is the entire joint simplex. Restricting both marginals to `.5` afterwards gives every fair coupling. First restricting each factor to the fair point and then forming its product gives the single distribution `(.25,.25,.25,.25)`. Strong product is closed but this refinement-commutation rule is false. **Checked.**

### X07 — Repeated shared bias escapes finite polytopes

Two conditionally independent Bernoulli trials with common unknown bias `t∈[0,1]` give `(t²,t(1−t),t(1−t),(1−t)²)`. In coordinates `m=t`, `s=t²`, the convex hull is

```text
{(m,s): 0≤m≤1, m²≤s≤m}.
```

The lower bound is Jensen's inequality; the upper bound is `t²≤t`. At a fixed `m`, the lower point is the single-bias model, and the upper point is a mixture of biases zero and one. Mixing those two constructions attains every intermediate `s`. Strict convexity of the square makes each point `(t,t²)` extreme: any nontrivial mixture with different means has a strictly larger second moment. There are infinitely many such points. **Analytic proof; rational spot checks only.**

### X08 — Five region relations do not close under exact composition

For nonempty convex sets, equality, proper inclusion, proper reverse inclusion, disjointness, and overlapping incomparability (`PO`) are jointly exhaustive and disjoint. Nevertheless `PO∘PO` is not a union of these five atoms.

Take `A=C=[1/4,3/4]` and `B=[0,1/2]`; then `PO(A,B)` and `PO(B,C)`, so this equality pair is in the composition. But for `A=C={1/2}`, no `B` can be `PO` with `A`: if it intersects the singleton, it contains it. That equality pair is absent. A union of atoms cannot contain only some equality pairs. A weak overapproximation table is a different claim. **Analytic proof with checked witnesses.**

### X09 — Rounded products need not be associative

For scale `M=2^63`, define `a⊗b=floor(ab/M)`. Let `a=M/4`, `b=5`, `c=7M/8`. Then `(a⊗b)⊗c=0` and `a⊗(b⊗c)=1`. Directed rounding can be sound for enclosure while failing the exact algebra's equations. **Checked.**

### X10 — Monotone iteration need not terminate

`K₀=[0,1]`, `K_(n+1)={1/2+p/2:p∈K_n}` gives `K_n=[1−2^(−n),1]`. All steps strictly refine their predecessors and converge to `{1}` without reaching it. The finite rational polytopes also lack some infinite meets under inclusion: take rational `r_n` decreasing to an irrational `α∈(0,1)`, and the intervals `[0,r_n]`. Their set intersection is `[0,α]`. Every nonempty rational-polytope lower bound has a rational maximum strictly below `α`, and can be enlarged toward `α`; no greatest lower bound exists in the carrier. Finite closure is not completeness of the lattice. **Analytic proof; finite iteration checked.**

### X11 — Copying, rebinding, and independent draws differ

Copy a fair coin by `x↦(x,x)`: the joint distribution is `(.5,0,0,.5)`. Combine fair marginal models on two distinct axes: every fair coupling is allowed. Independently draw from the precise fair distribution twice: the result is the point `(.25,.25,.25,.25)`. Variable identity and process identity cannot be inferred from numerical equality. **Checked.**

### X12 — Arbitrary compact convex models need another conditioning decision

Replacing finite polytopes with arbitrary compact convex sets is not an automatic closure repair. Consider the compact convex hull of

```text
v(t)=(t,t²,1−t−t²),    0≤t≤1/2.
```

Condition on either of the first two outcomes. For a positive `t`, posterior mass on the first is `1/(1+t)`, giving values in `[2/3,1)`. Every feasible prior mixture with positive evidence still has positive second-coordinate mass, so the endpoint `1` is absent. All values in that half-open interval occur. The conditional set is convex but not compact. Taking its closure is a new semantic convention, with unattained limits replacing exact posteriors. **Analytic derivation; no general compact-set solver claimed.**

## What can enter a planner

| Rewrite | Required evidence |
| --- | --- |
| Reassociate, reorder, deduplicate combination | Same explicit variable/context bindings; denotational results only, provenance handled separately |
| Scoped variable elimination | Exact P03 scope condition; overlap retained |
| Fuse maps or pull a payoff backward | Total deterministic typed maps; retain source axes if future operations need them |
| Distribute mixture over pool | Same aligned scope, exact fixed weight, correct empty-branch treatment |
| Replace model by event/coordinate intervals | Explicit abstraction result, never an equality rewrite in general |
| Use product coupling | Separate justified product operation; never inferred from join order or equal values |
| Collapse redundant generators | Exact convex membership proof, not numerical near-equality |
| Return optimized bound | Feasible witness plus exact optimality verification or exact oracle result |

## Executable evidence

Run `python3 proposal/spec-checks.py`. It uses standard-library rational arithmetic, exact linear elimination, and active-constraint vertex enumeration on small bounded systems. Its independent extended formulation defines joint combination through marginal constraints. It checks selected laws against that formulation, computes conditioning examples against half-space descriptions, and preserves finite counterexamples as regression assertions. X12 is an analytic example, not an executable compact-set test.

This is a specification checker, entirely under `proposal/`; it does not modify or exercise the engine. The large-dimensional closure claims, complete algebraic presentation, and infinite-boundary arguments rely on their proofs and citations rather than finite enumeration.
