# Shared-parameter dynamics in the TypeScript SDK

`FamilyKernel`, `FamilyRevision`, `ParameterRestriction` and
`ParameterRefinement.common` expose the existing native univariate family
semantics. Their results own checked source descriptions, functions, exact
parameter regions and translations. They do not introduce schema weight columns,
independence assumptions, a prior over the parameter, or implicit evidence reuse.

A Bernoulli family with `P(heads)=p` on `[0,1]` illustrates the differences:

| Operation | Head probability afterward | Admitted parameter domain |
| --- | --- | --- |
| Condition on heads | `1` | `(0,1]` |
| Likelihood factors heads→2, tails→1 | `2p/(p+1)` | `[0,1]` |
| Replace heads/tails masses by 1/2 each | `1/2` | `(0,1)` |
| Restrict the parameter to `p > 1/2` | `p` | `(1/2,1]` |

All four preserve every outcome at retained parameters. Posterior mass zero is
not structural impossibility. In the Jeffrey case, the receipt records `{0}` as
unsupported for heads and `{1}` as unsupported for tails, in the original order.

## Updates and original Events

Given a full measured `prior` and a `heads` Event selected from that source, this
runs inside the SDK's managed Effect runtime:

```ts
import { Effect } from "effect"
import { Event, FamilyRevision, ParameterRestriction, ParameterRefinement }
  from "@bjornpagen/bumbledb"

const learn = (identity: Uint8Array, prior: Event, heads: Event) =>
  Effect.gen(function* () {
    const revision = yield* FamilyRevision.condition(identity, prior, heads)
    const result = yield* FamilyRevision.inspect(revision)
    if (result.outcome.kind === "impossible") return result

    const posteriorHeads = yield* FamilyRevision.pullback(revision, heads)
    const probability = yield* Event.parameterMass(posteriorHeads)
    // Exactly one on the positive-evidence domain; undefined outside it.
    const restriction = yield* ParameterRestriction.describe(result.outcome.restriction)
    const refinedHeads = yield* ParameterRefinement.lift(restriction.refinement, heads)
    // Query heads can pull refinedHeads through result.outcome.translation.
    return { revision, result, posteriorHeads, probability, refinedHeads }
  })
```

`identity` is the caller's explicit 32-byte name for the refined presentation. An
everywhere-impossible update retains that requested name and complete receipt,
with no fabricated empty posterior. Missing laws, malformed input, incompatible
contexts, cancellation and resource refusal remain errors.

Every inspection owns `identity`, the original `prior`, the exact `defined`
region, a discriminated `receipt`, and an `outcome`:

| Receipt kind | Retained evidence |
| --- | --- |
| `condition` | Original `evidence` Event and `mass: ParameterFunction` |
| `likelihood` | Original `likelihood: FamilyFunction` and `normalizer: ParameterFunction` |
| `jeffrey` | Ordered `cells`, `targets`, `oldMasses` and `unsupportedRegions` |

A revised outcome owns `posterior`, `refinedPrior`, `restriction` and a checked
`translation` from posterior to refined prior. That map generally is not onto:
invalid parameter values lie outside the posterior. It does not target the
original prior presentation directly. `FamilyRevision.pullback` composes the
retained refinement with this map to translate original Events correctly.
It refuses an impossible revision, which has no posterior.

`likelihood(identity, prior, function)` requires total nonnegative factors on
all legal worlds. Scale survives in the original function and normalizer.
Multiplying all factors by three preserves numerical posterior probabilities
while tripling the normalizer. Applying a likelihood again multiplies it again;
no observation identity or deduplication policy is invented.

`jeffrey(identity, prior, targets)` accepts ordered `{ cell, target }` entries.
Cells form a complete disjoint partition; empty positions remain present. Every
target is a total nonnegative `ParameterFunction`, and targets sum exactly to one.
Zero targets skip undefined within-cell conditionals. Partial or malformed target
functions refuse even on empty cells. Piecewise targets cause explicit predicate
refinement, retain the original target functions, and preserve translations from
the original prior.

## Channels and dependency checks

`FamilyKernel.new(parentMap, density)` checks nonnegativity and unit total on
every parent fibre at every admitted parameter, including zero-prior parents.
`close(kernel, prior)` returns `{ space, parent }`: the full measured extension
and an onto map back to the supplied prior. A different law on the same named
structural parent is accepted; every old marginal and world is preserved. The
shared parameter is not sampled again and acquires no distribution.

A channel assigning the new outcome probability `p`, closed over an old draw
with heads probability `p`, gives joint two-head probability `p²`. A channel may
also depend on explicit parent observations. `factorsThrough(kernel, readout)`
checks numerical function descent through an onto readout. The caller supplies
the intended visibility boundary; the check does not infer actor authority.

`describe` retains the parent map and original density. `validate` replays native
admission. The same owned transport works after the constructing runtime closes.

## Restrictions and common refinements

`ParameterRestriction.new(identity, prior, predicate, additional?)` captures the
inhabited intersection of the prior domain and predicate. It retains the law at
each remaining parameter rather than conditioning the outcomes. Empty
intersections refuse construction. All additional predicates are checked; native
admission skips already representable and duplicate extra predicates.

`fromRefinement(refinement, predicate)` reuses an existing presentation in which
the predicate is already expressible. `describe` owns the original `prior`,
`refinedPrior`, restricted `space`, `refinement` and an `inclusion` into the refined
prior. `pullback` translates an original-prior Event through both steps.

`ParameterRefinement.common([{ identity, source }, ...])` returns refinements in
roster order, joining the predicates of sources on an equal named parameter
domain. Each source keeps its own law and actual worlds. It neither intersects
unequal domains nor couples independently named parameters. The roster must be
nonempty; incompatible domains refuse.

## Transport, resource bounds and evidence

These are nominal owned BESC v2 roles: family kernel tag 2, restriction tag 4 and
family revision tag 5. Revision subtype is reconstructed by the existing native
codec; no second receipt format is invented. Import/validation replays all claimed
updates and checks domains, functions, contexts and impossible-request identities.
Envelope recognition alone grants no mathematical certificate.

One worker arithmetic budget covers every nested input, map, law, update replay
and function/receipt serialization. Input blobs share the existing 16 MiB /
16,384-blob ceiling. Structured outputs have a combined 16 MiB bound; dynamics
receipts also share a 4,096-item budget. Jeffrey authoring accepts at most 2,047
entries, common refinement at most 4,094 sources and restriction at most 62 extra
predicates, subject to stricter nested descriptor, solver and coordinate bounds.
These are operation bounds, not the pending aggregate retained-memory policy.

Ten SDK tests cover the table above, receipt scale and repeated likelihoods,
impossible identities, ordered unsupported regions, partial-target refusal,
piecewise replacement, channels/FDs, zero-prior rows, common refinements, owned
transport, copied submission and late cancellation. A persisted joined query
translates refined-prior Events to the posterior after reopen and runtime release,
and agrees with the host original-prior pullback. One native bridge test checks
shared replay/serialization arithmetic, owned impossible receipts and role errors.

The [native dynamics](event-family-dynamics.md),
[conditioning](event-parameter-conditioning.md),
[family functions](event-family-functions.md) and
[transport](event-source-descriptors.md) Lean reference contracts remain unchanged.
This SDK boundary adds integration evidence, not Rust refinement or performance
proofs. Observation staging/arithmetic, general fixed-point binders,
multivariate/general function solving, explicit prior integration, TypeSafe
provenance/visibility and the remaining M0–M8 gates are still required.
