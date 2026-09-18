# Input views, output order, and local readout

The [first mapped product](VIEW-PRODUCT.md) proves a legal execution option,
not a universally good schedule. Its [complete initial comparison](VIEW-MEASUREMENTS.md)
retains 78 passing native processes, 240 distinct query cases and 96 matched
strategy pairs. Every case produces the oracle's canonical relation outputs.

At 18 coordinates, essential512/enum, with outer memo enabled, the five-sample
bit-major product median rises from 2.527 to 12.531 ms. Its retained bytes fall
from about 591 to 317 KB. Face-major rises from 2.290 to 29.447 ms and retained
bytes grow from about 520 to 1,018 KB. Fewer explicit intermediate operators do
not establish less work or less retained storage.

These initial results are controls, not a precise universal speed claim. They
combine two independent changes: reading mapped inputs without constructing
renamed Events, and moving the output permutation into the product traversal.

## Why output naming changes physical elimination

Composition computes `exists Y: R(X,Y) & Q(Y,Z)`, then presents XZ as XY.
The actual `face_order` visits descending faces: Z,Y,X (and descending bits
inside each face). In the materialized/late-output working order, retained Z
precedes hidden Y. Fusing the final Y/Z swap makes final Z represent old hidden
Y: traversal now starts with that hidden face, then retained old Z and X.

Thus fusion moves elimination **earlier** in this fixture. The initial design
hypothesis described the direction backward; inspecting `face_order` and the
raw arena's minimum-rank `top` exposed the error. The measured node difference
is unaffected. Earlier elimination can still cost more: a borrowed operand may
have to pin descendants before its stored root, and unions of partial retained
results can grow. A favorable schedule must respect operand structure and
output construction together; neither early elimination nor maximal fusion is
a universal rule. The factorial separates the two complete strategies, not
all costs within each strategy.

The native control now offers:

| Strategy | Operand maps | Elimination traversal | Output map |
| --- | --- | --- | --- |
| materialized | Construct canonical renamed operand | Working order | Construct canonical rename |
| views-inputs | Read through checked maps | Working order | Construct canonical rename |
| views | Read through checked maps | Final output order | Fused into traversal |

All three publish the same scoped canonical Event. No schedule becomes part of
its equality, complement or database key. The outer memo includes complete
root/map pairs, elimination mask and output map; its lifetime also fixes the
chosen schedule. Output renaming in `views-inputs` uses the existing cache and
its storage is charged. None of these modes admits constrained-support inputs
to the current full-product-only kernel.

## A separate local reader

The first kernel evaluates a deferred raw branch at every assignment of the
small residual cube. The new word reader recursively reads its normalized low
and high children into that same cube, then selects words with

```text
(low & !selector) | (high & selector)
```

The selector is the mapped branch coordinate in numeric local-bit order.
Inherited pins are normalized in each child before reading; the existing table
reader still handles cofactors, permutations, broadcasts and complement polarity.
Input borrows end before canonical insertion. Temporary plane buffers allocate;
this is not the earlier bounded-scratch classifier.

`EVENT_LAB_VIEW_KERNEL=assignments|words` is captured once per raw arena. Both
modes remain in the same executable and share product traversal, memo keys and
canonical constructors. Matched cases require identical input and final node,
cache and byte counts; the local reader must not quietly change the schedule.
The word kernel's branch merges are counted separately from scalar assignments.
Its recursive expansion is bounded by the residual cube, but may still repeat
work and allocate many buffers. This control changes deferred **branch** readout;
ordinary table cofactoring, permutation and broadcast already used word kernels
in the first revision. If no deferred branch reaches the local boundary, changing
this switch cannot remove table-alignment work.

## Proof and research boundary

[Lean](LEAN.md) now checks 44 central reports. The added output-bijection law
allows delaying output renaming. A joint-witness reindexing with a section
preserves existence, even if lifts are not unique. The local Shannon identity
is proved at one meaningful Boolean bit. These laws do not certify Rust
alignment, padding, arena canonicality or generated machine instructions.

