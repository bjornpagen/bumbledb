# Law registry and falsifiers

Companion to [the event revision](event-surface.md) and [retained process
proposal](research/process-theory.md). E01–E11 describe the current event representation
and queries. N01–N10
specify draft 0.3's named-process theory; its equality is not event equality.
P01–P07 retain the exact finite-polytope subtheory from draft 0.2; their convexity,
closedness, and rationality assumptions do not apply automatically to the wider
carrier. The original X01–X12 remain valid counterexamples on their stated examples.

**Cited** identifies a primary theorem. **Derived** means the argument is given here.
**Checked** means exact finite examples in `event-checks.py`, `process-checks.py`,
or the retained `spec-checks.py`; finite checks are not universal proofs or a
real-quantifier-elimination implementation.

Revision 0.8 adds [a fresh proof matrix](semantics/README.md). **Lean** below is
qualified by its actual finite/denotational premises and never means that native
Rust, canonical persistence or a general source solver has been verified.

## E01 — Pointwise keys and coverage prove normalized partitions

**Derived; finite cases checked; finite rational instance and the distinct-row
premises proved in Lean.** Fix an aligned event universe `W` and a
normalized probability measure `P`. For one scalar determinant, let the distinct
contributing rows denote measurable events `E1,...,En`. The pointwise key means
`Ei ∩ Ej = empty` for each distinct pair. Mutual coverage of `W` and the union
of those rows means `E1 ∪ ... ∪ En = W`. Finite additivity gives

```text
sum_i P(Ei) = P(union_i Ei) = P(W) = 1.
```

The argument holds separately for every normalized `P_theta` in a supplied
nonempty family of laws. It does not require lower or upper probability bounds
to sum separately to one. Covering a proper parent event `F` instead gives
`sum_i P(Ei) = P(F)`. Posterior normalization is a separate guarded operation.

This is the same set-theoretic partition argument as the cookbook's exact
interval partition. Green–Karvounarakis–Tannen §2–3 supplies event tables and
their set operations, not a theorem about bumbledb's pointwise admission rules.
The checker covers all 4,096 ordered triples of four-world events, with uniform
and nonuniform measures. Its 81 partitions include empty branches, which remain
valid stored Event values. Omitting those rows preserves world coverage but can
change scalar queries; it is an explicit sparse-view choice.

`FiniteMeasure.key_and_coverage` proves exactly one incident row from distinct
fact IDs, pointwise uniqueness and coverage. `schema_partition_mass` proves the
finite integer-mass identity; division by the positive common total gives the
finite rational law. `partial_partition_mass` retains proper-parent mass.
General parameter solving and arbitrary measure theory remain outside that module.

## E02 — Event algebra preserves relationships that masses erase

**Derived from set operations; finite cases checked.** In one aligned context,
union and intersection are associative, commutative, and idempotent; intersection
distributes over union. Relative complement satisfies `E ∪ !E = W` and
`E ∩ !E = empty`. Inclusion gives `E ⊆ F => P(E) ≤ P(F)`. These laws are exact
event judgments; probability is an observation of the event.

On four equally likely worlds, set `A={0,1}`. Choosing `B={2,3}`, `B=A`, or
`B={0,2}` gives the same pair of marginal probabilities `(1/2,1/2)`, but overlap
probabilities `0`, `1/2`, or `1/4`. Only the first pair partitions the universe.
Thus numeric normalization, equality of probabilities, and standalone Bernoulli
law equality cannot establish a partition or equality of events.

Union avoids double counting duplicate query paths: `P(E ∪ E)=P(E)`.
Unrestricted addition would yield `2P(E)`, and unrestricted multiplication for
self-conjunction would yield `P(E)^2`. Arithmetic evaluation requires its own
disjointness or independence premises. Kimmig et al. §3 discusses these
conditions for algebraic model counting. No universal arithmetic annotation is
added to ordinary joins by these event laws.

The structural parts of E01–E02 need an aligned universe; their measurement
consequences additionally need valid normalized laws. The current
[source descriptor](source-law.md), [representation](representation.md), and
[query algebra](query-algebra.md) specify the chosen finite presentation and
integration contracts; these set laws alone do not establish their implementation.

## E03 — Staged event queries preserve same-world truth

**Derived; finite query examples checked.** Given an ordinary relation of
bindings, let `E_r` denote the compatible event in binding r. Packing one group
g produces `U_g = union_{r in g} E_r`. Explicitly adding an empty seed changes
no nonempty group's value and creates an empty value for a seeded group with no
other contributors. It does not infer groups for missing parent records.

