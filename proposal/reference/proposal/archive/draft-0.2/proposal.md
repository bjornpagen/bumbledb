# Proposal: a first-class algebra of probabilistic information

**Status: draft 0.2, for deep design review.** This is campaign scaffolding, not an implementation authorization or a standing repository rule. The candidate is the information algebra of finite rational credal models, equipped with the convex-semilattice operations for chance and unresolved choice. “Credal” is the working mathematical name; public naming remains open.

This document supersedes the interval-first recommendation in `synthesis.md` and the practical-first representation recommendation in `review-astra.md`. It develops `algebra-analogy.md`. The companion [law registry](/Users/bjorn/Documents/bumbledb/proposal/laws.md) contains proofs, preconditions, and counterexamples. [Decisions and review rounds](/Users/bjorn/Documents/bumbledb/proposal/decisions.md) record what is chosen provisionally and what must be settled before implementation.

## 1. The proposition

A probability interval should be one small instance of a more general object: **a piece of information about the distribution of a finite joint outcome**.

That object has three complementary descriptions:

1. A convex set of possible distributions.
2. A finite collection of exact linear constraints on those distributions.
3. A functional giving the guaranteed expectation of every possible payoff.

The third determines the first; finitely many rational constraints or extreme distributions describe the finite objects proposed here. These are coordinates on one denotation. A vector of lower and upper probabilities is an exact representation only for some shapes.

The algebra earns its place by relating knowledge, chance, ordinary constraints, and decisions:

```text
combine     accumulate compatible constraints
project     forget variables while retaining exactly their consequences
pool        retain unresolved alternatives, then take their convex hull
mix         choose between models with a specified probability
push        ask for the distribution of a deterministic function
lower       ask what payoff every allowed distribution guarantees
refines     ask whether one model permits only distributions another permits
```

An uncertain coin and a fair coin are different values. Repeating the same constraint adds no evidence. Combining two marginal forecasts retains all compatible dependencies. A decision can be provably better even when its individual expected-payoff interval overlaps its competitor's. These consequences come from the same object and laws.

### What the Allen analogy requires

The existing [Allen theory](/Users/bjorn/Documents/bumbledb/crates/bumbledb-theory/src/allen.rs:1) fixes thirteen endpoint-order configurations, Boolean masks over them, and their symmetries. The [NEON implementation](/Users/bjorn/Documents/bumbledb/crates/bumbledb/src/exec/kernel/neon.rs:31) packs endpoint comparisons into a six-bit signature, looks up the configuration, then evaluates the chosen predicate mask. The [assembly gate](/Users/bjorn/Documents/bumbledb/scripts/check-asm.sh:41) checks the emitted kernels.

The lesson is to find the mathematical coordinates and laws first, then derive execution. Here the analogous assets are a unique extreme-point basis, a complete equational theory for the two choice operations, scoped combination and projection laws, and a duality between model containment and guaranteed expectations.

This is **not yet a finite qualitative relation algebra analogous to Allen's thirteen atoms**. The obvious five-way classification of convex regions fails closure under exact relational composition; the counterexample is in `laws.md`, X08. If a finite complete relation table is the essential requirement, this candidate has not met it. Its case rests on the richer algebra above.

## 2. Semantic universe and identity

A scope `S` is a finite set of distinct variables. Each variable has a finite, nonempty outcome carrier inferred through the schema's existing laws. Its joint frame is

```text
Ω_S = product of the outcome carriers of the variables in S
Δ_S = {p ∈ R^Ω_S : p(x) ≥ 0 and Σ_x p(x) = 1}.
```

The full product is the ambient frame. A law excluding particular tuples is represented by a support constraint inside that frame, so it remains available to composition and consistency checking. Scope `∅` has the single outcome `()`, not zero outcomes.

A normal model is

```text
K_S = conv{p₁, …, pₙ},    n ≥ 1, pᵢ ∈ Δ_S with rational coordinates.
```

`conv` here is the real convex hull. There are continuously many admissible distributions; only the finite description uses rational numbers. This is not a finite probability grid. Arbitrary finite denominators are semantic values; an implementation resource ceiling cannot silently round them away.

The completion also contains `Conflict_S = ∅`. `Vacuous_S = Δ_S`. Ordinary stored assessments are proposed to require nonempty models; intermediate information operations return either a model or explicit conflict. The mathematics remains total where intersection needs an empty result.

