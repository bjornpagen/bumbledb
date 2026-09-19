# Signed expectations under shared-parameter laws

`FiniteFunction::parameter_expectation` contracts a total finite signed payoff
under its captured parameter-family law. The result is an owned partial function
of the same unknown parameter. It retains the payoff, evidence, exact numerator,
evidence-mass function and defined domain. No prior over parameters is introduced.

This extends the existing `FiniteFunction`: its rational values select ordinary
Event regions and its omitted regions have an explicit zero default.
[`FamilyFunction`](event-family-functions.md) additionally admits guarded rational
payoff values depending on the parameter itself, with the same owned conditional
observation contract. Neither construction introduces a prior over parameters.

## A payoff can return an exact region of parameter cases

For a source with `P(heads)=p`, paying `+4` on heads and `-2` on tails gives
expected payoff `6p-2`. The whole function is retained, and its positive region
is exactly `p>1/3` within the captured source domain.

```rust
let payoff = FiniteFunction::new(&source, &[
    FunctionPiece { region: heads.clone(), value: reward }, // +4
    FunctionPiece { region: heads.complement(), value: loss }, // -2
], limits.functions, &mut work)?;
let observation = payoff.parameter_expectation(&source.full(), limits, &mut work)?;
let profitable = observation.conditional().where_sign(
    PolynomialSigns::POSITIVE, limits.parameters.region, limits.functions, &mut work,
)?;
let refined = ParameterRefinement::new(
    new_identity, &source, std::slice::from_ref(&profitable), limits, &mut work,
)?;
let profitable_worlds = refined.refined().parameter_event(&profitable, limits, &mut work)?;
```

`profitable_worlds` is again an ordinary Event. It can be complemented, joined
with other conditions, stored, and used in pointwise keys and containments.
This is sensitivity analysis: it describes assumptions under which the payoff
is positive. An actor cannot use an inaccessible parameter as policy information;
the information/FD check remains a separate modeling obligation.

For two explicitly conditionally independent draws sharing unknown `p`, observe
that they differ. The same `+4/-2` payoff on the first draw has conditional
expectation `1` on `(0,1)`. Its numerator is `2p(1-p)` and evidence mass is
`2p(1-p)`. Both are retained; the constant value does not erase the undefined
endpoints. Even a zero payoff remains undefined at zero-evidence parameters.

## Owned observation and exact contraction

`ParameterExpectationObservation` exposes `space()`, `function()`, `evidence()`,
`numerator()`, `evidence_mass()`, `conditional()`, `defined_on()` and
`is_impossible()`. `value_at` returns an exact signed rational or `None` at an
undefined/outside parameter. Values may be negative or exceed one. An
everywhere-undefined observation can still have structurally possible evidence.

The constructor aligns the evidence even for a zero payoff, requires a
designated family law, and computes

```text
numerator(p) = sum_payoff_cells value(cell) * mass_p(cell & evidence)
conditional(p) = numerator(p) / mass_p(evidence), when mass_p(evidence) > 0.
```

Each mass contraction fixes the parameter guards and sums outcomes only. All
terms share the same actual `p`. Signed cancellation does not turn an undefined
conditional into zero. Linearity holds for payoffs with the same evidence and
source; an indicator payoff agrees with `parameter_probability`.

Function-cell/work limits, parameter-source/solver limits, graph operations and
the caller's shared exact-arithmetic budget remain explicit. Nested contractions
retain their own per-operation shape/work limits; there is no aggregate retained
memory claim. No observation or partial answer is published on refusal.

The result owns the source and inputs after external owners and the database
are released. Existing Free Join Event results can feed this host operation.
It does not add native expectation query heads, an observation field/codec, SDK
authoring, or the query-roster coverage rule. Missing query value rows must still
be rejected before building a total payoff; they cannot inherit its zero default.

## Verification scope

Six core tests include an independent four-outcome oracle for 48 observations
at nine parameter values (432 signed expectation cases), exact function
equivalence and linearity, indicators, zero-payoff holes, impossible evidence,
irrational singleton domains, profitable-case Event construction, independently
decoded inputs, retained owners and resource/cancellation refusal. A persisted
posterior consumer constructs expectations from results of both Free Join paths
after query/source/database release, retaining undefined endpoints.

[ParameterExpectations.lean](../crates/bumbledb-event/semantics/ParameterExpectations.lean)
adds twelve reports over a generic field for finite-sum laws, contraction through
the supplied payoff-cell presentation, numerator linearity, exact conditional
definedness and indicator agreement. A rational example retains signed values
outside `[0,1]`. Complete outcome rosters, normalized laws and the admitted cell
representation remain explicit premises. These proofs do not verify Rust graph
execution, the solver, native ownership or future query-observable admission.
