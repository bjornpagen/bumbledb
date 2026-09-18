# The larger algebra: events, relations, and possibility

**Revision 0.9; normative direction.** Event's unary Boolean operations are one
fragment of a typed relational algebra. The design must expose the maps between
predicates and relations, not trap a powerful representation behind an opaque
probability getter. This expands the initial 0.5 draft after the request to push
the algebra as far as its mathematical representation supports.

The closest established program-algebra lineage is **Kleene algebra with domain
(KAD)**, in its concrete relational model. We also retain converse, relational
complement, residuals, and typed faces, which are additional structure. We do
not rename the database field or claim every abstract KAD has all these operators.
[The research record](research/algebra-resolution.md) explains the papers and
their expressiveness results.

Native implementation now includes
[checked finite maps](../docs/event-maps.md) and the
[structural relation core](../docs/event-relations.md): full legal products,
joint-fibre certificates, membership-FD roles, composition, modalities, residuals
and graph/readout conversion. [Safe diagram inspection and reconstruction](../docs/event-inspection.md)
now expose the carrier and its legal support. The [information layer](../docs/event-information.md)
adds readout saturation, evidence cases, FD factors and indistinguishability
relations. The finite host API does not yet
supply complete descriptor transport, query staging or parameter/source integration. This status does not change their mathematical contract.

## 1. The mathematical objects are public; their encoding is shared

An Event E on S selects admissible worlds. A world relation `R:S→T` selects
admissible pairs of worlds. It is an Event on a checked product presentation,
with input/output faces specifying how coordinates are bound. Higher-arity
constraints use more faces of the same region representation.

For example, a relation can describe:

- a legal game transition between positions;
- two worlds an observer cannot distinguish;
- assignments consistent with one interpretation of a model judgment;
- a candidate action and the states in which its continuations meet a goal.

`WorldRelation` is an owned typed view over an Event and its faces, not a new
probability carrier or a `Model<Outcome>` field. Application catalogs, cases,
actors and options remain ordinary relations and containments. Composing world
relations is conceptually a natural join on the middle face followed by
existential projection of that face.

The ambient product is the **full fiber product of the endpoint state spaces**:
its support is `H_S(theta,s) & H_T(theta,t)` on a common admitted parameter domain.
Game-transition constraints belong in R, rather than being hidden in that
ambient product. Otherwise relation complement, identity, residuals, and
composition could acquire different meanings. A triple composition presentation
likewise has support `H_S & H_T & H_U`. Source-domain alignment is explicit and
must leave a nonempty common domain; each endpoint has a nonempty fiber there.

Input/output role aliases are explicit coordinate maps. Duplicating a coordinate
name to describe two possible states does not draw another random sample.
Shared source parameters remain shared through composition. Two unrelated law
contexts never become probabilistically independent by forming a product face.

Concretely, the initial relation constructors are **parameter-preserving**:
`R(theta,s,t)` uses one shared environment theta and finite state coordinates
s and t. The product is fibered over that environment. Identity compares the
state coordinates; theta is already the same binding, rather than two reals
whose equality is guessed from their guard bits. A common guard refinement
must make each relation constant on its finite state/guard cells before a
relational product runs. It is unsound to combine a left witness at theta=0.2
with a right witness at theta=0.8 merely because both satisfy one coarse guard.

There are three map obligations: preserve admissible support, supply the
complete fibres for the specific projection/substitution square, and preserve
the designated joint law when measurement is requested. None implies all the
others. In particular, a total extension with a copied coordinate may retain
information that a proposed projection rewrite would erase. The
[map adjudication](research/space-maps.md) gives the exact finite criterion and
its primary-source basis. This is why ambient relation products are full fibre
products and why transition constraints stay in the relation Event.

General semialgebraic coordinate maps can be admitted through the exact source
layer by substituting formulas, eliminating real variables when required, and
recompiling the resulting guards. That operation has a solver obligation beyond
BDD bit renaming. Only the parameter-preserving, finite stable presentation
qualifies directly for the finite-closure kernel below.

A concrete implementation consequence is that `Id(S)` must not require listing
all states. On binary state faces, it is the conjunction of corresponding bit
equalities, masked by the admitted endpoint support. The [diagonal laboratory](experiments/event-repr-lab/DIAGONAL.md)
constructs it symbolically and checks relation laws over sixty coordinates.
It also retains a direct finite constructor: algebra specifies the diagonal,
while the representation supplies its physical construction. Preserving support
under face swaps alone is insufficient to prove a full product; a symmetric
constraint such as `X=Y=Z` is a counterexample.

