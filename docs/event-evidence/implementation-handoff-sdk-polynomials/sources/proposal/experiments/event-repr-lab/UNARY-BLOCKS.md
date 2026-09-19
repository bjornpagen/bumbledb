# Block cofactoring and paired projection

This changes the kernels beneath the [inline-chain normal form](UNARY-CHAINS.md),
keeping its sixteen-byte records, complement rule and fixed bottom-up packing.
The new constructor accumulates a whole word, pushes complement through its
letters, removes constant prefixes and publishes only the needed packed blocks.
Cofactoring walks the stored label directly. Projection keeps the pair
`(Exists hidden F, Forall hidden F)` so an antitone letter can exchange and
negate the two answers correctly. A complemented root swaps/negates this pair;
the pair cache can therefore use the regular root.

Ordinary binary Apply and substitution remain one-bit operations. This is not
a whole-query block kernel yet. Paired projection is enabled on plain Shannon
as well, providing a control for the cost of constructing both bounds separately
from the benefit of packed labels.

## The nonempty premise is explicit

The general internal summary of a hidden fibre is two bits:
`(can be false, can be true)`. All four states matter, including `(false,false)`
for an empty fibre. A Boolean transform acts on this set by ordinary image:

```text
out0 = (in0 AND NOT t0) OR (in1 AND NOT t1)
out1 = (in0 AND t0) OR (in1 AND t1)
```

This action preserves empty fibres and composes exactly. The optimized bound
rules assume `in0 OR in1 = true`. The raw Boolean cube supplies that premise:
every assignment of visible raw coordinates has a full hidden cube. It does
not assert that every supported observation fibre is inhabited, that every
legal world has positive probability, or that source laws are independent.
Supported Event operators still require their ordinary gates and completion.

[UnaryProjection.lean](lean/UnaryProjection.lean) adds eleven reports. It proves
the image calculation for arbitrary represented fibres, including infinite or
empty ones; composition, negation and domain preservation; the exact nonempty
conditions for the fast bound steps; an empty-fibre counterexample; and the
complement-normalization equation. Seven reports are axiom-free, four use
`propext`. Packed Rust encoding and arena refinement are not formally verified.

## Verification and matched results

The [first kernel check](results/unary-block-check.json) passes 25,165,824 binary
comparisons, 49,152 projections, 178,176 simultaneous maps and 49,152 completed
projections across both kernels and both layouts. The
[deeper check](results/unary-block-deep-check.json) preserves these and adds
128 noncontiguous four-axis masks across chains up to 61 coordinates. It checks
16,384 visible assignments by explicitly enumerating all sixteen hidden
assignments for each, testing both existential and universal answers. It also
checks hiding all coordinates, complement duality, and operations after copying
only live graph records.

[All 48 structural processes](results/unary-block-shapes.json) pass. Every
one-bit control reproduces the previous screen. Inputs, requested output graph
census, legal checksums and post-collection census match across kernels. The
following rows use compacted inputs and chain records:

| Fixture / order | Step records → block records | Step retained bytes → block bytes | Step projection misses → pair-cache misses |
| --- | ---: | ---: | ---: |
| Relation / bit | 164 → 163 | 15,552 → 14,152 | 63 → 34 |
| Relation / face | 395 → 402 | 28,160 → 26,760 | 98 → 47 |
| Bilinear / bit | 750 → 882 | 61,872 → 61,872 | 410 → 350 |
| Bilinear / face | 12,210 → 13,743 | 986,352 → 986,352 | 6,520 → 6,198 |
| Coup / bit | 93,302 → 93,302 | 12,304,576 → 12,304,576 | 0 → 0 |
| Coup / face | 121,066 → 121,066 | 14,713,024 → 14,713,024 | 0 → 0 |

The last column counts different memoized units (one polarity versus a bound
pair); it is diagnostic, not a runtime ratio. The plain-layout control shows
that paired projection itself causes most extra bilinear nodes: both layouts
reach 882 / 13,743 records under the block kernel. Coup exercises no projection
in this fixture and its Boolean Apply remains unchanged. Fewer projection
visits therefore do not establish lower whole-query work.

No timings, native Free Join speedups or peak-memory reductions are claimed.
Retained byte estimates include the pair cache and exclude temporary pending
words, count memos, allocator overhead and oracle data. The result does not
justify selecting this paired kernel as the default.

## What the paper cross-check changes next

Bryant's **Chain Reduction for Binary and Zero-Suppressed Decision Diagrams**,
[arXiv:1710.06500v1](https://arxiv.org/abs/1710.06500v1), §3, compresses chains
whose repeated exit may be an arbitrary subfunction. That is broader than a
unary letter's constant exit. Its CBDD Apply in §6, equations (9)–(10), operates
on shared ranges rather than individual coordinates. Section 7 changes the
cofactor/combination algorithms and counting along with storage. Section 8.1
explicitly qualifies operation counts by cache and garbage-collection policy.
Section 9 handles complement edges, with a different chosen regular edge from
this probe. None of its benchmark speedups transfer to our implementation.

The [retained reading record](results/unary-block-reading.json) pins both this
paper and the λDD source. The next concrete candidate is a common-exit selector
block with grouped Apply, described in [the range plan](RANGE-PLAN.md). It must
compete against a one-bit kernel on the same normal form, then satisfy complete
native carrier admission before Free Join timing can select it.

The [first block snapshot](results/unary-block-src/manifest.json) and
[deeper-check snapshot](results/unary-block-deep-src/manifest.json) are separate.
Library and main fixture code are identical between them; only test coverage
and its checker changed.
