# Tables over essential coordinates

Two complete lab carriers, `essential64` and `essential512`, implement this
competing canonical representation. It is **not a selected backend**. The
[native baseline](ESSENTIAL-BASELINE.md) retains 163 measured cases and both
scalar/word algorithms in one executable; all 33 processes passed. Fixed-tail
packed carriers remain the measured controls. The experiment follows from two observed
boundaries: maps crossing a fixed prefix/tail cut require rebuilding, and
tables can carry axes on which their function no longer depends.

## Derive the cut from the represented function

Fix a binary coordinate order and a small integer k. A coordinate is essential
to f when its two cofactors differ. Let `vars(f)` be that exact set, independent
of how f was constructed. Define the canonical raw Boolean representation:

```text
if f is constant:
    use a terminal
else if |vars(f)| <= k:
    store a truth table over exactly vars(f)
else:
    split on the earliest essential coordinate in the fixed order
    recursively represent its two cofactors
```

Each table orders its local axes deterministically by semantic coordinate
number. Each branch follows the manager's fixed physical order. Those orders
have different roles and must not be silently confused by a map or an importer.

This is stronger than choosing a different global leaf cut. An event depending
on nine coordinates can be a 512-bit table even when those coordinates are
spread across a sixty-coordinate ambient presentation. After projection or
substitution it can stay a local table, provided its exact essential set still
fits. A combination depending on more coordinates returns to ordered splitting.

## Canonicality has an explicit proof obligation

The decision to use a table depends on an extensional property of f, not a
construction history or an allocation heuristic. The base case has one exact
axis list and one truth table. In the recursive case the earliest essential
coordinate and both cofactor functions are fixed. Induction gives uniqueness
for the stated order and k. Complemented edges require one consistent raw
orientation, as in the existing packed carrier.

For a reduced branch with different children,

```text
vars(branch(v, low, high)) = {v} union vars(low) union vars(high)
```

Every child variable affects f in its respective cofactor; v is essential
because the children differ. This permits exact support metadata. Whenever
the resulting union has at most k coordinates, the constructor must collapse
the branch to a normalized table. Keeping either representation opportunistically
would destroy the uniqueness argument. Table normalization must remove every
irrelevant axis, including axes made irrelevant by a Boolean combination.

The [Lean proof](LEAN.md) now establishes that, on a finite binary product,
the essential set is the unique least sufficient coordinate set. It also proves
cofactor characterization, exact projection to those coordinates, complement
invariance and removal under raw existential abstraction. The recursive
branch/table canonicality argument and its Rust implementation remain separate
proof obligations.

“Essential coordinates” here means Boolean dependence, **not admissible support**
and not statistical dependence. The ambient coordinate roster and designated
joint law remain intact. An omitted random draw still contributes to source-law
contraction, including coupling through shared unknown parameters. The existing
anchored construction supplies support-relative Event identity above this raw
Boolean representation.

The least-set theorem cannot be generalized to arbitrary admitted support.
If support says x=y, either x alone or y alone determines the Event x=true,
while their empty intersection does not. The
[supported-dependence proof](SUPPORTED-DEPENDENCE.md) makes that boundary exact.
The canonical raw representative still has a unique least physical mask;
logical dependency views may have several incomparable sufficient coordinate
sets. A new support-aware normal form must choose and justify its own convention.

## This retains the ordered-BDD complexity boundary

For fixed k, each local table can be expanded into an ordered Boolean decision
tree with at most `2^k - 1` internal nodes. Choosing its tests in the manager's
physical order preserves the order of all ancestor branches. Reducing the
expanded graph gives an ordinary reduced ordered BDD for the same function.
Conversely, bottom-up exact essential masks tell a reduced ordered BDD where
this normal form must collapse to a table. Constructing each such table costs
at most `2^k` local assignments.

Thus the representations translate with a bounded factor for fixed k. This is
our derivation for this normal form, not a theorem about a new arbitrary SDD
family. The [canonicity research review](../../research/representation-search.md)
and its retained primary paper distinguish ordered BDDs from reduced SDDs on
general vtrees. The candidate keeps the fixed-order BDD's fundamental boundary:
coordinate reordering and quantified results can still grow exponentially.
Flexible local tables change physical work and sharing; they do not establish
polynomial cost for every relational query or eliminate bad coordinate orders.

## Memory shape and matched kernels