A relation lifted into a larger workspace must also depend only on its declared
environment and endpoint faces. Full product support alone does not certify
that role: interpreting the Event `F(x,y,z)=z` as an XY relation makes `F;Id`
read y and violates right identity. Over a legal-domain product, the role can
be checked using the algebra itself: `Exists(scratch_faces, E) = E`, with the
result lifted back into the same support. This is supported Event equality;
the raw physical mask may still mention the scratch coordinates to encode
their legal values. The [legal-relation investigation](experiments/event-repr-lab/LEGAL-RELATIONS.md)
separates this dependency certificate from the workspace's support certificate.

## 2. Structural possibility is not a model's nonzero probability

**This revises 0.4's implicit positive-probability support.** The Event universe
is an explicitly declared admissible support S. A measurement law, when supplied,
is normalized on S and may assign zero mass to nonempty regions. Structural
spaces need not have a designated law at all.

This is necessary for a useful algebra of legal moves, epistemic alternatives,
and admissible relations. If a model assigns zero probability to a legal bluff,
the bluff must not become structurally impossible without a separate assertion.

```text
IsEmpty(E)           => Probability(E,Full) = 0, when measured
Probability(E,Full)=0   does not imply IsEmpty(E)
```

Probability one is likewise distinct from a structural guarantee. The Boolean
laws and pointwise FD/IND partition proofs still hold with zero-mass cells.
The denotational partition invariant `E & !E=empty`, `E | !E=S` is unchanged.
It does not require storing two roots.

A measured space captures exactly one designated law or parameterized law
family, so the existing `Probability(E,G)` remains unambiguous. Changing that
designation creates a new context with an explicit identity-on-worlds map.
An unmeasured structural product refuses Probability with `MissingLaw`; it does
not invent a product law. `Supported(law)` is an explicit derived event/view
where mass is positive, with guard refinement as necessary. Restricting to that
view is allowed when that is the intended question. It is never automatic.

This separates three questions: what is allowed, what a model predicts, and
what has actually been observed. Source assumptions can explicitly identify
some of these, but the representation does not silently identify them.

## 3. The operator basis

Coordinate maps and observation partitions fit this algebra too. A total
function graph G is characterized by `Domain(G)=Full` and
`Converse(G);G <= Id` on its result face. Reindexing E is `May(G,E)`; grouping
worlds with the same observation gives `G;Converse(G)`. A checked coordinate
permutation is an efficient representation of that graph, not a separate
semantic primitive. [The map derivation](research/space-maps.md) checks these
identities and keeps map-graph semantics separate from native implementation.

Let `R:S→T`, `Q:T→U`, and `V:S→U`. Complements of relations use the admissible
product universe, not the relation's selected region as its own universe.

| Operation | Exact relational meaning |
| --- | --- |
| `Id(S)` | Pairs `(s,s)` |
| `TestRelation(E)` | Identity restricted to states satisfying E |
| `R \| R2`, `R & R2`, `!R` | Union, intersection, complement on the same typed product |
| `Converse(R)` | Swap input/output faces: `(t,s)` iff `(s,t)` is in R |
| `Compose(R,Q)` | `(s,u)` iff some t satisfies R(s,t) and Q(t,u) |
| `Domain(R)` | States s with at least one R successor |
| `Range(R)` | States t with at least one R predecessor |
| `May(R,E)` | States having some R successor in E |
| `All(R,E)` | States whose every R successor is in E, including dead ends |
| `Must(R,E)` | `Domain(R) & All(R,E)`; nonvacuous guarantee of an allowed next outcome |
| `Post(R,E)` | Target states with a predecessor in E; `May(Converse(R),E)` |
| `LeftResidual(R,V)` | Largest Q with `Compose(R,Q) <= V` |
| `RightResidual(V,Q)` | Largest R with `Compose(R,Q) <= V` |
| `Star(R)` | Reflexive transitive closure on a sealed finite state presentation |

The fundamental connection is:

```text
May(R,E) = Domain(Compose(R, TestRelation(E)))
All(R,E) = !May(R,!E)
```

Tests send predicates into the relation algebra. Domain sends relations back
into predicates. This makes intermediate conditions constructible, not merely
assertable after the application happens to invent them.

Examples of exact laws, with all intermediate faces aligned; here Z is a third
relation after Q, and Q2 has Q's signature:

```text
Compose(Compose(R,Q),Z) = Compose(R,Compose(Q,Z))
Compose(R,Q|Q2)        = Compose(R,Q) | Compose(R,Q2)
Converse(Compose(R,Q)) = Compose(Converse(Q),Converse(R))
May(Compose(R,Q),E)    = May(R,May(Q,E))
All(Compose(R,Q),E)    = All(R,All(Q,E))
TestRelation(A&B)      = Compose(TestRelation(A),TestRelation(B))
```

All and Must deliberately differ at dead ends. Must does not generally satisfy
the same composition equation on partial relations. Returning vacuous safety
as “this plan successfully has an allowed outcome” would be a semantic bug.
The [finite checks](algebra-checks.py) retain that counterexample. The fresh
[modal contract proofs](semantics/ModalContract.lean) establish the exact rule:

