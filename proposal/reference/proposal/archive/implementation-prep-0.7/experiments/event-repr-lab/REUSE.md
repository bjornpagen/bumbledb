# Normalize residual functions, or reuse their aligned words

The [previous factorial](VIEW-ORDER.md) found two different costs of deferring
canonical intermediates. Local products repeatedly cofactor and broadcast tables.
Their `(root,pins,values)` descriptors can also distinguish residual functions
that canonical normalization would identify, or retain axes that no longer
matter. This experiment tests those mechanisms independently under the same
Event semantics and native Free Join output contracts.

## Two independent controls

`EVENT_LAB_VIEW_REUSE=off|bounded` controls an invocation-local plane cache.
`EVENT_LAB_VIEW_NORMALIZE=deferred|source` controls whether pending source pins
are reduced through the existing canonical cofactor operation before continuing.
Both are captured once per raw arena. The earlier `materialized`, `views` and
`views-inputs` product strategies remain separate, as do assignment/word readers.
No public Event field, scope rule or equality key changes.

Source normalization turns the pinned predicate into its exact cofactor in the
source coordinate order, then clears the pins. The existing constructor removes
inessential coordinates and interns equal residual functions. The resulting
root still has the operand's original checked map. Product memo lookup occurs
after normalization, so equivalent cofactors can now share a subproblem.
These canonical intermediates and their cofactor caches stay resident and must
be charged. Normalization is not a probability update or a new source law.

The discarded pins are already encoded in the cofactored function. Its value
agrees with the operand on the current branch; parent selectors or existential
unions retain that branch's role in the final answer. The global elimination set
and each operand's map remain fixed. Dropping pins without first absorbing their
meaning into the predicate would be unsound.

## Bounded plane reuse and its lifetime

There are two direct-mapped caches, each lazily allocating 256 slots, one per
operand. A slot retains the full `(root,pins,values,target_axes)` key plus either
aligned words or a stable source-table ID. Root polarity remains part of the
key. Each cache's map, source arena, order and support context are fixed by its
one enclosing product invocation. Separate operand caches prevent preparing the
second plane from evicting the first before their joint read.

Every hit compares the full key. A collision replaces an entry; it cannot merge
functions or change an answer. No cache entry escapes the invocation. Owned
aligned words use a boxed slice. A borrowed table retains its numeric node ID,
not an arena or slab address, and borrows the payload anew during the immutable
read phase. Canonical output insertion occurs after both read borrows end.
Existing nodes are immutable, so growing the arena does not invalidate an ID.

For cutoff K, logical cache storage is bounded by

```text
2 * 256 * (sizeof(Option<PlaneEntry>) + 8 * ceil(2^K / 64))
```

The cache arrays and held word payloads have exact logical lengths. Allocator
overhead, source maps, recursive frames, product memo and incoming alignment
workspace are outside this bound. The diagnostic records the maximum held cache
bytes per product; it does not measure whole-query peak memory. Bounded-cache
and uncached pairs must retain identical resident nodes, arena bytes and ordinary
operation caches. Cached planes do not become resident Event identities.

## What the work counters establish

The [untimed replay](results/reuse-work.json) checks 192 cases: both stores, both
normalization modes, cache on/off, both cutoffs, two widths and all three layouts.
It uses the 24 unique input-pair indices from the clover fixture, fresh input
clones and exact canonical materialized-output checks. It omits Free Join,
grouped union and cross-product arena reuse; it is not a timing result or a
complete relation-program profile.

For K=9 at 18 coordinates, bit-major fused output gives:

| Source handling / plane cache | Subproblems | Local cubes | Local cofactors | Broadcasts | Cache hits / misses | Canonical cofactor calls |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Deferred / off | 12,360 | 6,144 | 42,240 | 42,240 | 0 / 0 | 0 |
| Deferred / bounded | 12,360 | 6,144 | 8,256 | 8,256 | 9,984 / 2,304 | 0 |
| Source / off | 1,917 | 486 | 0 | 3,171 | 0 / 0 | 2,946 |
| Source / bounded | 1,917 | 486 | 0 | 2,001 | 339 / 564 | 2,946 |

