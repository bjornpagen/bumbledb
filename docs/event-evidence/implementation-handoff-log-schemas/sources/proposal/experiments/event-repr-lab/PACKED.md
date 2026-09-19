# Packed terminals inside an anchored canonical diagram

`packed64`, `packed256`, `packed512` and `packed4096` retain exact events, with a
fixed ordered binary prefix and truth tables over the last six, eight, nine or
twelve coordinates. A leaf has one, four, eight or sixty-four `u64` words.
Smaller spaces use only the active bits. This is a local implementation
of ordered Shannon decomposition with packed terminals, not a claim to a new
historical diagram family or an implementation of general SDDs.

## Resident data

The external sink still passes the same sixteen-byte `(scope, region)` key.
The manager has separate append-only branch and terminal arenas:

```rust
#[repr(C, align(16))]
struct Branch {
    level: u32,
    low: u32,
    high: u32,
}
// 16-byte branch records; terminal payload is [u64; W], W in {1, 4, 8, 64}.
// Hash tables and caches occupy additional, measured/estimated memory.
```

A raw 32-bit reference uses bit 0 for complemented edges, bit 1 to distinguish
branches from tables, and higher bits for an arena index. References 0/1 are
constant false/true. The public region handle has a separate support-relative
polarity bit, using the [anchored invariant](ANCHOR.md). Flipping the public bit
complements within declared support, which need not be the whole bit domain.

The leaf boundary, coordinate order, active terminal masks, support and admissible
anchor are fixed for the manager. A new order is a new representation context;
it cannot silently reuse old handles.

## Why identity is exact

For a fixed order and boundary, each function has a unique truth table at every
prefix assignment. A table is interned after clearing inactive bits and choosing
the complementary orientation whose last active assignment is false. Complete
array equality resolves hash collisions.

Inductively, two canonical child references determine a canonical function at a
prefix level. Equal children eliminate that test. Otherwise orient both children
so the high reference is regular and intern the complete `(level, low, high)` key.
This is the ordinary ordered Shannon uniqueness argument with canonical tables
as its base case. A table depending on fewer tail variables still has a unique
full active table; it is not given an alternative reduced shape.

This proves unique structure within the stated decomposition, assuming correct
interning and constructors. Arena allocation IDs and hash iteration order are
not portable canonical bytes. It also does not prove a size advantage over an
ordinary BDD for all functions or all coordinate orders.

For support-relative identity, fix admissible anchor ω and store the side of each
complementary pair that excludes ω. Its raw representative vanishes outside
support. The public bit says which side is selected. The anchored proof applies
unchanged because the raw packed diagrams are canonical Boolean functions.

## Operators and their costs

Boolean Apply descends through the earliest prefix test and evaluates leaf pairs
with word operations. Transforming the four-bit operator for the public input
polarities makes it zero-preserving before raw Apply. The result therefore still
vanishes outside support and at the anchor. Ordinary Boolean publication needs
no extra support mask or complementary-pair record.

Existential abstraction first recovers the actual support-masked event. Prefix
variables are eliminated by OR; terminal variables use exact dense axis reduction.
The result is sealed again. Quantification can change the anchor's membership,
and a lifted projection can include inadmissible encodings, so skipping this
normalization would be wrong on constrained support.

Fused relational product combines conjunction and abstraction in the same
recursion. At leaves it intersects truth tables and immediately reduces the
selected axes. Its intermediate operation need not allocate an entire global
world bitmap. Masks refer to semantic coordinate numbers, not prefix levels or
positions inside a packed word.

Count traverses canonical subgraphs with memoization, accounting for skipped
prefix coordinates and table population. A complemented public event has count
`|support| - |representative|`. This is exact cardinality, not contraction against
an arbitrary joint probability law.

Coordinate permutation is genuine substitution: old coordinate `i` is replaced
by the requested destination coordinate. Branches rebuild through canonical
Boolean selection because moving a variable can violate the original order.
A table crossing the leaf boundary is decomposed into its small Shannon function
and rebuilt through the same raw operations. It never expands the whole symbolic
domain, but it can lose sharing and pay substantial rebuilding cost. That cost is
included in the relation lane. The [table-preserving permutation](TAIL-MAPS.md)
now checks whether the tail's coordinate set is invariant under the map. If it
is, the table uses exact broadword axis swaps and returns directly through the
leaf interner; prefix changes still rebuild canonically. Otherwise it retains
the general recursive path. Both algorithms remain selectable in the same binary
for matched timing and semantic verification.

The local permutation object validates bijectivity and coordinate bounds. The
low-level result is the substituted event intersected with current support.
Commutation with relative complement and inverse-map laws require a
support-preserving map; the relation lane establishes this with full Cartesian
support. This is **not** the proposed cross-space Face/CoordinateMap registry,
its environment checks, or general guard substitution.

## Bounds and machine-code expectations

The current prototype uses fewer than 63 binary coordinates, `u64` elimination
masks, `u64` exact counts and a combined two-million-record cap. The previous
32-bit-mask mismatch on forty-coordinate symbolic tests is corrected. Arbitrary
coordinate sets and arbitrary-precision counts are still required for an
unbounded production contract.

Leaf operations dispatch the Boolean operator outside the word operation.
One-word leaves use scalar word math. The inspected four-word Apply contains
paired 128-bit NEON `ORR`, `AND`, `BIC` and `EOR` operations. Branch recursion, hash-table
lookups, interning and count traversal remain part of query time. The [actual packed Apply](results/packed256-apply-relations-arm64.s)
and [recursive table permutation](results/packed256-permute-table-relations-arm64.s)
are retained separately. The latter rebuilds through canonical selection; the
whole graph operation is not one vectorized pass.

Fixed-size terminals trade less graph traversal for larger payloads and possible
extra work under renaming. Bigger leaves are not automatically better. These are
separate candidates, each with one canonical decomposition; the experiment does
not mix representations behind equal-looking handles.

Source and binary snapshots for the initial packed sweep are retained in
[packed-src](results/packed-src/) and [build-packed.json](results/build-packed.json).
The relation-lane extension is retained in [relations-src](results/relations-src/)
and [build-relations.json](results/build-relations.json), with its verification
record in [relations-v2-verification.json](results/relations-v2-verification.json). See the [report](REPORT.md) for measured outcomes and current limits.