The retained [reachability paper](../../research/block-decomposition/decision-diagram-reachability.txt),
arXiv:2212.03684v1, Section 2.5 equation (3), presents image as projection followed
by renaming. Section 2.6 explains the effect of variable order and intermediate
diagrams. The [AOMDD paper](../../research/block-decomposition/mateescu-dechter-weighted-aomdd.txt),
arXiv:1206.5266v1, distinguishes induced width and pathwidth. These motivate
separating order from representation; they do not predict this exact kernel's
runtime. The [reading record](results/view2-reading.json) retains source hashes.

## Measured result: retain several schedules

The [factorial tables](VIEW-FACTORIAL.md) retain 292 passing native processes
and separate 528 query cases and 768 matched comparisons. The broad sweep uses five samples; focused independent repeats use
nine, keep the same executable, and retain both dense and packed controls.
For the 18-coordinate full relation program with outer memo enabled, the
essential512 face-major repeat gives:

| Store | Materialized, ms / KB | Fused assignments, ms / KB | Fused words, ms / KB | Late-output words, ms / KB |
| --- | ---: | ---: | ---: | ---: |
| Enum | 7.096 / 1,056.6 | 80.734 / 1,734.3 | 49.126 / 1,734.3 | 22.060 / 275.9 |
| Slab | 6.981 / 668.1 | 90.416 / 1,108.2 | 49.502 / 1,108.2 | 22.557 / 194.9 |

These are medians and retained byte estimates, not peak memory. Word/assignment
pairs keep exactly the same output nodes and arena/cache footprint. Delaying
output renaming reduces retained nodes from 8,088 to 1,642; it is still slower
than materialization's 5,273-node result arena. Those counts include canonical
intermediates retained by the manager, not just the final Event graphs.

At 12 coordinates the same face-major full program favors views: enum
materialization takes 0.527 ms versus 0.387 ms for late-output words, and slab
0.494 versus 0.379 ms. This is a within-carrier win; dense and packed controls
remain part of the comparison. It does not select essential tables over them.

The 18-coordinate bit-major product lane shows why the two changes must stay
separate: enum materialization takes 2.528 ms, fused assignments 12.332 ms,
fused words 12.247 ms, and late-output words 13.053 ms. The branch-word reader
is not a general cure. A single smoke sample suggested a large face-major
product improvement that the repeated process does **not** reproduce; both
fused readers are around 28 ms in that repeat. Keep the smoke as evidence,
use the repeats for these comparisons.

The design conclusion is a choice of physical plan, not a new public type.
Borrowed input views, canonicalized intermediates and output-late contraction
are legitimate implementations of the same operation. Maximal fusion cannot be
required by Event's semantics or its stable key. Materialization remains the
default in this lab; restricted-support mapped products and an adaptive planner
still need implementation and independent acceptance.

## Checks and measurement status

Both raw stores pass nine tests, including the earlier exhaustive mapped-product
cases, all three-variable predicates under pending pins and several maps, and
64 scattered multiword cases in a 62-coordinate presentation. The latter compare
both local readers' exact canonical IDs, arena/cache sizes and traversal counts,
then compare materialization and 80 independent target assignments per case.

The first memory assertion accidentally compared an original arena with a clone,
whose collection capacities differed. The failure and exact sources are retained
in [the diagnostic record](results/view2-sanity-check.json) and
[its source snapshot](results/views2-sanity-src). Both sides now begin with equal
cloned capacities. This was a test accounting error; canonical IDs already agreed.

The shared release suites pass sixteen test functions with delayed-output/word/enum
and fused-output/word/slab strategies. Per-strategy native verification precedes
timing. The [native acceptance replay](results/view2-acceptance.json) passes
20 processes: 48 owned relation-query cases at 12/18 coordinates and 24 symbolic
programs at 18/36/60 coordinates. The owned lane verifies exact output packets,
new result publication, use after original inputs/owners are dropped, and final
owner destruction. Some owned rows target the packed control, which keeps the
materialized strategy. The mapped modes are exercised on both essential targets.
The symbolic lane includes the million-state chain, compares exact symbolic
outputs and counts, and retains result ownership; it does not enumerate its
2^60 ambient assignments.

Reproduce the full matched experiment, serially and outside other CPU work:

