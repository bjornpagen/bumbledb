# Canonical selector partitions in machine-word blocks

`block64` groups the ordered coordinates into blocks of at most six bits.
A node partitions that block's at-most-64 assignments into selector sets with
shared continuation functions. Selectors fit in one `u64`; the last block is a
packed Boolean truth table. Larger packed-terminal candidates (`packed512` and
`packed4096`) separately test the effect of increasing a binary-prefix leaf.

This design preserves the scoped Event denotation, explicit admissibility and
[anchored complement](ANCHOR.md). It does not attach weights to selectors or rows.
The [paper review](../../research/block-decomposition.md) distinguishes this
construction from general SDDs, decomposable AND/OR diagrams and Reach algorithms.

## Memory and uniqueness

The resident Event key remains sixteen bytes. Raw references distinguish branch
and leaf arenas and carry a complemented-edge bit; the public key has a separate
support-relative polarity. Branch descriptors and selector records are:

```rust
#[repr(C)]
struct Branch { level: u32, start: u32, len: u32, next: u32 } // 16 bytes
#[repr(C)]
struct Edge { selector: u64, child: u32 }                    // 16 bytes
```

`next` links a hash-collision bucket. Full level/edge equality, not a hash alone,
settles interning. Selectors partition every assignment of that block. They
include the false continuation when needed; omitted assignments never acquire
an implicit alternative meaning.

For each fixed ordered block decomposition, every assignment has one canonical
continuation by induction on the remaining blocks. Group equal continuations,
union their selectors, and sort by the resident child reference. Normalize the
complement orientation so the highest local assignment selects a regular child.
If all assignments share one continuation, eliminate the node. Leaf tables have
one normalized orientation and inactive bits cleared. These rules give one
resident structure per Boolean function under that decomposition.

Child-index sorting establishes uniqueness inside one manager. It is not a wire
format: indices depend on allocation order. Deterministic serialization must
rebuild from semantic coordinates and a deterministic selector traversal, with
translation and normalization costs charged separately.

An anchored public representative additionally vanishes outside support and at
the fixed admissible anchor. All sixteen Boolean functions transform into a
zero-preserving operation plus public polarity, so raw Apply preserves that
invariant. Quantification and substitution recover a true root and reseal it.

## Operations follow the partition

Apply intersects the two nodes' selectors. Only nonempty intersections recurse
on their continuation pair. These intersections form another disjoint partition;
merging equal output continuations reconstructs the canonical node. There are at
most 64 nonempty intersections, though the current implementation can test up to
64×64 selector pairs. This distinction matters for performance.

Quantification first reduces continuations. Hiding an entire block unions them.
Hiding part of a block existentially expands the affected selector bits, then
unions continuations where expanded selectors overlap. A fused relational product
performs conjunction and this elimination without first publishing an intermediate
intersection node. At the final block, these are word operations.

Count multiplies each continuation count by its selector population and accounts
for skipped blocks. This is exact cardinality on the declared finite presentation.
It is not a nonuniform or correlated-law measurement.

Permutations use two paths:

- If every coordinate block maps to itself, permute selector bits and recursively
  translate children. Decision levels stay fixed; canonical rebuilding never
  needs to move a test above another block.
- Otherwise, translate selectors into predicates at their destination coordinates
  and rebuild through canonical Boolean selection. Whole-block moves can reuse a
  word selector at another level. Maps splitting a block use its small Shannon
  decomposition. General reordering costs are included.

The local permutation validates a bijection. Its operation is substitution
followed by intersection with current support. Inverse-map and complement laws
need support preservation; the relational benchmark supplies full Cartesian
support. Production cross-space/environment checks remain a separate requirement.

## The layouts being compared

For the 18-bit relation fixture, each of X, Y and Z has six coordinates:

```text
face-major: Z5 Z4 Z3 Z2 Z1 Z0 | Y5 Y4 Y3 Y2 Y1 Y0 | X5 X4 X3 X2 X1 X0
bit-major:  Z5 Y5 X5 Z4 Y4 X4 | Z3 Y3 X3 Z2 Y2 X2 | Z1 Y1 X1 Z0 Y0 X0
pair-major: Z5 Z4 Y5 Y4 X5 X4 | Z3 Z2 Y3 Y2 X3 X2 | Z1 Z0 Y1 Y0 X1 X0
```

For block64, vertical bars are the block boundaries. All three describe the same
worlds and relations. The two interleaved layouts preserve blocks under any
permutation of whole X/Y/Z faces; face-major requires moving levels.

The binary, four-way and packed candidates receive the same flat orders.
Four-way nodes now admit nonadjacent grouped bits, including checked padding for
odd coordinate widths. Finite containers retain the semantic integer-world
bitplane; an internal diagram-order choice does not permute their worlds.

The [layout-planning derivation](LAYOUT-PLANS.md) identifies the coordinate-map
orbits behind that property. Grouping complete orbits predicts when a local
permutation kernel is valid; it does not predict whether abstraction over the
same blocks is fast.

The first partial block handles widths not divisible by six. Published group
layout remains immutable. Automatic group selection, variable block widths,
dense versus selector-based branch payloads and dynamic reordering are future
experiments, not hidden behavior in this candidate.

## Validation and limits

The shared semantic suite runs independently in a small debug checker and again
inside the optimized native engine binary. It includes the original exhaustive
support-relative Boolean checks, all candidates under two extra coordinate
orders, three full relation layouts, partial-block projections, face/converse
maps, residuals, closure, and forty-coordinate symbolic operations. Native timing
still uses real Free Join; the debug checker supplies no performance numbers.

The prototype uses fewer than 63 coordinates and `u64` counts. Its resource cap
charges branches, selector edges and leaves together at two million records.
The `nodes` diagnostic counts branches and leaves; edge storage is included in
bytes. Different carriers' node counts are therefore insufficient memory rankings.
There is no garbage collector or exact allocation profiler.

The [retained ARM64 Apply](results/block64-apply-tail-arm64.s) and
[product](results/block64-product-tail-arm64.s) also expose a kernel confound:
temporary `[Edge; 64]` arrays have a padded twelve-byte field payload in a
sixteen-byte record, and zero initialization currently emits long sequences of
separate scalar stores. A future matched experiment can give that padding an
explicit zero field while preserving record size and hashing of semantic fields,
or use a rigorously bounded initialized-prefix buffer. Neither improvement is
implemented in the measured candidate. Selector-partition losses must not be
interpreted as though every such temporary-buffer cost were intrinsic.

Selector records can cost more than a dense child array when many assignments
have distinct continuations. Larger packed leaves can similarly reduce traversal
while making projection or permutation more expensive. The measured comparisons,
including construction, caches and output observation, decide which tradeoffs
survive. No general fastest-representation theorem follows from one layout sweep.
