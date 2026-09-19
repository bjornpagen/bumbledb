# Event algebra: regions, observations, and transformations

**Revision 0.9; normative proposal with partial native host implementation.** This is the
operator contract for the [canonical Event representation](representation.md).
The [research adjudication](research/algebra-resolution.md) identifies primary
sources, derived laws, and limitations. The finite reference checker is
[algebra-checks.py](algebra-checks.py).

The [larger algebra](world-relations.md) treats events as predicates and as
relations on typed faces, with domain, modalities, residuals, and finite closure.
This document details its unary, observable, and information operations.
The region carrier remains one Boolean function on admissible worlds. An uncertain
judgment can be composed before an application commits to a decision. There is
no graded truth inside an individual world: the region selects where the
proposition is true. A normalized law measures that region afterward.

## 1. The type boundaries are part of the algebra

| Object | Meaning | Representation / authoring boundary |
| --- | --- | --- |
| `Event` | A region in one captured, nonempty world space | Two-word resident EventKey and owned canonical completed-function manager |
| A finite observable | One ordinary value in each world of a parent event | Ordinary `(group, value, when: event)` relations with pointwise functionality and coverage |
| An information partition | Worlds grouped by an explicitly supplied observation | Ordinary `(partition, case, when: event)` relations; same FD/IND proof as other partitions |
| A scope map | A checked deterministic map between world presentations | Owned source-layer descriptor, with its direction and support obligations |
| A conditional source | A normalized distribution over outcomes for each admitted input case | Named source/law descriptor; relational cases and source receipts |
| An exact observation | Probability, expectation, or a derived function of source parameters | Owned exact-function references, evidence domain, and provenance |
| An explanation | A witness or a separately checked sufficient condition | Owned certificate/view; never an unowned raw BDD path |

A structural Event need not have a designated probability law. A measured one
retains its law context; zero mass alone never means structural emptiness.
An observable does not introduce `Model<T>` or a new parameterized database
field. For example, `Balance(position, seat, coins, when)` already describes an
uncertain number using relations. A likelihood or payoff is likewise a function
over a case partition, not an extra probability weight on every Event row.

All operands are validated and aligned before algebraic simplification. Empty
and full remain scoped query values. Source changes produce new owned contexts
and explicit maps; they never mutate the meaning of an existing EventKey.

## 2. The complete Boolean core

| Operation | Definition | Proposed spelling / realization |
| --- | --- | --- |
| Empty, full | `∅`, `S` | `Empty(anchor)`, `Full(anchor)` |
| Complement | `S \ A` | `!a`; toggle region polarity |
| Intersection | `A ∩ B` | `a & b` |
| Union | `A ∪ B` | `a \| b` |
| Difference | `A ∩ !B` | `a & !b` |
| Symmetric difference | `(A \ B) ∪ (B \ A)` | `a ^ b` |
| Material implication | `!A ∪ B` | `!a \| b`; this produces an Event |
| Equivalence region | `!(A ^ B)` | `!(a ^ b)`; distinct from testing whole-event equality |
| If-then-else | `(G ∩ T) ∪ (!G ∩ F)` | `Ite(g, t, f)` inside `Event(...)` |
| Any fixed binary truth function | Its four Boolean truth bits | Internal `BoolOp4`; friendly syntax lowers to it |

`Ite` is deterministic branching on a retained event, not a random choice and
not permission to run side effects in both branches. It validates all three
inputs even if a constant gate makes one branch irrelevant. Useful equations:

```text
Ite(G,A,A)       = A
Ite(G,Full,Empty)= G
!Ite(G,A,B)      = Ite(G,!A,!B)
Ite(G,A,B) & C   = Ite(G,A&C,B&C)
```

Every finite Boolean expression is already expressible through this core.
Additional operations below expose useful structure, rather than inventing
alternative fuzzy meanings for AND and OR.

## 3. Structural questions return facts about events

The proposed `Test(...)` query head returns an ordinary `bool` after exact
event reasoning. It is not a probability threshold.

| Test | True precisely when |
| --- | --- |
| `IsEmpty(a)` | `A = ∅` |
| `IsFull(a)` | `A = S` |
| `Subset(a,b)` | `A & !B = ∅` |
| `Equal(a,b)` | `A = B` in the aligned space |
| `Disjoint(a,b)` | `A & B = ∅` |
| `Covers(a,b)` | `A \| B = S` |

Possibility under evidence is `!IsEmpty(E & G)`. A nonvacuous guarantee under
evidence requires both `!IsEmpty(G)` and `IsEmpty(G & !E)`. Empty evidence must
not be advertised as a successful guarantee. `Subset(Empty,E)` remains true
as a structural inclusion; its logical vacuity is explicit.