```sh
python3 proposal/experiments/event-repr-lab/view_check.py --layout enum --output my-view2-raw.json
python3 proposal/experiments/event-repr-lab/check.py --release --product views-inputs --view-kernel words --essential-kernel derived --output my-view2-shared.json
python3 proposal/experiments/event-repr-lab/run.py --build-only
python3 proposal/experiments/event-repr-lab/view_sweep.py --output my-view2.json
python3 proposal/experiments/event-repr-lab/view_factorial.py my-view2.json --output MY-VIEW2.md --record my-view2-comparison.json
```

The comparison refuses to merge executables, keeps the latest supplied process
per exact case, invalidates old timings after a later failed job, and retains
resource refusals and timeouts as outcomes. It separately matches each strategy
to materialization, word to assignment readout, and late to fused output mapping.
It never merges the 16-output product lane with the 80-output relation program.

## Actual ARM64, rather than a SIMD assumption

The [new assembly record](results/assembly-views-v2.json) retains both product
visits, both entry points, and the emitted shared plane reader. Product visits
reserve 464-byte frames and still contain vector OR operations. The plane reader
reserves 480 bytes per call; this is not a complete recursive stack bound.

Its 39 vector Boolean instructions are largely in coordinate scattering for
the retained assignment control. The Shannon merge itself is a scalar word loop:
loads and polarity XORs feed `and`, `bic`, `orr` and a word store. There is no
basis here for calling it a batched NEON select kernel. It still replaces many
assignment evaluations with exact 64-bit operations. Separating constant/table
plane cases before the loop could make a later SIMD experiment concrete, but
that is an unmeasured implementation candidate, not the explanation of these
results. The [first revision's assembly](results/assembly-views-v1.json) is
retained separately, with its own executable hash.


## Work counters reveal the next substantive problem

The [untimed diagnostic](results/view2-work.json) checks 96 cases under both
stores and readers. It replays the 24 unique clover bank-index pairs from fresh
input clones. It omits actual Free Join, grouped union, cross-product caching and
the shared result arena, so these are **not** full-query profiles or timings.
Each output must equal the canonical materialized result. The input formulas
and order construction are retained in [the diagnostic source](essential-prototype/examples/view_work.rs).

At 18 coordinates and K=9, **none** of these products enters the deferred raw
branch fallback, in any of the three layouts. Thus assignment/word reader
switches have identical logical work here. For bit-major order, both schedules
visit 12,360 subproblems and 6,144 local cubes; they perform 42,240 cofactor and
42,240 broadcast steps. Fused output additionally performs 5,376 local
permutations. The face-major fused path performs 110,592 cofactors and 110,592
broadcasts, versus 70,656 each with delayed output renaming. Each current
cofactor/broadcast step allocates a new word buffer. These counts explain why
improving a nonexistent branch fallback cannot fix this product fixture; they
do not explain every cost in the full closure/residual program.

Canonical intermediates also purchase reduction and reusable identities, not
just storage. The diagnostic checks a concrete example with stored order z,y,x:

```text
F(x,y,z) = if x then y else z
pin x=true
conservative dependence: original axes minus x = {y,z}
exact reduced dependence: {y}
```

A deferred descriptor can keep the raw z root even though z is now irrelevant.
Canonical cofactoring collapses it to y. Distinct pin descriptors can likewise
represent the same function while occupying different memo entries. The reduced
BDD rules in the retained AOMDD paper explicitly remove equal-child tests and
merge identical nodes; borrowing does not automatically obtain those benefits.
This is a real mathematical distinction, but its share of the measured slowdown
has not yet been isolated.

The next controls should keep the current five strategy/reader combinations:

1. Fuse the local pin/permutation/broadcast map into one planned alignment,
   or reuse identical aligned planes within the invocation. Keep the traversal,
   canonical output sequence and resident footprint fixed; measure temporary
   workspace rather than silently calling all saved resident bytes a peak-memory win.
2. Separately compare canonicalized residuals against deferred pins. Measure
   whether exact reduced dependence and memo reuse repay normalization. Preserve
   descriptor maps, support and owner identity; a normalized residual must never
   be reused across incompatible contexts.
3. Exercise non-full joint support before promoting mapped products to the
   general Event capability. The current kernel still returns missing capability
   there; full-product timing cannot authorize a Cartesian substitution.

These are hypotheses for the next experiment. This cycle settles neither a
universal representation nor the production Free Join lowering. It does settle
that local word readout, working order and normalization are separate choices
under the same algebra.
