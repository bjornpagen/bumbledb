# Fuse constrained products by carrying their support

The previous mapped kernel was admitted only on the full binary cube. That
condition is sufficient, but the lower-level operation can support arbitrary
admissible regions without replacing them by a Cartesian product. The missing
piece is to preserve every support restriction in the staged expression.

## An exact identity, with no independence assumption

Write S for admitted support, TA and TB for the two Events' selected raw regions,
and `rename_m` for a raw coordinate permutation. TA and TB already lie inside S.
The existing scoped operations rename a region and clip it to S; they also clip
an existential result before returning it. Their full staged expression is:

```text
pa = S & rename_a(TA)
pb = S & rename_b(TB)
q  = S & exists_hidden(pa & pb)
result = S & rename_out(q)
```

Two support gates express the same result while borrowing the input maps:

```text
witness_gate = rename_inverse(a)(S)
output_gate  = rename_out(S)

effective_A = TA & witness_gate
raw_result  = mapped_product(effective_A, a, TB, b, hidden, out)
result      = seal(output_gate & raw_result)
```

`seal` clips to S and performs the existing anchored normalization. The first
gate absorbs the common S restriction into one operand before its map. Because
a is a bijection, mapping that gate forward gives exactly S at the joint
witness. The second gate retains the S restriction that originally occurred
**after projection and before output renaming**. Clipping only the final result
to S cannot in general replace that intermediate restriction.

When S is full, both gates are full and the formula reduces to the existing raw
product. If maps preserve S, some gates are redundant; such a certificate may
justify removing work in a subsequent control. The current version computes
the exact gates without assuming map preservation. It does not infer a law or
multiply marginal probabilities.

## Why both gates matter

Take two Boolean coordinates and let S contain only `(true,false)`, with both
input Events equal to full S. Quantify both coordinates.

- Swap the coordinates of both inputs and leave output names alone. The two
  renamed raw inputs meet outside S. Omitting the witness gate turns an illegal
  witness into a full result; the staged result is empty.
- Leave input names alone and swap output coordinates. Raw existential
  elimination gives the full cube. The staged computation first restores S,
  then its swapped singleton misses S. Omitting the output gate instead returns
  full S.

These counterexamples are [proved in Lean](lean/ScopedProduct.lean), along with
the general identity and its full-support specialization. The main identity
uses only a left inverse for the first read map; the Rust kernel uses checked
coordinate bijections for every map. Existing raw mapped-product proofs still
justify its output reindexing and witness handling. These statements do not
formally verify Rust or its normal form.

## Contraction capability and relation-algebra capability

The new lower-level `view_product` means exactly the staged scoped operations
above. It now accepts constrained support. It does not by itself establish that
clipped renaming preserves full, relative complement, or equality. Those
promises require their own support-map evidence. A map may deliberately discard
worlds and still define a valid scoped contraction.

Likewise, a symmetric coupled support such as x=y=z cannot stand in for the
three-face product used to interpret ordinary binary relation composition.
`relational_core::Algebra` retains its existing full-product admission check.
The new Free Join lane is named **scoped contraction**, and has a direct staged
oracle. Extending the relation-program constructor to products of legal
categorical domains remains a separate, stronger step.

This is consistent with the [supported-dependence result](SUPPORTED-DEPENDENCE.md):
an exact raw mask describes a physical function; the legal worlds may supply
additional logical dependencies. Keeping those worlds in the operator gives
the optimizer something exact to manipulate instead of an independence flag.

## Concrete Rust implementation and accounting

The change is in [the essential adapter](src/essential.rs). The raw kernel and
its local-table representation are unchanged. For constrained support the
adapter checks the three maps and elimination mask, builds the inverse input
map, constructs the witness gate and effective A, and maps S to output names.
It then calls the existing raw kernel and publishes through the existing seal.
The first operand's orientation and true supported region are resolved before
the guard is absorbed. Public complement and canonical Event identity retain
the same anchored contract.

The resulting gates, effective operands and canonical intermediate nodes are
resident work and must be charged. There is no new persistent gate cache: raw
renaming still uses its per-call memo, while the query's optional product memo
can reuse a complete contraction. Any future retained gate plan must be keyed
to its owner, support and complete maps. The current experiment must not be
reported as avoiding all intermediate materialization.