The four-bit Venn signature answers all pair tests and, more generally,
emptiness/fullness of any binary Boolean expression. A returned false inclusion
can carry a separately constructed witness from `A & !B`. The
[derived relationship calculus](experiments/event-repr-lab/SIGNATURE-CALCULUS.md)
supplies predicate masks, exact symmetries and a sound composition envelope.
It proves that the fifteen-class table cannot express exact composition for
finite scopes: a signature records which cells exist, but not whether they can
be split. These structural views neither determine independence nor replace
the richer constructive algebra below.

## 4. Cardinality and partitions

For a finite indexed roster `E1,...,En`, define `N(w)=Σi 1_Ei(w)`.

```text
AtLeast(k,E1,...,En) = {w : N(w) >= k}
AtMost (k,E1,...,En) = {w : N(w) <= k}
Exactly(k,E1,...,En) = {w : N(w)  = k}
```

These are derived Boolean operators and require no independence. The proposed
expression syntax accepts a literal nonnegative `k` and at least one event
operand. An empty dynamic roster needs an explicit space anchor at the library
boundary: exactly zero is full; every positive count is empty.

Positions in this roster identify the things being counted. Two different
physical cards can have identical holding events and must still count twice.
Repeated derivations of the same card ID count once. Do not deduplicate roster
members by EventKey. Three-way XOR is odd parity, not `Exactly(1,a,b,c)`.

One construction yields the entire count distribution as a relation. Start
with `C0=S` and all other buckets empty. For each roster event E, update
simultaneously:

```text
C'_k = (C_k & !E) | (C_(k-1) & E)
```

The final `(count, C_count)` rows form a disjoint full partition. Empty buckets
remain valid query and stored values. Explicitly omitting them creates a sparse
view and changes row existence. A library partition iterator materializes these rows; it does not require
a new uncertain-integer field or an unspecified multi-column macro binder.

The [native indexed partition API](../docs/event-partitions.md) now implements
this count roster and evidence-relative partition admission. It also supplies
ordinary value grouping, same-world refinement, pullback and checked readout
conversion. All declared output cells remain present, including empty buckets.
The helper stores Events only; scalar labels remain ordinary relation columns.
Query-level Pack now has a native grouped implementation. Host exact finite
expectation is available; aggregate expectation and its query result slots keep
their own implementation gates.

More generally, map a finite observable through ordinary value relations, then
`Pack` equal output values. If a Choice is split between two forced-Coup targets,
both map to seven coins spent. Packing the seven-coin branches gives full:
uncertain input, certain derived output, proved by coverage.

For a dynamic group, `Pack(E)` remains union. Intersection is a derived staged
operation `!Pack(!E)` with an explicit full seed when empty groups are desired.
It is not `Pack(E)` with a different interpretation. Group absence stays distinct
from empty/full event values.

## 5. Hiding information: may, must, and unresolved regions

Let `O={C1,...,Cm}` be an admitted partition of S. A cell contains worlds that
this specified observation cannot distinguish. Define same-space operations:

```text
May_O(E)  = union {C in O : C & E != empty}
Must_O(E) = union {C in O : C & !E == empty}
```

May says which observation cases permit E. Must says which cases guarantee E.
Both return Events on the original S, so their results remain composable.

```text
Must_O(E) <= E <= May_O(E)
Must_O(E) = !May_O(!E)
May_O(May_O(E))   = May_O(E)
Must_O(Must_O(E)) = Must_O(E)
May_O(E | F)     = May_O(E) | May_O(F)
Must_O(E & F)    = Must_O(E) & Must_O(F)
Unresolved_O(E)  = May_O(E) & !Must_O(E)
```

For any union H of whole O-cells:

```text
May_O(E) <= H  iff E <= H
H <= Must_O(E) iff H <= E
```

The [native finite readout API](../docs/event-information.md) now implements
these operators and evidence cases through the existing map/relation layer.

These give the least observable overapproximation and greatest observable
underapproximation. This is the set-algebra saturation/cylindrification of
Kohlas–Casanova–Zaffalon §6, with its Boolean dual derived here.

Evidence requires an explicit reachable parent:

```text
Reach_O(G)     = union {C : C & G != empty}
May_O(E;G)    = union {C : C & G & E != empty}
Must_O(E;G)   = Reach_O(G) & !May_O(!E;G)
```

Thus impossible cases do not yield vacuous certainty. The results select whole
original cells; intersect with G separately when the desired output is actual
remaining worlds. Full-space formulas are the special case `G=S`.

