# Bounded scratch for the same exact word readout

This control replaces temporary alignment vectors in the
[word classifier](WORD-CLASSIFIER.md). It does not change the resident normal
form, public Event identity, recursive subproblem key, result histogram, source
law, or enum/slab storage. Select `EVENT_LAB_OCCUPANCY_KERNEL=scratch`; `words`
and `scalar` remain distinct same-binary controls.

## What is bounded, and what is still allocated

[occupancy_scratch.rs](src/occupancy_scratch.rs) retains three plane buffers and
one work buffer. The workspace is constructed once per classification and passed
through sequential recursive visits. Every local contraction finishes before
the next visit reuses it. It occupies 32 bytes for K=6, 256 bytes for K=9, and
2,048 bytes for the K=12 raw test carrier. These are workspace object sizes,
not bounds on the compiler's complete stack frame or the recursion stack.

Matching table words remain borrowed from the immutable resident arena. A table
with pinned or missing axes alternates between its plane buffer and the work
buffer; if the final data is in the work buffer, it is copied back before the
next plane is aligned. Cofactoring clears its logical output slice before ORing
bits into it. Broadcast overwrites every logical output word. Support receives
the valid-bit mask during contraction. Polarity is applied on read.

The capacity is chosen from the carrier's cutoff, not just the smaller remaining
cube: a pinned input table can contain more coordinates before its first
cofactor. No buffer address or borrowed slice becomes a canonical Event or a
memo key. The memo retains `(root, pinned, values)` triples and exact signatures.
Only signature 15 licenses early saturation.

The workspace contains no heap-backed fields, and neither alignment function
allocates. **The classifier still allocates its traversal hash memo.** Optional
outer pair caching is another separately charged structure. This experiment is
not an allocation-free query engine. The old allocating word kernels remain
unchanged and continue to serve constructive algorithms and the `words` control.

## Correctness gates

The [optimized raw check](results/scratch-raw-check.json) passes ten test
functions. The full shared suite passes in both
[enum](results/scratch-enum-check.json) and [slab](results/scratch-slab-check.json)
storage, including ownership, dependencies, relation laws, transport and
shared-parameter observation. New checks cover:

- Scalar, allocated-word and scratch signature equality over every small
  supported predicate pair, every signature class, and complement/swap laws.
- Identical recursive subproblems, local cubes, memo entries, word batches and
  borrowed/aligned/constant plane counts. Each old alignment allocation matches
  one scratch transformation; scratch reports zero alignment allocations.
- Every cofactor and insertion position through twelve axes, both cofactor
  values, and multiple input patterns, against independent pointwise indexing
  and the existing allocated kernels.
- Dirty buffer reuse across sizes, masks, pins and polarities on twelve
  coordinates scattered across a 62-coordinate ambient presentation.
- Unchanged resident nodes/bytes and compile-time-sized workspace layouts.

Counters are implementation diagnostics, not an allocator census. The tests
show that the alignment code does not use `Vec`; they do not make assertions
about all allocations performed by the surrounding runtime.

Those standalone records belong to the preserved initial scratch revision.
The expanded native sweeps rerun the shared checks for the final executable
under both stores and both word kernels before measuring the new fixture.

## Native comparison contract

`scratch_sweep.py` runs allocated words, scratch, materialized-cell controls and
packed512/dispatched-dense controls serially, in recorded shuffled order. Both
Coup presentations, ordered cuts and the coordinate-axis fixture return the same complete
histograms and every group's checked survivor count. Ordered cuts prohibit
signature 15, preserving empty-cell proof work.

The initial [three-fixture smoke](results/scratch-native-smoke.json) passes 22
processes but shows little timing difference. It remains on its own preserved
source revision; its timings are not merged with the expanded comparison.

`axes_65536` deliberately exercises alignment: sixteen literals, sixteen
two-axis conjunctions and sixteen two-axis disjunctions, followed by their
complements and empty/full. Every proper predicate has one or two essential
axes. A pair has at most four, so both essential carriers enter one local cube.
Proper operands with different masks cannot take equality/complement shortcuts;
they need exactly the missing-axis broadcasts. The native oracle also requires
some signature-15 pairs. In this family such pairs must have different masks.

[An independent finite-cube checker](axes_reference.py) checks all 9,604 pairs
and the controlled row topology. Of 4,096 clover bindings, 3,888 need alignment:
11,860 broadcasts without the outer pair cache, or 1,168 on the first visit to
384 normalized pairs with that cache. Triangle uses 940 of 1,024 bindings and
665 normalized pairs. [The retained counts](results/axes-reference.json) are
derived from the fixture and algorithm, not an allocator profile. The native
histograms, survivor counts and essential pair-cache sizes must match them.

The timed query includes scope locking, input validation, actual Free Join,
classification, filtering and accumulation. Construction, cloning, publication,
native setup, checking and release remain outside. Compare fresh and warm with
pair memo on and off; do not use a warm-cache floor to choose an alignment
kernel. Resident storage and pair-cache sizes must match between the two direct
modes. Temporary work is charged in time even when absent from retained bytes.

```sh
python3 proposal/experiments/event-repr-lab/classify_check.py --essential-layout slab --occupancy-kernel scratch --output my-scratch-check.json
python3 proposal/experiments/event-repr-lab/run.py --build-only
python3 proposal/experiments/event-repr-lab/scratch_sweep.py --output my-scratch.json
python3 proposal/experiments/event-repr-lab/scratch_comparison.py my-scratch.json --output MY-SCRATCH.md --record my-scratch-comparison.json
```

The comparator rejects mixed executables, mismatched histograms/survivors,
resident growth in a direct path, and differing arena/cache sizes in a matched
pair. It keeps the latest supplied process per exact case, never the fastest.
Compile and inspect assembly outside timing sweeps.

