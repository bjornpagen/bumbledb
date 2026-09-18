# Borrowed local words: exact readout without resident intermediates

This control replaces the essential carrier's per-assignment local classifier.
It preserves the [Event signature contract](SIGNATURE-CALCULUS.md), the full
constructive algebra, both [resident stores](SLABS.md), and the recursive
support/operand traversal. It is implemented in
[occupancy_words.rs](src/occupancy_words.rs), with the scalar path retained in
[occupancy.rs](src/occupancy.rs).

Select `EVENT_LAB_OCCUPANCY_KERNEL=scalar|words`. Scalar remains the default.
The carrier reads this setting once at construction; queries do not read an
environment variable per binding. This is an execution setting, not part of
Event identity or its source law. Both kernels produce the same complete
four-bit signature, not a predicate-specific partial answer.

## The normal form supplies a useful boundary

The traversal enters a local cube only when the union of remaining coordinates
has at most K members. An essential canonical branch has more than K essential
coordinates, and branch views have no pinned coordinates. Thus every operand
at this boundary is a constant or a table. A branch there is an invariant
violation, not a reason to silently weaken the answer.

The word path does three things for each operand:

1. Borrow a matching table directly, including its complement flag.
2. If coordinates are pinned, select those cofactors in numeric local-axis order.
3. Broadcast each missing coordinate into the common cube, preserving that order.

The resulting word planes are scanned with the same four supported Venn masks.
Complement is applied on read. The support plane also receives the valid-bit
mask, so padding cannot create a witness. Constants need no plane allocation.
Only a complete signature of 15 permits early saturation; all other memoized
signatures are exact results of completed exploration.

The traversal memo keys all three `(root, pinned, values)` views. Keeping only
the roots would conflate different restrictions of one predicate. The
[Lean fixed-coordinate counterexample](lean/ReadoutMaps.lean) makes that danger
explicit: a legal restriction can remove a possibility. Conversely, a checked
map whose support image is exact preserves every occupied cell. These are
semantic conditions for reuse, not properties inferred from a physical pointer.

Broadcasting an absent operand axis repeats its truth value across that axis.
The support plane still carries any coupling between coordinates. No stochastic
independence is introduced, and no omitted draw is removed from its source law.
The classifier contracts the same admitted joint space in a different layout.

Pinning and broadcasting allocate temporary `Vec<u64>` buffers in this control.
Matching tables stay borrowed. With K=9 each aligned plane is at most eight
words, but multiple temporary buffers and the traversal hash memo still incur
allocation. This experiment has no bounded-stack or retained-scratch optimization;
the later [scratch control](SCRATCH.md) measures that change separately.
No temporary view becomes a public Event, a resident canonical node, or a
persistent identity. Immutable arena borrows last through classification, and
all temporary views are dropped before the owner borrow returns.

The raw word module adds axis insertion using word copies for axes above the
word boundary and broadword spreading/duplication within a word. Existing
constructive algorithms do not switch to this insertion path. This separates
the new readout kernel from representation normalization and output publication.

## Correctness evidence

The [optimized raw check](results/words-raw-check.json) passes eight test functions.
The complete shared suites pass under both [enum](results/words-enum-check.json)
and [slab](results/words-slab-check.json) storage. They include prior ownership,
dependency, transport, source-law and full relation checks, alongside:

- Scalar/word agreement for all small supported predicate pairs and all fifteen
  signatures, including complement/swap transformations.
- Identical recursive subproblem, local-cube and memo-entry counts between the
  two kernels. The word path performs no scalar local-assignment evaluation.
- Axis insertion against independent pointwise index deletion, for every axis
  position through twelve local coordinates and multiple word patterns.
- Borrowed, pinned and broadcast views against raw point evaluation, including
  both polarities, empty remaining masks, padding, word boundaries, and a
  twelve-coordinate roster scattered across 62 ambient coordinates.
- Unchanged resident nodes/bytes after the read-only operations.

The retained debug log reports the scattered ten-coordinate proper-inclusion
case. Both K=6 and K=9 use sixteen word batches and zero scalar assignments.
K=6 borrows 48 planes across sixteen cubes; K=9 borrows six across two cubes.
That fixture requires no alignment vectors; other tested views do. The traversal
memo remains temporary allocated storage. These counters are correctness
diagnostics, not native query timings or a universal operation-count bound.

The existing [Lean theorems](LEAN.md) establish signature sufficiency and the
meaning of essential coordinates. They do not verify this Rust word-insertion
loop. Differential checks and native execution retain that separate obligation.

## Native experiment and falsifiers

`words_sweep.py` runs scalar and word direct classification, materialized cells,
both essential storage layouts, and packed512/dispatched-dense controls in one
executable. Jobs are serial and shuffled with a recorded seed. Construction,
cloning, publication, checking and release remain outside the query timer; the
scope lock, input validation, actual join, signature classification, keep-table
filter and survivor counts remain inside. Full histograms and every group's
survivor count are checked against independent world/row enumeration.

The workload includes both Coup presentations and `ordered_65536`: 48 nested
prefix cuts, their complements, empty and full. Two prefixes are nested, so one
off-diagonal Venn cell is absent; complement merely permutes cells. No pair
can have signature 15. The benchmark asserts this from its independent oracle.
All three predicates still have nontrivial selectivity. This prevents an
evaluation based entirely on quickly finding all four witnesses. It does not
disable structural pruning, trivial-operand rules or exact pair memoization.

Compare novel-pair and cached replay, both physical stores, empty-cell proofs
and full occupancy, and the exact same returned histogram. Charge temporary
alignment and traversal work to the query even though it does not appear in
resident memory. A read-only kernel can still lose to constructive Apply.

