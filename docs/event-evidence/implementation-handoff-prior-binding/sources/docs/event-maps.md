# Checked Event maps

`bumbledb::event::CoordinateMap` is the native finite structural map layer on
`codex/event-algebra`. The [relation core](event-relations.md) and
[information operations](event-information.md) now build on it;
captured readout [query heads](event-queries.md) and [finite descriptor transport](event-descriptors.md)
are also implemented. The complete source/SDK API remains later work. It neither changes
BEVT v1 nor adds a release.

A map is a total function from legal source worlds to legal target worlds.
Its target bits are arbitrary Event readouts on the source: constants, repeated
coordinates, projections, permutations and nonlinear Boolean functions all use
one constructor. The source and target retain their own named identities and
original support. A map does not introduce a draw or a probability law.

[Parameterized sources](event-parameter-sources.md) additionally retain one actual
parameter across each map. Each source cell must equal one whole target cell;
the source domain may be a subset of the target domain. Explicit
[guard refinement](event-parameter-refinement.md) supplies common presentations;
`CoordinateMap::refine_parameters` lifts existing readouts and reconstructs new
guards before rechecking map admission. [Domain inclusions](event-parameter-conditioning.md)
retain unreachable target cells and their vacuous universal images. Maps that
change or forget the actual parameter remain open.

```rust
use bumbledb::event::{CoordinateMap, Space, SpaceId};

let source = Space::new(SpaceId([1; 32]), 1, &())?;
let target = Space::new(SpaceId([2; 32]), 2, &())?;
let copied = CoordinateMap::coordinates(&source, &target, &[0, 0], &())?;

let reachable = copied.support_image(&())?;
assert_eq!(reachable.count(&())?, 2); // exactly 00 and 11
assert!(!reachable.contains(1)?);    // 01 is not reachable
assert!(!reachable.contains(2)?);    // 10 is not reachable

let first = target.coordinate(0, &())?;
let second = target.coordinate(1, &())?;
assert_eq!(copied.pullback(&first, &())?, copied.pullback(&second, &())?);
```

Each individual readout above reaches both Boolean values. Their joint range
retains the copied-bit constraint. Neither marginal surjectivity nor equal
probabilities would establish the missing off-diagonal worlds.

## Public contract

For `f: S → T`, an Event `A` on S, and an Event `B` on T:

| Rust operation | Meaning |
| --- | --- |
| `new(source, target, readouts, control)` | One checked source Event per target bit; every legal source world must map into target support |
| `new_with_parameters(source, target, readouts, limits, work)` | The same admission with explicit parameter limits and shared exact arithmetic |
| `coordinates(...)`, `identity(...)` | Symbolic convenience constructors; repeated coordinates copy the same value |
| `pullback(B, control)` | Source worlds x satisfying `B(f(x))` |
| `image(A, control)` | Target worlds y with some legal x satisfying A and `f(x)=y` |
| `universal_image(A, control)` | Target worlds whose every source preimage satisfies A; includes unreachable worlds vacuously |
| `nonvacuous_image(A, control)` | Universal image intersected with the structural range |
| `support_image(control)` | Image of full source support |
| `certify_surjective(control)` | A separate owned `SurjectiveMap` certificate, only if the structural range is full |
| `SurjectiveMap::descend(E, control)` | Unique target Event when membership is constant on each legal readout fibre; otherwise role refusal |
| `then(next, control)` | Composition through an explicitly checked common middle context |
| `equivalent(other, control)` | Supported equality of readouts after checking both endpoint contexts |
| `source()`, `target()`, `readouts()`, `map_world(world)` | Safe inspection of owned descriptors and evaluation of a legal source code |

Pullback preserves every Boolean operation. Direct image preserves unions, but
generally loses intersection and complement: different source witnesses can
produce the same target. The exact laws governing filtering are

```text
image_f(A) ⊆ B                 iff A ⊆ pullback_f(B)
pullback_f(B) ⊆ A              iff B ⊆ universal_image_f(A)
image_f(A ∩ pullback_f(B))       = image_f(A) ∩ B
universal_image_f(A)            = !image_f(!A)
nonvacuous_image_f(A)           = support_image_f ∩ universal_image_f(A)
```