### Four identities that must remain distinct

| Identity | Meaning | Source of identity |
| --- | --- | --- |
| Carrier | Which outcome vocabulary a coordinate ranges over | Existing positional law pairing and closed-roster generator |
| Variable | Which axis of the joint outcome is being described | Explicit scope binding, following fresh query-variable identity |
| Assessment and context | Which claim, subject, time, or experiment the row describes | Application keys and ordinary relational laws |
| Distribution value | Which convex set the assessment denotes | Exact extensional equality after frame alignment |

Two coin variables share a carrier and remain separate axes. Two assessments may have equal models and distinct provenance. Two occurrences of one variable identify one outcome; two newly bound variables do not. Equal numerical forecasts do not imply a shared uncertain parameter or an independent experiment.

The existing [law typing](/Users/bjorn/Documents/bumbledb/ts/src/law.ts:1) derives carriers from statements. Keys do not unify domains; containment, mirrors, and capacity pair corresponding face slots. Distinct closed generators cannot inhabit one carrier. This proposal introduces no separate nominal domain declaration. A proposed frame binding refers to already law-typed slots and records distinct axis identities. The source precedent for fresh variable identity is [query scope](/Users/bjorn/Documents/bumbledb/ts/src/query/scope.ts:112).

Pooling, mixing, equality, and direct refinement require the same aligned scope. Combining different scopes requires explicit identification of their common variables. A carrier mismatch is a typing error, not a probabilistic conflict. An outcome renaming needs an explicit checked map; two rosters cannot be treated as interchangeable because their labels happen to match.

## 3. Constructors and canonical meaning

All coefficients below are exact rationals. Decimal text is interpreted as its exact finite-decimal rational. An explicit binary64 import preserves the finite binary64 value exactly as a rational. Neither interpretation certifies a forecast's calibration.

| Constructor | Denotation | Admission condition |
| --- | --- | --- |
| `certain(x)` | `{δ_x}` | `x` belongs to the frame |
| `precise(p)` | `{p}` | Complete coordinate assignment, nonnegative, sum exactly one |
| `vacuous(S)` | `Δ_S` | Well-typed finite frame |
| `support(R)` | `conv{δ_x : x∈R}` | `R⊆Ω_S`; empty `R` gives conflict |
| `vertices(S,V)` | `conv(V)` | Every generator is a distribution on `Ω_S` |
| `constraints(S,H)` | `Δ_S ∩ {p : Ap≤b, Cp=d}` | Rational finite constraints; emptiness reported explicitly |
| `eventBounds(S,E,l,u)` | `{p∈Δ_S : l≤p(E)≤u}` | `0≤l≤u≤1`, typed event |
| `expectationBounds(S,f,l,u)` | `{p∈Δ_S : l≤p·f≤u}` | Rational total payoff; `l≤u` |

Finite rational vertex and constraint descriptions are equivalent because the simplex bounds the system. Constraints need not be independently reachable: the joint feasible set defines the value. Tight reported bounds come from that set. Input audit records preserve whether redundant or incompatible assertions were supplied.

A binary interval `[l,u]` is the segment with endpoints `(l,1−l)` and `(u,1−u)`. It includes point intervals. Existing temporal intervals are half-open and nonempty, so their classifier and value constructor cannot be reused for this semantic type.

A roster box is `Δ∩∏[l_i,u_i]`. It is an exact subcase. Its feasibility condition `Σl_i≤1≤Σu_i` and its closed-form tightening remain useful, but a general model need not remain a box after conditioning or mapping.

The unique minimal generator set is the set of extreme distributions. Generator order, duplicate generators, redundant constraints, and construction history do not affect model equality. After an outcome map, previously extreme generators may become redundant and need reduction again.

**Why take a convex hull at all?** The model deliberately represents uncertainty through all linear payoff observations. Convexifying alternatives leaves their lower and upper expectations unchanged. This forgets distinctions that nonlinear questions about a hidden parameter or a sampling process can observe. Section 9 draws that boundary explicitly; it is a semantic choice, not a storage shortcut.

## 4. The operations

### Combination and projection

For `K` over `S` and `L` over `T`, define

