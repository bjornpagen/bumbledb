# Exact arithmetic over observations

The native host layer now computes exact partial numbers from completed
probability and expectation answers. It retains their original observations in
an immutable derivation. This is the arithmetic foundation for query numerical
operators; numeric query heads, result transport and the typed SDK are still
integration work.

```rust
let p = ObservationNumber::probability(
    probability.clone(), ObservationComponent::Value, limits, &mut arithmetic,
)?;
let utility = ObservationNumber::expectation(
    expectation.clone(), ObservationComponent::Value, limits, &mut arithmetic,
)?;
let score = p.apply(NumberOp::Multiply, &utility, limits, &mut arithmetic)?;
let positive = score.where_sign(PolynomialSigns::POSITIVE, limits, &mut arithmetic)?;
```

`score.expression()` retains the written operands, including the original Events,
evidence, laws and payoff functions. `positive.predicate().view()` exposes the
exact true, false and undefined cases. `possibly()` asks for a true assignment;
`always()` requires truth on the entire ambient domain, including definedness.
These are explicit numerical judgments. They do not turn a partial answer into
a Boolean by accident.

## Number domain and operators

`PartialNumber` is a fixed optional rational or an owned `ParameterFunction`.
Fixed values have one assignment; `None` means undefined. Parameter functions
retain their inhabited ambient domain and exact defined region. The native
solver remains univariate. This does not restrict the general reference
semantics or close the multivariate/general semialgebraic gate.

All binary operations first validate both complete numerical operands under the
caller's shared exact-arithmetic budget. A fixed number lifts to a constant on
the other operand's parameter domain. A fixed undefined value lifts to a
nowhere-defined function. Two parameter operands require the same named ambient
domain; numerical-domain restriction is explicit through `on_domain`. Distinct
parameter names are never silently equated or assigned a joint prior.

| Operation | Value and defined domain |
| --- | --- |
| Add, subtract, multiply | Exact pointwise result on the intersection of operand defined domains |
| Divide | That intersection with every zero-divisor assignment removed |
| Negate, absolute value | Exact result on the original defined domain |
| Natural power | Original defined domain, including exponent zero |
| Min, max | Pointwise branch selection on the common defined domain, with exact boundary points |
| Numerical equivalence | Equality of partial functions, including their defined domains |
| Sign mask / comparison | Exact true, false and undefined partition |

`p / p` equals one only where p is defined and nonzero. `p - p`, `0 * p`, and
`p ^ 0` keep p's original holes. Arithmetic bit/work limits, capacity refusal and
cancellation remain errors; they never become undefined numerical answers.

Multiplying two measured probabilities computes their numerical product. It
does not assert that this product measures the conjunction. A conjunction must
still be formed as an Event and contracted under the captured joint law.
Likewise, comparing observations from different fixed sources compares their
numbers without merging those sources. The derivation retains both.

## Truth partitions

`NumberPredicate` is immutable and constructed only from checked operations.
Its parameter view partitions the entire ambient domain into three disjoint
regions: holds, fails, undefined. Fixed predicates have the corresponding
`Some(true)`, `Some(false)`, or `None` value.

Negation exchanges holds and fails, retaining undefined. All sixteen `BoolOp4`
operations have a strict partial lift: both predicates must be defined. Even
a constant truth table retains its input holes. This is the ordinary algebra of
partial functions. No short-circuit three-valued policy is inferred. `always`
requires an empty false region and an empty undefined region; nowhere-defined
evidence therefore never makes a universal claim true by vacuity.

Numerical equivalence and pointwise equality are distinct. An undefined partial
value can be numerically equivalent to itself while its equality predicate is
undefined. Neither judgment equates its original Event/provenance with another
observation that happens to have the same values.

## Ownership and bounds

`ObservationNumber` owns an `Arc` derivation whose leaves are exact literals or
completed probability/expectation answers. Selecting Value, Numerator or
EvidenceMass retains the entire leaf observation. Binary, unary, power and
explicit domain-restriction nodes retain their input derivations. A restriction
changes the numerical ambient domain; it does not condition or replace the
original source law. `ObservationPredicate` owns its numerical derivation and
sign mask alongside the checked partition.

Expanded expression size and depth are checked before publishing a node. The
default node limit is 65,536; the depth ceiling is 256 even if a caller requests
more, bounding recursive inspection and destruction. Arithmetic/solver limits
validate operands anew, share the supplied work counter and preserve cancellation.
These limits do not constitute the complete retained-byte policy required by
the proposal. No persistence identity or numerical-expression codec is claimed.

## Evidence and remaining integration

Core tests compare fixed and parameter arithmetic with a rational oracle,
including independent poles, division zeros, exact min/max boundaries, powers,
all eight sign masks, all sixteen strict Boolean lifts, irrational roots,
domain/scope refusals, hidden oversized operands, shared arithmetic exhaustion,
function cell limits and cancellation.

Database consumers extract completed staged probability and expectation answers
on resident/cursor paths and retain derivations after owner closure. They check
distinct equal-valued observations, mixed signed arithmetic, zero-mass evidence,
shared-parameter holes, explicit restriction and expression bounds.

`PartialNumbers.lean` gives 26 reference reports for defined-domain
intersection, division exclusions, powers, truth partitions, strict Boolean
lifting, explicit quantification, restriction and provenance retention. It
assumes a mathematical assignment domain; it does not prove the native solver,
Rust arithmetic, concrete derivation representation or query lowering.

Next integration must add pure-data numerical query instructions, typed query
values, owned staging/identity, producer admission and cross-stage budgets,
macro and Node/SDK authoring, transport and result decoding. Numerical predicates
must retain their exact domains; turning a parameter region into an Event needs
explicit guard/refinement construction. These are not implemented by this host
checkpoint. All other M0–M8 gates remain in force; no release or version bump.
