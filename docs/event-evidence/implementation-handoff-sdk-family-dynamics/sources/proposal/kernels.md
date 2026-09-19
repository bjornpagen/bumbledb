# Region coordinates, relational product, and ARM64 kernels

The representation determines the kernels. The resident Event is a scoped
partition handle; Boolean graphs are canonical data; finite dense cofactors
are compiled bitplanes; probabilities are exact law contractions. Each has a
different useful machine operation. This page separates proposed paths from
[emitted ARM64 evidence](kernels/arm64-probe.s).

**Revision 0.8:** the [initial backend](representation.md) uses completed functions,
symbolic splits and essential-coordinate tables with an initial cutoff of nine
raw coordinates. The [Rust laboratory](experiments/event-repr-lab/REPORT.md)
compares complete carriers through native Free Join. The support-masked BDD and
dense layouts below explain retained controls and instruction probes; their
masking rules do not automatically apply to completed-function tables.

## 1. All binary connectives fit in four bits

`BoolOp4` contains a truth table indexed by `(a << 1) | b`:

| Operation | Code | Output |
| --- | --- | --- |
| False | `0x0` | 0 |
| A minus B | `0x4` | `a & !b` |
| XOR | `0x6` | `a ^ b` |
| AND | `0x8` | `a & b` |
| B | `0xa` | b |
| A implies B | `0xb` | `!a | b` |
| A | `0xc` | a |
| OR | `0xe` | `a | b` |
| True | `0xf` | 1 |

The other codes are ordinary truth functions too. Complement the output with
`op ^ 0xf`. Input complementation and swapping permute table bits. A checked
operation wrapper has no out-of-range modes. `Apply(op,a,b)` uses Shannon
cofactors, memoization, and the unique-node constructor. Terminal cases evaluate
one bit of op. Constants, equal roots, and opposite roots simplify before walking.

This is the same useful pattern as Allen's masks: the selected operation is data.
It does not imply that arbitrary graph Apply runs in a fixed number of instructions.

## 2. A finite coordinate system for event relationships

For A and B in nonempty support S, record which cells contain a world:

```text
bit 0: !A & !B       bit 1: !A & B
bit 2:  A & !B       bit 3:  A & B
```

Signature zero cannot describe a valid nonempty space. The other fifteen
signatures include empty/full event cases naturally. Exact predicates follow:

| Relation | Signature test |
| --- | --- |
| Disjoint | `(sig & 0x8) == 0` |
| A included in B | `(sig & 0x4) == 0` |
| Equal | `(sig & 0x6) == 0` |
| A and B cover full | `(sig & 0x1) == 0` |
| Complementary | `(sig & 0x9) == 0` |

Complementing A rotates the four bits by two. Complementing B exchanges adjacent
bits. Swapping operands exchanges bits 1 and 2. The reference checker verifies
these symmetries exhaustively. For a constant predicate, prepare a sixteen-byte
keep table and filter sixteen signature bytes with one NEON table lookup.
The existing `allen_filter_batch_neon` initializes entries only for Allen's
codes 0 through 12. It cannot be called unchanged with Event signatures:
valid signatures 13 through 15 would be rejected. A shared lookup core must
accept a complete validated sixteen-byte table, while each algebra constructs
its own table and retains its own code validity contract.

Computing the signature itself requires exact emptiness checks for the four
cells, using diagram traversal or already materialized bitplanes. The
[native classifier comparison](experiments/event-repr-lab/CLASSIFY.md) now
measures a direct readout against constructing those four cells through actual
Free Join, including owner validation and cache storage. Direct dense/packed
paths improve fresh queries, while the scalar essential-table reader can lose.
[Retained ARM64](experiments/event-repr-lab/results/assembly-classify.json)
shows a scalar word occupancy loop; it does not establish a batched SIMD filter.
The subsequent [borrowed-word experiment](experiments/event-repr-lab/WORD-CLASSIFIER.md)
also gives essential tables a word reader, with large matched first-use gains
under both stores. Ordered cuts test missing-cell proofs as well as fully
occupied pairs. Its occupancy loop remains scalar ARM64 word code; the shared
cofactor kernel contains NEON operations. The separate
[scratch control](experiments/event-repr-lab/SCRATCH.md) now uses reusable bounded
alignment buffers. Batched filtering remains a separate unimplemented control.
Cached pair signatures can make repeated relation
queries cheap. The `TBL` stage consumes
that work; it does not magically classify arbitrary symbolic events from their
handle bits. Cache keys retain space, pair identity, and orientation. Independence
cannot be decided from the signature: it needs the actual law and masses.

For every `BoolOp4 op`, its result is possible exactly when `(sig & op) != 0`,
and full exactly when `(sig & (op ^ 0xf)) == 0`. Thus the signature is the
smallest exact readout for emptiness of all binary Boolean expressions. Keep
operation codes, occupancy signatures and masks over signatures as distinct
types even where their physical widths match.

