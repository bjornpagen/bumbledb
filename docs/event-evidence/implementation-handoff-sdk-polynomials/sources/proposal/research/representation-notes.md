# Representation review: evidence and qualifications

This page records the earlier support-pair review and its isolated probes.
The [revision 0.8 representation](../representation.md) selects a completed-function
backend after the native laboratory comparisons. Retained candidate-specific
claims below do not define the current layout or persistence contract.

The supplied [brief](representation-brief.md) asks us to derive control flow from
precise data and invariants. This revision uses it as a design criterion, not as
proof that representation removes essential inference complexity. Normalized
partitions eliminate malformed complement states; Boolean canonicalization and
exact source feasibility still require real computation.

## Inspected native source

| Source | Finding used by the design |
| --- | --- |
| [theory/allen.rs](../../crates/bumbledb-theory/src/allen.rs) | Finite mask coordinates and bit-permutation converse |
| [engine/allen.rs](../../crates/bumbledb/src/allen.rs) | Total scalar classifier grounded in ordered endpoints |
| [kernel/allen.rs](../../crates/bumbledb/src/exec/kernel/allen.rs) | Extent-checked batched classification/filtering and constant-side path |
| [kernel/neon.rs](../../crates/bumbledb/src/exec/kernel/neon.rs) | Six-bit signatures, resident lookup tables, explicit final-window tails |
| [assembly gates](../../scripts/check-asm.sh) | Named-symbol instruction contracts, not blanket performance assertions |
| [schema/judge.rs](../../crates/bumbledb/src/schema/judge.rs) | Current pointwise keys/containments depend on endpoint sweeps and coverage runs; events need different domain operations |
| [image.rs](../../crates/bumbledb/src/image.rs) | Multiword values become parallel columns; interval WordPair has temporal meaning |
| [canonical.rs](../../crates/bumbledb/src/canonical.rs) | Logical canonical bytes and owned decoded values are real engine contracts |
| [ir.rs](../../crates/bumbledb/src/ir.rs) | Pure-data output terms and topological nonrecursive stages |
| [computed.rs](../../crates/bumbledb/src/api/prepared/computed.rs) | Complete bound-output evaluation; current interval emptiness drops a binding |
| [derived.rs](../../crates/bumbledb/src/api/prepared/derived.rs) | Resident/scratch stages retain generation and text owners; event owners require parallel support |
| [bindings.rs](../../crates/bumbledb/src/exec/run/bindings.rs) | Slot-indexed word bindings; a two-word EventKey alone does not supply ownership |

## Primary implementation references read for this revision

[CUDD internals](https://www.cs.rice.edu/~lm30/RSynth/CUDD/cudd/doc/node4.html),
sections on complement arcs, cache, reference management, and reordering:
complement uses a low reference bit; the then/high edge is regular; callers
normalize complemented children before unique-node insertion. Caches do not
own live nodes and must be invalidated with collection/reordering. The proposed
owner/slab policy is our engine-specific design, not CUDD's memory manager.
[Retained text](https://www.cs.rice.edu/~lm30/RSynth/CUDD/cudd/doc/node4.html).

[CUDD Boolean implementation](https://github.com/ivmai/cudd/blob/master/cudd/cuddBddIte.c):
inspected complemented-cofactor recursion, unique-node insertion, and memoization.
Its witness-intersection routine is not a replacement for full event intersection.
The proposal's private support-pair layer is a derived design on top of canonical
Boolean functions, not a quoted CUDD data type. [Retained source](https://raw.githubusercontent.com/ivmai/cudd/master/cudd/cuddBddIte.c).

[ARM ACLE Advanced SIMD reference](https://arm-software.github.io/acle/neon_intrinsics/advsimd.html):
inspected `vbicq_u64`, `vbslq_u64`, logical operations, and `vqtbl1q_u8` mappings.
The installed compiler emitted the corresponding instructions in the isolated
probe. [Retained text](https://arm-software.github.io/acle/neon_intrinsics/advsimd.html). Fetch URLs and original-byte
hashes are in [sources.json](representation/sources.json).

The earlier Green–Karvounarakis–Tannen event-table construction and Kimmig et al.
algebraic model counting remain the probability/query references. They do not
supply the proposed support-pair wire or ownership contract. The bibliography's
SDD/knowledge-compilation papers motivate comparison candidates; this revision
does not assert it benchmarked those implementations.

## Derived and checked here

The support-masked complementary pair gives exact relative event identity under
a fixed canonical Boolean order. Four-bit truth tables cover all binary Boolean
operations. Four-cell occupancy records exact Venn predicates and transforms
under operand symmetries. Finite logical guards give a finite event quotient of
the selected semialgebraic world presentation, provided the complete feasibility
and event-constancy obligations are discharged. Revision 0.5 uses explicit
admissibility; this round's positive-support coin fixture is retained as an
explicit Supported(law) view, not the default universe.

[Representation checks](representation/checks.json) independently compare small
bitset denotations to the BDD/pair reference, including reconstruction with another
allocation order. [Probe results](representation/probes.json) record installed
compiler versions, layout sizes, semantic tests, and selected assembly properties.
Neither checker proves arbitrary source feasibility, parallel memory safety,
full query correctness, or universally compact diagrams.

## Remaining empirical choices

Coordinate ordering, Apply cache layout, dense cofactor cutoff, batching, source
law contraction order, and eventual compaction require the workload matrix in
[the implementation plan](../implementation-plan.md). Canonical ROBDD growth can
be exponential. The representation is a selected baseline with explicit ways to
falsify its practical suitability, not a claim that every uncertainty workload
has become a few SIMD instructions.
