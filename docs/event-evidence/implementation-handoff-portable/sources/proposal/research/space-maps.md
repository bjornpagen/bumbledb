# Transport, quantifiers and measurement need different map proofs

This review accompanies the [Rust transport experiment](../experiments/event-repr-lab/TRANSPORT.md).
It resolves a substantive boundary: preserving the old possible worlds is
enough for a faithful Boolean lift, but does not grant every quantifier rewrite
or preserve a designated probability law.

## Primary sources read

| Source | Reading scope | Consequence |
| --- | --- | --- |
| [Abbadini and Guffanti, *On the Beck–Chevalley condition*, 2607.06386v1](https://arxiv.org/abs/2607.06386v1) | Introduction; Definitions 2.2 and 2.10; Remark 2.11; Example 2.12; Proposition 3.2 and its proof | Substitution is inverse image; existential quantification is direct image. A commuting square supplies an inequality. Equality requires the appropriate base-change condition; the powerset model satisfies it on pullback squares |
| [Pong, *Bundles of Probability Schemes*, 2605.03902v4](https://arxiv.org/abs/2605.03902v4) | Introduction; Section 1 and the three functors; Section 4, equations 14–17 and Proposition 7; Appendix A's trace/base-change calculation | Structural inverse image, unweighted fibre sum and conditional averaging are different operations. The paper's measured fibre product explicitly chooses conditional independence |
| [Van den Broeck and Darwiche, *On the Role of Canonicity in Knowledge Compilation*, 1404.4089v1](https://arxiv.org/abs/1404.4089v1) | Revisited Theorem 3 and “Canonicity or a Polytime Apply?” | Canonical compilation and compact transformation have distinct costs. The SDD results concern a fixed vtree and the specified compression rules; they are not universal impossibility claims about every data structure |
| [Darwiche and Marquis, *A Knowledge Compilation Map*](https://arxiv.org/pdf/1106.1819v1) | Revisited ordering definitions and Table 5 / Proposition 4.1 | Distinguish polynomial equivalence checking from cheap canonical normalization. The table supports EQ for OBDD as well as fixed-order OBDD; it does not promise cheap conversion into a particular order |

The two newly retrieved versioned PDFs, extracted text and SHA-256 hashes are
retained in [the source manifest](space-maps/sources.json). These are primary
preprints, not benchmark evidence. The statements above use the specified
definitions and elementary finite-set calculations, rather than treating a
paper's broad terminology as an implementation certificate.

## A total extension can still block a quantifier rewrite

Start with one binary coordinate X. Extend it with a **copy** Y, so the legal
worlds are `(0,0)` and `(1,1)`. The extension retains both old X values. Let E
mean X=1.

```text
hide X, then lift:    full in the extended space
lift, then hide X:    Y=1 in the extended space
```

The second operation retains Y, so it retains information about X. Treating
those expressions as equal would erase that dependence. Both source spaces
are finite, both algebras have exact complements, and the extension is
surjective onto the old space. None of those facts repairs the missing square.

For finite supports this has an exact characterization. For every retained
target context, each compatible old world must have a lift in that context.
This is stronger than asking whether each old world has a lift somewhere.
If the property fails, the missing old world supplies a singleton event that
distinguishes the two orders of evaluation. If it holds, every old existential
witness can be lifted in the retained context, proving the reverse inclusion.

[The independent checker](space-maps/checks.py) exhausts the small supports,
events and hidden-coordinate sets. It checks the automatic inclusion and the
equivalence between this fibre-completeness condition and commutation for all
events. [Raw results](space-maps/checks.json) include the copied-bit counterexample.

Abbadini–Guffanti's Remark 2.11 and Example 2.12 describe exactly this distinction
in the powerset model: a commuting square gives an inclusion; a pullback gives
the base-change equality. Their theorem about every Boolean hyperdoctrine over
`FinSet^op` concerns that particular category of variable contexts. It is not a
license to call an arbitrary square of restricted finite world supports a
pullback.

The [Lean base-change proofs](../experiments/event-repr-lab/lean/BaseChange.lean)
now check the criterion over arbitrary admitted-world types. For a commuting
square, inverse image followed by existential image agrees with the other route
for all Boolean predicates iff every compatible pair has a lift. Uniqueness
is unnecessary: covering the ordinary pullback's pairs suffices. A two-world
duplicate-lift example proves that this is strictly weaker than asking the
square itself to be a set pullback. The proof also covers universal elimination
with a nonempty-fibre guard, and the copied-bit counterexample.

This sharpens the capability a future optimizer may accept. A full fibre product
remains a sufficient constructor; a separately checked complete-fibre square
can grant the same structural rewrite. It does not alter the declared world
relation product or license arbitrary support restrictions. Duplicate witnesses
may change a probability law or a count, so structural quantifier commutation
still supplies no measured base-change certificate. The iff is a derived
powerset result; Abbadini–Guffanti provide the framework and the sufficient
pullback case, not a claim about these Rust kernels.

There is no need to give this proof an opaque new mathematical meaning. Write
U, V, F and G for the maps' total functional graphs. Then `Converse(U);V` is
the relation of pairs with a joint lift, and `F;Converse(G)` is the relation of
pairs compatible over the base. Lean now proves that their equality is exactly
commutation plus complete fibres. Given commutation, the remaining proof is
the ordinary containment

```text
F;Converse(G) <= Converse(U);V
```

Its failure is the region `(F;Converse(G)) & !(Converse(U);V)`: compatible pairs
with no witness. This keeps the optimizer's algebraic capability connected to
the proposal's existing inclusion and obstruction semantics. It does not imply
that the native macro accepts these expressions today or waive graph
functionality, totality, owner alignment, or declared-key requirements.

The proposal already requires a **full fibre product** for world relations.
This experiment makes the reason operational. An Event lift remains useful
when the square is incomplete; it simply does not license moving existential
elimination across it. A future `CoordinateMap` must separate its support
inclusion/totality proof from its projection-commutation proof. Record a checked
square or a full-fibre-product constructor, not a generic `safe_map` flag.

## Structural compatibility is not law compatibility

A full two-bit support can carry either independent fair bits or the strictly
positive joint law `(3/8,1/8,1/8,3/8)`. Both have fair marginals. The event
“both one” has probability `1/4` under the first and `3/8` under the second.
FD/IND coverage and structural source maps cannot choose between these laws.

Likewise, a structural extension may preserve every old possibility while
changing the old event's probability from `1/2` to `3/4`. A measured extension
therefore needs a separate pushforward-law equality: integrating out the new
outcomes must recover the designated old law. Unknown parameters remain named
and shared; their environments cannot be renamed or averaged implicitly.

Pong's equation 16 constructs a measured fibre product by dividing the product
of the two marginal densities by their shared base density. Its independence
within each fibre is an explicit premise. This is a useful optional source
constructor, not the probability interpretation of every structural product.
The paper also assumes **strictly positive** finite distributions throughout.
Our admissible zero-mass worlds and undefined zero-evidence observations remain
outside that assumption. They cannot be deleted to make the theorem apply.

## Implementation consequences

The first Rust transport checks finite binary outcome embeddings and support
projections. It does not claim general real-guard substitution, measured
extension, or arbitrary Beck–Chevalley certificates. Its native query replay
uses equivalent full-product spaces with the same semantic coordinates; a
physical order change is not the copied-coordinate counterexample above.

The data structure should keep three reusable proofs beside the same map:

1. Support inclusion and, when promised, surjectivity onto the old support.
2. The complete fibres needed by a particular quantifier/product rewrite.
3. The pushforward equality required by a particular law interpretation.

These are algebraic capabilities shared by admission, queries and source
construction. Preserving the distinction makes more operations available with
precise meanings; collapsing them into one unchecked cast would hide the very
dependence Event is intended to retain.

The [Lean readout proofs](../experiments/event-repr-lab/lean/ReadoutMaps.lean)
now establish an exact boundary for possibility reuse. Inverse image preserves
every Boolean possibility query iff the image of the admitted source support is
exactly the admitted target support. Singleton predicates prove necessity;
lifting witnesses proves sufficiency. The same hypotheses preserve every Venn
occupancy bit and reflect supported equality. Support inclusion alone suffices
for relative complement, while fixing a coordinate can remove possibility.
This elementary derived result is distinct from the paper's base-change theorem
and does not supply a quantifier-square or law certificate. See the
[retained proof check](../experiments/event-repr-lab/results/lean-check.json).

## A coordinate map is a special relation, not an unrelated callback

For finite spaces, a map `f:T→S` has graph `G:T→S`. Its totality and functionality
are already statements in the world-relation algebra:

```text
Domain(G) = Full(T)
Converse(G);G <= Id(S)
```

The second statement says that one T world cannot map to two distinct S worlds.
Its orientation matters: `G;Converse(G)` instead relates T worlds that have the
same image. Surjectivity adds `Range(G)=Full(S)`. Inverse-image lift is `May(G,E)`;
direct image is `Post(G,A)`. For a total functional graph, lift preserves Boolean
operations; for a surjective one it also reflects equality and containment.

Thus a checked renaming descriptor is a compact executable form of a relation
graph with these proofs. The general graph gives a reference semantics and a
possible fallback, while a coordinate permutation can compile to indexed moves
and word permutations. It need not be expanded to world pairs for execution.
This follows directly from finite relational composition and complements; the
powerset reindexing in Abbadini–Guffanti supplies the surrounding map framework.
It is a derived design consequence, not a claimed theorem about engine throughput.

For an observable `O:World→Case`, `O;Converse(O)` is exactly the equivalence
relation “same observation.” Lifting the image of E is its least saturation by
observable cells. The ordinary Event-valued case relation, with pointwise key
and full coverage, already defines this function. There is no need for a new
opaque information-partition scalar. In Coup, an admitted relation assigning
each legal deal to a visible case supplies both the case data and the map whose
fibres govern uniform decisions.

[An independent graph checker](space-maps/graphs.py) exhausts all 64 relations
from three worlds to two values. Exactly the eight total function graphs induce
the required Boolean inverse-image maps. It also checks surjectivity versus
faithfulness, the image/preimage adjunction and the observation equivalence
relation. [Raw evidence](space-maps/graphs.json) is finite mathematics; arbitrary
graph-backed CoordinateMaps and their native compilation remain proposed.

Keep the environment boundary explicit. An observation graph formed separately
at each parameter value gives a family of information relations at those values.
It does not automatically make a policy uniform across unknown parameters.
That broader requirement must retain and quantify those alternatives explicitly.

## Combining views retains their joint image

The [borrowed-product proofs](../experiments/event-repr-lab/lean/ViewProduct.lean)
apply this graph account to the proposed direct relational-product kernel.
For maps u, v and h out of admitted W, form the image relation
`J(t,x,y) = exists w: h(w)=t and u(w)=x and v(w)=y`. Reading A and B through u
and v and projecting through h is exactly contraction against J. Lean proves
that a replacement relation gives the same result for all Boolean A/B iff it
equals J. This is an elementary image/witness result derived here; the cited
papers do not state a performance theorem for this kernel.

Surjectivity of u and v separately cannot replace J by a Cartesian product.
Nor can an operation memo omit different maps merely because its operand roots
match. The finite counterexamples are now machine-checked. This connects the
code's view descriptors to a relation whose full meaning stays algebraically
available, without requiring that relation to be physically expanded. See the
[representation/acceptance brief](../experiments/event-repr-lab/VIEW-PRODUCT.md).
