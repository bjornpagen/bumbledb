# Shared parameters in first-class Event sources

A native `Space` can now retain one exact real parameter, its inhabited domain,
a sealed roster of logical guards, and an optional normalized rational-function
law. Its Events use the ordinary owned `Event` type, schema fields, FD/INDs,
canonical rows, registry and Free Join. Parameters do not become schema weights
or independent sampled bits.

The mathematical worlds are `(p, outcome)`, with `p` in the captured domain and
an outcome allowed by the source's support. The native diagram contains a finite
logical quotient of these worlds: `(guard(p), outcome)`. Guard predicates can
include strict bounds, disconnected regions and irrational singleton points.

## Constructing and measuring a source

`Space::with_parameters(domain, guards, limits, work)` interprets declared
coordinates as deterministic parameter guards. Remaining coordinates are outcome
bits. The input must be unmeasured and not already parameterized. The constructor
checks names and coordinates, computes exact feasible guard combinations, removes
impossible combinations, and requires a legal outcome at **every** admitted
parameter assignment. A missing fibre is `EmptyParameterFibre`, not an implicit
change to the requested parameter domain.

For a coin with unknown `p` in `[0,1]`, declare an outcome bit for heads and,
optionally, guards such as `p=0` and `p=1`. The two endpoint guards cannot both be
true. Both heads and tails may remain possible at either endpoint.

`Space::with_parameter_density` assigns `GuardedRationalFunction` per-outcome
mass to disjoint Event pieces. It establishes definedness, nonnegativity and unit
outcome sum on each real parameter fibre. Missing pieces have zero mass and
remain possible. A density may have holes outside its Event's active parameter
cases. Its complete arithmetic inputs still participate in validation.

```rust
// `domain` is the checked [0,1] domain of the named polynomial `p`.
let scoped = raw.with_parameters(domain.clone(), &guards, limits, &mut work)?;
let heads = scoped.coordinate(0, &())?;
let law = scoped.with_parameter_density(&[
    ParameterDensityPiece {
        region: heads.clone(),
        density: GuardedRationalFunction::new(
            domain.clone(), p.clone(), ExactPolynomial::one(),
            limits.parameters.region, &mut work,
        )?,
    },
    ParameterDensityPiece {
        region: heads.complement(),
        density: GuardedRationalFunction::new(
            domain, one_minus_p, ExactPolynomial::one(),
            limits.parameters.region, &mut work,
        )?,
    },
], limits, &mut work)?;
let heads = heads.in_space(&law, &())?;
let observation = heads.parameter_probability(&law.full(), limits, &mut work)?;
```

Ordinary `with_density` on a parameterized source embeds the explicitly supplied
rational constants as fibre densities. It never divides by the number of guard
cases. A guard and two skipped outcome bits therefore need per-outcome density
`1/4`, regardless of how many parameter cases are feasible.

`Event::parameter_mass` returns an owned `ParameterFunction`: disjoint guarded
rational pieces selecting parameter cases. It fixes every guard and sums only
outcomes, including skipped outcome multiplicity. `parameter_probability` retains
the source, original Event/evidence, contracted numerator, evidence function and exact positive-evidence
domain. Undefined conditioning returns `None` at a point; an everywhere-undefined
observation is distinct from structural emptiness and from a missing law.

For two explicitly conditionally independent draws sharing `p`, the four joint
masses are `(1-p)²`, `p(1-p)`, `p(1-p)` and `p²`. Given that the draws differ,
first-heads has numerator `p(1-p)` and evidence `2p(1-p)`. The conditional value is
exactly `1/2` on `(0,1)` and undefined at both endpoints. Neither the observation
nor database storage invents a prior on `p`.

[`Space::parameter_condition`](event-parameter-conditioning.md) now materializes
this posterior family on its positive-evidence domain. Its receipt retains the
original evidence-mass function, including zero-evidence endpoints. All outcomes
remain structurally possible at admitted posterior parameters.

`ParameterFunction` also supplies restriction, addition, subtraction,
multiplication, division, exact sign regions, rational evaluation and numerical
partial-function equality across different piece decompositions. Arithmetic
retains inherited holes, including multiplication by zero. Missing pieces mean
undefined, not a zero default. Functions do not independently identify source
laws or observation provenance.

The existing `mass`/`probability` scalar APIs refuse parameter-family measurement
with `ParameterizedMeasurement`. This prevents squeezing a function into a fixed
rational result. General owned observation query heads and the complete source
SDK remain required work.

[`FiniteFunction::parameter_expectation`](event-parameter-expectations.md) now
measures total signed finite payoffs under these same laws, retaining their exact
conditional functions and original evidence. General parameter-dependent payoff
values and owned observation query/SDK results remain separate gates.

## Ownership, restriction, maps and products

The Event owner retains an immutable parameter context, exact guard-cell regions,
canonical binding bytes and optional law. Law cells retain raw arena references
and arithmetic functions; they do not hold Events that would create ownership
cycles. Empty/full Events and query results retain the same context. Aligning
independently decoded Events checks the binding and declared law as well as
source identity and support. A diagram snapshot also checks that binding before
reconstruction.

Structural restriction remains explicit and unmeasured. It keeps exactly those
parameter cases that still have a legal outcome and captures their union as its
new inhabited domain. It does not condition or normalize a probability law.
`ParameterRestriction` instead narrows only the parameter domain, retaining every
outcome fibre and its law. Both operations capture the domain explicitly.
Transfers within the original coordinate interpretation validate support; an
Event cannot silently discard an existing parameter context. `unmeasured`
forgets the law while retaining the domain and guards.