For a group-constant event A, distributivity gives
`A ∩ U_g = union_{r in g}(A ∩ E_r)`. De Morgan gives
`!U_g = intersection_{r in g} !E_r`, not the union of the complements. Repeating
an event, whether through identical or distinct relational bindings, changes
neither its union nor its self-intersection.

For each world w, `i(A ∩ B)(w) = iA(w)iB(w)` and
`i(!A)(w) = 1-iA(w)`. Taking their expectations under the retained joint law
therefore measures the Boolean query correctly without an independence premise.
`P(A ∩ B)=P(A)P(B)` is a different statement requiring independence.
Conditional measurement divides `P(E ∩ G)` by `P(G)` only on the positive-evidence
domain and preserves that denominator. Reusing G gives the same result.

[Query algebra](query-algebra.md) requires scoped empty values in query outputs:
`A ∩ !A` is a value whose complement is full. It must not drop its binding.
[The Coup query oracle](coup/query-checks.py) checks these consequences against
direct predicates on finite deals and trajectories, including counterexamples
to marginal multiplication, anti-probe negation, and complement-before-Pack.
This is not a verification of native Free Join execution or scope alignment.

## E04 — Canonical complementary partitions

**Derived; representation alternatives checked; denotational completion and
anchoring laws proved in Lean.** Fix nonempty legal support S. An Event determines
the partition `(S & E, S & !E)`. Its oriented identity must depend only on that
partition, whatever internal structure represents it.

The initial implementation uses a fixed decoder rho onto S that fixes S, and
canonically stores `E(rho(code))`. Retraction.exact_identity and its Boolean and
complement homomorphisms establish the denotational invariant. Full structural
interning plus signed roots supplies the proposed constant-time resident
equality/complement path. The Rust interner and persistent encoding still need
their implementation arguments and tests.

The earlier support-masked BDD pair is one correct alternative. Anchored single
representatives and completed functions also supply exact identity and one-bit
complement. A two-root pair is therefore not a required consequence of the
algebra. Hash equality alone never proves semantic or structural identity.

## E05 — Truth functions and Venn signatures

**Derived; exhaustive small cases checked.** A Boolean function of two inputs
is its four output bits indexed by `(a << 1) | b`; therefore BoolOp4 covers all
sixteen binary operations. On an event space, support masking makes its output
another event. Complementing the function toggles all four output bits.

The four Venn cells partition S. Their nonemptiness signature is therefore
nonzero. Complementing A exchanges cells whose indices differ in bit 1;
complementing B exchanges cells differing in bit 0; swapping operands exchanges
indices 1 and 2. These permutations justify the bit symmetries and lookup
predicates in [kernels.md](kernels.md). They do not determine event probabilities,
independence, or strong qualitative relation composition.

## E06 — Finite guards preserve the event quotient

**Derived, conditional on complete guard compilation.** Let the finite guard
roster distinguish all event predicates and explicit admissibility conditions in a
presentation. Quotient W by guard valuation and finite outcome assignment.
Every admitted Event is constant on each resulting cell. A cell belongs to
finite support exactly when its source-domain feasibility formula has a witness.
Thus equality of support-masked Boolean functions is equality of Events on W.

Guard bits have their defined truth values at theta; they carry no probability.
Weighted evaluation sums over stochastic outcomes while selecting guard cases.
Adding a redundant guard must not multiply probability mass. The constructor
must prove infeasible guard combinations impossible and refine/lift the scope
when new predicates are introduced. The finite checker exercises coin endpoint
guards in an explicit Supported(law) fixture; it does not implement general
real quantifier elimination. Positive mass is not the default admissibility rule.

## E07 — Domain and tests connect predicates to world relations

**Derived from finite relations; checked examples.** Let `R:S→T` and `Q:T→U`
have compatible owned faces and one shared parameter environment. Composition
is existential conjunction on the middle state. Identity, associativity,
distributivity over union, and converse reversal follow from finite quantifier
and set laws. `May(R,E)=Domain(R;TestRelation(E))`. Hence May composes and
preserves unions. `All(R,E)=!May(R,!E)` composes and preserves intersections.
`Must(R,E)=Domain(R)&All(R,E)` adds nonvacuity and need not compose: an R branch
with no Q successor disappears from R;Q while refuting nested Must.