```text
combine(K_S,L_T)
  = {p∈Δ_(S∪T) : marginal_S(p)∈K and marginal_T(p)∈L}.

project_T(K_S) = {marginal_T(p) : p∈K},    T⊆S.
```

Combination retains **all compatible couplings**. It has no independence mode. On the same scope it is intersection. It is associative, commutative, and idempotent; a repeated assertion changes nothing. Projection is an exact image, including on conflict.

Different scopes have scoped units: combining with `Vacuous_T` vacuously extends a model to `S∪T`; when `T⊆S`, it leaves the model unchanged. Conflict is absorbing on the union scope. Projecting a feasible model to zero axes yields the unique distribution on `()`; projecting conflict yields zero-scope conflict.

The valid elimination law is

```text
project_S(combine(K_S,L_T))
  = combine(K_S, project_(S∩T)(L_T)).
```

It follows from gluing distributions that agree on their overlap. Projecting both operands arbitrarily before combining can erase a contradiction and is forbidden.

### Unresolved choice and chance

On the same scope:

```text
pool(K,L)  = conv(K∪L)
mix_w(K,L) = {w p+(1−w)q : p∈K, q∈L},  0≤w≤1.
```

`pool` is associative, commutative, and idempotent. `mix` obeys barycentric associativity, symmetry with the complementary weight, and idempotence. Mixture distributes over pooling. Together they form a convex semilattice.

```text
mix_1/2(certain(H), certain(T)) = the fair distribution
pool(certain(H), certain(T))   = every distribution on {H,T}
```

Pooling conflicting opinions expresses unresolved alternatives. Combining them treats both as constraints and may conflict. Mixing them requires an actual mixture weight. No default “fusion” picks among these meanings.

For the empty completion, `pool(Conflict,K)=K`. Interior-weight mixture with conflict is conflict. Endpoint mixtures select their used operand: `mix_0(K,L)=L` and `mix_1(K,L)=K`, even if the unused operand is conflict. These extensions are specified separately from the literature's nonempty free-algebra theorem.

### Deterministic maps and ordinary logic

For a total deterministic map `f:Ω_S→Ω_T`,

```text
push_f(K) = {f_*p : p∈K}
(f_*p)(y) = Σ_(x:f(x)=y) p(x).
```

This covers marginalization, event truth values, merging categories, copying an outcome, and a distribution of a finite-valued expression. `push` preserves pooling and mixture. Bijective maps preserve intersection; general maps need not.

A computed result receives a fresh variable identity. An axis retained under its existing identity must be copied unchanged. A global renaming can translate labels and all expressions that refer to them together; a local transformation such as `not A` cannot overwrite the meaning of the existing variable `A`.

To retain both input and result, use the graph map `x↦(x,f(x))`, equivalently combine with the ordinary deterministic graph constraint. A plain image may discard dependencies with the forgotten inputs.

Boolean `and`, `or`, and `not` operate on outcomes inside a joint model. For example `probability(K, A and B)` asks for bounds on the event where both variables are true. If only marginal models exist, combine them first; the result is the Fréchet family of couplings. Reusing `A` in `A and A` refers to the same variable and returns `A` exactly. Complementing an event is a deterministic map; complementing a convex region is generally outside the type.

### Refinement, observations, and decisions

```text
refines(K,L)      iff K⊆L                     same aligned scope
equivalent(K,L)   iff K=L
compatible(K,L)   iff combine(K,L) is nonempty
lower(K,f)       = min_(p∈K) p·f
upper(K,f)       = max_(p∈K) p·f = −lower(K,−f)
probability(K,E) = [lower(K,1_E), upper(K,1_E)]
```

Payoffs are total rational-valued functions on the frame. Their minimum and maximum are attained. Results should carry an attaining distribution on request. A strict robust preference for action `a` over `b` is `lower(K,u_a−u_b)>0`. Equality to zero is a boundary case; it does not establish strict preference. A robust optimum may not exist; the output can be a set of undominated actions under the declared decision rule.

Lower and upper bounds on conflict are not valid decision outputs. The internal extended-real convention `lower(∅)=+∞`, `upper(∅)=−∞` is only algebraic bookkeeping and must not leak into an ordinary probability or utility result.

### Conditioning

Conditioning observes an event or applies a fixed likelihood. For a nonnegative rational likelihood `λ` on the original frame:

