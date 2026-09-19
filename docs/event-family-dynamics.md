# Conditional family channels and Jeffrey replacement

`FamilyKernel` and `Space::parameter_jeffrey` extend the captured univariate
source family. They hold the same actual parameter fixed in every outcome sum.
Neither operator supplies a prior over it or converts a model's posterior
answer into an observation likelihood.

## Conditional channels

A checked map `parent: extension → prior` names the old world in each extended
world. A `FamilyFunction` supplies the conditional density on its finite fibres:

```rust
let channel = FamilyKernel::new(&parent, &conditional_density, limits, &mut work)?;
let extended = channel.close(&prior, limits, &mut work)?;
let old_event = extended.parent().pullback(&holding, control)?;
```

Admission checks nonnegativity and unit fibre sum at **every** actual parent
world, including zero-prior worlds and parameter endpoints. The map must be
onto. Invalid categorical encodings belong outside structural support; a zero
channel probability does not exclude an otherwise possible outcome.

Closing under the designated prior constructs

```text
joint(p, extension) = channel(p, extension) * prior(p, parent(extension))
sum_{extension: parent(extension)=old} joint(p, extension) = prior(p, old).
```

The result is a `SourceExtension`, retaining the new normalized law and a
surjective map to the supplied prior. Closing against an explicitly replaced
law is allowed only in the identical named unmeasured structural context.
An unmeasured prior refuses. New outcome identity comes from the extension's
explicit world presentation; repeatedly copying a readout does not create a draw.

For two explicitly conditionally independent Bernoulli channels using the same
unknown `p`, the joint head Event has probability function `p²`. Conditioning on
the first Event retains the same `p`; no integration or parameter learning is
inferred. Other channels can depend on old outcomes and encode dependence.

## Numerical information dependencies

`channel.factors_through(observation, limits, work)` checks whether the conditional
density depends only on an onto extension readout, such as actor-visible state
and the new outcome. The check compares numerical functions at all actual worlds,
not their authored cells or expression bytes. Equal values written as `p` and
`2p/2` therefore agree, even if their author selected different hidden-state cells.

`FamilyFunction::descend` uses the existing symbolic weighted image to build a
candidate:

```text
candidate(p,t) = sum_{s: f(s)=t} value(p,s) / number_of_outcomes_in_that_fibre
```

It returns this candidate **only if pulling it back exactly recovers the input**.
Otherwise it returns `RoleMismatch`; `factors_through` reports false. The count is
positive because the map is onto. This arithmetic device tests an FD; it does
not designate a uniform distribution or grant permission to inspect hidden
parameters. Current same-parameter readouts preserve the parameter itself;
an observation that also hides it needs the pending parameter-changing map work.

## Replacement probabilities

A full `EventPartition` and ordered, total `ParameterFunction` targets specify
new cell probabilities. Targets must be nonnegative and sum to one over the
whole original domain. Partial functions, negative targets, different ambient
domains and non-unit totals refuse before a posterior is constructed.

```rust
let update = prior.parameter_jeffrey(
    presentation_id, &partition, &targets, limits, &mut work,
)?;
```

For cell `i`, let `m_i(p)` be its old mass and `q_i(p)` its target:

```text
unsupported_i = {p: q_i(p) > 0 and m_i(p) = 0}
valid_domain = prior_domain minus union_i unsupported_i
posterior(p,s) = 0                                  when q_i(p) = 0
posterior(p,s) = prior(p,s) * q_i(p) / m_i(p)         otherwise, for s in cell i
```

Zero targets skip undefined within-cell conditionals. They do not evaluate
`0/0` and do not delete worlds. On the valid domain, each new cell mass equals
its target and each positive-target within-cell conditional is preserved.
Repeating the same targets is numerically idempotent. Replacements on overlapping
partitions may not commute. This differs from multiplying likelihoods repeatedly.

`ParameterJeffrey` retains the original source, ordered partition (including
empty cells), every target and old-mass function, one unsupported region per
cell, exact valid domain and optional `ParameterRevisedSource`. An all-invalid
request returns that complete receipt with no posterior. There is no intrinsic
evidence probability for a replacement request.

Piecewise targets and ratio-defined regions can split existing parameter cells.
`ParameterRestriction::with_predicates` captures the valid domain and those
boundaries in one retained refinement of the **original** prior. Additional
expressible or duplicate predicates consume no new coordinates. The posterior's
translation targets that refined prior, while `pullback` accepts original Events.
Endpoint exclusions remain in the receipt instead of silently altering the prior.

## Qualification and scope

Nine new core tests exercise successive channels, exact old marginals,
replacement prior laws, hidden-state FD violations, syntactically different
coefficients, zero-prior rows, unused outcome encodings, repeated Jeffrey targets,
piecewise targets, endpoint `0/0`, empty-cell positions, per-cell invalid regions,
all-invalid requests, point-only valid domains, noncommuting replacements,
ownership, decoding, foreign contexts, budgets and cancellation. An independent
small-world numerical oracle checks each posterior atom across five parameters.
A persisted consumer runs channel and Jeffrey translations through both Free Join
paths and measures retained results after releasing source, query and database.

[FamilyFunctions.lean](../crates/bumbledb-event/semantics/FamilyFunctions.lean)
adds ten generic-field reports for channel marginals/normalization, exact descent,
replacement cell masses/normalization, zero-target skipping and idempotence.
The faithful outcome roster, nonnegative admitted inputs, correct map cells,
exact arithmetic and graph correspondence remain native premises. These proofs
are denotational contracts, not verification of Rust or performance claims.

Event values and checked maps use existing BEVT v3 and BEDC persistence. Family
functions, channels, refinements and full revision receipts do not yet have
transport or complete TypeScript authoring/results. Multivariate/general function
solving, parameter-changing maps, explicit prior integration, TypeSafe import
intents, observation query heads and the remaining M0–M8 gates stay open. Resource
limits cover graph, cells, source steps and shared exact arithmetic; aggregate
retained-memory accounting remains pending. No release or version change.
