# Canonical real-algebraic and parameter-set identity

Two checked root descriptions can name the same real value. An interval around
the positive root of `x²-2` and one around the same root of `(x²-2)(x²-3)` must
therefore export the same numeric identity. `AlgebraicRoot::to_bytes` now provides
that identity; `ParameterRegion::to_bytes` uses it for exact named sets.

This supplies standalone arithmetic transport. Parameterized Event source owners,
their guard/outcome coordinates and BEVT integration remain separate work. These
formats do not declare a source, choose a prior or turn parameter cells into
stochastic alternatives.

## Real numbers: minimal polynomial and root ordinal

`minimal_polynomial` computes the monic irreducible polynomial over the rationals
that vanishes at the captured value. It retains the certificate's formal variable
name. Numeric export instead uses one fixed all-zero `ParameterId` as a formal
indeterminate, then records the zero-based index in the polynomial's ascending
distinct real-root roster. Rational values use their monic linear polynomial.

```rust
use bumbledb::event::{
    AlgebraicLimits, AlgebraicRoot, ArithmeticLimits, ExactArithmetic,
    ExactPolynomial, ExactRational, ParameterId, PolynomialLimits, RootLimits,
};

let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
let name = ParameterId([9; 32]);
let x = ExactPolynomial::parameter(name);
let equation = x.pow(2, PolynomialLimits::default(), &mut work)?.sub(
    &ExactPolynomial::constant(ExactRational::from(2u64)),
    PolynomialLimits::default(), &mut work,
)?;
let root = AlgebraicRoot::from_interval(
    &equation, name, ExactRational::one(), ExactRational::from(2u64),
    RootLimits::default(), &mut work,
)?;
let identity = root.to_bytes(AlgebraicLimits::default(), &mut work)?;
let restored = AlgebraicRoot::from_bytes(
    &identity, AlgebraicLimits::default(), &mut work,
)?;
assert_eq!(root.compare(&restored, RootLimits::default(), &mut work)?, std::cmp::Ordering::Equal);
```

Authored factors, repeated factors, coefficient scaling, interval choices and
formal variable names do not affect the bytes. The two real roots of `x²-2`
have different ordinals and therefore different identities. Changing a root's
formal indeterminate does not identify two captured source parameters.

BEAR v1 is:

| Field | Encoding |
| --- | --- |
| Magic/version | `BEAR`, byte `1` |
| Polynomial length | little-endian `u64` |
| Minimal polynomial | canonical BEPL v1, monic, using the numerical indeterminate |
| Real-root ordinal | little-endian `u64`, zero-based |

Decoding validates the polynomial, reconstructs the real-root roster, selects the
claimed root and recomputes canonical identity. Reducible, repeated, nonmonic,
misnamed, nonreal, out-of-range and trailing-byte descriptions cannot bypass
admission. No claimed irreducibility or serialized Sturm chain is trusted.
The working root still has no derived `Eq`/`Hash`; fallible numeric comparison
and canonical export have explicit budgets.

## Bounded factor search

The first native implementation uses complete Kronecker search. It removes
repeated factors, clears coefficient denominators and looks for a proper factor
of each degree through half the current degree. A reducible polynomial has a
factor in that range. Gauss's lemma places a primitive representative in `Z[x]`.
At each of `k+1` distinct nonroot integer inputs, its nonzero value must divide
the integer polynomial's value. Enumerating the signed divisors and interpolating
therefore includes every possible degree-`k` integer factor. Fixing the first
value's sign removes only a global factor sign.

Each interpolated candidate is checked for integer coefficients, exact degree
and zero polynomial-division remainder. When a split succeeds, the existing
exact algebraic sign test selects the factor containing the captured root.
Degree strictly decreases. Only an exhausted search establishes irreducibility.

This is a bounded exact algorithm, not a performance qualification. Trial divisor
enumeration and candidate products can be expensive. `AlgebraicLimits` explicitly
bounds candidate count, search steps, root/polynomial operations and output bytes;
all arithmetic shares the caller's bit/work budget. Refusal publishes no identity
or partial irreducibility claim. A faster rational factorizer can replace this
implementation without changing BEAR's mathematical identity.

## Named regions: essential boundaries and membership

BEPR v1 preserves the actual captured `ParameterId`, sorted distinct canonical
BEAR boundaries, and one membership byte per point/open sector. A boundary is
retained exactly when its point and two adjoining sectors do not all agree.
This makes the representation unique for a named univariate semialgebraic set.
Internal complement polarity is eliminated when exporting membership bytes.

| Field | Encoding |
| --- | --- |
| Magic/version | `BEPR`, byte `1` |
| Captured parameter | 32 bytes, retained even for empty/full sets |
| Boundary count | little-endian `u64`, `n` |
| Each boundary | little-endian `u64` byte length followed by canonical BEAR v1 |
| Membership | `2n+1` bytes, each exactly `0` or `1`; open sector, point, open sector, … |

Decoding checks numerical strict order, distinctness, boundary necessity and every
extent before publication. `ParameterCodecLimits` bounds the region, each root
identity operation and total bytes. All roots share the supplied arithmetic
budget. `ParameterDomain::from_bytes` additionally requires nonemptiness; an
empty BEPR remains a valid region. Separate parameter names remain separate
identities even when they admit the same numerical set.

Equivalent polynomial sign descriptions export identically, including strict
versus closed boundaries, disconnected alternatives and irrational singleton
points. The canonical numeric variable inside a BEAR boundary does not replace
the source parameter named by its enclosing BEPR.

## Evidence and remaining proof boundaries

Seven native tests cover redundant/repeated/scaled/renamed presentations,
rational and higher-degree roots, numeric ordinals, all eight sign sets,
complement normalization, parameter scope, malformed inputs, canonical region
roundtrip, explicit exhaustion and cancellation. The independent exact
[SymPy generator](../scripts/event-algebraic-identity-oracle.py) supplies 143
minimal-polynomial/root-ordinal judgments from 104 authored polynomials. Runtime
and native tests need no Python or external solver dependency.

The [AlgebraicIdentity reference](../crates/bumbledb-event/semantics/AlgebraicIdentity.lean)
adds nine reports. These cover factor selection, bounded-degree coverage,
exhaustive interpolation-grid reasoning and canonical key/value equivalence.
Gauss's lemma, interpolation recovery, minimal-polynomial uniqueness and ordered
root-rank injectivity are explicit mathematical premises at the corresponding
boundaries. They are not new axioms asserted by the native decoder, nor a Lean
proof of its factor enumeration, byte parser or Rust implementation. The existing
Sturm proof limitations also remain. No general multivariate or source refinement
claim follows from these records.
