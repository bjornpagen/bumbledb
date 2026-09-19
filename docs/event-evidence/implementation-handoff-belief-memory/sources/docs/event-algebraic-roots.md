# Exact real roots for source-domain reasoning

`AlgebraicRoot` is an owned real-algebraic witness. It supplies exact roots and
polynomial signs needed by parameter guards, including domains with no rational
point. `p² = 1/2` and `0 < p < 1` has such a witness; a rational sampling grid
cannot establish its nonemptiness.

```rust
use bumbledb::event::{
    AlgebraicRoot, ArithmeticLimits, ExactArithmetic, ExactPolynomial,
    ExactRational, ParameterId, PolynomialLimits, RootLimits,
};
use std::cmp::Ordering;

let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
let name = ParameterId([8; 32]);
let p = ExactPolynomial::parameter(name);
let half = ExactRational::fraction("1", "2", &mut work)?;
let equation = p.pow(2, PolynomialLimits::default(), &mut work)?.sub(
    &ExactPolynomial::constant(half), PolynomialLimits::default(), &mut work,
)?;
let witness = AlgebraicRoot::from_interval(
    &equation, name, ExactRational::zero(), ExactRational::one(),
    RootLimits::default(), &mut work,
)?;
assert_eq!(witness.sign(&equation, RootLimits::default(), &mut work)?, Ordering::Equal);
assert_eq!(witness.sign(&p, RootLimits::default(), &mut work)?, Ordering::Greater);
```

These are algebraic checks at a real value, not a distribution over parameters.
Root isolation allocates no outcome coordinates, assigns no probability mass,
and does not silently restrict structural Event support.

## Checked representation and operations

A root retains an `Arc` containing its authored canonical polynomial, formal
parameter name, monic square-free polynomial and exact rational Sturm chain.
Each root adds two owned rational endpoints. An open interval must have nonroot
endpoints and contain exactly one distinct real root. Equal endpoints instead
claim an exact rational root. `from_interval` recomputes all these obligations;
it accepts no asserted root count or trusted serialized Sturm chain.

`ExactPolynomial::isolate_roots` returns every distinct real root in ascending
order. Multiplicity does not duplicate a real value. A constant nonzero
polynomial has an empty roster; the identically zero polynomial returns
`IndeterminateRoots`, because its root set is infinite. This refusal is not a
claim that a parameter domain is empty. An input containing another named
parameter returns `NotUnivariate`, not an infeasibility judgment.

`sign` checks a polynomial at the captured root. `compare` compares exact real
values, including roots of different defining polynomials. A reducible polynomial
and a minimal polynomial may name the same value. Overlapping intervals alone
never establish equality. Different parameter names can be formal indeterminates
in two numerical descriptions of the same real number; comparing their values
does not identify the corresponding captured source parameters.

The working type has no derived `Eq` or `Hash`. A polynomial plus an isolating
interval is a checked description, not numeric identity. The separate
[BEAR v1 export](event-algebraic-identity.md) now computes canonical minimal-
polynomial/root-index identity with an explicit bounded factor search. Stable
source descriptor integration remains separate work. Reconstructing a root
through its exposed polynomial and interval rechecks the claim; retained roots
survive release of the original polynomial and other root handles.

## Native algorithm

Sparse named polynomials are admitted into bounded dense univariate coefficient
vectors. Exact Euclidean division and GCD remove repeated factors. Cauchy's bound
`1 + max |a_i/a_n|` supplies strict rational bounds on all roots. A Sturm chain
starts with the square-free polynomial and its derivative, then uses negative
remainders. Chain elements may only be rescaled by positive factors: making a
negative-leading remainder monic would reverse a sign and corrupt root counts.

Ignoring zero entries, the variation difference `V(lower)-V(upper)` counts
distinct roots in `(lower, upper]`. Published non-singleton root intervals have
nonroot endpoints, so this is their open-interval count too. Isolation bisects
with exact rationals and chooses a nonroot split when the midpoint is a root.
Later refinement can capture an exact rational midpoint as a singleton.

For sign evaluation, GCD first detects whether the test polynomial vanishes at
the captured root. Otherwise refinement separates that root from every root of
the test polynomial, and a rational interior sample determines its constant
nonzero sign. Equal endpoint signs are insufficient: two test-polynomial roots
can lie between them. Comparing two algebraic values similarly combines GCD,
intersection root counts and exact separation. No tolerance or floating-point
approximation participates in an accepted judgment.

This is a native univariate root/sign capability. It is groundwork for real
parameter-domain construction and multivariate elimination, not a replacement
for the proposal's full semialgebraic source language. Multivariate feasibility,
parameterized Space laws,
guard/outcome contraction and canonical source transport remain open M6 gates.

## Budgets and verification

`RootLimits` bounds authored/intermediate degree, the number of retained roots,
and solver steps. Dense zero initialization is charged before allocation, so a
sparse term with an enormous exponent cannot bypass the work bound by requesting
an enormous dense vector. Exact coefficient arithmetic shares the caller's
`ExactArithmetic` bit/work budget. Existing roots and every retained chain
coefficient are rechecked against the current operation's limits, even for a
zero test polynomial or comparison with the same root.

Cancellation and exhausted budgets publish no partial root roster, no guessed
sign and no false equality. Per-operation bounds do not close the separate
aggregate retained-memory policy. No process-wide memory or speed claim follows.

Eight native tests include 245 polynomials with known rational roots, repeated
factors, irrational witnesses, invalid isolation claims, sign changes hidden
between equal endpoint signs, distinct roots separated by `10^-30`, and bounded
failure. A committed independent fixture covers 105 dense polynomials through
degree seven, every distinct real root, and four polynomial signs at each root.
[The generator](../scripts/event-root-oracle.py) pins SymPy 1.14.0 and uses exact
rational isolation/GCD/sign judgments only. Native tests need neither Python nor
SymPy; no production dependency is added.

The [AlgebraicRoots reference](../crates/bumbledb-event/semantics/AlgebraicRoots.lean)
adds fifteen reports. Generic field lemmas justify remainder and GCD zero
identities; generic linear-order lemmas describe isolation, comparison and
refinement. A finite sign lemma covers the middle-zero Sturm case. These do not
formalize Sturm's theorem, Cauchy's bound, constant-sign continuity, the Euclidean
implementation or Rust refinement. Independent oracle coverage is evidence for
the native implementation; it does not remove those formal proof boundaries.

[Parameter regions](event-parameter-domains.md) now use these roots to solve
univariate polynomial sign sets, merge exact boundaries, and retain guarded
rational-function domains. Root/rational comparison and strict rational
separators supply the open-cell witnesses. [Canonical numeric and region bytes](event-algebraic-identity.md)
are now available separately; multivariate elimination and parameterized Event
sources remain open.