```text
condition_λ(K)
  = { λ(x)p(x) / Σ_y λ(y)p(y) : p∈K, Σ_y λ(y)p(y)>0 }.
```

For an event, `λ=1_E`. The output retains the same frame with zero mass outside the event; a separate map can change the frame. The exact result is the convex hull of the normalized vertices whose evidence mass is positive. Vertices with zero evidence mass are discarded. If none survive, return `ImpossibleObservation`; if the input was conflict, preserve that diagnostic instead.

This is the proposed regular-extension convention: a positive upper probability suffices, even when lower probability is zero. It does not claim that an event with upper probability zero can be conditioned upon, or invent a posterior for such an event. Sequential fixed likelihoods compose by pointwise multiplication when their normalization is defined.

Conditioning and asserting certainty are different. Conditioning a fair coin on heads returns certain heads. Intersecting the fair distribution with the constraint `P(H)=1` returns conflict. Conditioning also changes mixture weights through the evidence probabilities; it is not a fixed-weight mixture homomorphism.

## 5. What makes the algebra coherent

The information operations have direct finite-dimensional proofs, collected in `laws.md`. The connection with information algebras is also established in Kohlas, Casanova and Zaffalon, [*Information algebras of coherent sets of gambles*](/Users/bjorn/Documents/bumbledb/proposal/papers/casanova-kohlas-zaffalon-2021-information-algebras-coherent-gambles.pdf): Theorem 3, §7, and Theorem 12. The relevant formulation uses coherent lower previsions or strictly desirable gambles; arbitrary desirable-gamble sets can carry distinctions lost by passage to lower previsions.

Bonchi, Sokolova and Vignudelli, [*Presenting Convex Sets of Probability Distributions by Convex Semilattices and Unique Bases*](/Users/bjorn/Documents/bumbledb/proposal/papers/bonchi-sokolova-vignudelli-2021-convex-semilattices-unique-bases.pdf), Theorems 1 and 3, give the unique basis and complete equational presentation for nonempty finitely generated convex sets with real weights. Our rational-coefficient specialization has a constructive proof sketch in `laws.md`, P02. We do not attribute that restriction verbatim to the paper.

The payoff duality is especially useful:

```text
K⊆L iff lower(K,f)≥lower(L,f) for every payoff f
lower(pool(K,L),f) = min(lower(K,f),lower(L,f))
lower(mix_w(K,L),f) = w lower(K,f)+(1−w) lower(L,f)
lower(push_g(K),f) = lower(K,f∘g).
```

Assign payoffs to outcomes, interpret unresolved choice as minimum and chance as weighted average, and the lower expectation follows compositionally. All such observations distinguish unequal closed convex models. The payoff operation is therefore an interpretation of the generating algebra, not an unrelated utility helper.

Ordinary finite constraint relations embed by `Face(R)=conv{δ_x:x∈R}`. Their natural join, projection, and union become combination, projection, and pooling. This embedding does not turn ordinary database rows into uncertain possible-world existence events; that would require a separately specified provenance semantics.

The complete choice theory covers `certain`, `pool`, and `mix`. It does not claim a complete equational presentation for conditioning, intersection, recursion, or arbitrary probabilistic programs. The combined structure is not distributive under intersection and pooling, and is not a Boolean algebra.

## 6. Three examples the new type should make natural

### Coupled facts, a consequence, and a contradiction

Suppose `P(A)=4/5` and `P(B)=7/10`. Combination yields, in the order `(AB,A¬B,¬AB,¬A¬B)`,

```text
K = conv{(1/2,3/10,1/5,0), (7/10,1/10,0,1/5)}.
probability(K,A and B) = [1/2,7/10].
```

Both endpoints have explicit joint-distribution witnesses. The independent distribution `(14/25,6/25,7/50,3/50)` is one interior possibility, not a default selected by the join. Reasserting either marginal leaves `K` unchanged. Adding the support law `A⇒B` makes it empty because it would require `P(A)≤P(B)`.

### A decision whose intervals overlap

Let `P(first)∈[1/5,4/5]`. Action `a` pays `(10,100)` and action `b` pays `(0,101)`. Their expectation ranges are `[28,82]` and `[101/5,404/5]`, which overlap substantially. But the payoff difference is `(10,−1)`, whose lower expectation is `6/5`. Action `a` is strictly better under every allowed distribution. An endpoint comparison of the separate expectation intervals cannot express this shared-model fact.

