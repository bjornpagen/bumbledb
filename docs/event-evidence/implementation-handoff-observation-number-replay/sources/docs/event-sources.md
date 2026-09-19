# Exact fixed finite Event laws

The native finite source slice designates a normalized rational joint law on an
existing `Space`. It preserves the Event algebra, original support and ordinary
schema fields. Probability is an owned observation of an Event, never its key.
The [finite function/channel layer](event-functions.md) now adds weighted images
and checked conditional sources. [Univariate parameter families](event-parameter-sources.md)
now have a separate native source contract. General family channels/revisions,
multivariate solving, TypeSafe adapters and query observation heads remain open.

## Representation and construction

`Space::with_density` accepts disjoint `DensityPiece { region, density }` inputs.
A density is the exact mass **of each legal world** in that region. Unmentioned
worlds have zero mass and remain legal. The constructor validates every input's
owner, including empty/zero pieces, refuses negative densities and overlap, and
requires `sum(count(region) * density) == 1`. It never repairs the supplied law
by dividing by its total. This is source construction, not a schema weight field.

Every coordinate in this implemented fixed finite fragment denotes an outcome
bit. A parameter guard is not a stochastic coordinate. On a source constructed
with `with_parameters`, `with_density` instead embeds constants into the family
law constructor, summing outcomes separately in each parameter fibre. It never
turns feasible guard valuations into equiprobable outcomes.

Equal-density regions are unioned, zero and empty regions are omitted, and
nonzero pieces are ordered by their canonical rational bytes. The result is a
canonical partition by density, independent of authored piece decomposition,
allocation or graph working order. It can describe any fixed finite rational law;
a distinct density at every world may still require exponentially many pieces.
A one-piece uniform law over 62 outcome bits stays symbolic.

The owner retains region references in its shared Boolean arena and exact
rational densities. It does not retain self-owning Events, avoiding an `Arc`
cycle. Canonical law bytes belong to the owner. `density_pieces()` exposes
ordinary owned Event regions and borrowed exact densities for inspection.

```rust
use bumbledb::event::{ArithmeticLimits, DensityPiece, ExactArithmetic,
    ExactRational, LawLimits, Space, SpaceId};

let mut math = ExactArithmetic::new(ArithmeticLimits::default(), &());
let outcomes = Space::new(SpaceId([7; 32]), 2, &())?;
let matching = outcomes.table(3, &[0b1001], &())?; // 00 or 11
let half = ExactRational::fraction("1", "2", &mut math)?;
let source = outcomes.with_density(&[
    DensityPiece { region: matching, density: half },
], LawLimits::default(), &mut math)?;
let first = source.coordinate(0, &())?;
let second = source.coordinate(1, &())?;
let observation = first.probability(&second, &mut math)?;
assert_eq!(observation.evidence_mass().to_string(), "1/2");
assert_eq!(observation.value(&mut math)?.unwrap(), ExactRational::one());
// 01 and 10 remain legal, although both have zero probability.
# Ok::<(), bumbledb::event::Error>(())
```

The same marginals can belong to a uniform four-world law where the conditional
above is one half. No operation infers independence from separately supplied
marginals or from distinct source names.

## Observation and ownership

`Event::mass` contracts each density piece against the Event using exact legal
counts, including skipped outcome multiplicity and excluding decoder aliases.
`Event::probability(evidence)` retains the designated source, the intersection
mass and evidence mass. `value()` computes their exact ratio or returns `None`
for zero evidence mass. `is_impossible()` means undefined conditioning, not
structural impossibility. `MissingLaw` is a separate error.

A fixed finite source has one parameter assignment, so the positive-evidence
domain is either that singleton or empty. General parameter observations must
retain their explicit function/domain objects; this finite API is not a claim
that every future observation has a scalar value. The [SDK](event-sdk.md) now
returns owned fixed-law observations with canonical Event/evidence inputs,
numerator, evidence mass and a rational-or-null value. Database observation
fields and query observation slots remain separate gates.

Designating a law creates a new immutable owner. `align_to` and diagram rebuild
require identical designated law bytes as well as source name and support.
Empty/full Events retain those distinctions. `in_space` explicitly transfers
structure between related shared-arena contexts; it makes no assertion about
preserving probability. `restrict` produces an **unmeasured** structural view,
even when its input is measured. `unmeasured` explicitly forgets a designation.
Conditioning/revision is separate from structural restriction.

Structural maps can connect measured endpoints, but do not certify law
pushforward. Logical products remain unmeasured until an explicit joint law is
designated. BEDC carries measured endpoint Events through BEVT v2 and still
reconstructs its structural certificates; it adds no probabilistic certificate.

## Arithmetic, persistence and limits

`ExactRational` is a reduced arbitrary-precision numerator and positive
denominator. Decimal text is parsed exactly; finite binary64 import preserves
the exact IEEE rational, so decimal `0.1` and binary64 `0.1` differ. No rounding
enters normalization or conditioning. `ArithmeticLimits` bounds conservative
input/intermediate bit extents and arithmetic steps. Conservative limits may
refuse a calculation with a small reduced answer. Cancellation is checked around
bounded bigint primitives, not within every limb of a library multiplication.
Standard bigint allocations are not individually fallible allocations.

`LawLimits` bounds input pieces (65,536 by default) and retained canonical law
bytes (16 MiB by default). These are separate from graph limits and the default
65,536-bit/1,000,000-step arithmetic budget. They are not an aggregate process
or registry memory quota. Arena statistics exclude law storage. Encoding an
individual piece can consume graph workspace before its payload-size check.

[BEVT v2](event-value-format.md) encloses the original v1 structural Event and a
canonical BELW v1 law. BERA v1 encodes each rational. Decoding validates exact
arithmetic, every region's context, disjointness and normalization, then checks
canonical reconstruction. The explicit `from_bytes_with_source_limits` variant
allows caller-selected law/arithmetic budgets; ordinary decoding uses defaults.
Measured values persist, reopen, deduplicate and join through existing Event
fields. Version 1 bytes retain their original meaning. No release is made.

## Evidence and remaining obligations

[FiniteDensity](../crates/bumbledb-event/semantics/FiniteDensity.lean) contains
20 reports, six axiom-free. It proves density/count contraction, disjoint
regrouping, conditional complement, repeated evidence, zero-evidence handling,
skipped-outcome multiplicity and separation of context laws. Finite rational
weights use integer numerators with a shared positive denominator. The roster
must enumerate original legal worlds exactly once. Native graph counting,
arbitrary-precision arithmetic, parser admission and ownership establish those
premises through tests; they are not extracted Lean implementations.

Twelve core tests add 61,440 exhaustive four-world conditional observations,
small exact-arithmetic oracles, a 62-coordinate symbolic law, nonempty zero-mass
regions, canonical decomposition/order checks, malformed source bytes, measured
map descriptors and cancellation/capacity refusal. Three database consumers
cover reopening, owned results, law-sensitive Free Join on resident/cursor paths,
zero-mass keys/coverage and transaction rollback. The raw Node runner checks
measured persistence/deduplication, complement and malformed-law refusal.

The [qualification record](event-evidence/native-finite-law-qualification/check.json)
and [semantic run](event-evidence/native-finite-law-semantics/check.json) pin this
fixed-law checkpoint. Conditional generators now have separate
[implementation and evidence](event-functions.md);
[finite revisions and signed expectation](event-revisions.md) likewise have
separate contracts and retained mathematical receipts. [Parameterized sources](event-parameter-sources.md)
add shared univariate domains, guarded laws and owned function-valued host
observations. Multivariate solving, general family dynamics, model import intents
and source-aware observation query results still keep M6 open.
