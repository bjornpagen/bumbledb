# The Event design after the representation experiments

The central choice is now clear: **store a condition with its world identity,
and let the database construct further conditions from it.** Probability is an
observation of that value. A TypeSafe answer can supply a law or revise an
existing judgment under a declared import contract; it does not reduce the
condition to a Boolean or specify every dependency by itself.

The best-supported representation direction is a canonical completed function
over legal worlds, with symbolic splits and small tables over the coordinates
the function actually needs. The strongest performance result comes from using
dependency proofs to remove work. Packed tables still win important workloads;
the evidence does not select one universally fastest physical carrier.

The [difference-basis experiment](experiments/event-repr-lab/DIFFERENTIAL.md)
now provides a useful competing case: positive Davio gives about 30× fewer
reachable output records on grouped bilinear parity, but uses more query
workspace and Apply work; Coup favors Shannon structurally. The
[counting review](research/compact-counting.md) establishes why compactness
alone cannot promise cheap exact observation. Sparse cubic functions can stay
compact in Davio while exact uniform counting is #P-hard. This leaves the
public algebra intact and makes observation a separate carrier obligation.

The [inline-chain competitor](experiments/event-repr-lab/UNARY-CHAINS.md) retains
Shannon semantics and packs up to ten unary steps per sixteen-byte record.
It shrinks collected graphs, including a 62-to-10-record long-chain control,
but its one-bit operations rebuild the same query intermediates. The
[block follow-up](experiments/event-repr-lab/UNARY-BLOCKS.md) now answers part of
that question: cofactoring and paired projection consume labels directly, but
paired output construction can increase retained nodes. The
[shared-exit experiment](experiments/event-repr-lab/RANGES.md) adds grouped Apply
and passes 64 structural processes.
Grouped Apply strongly helps its deliberate strength family, but saves only
1.66% of Apply states in Coup's face order and under 0.1% in bit order. The
[native comparison](experiments/event-repr-lab/RANGE-NATIVE-MEASUREMENTS.md) now
passes 56 configurations: grouping does not beat the same records with one-bit
operations on fresh Coup queries. It is not promoted. The subsequent
[grouped-query factorization](experiments/event-repr-lab/FACTORIZED-PACK.md)
now runs through native Free Join: 5,376 differential executions pass, and all
72 matched fresh-query comparisons improve. This changes the schedule, not the
resident representation. Twelve separate Lean reports cover the exact join
dependency, group presence, participating validation and relational analogues.

## The invariant that makes the representation useful

An owner defines the legal worlds, semantic coordinates, permitted face maps,
and optional normalized joint law. Choose a fixed decoder `rho` that maps raw
codes onto legal worlds and fixes legal worlds. Represent Event A by the
canonical function `A(rho(code))`.

This normal form is itself the FD:

```text
decoded world -> Event membership
```

It preserves exact Event equality, complement and Boolean operations. Unused raw
codes are aliases of legal worlds, not extra probabilistic outcomes. Counts and
probabilities still refer to original support and the designated law. A witness
is decoded before being reported. Zero probability does not erase a legal world.

On a certified product of state faces, a face-independent Event can omit that
face physically. Each face can be an entire correlated Coup state. Copies of
legal state spaces do not assert independence between players or random draws.
For coupled support, the general support-aware operations remain exact.

## What lives in memory

The proposed row value stays two ordinary binding words:

```rust
#[repr(C)]
struct EventKey { space: u64, region: u64 }

// Current experimental slab record, beneath the owner:
#[repr(C)]
struct Record { variables: u64, payload: u64 }
```

The row's region selects a canonical function and its polarity; complement
toggles one bit. Constants share the distinguished empty/full identity. The
record's mask names exact raw dependence. Its payload selects either two signed
children plus a split coordinate, or an offset into one word slab. A small table
stores only its essential coordinates: nine coordinates need at most eight
64-bit words. Larger functions split in an immutable coordinate order.
Fingerprints locate candidates; complete content comparison establishes equality.

The owner holds decoder roots, original support, coordinate descriptors, unique
tables and operation caches. Retained result batches keep owners alive without
an Arc operation per binding. Cached transformations retain signed roots where
required, maps, eliminated coordinates and the decoder selection. Arena IDs are
resident handles, never canonical wire identities.

These are concrete experimental layouts, not unlimited production capacities.
The current raw mask supports fewer than 63 coordinates, and compact child
fields have checked index bounds. Arbitrary coordinate sets, large exact counts,
concurrent publication and persistent fact identity need their own implementation.

## One algebra across schema, query and storage

For a scalar determinant, a pointwise key says contributing Events are disjoint.
A containment says the source Event lies inside the matching target union.
A mirror to full proves a partition, so its probabilities sum to one under any
admitted normalized law. There is no row-weight normalization mechanism.

The same region can denote a set of states, a relation between states, an
observation case, or a counterexample to a dependency. Typed face descriptors
make those roles explicit. Composition joins on one shared middle world and
eliminates it. May, All, nonvacuous Must, converse and residuals construct further
Events. Finite closure requires a sealed finite state quotient. Observing a
probability comes after those constructions, with the original dependence intact.