A checked `CoordinateMap` between parameterized sources retains the **same actual
parameter assignment**. The source domain may be a subset of the target domain;
every source cell must equal one whole target cell. Every target guard readout is checked against
that predicate's meaning in the source. Guard coordinates can move or be
complemented when the declared predicates warrant it; ordinary outcome readouts
remain arbitrary Boolean functions.

Different guard partitions require [explicit common refinement](event-parameter-refinement.md),
which now supplies checked lifts and exact descent. A coarse matching code is
insufficient for an exact parameter-preserving image, so an unrefined map still
returns `ParameterRefinementRequired`. Mapping between a parameterized and parameterless
space also requires a separate scope-changing operation. It is not an implicit
way to forget or manufacture a parameter.

Face/fibre products capture one common parameter assignment and retain every
compatible outcome tuple. Their duplicate guard coordinates are deterministic
aliases. Products remain unmeasured. BEDC descriptors reconstruct these checked
maps and products using their embedded Event sources. For weighted finite-function
images, each reachable target case admits exactly one source guard pattern, so the existing
finite-code sum counts each outcome once; it does not assign a distribution over
parameters. Unreachable target parameter cases receive zero. General
parameter-family channels and law pushforwards remain open.

`Event::saturate` may forget outcome coordinates while retaining the parameter;
it refuses direct guard-bit elimination. `ParameterRefinement::descend` can remove
appended guards only after proving exact representability in the old presentation.

## Logical cells and actual worlds

`atom_count` counts legal cells of the sealed presentation. `world_cardinality`
returns `Finite(n)` or `Continuum` for actual worlds. `count` returns the finite
count and refuses a continuum with `InfiniteWorlds`. A finite result outside
`u64` refuses with `WorldCount`. An empty Event has zero worlds even in a continuous
source. An irrational singleton parameter contributes one parameter assignment,
not zero rational-grid samples.

A `ParameterWorld` pairs a `RealWitness` with an outcome code whose guard bits are
absent. `contains_parameter` derives the guard bits and checks actual membership;
`parameter_witness` returns an exact semantic witness. Existing `contains` and
`witness` operate on the finite logical code presentation.

`FiniteCarrier::atoms()` now seals the finite Event algebra rather than asserting
that every parameter value is a separate finite world. Every admitted Event is
constant on each legal cell, and typed iteration cannot introduce new guards.
The usual monotone fixed-point bound therefore uses the number of cells even
when their denotation contains infinitely many worlds. This does not certify
unbounded transcripts or an iteration that changes its source presentation.

## BEVT v3 and declared law identity

BEVT v3 contains three length-prefixed blobs, each length a little-endian `u64`:
a canonical BEVT v1 logical Event, a BEPX v1 parameter binding, and an optional
BEPM v1 rational-function law (zero length means unmeasured). Nested measured
Events are forbidden. Existing BEVT v1/v2 bytes remain unchanged.

BEPX stores the canonical BEPR domain and the coordinate-sorted guard roster.
Each guard stores its coordinate byte and canonical BEPR predicate clipped to the
domain. The decoder recomputes feasibility and every nonempty-fibre obligation;
it rejects a structural payload containing impossible guard codes.

BEPM stores definition-sorted density cells. Each cell contains a BEGF v1 function
definition and a BEVT v1 region in the same logical source. BEGF retains canonical
BEPL numerator/denominator and canonical BEPR defined region; its ambient domain
comes from the source. Decoding reconstructs the partial function, rejects
inconsistent definedness claims, checks disjointness and normalization, and
compares the resulting declared law bytes before publication.

This law identity is the **named allocation and validated arithmetic definition**,
not arbitrary numerical equivalence between law presentations on constrained
domains. Equal definitions merge their Event pieces and ignore piece order.
Different fractions or explicit zero definitions may name different source
contexts even when a separate numerical comparison would establish equal values.
Conditional observation numerators and evidence remain inspectable. No schema
row identity depends on an unproved rational-function equivalence.

`ParameterSourceLimits` bounds case rosters, source steps, parameter solving,
function construction and law payloads. Nested arithmetic shares the caller's
budget on admission and measurement. `from_bytes_with_parameter_limits` permits
explicit policy. Operations without such an argument use documented default
source limits. These bounds do not close aggregate retained-memory policy.

## Verified coverage and remaining gates

Ten core tests exercise guard feasibility, missing fibres, endpoint laws,
undefined/negative density, skipped multiplicity, shared-bias conditioning,
zero-mass possibility, semantic cardinality, piecewise arithmetic, actual-parameter
maps/products, weighted images, fixed points, canonical transport, hostile
claims, ownership, capacities and cancellation. Two database integration tests
retain these sources across reopen, both Free Join paths, query-owner release and
pointwise FD/IND coverage/conflict checks.

The SDK accepts owned BEVT v3 carriers through native admission, storage, joins,
reopen and host algebra. `Event.atomCount` distinguishes finite logical cells from
`Event.count`. A pinned cross-language fixture retains the exact source bytes.
Parameter/source construction and function-valued observations still need their
complete TypeScript authoring/result APIs.

The [ParameterSources reference](../crates/bumbledb-event/semantics/ParameterSources.lean)
adds sixteen reports for quotient occupancy/equality/inclusion, feasible-domain
restriction, parameter-preserving maps and exact-image obligations, guard
selection, skipped outcome multiplicity, and shared-bias conditioning. Feasibility,
support uniformity and the map's exact guard-partition correspondence remain
explicit constructor premises. These do not verify the Rust builder, parser,
Sturm solver or Free Join compiler.

Required remaining work includes multivariate domains and general semialgebraic
functions, parameter-changing maps, source channels
and likelihood/Jeffrey revisions over families, observation query heads, TypeSafe import intents,
complete SDK authoring/results, retained-memory policy and integrated performance.
The complete M0–M8 proposal remains the goal; no release or version bump is made.