These operations do not invent a prior on unknown parameters, condition the
source, or expose hidden information. In Coup they describe what an observation
can distinguish **relative to the supplied support**. Calling the result “what
Bob knows” additionally requires a space containing all alternatives Bob admits
and an observation partition reflecting his information and memory. Alice's
privately restricted space is not automatically Bob's epistemic universe.

On constrained support, saturations for different partitions need not commute.
For S=`{00,01,11}`, start at `{00}`. Saturating first by equal first coordinate
and then by equal second coordinate reaches S; the reverse order reaches only
`{00,01}`. Do not transfer unrestricted cylindric-algebra commutation laws to
arbitrary legal-world partitions. A refinement of observations is different:
finer cells reduce May and enlarge Must.

The [information-readout Lean proofs](experiments/event-repr-lab/INFORMATION-ALGEBRA.md)
now establish these bounds and refinement laws over arbitrary observation
functions. They also give an exact optimizer criterion: for every E,
`May_O(E & H) = May_O(E) & H` iff the observation determines H's membership.
The dual uniform Must/union rule has the same FD condition. For two observation
functions, the uniform containment order on May is exactly the FD from the
finer observation to the coarser one. The laboratory's names Possible,
Guaranteed and Ambiguous refer to these same operators.

Query realization: join cells with the event, compute `Test(IsEmpty(...))`,
filter that boolean in a later interior, and `Pack` the qualifying **whole
cells**, with explicit group seeds. The [advanced Coup queries](coup/algebra-queries.rs)
show this using existing `PolicyCase` rows. No opaque partition-valued database
column or special knowledge-graph subsystem is needed.

## 6. Change of coordinates and finite transition reasoning

For a checked total map `f:S -> T` of finite guarded presentations:

```text
pull_f(B) = {w in S : f(w) in B}
some_f(A) = {v in T : some w in A has f(w)=v}
every_f(A)= {v in T : every w in S with f(w)=v belongs to A}
```

Pullback preserves the entire Boolean algebra, including full and complement.
Direct image preserves unions. Universal image preserves intersections and is
dual to direct image relative to T. The adjunctions are:

```text
some_f(A) <= B  iff A <= pull_f(B)
pull_f(B) <= A  iff B <= every_f(A)
```

Universal image includes target points with no preimage. For a nonvacuous
guarantee use `some_f(S) & every_f(A)`, or use the reachable target as T. A
surjective map makes pullback injective. A support restriction generally does
not: previously distinct events can become equal in the new context.

**Choose separate APIs for separate results.** `cofactor` substitutes coordinate
values and returns a support-bearing predicate view. `pull`, `some`, and `every`
return Events through checked source-layer maps. `marginalize` constructs the
target law by summing outcome masses. For a parameter-preserving outcome map:

```text
nu_theta(v) = sum_(w:f(w)=v) mu_theta(w)
P_nu(B) = P_mu(pull_f(B))
```

Image is not marginal probability. Forgetting one fair bit makes “heads was
possible” full in the remaining space; the original heads mass is still 1/2.
Default law maps retain source parameters; eliminating their admissible values
logically does not supply an integration measure over them.

A supplied finite transition relation `R ⊆ S×T` similarly supports:

```text
PreMay_R(B) = {s : some t has R(s,t) and B(t)}
Enabled_R  = PreMay_R(T)
PreMust_R(B)= Enabled_R & !PreMay_R(!B)
Post_R(A)  = {t : some s has A(s) and R(s,t)}
```

PreMust means every allowed next state meets B and at least one next state
exists. Relational composition uses conjunction plus existential elimination;
backward reasoning composes accordingly. For partial transitions,
`PreMust_(R;Q)(B)` need not equal `PreMust_R(PreMust_Q(B))`: a dead intermediate
branch is ignored by relational composition but refutes the nested guarantee.
The exact replacement is
`PreMust_R(PreMust_Q(B)) = PreMust_(R;Q)(B) & All(R,Domain(Q))`,
proved in [ModalContract.lean](semantics/ModalContract.lean). Q being enabled on
every R-successor makes the extra factor redundant. The algebra also includes
exact Star and monotone fixed points on a sealed finite stable state presentation, as specified in
[world-relations.md](world-relations.md). These are dedicated algebra stages,
not unrestricted event-valued recursive Rust macro rules.

Probability requires the actual transition kernel, beyond R's support.
Backward propagation through a stochastic kernel K yields
`K*1_B(s)=Σt K(s,t)1_B(t)`, usually a numeric function, not an Event. To retain
an Event for the actual future outcome, keep the named outcome coordinate in
the joint trajectory. This is a substantive reason to preserve trajectories.

## 7. Observation, expectation, and decisions