[The ARM64 extraction](results/assembly-scoped.json) retains ten named symbols
from the scoped executable, including both adapters and the shared mapped-word
path. The adapters call the existing raw kernel; vector Boolean operations
remain in the mapped traversal and local-plane helper. Support renaming and
canonical interning remain real work. Static instruction sites do not establish
a query speedup or eliminate the cost of constructing the support gates.

## Checks and native comparison

The shared release suites pass under enum storage with source normalization and
bounded word reuse, and slab storage with deferred pins and reuse off. Each
essential-table carrier passes **40,128** new scoped cases: all nonempty two-bit supports,
all distinct supported operand pairs, every two-bit map and elimination set,
both physical orders, plus larger asymmetric/coupled supports and three-cycle
maps. Pointwise answers, canonical IDs and the original staged operations agree.
Existing checks still reject symmetric coupled support as a full relation
product. Earlier raw, ownership and symbolic tests run in the same suite.

[The independent finite checker](scoped_reference.py) passes **157,668** cases:
19,968 exhaustive distinct two-bit cases and 137,700 selected three-bit cases
over every nonempty support. Non-involutive maps distinguish a permutation from
its inverse. The [raw record](results/scoped-reference.json) retains both missing-
gate counterexamples. That revision's 54 central Lean reports and exact compiler
output are retained in [its check record](results/scoped-lean-check.json).
The [current 65-report revision](results/legal-relations-lean-check.json) also
proves when checked support maps make the gates redundant and establishes the
separate [legal relation-role boundary](LEGAL-RELATIONS.md).

The [new native lane](src/scoped_bench.rs) returns sixteen grouped Event regions
through the actual Free Join executor. It rotates the first input's bits within
each face, cycles the second input's faces, eliminates the shared Y coordinates,
then swaps output Y/Z. Its four supports are the full cube, a product of equal
legal domains with three unused codes each, a copied-face constraint, and an
asymmetric coupled domain. The oracle checks each stage on logical XYZ values;
it does not implement the inverse-gate identity. It validates every fresh and
warm output outside timing. A and B remain ordered in the query memo because
their maps differ.

The optimized native smoke and broad sweep pass all 33 and 186 processes,
respectively. The [comparison](SCOPED-MEASUREMENTS.md) retains 704 distinct query
configurations and 1,024 matched comparisons with no missing baselines. It
compares materialization, fused views and output-late views under one executable,
including source/deferred normalization and packed/dense controls. This lane's
answer contract is distinct from the eighty-output relation program. The
separately shuffled nine-sample repeat passes 43 processes; a targeted full-space
normalization repeat passes another eight. In total these four scoped sweeps
retain **270 passing processes**, 704 distinct configurations and 1,024 matched
comparisons, all on the same executable.

For **legal-product support**, eighteen coordinates, bit-major order and the
outer product memo enabled, the nine-sample repeat gives:

| Carrier/store and strategy | Fresh ms | Min–max ms | Resident KB | Nodes |
| --- | ---: | --- | ---: | ---: |
| Essential512 slab, materialized | 13.909 | 13.495–15.143 | 2,650.5 | 20,047 |
| Essential512 slab, fused/source | 8.177 | 7.847–8.925 | 1,389.2 | 10,459 |
| Essential512 slab, output-late/source | 11.927 | 11.855–12.113 | 1,876.6 | 16,302 |
| Packed512, materialized | 4.287 | 4.181–4.449 | 2,904.7 | 23,658 |
| Dense-dispatched, materialized | 2.640 | 2.613–2.663 | 4,570.2 | 137 |

The fused essential path improves its materialized control by about 1.70× while
retaining about 48% fewer bytes. Packed and dense still finish sooner at higher
resident cost. These include the actual Free Join, contraction, grouping and
count checksum; inputs and pointwise verification are outside query timing.
Resident estimates include retained support gates, maps and outer memos, but
exclude allocator overhead and temporary workspace.