### Even every event bound can lose information

Start with four outcome masses, the first three in `[1/10,1/5]` and the fourth in `[2/5,7/10]`, and condition on one of the first three occurring. The posterior is

```text
K = conv{permutations of (1/2,1/4,1/4) and (2/5,2/5,1/5)}.
```

Its coordinate enclosure is `L={q:Σq=1, 1/5≤q_i≤1/2}`. On three outcomes all nontrivial events are singletons or their complements, so **every event probability interval agrees for `K` and `L`**. Yet for payoff `(1,−2,0)`, `upper(K,f)=0` while `upper(L,f)=1/10`. Replacing the model with event intervals has erased a real constraint. General lower expectations, rather than only belief/plausibility on events, carry the full information.

## 7. A high-level type in the relational language

The conceptual type is `Credal<Frame>`: a law-bound structural model family with first-class query operations. This notation is proposed; no such SDK or IR API currently exists.

The proposed authoring shape follows the spirit of [alternatives](/Users/bjorn/Documents/bumbledb/ts/src/alternatives.ts:45): a high-level view connects ordinary relations, keys, faces, and closed carriers. It must make each new numerical law visible. Existing containment cannot by itself enforce normalization, nonemptiness, or polytope inclusion.

For a categorical assessment, a relational spelling could have:

```text
Outcome(id, label, optional utility/rank payload)       closed roster
Assessment(id, subject, context, source)
Generator(assessment, generator)
Mass(assessment, generator, outcome, rationalMass)

key Assessment(id)
key Generator(assessment,generator)
key Mass(assessment,generator,outcome)
Generator(assessment) is contained in Assessment(id)
Mass(assessment,generator) is contained in Generator(assessment,generator)
Mass(outcome) is contained in Outcome(id)

proposed model law:
  every assessment has at least one generator;
  omitted outcome coordinates mean zero, by an explicit sparse encoding;
  every generator has nonnegative exact masses summing to one;
  the frame is Outcome.id as bound by the containment laws;
  the model denotes the convex hull of its generators.
```

All containment targets above have their required keys. A generator with no positive mass is invalid. A zero coordinate is a specified part of the sparse model convention, not generic database absence. Generator identifiers identify description rows and carry no mathematical probability meaning. A product frame generalizes the outcome field to a tuple of typed axis fields.

The example makes the semantics reviewable without committing to this physical storage. It needs an exact rational field or structured encoding, a new model-coherence judgment, and an unambiguous typed frame binding. Those are explicit additions, not capabilities already hidden in `capacity` or `alternatives`.

### Proposed query notation

The following is declarative pseudocode. `bind` associates an assessment with explicit axis variables and context; it does not mint a new nominal carrier.

```text
a = fresh variable over the law-bound binary outcome carrier
b = fresh variable over that same carrier

ka = bind(assessmentA, { outcome: a }, context)
kb = bind(assessmentB, { outcome: b }, context)
joint = combine(ka, kb)

find probability(joint, a=true and b=true), with witnesses
find lower(joint, payoff_a - payoff_b), with witness
find project(joint, [a])
find condition(joint, a=true)
find refines(joint, policyModel)
```

The context association must be supplied by query bindings or schema laws. Coincidental equality of `subject` strings cannot silently align unrelated assessments. Rebinding a forecast onto a second axis declares another marginal assertion; it does not declare repeated sampling.

Ordinary containment says projected database rows are included in another relation. Model refinement says every allowed distribution satisfies another model. Both may use containment language at the authoring level, but their typed IR cases and proof obligations must be distinct. A future schema refinement law must specify final-state evaluation, aligned frames, conflict policy, and diagnostics; it cannot be implemented by comparing sets of generator rows, since generator-set inclusion is stronger than convex-hull inclusion.

### Set semantics and folds

Rows remain sets. Grouping selects assessments; a specified fold then interprets the group. `combineAll` and `poolAll` are permutation-invariant and idempotent, so duplicate derivations are harmless semantically. Their empty folds are zero-scope vacuity and same-scope conflict respectively; an application may instead return a separately typed `MissingAssessment` before invoking a fold.

