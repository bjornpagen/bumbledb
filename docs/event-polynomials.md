# Exact named-parameter polynomials

`ExactPolynomial` supplies the arithmetic normal form needed by shared source
parameters. Coefficients are exact rationals; `ParameterId` identifies a captured
real value. Reusing a name reuses that value. A different name does not establish
independence, and an unknown value does not acquire an implicit probability law.

```rust
use bumbledb::event::{
    ArithmeticLimits, ExactArithmetic, ExactPolynomial, ExactRational,
    ParameterId, PolynomialLimits,
};

let limits = PolynomialLimits::default();
let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
let name = ParameterId([7; 32]); // The application supplies its stable identity.
let p = ExactPolynomial::parameter(name);
let tail = ExactPolynomial::one().sub(&p, limits, &mut work)?;
let ht = p.mul(&tail, limits, &mut work)?;
let evidence = ht.add(&ht, limits, &mut work)?; // 2p(1-p), not a constant.
let hh = p.pow(2, limits, &mut work)?;

// Explicitly integrate this one shared parameter under a uniform Beta prior.
let one = ExactRational::one();
let integrated = hh.integrate_beta(name, &one, &one, limits, &mut work)?;
let third = ExactRational::fraction("1", "3", &mut work)?;
assert_eq!(integrated, ExactPolynomial::constant(third));
```

The multiplication above is arithmetic. A future source constructor must supply
the justification for conditionally independent fresh outcomes before using it
as their joint law. Copying an existing Event remains idempotent. Its probability
does not get squared by referring to it twice.

## Normal form and scope

The owned representation is a sparse ordered table of terms. Each term contains
one reduced rational coefficient and an ordered list of `(ParameterId, u32)`
positive powers. Repeated names add exponents; equal monomials add coefficients;
zero coefficients disappear. A zero polynomial has no terms. Published monomials
are unique and sorted lexicographically by their lists of named powers. No arena
indices, expression hashes or allocation order enter equality or export.

Construction accepts unordered presentations, repeated powers and terms, and
explicit zeros. Inspection returns the immutable canonical terms. Addition,
subtraction, multiplication, natural powers, simultaneous substitution and exact
rational evaluation preserve the same normal form. Substitution retains
unmentioned parameters and evaluates all replacements in the original scope;
it does not recursively substitute a replacement into another replacement.
Duplicate binding names refuse, including unused bindings. Evaluation requires
every referenced name; unused supplied values still undergo validation.

Equality means polynomial identity over unconstrained real assignments. It is
not an implication solver: `p²` and `p` differ here even when a separately
declared source domain might restrict `p` to `{0,1}`. Eliminating one categorical
simplex coordinate is an explicit substitution, not an implicit assumption.

This module is an arithmetic foundation for M6. It does not yet attach a
parameter domain or polynomial density to a Space, designate a probability law,
introduce parameter guards into Event identity, or produce a parameterized
probability observation. Existing Space coordinates remain finite outcomes.
Parameterized law admission, positive-evidence domains, solver certificates,
BEVT integration, SDK transport and database observation heads remain required.
BEPL cannot substitute for that complete source descriptor.

## Explicit Beta moments

`integrate_beta(p, a, b)` eliminates exactly one named parameter using

```text
E[p^0] = 1
E[p^(n+1)] = E[p^n] (a+n)/(a+b+n).
```

Both shapes must be strictly positive exact rationals, even for zero or
parameter-independent inputs. Other named parameters remain symbolic. This is
a linear moment operation on polynomials over the full `[0,1]` interval. It does
not integrate rational functions or truncated/constrained prior domains.
A source-level binding must separately establish that this complete interval is
allowed independently of the remaining parameter assignment.

Compose all experiments sharing the prior allocation before integration. Two
heads under one uniform prior integrate `p²` to `1/3`; separately integrating
`pq` under two explicitly independent uniform priors gives `1/4`. Integrate the
unnormalized numerator and evidence separately, then condition. Cancelling the
symbolic ratio first would lose the evidence's weighting of the prior.

The shared-bias fairness test retains numerator `p(1-p)` and evidence `2p(1-p)`.
Their algebraic relationship is exact, but the conditional ratio is undefined
at both endpoints. A later guarded observation must retain that domain even if
its reported value simplifies to `1/2`.

## BEPL version 1

This standalone arithmetic envelope leaves BEVT, BERA, BEDC and BESC unchanged.
It introduces no persisted field kind, package version or release.

The header is ASCII `BEPL`, byte `1`, followed by a little-endian u64 term count.
For each term, write a u64 length and canonical BERA coefficient bytes, then a
u64 power count and that many `(32-byte ParameterId, little-endian u32 exponent)`
pairs. Terms and powers occur in canonical order. Zero is the header and a zero
term count. The coefficient must be nonzero; powers must be positive; parameter
names and monomials must be strictly increasing within their respective rosters.

Decoding checks these invariants directly. Noncanonical scalar bytes, alternate
zero presentations, duplicate/unordered names or monomials, zero powers,
truncation and trailing bytes refuse. An unknown version refuses explicitly.
Successful byte decoding validates polynomial syntax and identity, not a source
domain or law. The fixed constant-one fixture and independent test encoder guard
the wire grammar.

## Resource and proof boundaries

`PolynomialLimits` bounds input/intermediate term counts, factors per monomial,
total monomial degree, structural work and wire bytes. Structural steps count
visited terms/factors and construction operations, not sorting comparisons or
bigint limbs. `ExactArithmetic` supplies a shared operation/bit budget across
the whole call; exponentiation uses repeated squaring. Beta integration visits
successive degrees up to the largest requested power, retaining only requested
moments. It obeys both budgets.

All authored inputs are validated before algebraic simplification. A zero
multiplier, zero exponent, unused replacement or zero output cannot hide an
oversized operand or invalid prior. Conservative intermediate bounds can refuse
calculations whose final reduced result is small. Refusals and cancellation are
errors, never polynomial zero or evidence impossibility. These per-operation
limits do not establish the separate aggregate retained-memory gate.

The [Polynomials reference](../crates/bumbledb-event/semantics/Polynomials.lean)
proves 23 rational-assignment laws for term merging, multiplication, powers,
simultaneous substitution, shared-bias observations and explicit moment examples.
It does not prove analytic Beta integration, normal-form completeness, Rust
refinement or codec correctness. Native tests independently check signed integer
cross-products at 625 input/assignment pairs, 55 Bernstein elevation identities,
36 conjugate-update cases, canonical bytes, malformed inputs and bounded refusal.
A stored Rust consumer integrates an explicitly declared full prior cube into a
checked finite law. Its shared-versus-separate prior correlation survives reopen,
both Free Join execution paths and release of the database and original owners.
This verifies the completed prior specialization, not storage of a live unknown
parameter family.

The next source layer must check nonnegative normalized laws on each parameter
fiber and distinguish parameter guards from outcome coordinates. Neither
evaluating a rational grid nor comparing unconstrained coefficients establishes
those general semialgebraic obligations.