The counterexamples to a universal schedule remain visible. On copied-face
support in face-major order, essential512 slab's fused/source path takes
**17.337 ms and 3,699.3 KB**, versus materialization's **13.785 ms and 2,979.0 KB**.
Output-late/source takes 13.265 ms and 2,581.7 KB; the time ranges overlap with
materialization, while its retained memory is lower. In bit-major order the
same support's fused path takes 2.024 ms and 677.2 KB, versus materialization's
2.686 ms and 646.8 KB. Fewer graph nodes can still mean more retained bytes.

Asymmetric support is particularly sensitive to layout. Fused/source takes
**94.348 ms and 15,601.7 KB** face-major, versus **14.422 ms and 2,524.9 KB**
bit-major. Dense takes 2.583 ms and 4,570.2 KB in the latter layout. This is
evidence for retaining multiple exact representations and schedules, rather
than selecting from the word “symbolic” or the number of possible outputs.

The targeted full-support repeat separates source normalization from output
order. Face-major fused views improve from **28.969 to 6.577 ms** with source
normalization, retaining 902.8 rather than 872.4 KB. Output-late views improve
from **24.907 to 2.496 ms**, retaining 331.3 rather than 301.0 KB. Materialization
in the main repeat takes 2.571 ms and 515.4 KB. The normalization improvement is
substantial; the small materialized/output-late timing difference does not
justify a general speed claim.

Two large initial medians do not reproduce: legal-product bit-major
materialization's 49.277 ms becomes 13.909 ms, and full-support face-major
fused/source's 51.032 ms becomes 6.462 ms in the main repeat and 6.577 ms in the
targeted repeat. All original samples remain retained. The [environment snapshot](results/scoped-environment.json)
shows other desktop CPU activity. These serial experiments are not an isolated
machine benchmark, and their ranges are not confidence intervals.

## Design consequences

The low-level algebra now has an exact constrained-support implementation.
That semantic gain survives whichever physical schedule wins. It does not
select a new default carrier, storage layout, normalization strategy or cache.
The scoped timing sweeps keep plane reuse off; bounded reuse has shared semantic
checks but no new scoped performance claim here.

Support gates are currently recomputed through canonical operations. A future
checked plan can retain them or prove them redundant, and must charge its
construction and retained storage. The [legal-relation proofs](LEGAL-RELATIONS.md)
show why equal-domain face maps qualify for gate removal without admitting
every bit code. The current benchmark deliberately rotates A's encoding bits,
which need not preserve the legal domain; that certificate cannot simply be
assumed for these queries.

The separate [native acceptance](results/scoped-acceptance.json) passes all
33 processes, covering 116 owned-result cases with eighty outputs each, and
27 symbolic programs including million-state closure. The essential carriers
run source normalization with bounded word reuse under both stores and both
view strategies; packed/dense controls are retained. This brings the current
revision to **303 passing native processes** across its five retained sweeps.

This extension's arbitrary-support native checks are finite. These full-space
ownership and symbolic regressions do not establish constrained symbolic
ownership. The subsequent [legal-domain experiment](LEGAL-RELATIONS.md)
implements a native checked constructor, role admission, and symbolic constrained
computation. These later results do not enlarge the claims of this earlier sweep.

## Primary-source grounding

The [reading record](results/scoped-reading.json) retains hashes for the revisited
papers. Abbadini–Guffanti, [arXiv:2607.06386v1](https://arxiv.org/abs/2607.06386v1),
Remark 2.11 and Example 2.12 distinguish general inverse/direct images from the
additional complete-square condition needed for unrestricted base change.
Here the gates are retained explicitly; we do not assume that condition.
Pong, [arXiv:2605.03902v4](https://arxiv.org/abs/2605.03902v4), describes measured
fibre products with a specified conditional-independence law, a different
contract. Darwiche–Marquis's restriction/forgetting definitions in
[arXiv:1106.1819v1](https://arxiv.org/abs/1106.1819v1) distinguish fixing inputs
from existential elimination. The gate-absorption identity is our elementary
derived set identity, checked separately in Lean; none of these papers supplies
a throughput or Rust-refinement theorem for this implementation.