A weighted mixture fold has a different input: a finite relation keyed by component identity, with nonnegative weights summing exactly to one. Equal model values at different component keys still contribute their assigned weights. Zero-weight components are ignored. The weighted relation must retain its keys through joins so fan-out does not accidentally change weights.

Within one distribution, marginalization sums mass over **distinct original outcome coordinates**, including coordinates with equal numerical mass. Projecting away their identities and then performing set deduplication would give the wrong answer. The IR must preserve coordinates until this sum is complete. Expectation similarly multiplies each outcome's mass by its payoff before reducing.

No generic fold named `sum(probabilities)` or `mean(confidence)` is part of the model algebra. Summing event probabilities is justified by a disjoint partition; mixing models requires specified mixture weights; evidence counts belong to an explicitly named statistical constructor. A roster's declaration order is not a payoff: Score requires a rank or utility payload.

## 8. Exactness, admission, and execution

### Admission contract

A proposed change is judged on its final state. Input model descriptions must be well-typed, finite, rational, normalized, and coherent under their declared laws. A valid stored assessment is nonempty. Queries may return explicit conflicts between valid assessments. No model call, random draw, or calibration procedure runs inside admission.

Distinguish these outcomes:

| Outcome | Meaning |
| --- | --- |
| `InvalidDescription` | Malformed frame, coefficient, normalization, or map |
| `ScopeMismatch` | Incompatible carriers, unbound axes, or unresolved alignment |
| `Conflict` | Well-formed constraints have no common distribution |
| `ImpossibleObservation` | A feasible prior gives the likelihood zero mass everywhere |
| `MissingAssessment` | Application data was absent; no model operation was performed |
| `ResourceLimit` | The exact computation was not completed |

None is silently converted to a midpoint, vacuous model, or successful Boolean predicate. Resource exhaustion must not be reported as infeasibility. Minimal inconsistent cores are useful but not required for every conflict; a verified rational infeasibility certificate is a sound target. Successful optimization can supply a feasible distribution and matching dual bound. A witness alone proves attainability, not optimality.

### Equality and canonical form

Semantic equality is equality of the convex sets after frame alignment. A reference canonical description orders axes and outcomes structurally, reduces rationals to coprime numerator/positive denominator, removes duplicate and nonextreme generators, and orders the resulting vertices lexicographically. A singleton frame and a lower-dimensional polytope remain legitimate values.

Mathematical canonicality does not force every intermediate plan to enumerate vertices. An implementation can use constraints, vertices, or symbolic expressions with an exact equality procedure. A content fingerprint must derive from a canonical denotation or be accompanied by exact collision/equality checking; hashing the input constraint text is not model equality. Source history and calibration provenance stay in separate keyed relations.

Physical byte ordering is a deterministic index order, not the semantic refinement order. There is no promised single lexicographic range for polytope containment. A specialized index may reject candidates soundly; it may accept them only after an exact check or certificate.

### Representation alternatives to decide after the algebra

| Representation | Algebraic advantage | Obligation it cannot avoid |
| --- | --- | --- |
| Canonical extreme distributions | Unique finite normal form; map/mix/pool and payoff extrema are direct | Intersections and projections may cause large basis changes; equality requires reduction |
| Rational half-spaces and equations | Combination and admission constraints compose directly | Redundancy and lower-dimensional affine hulls complicate canonical bytes; images need elimination |
| Exact expression DAG plus geometric views | Retains useful factorization; can postpone expansion | Syntactic equality is insufficient; rewrites need the law registry; some queries still require expansion |
| Normalized relational description | Fits keys, law-bound carriers, and inspectable provenance | Model equality differs from equality of description rows; new aggregate numerical judgment is required |
| Flat structural model handle with owned payload | First-class value in rows; can intern canonical denotations | New descriptor, image/log ownership, lifetime, encoding, and equality contracts across the Rust/TS boundary |

The last two are exposure/storage decisions and can use any of the first three internal coordinates. The current [ValueType](/Users/bjorn/Documents/bumbledb/crates/bumbledb-theory/src/schema.rs:65) is flat and structural. A model-handle option must preserve that invariant rather than introducing a recursive `ValueType` unnoticed. We do not choose a 16-byte layout or treat a variable-sized polytope as an existing `WordPair`.

### Planner and machine contract

