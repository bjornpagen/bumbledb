# Shared-exit selectors: grouped operations help when the pattern exists

The [planned selector](RANGE-PLAN.md) is now implemented in a separate
[Rust crate](range-prototype/src/lib.rs). All 64 structural processes pass.
Grouped Apply reduces work strongly on the deliberate shared-exit family,
modestly on one Coup order, and negligibly or not at all on the others.
This earns an exact competing primitive; it does not select a default carrier.

## What is stored

```rust
struct Node {
    variables: u64, // exact dependence; one high bit tags a group
    edges: [u32; 2], // signed low/high references
}
```

Each sixteen-byte record denotes `if any(S) then high else low`. Both children
are arbitrary functions. `S` is recovered from the parent dependency mask minus
the union of the children's masks. Every selected coordinate precedes both
children in one fixed order; the selected coordinates need not be contiguous.
There is no external selector buffer. The high kind bit is redundant to the
current mask decoder and reserved for dispatch; it is included in the census.

Normalize the low reference, eliminate equal children, and maximally merge a
low child's selector when its high continuation equals the parent's high
continuation. A one-coordinate selector has the ordinary Shannon tag. This
fixed policy is checked against canonical imports, including imports performed
after copying only reachable physical records. It is not a machine-checked
canonical-arena proof.

## What the kernel does

At the first dependent coordinate, grouped Apply chooses a common initial
selector segment. If only one operand begins there, it stops before the other
operand's first dependency. If both begin there, it stops at their first
selector disagreement. An absent operand has identical cofactors. Otherwise,
the high cofactor is the shared continuation and the low cofactor is the
remaining selector. Recurse on the two cofactor pairs, then publish one block.

This is the equation, for any binary Boolean function `op`:

```text
op(select(S,L,H), select(S,A,B)) = select(S, op(L,A), op(H,B))
```

The exact shared selector is essential. Merely related probabilities, separate
source judgments, or separate possible witnesses do not justify this rewrite.
The public Event algebra, legal-world completion and support gates are unchanged.

Cofactoring an interior selected bit false removes it directly; true chooses
the high continuation. Existential projection computes the requested result
alone. After projecting child coordinates, if any selector coordinate was
hidden, combine the low child with the high child using OR, then retain only
the visible selector coordinates. Complemented roots retain signed projection
keys. Substitution still uses one-bit reconstruction, preserving simultaneous
maps even when selectors overlap or identify coordinates.

Counting reads the compressed record directly. With `k` selected coordinates
and child counts aligned to the union of child coordinates, its count is
`low + (2^k - 1) * high`. The selector cases are disjoint even when the child
Events overlap. This counts the full raw cube. It is not a legal-world count
for a decoder, nor an independence assumption about a designated law.

## Correctness and structural evidence

The [raw check](results/range-initial-check.json) passes 25,165,824 binary
comparisons, 49,152 projections, 178,176 simultaneous maps and 49,152 completed
projections across all six three-bit orders, both layouts and both kernels.
Long checks cover 19,200 sampled worlds through 61 coordinates. Another 128
four-axis masks check 16,384 visible assignments against all sixteen hidden
assignments each. The eight-axis sparse/shared-exit suite checks 115,200 binary
combinations, 61,440 projections and 3,840 cofactors against independent tables,
including complements and operations after collection.

The [64-process screen](results/range-initial-shapes.json) crosses four fixtures,
two orders, plain/range records, retained/collected inputs and step/group kernels.
Every requested output is checked over the full finite raw cube, then checked
again after collection. Exact counts are checked against enumerated populations.
The [auditor](range_audit.py) verifies 32 matched kernel pairs, twelve unchanged
plain controls from the original unary screen, frozen hashes, and log markers.

The following rows use range records and collected inputs:

| Fixture / order | Final records, step → group | Retained bytes, step → group | Apply misses, step → group | Projection misses, step → group |
| --- | ---: | ---: | ---: | ---: |
| Relation / bit | 164 → 164 | 15,552 → 15,552 | 122 → 122 | 63 → 60 |
| Relation / face | 395 → 394 | 28,160 → 28,160 | 324 → 320 | 98 → 86 |
| Bilinear / bit | 750 → 750 | 61,872 → 61,872 | 586 → 586 | 410 → 410 |
| Bilinear / face | 12,210 → 12,210 | 986,352 → 986,352 | 12,310 → 12,310 | 6,520 → 6,520 |
| Coup / bit | 93,302 → 93,297 | 12,304,576 → 12,304,576 | 309,518 → 309,247 | 0 → 0 |
| Coup / face | 121,066 → 118,379 | 14,713,024 → 14,713,024 | 429,412 → 422,284 | 0 → 0 |
| Shared exit / either | 127 → 41 | 9,596 → 3,467 | 108 → 32 | 16 → 2 |

The strength fixture has sixteen leading selector coordinates and arbitrary
overlapping four-coordinate children. It intentionally supplies the pattern
the representation should exploit. Coup's bit order executes 1,393 multi-axis
splits spanning 3,069 axes, yet saves just 271 Apply states; its face order
executes 6,885 such splits spanning 15,581 axes and saves 7,128 states (1.66%).
Cache and residual-block behavior make grouped-axis counts different from saved
work. Capacity rounding leaves Coup's estimated retained bytes unchanged.

Collected graphs are identical across kernels. Against ordinary Shannon,
range collection changes Coup from 11,431 to 11,405 records in bit order and
15,080 to 14,699 in face order. Conversely, the 61-coordinate mixed-unary control
still takes 62 range records: the earlier unary-label representation needs ten.
These are competing decompositions with distinct strengths.

No runtime ratio, native Free Join speedup or peak-memory improvement is claimed.
Estimated bytes include live arena capacity, unique/apply/cofactor/projection
caches and order metadata; they exclude transient import/oracle data, count
memos, hash-table control overhead beyond the estimate and allocator overhead.
This prototype has a 62-coordinate and two-million-record limit.

## Proof, paper and next gate

[SharedExit.lean](lean/SharedExit.lean) contributes ten reports: shared-selector
Apply, selector splitting, complement and normalization, sparse-list splitting,
hidden and visible selector projection, essential-selector witnesses, arbitrary
width raw-cube counting, and a different-selector counterexample. Seven reports
are axiom-free; three use `propext` and/or `Quot.sound`. The central suite has
195 reports, 126 axiom-free. The proofs do not verify the Rust segment-selection
algorithm, arena canonicality or machine code.

Bryant's [arXiv:1710.06500v1](https://arxiv.org/abs/1710.06500v1), §§3–4 and §6,
equations (9)–(10), supplies arbitrary shared exits and range-based Apply.
Our sparse masks and low-regular complement convention require the additional
checks above; its contiguous-range size bounds and reported speedups are not
asserted for this implementation. The [reading record](results/unary-block-reading.json)
pins the inspected primary source. Sources, checker and executable are frozen
in [the initial snapshot](results/range-initial-src/manifest.json).

The [native follow-up](RANGE-NATIVE.md) implements a scoped adapter, including symbolic packet transport,
support-relative identity, exact source spectra retaining shared parameter
groups, ownership checks and symbolic relation acceptance. Its shared admission
checks pass, as do the [native comparison](RANGE-NATIVE-MEASUREMENTS.md) and
acceptance runs. Grouped fresh Coup queries do not beat the matched step
control in that sweep. The representation is not promoted.
The raw result does not justify replacing the completed-function/essential-table
direction or combining representation tags without a canonicality argument.
