# Classify relationships without constructing four result Events

This page retains the original classifier comparison and its scalar essential
baseline. The subsequent [borrowed-word control](WORD-CLASSIFIER.md) replaces
that local consumer in a new same-binary experiment, with both storage layouts
and an ordered-cut fixture. Its results do not overwrite the evidence below.

This is a scoped predicate-kernel experiment over actual Free Join bindings.
It follows the actual [Allen execution kernel](../../../crates/bumbledb/src/exec/kernel/allen.rs):
classification and selection by a supplied predicate mask are separate costs.
The proposed [Event signature](../../kernels.md) already has exact semantics.

For aligned A and B in a nonempty admitted support S, bit `2*a+b` records
whether some supported world has those two truth values. The fifteen nonzero
signatures cover empty, full and proper query values. A predicate over these
signatures is a sixteen-entry keep table, with entry zero reserved as invalid.
The keep-table stage can reuse the same table-lookup pattern as Allen. It does
not make classification of arbitrary symbolic regions constant time.

The [shared read-only Rust kernel](src/occupancy.rs) borrows
the actual essential-table arena. Its [standalone wrapper](classify-prototype/src/lib.rs) passes
[15,360 supported predicate triples](results/classify-check.json), their
complement/swap symmetries, all fifteen signature classes and scattered
sixty-coordinate cases. Both the node count and retained arena bytes remain
unchanged. It does allocate temporary memo/view data; this is not a claim of
allocation-free execution or native performance.

Borrowed table views carry fixed coordinate assignments without publishing
canonical cofactor nodes. Branches use the manager's physical order. A bounded
scalar cube computes leaf occupancy. The subsequent
[word-classifier control](WORD-CLASSIFIER.md) also contracts borrowed/aligned
local tables, retaining the scalar path in the same executable.
The raw kernel accepts an explicit support root and can return zero for
empty support. The scoped carrier adapter supplies the owner's admitted support
and applies public complement polarity after classification. It is not a public
raw-ID API.

The [relationship-calculus derivation](SIGNATURE-CALCULUS.md) establishes the
signature's larger role: it decides emptiness/fullness of every binary Boolean
expression, supports all masks of pair predicates, and supplies a sound finite
composition envelope. It also proves that the latter is not strong composition
in finite scopes. The classifier is an exact readout, not a replacement for
the represented Event.

## Three matched paths

1. Construct all four support-relative Venn cells with the existing carrier,
   then inspect their canonical empty IDs. Charge construction and retained nodes.
2. Traverse S and the two operands together, accumulating occupied cells. Reuse
   common cofactors; combine leaf words without interning four new regions.
   Once all four cells are known occupied, further traversal cannot change the
   answer. Fixed-tail tables share physical axes; essential tables must pay any
   required bounded axis alignment.
3. Reuse exact pair signatures after the first classification. Normalize
   complement and swap by permuting the four bits, and charge cache storage.

All paths return the same signature. A requested witness or obstruction Event
is a separate constructive output and still pays canonical publication.
Comparing a read-only test with a requested four-Event result is not fair.

## Proof boundaries that the implementation must preserve

Support participates in classification. If S contains one of two raw assignments
and A=B=S, the only occupied cell is `11`: signature 8. Including the unsupported
assignment would falsely add cell `00` and report signature 9. In particular,
anchored raw representatives are false outside support; their all-false cell
cannot be used without the support restriction. Public complement polarity
permutes cell labels only after this restriction.

Skipped variables denote complete Boolean fibres, not missing samples. A
cofactor that does not depend on a coordinate is reused on both branches.
A scope's nonempty support is an admitted invariant, never inferred from a
positive model probability. Cross-scope handles must be aligned or rejected
before an equal-ID/constant fast path.

If a traversal stops early, it has proved that the cells it found are occupied.
It has not proved that unvisited cells are empty. An incomplete local result
must not enter the cache as an exact signature. Stopping with signature 15 is
safe because no additional occupied cell exists. A predicate-specific traversal
can use stronger termination rules, but its partial result needs a separate
contract and cannot masquerade as full classification.

Canonical constants and equality give valid shortcuts. For example, equal
proper Events occupy exactly `00` and `11`; complementary proper Events occupy
exactly `01` and `10`. Empty/full cases are distinguishable by canonical IDs.
These rules derive from resident identity rather than inspecting probability.

## Scoped implementations and ownership

Eight candidates implement a genuinely direct path: `essential64`,
`essential512`, the four `packed` sizes, `dense`, and `dense-dispatched`.
Dense scans support-masked words; packed recursively follows shared physical
axes and contracts terminal words; essential follows borrowed restricted views
and bounded local cubes with selectable scalar/word evaluation. The other eight carriers expose the
materialized-cell control only. No direct fallback silently constructs cells.