`Probability(E,G)` keeps the exact numerator `mass(E&G)`, evidence `mass(G)`,
and positive-evidence parameter domain. Its source family remains available;
a range is a report. Boolean guarantees do not require first computing it.

For a finite observable V represented by disjoint value events `E_v`, define:

```text
Expectation(V,G) = sum_v v * mass(E_v & G) / mass(G)
```

The proposed aggregate head is `Expectation(value, when, given)`. It first
unions equal-value regions, requires `given` to be the same event per group,
and checks distinct values are disjoint on given and collectively cover it.
Missing values are a coverage error, not implicit zero. Values are initially
exact signed integers/rationals supplied through supported numeric columns or
the exact-function library; arbitrary floating score arithmetic is not an exact
kernel. The owned result supports signed values and cannot be a probability
field or an existing half-open interval. Zero-mass evidence yields Impossible,
even when structurally nonempty. The [native host observation](../docs/event-revisions.md)
now implements signed finite-function contraction; the aggregate retains its
separate grouping/coverage and query result obligations.

Arithmetic on finite observables is relational: join their value branches,
intersect the events, compute the ordinary output value, and pack equal values.
For robust preference, compare the expected **difference** under the same source
assignment. Separate expectation bounds can hide a strict dominance relation.
No min/max over source assignments is pushed inside a sum without proof.

Given an explicit action-payoff table and observation partition, the fixed-law
value of seeing the observation before choosing is:

```text
sum_C max_a sum_(w in C&G) mu(w) U_a(w) / mass(G)
    - max_a sum_(w in G) mu(w) U_a(w) / mass(G)
```

This is nonnegative when the same finite actions are available in each cell;
one can always ignore the observation. Observation costs are subtracted
explicitly. The maximization chooses one action per visible cell, not one per
hidden world. A law family yields a function of its **shared** parameters;
robust policy choice needs an explicitly stated minimax/dominance rule. The
maximin of a fixed policy differs from knowing the true parameter first.

Action payoffs must come from supplied action semantics. Conditioning on the
action an opponent happened to select does not construct the payoff from
forcing that action. Causal intervention and cross-action counterfactuals need
additional source structure; Event alone does not provide it.

## 8. Explanation and sensitivity without inventing certainty

- `witness(E)` gives one legal satisfying world/cell, or Empty. For parameter
  guards, recover an exact feasible parameter witness/certificate; a raw bit
  assignment is not enough. A witness is not a sample or the most probable world.
- A sufficient explanation is a literal cube C with nonempty `S&C` and
  `S&C <= E`. A deletion-minimal cube is irredundant, not necessarily shortest.
  Minimal conflicting fact sets and schema provenance are separate certificates.
- A source-parameter derivative acts on the retained mass/expectation function.
  On an open guard cell, polynomial and rational DAGs support exact automatic
  differentiation. For `p(1-p)`, this gives `1-2p`; for a conditional numerator
  n and evidence z it gives `(n' z - n z') / z²` where z>0.
- Piecewise boundaries may be nondifferentiable. Derivative output must retain
  its domain; general semialgebraic functions have no blanket derivative API.
  This differentiates the adopted numerical source model, not Jev or its prompt.
- Sensitivity can identify which supplied forecast rate affects an answer.
  It is distinct from information value, which also needs observation and
  action-payoff semantics.

## 9. What lowers to the existing representation

| Family | Shared work |
| --- | --- |
| Boolean expressions / ITE | Memoized Apply/ITE and canonical pair interning; dense views use bitwise/select kernels |
| Structural tests / witnesses | Root checks, Venn occupancy, satisfying-path extraction, optional exact guard solver |
| Cardinality | BDD bucket dynamic program or compiled bit-sliced counter; identities remain roster-sensitive |
| May / Must / finite preimages | Cell union or fused conjunction-and-abstraction; legal support masks retained |
| Deterministic maps | Checked substitution/renaming, Apply, abstraction, and target canonicalization |
| Composition / residuals | Typed relational product; residuals use universal abstraction and complement |
| Finite closure / predicate fixed points | Monotone iterations over a sealed finite quotient, with canonical equality for stabilization |
| Probability / expectation | Shared law contraction and exact-function DAGs; outcome sum versus guard selection |
| Sensitivity | Dual-number/reverse derivative programs over the exact contraction circuit on a valid domain |

Darwiche–Marquis distinguishes bounded binary Apply from many-variable
forgetting: the latter can grow exponentially even for OBDDs. These operators
are exact and finitely specified, not promises of uniformly small diagrams.
AMC supplies conditions for circuit evaluation, not a license to multiply
arbitrary marginal Noul probabilities. Free Join manages relational witnesses;
law contraction manages probability; the planner can connect them through
proved algebraic rewrites.
