# Native Event information operations

Information uses the existing `CoordinateMap` and `WorldRelation` types. A
readout groups legal worlds by their observed value; its information operations
return ordinary Events on the original world space. No parameterized model or
new probability carrier is introduced.

```rust
use bumbledb::event::{CoordinateMap, Space, SpaceId};

let worlds = Space::new(SpaceId([1; 32]), 2, &())?;
let visible = Space::new(SpaceId([2; 32]), 1, &())?;
let observation = CoordinateMap::coordinates(&worlds, &visible, &[0], &())?;
let hidden = worlds.coordinate(1, &())?;

assert!(observation.possible(&hidden, &())?.is_full());
assert!(observation.guaranteed(&hidden, &())?.is_empty());
assert!(observation.ambiguous(&hidden, &())?.is_full());

let cases = observation.information(&hidden, &hidden, &())?;
assert!(cases.guaranteed().is_full());
assert!(!hidden.is_full());
```

The readout sees bit 0. Both values of the hidden bit remain possible in every
observation cell. After supplying evidence that the hidden bit is true, every
reachable cell guarantees it. The answer still describes whole original cells;
intersect with the evidence to select actual remaining worlds. The crate has a
compiled version of this example.

## Operators and evidence

For a total legal readout f and source Event A:

```text
possible_f(A)   = pullback_f(image_f(A))
guaranteed_f(A) = !possible_f(!A)
ambiguous_f(A)  = possible_f(A) & possible_f(!A)
```

`possible` is the least observable region containing A. `guaranteed` is the
greatest observable region contained in A. A source world always witnesses its
own readout, so this full-support guarantee is nonvacuous. The target need not
be fully reachable: pullback never introduces an absent target code as a world.

`CoordinateMap::information(A, G, control)` accepts explicit evidence G and
returns an owned `InformationCases` record of Event results. Both inputs are
checked against the source before construction, including empty/full operands.
Let `yes = possible_f(G & A)` and `no = possible_f(G & !A)`:

| Result getter | Event |
| --- | --- |
| `reachable()` | `yes | no`, exactly `possible_f(G)` |
| `possible()` | `yes` |
| `guaranteed()` | `yes & !no` |
| `ruled_out()` | `no & !yes` |
| `ambiguous()` | `yes & no` |

Guaranteed, ruled-out and ambiguous are disjoint and cover reachable cells.
Impossible evidence yields empty results in all five fields. Unreachable cells
do not become vacuously certain. No legal-support restriction, probability
conditioning, prior, actor memory or inaccessible information is inferred.

These are the proposal's same-space May, Must and Unresolved operations for an
observation partition. Ordinary transition-relation `All` retains its separate
vacuous semantics at dead ends. Incomparable observation saturations need not
commute on coupled support; the native counterexample retains that boundary.

## Information order is a functional dependency

`SurjectiveMap::factor(other, control)` checks whether the first readout
determines the second. It returns the unique ordinary `CoordinateMap` h with:

```text
fine.then(h) = coarse
same fine observation ⇒ same coarse observation
```

Admission descends each coarse output bit through the fine map's legal fibres,
then constructs h and checks its original target support. The fine map is onto,
so the factor is defined and unique over its entire admitted observation space.
The coarse map need not be onto. Differing sources fail even for zero-bit
targets. An undetermined bit fails with `RoleMismatch`; the function never
chooses an arbitrary representative world.

For an individual predicate, `SurjectiveMap::descend` already supplies the same
membership-FD certificate and returns its unique observable Event. Physical
dependency masks, cardinalities and probabilities cannot substitute for either
admission check.

## The relation view is the same algebra

`WorldRelation::indistinguishable(product, observation, control)` constructs
equality of the observation's readout on a supplied full pair product. Both
endpoints must use the observation's legal source and the same environment map.
The observation must determine that environment. Otherwise intersecting
readout equality with product support would silently change its cells, so
construction refuses with `EnvironmentMismatch`.

The admitted relation is reflexive, symmetric and transitive. Its May operation
equals `possible`; its All and Must both equal `guaranteed`. Under evidence,
restrict its target to G, then use May and nonvacuous Must. These agree with the
information-case results above. Refinement between observations is exactly
containment between their indistinguishability relations.

This is a finite structural host API; designated fixed laws do not change its
possible/guaranteed judgments. The environment is an actual finite
readout value. Future shared-parameter sources must retain the real parameter
binding and their stronger capabilities; coarse guard-code equality cannot
stand in for parameter identity. Actor information adequacy remains a supplied
modeling contract.

## Representation and evidence

The direct information operations reuse map image and pullback; they do not
construct a doubled world workspace. A native test observes 61 bits of a 62-bit
space, reasons about the omitted bit and factors a coarser readout symbolically.
The explicit relation view still obeys the full product's combined-coordinate
limit. All outputs retain their Event context after maps and inputs are dropped.
Cancellation/capacity failures publish no partial case record.

[Information.lean](../crates/bumbledb-event/semantics/Information.lean) adds 19
reports for image/pullback lowering, evidence classification/nonvacuity, bitwise
FD factorization, legal range and unique factors, relation kernels and the
environment gate. These reference proofs assume exact admitted maps and legal
world types; they do not verify Rust graph operations or observer knowledge.
The proposal's existing InformationReadout proofs retain the best-bound,
refinement, absorption and optimizer-filter laws.

Eight native tests compare 20,480 four-code readout/predicate cases and 23,040
evidence cases against independent partition oracles, check 540 factorization
cases, and connect information answers to relation modalities and evidence
restrictions. Other cases cover nonlinear readouts, unreachable target codes,
coupled support, environment mismatches, independent owners, foreign constants,
cancellation, lifetime and the 62-bit symbolic case. The persisted relation
consumer computes information from its recovered readout after reopening and
closing the database.

The [semantic run](event-evidence/information-semantics/check.json) records 114
reports across seven modules. The [native qualification](event-evidence/native-information-qualification/check.json)
and [handoff audit](event-evidence/implementation-handoff-information/check.json)
pin exact sources and checks. [Sealed finite programs](event-fixed-points.md)
now supply least/greatest iteration and structural closure. The
[finite partition helper](event-partitions.md) now supplies indexed observable
rosters and checked readout conversion. The [action layer](event-actions.md)
now supplies enabled choices, inhabited uniform permissions and fully observed
strategy witnesses. M7's expectation and belief/information-memory construction
remain open, as do query heads and measured sources in their own milestones.
