# Exact parameter regions and guarded rational functions

`ParameterRegion` represents a semialgebraic set on one named real parameter.
Polynomial sign predicates construct regions; all sixteen `BoolOp4` operations
combine them. Complement, inclusion, equality, exact membership and witnesses
preserve open boundaries, disconnected alternatives and irrational singleton
points. A different parameter name requires explicit alignment; even empty/full
regions cannot silently combine two source coordinates.

`ParameterDomain::new` admits a nonempty region as a source-domain building
block. Empty regions are valid results, but an empty source domain is a distinct
error. Neither type creates stochastic outcomes, supplies an implicit prior,
asserts independent source parameters or designates a normalized probability law.
The current solver capability is **univariate**. Joint constraints on multiple
parameters remain M6 work. [BEPR transport](event-algebraic-identity.md) supplies
persistent region identity, and [parameterized sources](event-parameter-sources.md)
now integrate these domains and functions into owned Events and BEVT v3.

```rust
use bumbledb::event::{
    ArithmeticLimits, BoolOp4, ExactArithmetic, ExactPolynomial, ExactRational,
    GuardedRationalFunction, ParameterDomain, ParameterId, ParameterLimits,
    ParameterRegion, PolynomialSigns,
};

let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
let limits = ParameterLimits::default();
let name = ParameterId([7; 32]);
let p = ExactPolynomial::parameter(name);
let one_minus_p = ExactPolynomial::one().sub(&p, limits.polynomial, &mut work)?;
let lower = ParameterRegion::from_polynomial(
    name, &p, PolynomialSigns::NON_NEGATIVE, limits, &mut work,
)?;
let upper = ParameterRegion::from_polynomial(
    name, &one_minus_p, PolynomialSigns::NON_NEGATIVE, limits, &mut work,
)?;
let domain = ParameterDomain::new(lower.apply(BoolOp4::AND, &upper, limits, &mut work)?)?;

// Two conditionally independent draws sharing the same unknown bias:
// numerator p(1-p), evidence 2p(1-p).
let numerator = p.mul(&one_minus_p, limits.polynomial, &mut work)?;
let evidence = numerator.add(&numerator, limits.polynomial, &mut work)?;
let conditional = GuardedRationalFunction::new(
    domain, numerator, evidence, limits, &mut work,
)?;
assert_eq!(conditional.value_at(&ExactRational::zero(), limits, &mut work)?, None);
let half = ExactRational::fraction("1", "2", &mut work)?;
assert_eq!(conditional.value_at(&half, limits, &mut work)?, Some(half));
```

This example constructs standalone arithmetic under the stated experiment
assumption. The [source API](event-parameter-sources.md) additionally constructs
an owned Event observation from a checked normalized family. The original
numerator and evidence polynomial remain available; their conditional ratio's
constant value cannot erase the endpoints where conditioning is undefined.

## A finite partition of an infinite real line

An immutable shared partition owns sorted distinct algebraic boundaries and one
membership bit per point/open sector: `2n+1` bits for `n` boundaries. A region adds
its captured parameter name and a complement-polarity flag. Complement shares
the partition. Points retain independent membership from their adjoining open
sectors; an included interval does not automatically include either endpoint.

The polynomial sign constructor isolates every distinct real root. It then
evaluates one exact rational witness in every open sector, where the polynomial
has no root and hence constant sign. Root points have zero sign. An identically
zero polynomial has constant zero sign everywhere; it does not request a finite
root roster. `AlgebraicRoot::rational_between` proves the order and returns a
strict interior rational witness by exact refinement. Irrational singleton sets
instead return `RealWitness::Algebraic`.

Boolean operations merge the sorted boundary rosters using exact algebraic
comparison. They align both memberships on the **same** parameter value. A
boundary is removed only when its point and both adjoining sectors all have the
same membership. This reduces empty/full results and preserves holes, isolated
points and strict boundaries. `cells()` exposes the entire resulting partition,
including excluded cells; this is a logical decomposition, not a probability
distribution or an enumeration of real parameter values.