The concrete-model lineage is Desharnais–Möller–Struth §6–7 and Struth 2015
§2–4. The checked BDD relational product is compared to explicit join/projection;
the native typed-face and guard-refinement implementation remains unverified.
ModalContract.may_composition and all_composition now prove the typed modal
laws. Its must_composition_exact proves the necessary extra
`All(R,Domain(Q))` factor, with a checked dead-branch counterexample.

## E08 — Residuation makes relational inclusion constructive

**Derived; 4,096 finite triples checked.** For compatible R, Q, V:

```text
R;Q <= V iff Q <= LeftResidual(R,V)
R;Q <= V iff R <= RightResidual(V,Q)
```

Expand the left inclusion to `forall s,t,u: R(s,t)&Q(t,u) => V(s,u)` and
regroup the quantifiers to obtain the residual definitions in
[world-relations.md](world-relations.md). The results are the largest compatible
relations; actual availability and nonempty domain remain separate requirements.
Uniform decision synthesis must keep existential action choice outside universal
hidden-state quantification. The forced-Coup fixture falsifies swapping them.
ModalContract now proves both typed adjunctions, permission soundness/maximality
on inhabited information cases, and the uniform-action counterexample.

## E09 — Information abstraction is an adjunction on observable regions

**Derived; all 15 partitions of four worlds checked.** For a complete disjoint
partition O, May saturates across cells intersecting E and Must keeps cells
contained in E. Thus `Must(E)<=E<=May(E)`; both are idempotent; and for any
whole-cell union H, `May(E)<=H iff E<=H`, `H<=Must(E) iff H<=E`.
Refining O reduces May and enlarges Must. Evidence-relative Must additionally
requires a cell to meet evidence. Saturations of different constrained-support
partitions need not commute. These claims use the set saturation in
Kohlas–Casanova–Zaffalon §6; they do not assert an observer knows inaccessible facts.

## E10 — Finite fixed points distinguish possibility, inevitability, and safety

**Derived; 4,096 relation/goal cases checked.** On a fixed finite state quotient,
monotone ascending or descending predicate iterations stabilize. Star is the
reflexive transitive closure. `μX.E|May(R,X)` equals `May(Star(R),E)`.
`μX.E|Must(R,X)` means every maximal path reaches E: dead ends outside E and
avoiding cycles fail. `All(Star(R),E)` instead means E holds throughout all
reachable states. These are structural properties, not stochastic hitting
probabilities. Arbitrary continuous-parameter transformations do not inherit
the finite termination argument. Game action quantifiers require the separate
arena and observation contracts in [world-relations.md](world-relations.md).
ModalContract proves finite-path reflexivity/transitivity through its constructor
and transitivity theorem, least reflexive-transitive containment, and May/All
unfolding. Native fixed-point termination and the maximal-path inevitability
algorithm remain implementation proof/test obligations; the finite reference
examples alone do not discharge them.

## E11 — Admissibility and source revision preserve different information

**Derived; exact finite falsifiers checked.** Empty implies zero probability
under a normalized designated law; the converse fails. Likewise full implies
probability one but probability one need not imply full. Reweighting can retain
all admissible worlds even when some receive zero mass. Explicit support
restriction can merge previously distinct events. Unmeasured spaces still
support structural operations and refuse probability with MissingLaw.
FiniteMeasure now proves the finite conditional-complement and evidence-domain
rules and retains kernel-checked zero-mass/nonempty and full-mass/nonfull
counterexamples. Its finite rational scope does not establish the general
posterior/likelihood updates in the following paragraph.

For a partition C and posterior target q, Jeffrey revision retains each defined
within-cell conditional and assigns cell mass q. It is idempotent on that same
partition; different partitions need not commute. Likelihood reweighting retains
the normalizing evidence and generally is not idempotent. Prior P(A)=1/5 and
target q(A)=4/5 yield 4/5 by revision, but likelihoods (4/5,1/5) yield 1/2.
A requested positive posterior on a zero-prior cell is undefined without an
additional within-cell law. These are the explicit support guards used by the
Jacobs and Jacobs–Stein update distinctions, detailed in
[typesafe-inference.md](typesafe-inference.md).

## N01 — Semialgebraic closure and exact judgments

**Derived using real quantifier elimination / Tarski–Seidenberg.** A finite kernel graph is a relation `G(θ,W)` with real matrix-coordinate variables. Its admission formula requires exactly one matrix at each `θ` in its source domain and requires nonnegative entries with row sums at most one.