Surjectivity additionally reflects inclusion, equality and all four pair
occupancy cells through pullback. `SurjectiveMap` does **not** certify a complete
projection/substitution square or a law pushforward. The relation layer now checks
the former with a distinct joint-image certificate; the latter remains M6 work.

## Admission, memory and kernels

Every readout aligns to the source, including empty/full readouts. Admission
substitutes them into the **original target support**, then checks that no legal
source world reaches its complement. Checking the target's completed `full`
Event would be vacuous and is explicitly covered by a Lean counterexample.

The map owns source/target spaces and its aligned readouts. Clones share the
descriptor. Empty/full results retain their destination owner, just like other
Events. Independently owned equivalent inputs are aligned explicitly inside
the map operation; unrelated identities or unequal support refuse, even when
the eventual answer would be constant. Composition checks its middle context
before zero-coordinate shortcuts.

Pullback substitutes directly into the canonical graph using ITE. Local tables
expand over their at most nine essential coordinates; symbolic nodes recurse
with a memo scoped to one source and replacement vector. Same-arena substitution
copies local table data before appending nodes, so simultaneous replacements do
not become sequential assignments. No BEVT serialization is used by the kernels.

Image masks the input by original source support and splits on readouts in target
working order. After each split it existentially eliminates source coordinates
unused by all remaining readouts. The memo key contains target position and the
remaining source predicate. This retains a shared source witness while avoiding
exponential prefix enumeration for identity/projection maps. Source and target
each support 62 coordinates; they need not fit a combined 62-coordinate arena.
Adversarial diagrams can still exhaust limits. No general complexity improvement
or performance ranking is claimed.

Cross-arena operations use the same total lock order as `Event::align_to`.
Allocations, memo growth and kernel steps retain explicit refusal and cancellation.
Failed operations may retain canonical intermediates but publish no result.
Arena lifetime/reclamation and retained-byte limitations are unchanged from the
[native implementation ledger](event-implementation.md).

[BEDC v1](event-descriptors.md) now transports finite map descriptors and
reconstructs their certificates at import. Their **result Events**
already use ordinary BEVT identity, storage, dependency admission and Free Join
equality. M5 now retains these descriptors through `EventImport` and `use map`
bindings, with input/output context checking and all six readout query operations.

## Evidence and remaining work

[CoordinateMaps.lean](../crates/bumbledb-event/semantics/CoordinateMaps.lean)
checks support admission, both image adjunctions, Boolean pullback, composition,
surjectivity/occupancy, the simultaneous-substitution reference, recursive image
splitting and safe abstraction after last use. It keeps counterexamples for
unsupported target images, decoder aliases, absent fibres and over-eager
abstraction. These are denotational/reference proofs, not Rust refinement.

The native oracle covers all 256 maps from four codes to four codes, all 15
nonempty source supports, and all 16 predicates: 61,440 map/predicate cases.
Targets alternate between exact image support and a full target, so possibility
reflection cannot be assumed. Additional tests cover 12-bit nonlinear symbolic
readouts, opposite-direction concurrency, capacities/cancellation, same-arena
restrictions, owned results and 62-bit maps with a bounded kernel-step budget.
Database integration persists mapped Events, reopens them, checks Free Join
equality on resident and cursor paths, and retains results after closing owners.

The [map semantic run](event-evidence/map-semantics/check.json) records exact Lean
sources and reports. The [native qualification](event-evidence/native-map-qualification/check.json)
records source hashes and test logs for that earlier map slice. Complete finite
fibre products, role admission, relation/residual operators and graph/readout
conversion now have [their own qualification](event-relations.md). Solver-backed
parameter maps and general bound fixed-point query programs remain acceptance work.
Complete-binding relation query trees now retain checked products and readouts. Safe
[diagram inspection and reconstruction](event-inspection.md) are now implemented.