The [slab-layout control](SLABS.md) now implements a sixteen-byte raw record,
single-copy table storage and collision-checked interning alongside the original
enum/key store. Both share all algorithms and pass the full semantic suite.
[Bounded scratch](SCRATCH.md) and clone elimination are separate controls so that storage
and transient allocation effects can be distinguished.

Keep sixteen-byte external `(scope, region)` values and one-bit public relative
complement. Compare k=6 and k=9 first. Internal raw references retain a tag and
complement bit. Candidate arenas contain:

- Ordinary ordered branch records and an exact essential-coordinate mask per
  node. A side array can preserve the existing sixteen-byte branch payload;
  its extra memory and load cost must be measured.
- Table records with an essential-coordinate mask and an offset to packed
  words. The number of live words follows from the mask's population, so a
  constant does not occupy a full 512-bit payload.
- Full-content interning, Boolean/product memos, and normalized table/cofactor
  caches. Hashes select buckets; collisions never establish equality.

Two tables first align their local axes. When the union fits the limit, Boolean
Apply broadcasts missing axes and uses word operations on that bounded local
cube. Quantification reduces selected axes and removes newly irrelevant axes.
Larger unions split on the earliest coordinate and recurse. A fused product
can combine intersection and elimination before publishing an intermediate.

A permutation of a table renames its axis mask and permutes its local bit axes.
It does not cross a globally fixed leaf boundary. A renamed table can still
violate an ancestor's physical order, so general branch reconstruction remains
necessary; this is not a claim that arbitrary reordering becomes cheap.

The retained broadword axis kernels are useful building blocks. Irrelevance
tests compare paired cofactors; axis removal compacts a chosen cofactor. Inspect
the actual ARM64 produced for these loops after profiling. More metadata, table
normalization and repeated local-axis alignment may cost more than they save.

## Acceptance and matched comparisons

The [independent finite reference](essential_reference.py) now passes
[525,888 function cases](results/essential-reference.json), covering every
function on two, three and four coordinates under two orders and every table
limit from one through the coordinate count. It also checks 13,120 arbitrary
Shannon reconstructions, 59,136 Boolean cases and 46,080 quantifier cases.
A separate sixty-coordinate example keeps an XOR of coordinates 0 and 59 in
one two-axis table. This establishes executable evidence for the normal form;
it is not a Rust carrier or a performance result.

The [standalone Rust prototype](essential-prototype/src/lib.rs) now implements
this raw normal form: exact essential masks, local tables, canonical branches,
complemented references, arbitrary cofactors, all sixteen Boolean operators and
existential abstraction. [Its retained check](results/essential-rust-check.json)
passes 262,144 four-coordinate function cases across two table limits and two
orders, with sampled Shannon reconstruction, Boolean and quantifier comparisons.
It also keeps the sixty-coordinate noncontiguous XOR in one local table.

The shared [raw implementation](src/essential_raw.rs) now underlies the
[scoped carrier](src/essential.rs). Both support the enum/key and slab stores with full-content equality and exact
essential-coordinate masks. The native runner labels the selected layout.
Memory estimates separate records, table words, interning, metadata and raw
operation/cofactor caches. Enum estimates include duplicate key payloads; slab
estimates include fingerprint collision links. Allocator metadata is excluded.

The scoped carrier implements anchored support-relative identity, one-bit
complement, simultaneous abstraction, fused relational product, checked local
renaming, count, graph/table packets and exact source-law contraction. There is
no probability weight in an Event row and no conversion through another resident
carrier to establish equality.

Both [scalar](results/essential-scalar-check.json) and
[word](results/essential-word-check.json) paths pass the shared checks. The
expanded matrix contains sixteen carriers, 1,024 finite source/target transfers,
121 forty-coordinate transfers, and eleven sixty-coordinate relation checks.
The new carriers also pass the existing pointwise dependency, partial-support,
borrowed-operation and owner-publication checks. Those are shared lab semantics;
production Event schema, persistence and transactions remain separate work.

The scalar and [word kernels](src/essential_words.rs) share every representation
invariant and interner. The word path aligns axes by broadcast/permutation,
compares paired word cofactors to detect irrelevant axes, and selects a Boolean
operator outside its word loop. A same-arena test clears operation caches and
requires exact matching resident IDs across scalar/word construction, Boolean
Apply, fused product and renaming on noncontiguous sixty-coordinate examples.