Empty belongs to the value through storage too. The [storage review](event-storage.md)
closes an earlier hole in the proposal and proves the planner consequence:
complete Event-key equality identifies one fact only with a nonempty witness.
Scalar roster constraints still apply to empty-valued rows.

## How Free Join participates

Free Join binds ordinary facts and Event handles. Explicit Event operations then
construct regions from complete bindings; Pack unions contributors in a group.
For a relation composition, every conjunct must use the same hidden world.
Projecting each conjunct with an independent witness changes the answer.

The owner and dependency certificates prepare an internal operation: which
support gates are needed, which coordinates are hidden, and which decoder
coordinates need repair afterward. Product-fibre coverage can remove gates;
preserved membership FDs can remove repair on untouched faces. A fallback keeps
the full constraints whenever those certificates are unavailable. The public
schema acquires no tuning flags or representation-specific meaning.

The [projection comparison](experiments/event-repr-lab/FUSED-PROJECTION.md) establishes
another boundary: direct construction and staged projection produce identical
canonical Events, but staging wins the repeated difficult readout. Elimination
can simplify a function before substitution. Algebraic equality licenses a
choice of schedule; it does not prove a speedup.

The [native Pack experiment](experiments/event-repr-lab/PACK-NATIVE-MEASUREMENTS.md)
now demonstrates that distinction at the scalar join layer too. With the group
and owner fixed, a certified Cartesian branch relation permits reducing each
branch's Events before forming combinations. At fanout eight, 32,768 complete
bindings become 1,536 branch emissions plus 64 summary bindings. The same shared
world is retained throughout. Dense compact Coup with memo improves from 7.00 to
0.183 ms fresh, and its retained Event/memo estimate falls from 5.22 to 0.24 MB.
Several warm fanout-two cases lose to staging overhead. The planner must choose
between two proved schedules using their actual costs.

The [reversed-order follow-up](experiments/event-repr-lab/PACK-CROSSOVER-MEASUREMENTS.md)
confirms compact Coup at 7.365 → 0.172 ms. Card-coordinate dense improves from
47.512 to 0.878 ms; the main sweep's complete baseline was substantially slower,
so the precise speedup is process-sensitive. Every fanout-one case loses, which
rules out unconditional factorization even when its semantic proof applies.

Group presence and input validity are part of this query representation.
`Some(Event(empty))` is a result; an absent group is not. Validate every
participating row, even after a fold saturates, while leaving unmatched invalid
rows unevaluated. The tested fault contract is an unordered set, not first-error
ordering. Typed relational composition also distributes over branch unions;
residuals turn the corresponding unions into intersected obligations. Those
larger laws now have a [native relational Pack experiment](experiments/event-repr-lab/RELATIONAL-PACK.md):
11,904 differential executions and 384 main timing configurations pass. The
prototype checks a proposed separator against the actual normalized query,
rejects unaccounted-for conditions, and retains checked legal faces and one
shared environment. Six further Lean reports prove assignment gluing and the
residual aggregate. These remain denotational proofs, not Rust refinement.

The reversed-order repeat confirms fresh dense composition at 2.523 → 0.463 ms
and residual construction at 4.513 → 0.608 ms for the larger, fanout-eight case.
Packed composition's preferred working layout reverses after reduction: complete
pairs favor bit-major, while reduced branches favor face-major. This is direct
evidence for choosing the legal algebraic schedule before ranking layouts.
Small warm queries with cached operations still favor complete traversal.

## What ARM64 gets to execute

Once dependencies and coordinate alignment are resolved, small tables expose
word operations: AND/OR/XOR, conditional bit-select, occupancy of Venn cells,
cofactor shuffles, quantifier reductions and population counts. The retained
[ternary-kernel inspection](experiments/event-repr-lab/TERNARY-COMPLETION.md)
shows emitted ARM64 vector bit-selects. Those local kernels implement the same
canonical constructors; they do not replace ownership or support checks.

The larger gain is already above the instruction level. With ITE held fixed,
dependency gates reduced one repeated difficult query from 392.599 to 10.758 ms.
A separate controlled round then reduced full to local/selective completion from
10.659 to 2.247 ms. These are different frozen experiments, not one combined
speedup measurement. Packed still took 1.587 ms in the latter round. Keeping the
controls and negative results is part of choosing a defensible representation.

## The Allen analogy, precisely

Both types make infinitely or combinatorially many points available through a
compact region value, so keys and containments can reason pointwise. Event adds
closed set operations, named coordinate faces, relational contraction and
information abstraction while retaining alternatives for later observation.

An exact four-bit occupancy signature answers every binary Boolean possibility
test. It does **not** determine arbitrary relation composition: the Lean proofs
give two finite examples with identical signatures and different possible
intermediate Events. The finite table is a useful readout of the richer value.
Forcing it to carry the entire algebra would discard the very structure that
permits later questions.

The design now has a precise denotation, concrete competing carriers, native
Free Join evidence, and machine-checked optimization premises. Production Event
storage/planning, Rust refinement proofs, reclamation and general source-law
solving remain explicit obligations. That is the confidence boundary: the
mathematical contract is considerably firmer than the choice of every engine
subsystem or a claim of universal performance superiority.
