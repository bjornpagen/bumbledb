# Inline unary chains: compact records need matching operations

This competitor preserves Shannon cofactors while compressing unary structure.
The [inspected λDD paper](../../research/difference-decomposition.md) supplies
canalizing/XOR reductions and a canonicality result for its own model. It also
explicitly warns that node savings can disappear into edge-label storage. This
probe charges the complete labels in a concrete Rust record; the paper's Coq
proof has not been imported, and does not verify our bounded blocks or arena.

## Sixteen bytes, including the label

```rust
#[repr(C)]
struct Node {
    variables: u64, // exact raw dependence; high bit distinguishes a chain
    edges: [u32; 2], // binary children, or regular tail + packed letters
}
```

The ordinary case holds two signed child references. The chain case holds one
regular tail reference and up to ten three-bit letters in the other word:

| Code | Function of the new bit x and remaining function g |
| --- | --- |
| 0 | x AND g |
| 1 | x AND NOT g |
| 2 | NOT x AND g |
| 3 | x OR g |
| 4 | x XOR g |

The exact mask difference between the node and its tail identifies the chain's
coordinates in the immutable order. No coordinate list, external label buffer
or hidden suffix reference is excluded from this layout. The current prototype
admits at most 62 coordinates and has the common two-million-record cap.

Ordinary cofactor equality removes an unused variable. Complement normalization
makes the low child regular; XOR recognition precedes constant-child recognition
to resolve the literal/terminal ambiguity. Prepending to a short chain merges
its label. A full ten-letter child starts a new block, so segmentation is fixed
from the bottom. The tail remains regular and output complement is one bit.
These are fixed owner-internal representation rules, not new public Event syntax.

The first kernel obtains ordinary cofactors by reifying shortened blocks as
needed. That remains valid after unreachable suffix records are removed. The
uniform raw counter reads labels directly and never creates those suffixes.
It uses each label's affine action on population and cube size; it does not
assume that an arbitrary Event's designated law is independent or uniform.

## Checks

The [initial check](results/unary-initial-check.json) and subsequent
[long-chain census check](results/unary-long-check.json) pass 12,582,912 binary
results across every three-bit function, six orders and both layouts, plus
24,576 projections, 89,088 simultaneous maps, and 24,576 completed projections
on coupled support. Results compare exact root identity with independently
constructed truth tables. Each three-bit root is also compacted alone before
its cofactors are rebuilt and checked again.

Long mixed chains exercise both sides of the ten-letter boundaries, a genuinely
binary tail, opposite orders, complement, cofactoring every coordinate,
projection and exact reconstruction into the compacted manager. Their 9,600
sampled world checks reach 61 coordinates; smaller cases also enumerate every
world for uniform-count verification. This is finite Rust evidence, not a proof
of arbitrary arena canonicality.

The 61-coordinate positive control makes the storage benefit concrete: both
layouts construct 123 records, while the collected single result uses 62 plain
records versus 10 chain records (4,108 versus 1,282 estimated bytes). Both
coordinate directions agree. This is a storage/control case, not a timed query.

[UnaryChains.lean](lean/UnaryChains.lean) checks nine denotational reports:
cofactor reification, the associative monoid of four unary Boolean maps,
compilation of an arbitrary label word into that monoid, concatenation, and the
uniform counting law. Six reports are axiom-free; the others use only standard
`propext`/`Quot.sound`. These laws support future block kernels. They do not
prove the packed bit encoding, interner, collector or general source-law solver.

## Matched structural screen

[Twenty-four processes](results/unary-initial-shapes.json) all pass. They cross
plain/chain layouts, two coordinate orders, three fixtures, and keeping versus
compacting inputs before the query. The fixtures exactly reuse the relation,
Coup clover replay and twenty-bit bilinear definitions from the
[difference experiment](DIFFERENTIAL.md). All outputs are checked on every raw
assignment. The live input/output union is then copied into a fresh manager and
every restored root and its raw uniform population is checked again.

This is copied-live-graph compaction, not an in-place concurrent collector or a
wire codec. Bytes include arena/interner/operation-cache capacities, excluding
allocator overhead and temporary/oracle storage. No latency or native Free Join
result is claimed.

| Fixture / order | Final retained records, both layouts | Collected plain records | Collected chain records | Collected plain bytes | Collected chain bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Relation / bit | 164 | 61 | 44 | 3,520 | 2,344 |
| Relation / face | 395 | 120 | 98 | 6,896 | 4,544 |
| Coup / bit | 93,302 | 11,431 | 11,074 | 563,392 | 563,392 |
| Coup / face | 121,066 | 15,080 | 13,649 | 864,448 | 563,392 |
| Bilinear / bit | 750 | 750 | 750 | 35,440 | 35,440 |
| Bilinear / face | 12,210 | 12,210 | 12,208 | 563,440 | 563,440 |

Counts include the terminal record. Both input policies reach the same final
counts for a given fixture/layout. Every matched plain/chain case also has the
same Apply misses and projection misses. Reifying cofactors reconstructs the
intermediate functions that compression had bypassed.

The face-ordered Coup case saves about 9.5% of collected records. Its roughly
35% byte reduction is primarily an interner capacity threshold, not a 35%
reduction in logical records. For bilinear face order, the smaller output-only
graph (9,840 versus 10,036 records) barely changes the live union: the retained
inputs already keep most of those records alive. Root lifetime matters.

## What this changes

The label representation is correct on these checks and can reduce collected
storage. It has not reduced the one-bit query kernel's retained work. Merely
changing records therefore does not justify promoting it over the existing
native table/diagram carriers.

The next discriminating experiment is a kernel that consumes label blocks
without publishing every intermediate suffix function. The four-map monoid
gives a closed local composition operation; quantifying through negating labels
must retain both possible and guaranteed tail values rather than push an
existential quantifier through negation. That is a concrete derived direction,
not an implemented speedup. Any new kernel must compete against this same
normal form and preserve post-collection operation closure.

The [initial sources and executable](results/unary-initial-src/manifest.json)
are frozen. The long-chain census only adds reporting to the tests and has its
own source snapshot. Production sources and the previous native executable
remain unchanged.

The [block-kernel follow-up](UNARY-BLOCKS.md) passes all 48 structural processes
and deeper multi-coordinate checks. Paired projection sometimes saves work but
can retain more nodes; Coup's one-bit Apply remains unchanged. The next
[shared-exit candidate](RANGE-PLAN.md) targets grouped Boolean operations.