```sh
python3 proposal/experiments/event-repr-lab/classify_check.py --essential-layout slab --occupancy-kernel words --output my-words-check.json
python3 proposal/experiments/event-repr-lab/run.py --build-only
python3 proposal/experiments/event-repr-lab/words_sweep.py --output my-words.json
python3 proposal/experiments/event-repr-lab/words_comparison.py my-words.json --output MY-WORDS.md --record my-words-comparison.json
```

The comparison rejects mixed binaries, mismatched result histograms, changing
resident footprints in direct mode, and scalar/word differences in input arena
or signature-cache size. It retains the latest supplied process per exact case,
never the fastest repetition. This remains a computed-stage Free Join sink;
native Event schema support, planner placement and batched per-column residuals
remain separate work.

## Native results and decision

The [full comparison](WORD-CLASSIFIER-MEASUREMENTS.md) retains 96 passing serial
processes: a smoke, a three-sample complete sweep, and an independently shuffled
nine-sample repeat for essential512 and the packed/dense controls. There are
576 distinct cases and 144 matched scalar/word pairs. Every pair has identical
input/final resident storage and pair-cache size. Every carrier returns the same
complete histogram and checked survivor counts for a given query.

These are nine-sample fresh medians for clover inclusion with pair memo enabled:

| Presentation | Store | Scalar, ms | Words, ms |
| --- | --- | ---: | ---: |
| Compact Coup | Enum | 7.765 | 0.292 |
| Compact Coup | Slab | 9.757 | 0.299 |
| Card-coordinate Coup | Enum | 88.185 | 1.470 |
| Card-coordinate Coup | Slab | 102.905 | 1.471 |
| Ordered cuts | Enum | 7.376 | 0.581 |
| Ordered cuts | Slab | 7.367 | 0.547 |

The two Coup queries retain 321 pair signatures; the ordered query retains 384.
Each cache occupies about 11.2 KB in this implementation. Warm cached replay is
roughly 0.11 ms and does not establish the first-use improvement. Without pair
memoization, the enum medians are 92.185 → 2.097 ms for compact Coup,
1,136.332 → 16.354 ms for card-coordinate Coup, and 72.456 → 4.342 ms for ordered
cuts. The word path creates no resident nodes in either cache mode.

The complete first sweep is retained even where timings were unstable. Its
ordered slab/scalar sample ranged from 27.478 to 49.754 ms; the subsequent
independently shuffled repeat has a 7.367 ms median. Tables report the latest
supplied process per exact case, all samples and ranges, without selecting the
fastest process or attributing cross-process variation to a code change.

This resolves the per-assignment consumer problem, **not the representation
competition**. Same-repeat direct controls give dense/packed512 medians of
0.132/0.207 ms for compact Coup and 0.311/0.662 ms for card-coordinate Coup.
For ordered cuts, packed512's materialized-cell path takes 0.248 ms versus
0.364 ms direct, while retaining another 145.2 KB. Essential512's word reader
and its constructed-cell path are much closer there. The query should specify
the exact answer it needs without prescribing one universal execution algorithm.

The compact store remains useful with this consumer. Its read-only carrier
occupies 60.9 versus enum's 108.7 KB for compact Coup, 602.2 versus 932.4 KB for
card-coordinate Coup, and 20.3 versus 38.2 KB for ordered cuts. These are carrier
estimates, excluding pair caches and temporary alignment/traversal storage.
The earlier scalar layout regression cannot by itself reject the compact store;
nor do these readout results establish lower latency for constructive relation
programs. The enum and scalar controls remain available and remain lab defaults.

## Actual ARM64

[Nine extracted symbols](results/assembly-words.json) match the exact executable
and source revision used by the sweeps. Both scalar and word visits are present
for K=6 and K=9. The word classifier was inlined into the recursive visit; it is
not a separate exported loop symbol.

The scalar local path calls the raw evaluator three times per assignment. The
word path decodes three operand planes, invokes cofactor/broadcast only for
alignment, then reads their words with scalar loads, `EOR`, `BIC`, `AND`, tests
and conditional selection. Its loop exits on signature 15. Both word-visit
symbols contain no NEON Boolean instructions. `CNT`/`ADDV` compute coordinate
counts; vector register loads also copy metadata. Neither constitutes vectorized
occupancy. The shared cofactor kernel does contain NEON Boolean code, while
broadcast uses scalar spreading/masks and block copies in this extraction.

The recursive path still uses restricted cofactors, which may evaluate fully
pinned tables. Temporary buffers and memo work remain. The evidence establishes
word contraction in place of the local assignment loop; it does not establish
an allocation-free classifier, batched `TBL` filtering, or native planner pushdown.

```sh
python3 proposal/experiments/event-repr-lab/inspect_assembly.py --tag my-words --scope classify
```

## Isolated allocation control

Alignment has a fixed bound once the local cutoff is fixed: one word per plane
at K=6, eight at K=9. The later [scratch kernel](SCRATCH.md) keeps matching
tables borrowed and uses reusable bounded buffers for cofactors and broadcasts. It must
finish a local contraction before reusing those buffers; no borrowed scratch
may enter the recursive memo or become a resident Event. A workspace passed
through sequential visits can enforce that lifetime without a scratch arena
attached to every canonical node.

Keep the allocated-word path as a same-binary control. Match the complete
histogram, traversal and pair-cache footprint under both stores, and charge all
alignment work inside the query timer. A zero alignment-allocation count would
not mean an allocation-free classifier: the traversal hash memo still allocates.
General [renamed operand views and constructive relational products](VIEW-PRODUCT.md)
remain a different experiment with checked maps and canonical output publication.
