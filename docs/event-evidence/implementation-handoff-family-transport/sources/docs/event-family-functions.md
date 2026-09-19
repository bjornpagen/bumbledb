# Exact family functions, weighted images and likelihoods

`FamilyFunction` is a total signed function on a captured `(parameter, outcome)`
space. Ordinary Event cells select guarded rational-function values. This extends
`FiniteFunction` to parameter-dependent payoffs, densities and likelihoods while
retaining the same source, structural possibilities and unknown parameter.

It is a host algebra object, not a new schema field or a weight column. A
`ParameterFunction` instead describes a possibly partial function of the
parameter alone, such as a conditional probability or expectation.

## Construction and algebra

```rust
let payoff = FamilyFunction::new(&source, &[
    FamilyFunctionPiece { region: heads.clone(), value: p_function },
    FamilyFunctionPiece { region: heads.complement(), value: one_minus_p_function },
], limits, &mut work)?;
let observation = payoff.expectation(&evidence, limits, &mut work)?;
```

Each supplied coefficient has the same ambient parameter domain as the source.
The cells must be disjoint, including explicit zero cells. The coefficient must
be defined at **every actual world in its cell**. Holes elsewhere are harmless;
an active hole is `UndefinedFunction`, never an implicit zero. Every written
coefficient and Event is validated even for empty cells. Omitted worlds have
the explicit zero default of a total function.

The function retains its `Space` and shared immutable cells of raw graph
references plus exact arithmetic values. Event owners are reconstructed for
inspection. Explicit zero definitions are retained, so `density(space)` followed
by `designate` preserves the source's declared law definition. General numerical
equivalence of functions does not imply identical source definitions.

| Operation | Contract |
| --- | --- |
| `constant` | One parameter-dependent value on all legal outcomes |
| `from_finite` | Embed a total rational-valued payoff with the same source |
| `from_parameter` | Embed a total parameter function with expressible piece boundaries; holes refuse |
| `add`, `sub`, `multiply` | Pointwise arithmetic at the same parameter and outcome |
| `divide` | Total division; any zero divisor at a legal world refuses without changing support/domain |
| `equivalent` | Exact numerical equality on all actual legal worlds after source alignment |
| `is_nonnegative` | Exact sign check on active parameter fibres, including zero-prior possibilities |
| `where_sign` | Ordinary Event for the exact sign set, or explicit refinement refusal |
| `align_to`, `refine` | Checked context alignment or lift through a `ParameterRefinement` |
| `value_at(parameter, outcomes)` | Exact rational value; `None` outside the parameter domain; illegal outcome codes/support refuse |
| `density`, `designate` | Capture a designated family, or designate a checked nonnegative normalized one |

Arithmetic intersects Event cells, including their explicit zero complements,
and operates on the guarded rational coefficients. Constructor validation checks
the result's active definedness. Numerical equality solves whether a difference
can be nonzero on any active fibre; it does not compare expression bytes.

A sign change inside a sealed parameter cell requires refinement. The caller
can construct the relevant coefficient sign predicates, refine the source,
lift the function with `refine`, then obtain the exact sign Event. No rounding
to coarse guard codes or probability threshold occurs. Hidden parameter cases
still do not become actor-visible policy information without an information FD.

## Sums, pullbacks and weighted images

`outcome_sum(evidence)` sums function values over finite legal outcomes at each
actual parameter and returns a `ParameterFunction`. It needs no probability law.
`expectation(evidence)` first multiplies by the captured law, contracts the
numerator, and retains the original evidence-mass function and conditional domain.
Neither operation sums parameter guards or supplies a prior on the parameter.

The result uses `ParameterExpectationObservation<FamilyFunction>`; the existing
default `ParameterExpectationObservation<FiniteFunction>` remains compatible.
The payoff type only selects the owned host input. Schema fields remain `event`.

For a checked same-parameter map `f: S → T`:

```text
pullback(v)(p, s) = v(p, f(s))
pushforward(w)(p, t) = sum_{s: f(s)=t} w(p, s)
```