Cache reuse alone preserves traversal and reduces repeated alignment. Source
normalization changes the traversal itself. Its canonical cofactor calls can
perform recursive work or hit the existing arena cache; they are not equivalent
cost units to local word cofactors. It removes 5,685 coordinate occurrences from
conservative masks in this replay. Those are occurrences across calls, not 5,685
distinct coordinates. Native timings and retained storage must decide whether
the additional normalization is worthwhile.

## Proofs, checks and research

[Lean](LEAN.md) now checks 54 central reports. The new cofactor theorem proves
that every surviving essential coordinate was essential before restriction and
is different from the pinned coordinate. A second theorem proves the strict
counterexample: `if x then y else z` uses all three coordinates, but fixing
x=true leaves exactly y. Four further reports separate raw essential masks from
[support-relative dependency views](SUPPORTED-DEPENDENCE.md). Subtracting pins from an old dependence mask is a sound
upper bound, not necessarily the least dependency set. These are denotational
proofs; arena canonicality and Rust refinement still have their own obligations.

The raw suite passes twelve test functions for both stores and normalization
modes. It retains the exhaustive small map/product oracle, 62-coordinate scattered
cases, complements and exact canonical identity checks. New tests force cache
collisions with one slot, change pins/values/target axes, grow the source arena
under a saved table ID, and compare cached versus uncached construction exactly.
The source/deferred controls also agree with an independent finite pointwise
oracle. The shared suites pass under both stores with source normalization and
bounded reuse, including composition, residuals, closure, ownership and symbolic
sixty-coordinate relations.

An initial cache test incorrectly added a broadcast axis that overlapped a pinned
mapped coordinate, violating the plane reader's contract. The assertion caught
it. [The failure](results/reuse-sanity-check.json) and its exact source snapshot
are retained; the test now chooses an axis outside both live and pinned sets.
The production kernel's guard was not weakened.