The [derived relationship calculus](experiments/event-repr-lab/SIGNATURE-CALCULUS.md)
now supplies a sound fifteen-by-fifteen composition envelope, exact operand
symmetries and a checked associativity law. It also proves why this table is
not strong composition for finite Event scopes: equal endpoint signatures can
hide different capacities to split a cell. For one existential intermediate
Event in a full finite powerset, four cell counts capped at two settle that
extra question. This is composition of predicates on Event values; state
relational product below remains a separate constructive operation.

## 3. Resident handle kernels

After owner/scope validation, relative complement is:

```asm
eor x0, x0, #1
ret
```

This is the exact output of the isolated candidate `event_complement` function.
No graph traversal, allocation, or new draw occurs. Equality compares the scope
and canonical region words. For a homogeneous scope batch, validate/resolve that
scope once and vector-compare region words. Heterogeneous batches must retain
scope checks or partition into validated groups; equal indices from different
spaces cannot be treated as equal events.

NEON `CMEQ` can compare fixed-width lanes; scalar hashing can probe interning and
Apply caches. SIMD fingerprint filtering may reduce full cache-key loads, but
collisions always compare complete keys. The native arena now has bounded
per-operation Apply/ITE/count memoization. SIMD cache filtering and integrated
ARM64 performance qualification remain separate work.

## 4. Dense cofactor execution

A dense block is a truth bitplane over a declared, ordered list of assignments.
Bit k always refers to the same assignment in every operand. The initial
essential-table cutoff is nine raw coordinates: at most 512 truth bits, or four
NEON vectors. The older fixed-block probes use smaller blocks. A deliberately
finite scenario can also use a longer bitmap.

In a support-masked control, unused categorical codes and padding are masked by
support. In a completed-function table, invalid raw codes instead alias legal
worlds through the decoder. Boolean operations preserve that completion without
masking those aliases to zero. Allocation padding still needs bounded access;
quantification, legal counting and measurement retain their separate support rules.

The whole Coup predicate `B & !(C | A)` reduces to:

```asm
ldr q0, [x1]             // B
ldr q1, [x2]             // C
ldr q2, [x3]             // A
orr.16b v1, v2, v1
bic.16b v0, v0, v1
str q0, [x0]
ret
```

This is emitted code for the fixed 128-bit probe, processing 128 assignment
truth values. In its support-masked representation it relies on B already lying
in support. A general BoolOp4 kernel for that representation needs an explicit
mask because functions such as NOR produce true on zero/zero inputs outside
support. The completed-function representation follows the alias rule above.

A generic BoolOp4 bitplane evaluator can expand the four output bits into
all-zero/all-one masks and use three Boolean selects. The emitted probe uses
one `BIT`, two `BSL`, and a support `AND`, plus mask construction and loads/stores.
In a batch implementation, prepare masks once outside the block loop. Frequently
used op codes can select specialized AND/OR/EOR/BIC loops once per program.

Dense views are keyed by scope/order, cofactor prefix, roster, and root. Their
creation and canonical reconstruction are real costs. A local cofactor is valid
only under its fixed prefix; it cannot be interpreted as an unconditional
whole-space event. Results pass through canonical reconstruction before being
used as stored/derived Event values. Cache the view only when reuse justifies it.

The native wrapper must validate extents, alignment requirements, aliasing,
owner retention, and padding once; process full blocks; and handle the remaining
bytes/bits without an out-of-bounds read. Overlapping final-window tails are
appropriate only when duplicate stores are semantically harmless. Do not copy
the Allen tail strategy into reducers where it would double-count mass.

The baseline is 128-bit NEON. An SVE backend would be a separately detected,
vector-length-aware implementation; Apple ARM64 targets must not be assumed to
provide it. Portable scalar/word paths remain the semantic fallback.

## 5. Symbolic traversal and admission

General BDD Apply visits pairs of nodes, branches on their variable order,
looks up memo/unique tables, and constructs output nodes. ARM64 NEON has no
general indexed gather equivalent to a vector of arbitrary graph pointers.
Useful work is batching homogeneous operations, separating hot references from
cold descriptors, reusing constant cofactors, retaining contiguous node slabs,
and using scalar early exits for satisfiability. Prefetch and layout changes
need actual profiles; they are not claimed wins in this proposal.

For pointwise keys, test overlap while building a union; for containments, build
coverage and test the remaining gap. Both use the same core Boolean primitives.
A Venn-signature cache is optional reuse, not an admission prerequisite. Keep
witness provenance for diagnostics. Incremental deletions require recomputing
union structure or support counts, not blindly subtracting the deleted event.

## 6. Measurement kernels

The Boolean path can prune zero-support terms before exact arithmetic. A flat
uniform finite scenario can use population count; ARM64 NEON `CNT` plus widening
reductions is a candidate. Nonuniform weights require weighted contraction.
Do not replace it by popcount divided by the number of worlds.

Exact rational/polynomial arithmetic may use limb addition/multiplication,
coefficient grouping, and repeated contraction shapes. Scalar `MUL`/`UMULH` and
carry chains are plausible building blocks. General exact real-algebraic or
semialgebraic reasoning is not a SIMD dot product. Floating evaluation, if
introduced for previews or certified enclosures, needs its own explicit contract
and cannot decide equality, emptiness, admission, or exact normalization.