```text
Must(R,Must(Q,E)) = Must(Compose(R,Q),E) & All(R,Domain(Q))
```

The final factor certifies that every R intermediate state has a Q successor.
The same Lean module proves both typed residual adjunctions, greatest uniform
permissions and finite-path closure unfolding. These denotations do not verify
native face admission, rule completeness or fixed-point termination; see the
[proof matrix](semantics/README.md).

## 4. Solve for admissible behavior with residuals

Residuals compute the largest relation compatible with a composition constraint:

```text
Compose(R,Q) <= V iff Q <= LeftResidual(R,V)
Compose(R,Q) <= V iff R <= RightResidual(V,Q)

LeftResidual(R,V)(t,u)  iff forall s, R(s,t) implies V(s,u)
RightResidual(V,Q)(s,t) iff forall u, Q(t,u) implies V(s,u)
```

This turns containment into a constructive question. Given a transition R and
an allowed overall behavior V, compute which continuations are permissible.
Existence is not enough when several hidden predecessor states share the same
middle observation: the continuation must satisfy the constraint for all of them.

Intersect a residual with actual availability and retain its domain. The largest
relation can still have no available continuation at a state. It is a permission
relation, not a normalized stochastic policy and not a magically chosen strategy.

More generally, named faces expose finite first-order constraint reasoning:
join constraints by shared coordinates, existentially project witnesses, and
use complement for universal quantification. Quantifier order is retained in
the plan. `exists action, forall hidden state` is a uniform decision;
`forall hidden state, exists action` grants the chooser information they may
not have. These are different queries even when their factors are identical.

In Coup, an action's outcome relation can be checked against “Alice remains
alive.” `All` finds its weakest liberal precondition; `Must` excludes absent
actions. A relation from visible information to hidden states lets the query
find actions satisfying that condition for every hidden state in a case.
The result is a relation of admissible actions that a model or application can
then choose among. No probability threshold is needed for that guarantee.

## 5. Finite reachability and fixed points are part of the target algebra

For an endorelation R on a sealed finite state presentation, `Star(R)` is exact:

```text
Star(R) = Id | R | R;R | R;R;R | ...
CanReach(R,E) = May(Star(R),E)
SafeThroughout(R,E) = All(Star(R),E)
```

Finite closure stabilizes, although its diagram may become large. We include
this structural operator in the target; it does not require permitting arbitrary
model-valued recursive Rust macro rules. A closure stage uses its own validated
finite carrier and monotone fixed-point program.

Two useful state-event programs are:

```text
CanReach(R,E)       = least X satisfying X = E | May(R,X)
InevitablyReach(R,E)= least X satisfying X = E | Must(R,X)
```

The second requires every maximal path to reach E, including rejection of
dead ends outside E and cycles that can avoid E forever. These are different
from `All(Star(R),E)`, which says E holds at every reachable state, starting now.
No fairness assumption is implicit. Other finite monotone predicate programs
can be specified by least/greatest fixed points with variance checked in the IR.

The termination argument needs a finite, stable state quotient. The
[bounded iteration proof](../crates/bumbledb-event/semantics/FixedPoint.lean)
establishes stabilization after at most N applications for N enumerated legal
states, together with least/greatest extremality. For relation closure the carrier
is legal endpoint pairs, so the generic bound counts pairs, not only states.

Source parameters can remain as a fixed shared environment. A common finite
roster and an operator acting separately in each environment supply a uniform
bound even when the environment itself is infinite. A finite guard-cell
presentation is one sufficient implementation certificate, not a theorem that
parameters must be discretized. The module proves that monotone mixing between
environments can defeat this bound even with one state per environment.
Arbitrary transformations of continuous
parameters do not qualify: iterating `x -> x/2`, for example, need not have a
semialgebraic transitive closure. Such an expression is refused by the finite
closure constructor, rather than approximated and called exact.

Structural reachability says some allowed path exists. It is not the probability
that a stochastic process eventually follows one. Infinite-horizon stochastic
observations require their own process semantics; they are not silently supplied
by finite relational Star.

For a supplied finite action arena `Step(s,a,t)`, a controllable predecessor is

```text
CPre(E)(s) = exists a, Enabled(s,a) and
                     forall t, Step(s,a,t) implies E(t)
WinningReach(E) = least X. E | CPre(X)
WinningSafe(E)  = greatest X. E & CPre(X)
```

Here the player chooses a at s, and the environment chooses any allowed t.
These formulas construct strategies in a fully observed finite arena; ranking
the least-fixed-point iterations supplies progress toward the goal. A strategy
must retain a witnessing action per admitted state and rank, rather than pick
arbitrary actions from different iterations. Safety requires a continuing
enabled action at every selected state. Terminal success can instead be modeled
with an explicit absorbing success state.