The reference denotation owns correctness. New pure-data IR operations carry frame bindings, exact constants, operation identity, and an explicit result/error type. Model derivation and bound computation are explicit stages; ordinary Free Join does not acquire a generic probability tag multiplication.

Useful exact rewrites include associativity/idempotence of combination, scoped elimination, map fusion, payoff pullback, and distribution of mixture over pooling. A binary segment or simplex face can have a specialized representation and kernel when a classifier proves the subcase. These are semantic subcases, not lossy replacements of arbitrary models.

The numerical kernel target is a deterministic answer with exact verification. A fast approximate solver may propose a basis or certificate, but an exact checker owns acceptance. Directed numerical export can enclose a rational result; internal stepwise rounding must not change algebraic equality. Even a fixed-point product with directed rounding can fail associativity (`laws.md`, X09).

Once an ABI and kernels exist, follow the Allen pattern: scalar specification, differential/naive oracle coverage, canonical-byte goldens, planner equivalence tests, then emitted-code gates suited to each specialization. No claim is made that arbitrary polytope operations have Allen's finite lookup-table cost or must use its branchlessness gate.

## 9. Closure boundaries that affect the choice of type

### One outcome model does not describe every sampling process

Copying an outcome uses `x↦(x,x)` and preserves perfect equality. Combining two renamed marginal models retains all couplings. Independently sampling from two precise distributions gives their product. These are different operations even for equal marginal values.

The convex strong product

```text
strongProduct(K,L) = conv{p⊗q : p∈K, q∈L}
```

is a finite polytope when its inputs are. It contains mixtures of independent distributions, whose members need not themselves factorize. It also loses an independence promise needed for some later refinements: the strong product of two vacuous binary models is the entire four-outcome simplex. Constraining both marginals to fair afterwards leaves all fair couplings; constraining both factors to fair before taking the product leaves only the independent point. For this reason, strong product is a documented optional extension with explicit convex semantics, outside the normative information core.

Repeated independent draws sharing one unknown bias are a stronger boundary. For `p∈[0,1]`, two such Bernoulli draws have probabilities

```text
(p², p(1−p), p(1−p), (1−p)²).
```

Writing `m=P(first succeeds)` and `s=P(both succeed)`, the convex hull is described by `m²≤s≤m`. Every point of the lower parabola is extreme. The result has infinitely many extreme points and is not a polytope. A finite basis cannot represent it exactly, irrespective of optimization.

If this operation is essential to the intended type, the finite-polytope choice must be reopened. Arbitrary compact convex models or a dependency-bearing process language are real alternatives. The local 2026 imprecise-programming paper motivates named choices, but its restriction on duplicate names along a path must not be cited as a solution to unrestricted repeated shared-bias sampling.

### Recursion

The proposed finite rational carrier has infinite chains and is not a complete lattice. For example, starting with `K₀=[0,1]`, iterating the affine image `p↦1/2+p/2` gives `Kₙ=[1−2^(−n),1]`. The limit is `{1}` and no finite iterate reaches it.

The choice/information laws do not imply the earlier semiring proposal's N-step convergence. Existing `rec` remains ordinary relational recursion with its current projection-only heads. A future model-valued recursive operator requires a specified order, fixed-point semantics, and either a termination theorem or exact limit/certificate mechanism. Finite unfolding is not silently exact infinite inference.

Likewise, ordinary query negation remains stratified absence of rows. Event negation inside a model is complement of an event. Missing a database fact is not a probability assignment to its negation.

## 10. The TypeSafe/jev boundary

The retained [documentation snapshots](/Users/bjorn/Documents/bumbledb/proposal/review-evidence/README.md) and [review](/Users/bjorn/Documents/bumbledb/proposal/review-astra.md) are the evidence for these rules:

- Noul's finite point probability maps to a singleton binary model, recording that this is the supplied forecast. Choice maps its full normalized vector to a singleton model over the law-bound roster. Score uses its distribution and an explicit level-rank payoff. The supplied mean and argmax can be retained as source outputs and checked against their stated conventions.
- Import must specify normalization. The exact constructor rejects a vector whose exact entries do not sum to one. An explicit upstream normalization transform may divide by its positive exact total and must record that transformation; it is not silently performed by admission.
- Source `confidence` remains a source-reported numerical fact. Its dispersion meaning does not supply a calibrated interval width, evidence count, or mixture weight. No exact confidence formula beyond the documentation is invented here.
- External calibration or an evidence model may supply a larger credal set. Its assumptions, dataset/version, and method belong to provenance. A conformal coverage guarantee does not automatically make `support(predictionSet)` a valid hard statement that the true outcome is inside that set for this case.
- “Questions evaluated independently” describes evaluation and does not establish statistical independence of the propositions. Related answers combine as constraints on explicitly aligned variables unless further semantics is supplied.