Conjunction, disjunction, and coordinate renaming preserve semialgebraic relations. To compose two graphs, introduce matrices `U,V`, conjoin their graph formulas, impose `W_xz=Σ_y U_xy V_yz`, and existentially eliminate `U,V`. The resulting graph is semialgebraic, total on the intersected source domain, and single-valued. Its row sums are at most one because

```text
Σ_z W_xz = Σ_y U_xy Σ_z V_yz ≤ Σ_y U_xy ≤ 1.
```

Tensor is similar, with product entries and product row sums. Finite mixture, deterministic maps, and bounded diagonal likelihoods also preserve the class. The posterior graph uses `z>0` and `z q_y=w_y`; no unguarded division or topological closure is needed.

The exact image of a state is a projection of its graph. The convex hull of a semialgebraic set in `R^d` is also semialgebraic: Carathéodory allows at most `d+1` points, so membership has a finite existential formula with simplex weights. For the empty set the hull is empty. No closed hull is implied.

Feasibility, totality, functionality, graph equality, and implication are first-order real-arithmetic judgments. Quantifier elimination decides them. Rationally defined nonempty semialgebraic sets have real-algebraic witnesses; finite extrema/infima/suprema of the bounded payoff images are real algebraic. This does not imply rational witnesses, attained extrema, an efficient solver, or a finite equational rewrite presentation for the full signature.

## N02 — Composition retains the source assignment

**Derived; polynomial examples checked.** At a fixed aligned `θ`, kernels compose by ordinary finite matrix multiplication. Hence associativity follows from distributivity and reordering a finite double sum. Tensor symmetry and interchange follow by reordering finite products and sums. The same pointwise proof works when operands share parameters.

Source domains lift to a common scope and conjoin associatively. No proof freshens a shared source or resolves it separately at different intermediate outcomes. Reindexing by a well-typed deterministic parameter map commutes with matrix composition because it substitutes the same parameter expression everywhere.

Liell-Cock–Staton Theorems 5.6 and 6.2 explain why erasing the interface is not generally a compositional equality; their precise categorical hypotheses are recorded in `literature-resolution.md`. Our pointwise proof owns the particular continuous-parameter design here.

## N03 — Copy, sampling, and discard

**Derived; checked.** For a state `k=(p,1-p)`, copying its result gives `(p,0,0,1-p)`. Two independent draws at fixed `p` give `(p²,p(1-p),p(1-p),(1-p)²)`. Their off-diagonal entries differ whenever `0<p<1`. Therefore `copy∘k ≠ k⊗k` in general. Deterministic maps do preserve copy.

The deterministic diagonal is coassociative and cocommutative; deleting either copied coordinate returns the original value. Discarding a total kernel returns the unit effect because its row sums are one. Discarding a filtered kernel returns its row-sum effect, which can depend on the input and source. An unused observation cannot be removed through the total-kernel discard law.

For one retained event, `observe_E∘observe_E=observe_E` since indicators are idempotent. For two newly sampled observations, the joint likelihood generally multiplies two separate path factors; this is not assertion idempotence.

## N04 — Bernstein normal forms and repeated experiments

**Cited for the sampling-language presentation; derived identities checked.** Staton et al., *The Beta-Bernoulli process and algebraic effects*, Proposition 7 identifies its finite-output free-parameter terms with nonnegative rational Bernstein coordinate maps. Theorem 9 gives completeness for its full specified language. These results are not a theorem about all semialgebraic graphs.

A word with `k` heads among `n` draws has weight `p^k(1-p)^(n-k)` independent of its order. Summing the `choose(n,k)` words gives `B[n,k]`. The binomial theorem gives partition of unity. Multiplying by `p+(1-p)` gives the exact degree-elevation identity in the proposal. Linear independence gives uniqueness at a fixed degree. Ordinary monomial expansion gives a canonical unconstrained polynomial.

For constrained domains, coefficient equality is sufficient but not necessary. For example, `p²=p` on `p(p-1)=0`. Use N01's implication judgment for domain-relative equality. Do not call an arbitrary expression DAG or constraint-dependent decomposition a canonical byte form.

## N05 — Fairness with a retained success effect

**Derived; symbolic identity checked; also Piedeleu et al., Example 1.2.** Two draws at fixed `p` produce equal disagreeing-path weights `h=p(1-p)`. After filtering disagreement and retaining the first result the weights are `(h,h)`; evidence is `z=2h`. On `0<p<1`, normalization gives `(1/2,1/2)`.

Thus the unnormalized identity is

```text
extractor(p) = effect[2p(1-p)] ⊗ fairCoin,
```