## Alignment-heavy results

The [final same-binary comparison](SCRATCH-MEASUREMENTS.md) retains 130 passing
serial processes, 768 distinct query cases and 192 matched word/scratch pairs.
It uses the latest supplied process per exact case; it does not choose the
fastest process. The historical three-fixture smoke has a different executable
and is excluded. [The comparison record](results/scratch-comparison.json)
names every included run and retains the checker hashes.

The independent eleven-sample [axes repeat](results/scratch-axes-repeat.json)
confirms a first-use gain with the same complete histogram. Clover inclusion
retains 188 of 4,096 bindings. Times below are median milliseconds:

| Carrier / store | Pair memo off, words → scratch | Pair memo on, words → scratch |
| --- | ---: | ---: |
| essential64 / enum | 0.678 → 0.457 | 0.174 → 0.158 |
| essential64 / slab | 0.685 → 0.492 | 0.175 → 0.168 |
| essential512 / enum | 0.667 → 0.467 | 0.178 → 0.164 |
| essential512 / slab | 0.688 → 0.494 | 0.172 → 0.157 |

Without the outer pair cache, the word/scratch ratios are about 1.39–1.48.
With it, only first visits perform alignment and the gain is smaller. Warm
pair-cache replay is around 0.11 ms for both kernels and does not measure this
choice. Both direct paths retain the same 5,152-byte enum arena or 2,836-byte
slab arena; neither adds canonical nodes. These are resident estimates, not
process memory or peak temporary allocation measurements.

The same repeat's packed512 direct control takes 0.519 ms without the pair
cache and 0.163 ms with it. Its materialized-cell control takes 0.368/0.284 ms,
while adding 240,176 estimated resident bytes. Dispatched-dense direct takes
1.047/0.232 ms and begins with a 420,532-byte arena. Scratch improves the
essential direct path on this deliberately alignment-heavy family; it does
not dominate every execution strategy or erase their different memory costs.

Full raw ranges remain in the records. There are scheduling outliers, including
one 1.119 ms sample in the essential64/slab uncached scratch case. These are
serial shuffled-process comparisons on a shared development machine, not
machine-wide controlled confidence intervals.

## The older fixtures do not support a universal switch

Fresh clover inclusion with pair memo enabled gives the following medians.
Compact Coup and ordered cuts use the eleven-sample direct repeat; card-coordinate
Coup uses the fifteen-sample confirmation. Every number is from this executable.

| Presentation | Enum words → scratch, ms | Slab words → scratch, ms |
| --- | ---: | ---: |
| Compact Coup | 0.306 → 0.287 | 0.292 → 0.314 |
| Card-coordinate Coup | 1.492 → 1.477 | 1.499 → 1.487 |
| Ordered cuts | 0.577 → 0.536 | 0.576 → 0.557 |

These differences are small or mixed across the initial run and independent
repeat. The large scalar-to-word gain belongs to the earlier experiment;
removing alignment allocations does not reproduce it. In the final card-coordinate
confirmation, uncached words/scratch medians are 16.305/16.451 ms for enum and
16.121/16.444 ms for slab. The buffers do not remove the recursive traversal or
its memo work.

One earlier card-coordinate enum/scratch process has a 4.373 ms median and a
1.692–51.965 ms range. It is retained in
[the direct repeat](results/scratch-direct-repeat.json). The independently
shuffled [confirmation](results/scratch-card-confirmation.json) gives 1.477 ms
with a 1.456–1.540 ms range, against 1.492 ms allocated words. The severe slowdown
did not reproduce; do not attribute that single process to the kernel. The
ordered dense control also retains a visible scheduling spike.

Same-repeat direct controls still lead both Coup presentations: dense/packed512
take 0.126/0.199 ms on compact Coup and 0.308/0.634 ms on card-coordinate Coup.
The older five-sample ordered materialized-cell control remains in the full
tables with its own sample count. It is not relabelled as an eleven-sample
control.

## Actual ARM64 and stack cost

[Fifteen extracted symbols](results/assembly-scratch.json) match this executable
and source snapshot. Both allocated and scratch recursive visits contract table
words with scalar loads, `EOR`, `BIC`/`BICS`, `AND`, condition tests and the
signature-15 exit. Neither contains a NEON Boolean occupancy loop. The scratch
cofactor helper does contain NEON Boolean instructions; broadcast uses scalar
spreading and copies. `CNT`/`ADDV` sites count coordinate bits, and vector register
copies/initialization do not establish batched classification.

The scratch workspace is created in the classification dispatcher and passed
through recursive calls. Observed dispatcher frames are 256 bytes at K=6 and
480 at K=9. Each scratch visit reserves 720 bytes including saved registers,
versus 624 for the allocated-word visit. The scratch cofactor helper reserves
800 bytes. These are this build's per-function frames, not a combined maximum
stack bound: recursion and helper calls add their own live frames. Smaller
heap-allocation surface does not imply smaller stack frames or faster traversal.

```sh
python3 proposal/experiments/event-repr-lab/inspect_assembly.py --tag my-scratch --scope classify
```

## Decision for the next experiment

Keep both buffer strategies. Bounded scratch is useful when repeated local
alignment dominates, and preserves the exact canonical/readout contract.
These results do not justify selecting it globally or selecting essential
tables as the universal resident representation. Stop tuning this allocation
detail for now and test mapped operands consumed directly by relational product.
The scalar oracle and allocated-word path remain available; no production
default changes in this laboratory.

General [renamed operand views](VIEW-PRODUCT.md) remain a separate experiment. A borrowed view
must retain its checked coordinate map and support meaning, and actual outputs
still need canonical publication. The [map proofs](LEAN.md) now distinguish
faithful readouts from quantifier base change; scratch reuse supplies neither
certificate by itself.
