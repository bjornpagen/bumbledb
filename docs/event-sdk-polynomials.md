# Exact named expressions in the TypeScript SDK

`ExactPolynomial` exposes the native named-parameter algebra through the same
owned, lazy, cancellable runtime as `Event` and `ExactRational`. It supplies the
expression layer for family authoring. It is not a stored field, a source law,
a parameter domain or a probability observation.

```ts
import { Effect } from "effect"
import { ExactPolynomial, ExactRational } from "@bjornpagen/bumbledb"

const sharedBias = Effect.gen(function* () {
  const name = new Uint8Array(32).fill(3) // application-supplied parameter identity
  const p = yield* ExactPolynomial.parameter(name)
  const bothHeads = yield* ExactPolynomial.pow(p, 2n)
  const one = yield* ExactRational.fraction(1n)
  const integrated = yield* ExactPolynomial.integrateBeta(bothHeads, name, one, one)
  return yield* ExactPolynomial.evaluate(integrated, []) // exactly 1/3
})
```

This example explicitly assumes two conditionally independent Bernoulli draws
sharing one unknown bias and explicitly integrates a uniform Beta(1,1) prior on
its full interval. Reusing a parameter name preserves the shared value. Different
names alone establish neither independent priors nor independent observations.
With an explicitly factored pair of uniform priors, the corresponding two-head
probability is 1/4. No prior is supplied by Event algebra itself.

## Public operations

| Operation | Result and contract |
| --- | --- |
| `parameter(identity)` | One unknown named by exactly 32 owned bytes |
| `constant(rational)`, `zero()` | Exact constants |
| `fromTerms(terms)` | Normalize repeated/unordered terms and powers; validate all inputs before discarding zeros |
| `add`, `subtract`, `multiply`, `pow` | Exact native polynomial algebra; powers are unsigned 32-bit bigint inputs |
| `substitute(value, bindings)` | Simultaneous substitution, preserving names that have no binding |
| `evaluate(value, bindings)` | Exact rational result; required names must be bound, all supplied bindings are checked |
| `integrateBeta(value, identity, alpha, beta)` | Explicit algebraic Beta moments over full [0,1], retaining other parameters |
| `equal`, `isZero` | Unconstrained polynomial identity, not equality under a source domain |
| `describe` | Owned canonical terms and exact coefficients |
| `validate` | Re-admit a portable value through native BEPL decoding |
| `fromBytes`, `toBytes`, `isExactPolynomial` | Pure owned transport; `fromBytes` checks the envelope only |

A `PolynomialTerm` contains an `ExactRational` coefficient and ordered input
`PolynomialPower` records `{ parameter: Uint8Array, exponent: bigint }`.
`fromTerms` accepts unnormalized order, duplicate monomials, repeated parameters,
zero coefficients and zero exponents. Native construction publishes a canonical
sparse normal form. `describe` returns frozen term/power arrays and records;
parameter byte arrays are independent copies, so editing them does not mutate
the polynomial. Exact scalar carriers also retain independent ownership.

`ParameterBinding<A>` is `{ parameter: Uint8Array, value: A }`. Substitution uses
polynomial values; evaluation uses rational values. Substitution replaces names
simultaneously: swapping `p` and `q` in `p + 2q` gives `q + 2p`. It does not then
rewrite the replacement expressions again. Duplicate bindings refuse; unused
bindings are still admitted, including malformed or oversized values.

`p²` and `p` differ as polynomials even when a separate domain might constrain
`p` to {0,1}. Evaluating a rational assignment does not certify source-domain
membership. Beta moments are algebraic operations on polynomials; applying them
to a constrained family requires a separate source/prior contract. Integrate
unnormalized evidence and payoff first, then divide by the integrated evidence
mass. This API does not implement general rational-function integration.

## Ownership, bounds and evidence

BEPL v1 remains unchanged. Opaque carriers own their bytes; `toBytes` copies them,
spreading a wrapper cannot forge a value, and carriers survive runtime release.
Pure import and constructing Effects load no addon. Canonical polynomial admission
and arithmetic run on the native worker; JavaScript only checks and marshals input
shapes and reconstructs owned result records. Shared, detached and oversized input
buffers refuse, and mutable buffers are copied before worker dispatch.

One native `ExactArithmetic` budget covers all decoded operands, the operation
and encoded result. Polynomial limits also bound terms, factors, degree, traversal
steps and wire size. SDK requests have a 16 MiB combined input bound, at most
65,536 terms or bindings, and 256 raw powers per authored term. Native canonical
outputs have a 16 MiB bound. Intermediate expansion may exceed a bound even when
an eventual reduced answer is small. These are operation/shape bounds, not the
pending aggregate retained-memory policy. Cancellation drains owned outputs;
it cannot return polynomial zero as a substitute for failed work.

Five SDK tests cover normalization, symbolic arithmetic, simultaneous substitution,
explicit prior correlation, immutable transport, runtime release, noncanonical
inputs, absent/duplicate/unused bindings, metadata/buffer refusals and late
cancellation of a completed structured description. The stored consumer builds
checked joint laws from the explicitly integrated full prior cube; 1/3 versus
1/4 survives database reopen and query construction. This is completed prior
specialization, not SDK construction or integration of a live source family.
Two native bridge tests check shared arithmetic exhaustion, malformed metadata,
unused invalid operands and cancellation. Existing native polynomial tests and
23 [Lean reference reports](../crates/bumbledb-event/semantics/Polynomials.lean)
continue to supply the arithmetic contracts; no new Rust/codec refinement claim
is made. Domain/guard/family builders, owned family observations and remaining
M0–M8 work stay open. No package version change or release.