With partial observation, actions must be uniform over the current information
case. One-step uniform safety is a quantified relational query. Repeated play
requires updating the information state (including memory); a finite belief-set
arena is a possible exact construction, with up to exponentially many subsets.
Applying fully observed CPre directly to hidden worlds does not solve Coup.
The [Coup applications](coup/algebra-applications.md) show the uniformity test and
its concrete hidden-target counterexample.

## 6. Information is another relation, not another subsystem

Given an observation partition O, define `Indistinguishable(O)` to relate states
in the same cell. Then the [information operators](event-algebra.md) are:

```text
May_O(E)  = May(Indistinguishable(O),E)
Must_O(E) = All(Indistinguishable(O),E)
```

The relation is reflexive, so its domain is full and All equals Must. Its
equivalence-relation laws explain the closure/interior laws. Arbitrary transition
relations need not satisfy those laws. This is why different observation
partitions and different transition systems can use one core without conflating
their semantics.

A total observation function f has a functional graph F. Its indistinguishability
relation is `F;Converse(F)`. For another total observation g with graph G, the FD
`f -> g` is exactly the containment

```text
F;Converse(F) <= G;Converse(G)
```

The [information-readout proofs](experiments/event-repr-lab/INFORMATION-ALGEBRA.md)
show that this is also exactly the uniform containment order on the associated
May operators. Thus refinement of information, a functional dependency, and an
inclusion between ordinary world relations describe the same condition. No
separate opaque information type is needed. The uniform filter rule
`May_O(A & B) = May_O(A) & B` has the analogous exact membership-FD condition.

With evidence G, restrict the relation's **target** to G before May/All and use
Must when nonvacuity matters. A later pullback to actual remaining worlds is an
explicit intersection with G. Observer privacy and epistemic adequacy remain
constructor contracts; a relation named “Bob's information” proves neither.

## 7. Source channels are a compatible, distinct interpretation

A stochastic channel K has an allowed relation R and normalized per-input
probabilities, with K zero outside R. It may also be zero inside R. Channel
composition uses sum/product; structural relation composition uses exists/and.
Both preserve named input/output interfaces and use the same coordinate maps.

A deterministic map pulls an event back to an event. A stochastic channel pulls
an event indicator back to an effect `s -> P(E after K | s)`, generally numerical.
Our API exposes that result honestly as an exact observable/function. To obtain
an Event describing the actual outcome, retain the channel's named draw in the
joint space. This distinction prevents a lossy conversion from being presented
as a closed Boolean operation.

TypeSafe can supply a conditional channel over the structurally admissible
options. Relational algebra can first construct an admissible option set, then
the channel retains uncertainty among those options. Conversely, observations
of the channel's outcome update the joint law and answer questions about other
events. [The adapter contract](typesafe-inference.md) specifies those imports.

## 8. Concrete memory and lowering obligations

Keep `EventKey { space, region }` and exact canonical resident identity. The
[representation laboratory](experiments/event-repr-lab/REPORT.md) compares
anchored graphs, local tables, bitmaps and other carriers for that contract;
the relational plan does not require a BDD pair. Extend the checked descriptors:

```rust
struct WorldRelation {
    region: Event,             // Event on an owned product presentation
    input: FaceRef,
    output: FaceRef,
}
struct RelProductPlan {
    left_lift: CoordinateMapRef,
    right_lift: CoordinateMapRef,
    eliminate: CoordinateSetRef,
    result_map: CoordinateMapRef,
}
```

Face/coordinate references are owned indices into validated descriptors, not
user-supplied numeric offsets. Construction validates the region's dependency
on the declared faces, including any retained environment. Diagonal equality encodes identity and tests.
Converse is a checked face permutation. Composition uses a fused **relational
product**: lift, conjoin and existentially abstract middle coordinates, then
canonicalize the result relative to its output support. It need not enumerate
the represented states or materialize a Cartesian table.

`All`, residuals, and universal projection use complement plus existential
abstraction with the correct product support. Domain and image reuse that same
kernel. Closure uses canonical equality to detect stabilization. Dense cofactor
views admit word/NEON Boolean selection and reductions; published results belong
to their canonical resident owner. Probability remains a separate exact contraction.

The library exposes validated borrowed graph/face views, factorization,
coordinate maps, all these operators, and result certificates. Ordinary query
stages can consume or materialize their results. A custom source can participate
by supplying those contracts; it does not need to live inside an opaque model
callback. This is the required meaning of a porous implementation.

Free Join and BDD relational product operate at different levels: ordinary row
bindings versus compressed world coordinates. The planner may connect them via
factorization and elimination rules, but cannot inherit one algorithm's cost
guarantees for the other. Both are necessary parts of the implementation plan.