Observation smooths missing coordinates into the same parameter groups before
integration. A plan prepared before the query may lack a table's newly created
axis subset. Prepared subsets use the shared masked-popcount contraction; new
subsets have a bounded exact fallback. Omitting an axis from an Event's table
never removes that draw from its law, even when it shares an unknown parameter
with coordinates still present.

Raw `Ref` values remain trusted internal indices. The carrier's same owner and
support obligations apply before publication, as for the other lab candidates.

Reproduce its retained correctness record with:

```sh
python3 proposal/experiments/event-repr-lab/essential_prototype_check.py
```

First compare all small functions and variable orders with explicit tables:
canonical reconstruction through different branch histories, every Boolean
operator, cofactor/quantifier laws, relative complement, exact counts and maps.
Retain the full source-law contraction tests, especially skipped draws and
shared parameters. Test transport of local tables on noncontiguous coordinates.

Then run the same native Boolean Coup, full relation, owned-output and symbolic
counter fixtures. Keep the fixed-tail packed64/packed512, block64 and dense
controls. Include construction, map conversion, eventual publication/equality,
observation and retained bytes. Do not select a candidate from one kernel or
assume that eliminating the cut makes every workload faster.

This changes physical decomposition under the same Event algebra. It does not
add an application-level parameter, a new probability interpretation or a
different meaning for dependencies.

## The first native result argues against immediate selection

The [same-binary sweep](results/essential-initial-sweep.json) uses five samples
per case, bit-major relation layout, native identity and word-based transport.
It measures both essential table sizes with scalar and word algorithms, alongside
packed64, packed512, block64 and dispatched dense. The comparison generator keys
on the kernel mode: it never blends scalar and word timings into one candidate.

| Carrier / algorithm | Coup clover fresh, ms | Owned 18-coordinate relation query, ms | Full 60-coordinate counter query, ms |
| --- | ---: | ---: | ---: |
| essential64 / words | 241.107 | 4.184 | 5.604 |
| essential512 / words | 35.336 | 2.681 | 5.742 |
| packed64 | 11.571 | 0.858 | 0.828 |
| packed512 | 3.327 | 0.751 | 0.953 |
| block64 | 13.278 | 1.164 | 1.641 |
| dense-dispatched | 0.668 | 2.769 | not materializable in this carrier |

Coup includes 4,096 real joined bindings, grouping and exact counts with the
common operation memo enabled. The owned query imports only 24 inputs and
computes 80 results in the destination owner; its timer excludes observation.
The counter has one joined binding and five full algebraic outputs over
1,048,576 states per face. It tests symbolic capability, not join throughput.
Keep these different phase boundaries when reading the numbers.

The word path materially improves essential512's scalar results: the corresponding
three scalar medians are 410.528, 17.860 and 10.489 ms. It does not make flexible
tables a winner. Essential64 even runs this Coup case more slowly with word
kernels than with scalar loops. A 64-bit payload leaves little vector work to
amortize normalization and axis-alignment overhead.

[Actual ARM64](results/assembly-essential.json) contains NEON Boolean instruction
sites in table combination, permutation and cofactor compaction. There are 32,
16 and 105 such static sites in the three extracted symbols respectively; these
are **not dynamic operation counts or speedups**. Graph recursion, canonical
interning and coordinate metadata remain separate work. A retained
[three-second sampling diagnostic](results/essential-baseline-profile.json)
also points to allocation, hash-table work and repeated structural counting.
The diagnostic's instrumented timings are excluded from performance comparisons.

## Derive certificates from the normal form

A third algorithm mode, `derived`, keeps the same canonical normal form and
full-content interner. It uses properties already proved by constructors:

1. A bijection preserves exact essential axes. A literal has exactly its one
   axis. Reconstructing a reduced ordered branch has exactly the union proved
   above. These three paths can intern an already normalized table without
   rescanning for irrelevant axes. Arbitrary imported tables, Apply, cofactoring
   and abstraction still normalize their potentially changed dependence.
2. Every regular raw reference is false at the all-one assignment. Input edge
   polarities can therefore be absorbed into the four-bit operation, its bit
   zero selects output polarity, and operand sorting transposes the same four
   bits. Equivalent complemented/swapped calls share one Apply memo entry.
3. The cofactor at a branch's own variable is its corresponding child.
   Cofactoring also commutes with complement. Neither fact needs a graph walk;
   nontrivial cofactor memos can key on the regular root.