The variable-shift SDD paper, [arXiv:2004.02502v1](https://arxiv.org/abs/2004.02502),
Theorem 6, the identical-vtree rule and Section 7, shows how permitted common
substitutions support sharing and a narrower Apply cache. It does not justify
omitting arbitrary maps or contexts. Its mandatory-compression caveat also
prevents importing a blanket polynomial-time claim. The reduced-BDD rules in
[arXiv:1206.5266v1](https://arxiv.org/abs/1206.5266) explain the separate benefit of
removing equal-child tests and merging equal residual functions. The
[reading record](results/reuse-reading.json) retains the local source hashes.

## Native result: normalize when it buys useful sharing

The [same-executable measurements](REUSE-MEASUREMENTS.md) retain the full
factorial controls: two stores, two essential-table cutoffs, three coordinate
orders, two query lanes and outer memo on/off. The initial sweep passes all 246
processes, with 912 query cases and 1,920 matched comparisons. A separately
shuffled nine-sample repeat retains both packed512 and dense controls.

For the **80-output relation program**, essential512 slab, 18 coordinates,
face-major order and outer memo enabled, the nine-sample repeat gives:

| Strategy | Source handling | Plane cache | Fresh ms | Min–max ms | Resident KB | Nodes |
| --- | --- | --- | ---: | --- | ---: | ---: |
| Materialized | — | — | 6.931 | 6.861–7.306 | 668.1 | 5,273 |
| Fused views | Deferred | Off | 50.445 | 48.884–51.218 | 1,108.2 | 8,088 |
| Fused views | Deferred | Bounded | 29.197 | 28.487–29.548 | 1,108.2 | 8,088 |
| Fused views | Canonical | Off | 8.467 | 8.238–8.765 | 1,573.4 | 11,800 |
| Output-late views | Deferred | Off | 22.443 | 22.093–22.932 | 194.9 | 1,642 |
| Output-late views | Canonical | Off | 5.934 | 5.855–6.306 | 525.2 | 3,752 |
| Output-late views | Canonical | Bounded | 5.272 | 5.115–5.466 | 525.2 | 3,752 |
| Packed512 materialized | — | — | 2.310 | 2.255–2.407 | 1,213.6 | 7,882 |
| Dense materialized | — | — | 2.441 | 2.255–2.552 | 5,823.7 | 175 |

This separates three effects. Cache reuse improves repeated local word work
without changing any retained nodes. Source normalization can recover much more
sharing in the traversal, while adding canonical nodes and cofactor caches.
Output-late execution avoids the especially large intermediate graph created by
fusing the output swap in this layout. None of those effects changes the Event
answer or the dependency contract.

The bit-major full program tells a different part of the story. Output-late,
uncached views improve from **7.574 to 2.001 ms** with source normalization,
retaining **214.0 KB** rather than 143.8 KB. The materialized control takes
2.342 ms and 301.6 KB. Packed512 remains faster at 0.738 ms and uses 512.4 KB.
The repeat's cache-on normalized path takes 2.032 ms; this small difference
cannot justify an always-on cache. At twelve coordinates the same normalized
output-late full-program path takes 0.257 ms without the cache and 0.297 ms with
it. Reuse has bookkeeping and allocation costs even with exact bounded storage.

The separate enum-storage repeat reproduces the direction: bit-major
materialization takes 2.531 ms versus 2.041 ms for normalized output-late views
without plane reuse, using 463.5 versus 315.0 KB. Face-major materialization
takes 7.015 ms versus 5.379 ms with normalization, output-late execution and
bounded reuse, using 1,056.6 versus 751.2 KB. The enum uncached bit-major sample
range reaches 21.969 ms despite a 2.041 ms median; retain that scheduling noise
in any interpretation of small differences.

Keep the **16-output product lane** separate. On its face-major query, the
normalized output-late path takes 2.363 ms with the cache, versus 2.240 ms for
materialization. A win in the complete relation program is not a win in every
constituent workload. Warm outer-memo replay is close to the join floor and does
not measure the cost of a new contraction.

Timings include real Free Join, canonical construction and exact counting;
setup, input cloning and independent bitset validation are outside. Resident
estimates include the ordinary operation caches, but exclude temporary plane
caches and other transient workspace. The diagnostic's held-cache bound is not
a whole-query peak estimate. All samples remain in the raw records. A
[desktop-load snapshot](results/reuse-environment.json) records other active
applications during the initial sweep; serial lab execution is not an isolated
machine-wide benchmark. Small differences need further evidence.

## Ownership, symbolic answers and retained artifacts

All **445 native processes** pass: the 81-process smoke, 246-process initial
sweep, 53-process slab repeat, 9-process enum repeat and 56-process acceptance
run. The final comparison retains 912 distinct timed query configurations and
1,920 matched comparisons, with no missing baselines. It checks cache pairs for
identical retained nodes and bytes, and keeps later failed runs from silently
reusing earlier timings.

[Native acceptance](results/reuse-acceptance.json) adds 192 owned-result cases
and 48 symbolic programs across both cutoffs, both stores, both output schedules
and cache settings with source normalization. Owned cases compute and publish
80 results, validate exact packets and retain their output owners after inputs
are dropped. Symbolic programs include the sixty-coordinate presentation of
million-state counter closure and four other outputs. No explicit world
materialization is needed for that case. These checks preserve publication and
lifetime behavior, not just final checksums.

The [comparison record](results/reuse-comparison.json),
[build metadata](results/build-reuse.json), [build log](results/build-reuse.log),
source snapshot and copied experiment tools retain the exact executable and
controls. The original failed cache-test fixture and its actual failure log
remain linked separately; neither that failure nor the earlier v2 readout error
is relabelled as a passing run.

## What this settles for the representation

Retain canonical cofactoring as a reusable operation available inside mapped
contraction. A pending-pin descriptor is a useful execution view, but its
conservative mask and syntactic identity are not substitutes for normalizing
residual functions when sharing matters. Keep plane reuse and output working
order independently selectable. The public Event still has its stable canonical
identity and the same algebra; neither the cache nor an unfinished mapped view
becomes a resident value.

The [retained ARM64](results/assembly-reuse.json) covers eight emitted symbols,
including cache preparation/readout and source normalization. The cache checks
all key fields; normalization calls the existing canonical cofactor. This is
primarily a change in how much graph and alignment work is done. Vector
instructions elsewhere in the readout do not establish a batched SIMD product
kernel or explain the measured gain by themselves.

These measured controls used full binary-product support. The subsequent
[scoped-product extension](SCOPED-PRODUCT.md) generalizes the lower-level kernel
by retaining exact support gates; its timing is separate. Products of legal
domains with checked encodings and reconstruction remain a stronger relation-
program capability. This comparison selects no universal backend and does not
certify probability-law transport.
