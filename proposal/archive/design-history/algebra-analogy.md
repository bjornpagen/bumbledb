> Historical design research. The [active proposal](../../proposal.md) supersedes recommendations here.

# The algebraic analogy to Allen

**Conceptual precursor.** The [active proposal](/Users/bjorn/Documents/bumbledb/proposal/proposal.md) and [law registry](/Users/bjorn/Documents/bumbledb/proposal/laws.md) develop this direction, specify rational coefficients and conflict, and add the closure and relation-composition counterexamples. Read those for the current specification.

The user's clarified criterion is algebraic unity. This revises the practical-first recommendation in `review-astra.md`. Representation cost should follow the choice of algebra. The strongest candidate is the **information algebra of finite credal models**, with its **convex-semilattice structure for probabilistic and unresolved choice**. A box of probability intervals is one representable shape inside that algebra, not the definition of the type.

## What the Allen implementation actually demonstrates

The source owns a qualitative coordinate system, then derives its execution:

* [The theory](/Users/bjorn/Documents/bumbledb/crates/bumbledb-theory/src/allen.rs:1) partitions nondegenerate interval pairs into thirteen endpoint-order configurations. Its 8192 masks are the Boolean algebra of predicates on those configurations.
* The palindromic numbering makes [converse](/Users/bjorn/Documents/bumbledb/crates/bumbledb-theory/src/allen.rs:158) `reverse_bits() >> 3`; complement is the complement of the low thirteen bits.
* [The NEON kernel](/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/exec/kernel/neon.rs:31) packs endpoint comparisons into a six-bit signature. A resident 64-byte table maps it to the basic relation; a second table evaluates the chosen predicate mask. The arithmetic does not depend on which of the 8192 predicates the caller wants.
* [The assembly gate](/Users/bjorn/Documents/bumbledb/scripts/check-asm.sh:41) checks the emitted classification and filtering routines for scalar flag operations and calls. The reference classifier and algebraic properties provide the semantic oracle.

The useful analogy is a complete mathematical account with canonical generators, symmetries, and laws from which implementations can be derived. Probability has to earn its own account. Reusing an interval's geometry while adding unrelated arithmetic operations does not establish one.

Allen's finite completeness is relative to qualitative endpoint-order configurations. Metric predicates, such as a ratio of durations, are outside that quotient. We should be equally explicit about what any proposed uncertainty algebra characterizes.

## The object: a piece of probabilistic information

Fix a finite frame of outcomes `X`. A value `K` is a nonempty finitely generated convex set of distributions on `X`. Geometrically, it is a polytope in the probability simplex. An explicit empty/conflict result extends the information operations when constraints disagree.

Several familiar objects are exact special cases:

| Knowledge | Geometric object |
| --- | --- |
| A known outcome `x` | The single Dirac distribution `{δₓ}` |
| A precise forecast | A single point in the simplex |
| A binary probability interval | A segment in the binary simplex |
| Any one of the outcomes in a set `R` | The simplex face `conv{δₓ : x∈R}` |
| Complete ignorance | The entire simplex |
| Coupled uncertain probabilities | A general credal polytope |
| Incompatible constraints | The empty set, represented explicitly as conflict |

This is a reason to choose the full object even before discussing its storage. It is closed under the geometric operations the intended semantics needs. Restricting the value to coordinate boxes discards some of those operations' exact results.

## The relational algebra is already there

For a set of named variables `S`, let `K_S` describe allowed distributions over their joint outcomes. Define combination with a value over variables `T` by

```text
K_S ⋈ L_T = {p over S∪T : marginal_S(p)∈K_S and marginal_T(p)∈L_T}.
```

It retains all compatible couplings. It does not assert statistical independence. Projection is the set of marginals:

```text
π_T(K_S) = {marginal_T(p) : p∈K_S},  T⊆S.
```

On the same frame, combination reduces to intersection. The resulting laws include

```text
K ⋈ L = L ⋈ K
(K ⋈ L) ⋈ M = K ⋈ (L ⋈ M)
K ⋈ K = K
π_R(π_T(K)) = π_R(K),                   R⊆T⊆domain(K)
π_S(K_S ⋈ L_T) = K_S ⋈ π_(S∩T)(L_T)
K_S ⋈ π_T(K_S) = K_S,                   T⊆S.
```