with the evident unit-frame identifications. It is not equality to the total fair kernel. At `p=0` or `1`, evidence is zero. Over `[1/5,4/5]`, `z=1/2-2(p-1/2)²` has range `[8/25,1/2]`.

For different parameters the weights become `p(1-q)` and `(1-p)q`. Feasible endpoint assignments `(p,q)=(1,0)` and `(0,1)` give posterior heads one and zero respectively. Copying one draw instead gives zero disagreeing-path weights everywhere.

## N06 — Evidence retention and normalization

**Derived; checked.** Composing an input prior with an unnormalized kernel multiplies each input mass by its likelihood. Input-row normalization removes that factor and is not a congruence for composition. The proposal's prior `(1/10,9/10)` and row likelihoods `(1,1/2)` give posterior input heads `2/11`; early normalization gives `1/10`.

For a state, sequential filters multiply pointwise. Normalizing once at the end yields the same posterior as successive state conditioning when the retained joint model and all positive-evidence guards are respected. This state identity does not license replacement of a reusable input kernel by its rowwise normalized matrix.

Our equality retains absolute evidence. Piedeleu et al. Definition 2.11 instead quotients by one global positive scalar. Its theorem does not authorize per-input or per-source scalar erasure in our type. In particular, a factor depending on an uncertain source changes Bayesian evidence when that source is integrated against a prior.

## N07 — Exact Beta moments and binding scope

**Derived; checked against finite path sums and conjugacy.** For positive rational `a,b` and nonnegative integers `h,t`, the beta-integral identity gives

```text
E_Beta(a,b)[p^h(1-p)^t] = (a)_h (b)_t / (a+b)_(h+t).
```

All factors are rational. Integrating a finite polynomial in `p` therefore yields rational coefficients in its remaining variables. The bind requires a full independent `[0,1]` source domain for `p` and a polynomial integrand; arbitrary semialgebraic integration is not claimed.

The moment identity gives

```text
E_a,b[p f(p)] = a/(a+b) E_(a+1),b[f(p)]
E_a,b[(1-p) f(p)] = b/(a+b) E_a,(b+1)[f(p)].
```

These prove finite conjugate updating. The cited paper develops the algebra for its own parameter signature; positive rational shapes here follow from the displayed moment calculation, not an unqualified transfer of its whole completeness theorem.

Integrating `p²` once under Beta(1,1) gives `1/3`. Integrating each separate draw first gives `(1/2)²=1/4`, corresponding to fresh independent prior allocations. Integration must not be pushed through a tensor whose operands share the bound source. Observations precede the bind when they concern that source, and normalization follows it.

## N08 — Information operations survive, convex dual completeness does not

**Derived.** The gluing proof in P03 uses only existence of compatible marginals, so it also proves combination, projection, and scoped elimination for arbitrary semialgebraic law sets. Their closure follows from N01. On a fixed scope, intersection/union give the ordinary distributive lattice of sets; convexPool is a different operation with different laws.

Linear payoff infima are unchanged by convexification and closure. Hence they cannot identify a nonconvex or nonclosed model. Let `A={δ_H,δ_T}` and `B=Δ_{H,T}`. All one-draw payoff bounds agree. Reuse one law from each set for two conditionally independent draws: disagreement is zero throughout `A` and reaches `1/2` in `B`. Convexifying before that reuse is unsound. P05's converse characterization of refinement is restricted to its closed convex subtheory.

For support faces, the correct ordinary-union embedding remains `Face(R∪Q)=convexPool(Face(R),Face(Q))`, not plain union of faces. For independent fair facts, explicit independence is the polynomial constraint `q_HH q_TT=q_HT q_TH`; it is not implied by equal marginals. The general conditional-independence equations appear in the literature resolution.

## N09 — Open sets, exact answers, and distinct failure modes

**Derived.** For `p∈(0,1)` and output `coin(p)`, the lower heads probability is zero and upper is one; neither is attained. Replacing the source domain by `[0,1]` preserves these bounds but changes equality and membership. An exact bound API therefore needs attainment information.

A source satisfying `2p²=1` and `0≤p≤1` has the unique value `1/sqrt(2)`. Rational coefficients do not guarantee rational bounds or rational witnesses. The real-algebraic representation in N01 handles this case.

`D=∅` is conflict. `D≠∅, K=0` is a valid all-failing model. Posterior normalization of the latter is impossible. A zero-scope total model is unit mass on `()`; a zero-scope filtering model is an evidence scalar. Neither is represented by a relation with an empty outcome carrier.

## N10 — The Allen requirement and completeness claims