The common [signature layer](src/signature.rs) handles constant/equal/complement
shortcuts, exact orientation changes and optional pair memoization. The memo
is bound to one owner scope. Its keys normalize complement and operand order;
its cached signatures are reoriented for the caller. Canonical intermediate
nodes retained by the constructive control and pair-cache storage are charged
separately. The direct paths use fresh temporary traversal memos; zero resident
growth does not mean zero temporary allocation.

`Space::inspect_inputs` locks the actual retained owner and validates **all**
input scopes and published region IDs before entering the closure. That includes
inputs to an expression that could return through a constant shortcut. The
closure borrows the carrier and receives its scope token. Unlike the constructive
query boundary, it publishes no result Events.

[The shared suite](results/classify-owned-check.json) passes 7,788 scoped
classification cases per carrier, for all sixteen carriers, and 36 symbolic
sixty-coordinate pairs for each of six direct symbolic carriers. It also checks
foreign-owner and unpublished inputs before closure entry, and the entire prior
algebra, law, transport and dependency suite. Direct paths must leave resident
node count and estimated retained bytes unchanged. The standalone raw check
separately exercises all fifteen signatures and complement/swap symmetries.

## Native benchmark contract

[The native lane](src/signature_bench.rs) imports inputs only, including
complements and empty/full Events, then runs triangle and clover joins in the
actual engine. Each join applies inclusion, disjointness or overlap as a
sixteen-entry keep table and counts survivors in 64 groups. Pair memoization
is tested both on and off. Every predicate must accept and reject some bindings.

The oracle iterates supported worlds to classify each input pair, then runs
ordinary nested joins to derive the full signature histogram and every group
count. It neither calls a timed classifier nor imports expected output Events.
Fresh query timing includes the retained-owner lock, all input-key validation,
the real join, classification, keep-table lookup, histogram and grouped counts.
Warm queries retain constructed nodes and pair memos. A join-only baseline
includes the same owner lock and input resolution, but omits classification
and histogram/group accumulation.

Input construction/cloning/publication, native setup, oracle construction,
verification and final owner release are outside the query timer. Native setup
is reported separately. Rows carry actual destination scope tokens, validated
in the callback. A weak reference verifies owner reclamation after the query.
Direct queries assert unchanged resident nodes and bytes after both fresh and
warm execution; optional pair-cache bytes are reported separately. Byte estimates
exclude transient traversal allocations, allocator metadata and owner overhead.
The initial native smoke caught a harness error: the construction template had
more spare `Vec` capacity than its clone. The comparison now measures the
actual retained owner immediately before the query and checks consistency across
trials. The [failed run](results/classify-smoke.json) and its source revision
remain evidence; they are not included in performance comparisons.

This is a complete-binding computed-stage sink. It does not establish native
Event schema support, planner pushdown, per-column residual placement, or a
batched NEON keep-table kernel. The existing Allen kernel has those native
execution paths; a fast sink test is not evidence that Event has surpassed them.

## Native evidence and the resulting design choice

The [matched sweep and repeats](CLASSIFY-MEASUREMENTS.md) retain one executable,
six carriers, two Coup presentations, both join shapes, three predicates and
pair memos on/off. Every complete histogram and group count matches its oracle.
The eleven-sample dense/packed repeat gives these medians for the fresh clover
inclusion query with pair memo enabled (196 of 4,096 bindings survive):

| Presentation | Carrier | Four cells, ms | Direct, ms | Avoided resident growth |
| --- | --- | ---: | ---: | ---: |
| Compact 4,290 deals | dense-dispatched | 0.256 | 0.144 | 734 KB |
| Compact 4,290 deals | packed512 | 1.213 | 0.213 | 2,434 KB |
| Four card coordinates | dense-dispatched | 1.226 | 0.339 | 9,935 KB |
| Four card coordinates | packed512 | 3.884 | 0.699 | 6,797 KB |

Both algorithms retain 321 pair signatures in about 11.2 KB. Direct creates no
resident regions. Warm results are approximately 0.124–0.130 ms; join-only is
approximately 0.102–0.108 ms. These times include the contract above, not input
construction. Without the outer pair memo, compact dense's fresh query falls
from 1.687 to 0.206 ms by reading occupancy directly. Internal constructive
Apply caches remain intrinsic to the diagram controls.

The controlled clover topology visits nine of the fifteen signatures; 3,512
of its 4,096 bindings have signature 15, where direct classification can stop
early. All fifteen classes are covered by the separate correctness suite.
This fixture does not measure native equal/complement-only distributions or a
workload dominated by hard emptiness proofs. Its synthetic row topology and
stationary pair reuse limit any general carrier ranking.