The same arithmetic law expression can be shared by an event and its complement;
conditioning uses the evidence denominator rather than silently normalizing
intermediate regions. [Source-law.md](source-law.md) specifies the contraction.

## 7. What was actually run

[arm64-probe.c](kernels/arm64-probe.c) was compiled with the installed Apple Clang
at `-O3` on ARM64. [Saved assembly](kernels/arm64-probe.s) contains the four leaf
functions. [Probe results](research/representation/probes.json) record toolchain
and checks. The semantic harness checks 32,000 general Boolean blocks (4,096,000
truth bits), 2,000 steal blocks, complement involution, and table lookup.
[layout.rs](kernels/layout.rs) checks the proposed Rust sizes/alignment.

These are fixed-block instruction/semantic probes, not a measured query engine,
a BDD performance benchmark, or evidence that the current Rust event macro works.
The existing [Allen kernels](../crates/bumbledb/src/exec/kernel/neon.rs) and
[assembly gates](../scripts/check-asm.sh) are the inspected engineering precedent.
The [implementation gates](implementation-plan.md) describe the next
measurements and the full-path costs that must accompany any throughput claim.

## 8. The larger algebra has a shared relational-product kernel

The relational core lifts `R(theta,s,t)` and `Q(theta,t,u)` into one checked
presentation, intersects their regions, and existentially eliminates t. It
rebuilds the result in the outer `(theta,s,u)` presentation. With completed
functions, the plan supplies the required support gates and decoder repair;
certificates may remove only the gates or repairs they justify. Shared theta is
never freshly drawn or independently existentially resolved on each side.

The [legal-domain implementation](experiments/event-repr-lab/LEGAL-RELATIONS.md)
now checks the workspace and each input's face dependency independently.
The general scoped kernel retains witness and output support gates; a separate
kernel removes them only after exact support-preservation checks on the required
maps. Its [compiled ARM64](experiments/event-repr-lab/results/assembly-admission.json)
retains scalar admission wrappers and NEON operations inside the local word
reader. This is a native laboratory path over Free Join, not production Event
storage. Measurements compare complete strategies; a cached-renaming control
is still needed to separate caching gains from gate-elision gains.

The initial schedule uses staged support-aware projection. A fused recursive
product is an alternative that avoids materializing the entire intermediate AND;
the experiments do not establish fusion as a universal improvement:

```text
RelProd(f,g,eliminate):
    use validated terminal/equality shortcuts and a plan-scoped memo
    v = first remaining variable tested by f or g
    lo = RelProd(cofactor(f,v,0), cofactor(g,v,0), eliminate)
    hi = RelProd(cofactor(f,v,1), cofactor(g,v,1), eliminate)
    if v is eliminated: return Apply(OR,lo,hi)
    else:               return mk(v,lo,hi)
```

This pseudocode shows the Boolean recursion after preparing compatible inputs
and required witness gates; it is not the full support/repair program. Terminal
handling, complemented references and canonical result construction use the
selected core. The memo retains the plan/quantified set and
scope, as well as root identities. A skipped existential variable has no numeric
multiplicity; existential truth is idempotent. Weighted elimination is a different
program. Guard variables remain selected logical parameters unless a separately
validated logical projection explicitly eliminates them.

Lift/rename can require ITE reconstruction if the target coordinate order differs.
Simply rewriting variable indices can violate the ROBDD ordering invariant.
Input/output face swaps may be cheap views until materialization, but converse
is not promised to be a one-instruction graph permutation. Common guard
refinement happens before the kernel; it is not repaired by bitwise AND.

Domain and May reuse existential projection. Universal projection, All and
residuals use the corresponding complement-and-existential program, with the
correct source/target/product universe masks. Must adds an enabled-domain AND.
Star and monotone fixed points repeat these kernels and compare canonical roots
for stabilization. A fixed finite quotient guarantees termination, not a small
number of graph nodes or cheap intermediate results.

Dense blocks can compile bounded cofactors into a Boolean matrix/tensor view.
An existential coordinate becomes OR-reduction over its assignment axis;
universal abstraction can use masked complement/OR/complement. Coordinate layout
decides whether shifts/masks, pairwise word operations, or NEON shuffle/select
instructions are useful. ITE maps directly to select; bit-sliced cardinality
can use XOR/AND carry networks. These are proposed kernels, **not additional
assembly probes already run**. Reconstruct and intern results before publishing
Event values, and include compilation/reconstruction in benchmarks.

## 9. Porous does not mean unowned or untyped

Prepared code can inspect a borrowed node/face view, select dense tiles, supply
an elimination order, and build validated operation programs. The view retains
its owner, order, support, fixed cofactor prefix, and source-coordinate meanings.
External compilers can return a candidate circuit, which is rebuilt through the
same canonical constructors. No plugin can inject unchecked region IDs or declare
an approximate result structurally equal.

Free Join plans and region plans remain inspectable separate objects. A fused
native path must preserve every participating operand's validation and the
deterministic fault-set contract, including after value saturation. Unmatched
operands remain unevaluated. This architecture permits optimization across stages without
pretending that a row join enumerates hidden worlds or that a graph kernel
already inherits Free Join's join-complexity guarantees.
