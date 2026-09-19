# Explicit parameter guard refinement

`ParameterRefinement` changes the finite presentation of a source while keeping
its actual `(parameter, outcome)` worlds and any designated law. It makes new
parameter predicates expressible as ordinary Events. The original owner stays
immutable; existing schema values and fixed-point programs keep their meaning.

For example, an Event representing heads may have no way to express `p > 1/2`.
Refinement adds that deterministic guard. The new Event can represent
`heads & (p > 1/2)` and its complement, participate in pointwise keys/containments,
and retain the exact mass function. This does not sample a new bit or choose a
prior for `p`.

## Native API

```rust
// `source` already owns a parameter domain and optional normalized family law.
// `above_half` is the exact ParameterRegion for 2*p - 1 > 0.
let refinement = ParameterRefinement::new(
    new_identity, &source, std::slice::from_ref(&above_half), limits, &mut work,
)?;
let refined = refinement.refined();
let heads = refinement.lift(&old_heads, &())?;
let high_bias = refined.parameter_event(&above_half, limits, &mut work)?;
let selected = heads.apply(BoolOp4::AND, &high_bias, &())?;
let observation = heads.parameter_probability(&high_bias, limits, &mut work)?;
```

For a source whose heads mass is `p`, this observation is `p` on `(1/2, 1]`,
undefined on `[0, 1/2]`. `selected` remains a first-class Event, not that function
or a thresholded scalar. If the old source lacks this distinction,
`source.parameter_event(above_half, …)` returns `ParameterRefinementRequired`
rather than broadening or narrowing it.

`new` appends one guard per supplied predicate, in authored order. Old semantic
coordinate positions stay fixed, and the outcome mask is unchanged. Repeated
or complementary predicates are deterministic aliases. `with_limits` additionally
selects the new graph owner's resource policy. The caller names the new
presentation explicitly; its persistent Event identity is distinct from the old
one. This is a checked representation change, not implicit source alignment.

The builder transfers original support, reconstructs guard feasibility and
every inhabited parameter fibre, then lifts and revalidates the family law.
There is one value of each new guard at each actual parameter, so no probability
multiplicity is introduced. Zero-mass possibilities remain possible. No Event
owner retains a reference cycle through its law.

## Lifting and exact descent

`lift` preserves every Boolean truth function, actual membership and designated
mass. The new presentation may have more logical atoms; actual world cardinality
does not change. This holds for continuous domains and isolated irrational
parameter values. Inputs may be independently decoded but must align to the
original source, including its law.

`descend` returns an Event in the original presentation only if it has exactly
the same actual worlds. It constructs a candidate by existentially eliminating
the appended logical guard bits from supported membership, then checks that
lifting this candidate recovers the whole input. Failure is
`ParameterGuardEssential`. No approximate envelope is published as the answer.

Thus lifted old Events always descend. A new guard cutting an old logical atom
cannot descend. A compound expression using new guards can descend when those
distinctions cancel. This is exact representability, not dependency analysis of
the expression's syntax. Direct `Event::saturate` still refuses guard elimination.

## Common presentations and existing maps

`ParameterRefinement::common` accepts `(new_identity, source)` pairs and returns
one refinement per input. All inputs must retain the same named inhabited domain.
It collects the canonical guard predicates, clips them to that domain, removes
duplicate predicates and appends those not already expressible in each source.
It does not shrink mismatched domains to their intersection. The resulting
presentations resolve the same exact parameter partition, with each source's
own outcome roster and law preserved.

These sources can now enter ordinary checked `CoordinateMap`, face/fibre products
and relations. A target guard readout is its `parameter_event` in the refined
source. Their maps preserve the same actual parameter assignment, not merely a
matching guard pattern.

[Parameter-domain inclusion](event-parameter-conditioning.md) additionally admits
maps from a smaller domain when each source cell is a whole target cell. Common
refinement itself still requires equal domains; explicit `ParameterRestriction`
constructs the refined prior and the inclusion for a captured smaller domain.

`CoordinateMap::refine_parameters` transports an existing checked map using its
source and target refinements. It lifts every original readout, constructs each
new target guard on the refined source, and runs ordinary map admission again.
The resulting map commutes with lifts of the old Events for pullbacks and images.
If one side still lacks a required distinction, it refuses with
`ParameterRefinementRequired`.

Refined Events already persist through BEVT v3, and maps/products through BEDC.
BESC v2 now transports refinement certificates and SDK authoring supports both
individual and common source refinements. Captured or row-bound guard queries
resolve a predicate roster on one source; multiple-source query
construction remains open. No public package version or release changes.

## Evidence and scope

Six core tests exercise all sixteen Boolean functions, exact membership and mass
preservation, all 64 Events of an independent six-cell descent oracle, map
pullback/image squares, common-partition products, irrational singleton worlds,
cross-order persistence, diagram reconstruction, owners, capacities and
cancellation. A database consumer atomically replaces an old two-branch
partition with a three-branch refined partition, rejects lost coverage, reopens
storage and retains exact branch mass functions through both Free Join paths
after owner/database release.

The [Lean reference](../crates/bumbledb-event/semantics/ParameterRefinement.lean)
adds nine reports for denotation preservation, inclusion reflection, complement,
the existential candidate, exact descent iff guard invariance, uniqueness,
an essential-guard counterexample and unchanged outcome-density rosters. It
assumes the constructor establishes the logical projection's surjectivity and
commutation with actual worlds. It does not verify the native graph transfer,
solver, resource policy or database compiler.

`ParameterSourceLimits` and shared `ExactArithmetic` bound admission; graph limits
bound lifting/descent. The original presentation and its fixed-point atom bound
never mutate. These are per-operation policies, not aggregate retained-memory
accounting. [BESC v2](event-source-descriptors.md) now reconstructs refinements
and restrictions while retaining original-source translations. Multivariate
solving, parameter-changing maps, multiple-source query refinements,
TypeSafe import intents, complete SDK and M0–M8 qualification remain required.
[Guard queries](event-observation-predicates.md) now refine one captured or row-bound source
for a bound predicate roster and return ordinary Event cases with exact transport.