`equivalent` and `included` are exact scoped judgments. Working root presentations
are not canonical numeric identity, so regions have no derived `Eq` or `Hash`.
[BEPR v1 export](event-algebraic-identity.md) now provides canonical named-set
bytes by minimizing every retained algebraic boundary. A witness's
defining-polynomial indeterminate is a numerical
description; membership may use the same real number described with another
formal indeterminate without renaming the region's captured source parameter.

## Partial rational functions retain their domains

`GuardedRationalFunction` owns an inhabited ambient `ParameterDomain`, its
defined region, and exact numerator/denominator polynomials. Construction
defines the function precisely where the denominator is nonzero inside that
ambient domain. Further `restrict` operations can add holes without changing
the ambient domain or arithmetic presentation.

`on_domain` explicitly captures a smaller inhabited ambient domain. It refuses
extension and preserves inherited holes and arithmetic definitions. Both the
single-piece and piecewise `ParameterFunction` APIs support it. It underlies
[source-domain restriction and family conditioning](event-parameter-conditioning.md).

Addition, subtraction and multiplication operate on the intersection of their
input defined regions. Division additionally excludes zeros of the divisor;
reciprocal does the same for its input. Every inherited hole remains, including
when a numerator reduces to zero. A nowhere-defined function is valid over an
inhabited ambient domain; it is not a source conflict or a fabricated zero.
Binary arithmetic requires equivalent ambient domains with the same captured
parameter name. There is no inferred source-domain extension.

`value_at` returns an exact rational or `None` for undefined/outside points.
`where_sign` returns the exact region where the defined function has any requested
subset of negative/zero/positive signs. It accounts for negative denominators
and excludes denominator zeros even for `PolynomialSigns::ANY`. Subtracting two
functions and testing its sign therefore retains the precise parameter set
where one exceeds the other, rather than reducing it to a numeric interval hull.

Function `equivalent` compares ambient domains, defined regions and values
everywhere on that region. This is numerical partial-function equality. It does
not equate original evidence masses, observation identities, source laws or
provenance. The shared-bias ratio above is equivalent to `1/2` restricted to
`0<p<1`, but not to an everywhere-defined `1/2` on `[0,1]`.

A general rational denominator may be negative. The parameter-source law and
observation constructors separately prove nonnegative mass, normalization,
source support and the positive-evidence domain. This host arithmetic type does
not discharge those admission obligations and occupies no EventKey/query slot.

## Resource and verification boundaries

`ParameterLimits` bounds cell rosters and visited cell/solver operations. Its
nested polynomial/root limits apply to their respective construction operations;
composite rational operations invoke a bounded number of those operations.
One caller-owned `ExactArithmetic` budget spans all nested arithmetic. These are
explicit per-operation bounds, not an aggregate retained-memory guarantee.

Written inputs participate before constant-result shortcuts: empty/full region
operations check parameter scope and every retained root; functions check both
domains and both polynomial inputs even for nowhere-defined results. Rational
arguments are checked before copying. Cancellation and resource refusal never
produce a successful empty region, witness absence, false comparison or undefined
function value.

Eight native tests cover every Boolean operator at rational and algebraic
boundaries, exact empty/full reduction, strict/non-strict endpoints, irrational
singleton witnesses, alternate root descriptions, shared-bias denominator
retention, all eight sign predicates for signed rational arithmetic, zero/undefined
distinctions, source scopes and budget/cancellation refusal. Existing root tests
retain the independent exact SymPy fixture.

The [ParameterDomains reference](../crates/bumbledb-event/semantics/ParameterDomains.lean)
adds sixteen reports. It proves cell-denotation laws given a complete inhabited
cell presentation, common refinement preserving one parameter assignment, and
domain-retaining arithmetic over a generic field. It does not formalize the
Sturm solver, native boundary merge/normalization, source/BEVT correspondence or
general multivariate semialgebraic solving.
