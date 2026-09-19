# Explicit Beta prior binding

`BetaSource` adds an explicitly supplied Beta prior to a measured univariate
source. It retains that source's actual parameter, legal worlds, guard
coordinates and per-parameter outcome law. It does not replace the source with
a finite marginal or change Event equality. The full named parameter domain
must be exactly `[0,1]`, and both Beta shapes must be positive exact rationals.

This is the source-owned observation counterpart of
[`ExactPolynomial::integrate_beta`](event-polynomials.md). The supported
integration fragment is deliberately exact: a **total** raw integrand must
equal one rational-coefficient polynomial except at finitely many parameter
points. A contraction returns that polynomial, the exact exceptional region,
the original function, the prior and the resulting rational value.

## Bind the evidence before normalizing

For two labelled draws sharing an unknown bias `p`, the family law gives
`P(H₁ ∧ H₂ | p) = p²` and `P(H₁ | p) = p`. With an explicit Beta(1,1) prior:

```text
P(H₁ ∧ H₂)       = integral p²       = 1/3
P(H₂ | H₁)       = integral p² / integral p = 2/3
P(H₁) * P(H₂)    = (integral p)²    = 1/4
```

The final line describes a different allocation of uncertainty. It cannot
replace the joint contraction of shared draws. Averaging the already normalized
conditional `p` would give `1/2`, losing the evidence's effect on the unknown
parameter. That original conditional also has a hole at `p=0`, where the first
head is impossible under the fibre law. Binding the two total raw masses gives
a defined posterior while retaining the original hole for inspection.

```rust,ignore
// source already contains the normalized joint law for two shared-bias draws.
let prior = BetaSource::new(
    &source, ExactRational::one(), ExactRational::one(), limits, &mut work,
)?;
let answer = prior.probability(&second_head, &first_head, limits, &mut work)?;
assert_eq!(answer.value().unwrap().to_string(), "2/3");
let original = answer.original(); // original Event, evidence and partial function
let raw_mass = answer.evidence_mass(); // integral certificate; value 1/2
```

`probability`, `expectation` and `family_expectation` construct and bind owned
observations. `bind_probability` and `bind_expectation` also accept admitted
native observations, including final query results. All source contexts are
checked before contraction. A zero integrated evidence mass returns `None`;
it does not return a default probability, delete the evidence Event, or invent
a posterior. Signed payoffs remain signed.

Ordinary final `Probability`/`Expectation` query heads continue to describe
their source's per-parameter law. Binding requires an explicit host operation.
No planner rule inserts a prior, duplicates an uncertain parameter or treats
distinct parameter names as an independence certificate.

## Zero mass retains real possibilities

Positive Beta distributions have no point atoms, including at the endpoints.
An Event supported only at `p=0` can be nonempty and have prior probability zero.
Its indicator agrees with the zero polynomial outside `{0}`. The returned
`BetaIntegral` retains the original indicator and `{0}` as its exception
certificate. Event keys, containments, witnesses and equality still see the
original worlds.

`integrate` first checks the exact ambient domain and total defined domain.
It selects an open piece, performs exact sparse polynomial division on that
piece, and checks agreement with the candidate polynomial on every remaining
piece using exact sign regions. Any differing open interval refuses; isolated
deviations are collected. The polynomial is integrated using the existing
exact Beta moment recurrence.

- Undefined points refuse even if they are isolated or have prior mass zero.
- A total rational presentation such as `p(1+p)/(1+p)` is admitted.
- `p/p` over `[0,1]` still has a hole and refuses as a scalar integrand.
- A genuine step function or `1/(1+p)` refuses as unsupported integration.
- A restricted domain refuses; a truncated prior is never silently normalized.

Conjugate moment identities motivate the implementation, but arbitrary rational
and semialgebraic integration is outside this fragment. Point exceptions do not
authorize almost-sure equality for Events or functions.

## Owned transport and SDK

[BESC v2](event-source-descriptors.md) family tag `6` carries a full measured
BEVT source and two BERA shape blobs. Import replays source admission, shape
positivity and exact domain equality under one arithmetic budget. Full/proper
scope markers, missing laws, malformed/trailing bytes and wrong roles refuse.
The existing Event/source formats and previous tags retain their meanings.

The SDK exposes `BetaSource.new`, `fromBytes`, `toBytes`, `isBetaSource`,
`validate`, `describe`, `integrate`, `probability`, `expectation` and
`familyExpectation`. The envelope carrier owns copied bytes; `fromBytes` alone
does not certify the mathematics. Every operation runs on a cancellable worker.

```typescript
const prior = yield* BetaSource.new(full, yield* ExactRational.fraction(1n),
  yield* ExactRational.fraction(1n))
const answer = yield* BetaSource.probability(prior, secondHead, firstHead)
// answer.value: exact 2/3
// answer.original.value: parameter function p, undefined at p=0
// answer.numerator: retained p², polynomial, exceptions, prior and exact 1/3
// answer.evidenceMass: retained p, polynomial, exceptions, prior and exact 1/2
```

SDK observations retain the original input Event/payoff and evidence, raw
functions, conditional definedness, both integral certificates and the prior.
They use `null` for zero integrated evidence. Supplied plain observation records
are not trusted certificates: these methods reconstruct the observation from
owned inputs. All nested output blobs share the existing 16 MiB output budget.
The fixed descriptor grammar uses bounded item counts; one caller arithmetic
budget covers import, contraction, certification, integration and output.
Late cancellation publishes no result and releases the operation handle.

## Evidence and remaining work

Seven core prior tests include 768 comparisons against independent labelled
beta-binomial world weights, finite-point and rational-presentation certificates,
signed finite/family payoffs, source alignment, exact domain refusal, sparse
division, capacities and cancellation. Descriptor tests cover canonical replay,
bad shapes/scope, every truncation, trailing bytes and shared resource limits.
The database consumer reopens stored Events and a separately persisted BESC prior,
runs both Free Join paths, releases owners and binds retained final observations.
Seven SDK tests cover all exposed operations, persisted joins, copied inputs,
owned results, explicit failures and late cancellation. A Node unit test checks
that nested prior admission cannot reset the arithmetic budget.

`PriorBinding.lean` supplies eighteen reference reports: moment linearity,
evidence-before-normalization, shared versus independent allocation, exact
definedness, totality and preservation of exceptional point values. Its
finite-exception soundness theorem takes non-atomic integration and polynomial
moments as explicit premises. It does not prove the analytic Beta construction,
Rust division/solver/codec correctness or SDK refinement.

This API does not expose a source-changing marginalization map. Multivariate
source binding, observation staging/arithmetic, rational/function payoff query
slots, TypeSafe imports and integrated performance qualification remain in the
[implementation ledger](event-implementation.md).
