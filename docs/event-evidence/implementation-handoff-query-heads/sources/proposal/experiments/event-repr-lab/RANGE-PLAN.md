# Next candidate: a shared-exit selector block

The raw candidate is now implemented and checked; see [results and boundaries](RANGES.md).
This document retains the implementation plan. It follows the
[unary-block result](UNARY-BLOCKS.md) and Bryant's inspected CBDD algorithms.

## Meaning and sixteen-byte candidate

For a nonempty ordered coordinate set S, represent

```text
F(w) = if OR_{v in S} w[v] then high(w) else low(w).
```

Both children depend only on coordinates after every coordinate of S. They
can overlap and need not be constants or complements. The existing record
`(exact dependency mask, two signed u32 references)` can encode this: a kind
bit marks a selector block, and
`S = node_dependencies - (low_dependencies union high_dependencies)`.
No external label buffer is needed. A one-coordinate block is an ordinary
Shannon node. Complement flips both children before normalizing the low edge.
If low equals high, the selector disappears. If low's block has the same high
child, their selectors merge and the lower low child becomes the new low.

Bryant's paper uses contiguous level ranges. Allowing a sparse coordinate mask
is our proposed extension and needs its own correctness tests/proof; it must
not silently apply the paper's level arithmetic to a gapped set. Keep every
selector axis before both children. A single deterministic reduction policy
must define the normal form; do not mix node-adaptive heuristics into identity.

## Grouped Apply

At the earliest variable in either operand, choose a nonempty common initial
selector segment. If both operands begin there, stop before the first differing
selector coordinate. If only one begins there, stop before the other's first
dependent coordinate. This makes both functions depend on that segment only
through its OR value. A constant/absent operand has two identical cofactors.

The high cofactor is the shared high child; the low cofactor is either the low
child or a residual selector block. Recurse on the two cofactor pairs and
reconstruct one selector block. Compare this with an explicitly one-bit Apply
kernel using exactly the same resident representation and constructor.

Fixing a selected bit true returns the high child. Fixing it false removes that
bit. Fixing a child coordinate recursively transforms both children and rebuilds
the selector. Removing an interior selected bit creates a sparse selector and
must remain canonical.

For existential projection, recursively obtain L and H from the children. If
any selector bit is hidden, the low branch on the remaining selector becomes
`L OR H`, while the high branch is H. With no remaining selector, return the
low result. With no hidden selector bit, retain the selector over L and H.
Signed-root memo keys preserve negation semantics. This directly computes one
requested bound; it need not pay the unary experiment's unconditional pair cost.

## Observation, interchange and evidence gates

Uniform counting is a disjoint case split. With k selector bits and child
counts aligned to the union of child coordinates, use
`count(low) + (2^k - 1) * count(high)`. This is a full raw-cube identity, not a
correlated-law shortcut. General source-law contraction remains an admission
requirement. Existing conditional-iid source spectra may exploit the selector's
generating polynomial, but the derivation must preserve shared parameter groups.

First verify all small truth functions, all sixteen Boolean operations, sparse
selector cofactors, arbitrary simultaneous maps, coupled-support completion,
exact count, complements, post-collection operations and symbolic width. Include
a deliberate shared-exit strength family, not only Coup and parity. Freeze a
source/binary-matched structural screen with both kernels and control layouts.

A promising raw result then needs a complete `RegionOps`/`Carrier` adapter:
support-relative identity/complement, legal owner construction, exact transport,
common packet import/export, designated source spectra, symbolic relation and
ownership acceptance. Inspect the existing essential/anchored wrappers for the
contract, and preserve the frozen native build. Only after those checks should
the native Free Join build and serial timing comparison select a direction.