Ignorance is the unit for combination; conflict is absorbing. Refinement is reverse model inclusion. Reasserting a constraint does not amplify it. Joining knowledge and then forgetting variables has an exact projection law. These are precisely the sorts of laws that should determine a relational implementation.

This connection is established in the literature, not just an appealing analogy. In [Kohlas, Casanova and Zaffalon, *Information algebras of coherent sets of gambles*](/Users/bjorn/Documents/bumbledb/proposal/papers/casanova-kohlas-zaffalon-2021-information-algebras-coherent-gambles.pdf), Theorem 3 gives the labeled information-algebra axioms, §7 supplies the coherent-lower-prevision instance, and Theorem 12, p. 33, embeds that instance into a generalized relational algebra while preserving natural join and projection. In the finite case, linear previsions are ordinary probability distributions. Use the lower-prevision/strictly-desirable formulation: the paper warns that translating arbitrary desirable-gamble sets to lower previsions can lose distinctions about inconsistency.

Ordinary finite constraint relations also embed directly. Write `Face(R)=conv{δᵣ:r∈R}`. Then, on a common frame,

```text
Face(R) ∩ Face(S) = Face(R∩S)
conv(Face(R) ∪ Face(S)) = Face(R∪S)
f_*(Face(R)) = Face(f(R)).
```

Thus deterministic constraints, probabilistic constraints, and imprecise constraints have one geometric interpretation. This is an embedding of finite constraint relations, not a claim that the existing database automatically acquires possible-world tuple-existence semantics.

## Chance and incomplete knowledge have a complete equational theory

There are two different kinds of choice:

```text
K ⊔ L       = conv(K∪L)                       unresolved choice
K +ₚ L      = {pμ+(1−p)ν : μ∈K, ν∈L}         probabilistic mixture
```

The first is associative, commutative, and idempotent. The second obeys the barycentric laws:

```text
K +ₚ K = K
K +ₚ L = L +_(1−p) K
(K +ₚ L) +_q M = K +_(pq) (L +_[q(1−p)/(1−pq)] M),  p,q∈(0,1).
```

Their interaction is a distributivity law:

```text
(K ⊔ L) +ₚ M = (K +ₚ M) ⊔ (L +ₚ M).
```

These axiom schemas characterize the **free convex semilattice** on the outcomes. Expressions denote the same convex set exactly when the equational theory identifies them. This supplies a completeness result for this signature; it does not purport to axiomatize every additional conditioning or program-composition operation.

[Bonchi, Sokolova and Vignudelli, *Presenting Convex Sets of Probability Distributions by Convex Semilattices and Unique Bases*](/Users/bjorn/Documents/bumbledb/proposal/papers/bonchi-sokolova-vignudelli-2021-convex-semilattices-unique-bases.pdf), CALCO 2021, Theorem 3 establishes this presentation. Theorem 1 establishes a **unique minimal generating set**, the extreme distributions. The normal form is therefore mathematically determined. Exact encodings and a canonical ordering of those generators remain implementation decisions.

For a coin, these expressions are different by construction:

```text
Heads +_.5 Tails     a specified fair coin
Heads ⊔ Tails        an unspecified distribution over the two outcomes.
```

Assigning both a midpoint of `.5` would erase this distinction. The algebra retains it by construction.

Deterministic maps are homomorphisms for both choices. Relabeling or merging outcomes can therefore commute with these operations. The unique basis may gain redundant points after a map, so mapped generators still need reduction if a minimal representation is required; the CALCO paper explicitly gives that counterexample.

## The dual coordinates: probability models and guaranteed expectations

For a payoff vector `f`, define

```text
lower_K(f) = min_{p∈K} p·f.
```

The whole lower-expectation functional determines the closed convex set. In particular,

```text
K⊆L  iff  lower_K(f)≥lower_L(f) for every payoff f.
```

This is the convex separation duality. A more informative model is exactly one that supports at least all the guaranteed expectations of the less informative model. Robust preference and probabilistic containment are two uses of the same geometry.

The choice operators have particularly clean dual equations:

```text
lower_(K⊔L)(f)    = min(lower_K(f), lower_L(f))
lower_(K+ₚL)(f)   = p·lower_K(f) + (1−p)·lower_L(f)
lower_(g_*K)(f)   = lower_K(f∘g).
```