4. Exact essential-axis counts give an immutable structural-cardinality
   certificate. For a regular branch with essential width d, cache
   `count(low) * 2^(d-1-d_low) + count(high) * 2^(d-1-d_high)`.
   A table uses popcount. Complement subtracts from `2^d`; ambient missing axes
   multiply by `2^(ambient-d)`. Cache one `u64` per node in a side array and
   include its capacity in the memory estimate. The baseline modes do not
   allocate that array. This changes counted storage, not Event identity.

The certificate is a count of Boolean assignments, not a probability weight.
Shared-parameter observation continues to contract the graph under the complete
designated source law, including omitted draws. No uniform-law assumption enters
Event formation or FD/IND semantics.

[The shared checker](results/essential-derived-check.json) and
[standalone checker](results/essential-rust-check.json) pass. The matched test
clears operation caches before switching paths and requires identical resident
IDs for all sixteen Boolean operations, products and renamings on scattered
sixty-coordinate tables. It compares cached counts with recursive counts and
their complements.

## Same-binary derived comparison

The [current tables](ESSENTIAL-MEASUREMENTS.md) retain a new 163-case sweep plus
an eleven-sample focused repeat of Coup and the symbolic chain. All 48 processes
pass, including native verification of both modes. The latest supplied process
is used for each exact case, regardless of speed; the original baseline remains
separate because it uses a different executable.

| Carrier / algorithm | Coup clover fresh, ms | Owned 18-coordinate query, ms | Full 60-coordinate counter, ms | Counter arena KB |
| --- | ---: | ---: | ---: | ---: |
| essential64 / words | 92.901 | 3.997 | 5.756 | 1,068.8 |
| essential64 / derived | 66.554 | 3.461 | 3.788 | 801.0 |
| essential512 / words | 36.633 | 2.723 | 6.006 | 1,083.9 |
| essential512 / derived | 30.095 | 2.490 | 3.795 | 816.1 |
| packed64 | 10.935 | 0.863 | 0.757 | 534.6 |
| packed512 | 3.255 | 0.749 | 1.020 | 541.3 |
| dense-dispatched | 0.693 | 2.823 | not materializable | — |

The derived bundle improves both essential carriers and reduces retained
memory even after charging the count side array. It still does not win these
fresh workloads. On Coup, word and derived paths retain exactly the same
81,872 nodes for essential64 and 27,896 for essential512. The gains therefore
do not come from giving the query a smaller Event denotation or selecting a
different normal form. Several algorithm changes are bundled, so these numbers
do not isolate the benefit of the count cache alone.

Warm Coup replay is close to the join floor: derived essential64/512 take
0.158/0.159 ms, compared with dense's 0.163 ms. This is stationary cache reuse,
not a reversal of the fresh-query result. Likewise, the two-group exact-law
query takes 2.840 ms for derived essential512 versus 2.862 ms for words in the
five-sample sweep; that small difference is not evidence of a stable law-query
improvement. Packed512 takes 1.331 ms on that same law case.

Cross-revision timing variation is material: essential64's original word
Coup median was 241.107 ms; in the current build its focused-repeat median is
92.901 ms before enabling the derived path. Do not attribute that difference
to the derived algorithm. Only the same-binary comparison supports its claimed
gain. The same-binary repeat confirms the direction of the substantial effects.

[The current assembly](results/assembly-essential-derived.json) retains the
same vector table operations. A separate
[derived sampling diagnostic](results/essential-derived-profile.json) now
shows Apply, allocation and table alignment among its active stacks; repeated
recursive count no longer appears among the leading sampled work. Sampling
does not apportion an exact percentage of total cost.

The subsequent [slab control](SLABS.md) now answers the duplicated-payload
question: sixteen-byte records and single-copy words reduce retained carrier
bytes while preserving the exact normal form. The 146 matched case pairs have
identical logical counts, but latency is mixed and the direct scalar classifier
regresses in an independent repeat. Keep both stores. The subsequent
[borrowed-word classifier](WORD-CLASSIFIER.md) replaces scalar local assignment
evaluation while preserving those stores and the recursive traversal. Bounded
[scratch](SCRATCH.md) and removal of owned-node copies are separate experiments.
The present results reject selecting this implementation as a universal backend;
they do not establish the best achievable essential-table kernel.