A precise forecast is precise about what the source reported, not certain knowledge of the world's actual probability. Combining two forecasts by intersection is therefore an application decision that both are adopted as simultaneous constraints. Source disagreement may instead warrant pooling or an explicitly weighted mixture. The algebra makes that choice visible.

## 11. Alternatives and the decision criterion

| Candidate | Its unifying algebraic idea | What it preserves or gives up |
| --- | --- | --- |
| Finite rational credal information + convex choice | Scoped information algebra, free rational convex semilattice, unique finite extreme basis, payoff duality | Exact finite linear information; excludes curved shared-parameter families |
| Arbitrary compact convex credal models | Same geometric information and linear observations with a broader closed-set universe | Includes the shared-bias convex hull; finite canonical basis disappears, and conditioning may require closure or leave compactness |
| Dependency-bearing process terms | Composition tracks named parameters, sampling, and reuse | Can distinguish repeated experiments; equality and inference need their own theory and may denote nonconvex or curved sets |
| Random-set/belief-function algebra | Subset-intersection convolution becomes multiplication under the commonality transform | Beautiful invertible coordinates for independent evidence; repeated evidence is not idempotent constraint accumulation |
| Interval/box and bilattice family | Endpoint orders and useful closed-form envelopes | Does not retain general coupled constraints; closure can require a declared abstraction |
| Evidence counts/opinions | Additive sufficient statistics and conjugate updating | An algebra for a stated sampling/prior model; not universal outcome-information combination |
| Boolean lineage with probability evaluation | Preserves repeated event identity; separates logic from measure | Essential for probabilistic tuple semantics, but still requires a probability/dependence model |
| Semiring annotations | Uniform rule evaluation and algebra-dependent fixed-point results | Choosing a semiring does not alone yield exact joint uncertainty or preserve arbitrary probability dependencies |

The strongest rival on the user's criterion is random-set algebra. If `m(B)` is mass on subsets, unnormalized conjunctive combination is

```text
(m₁⋆m₂)(A) = Σ_(B∩C=A) m₁(B)m₂(C)
Q(A) = Σ_(B⊇A) m(B)
Q_(m₁⋆m₂)(A) = Q₁(A)Q₂(A).
```

Möbius inversion recovers the masses. Mass on the empty set retains conflict; unit mass on the whole frame is the identity. This is a genuine coordinate-system insight. It describes intersection of independent random-set evidence, so reusing the same evidence as a fresh independent source changes its meaning. Normalized Dempster combination additionally divides away conflict and fails at total conflict. The question is which notion of assertion bumbledb should own, rather than which representation is cheaper.

The proposed preference is finite credal information because it unifies deterministic constraints, probabilistic constraints, forgetting, refinement, and robust decisions while making assertion repetition idempotent. The choice remains conditional on accepting the process boundary and the absence of a demonstrated finite Allen-like relation table.

## 12. What would make this ready to implement

The semantic candidate is now concrete enough to reject with counterexamples. It is not yet implementation-ready. The following decisions need a written resolution, in order:

1. Accept finite joint-outcome information as the target, or require shared-parameter process composition and revise the universe.
2. Accept convex information/choice as the sought Allen analogy, or require a finite qualitative relation algebra and continue that search.
3. Settle the law-bound high-level type and whether first-class exposure is a relational model view, an owned structural value, or both. Specify numerical law elaboration and exact rational representation without bypassing existing carrier rules.
4. Fix the conflict, observation, and empty-fold surface; the denotations in this draft are the proposed defaults.
5. Write the pure-data IR, canonical wire contract, and exact verification plan against that chosen surface.

The executable specification checks tiny exact models and explicit failed rewrites. They support the proposal and catch arithmetic/specification mistakes; they do not prove all dimensions, establish performance, or test an unimplemented engine. The law registry separates cited theorems, derived proofs, finite checks, and open obligations. That separation should survive every revision.