**Cited distinctions; design decision.** Dylla et al.'s definitions distinguish exact composition from weak qualitative composition. X08 refutes the proposed five-way geometric atom table as an exact relation algebra; it does not refute every possible finite probability abstraction.

The selected theory has (1) canonical polynomial equality in its free sampling fragment, (2) cited complete equational presentations for closely identified source languages, (3) direct compositional proofs above, and (4) complete real-arithmetic decision specifications for its finite constraint envelope. It does not yet have a proved finite axiomatization of every surface operation, a natural unique normal form for arbitrary semialgebraic domains, or an implementation of its full canonical persistence contract. A finite Allen-style atom table is not required or being promised.

## Retained finite-polytope subtheory

For P01–P07, frames are finite and nonempty, coefficients are rational, hulls are real convex hulls, and normal model arguments are nonempty rational polytopes. `∅` is the specified conflict completion. Their local operation `pool` means draft 0.3's `convexPool`; `mix` on independent law choices means the setwise mixture specified there, rather than silently identifying source ports in a process. Minimum/maximum attainment and rational vertices are properties of this subtheory.

## P01 — Closure and finite normal form

**Derived; cited for the unique-basis theorem.** A bounded rational polyhedron has finitely many rational vertices, and a rational polytope has a finite rational half-space/equality description. Inside a simplex, intersection and inverse image under marginalization remain bounded rational polyhedra. Linear images remain rational polytopes. Therefore combination, projection, pooling, fixed rational mixture, and deterministic pushforward are closed, including explicit conflict.

If `K=conv(V)` and `L=conv(W)`, then

```text
pool(K,L) = conv(V∪W)
mix_w(K,L) = conv{w v+(1−w)u : v∈V, u∈W},   0<w<1
push_f(K) = conv{f_*v : v∈V}.
```

For mixture, expand `p=Σα_i v_i`, `q=Σβ_j u_j` using product weights `α_iβ_j`. Conversely each candidate belongs to the convex Minkowski sum. Each vertex of a finitely generated hull belongs to every generating set, so the set of extreme points is its unique minimal generator set. Bonchi–Sokolova–Vignudelli, Theorem 1, is the general unique-base result; see [the paper](https://drops.dagstuhl.de/storage/00lipics/lipics-vol211-calco2021/LIPIcs.CALCO.2021.11/LIPIcs.CALCO.2021.11.pdf).

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

The labeled information-algebra connection is supported by Kohlas–Casanova–Zaffalon, Theorem 3, §7, and Theorem 12 in [the paper](https://arxiv.org/pdf/2102.13368v1). Direct finite proofs above own the exact definitions in this proposal, including marginal coupling semantics.

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

The active Event/relation rewrites follow E01–E11:

| Rewrite | Required evidence |
| --- | --- |
| Reorder/deduplicate Event union or intersection | All operands aligned and validated; ordinary scalar rows still have their own semantics |
| Reassociate relation composition | Matching faces, full fiber products, same shared environment; operand order is preserved |
| Fuse nested May or All | Checked compatible relation composition; the same rule does not hold for arbitrary partial Must |
| Construct a residual from an inclusion constraint | Exact universal/existential order, complete ambient product, and separate enabledness where required |
| Push a group-constant intersection through Pack | Distributivity with all group contributors retained; complement follows De Morgan instead |
| Evaluate a finite fixed point | Positive program on a sealed stable finite state quotient; no silently changing guard universe |
| Lift a Boolean expression through a map | Total functional graph on admitted supports; a surjective map additionally reflects equality and containment |
| Move existential elimination across a lift | Complete fibres for that specific projection square; support-total extension alone is insufficient |
| Reuse an old probability after extension | Pushforward equality for the designated law; structural support preservation alone is insufficient |
| Factor a dependency-obstruction self-join into coverage/conflict summaries | Distinct whole-fact identity, matching scalar determinants and exact union/intersection; retain the obstruction Event |
| Measure or differentiate a constructed Event | A designated valid law, positive-evidence/derivative domain, and retained coordinate roles |

The [map derivation](research/space-maps.md) and
[dependency-query derivation](experiments/event-repr-lab/DEPENDENCIES.md) supply
independent finite checks for the added map/admission rows. They are acceptance
contracts for future planner integration, not claims that the current native
planner already implements those Event rewrites.

The following table is the **retained source/polytope subtheory**, under the
specific N/P laws above. Its commutative combination is not world-relation
composition, and its convex-pool operations do not define Event union:

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