That last equation is an exact rewrite between transforming distributions and transforming the question. Vertices, linear constraints, lower expectations, and symbolic expressions are related representations of one denotation, rather than unrelated interval utilities.

More strongly, giving every outcome a numeric payoff extends uniquely to a convex-semilattice homomorphism: interpret unresolved choice as `min` and probabilistic choice as weighted average to obtain the lower expectation. Interpret unresolved choice as `max` to obtain the upper expectation. These are consequences of the generating algebra. The family of all such payoff observations separates distinct closed convex models.

A small example: combine `P(A)=.8` with `P(B)=.7`. On the four outcomes `(AB,A¬B,¬AB,¬A¬B)`, the joint information is the segment

```text
conv{(.5,.3,.2,0), (.7,.1,0,.2)}.
```

Its projection onto the event `A∧B` is `[.5,.7]`. An independently justified coupling selects the product distribution `(.56,.24,.14,.06)` from this family. Reasserting `P(A)=.8` changes nothing. Adding `A⇒B` produces conflict, since it is incompatible with those marginals. Combination, query, refinement, and inconsistency all use the same model.

## What this does and does not inherit from Allen

The semantic target now has a canonical object, a unique finite basis for each finitely generated value, complete equations for its two choice operators, exact relational combination/projection laws, and a dual account of containment and expectation. This is the stronger analogy I should have investigated first.

There is no established thirteen-atom probability relation algebra demonstrated here. Equal, proper-subset, proper-superset, disjoint, and overlapping-incomparable form a natural five-way classification of nonempty credal regions, but that classification alone does not contain quantitative probability information, and I have not established a composition theorem for a proposed bumbledb relation mask. A finite qualitative layer would need its own scope and proofs.

The full structure is also not a Boolean algebra. For example, let `K` be the fair-coin point, `L` certain heads, and `M` certain tails. Then `K∩(L⊔M)=K`, while `(K∩L)⊔(K∩M)=∅`. Outcome negation is a relabeling/pushforward; it is not the set complement of a convex model.

Nor does an unnamed set of distributions retain every compositional dependency. The already-local `imprecise-probabilistic-programming-precisely-2026.pdf`, §3, explains why naming uncertain choices matters and why the ordinary convex-powerset monad is not commutative. Copying a sampled outcome, drawing again with a shared unknown bias, and drawing from a separately specified bias are different operations. Their distinction belongs in the algebra's domains or dependency structure, before selecting kernels.

## The serious alternative on algebraic grounds

Random-set/belief-function algebra deserves comparison for its own structure. It assigns mass `m(B)` to subsets `B` of a frame. Unnormalized conjunctive combination is subset-intersection convolution:

```text
(m₁ ⋆ m₂)(A) = Σ_(B∩C=A) m₁(B)m₂(C).
```

In commonality coordinates `Q(A)=Σ_(B⊇A)m(B)`, it becomes pointwise multiplication:

```text
Q_(m₁⋆m₂)(A) = Q₁(A)Q₂(A).
```

The equivalence follows immediately from `B∩C⊇A` iff `B⊇A` and `C⊇A`. This is an exact invertible change of coordinates on the finite subset lattice, with Möbius inversion recovering the masses. Keeping mass on the empty set retains conflict and makes the convolution total. Its unit is mass 1 on the whole frame. The normalized Dempster rule adds a division and is undefined at total conflict. The convolution formula and normalization appear in the existing `cuzzolin-2021-uncertainty-measures-big-picture.pdf`, §8.1.

That transform is a real analogue of the Allen coordinate-system insight. But the probabilistic interpretation is intersection of independent random-set evidence; combining a source with itself as a fresh independent draw generally changes it. It is not the idempotent accumulation of constraints used above, and it is not an arbitrary event's truth-functional AND. The choice between these algebras should turn on what an assertion and its repetition mean. A smaller representation is not the deciding argument.

**Revised direction:** take the full credal information/convex-choice structure as the semantic candidate. Compare the random-set convolution algebra against it on meaning and laws. Only then choose representations and compile their operations. The interval box can be an exact recognized subcase, and an explicitly lossy abstraction when needed; it should not set the limits of the type in advance.
