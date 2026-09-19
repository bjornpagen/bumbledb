# Parameter-domain inclusion and family conditioning

`Space::parameter_condition` materializes an exact posterior family without
choosing a prior over the unknown parameter. The result owns its original source,
evidence Event, evidence-mass function, positive-evidence domain and optional
posterior. It keeps every original outcome at each admitted posterior parameter,
including outcomes whose posterior mass is zero.

Three operations answer different questions:

| Operation | Meaning | Law |
| --- | --- | --- |
| `Space::restrict` | Keep only worlds satisfying a structural condition | Result is unmeasured |
| `ParameterRestriction::new` | Capture a smaller inhabited parameter domain, preserving its outcome fibres | Retains the original law on that domain |
| `Space::parameter_condition` | Update the outcome law after observing an Event | Renormalizes where evidence mass is positive |

None of them silently averages over parameters or assumes independence.

## A posterior is a family on its valid domain

Suppose two draws explicitly share unknown `p` in `[0,1]`, with joint masses
`(1-p)²`, `p(1-p)`, `p(1-p)` and `p²`. Observing that they differ has mass
`z(p) = 2p(1-p)`. Conditioning produces a family on `(0,1)` with masses
`0`, `1/2`, `1/2`, `0`. It retains all four outcomes there.

```rust
let first = prior.coordinate(0, &())?;
let second = prior.coordinate(1, &())?;
let differ = first.apply(BoolOp4::XOR, &second, &())?;
let update = prior.parameter_condition(
    refined_identity, &differ, limits, &mut work,
)?;
if let Some(posterior) = update.revised() {
    let head = posterior.pullback(&first, &())?;
    let observed = posterior.pullback(&differ, &())?;
    let mass = head.parameter_mass(limits, &mut work)?;
    // mass is exactly 1/2 on (0,1), undefined at 0 and 1.
    assert!(!observed.complement().is_empty());
    // That complement has zero posterior mass everywhere it is defined.
}
// update.evidence_mass() retains 2p(1-p) on the original [0,1] domain.
// update.defined_on() retains the exact open interval (0,1).
```

The parameter endpoints remain explicit in the prior and receipt. They are
outside this posterior's validity domain because conditioning is undefined
there; the update does not assert that those prior worlds were structurally
impossible. A numerically constant answer cannot erase that distinction.

If evidence has zero mass everywhere, `is_impossible()` is true and `revised()`
is `None`. The owned receipt still contains the original evidence, including
nonempty evidence of zero mass. No empty `Space` or fabricated law is returned.
Missing laws, foreign sources, cancellation and resource refusal are errors,
not impossible mathematical outcomes.

## Exact domain restriction and its map

`ParameterRestriction::new(identity, source, predicate, limits, work)` captures
`source.domain ∩ predicate`. An empty intersection refuses with
`EmptyParameterDomain`. Construction refines the prior so the predicate is an
ordinary Event, restricts the parameter domain, and rebases/revalidates any law.
It does not delete outcomes within a retained parameter fibre.

`prior()`, `refined_prior()` and `space()` retain all three contexts.
`refinement()` provides the checked guard lift; `inclusion()` maps the restricted
space into the refined prior. `pullback(old_event, control)` performs both
translations for an Event from the original prior. The caller's `SpaceId` names
the refined presentation, as in `ParameterRefinement`.

Ordinary `CoordinateMap` now admits source-domain inclusion into a larger target
domain while retaining the same named, actual parameter. Every source cell must
equal **one whole target cell**, and target guard readouts must have their declared
meaning. Merely overlapping cells or matching coarse guard codes is insufficient:
it could manufacture an image witness at a different parameter. Refine the
target first when a source boundary cuts one of its cells.

Unreachable target parameter cells have no source witnesses. Therefore:

- `support_image` gives exactly the admitted source parameter subset.
- `universal_image(empty)` contains the unreachable target cells vacuously.
- A proper-domain inclusion cannot receive a surjectivity certificate.
- Finite weighted images sum to zero on unreachable cells and count only outcomes.

`new_with_parameters` exposes source limits and caller-owned arithmetic for map
admission. Ordinary `new` uses defaults. Parameter-changing maps and maps between
parameterized and parameterless contexts still need separate constructors.

## Conditioning construction and owned observations

The constructor contracts the evidence to an exact piecewise `ParameterFunction`,
solves its positive region, and constructs a `ParameterRestriction` to that region.
On each exact guard case it constructs

```text
posterior_density(p, outcome) =
    prior_density(p, outcome) * indicator(evidence)(p, outcome) / evidence_mass(p)
```

The ordinary family-law constructor then proves definedness, nonnegativity and
normalization again. It assigns zero to omitted density pieces while preserving
their structural worlds. `ParameterRevisedSource::translation()` is a checked
map from posterior to **refined** prior. Unlike a fixed-law revision's translation,
it need not be onto: zero-evidence parameter values may have been excluded.

`GuardedRationalFunction::on_domain` and `ParameterFunction::on_domain` explicitly
capture a smaller ambient domain. They retain arithmetic definitions and
inherited holes, refuse extension, and differ from `restrict`, which keeps the
original ambient domain. Repeating the same evidence is numerically idempotent;
different presentations and declared arithmetic definitions can still have
different persistent identities.

`ParameterProbabilityObservation` also retains its original `event()` and
`given()` Events alongside the source, numerator, evidence mass and conditional
function. A value or numerical function equivalence is not provenance equivalence.

Posterior Events already use BEVT v3; their translations use checked BEDC maps.
They persist, join and participate in FD/INDs without a new schema type. Native
typed `Pullback` queries run on both Free Join paths. Complete conditioning and
refinement receipt codecs, function-valued query observations and SDK authoring
remain open; transporting a posterior and its map is not transporting its receipt.

## Evidence and resource contract

Seven core tests cover domain inclusions, vacuity, weighted images, target-cell
refusal, inherited holes, shared-bias conditioning, endpoint receipts, repeated
evidence, impossible conditioning, irrational singleton domains, ownership,
transport, cancellation and resource refusal. Two database consumers reopen
posterior/prior Events, run both Free Join paths after descriptor-owner release,
retain results after database release, and enforce coverage of zero-posterior
outcomes in pointwise FD/INDs.

[ParameterRevisions.lean](../crates/bumbledb-event/semantics/ParameterRevisions.lean)
adds fifteen reports for whole-cell domain inclusion, absence of outside-domain
images, coarse-code unsoundness, finite-sum normalization, exact partial family
domains, repeated evidence and structural support despite zero posterior mass.
Outcome-roster fidelity, admitted law/cell semantics and arithmetic correspondence
remain constructor premises. This is a denotational reference, not a proof of
the Rust solver, graph, codecs or query compiler.

Graph, source, function, law and solver limits retain their existing per-operation
contracts. Nested exact arithmetic shares the caller's budget, including domain
restriction and map inclusion. Aggregate retained-memory accounting is separate
work. [Family functions and likelihood revisions](event-family-functions.md) now
build on this domain contract. Family Jeffrey revisions/channels,
parameter-changing maps, multivariate solving and explicit prior integration
remain open. No performance ranking, version bump or release is claimed.