Pullback rebases coefficients to a smaller source domain when necessary.
Weighted image decomposes the input into its disjoint Event cells. For each cell
the existing symbolic `FiniteFunction` image kernel counts outcome witnesses,
then multiplies that multiplicity by the cell's parameter coefficient. The
contributions add exactly. A copied readout keeps one shared witness; no world
enumeration, source/target combined coordinate arena, or BEVT serialization is
used by the weighted-image kernel.

Same-parameter map admission requires each source guard cell to be a whole
target cell. That premise makes coefficient factoring sound. An ambient target
domain extension retains the coefficient's original defined region, and output
cells are revalidated. Unreachable target parameter cells receive zero, not a
witness borrowed from another parameter. The weighted pairing law is

```text
sum_t v(p,t) * pushforward(w)(p,t)
    = sum_s pullback(v)(p,s) * w(p,s).
```

On equal parameter domains, a law pushforward has unit total at every parameter
and can be explicitly designated on the target. A proper-domain inclusion gives
zero total outside its source domain; designating it as a law over the larger
target domain refuses. Weighted image itself does not change a target law.

## Explicit family likelihood revision

`Space::parameter_likelihood(identity, factor, limits, work)` applies an explicitly
interpreted nonnegative `FamilyFunction`. It is separate from importing posterior
targets and from conditioning an observed Event.

```text
z(p) = sum_outcomes prior_density(p, outcome) * factor(p, outcome)
posterior_density(p, outcome) = prior_density(p, outcome) * factor(p, outcome) / z(p)
```

Construction retains the exact positive-normalizer domain, revalidates the
posterior law and keeps all outcomes there, including posterior-zero outcomes.
`ParameterLikelihood` owns the original source, supplied factor, normalizer,
defined domain and optional `ParameterRevisedSource`. Its translation targets
a refined prior; zero-normalizer parameters remain explicit in the receipt.
An everywhere-zero normalizer has no posterior. Missing laws, negative factors,
foreign sources and operational refusal remain errors.

A factor can exceed one, and its normalizer need not be an observation
probability. Positive scaling changes the receipt while preserving posterior
values. Reapplying a factor multiplies it again; it is not idempotent evidence
reuse. An indicator factor agrees numerically with Event conditioning on the
same valid domain. Negative factors are invalid even on possible zero-prior
worlds. A provider's posterior answer cannot silently take this likelihood path.

## Evidence and remaining work

Eleven core tests cover active/inactive holes, arithmetic and total division,
exact comparison/refinement, parameter-dependent expectations, law-definition
roundtrips, every four-world map plus 56 restricted-support controls, weighted
pairing, domain-inclusion zeros and a symbolic 62-coordinate image. Likelihood
tests cover scale receipts, repeated-factor composition, parameter-dependent
factors, conditioning agreement, impossible receipts, zero-prior negativity,
owned/decoded posteriors, missing laws, capacities and cancellation. A persisted
Free Join consumer now measures parameter-dependent payoffs after database and
source/query-owner release on both execution paths.

[FamilyFunctions.lean](../crates/bumbledb-event/semantics/FamilyFunctions.lean)
adds seventeen reports over a generic field for finite sums, coefficient
factoring, weighted pairing/total mass, likelihood normalization/scaling and
outside-domain zeros. It retains counterexamples for treating active holes as
zero and accepting negative zero-prior factors. Faithful outcome rosters, exact
map cells, active definedness and arithmetic correspondence remain native
premises. These are denotational proofs, not Rust refinement or performance data.

Function/source/solver and shared arithmetic limits apply. Sparse cell products
and images can still grow exponentially; intermediate rosters can exhaust limits
even for a small final result. Aggregate retained-memory accounting remains open.
[Family channels and Jeffrey replacement](event-family-dynamics.md) now extend
this algebra, including exact numerical descent through onto readouts.
Multivariate/general semialgebraic functions,
explicit prior integration, function/receipt transport, SDK authoring/results and
owned query observations/aggregate coverage remain required. No release or
version bump is made.