Direct traversal is **not automatically faster**. On the card-coordinate
presentation, essential512's scalar local-cube classifier takes 74.929 ms with
pair memo, versus 32.239 ms for four constructed cells; without the pair memo it
takes 968.250 ms. These are five-sample medians from the main sweep. Avoiding
resident construction does not remove axis gathering, per-assignment predicate
evaluation or temporary memo work. Warm pair reuse conceals that first-use cost.
Do not select the essential carrier or a universal direct strategy from the
appeal of the read-only API.

The invariant that survives this comparison is the **reusable exact readout**.
Constructive algebra, relationship classification and witness publication can
share resident identity while asking for different outputs. A specialized
readout earns its place with a matched full-query result; its implementation
need not manufacture every Event from which its answer could be derived.

## Actual ARM64 inspection

[Five extracted symbols](results/assembly-classify.json) match the benchmark's
exact executable and source hashes. The word occupancy loop uses scalar word
loads, `BIC`/`AND`, condition-code selection and an exit when all four cells
are known occupied. It does not emit a NEON Boolean scan. The packed traversals
likewise contain no NEON Boolean instructions in the extracted symbols.

The essential traversal contains vector instructions in coordinate-scattering
work and `CNT`/`ADDV` for masks, but still calls the raw scalar evaluator three
times per local assignment. Those instructions are not a vectorized truth-table
occupancy kernel. The subsequent [borrowed-word experiment](WORD-CLASSIFIER.md)
implements word-aligned essential views. The separate
[bounded-scratch control](SCRATCH.md) now reuses alignment buffers. Both are separate from the
[resident slab-layout control](SLABS.md). The benchmark has no batched `TBL`
predicate filter, and no claim of flag-free symbolic classification follows.

```sh
python3 proposal/experiments/event-repr-lab/inspect_assembly.py --tag my-classifier --scope classify
```

## Acceptance retained for later kernels

The word-classifier control exploits a stronger fact than generic scalar
evaluation. At a local cube with at most K remaining coordinates, every live
restricted operand must be a constant or a table. A canonical branch has more
than K essential coordinates, and restricted branch views carry no pinned
coordinates. Thus the stopping rule itself excludes branches at that boundary.
This is an implementation invariant to check, not a new public Event assumption.

Keep the recursive traversal and exact signature memo fixed. Replace only its
local-cube evaluator with three borrowed/aligned word views:

1. Remove each table's pinned axes by exact cofactor selection. Its coordinates
   use semantic numeric order, independently of the branch order.
2. Broadcast absent axes into the common remaining-coordinate cube. If all
   masks already agree and no axis is pinned, borrow the resident words directly.
3. Apply raw polarity and the valid-bit mask, then accumulate the four supported
   Venn cells with word Boolean operations. Only signature 15 permits early
   completion of the present full-histogram contract.

For K=9 each aligned operand needs at most eight words. Neither alignment nor
classification needs a resident cofactor, normalized table, or Venn Event.
Scratch may be reused across sibling visits if no borrowed scratch escapes a
completed local contraction; Rust's mutable borrowing should enforce this.
Keep allocated-word and bounded-scratch implementations as separately
labelled controls. Resident enum/slab layout remains another independent axis.
The present scalar implementation stays available as a same-binary oracle and
timing control. The [implemented control](WORD-CLASSIFIER.md) uses borrowed
tables and temporary vectors for alignment. The [scratch control](SCRATCH.md)
now supplies a reusable workspace while retaining that allocated path.

Use novel and repeated pairs, equal/complementary/contained/overlapping pairs,
and constrained support under every existing carrier. Check against explicit
finite cells and all fifteen signature classes. Preserve complement/swap laws.
For the symbolic families, compare with canonical cell construction without
enumerating the ambient world space.

Any later per-row or fused residual kernel must retain first classification,
cache reuse, scope resolution, keep-table lookup and survivor selectivity.
Keep the source and exact executable hashes with the resulting ARM64 inspection.
Predicate-specific early termination needs its own groups-only query contract:
the present timed query produces a complete class histogram as well as survivors.
An incomplete occupancy result must not enter the exact signature memo, and a
boolean-only output must not be compared as though it had returned that histogram.

Reproduce the current correctness experiment with:

```sh
python3 proposal/experiments/event-repr-lab/classify_check.py
python3 proposal/experiments/event-repr-lab/check.py --essential-kernel derived --output my-classify-check.json
python3 proposal/experiments/event-repr-lab/run.py --build-only
python3 proposal/experiments/event-repr-lab/classify_sweep.py --output my-classifiers.json
python3 proposal/experiments/event-repr-lab/classify_comparison.py my-classifiers.json --output MY-CLASSIFIERS.md
```

The raw checker reuses the arena's existing canonicality tests and retains
source/Cargo hashes. Its standalone timing is excluded from the Free Join
comparison. Run timing sweeps serially without compilation or assembly inspection.
