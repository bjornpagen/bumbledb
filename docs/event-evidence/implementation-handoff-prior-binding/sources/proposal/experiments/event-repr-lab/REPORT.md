# Event representation shootout

Research is closed. The [handoff](../../RESEARCH-HANDOFF.md) summarizes the
conclusions and production obligations; this report retains the experimental
history. The optional participation-index draft has no validated results.

The previous “one BDD-pair carrier for everything” decision was premature.
This laboratory implements twenty Rust carrier/kernel configurations, including
both completed-root policies at two local cutoffs, and runs them through
the actual Free Join executor, and keeps the raw timing samples and correctness
checks. The leading representation depends on the shape of the worlds, the
query's operations, and how much work can be reused.

The decision order is **native dependency-language synergy, algebraic elegance,
then performance**. [The dependency review](DEPENDENCIES.md) traces the engine's
actual pointwise keys, exact target-key validation and final-state admission.
Its coverage/conflict summary uses Event values for both halves of that proof.
Query timing alone cannot select a production design.

The immediate finite Coup candidate is an interned dense bitmap with a
complement bit. Symbolic diagrams remain necessary competitors for factored
spaces that cannot be enumerated. Packed-terminal diagrams and selector blocks
test how physical decomposition interacts with the larger algebra. The
three-layout comparison includes real coordinate movement, residuals and finite
closure, with two packed renaming algorithms in the same binary.
Interval runs earn a real place when the regions are contiguous in the chosen
enumeration. This is a measured direction for the next prototype, not a universal
backend selector or a production feature.

The additional [difference-basis screen](DIFFERENTIAL.md) tests four fixed
Shannon/Davio schedules with two kernels, without changing the frozen native
engine build. Coup favors Shannon's graph and work census; grouped bilinear
parity gives positive Davio about 30× smaller output graphs but more retained
workspace. The [counting extension](COUNTING.md) confirms another distinction:
a sparse cubic input can remain tiny while exact cofactor counting grows a
large arena. The inspected literature, explicit reduction and Lean identities
establish the general counting boundary separately from those finite results.
These new screens do not supply native Free Join latency comparisons.

The subsequent [inline-chain screen](UNARY-CHAINS.md) tests packing unary Shannon
steps into the same sixteen-byte record. All 24 processes pass. The long-chain
control falls from 62 to 10 collected records, but ordinary one-bit operations
restore identical retained arena counts on all three query fixtures. This
separates a storage improvement from an unestablished query-speed improvement.

The [block-kernel follow-up](UNARY-BLOCKS.md) passes all 48 structural processes
and deeper multi-coordinate checks. Paired projection sometimes saves work but
can retain more nodes; Coup's one-bit Apply remains unchanged in that round.
The [shared-exit implementation](RANGES.md) then adds grouped Apply.

The [shared-exit screen](RANGES.md) now passes 64 processes with grouped Apply.
On its deliberate strength family, records fall from 127 to 41 and Apply states
from 108 to 32. Coup's face order saves 1.66% of Apply states with unchanged
estimated retained bytes; its bit order saves under 0.1%. Those are structural
results; the following native comparison pays complete query costs.

The [native shared-exit comparison](RANGE-NATIVE-MEASUREMENTS.md) now passes
56 configurations after full scoped/symbolic admission. Grouping does not beat
the same records with one-bit operations on fresh Coup queries. This rejects
it as the new default. A separate [query-factorization experiment](FACTORIZED-PACK.md)
now eliminates independent scalar branch indices before constructing their
Cartesian binding product, while preserving shared world witnesses and present
empty groups. All 5,376 differential native executions pass. The
[144-configuration comparison](PACK-NATIVE-MEASUREMENTS.md) improves all 72
fresh schedule pairs, with six carriers held fixed in turn. Several warm
fanout-two pairs lose to staging overhead. At fanout eight, dense compact Coup
with memo falls from 7.00 to 0.183 ms fresh; the card-coordinate presentation
falls from 154 to 0.955 ms. This includes summary planning and construction.
It establishes a useful certified scheduling choice, not a new universal carrier
winner. Twelve separate Lean reports retain the join-dependency, group-presence,
participating-validation and typed relational laws outside the central suite.
The [reversed-order follow-up](PACK-CROSSOVER-MEASUREMENTS.md) adds 48
configurations and eleven samples each. Dense compact Coup gives 7.365 → 0.172
ms; card-coordinate Coup gives 47.512 → 0.878 ms. The changed complete baseline
limits the precision of any speedup claim. All fanout-one cases lose, confirming
that the rewrite needs a cost choice even when its proof applies.

The [relational Pack extension](RELATIONAL-PACK.md) now proves and measures the
larger operation family. Composition reduces both branches by union; a left
residual aggregate unions its antecedents and intersects its requirements.
The actual normalized query supplies a conservative separator check. Six new
Lean reports and 11,904 differential native executions pass. The main sweep
and reversed-order repeat retain 448 timing configurations. Every fresh matched
pair improves; small warm queries with outer memo lose to staging.

The repeated dense bit-major queries at width five/fanout eight improve from
2.523 to 0.463 ms for composition and 4.513 to 0.608 ms for residuals. Packed512
shows a layout reversal: complete composition favors bit-major, but factored
composition favors face-major. Choosing a physical layout before applying
the legal query algebra can therefore choose the wrong working order. This
bounded transition fixture retains shared source environments; it is not a
full Coup simulation or a universal physical-representation ranking.

## Exact readouts now have native measurements and Lean proofs

The [classifier experiment](CLASSIFY.md) separates reading a pair's exact
relationship from constructing its four Venn-cell Events. The
[same-binary sweep and focused repeats](CLASSIFY-MEASUREMENTS.md) pass all
35 processes and retain 264 distinct query cases: six carriers, two Coup
presentations, two join shapes, three predicates and pair memo on/off.

For the compact Coup clover inclusion query with pair memo, packed512's fresh
time falls from 1.213 to 0.213 ms while avoiding 2,434 KB of resident regions.
Dense falls from 0.256 to 0.144 ms and avoids 734 KB. Each algorithm retains
321 exact pair signatures in about 11.2 KB. These eleven-sample medians include
owner locking, input resolution, actual Free Join, classification, predicate
lookup, histogram and grouped survivor counts. Warm times approach the join
floor. Inputs/setup and requested constructive witnesses have separate costs.

The negative result matters as much: essential512's scalar local-cube reader
takes 74.929 ms on the card-coordinate query with pair memo, versus 32.239 ms
for materialized cells. Avoiding resident nodes is not sufficient to make a
traversal fast. The compact essential512 repeat confirms 77.839 versus 10.621 ms
without outer pair memo. All direct paths preserve resident nodes/bytes; their
temporary allocations remain real work. The fixture is dominated by fully
occupied four-cell signatures; it is not a universal predicate workload.

[Actual ARM64](results/assembly-classify.json) exposes scalar word occupancy
and essential per-assignment evaluation, even though parts of coordinate
scattering vectorize. Native Event residual placement, per-column batching and
the full sixteen-entry SIMD predicate table remain unimplemented. The initial
smoke's cloned-capacity accounting error is preserved with its correction.

The [relationship-calculus derivation](SIGNATURE-CALCULUS.md) proves the four
bits are exactly the information needed for all binary Boolean possibility
tests. It derives an associative fifteen-class composition envelope and shows
why it cannot be exact on finite scopes: a singleton and a splittable region
can have the same pair signature. Four cell counts capped at two settle one
existential intermediate Event in a full finite powerset.

[Lean now checks 195 central theorem reports](LEAN.md), including
essential-coordinate minimality/projection and restriction, logical dependency
limits on coupled support, the signature laws and composition
counterexample, and the anchored representation's
support-relative identity, complement and adjusted Boolean Apply. The map proofs
add the exact support-image criterion for preserving possibility, pair occupancy
and supported equality. The base-change proofs supply the complete-fibre
criterion for existential, universal and nonvacuous rewrites: lifts must exist,
but need not be unique. Borrowed-product proofs establish the exact joint-image
contract and counterexamples to separate-coverage and root-only reuse. These are
denotational proofs; they do not formally verify Rust or establish canonicality
of the arena implementation. The table/capacity derivation remains separately
checked in Python. Together, the proofs and experiments support a specific
design: retain exact constructive regions and expose reusable readouts whose
cost is paid only when the query asks for them.

## Reading local words resolves a kernel problem

The [borrowed-word experiment](WORD-CLASSIFIER.md) keeps the essential normal
form, both stores, recursive traversal and exact signature contract. It replaces
local assignment enumeration with borrowed tables and temporary word alignment.
The [same-binary comparison](WORD-CLASSIFIER-MEASUREMENTS.md) passes 96 serial
processes, retaining 576 cases and 144 matched scalar/word pairs. Nine-sample
fresh clover inclusion medians, with pair memo enabled, are:

| Presentation | Enum scalar → words, ms | Slab scalar → words, ms |
| --- | ---: | ---: |
| Compact Coup | 7.765 → 0.292 | 9.757 → 0.299 |
| Card-coordinate Coup | 88.185 → 1.470 | 102.905 → 1.471 |
| Ordered cuts | 7.376 → 0.581 | 7.367 → 0.547 |

All kernels return the same histogram and survivor counts; direct paths retain
identical arenas and pair-cache footprints. Ordered cuts forbid signature 15,
so the improvement survives a fixture that must establish missing Venn cells.
The independent repeat and full ranges remain visible, including a noisy scalar
process in the first sweep. Warm pair-cache replay near 0.11 ms is not used to
justify a classifier choice.

This result favors consuming the representation's local words, without
selecting the representation itself. Same-repeat dense/packed512 direct controls
take 0.132/0.207 ms on compact Coup and 0.311/0.662 ms on card-coordinate Coup.
For ordered cuts, packed512's constructive control is faster than its direct
reader, 0.248 versus 0.364 ms, while adding 145.2 KB of resident regions. Output
contracts belong in the algebra; execution strategy remains a measured choice.

[Actual ARM64](results/assembly-words.json) shows scalar word loads and Boolean
contraction replacing three raw evaluations per local assignment. The word
visits contain no NEON Boolean occupancy loop; shared cofactoring does contain
vector operations. This is a representation-aware word algorithm, not evidence
of a batched SIMD residual. That word control still allocates temporary alignment
vectors. The separate [scratch experiment](SCRATCH.md) replaces those buffers;
general renamed operand views and native Event planner integration remain
separate work.

## Bounded scratch helps when alignment is the work

The scratch control keeps the normal form, recursive subproblems, complete
signature histogram and resident/cache footprint identical. It borrows matching
table words and reuses a 32-byte workspace at K=6 or 256-byte workspace at K=9.
Those are workspace sizes, not full stack bounds. Traversal memos still allocate.
The final comparison retains 130 passing native processes, 768 distinct cases
and 192 matched kernel pairs, including independent repeats.

The new `axes_65536` fixture makes alignment unavoidable in 3,888 of 4,096
clover bindings. An independent checker derives the exact expected histogram,
survivors and first-use pair counts. The eleven-sample repeat confirms uncached
clover inclusion improving from 0.678 to 0.457 ms for essential64/enum and from
0.667 to 0.467 ms for essential512/enum. Both slab carriers improve as well.
With pair memo enabled, gains shrink and warm replay reaches the join floor.
The [full evidence and limits](SCRATCH.md) retain controls and outliers.

The older fixtures do not support a universal switch. Fifteen-sample
card-coordinate Coup medians with pair memo are 1.492→1.477 ms for enum and
1.499→1.487 ms for slab; without that cache, scratch is slightly slower.
Compact Coup and ordered cuts show small or mixed differences across repeats.
A severe noisy process remains recorded; a focused confirmation does not
reproduce it. Same-executable dense/packed512 direct controls still lead Coup.

[Actual ARM64](results/assembly-scratch.json) shows scalar word occupancy in
both paths, and vector operations in the cofactor helper. A scratch recursive
visit reserves 720 bytes including saved registers versus 624 for allocated
words, in addition to dispatcher/helper frames. The small workspace object
is not the whole stack, and no batched SIMD classifier follows from these gains.

This does not select a universal direct algorithm: packed512's materialized-cell
control takes 0.368 ms on that uncached query, while adding about 240 KB of
resident intermediates. Its direct control takes 0.519 ms. The result supports
bounded alignment as a useful execution option, while retaining construction
and caching as legitimate alternative strategies.

The [borrowed-product experiment](VIEW-PRODUCT.md) is larger than buffer
tuning: consume mapped operands directly during composition and projection,
then publish the canonical result. Lean establishes the joint-image contract.
The brief specifies pending-pin handling, complete memo identity and matched
native acceptance. The first Rust kernel now passes raw and shared tests under
both stores, including the pending-branch case and sixty-coordinate composition.
The [first native comparison](VIEW-MEASUREMENTS.md) retains 78 passing processes,
240 cases and 96 matched pairs. It is slower despite frequently retaining fewer
nodes; face-major output fusion can retain more nodes as well. The [factorial experiment](VIEW-ORDER.md) now separates local word readout from
working order: 292 passing native processes, 528 query cases and 768 matched
comparisons, including independent repeats and owned/symbolic acceptance.

Delaying output renaming cuts the essential512 face-major 18-coordinate full
program from 8,088 retained nodes to 1,642. The slab repeat uses about 195 KB
versus 668 KB for materialization, but takes 22.557 versus 6.981 ms. At twelve
coordinates the same view strategy improves 0.494 to 0.379 ms. Word branch
readout improves some full-program cases, but the isolated input-pair diagnostic
finds no branch fallback at all in its 24 products: repeated table alignment is
still expensive. A checked counterexample also shows why deferred pins can retain
coordinates that canonical cofactoring proves irrelevant. Stable Event identity
does not require one physical contraction schedule. The evidence keeps
materialization, borrowed views and delayed output renaming as separate choices;
it does not select a universal carrier or an adaptive production planner.

The [normalization/reuse experiment](REUSE.md) now tests those hypotheses
independently. Its 246-process initial sweep passes with 912 native query cases
and 1,920 matched comparisons. Nine-sample repeats confirm that canonical source
cofactors recover sharing which pending descriptors miss. For essential512 slab,
the 18-coordinate bit-major full program's output-late view improves from
7.574 to 2.001 ms, with resident storage increasing from 143.8 to 214.0 KB.
Materialization takes 2.342 ms and 301.6 KB; packed512 takes 0.738 ms and 512.4 KB.

On the face-major full program, normalized output-late views with bounded plane
reuse take 5.272 ms and 525.2 KB, versus materialization's 6.931 ms and 668.1 KB.
Fusing the output mapping with source normalization instead retains 11,800 nodes
and 1,573.4 KB. The isolated product lane does not inherit every full-program
win, and a twelve-coordinate repeat finds cache overhead. [The detailed report](REUSE.md)
keeps ranges, both stores, outer memo controls, temporary-memory limits and the
observed desktop-load caveat. The eight retained ARM64 symbols expose both the
full-key cache and canonical cofactor calls. No production strategy changes.

The [new supported-dependence proofs](SUPPORTED-DEPENDENCE.md) also prevent a
false generalization: x=y makes either coordinate sufficient for x=true, with
no unique least logical dependency set. The raw essential mask remains an exact
invariant of the canonical physical function. Extending contraction to legal
categorical domains needs explicit support and reconstruction capabilities.

## Arbitrary support and lawful relation roles are separate capabilities

The [scoped contraction](SCOPED-PRODUCT.md) now accepts arbitrary admitted
support by retaining the exact witness and intermediate-output restrictions.
All 33 smoke and 186 broad-sweep processes pass; [the native comparison](SCOPED-MEASUREMENTS.md)
retains 704 distinct configurations and 1,024 matched comparisons through Free
Join. Each query returns sixteen complete grouped Events checked against a
direct staged oracle outside every fresh and warm timing. These are different
answers from the eighty-output relation program above.

The [legal-relation proofs](LEGAL-RELATIONS.md) establish the stronger boundary.
Relations on legal domains retain associativity, identity and residuals even
when the encoding has unused codes or the legal domain depends on a retained
environment. But an input must actually depend on its declared faces. Full
support alone does not make a scratch-face Event a binary relation.

Role admission can use the algebra itself: existentially forgetting scratch
faces must leave the Event unchanged. The general readout theorem identifies
that fixed point with the membership FD even on coupled support. The product
workspace requires independent construction evidence, because scoped Full
passes every such fixed-point check. These proofs connect the relation API to
the dependency language without introducing another scalar representation.

The nine-sample repeats bring scoped evidence to **270 passing processes**.
On legal-product support, eighteen coordinates, bit-major order and outer memo,
essential512 slab's fused/source path takes **8.177 ms and 1,389.2 KB**, versus
materialization's **13.909 ms and 2,650.5 KB**. Packed512 takes 4.287 ms and
2,904.7 KB; dense takes 2.640 ms and 4,570.2 KB. The memory/time tradeoff is
concrete, while the candidate remains slower than those controls.

Copied-face support in face-major order reverses the essential result: fusion
takes 17.337 ms and 3,699.3 KB, versus materialization's 13.785 ms and 2,979.0 KB.
Output-late execution has lower memory and overlapping timing ranges. The
[detailed report](SCOPED-PRODUCT.md) retains ranges, normalization controls,
two large initial medians that fail to reproduce, and desktop-load evidence.
No universal schedule follows from this comparison.

Checked support maps permit removing the two extra gates on legal-domain
face permutations. The [new checked interface](LEGAL-RELATIONS.md) now implements
legal-domain construction, exact support admission, membership-role checks and
owner-bound relation/goal handles. The general gated kernel remains available.
The native comparison keeps both strategies; the certified strategy also caches
support-map checks, so a cached-renaming control is still required to isolate
the cost of gate removal alone.

The preceding scoped revision's separate acceptance passes 33 processes,
116 owned-result cases and 27 symbolic programs including million-state closure.
Those full-space checks bring that revision's total to 303 retained processes.
The new constrained constructor additionally passes shared 61-coordinate checks
for essential64, essential512 and packed512, with two different million-state
legal domains, exact analytical counts, identity, converse and closure. That is
constrained symbolic computation, not production cross-owner publication.

The [new native comparison](LEGAL-MEASUREMENTS.md) retains 240 configurations
and 632 matched comparisons. Its four sweeps pass 93 processes; separate native
acceptance adds fifteen, with 34 owned cases and fifteen symbolic programs.
Nine-sample repeats retain the certificate path's gains and discard no noisy
samples. On the environment-dependent bit-major case, essential512 slab fused
certified takes 28.062 ms and 6,867.1 KB, versus materialization's 45.111 ms and
11,527.3 KB. Dense takes 6.424 ms and 45,018.8 KB; packed512 takes 8.463 ms and
13,322.2 KB. The certified range includes a 74.336 ms sample. Several large
initial medians fail to reproduce. The [detailed review](LEGAL-RELATIONS.md)
retains ranges, weaker gains in the other layout, and the caching/identity-map
attribution limit.

The [fixed-decoder competitor](FIBRE-RETRACTION.md) now has a complete Rust
carrier and seventeen new Lean reports. Its normal form is exactly an FD from
decoded world to membership. Whole-face relation roles can physically omit the
scratch state, while general projections/maps retain exact fallback semantics.
A further proof shows that some support-preserving maps cannot commute with
**any** decoder: fast-path limits can be structural, not implementation defects.

The [native comparison](RETRACTION-MEASUREMENTS.md) covers 136 configurations
and 216 matched completion/control comparisons. The initial and controlled
experiments plus native acceptance pass 85 processes across two binaries.
Five-sample repeats expose both the improvement and its limits. On face-major,
environment-dependent width-six input, the 512-cell decoder takes 22.549 ms and
9,126.5 KB versus anchored 63.080 ms and 12,052.8 KB. But across layouts,
packed's bit-major 8.414 ms remains substantially faster. Word observation
reduces the decoder's bit-major count phase from 180.018 to 8.854 ms with identical
roots/storage; this is cardinality, not a general probabilistic-inference result.

The [prefix-preserving decoder](FIBRE-RETRACTION.md) now has seven Lean reports,
including exact reflection of prefix membership FDs, and independent finite and
Rust checks. Its [native comparison](PREFIX-MEASUREMENTS.md) passes 80 processes
including owned-result and symbolic acceptance, with 160 configurations and 128
matched prefix/control comparisons. Five-sample width-six medians with outer memo
reduce minimum repair's fibred bit-major query from 29.087 to 11.850 ms and its
holed bit-major query from 20.759 to 7.162 ms. These are same-executable comparisons.

The fibred bit-major computation phase drops from 20.365 to 5.044 ms, but prefix
observation still takes 6.794 ms. Packed's complete 9.321 ms remains ahead of
prefix's 11.850 ms; dense takes 6.601 ms. Prefix retains 6,476.1 KB versus minimum's
9,957.0 KB and packed's 13,322.2 KB. Constructor costs also remain in the generated
tables; their current algorithms are not inherent lower bounds.

Prefix repair improves selected dependency/projection structure, not every map.
On `{00,01,10}`, preserving the high-prefix readout conflicts with commuting
with a bit swap, whereas minimum repair commutes. Exact fallback preserves the
semantic operator. The [native information-readout lane](PREFIX-READOUTS.md)
now isolates the extra suffix-projection capability and exposes general-fallback
costs that the whole-face relation workload does not exercise. No universal
carrier is selected.

The [product-fibre count experiment](FACTOR-COUNTS.md) now exploits the same
membership dependency during observation. Its four Lean reports and differential
Rust checks justify replacing ignored raw-state multiplicity by exact legal
completion counts. Original support and joint laws retain their distinct roles.
All 110 native processes pass; 216 configurations produce 552 matched comparisons.
Each of 48 joint/factored count pairs keeps nodes and memory identical.

On the width-six fibred bit-major query, the prefix candidate's fresh median
falls from 11.719 to 5.208 ms, with observation falling from 6.696 to 0.168 ms.
Same-executable packed and dense controls take 8.402 and 6.406 ms. Prefix retains
6,485.7 KB versus 13,322.2 and 45,018.8 KB respectively. The 64-cell prefix repeat
takes 7.418 ms and 9,227.8 KB, while 512-cell leaves also win the grouped-layout
query/storage comparison. Smaller leaves construct the grouped-layout owner
faster; no universal cutoff follows. Dense still leads the grouped layout and
packed leads bounded bit-major input. The detailed note preserves construction,
count, outlier and sample boundaries, including a noisy packed process that
does not reproduce in the focused repeat. This is an implemented cardinality
specialization, separate from the new partial-information workload.

## Information readouts expose both the gain and the remaining failure

The [new workload](PREFIX-READOUTS.md) publishes original, Possible, Guaranteed
and Ambiguous Events for sixteen groups after actual three-input Free Join.
Its 119 comparison processes retain 352 configurations and 864 matched pairs;
114 complete and five reach a resource cap. Five additional diagnostics locate
the four distinct caps in untimed canonical reimport. All completed outputs
pass the independent legal-world oracle; a capped process supplies no complete
timing comparison.

On the holed half-X/Y bit-major query, five-sample fresh medians are 0.758 ms
for prefix512, 13.885 for minimum512, 1.376 for packed512 and 1.006 for dense.
The prefix path retains 2,926.2 KB versus packed's 5,364.8 KB. This is a useful
benefit from the stronger projection certificate.

The non-suffix control prevents premature selection. On the fibred high-X-bit
query, prefix512 takes 1653.467 ms versus packed's 1.460 ms in a three-sample
repeat. Its readout stage dominates and its retained arena grows to 162,956.4 KB.
The smaller cutoff improves some grouped-layout fallbacks but does not resolve
the broader problem. The [equal-cofactor control](COMPLETION.md) now passes all four shared suites
and 32 native acceptance processes. Its twenty matched policy pairs preserve
all resident statistics, and all four capped configurations still cap under
both modes. Direct ternary ITE is now measured below; support-factor elimination and fused
projection/substitution remain separate candidates.

Fourteen [new Lean reports](INFORMATION-ALGEBRA.md) clarify the algebraic target:
observable bounds, exact FD conditions for moving a filter, and information
order as an FD between observations. That general algebra stays independent of
which physical hierarchy supplies a fast kernel.

## Direct conditionals separate compact answers from expensive construction

[The ternary completion round](TERNARY-COMPLETION.md) retains 160 configurations
and 364 matched comparisons from fifty processes. All four staged cap cases
complete with direct ITE; backtraces keep the staged failures in untimed canonical
reimport. Five shared suites and 32 native ownership/symbolic processes pass.

The three-sample fibred high-X-bit bit-major median falls from 1643.748 to
377.951 ms, while packed takes 1.486 ms in the same executable. Retained query
bytes fall from 162,956,364 to 83,850,572. The corresponding face-major query
falls from 14.269 to 9.062 ms; packed still leads at 0.564 ms. Full-support
storage is identical across sixteen policy pairs. Suffix controls retain their
semantic behavior, and the four cap resolutions are completion evidence from
one-sample screens, not fine performance ranks.

The graph census explains why another representation is not yet forced by this
failure: the 64 answers share only 343 records, each has at most 24, and all
inputs/outputs/owner infrastructure together reach 1,163. Staged retains
1,363,847 records; ITE retains 708,405. These counts do not measure reclaimable
bytes or a garbage collector. Actual ARM64 contains four vector bit-selects in
the local ITE loop; recursive construction remains the dominant problem.

Six new Lean reports validate the pure conditional/substitution rules. Six
more prove [support-factor elimination](PROJECTION-GATES.md): a membership FD
can remove a support gate exactly when joint witness coverage holds. Product
support with a retained environment supplies that coverage. This separate kernel
is measured in the following round.

## Dependencies remove support work before tuning the representation

[Dependency-directed gates](PROJECTION-GATES.md) preserve exact joint witnesses
while dropping factors justified by membership FDs and product coverage. Across
74 passing processes, the round retains 264 configurations and 816 comparisons.
Five shared suites and 32 native ownership/symbolic acceptance processes pass.
Eighty-four full-support/prefix-suffix pairs preserve all resident statistics.

The same-ITE, three-sample fibred high-X-bit bit-major query falls from 392.599 ms
with joint gates to 12.402 ms with active gates and 10.758 ms with witness gates.
Packed and dense take 1.467 and 1.446 ms. Face-major falls from 8.584 to 7.346
and 5.985 ms; packed takes 0.595 ms. Every policy starts with the same input
storage. Bit-major final nodes fall from 708,405 to 37,611 and 35,380, with
estimated bytes falling from 83,850,636 to 4,327,844. Output roots still share
343 records. The extra prepared gate adds one live infrastructure record.

A staged-normalizer control retains the large improvement, so the dependency
rewrite is the main lever here. This certificate concerns copies of an entire
legal game state, allowing card correlations within that state. It introduces
no probability independence. Coupled support and hidden environments retain
the general path. The following [local-completion round](LOCAL-COMPLETION.md) tests whether
already completed faces can survive projection unchanged.

## Completion locality and traversal each remove substantial work

[The local-completion comparison](LOCAL-COMPLETION.md) retains 368 configurations
and 1,472 matched pairs from 110 passing processes. All 32 native acceptance
processes pass, and 152 storage-identity pairs verify the direct/fallback
controls. Four shared suites compare
all gate/completion policies and exact canonical IDs against independent legal
worlds. Eight new Lean reports prove separate decoder invariance, preservation
through projection and exact local completion, including repaired environments.
No probability independence is inferred.

With the same witness gates and ITE, three-sample bit-major medians are 10.659 ms
for full repair, 3.373 for selective source traversal, 3.366 for local repair and
2.247 for both. Packed takes 1.587 and dense 1.495 ms. Face-major gives 6.006,
3.634, 3.120 and 2.655 ms, against packed's 0.638 and dense's 1.438 ms. The new
rule changes neither constructor inputs nor the canonical output functions.

Bit-major readout time falls from 9.548 to 1.123 ms; grouping remains near 1.0 ms
and counting near 0.13 ms. Retained records fall from 35,380 to 22,748, starting
from the same 19,801 input records. Estimated query bytes fall from 4,327,876
to 3,354,428. All four graph censuses preserve 343 answer records, 431
infrastructure records and 1,164 records in their union with inputs. This is
construction savings, not a smaller answer or a garbage collector.

The subsequent [direct-construction experiment](FUSED-PROJECTION.md) now passes
five shared suites, 81 comparative processes, sixteen acceptance processes and
two graph diagnostics. Its 256 configurations and 856 matched comparisons keep
gate, completion and constructor controls independent. Three-sample width-six
bit-major medians favor staging: 2.229 versus 2.711 ms; face-major gives 2.964
versus 3.286 ms. Packed takes 1.531 and 0.585 ms in that same executable. A
single-sample apparent exception with the other decoder fails to survive its
own repeat. Both schedules preserve the 343-record answer graph. Canonical
identity permits alternative execution schedules; algebraic fusion does not
imply a runtime gain.

## Compact essential storage: a measured tradeoff

The [slab control](SLABS.md) preserves exactly the same canonical normal form
and operation algorithms while replacing duplicated enum/key payloads with
sixteen-byte records, one table-word slab and collision-checked fingerprint
chains. Both layouts pass the full semantic suite, including forced hash
collisions, vector growth, cloning and exact resident-ID comparisons.

All 105 native processes pass: 506 cases and 146 matched layout pairs have the
same logical record/table/word counts. In the nine-sample repeat, essential512's
owned 18-coordinate program falls from 495.0 to 302.3 KB of carrier storage,
while its time is 2.414 versus 2.390 ms. Its full 60-coordinate counter program
falls from 770.0 to 497.6 KB, with 3.904 versus 3.881 ms. Ordinal Coup falls from
5,078.2 to 3,618.1 KB of carrier storage, with 28.189 versus 28.094 ms. These
estimates exclude external query memos; the generated tables keep the lane's
existing total accounting for matched comparison with controls.

Smaller storage is not a universal latency improvement. An eleven-sample
classifier repeat gives 91.471 versus 106.768 ms for uncached ordinal clover
inclusion, despite reducing its read-only carrier from 108,739 to 60,855 bytes.
The actual ARM64 decoder reconstructs compact table views, and the scalar
classifier repeatedly calls it for individual assignments. The subsequent
[borrowed-word control](WORD-CLASSIFIER.md) tests a replacement for that consumer
with both stores in the same executable. The old assembly identifies avoidable
work, not a proved attribution of the entire regression.

Dense and fixed-tail controls remain ahead on these fresh queries; the enum
layout stays available and remains the default. The [matched tables](SLAB-MEASUREMENTS.md)
retain negative results, separate phase timings and both memory definitions.
This control improves our knowledge of the representation; it does not select
an adaptive backend or change the Event algebra.

## What actually competed

| Rust candidate | Canonical stored representative | Complement | Main tradeoff |
| --- | --- | --- | --- |
| `dense` | Interned `Vec<u64>`, canonical orientation of complementary subsets | Flip one bit | Streaming work over every represented word |
| `dense-dispatched` | Same dense representation, with operator selection outside the word loop | Flip one bit | Isolates kernel dispatch/vectorization from representation choice |
| `sparse` | Interned sorted `Vec<u32>` | Flip one bit | Compact small sets; merges become expensive as sets grow |
| `roaring` | Interned Roaring bitmap | Flip one bit | Compressed containers; cardinality/distribution and interning matter |
| `runs` | Interned disjoint half-open interval runs | Flip one bit | Exploits contiguity; fragmented regions cost many endpoints |
| `bdd-root` | One canonical support-masked root | Construct or reuse `S & !root` | Less eager work; first complement can be expensive |
| `bdd-pair` | Interned pair of complementary support-masked roots | Flip one bit | Immediate access to either root; pair and graph overhead |
| `mdd4-pair` | Paired four-way decision diagram | Flip one bit | Fewer levels when two-bit grouping helps |
| `bdd-anchored` | One canonical root excluding a fixed legal world, plus polarity | Flip one bit | Avoids the pair table; projection can require normalization |
| `packed64` | Anchored ordered prefix with 64-bit truth-table terminals | Flip one bit | Word operations at leaves; fixed decomposition and rebuilds under maps |
| `packed256` | Same construction with 256-bit terminals | Flip one bit | Fewer prefix levels, larger tables; a separate canonical manager |
| `packed512` | Same construction with 512-bit terminals | Flip one bit | Nine tail coordinates can preserve three XYZ coordinate orbits under a suitable order |
| `packed4096` | Same construction with 4,096-bit terminals | Flip one bit | Larger word blocks; less recursion versus larger payloads and more local work |
| `block64` | Ordered six-coordinate blocks with `u64` selector partitions and shared continuations | Flip one bit | Compact repeated continuations; partial abstraction and selector cross-products can be expensive |
| `essential64` | Ordered branches ending in normalized tables over at most six exact essential coordinates, anywhere in the presentation | Flip one bit | Flexible local axes; normalization, alignment and metadata costs |
| `essential512` | The same normal form with at most nine essential coordinates | Flip one bit | More word work per table; maps preserve table width but ancestor rebuilding remains necessary |

These are canonical region arenas, not unverified expression hashes. Every
finite carrier resolves hash collisions with full content equality and uses a
unique canonical orientation. Within a scope, equal handles mean equal events.
None adds per-row weights or changes the public `event` / `Event` semantics.

The diagram kernels are experimental Rust implementations sharing one algorithm,
not tuned CUDD or state-of-the-art SDD implementations. Four-way grouping now
admits arbitrary pairs of binary coordinates with checked padding for odd widths.
It is not native fifteen-card branching.

## Essential-coordinate tables: constructive-query comparison

The [new normal form](ESSENTIAL.md) removes a global prefix/tail boundary by
choosing table axes from each function's exact Boolean dependence. It is
canonical for a fixed order and table limit. The full shared suite now covers
sixteen carriers, 1,024 finite transfers, 121 forty-coordinate transfers and
eleven sixty-coordinate relation-law checks. Missing axes are smoothed into
their original parameter groups during observation; they are not deleted draws.

The [first native sweep](ESSENTIAL-BASELINE.md) passes all 33 processes and 163
measured cases. Essential512's word path takes 35.336 ms on the fresh Coup clover
query, compared with 3.327 ms for packed512 and 0.668 ms for dispatched dense.
On the eighteen-coordinate owned relation query it takes 2.681 ms versus
packed512's 0.751 ms. Full million-state counter algebra takes 5.742 ms versus
packed64's 0.828 ms. All are five-sample medians from one binary, with different
phase boundaries explicitly retained in the tables. These results reject an
immediate promotion of flexible essential tables to the preferred carrier.

Matched scalar/word algorithms show that implementation matters: essential512's
scalar Coup query takes 410.528 ms. Actual ARM64 contains NEON table operations,
yet the manager still loses. A sampling diagnostic finds substantial allocation,
hash-table work and repeated counts. The `derived` path exploits exact
constructor dependence, complemented-operation symmetry and cached structural
cardinality. The [new same-binary sweep and focused repeat](ESSENTIAL-MEASUREMENTS.md)
pass all 48 processes. In the eleven-sample counter repeat, essential64 improves
from 5.756 to 3.788 ms and essential512 from 6.006 to 3.795 ms; their retained
arenas fall by about one quarter despite the extra count storage. Packed64
still takes 0.757 ms. Derived essential512's fresh Coup query takes 30.095 ms,
against packed512's 3.255 ms and dense's 0.693 ms. Warm replay is close to the
join floor and does not reverse the fresh result.

These results retain the useful derivations without selecting the essential
manager. The subsequent [compact storage control](SLABS.md) now measures removal of
duplicate payloads, finding lower retained bytes but mixed latency. Temporary
alignment vectors and owned-node copies remain separate allocation controls.
Structural count never substitutes for joint-law observation. Timing changes
between executables are not credited to the derived path; only matched modes
in the current binary establish its gains.

## The Free Join path is real

```text
ordinary relation rows containing [key, key, scope, region]
  → real canonical image cache
  → binary2fj / factor / validate
  → real COLT and Executor
  → candidate Event operations on complete bindings
  → grouped canonical Event results and exact cardinalities
```

The first query computes `A & !(B | C)` and unions the result by group.
A triangle produces 1,024 bindings; a clover produces 4,096, over 64 groups.
Both use a fixed controlled plan with real lowering, factorization and execution.
The sink forbids suffix skipping and distinct traversal, as required by this
initial complete-binding computed-stage contract.

Each job has one homogeneous scope. The transport extension now restores into
new owned scopes and validates destination scope tokens during native replay.
Its binary support maps do not establish general source/environment alignment.

The input distributions are random, repeated/overlapping, structured bit and
equality predicates, sparse sets, contiguous runs, and legal Coup hands. The
Coup fixture fixes Alice's Duke and Assassin and enumerates all 4,290 remaining
Bob/Cleo deals. The ordinary relation topology is controlled and synthetic;
it is not a benchmark of the entire proposed Coup schema or game engine.

The second native query exercises the larger algebra:

```text
reach   = ∃Y. R(X,Y) ∧ Q(Y,Z)              // relational composition
enabled = ∃Z. reach                       // domain
bad     = ∃Z. reach ∧ !Target(Z)           // a counterexample can occur

Must[group] = union(enabled) & !union(bad)
```

There are 128 matched bindings and 16 groups. A result state must have an enabled
continuation and no counterexample among the grouped alternatives. Direct nested
enumeration checks that definition, including dead states and surviving states.
The result is an X predicate lifted into the original XYZ presentation.
A subsequent lane groups 128 real bindings into 16 relation/constraint/goal
triples, then computes eighty canonical results:

```text
reach       = R*                        // reflexive-transitive closure
backward    = converse(reach)
continuation = R \ Allowed              // greatest Q with R ; Q ⊆ Allowed
may_reach   = May(reach, Goal)
must_next   = domain(R) & !May(R, !Goal)
```

Relations initially occupy the XY face. Composition actually renames another
copy to YZ, eliminates Y, and maps the result back to XY. Converse swaps X/Y;
the residual quantifies counterexamples and reorients its YZ answer. Closure
uses repeated squaring and exact canonical equality to stop. There are 16/64
states per face in the two cases. This finite transition fixture has bounded
components and dead states; it is not a complete Coup transition model.

The oracle uses explicit state-pair matrices, ordinary nested quantifiers and
Floyd-Warshall closure. It checks every exported result outside timing. All
coordinate movement, grouped algebra, interning and eighty exact output counts
are inside the timer. Common memoization includes Boolean operations, products
and permutations. Warm timings replay the same inputs, as in the earlier lanes.

These are lifted XYZ regions: the count observation includes unused ambient
coordinates; it is not a count of distinct endpoint states. Preparing this lane
imports the inputs and identity relation and validates its four fixed maps.
That preparation is recorded as construction work; fresh query timing starts
from that prepared arena, with its outer operation memos empty.

The local permutation constructor checks bounds and bijectivity. The relation
lane uses full Cartesian support, which is preserved by its face permutations.
Constrained-support tests separately check substitution followed by remasking.
The production cross-space Face/CoordinateMap registry, environmental alignment
and general guard maps remain unimplemented.

This is the useful Free Join interaction: the engine matches ordinary bindings
while the Event carrier composes and eliminates internal worlds. A world does
not have to become an ordinary joined row. Free Join itself does not solve
diagram growth or choose a good internal coordinate order.

## Novel query outputs stay in the retained owner

The [owned lane](OWNED.md) removes a limitation of the initial transport replay:
only its 24 input roots are restored, and `Algebra<&mut C>` computes 80 new
result roots in the retained target arena. All fifteen carriers passed
82 source/target/size cases. Every result is checked after dropping the native
executor, inputs and original owner variable. The result batch must then be
the sole owner; dropping it must reclaim that owner.

The [baseline measurements](OWNED-BASELINE.md) exposed preparation as a larger
cost than the packed query. At eighteen coordinates in bit-major order,
packed512 spent 1.246 ms constructing the program and 0.698 ms executing it;
packed64 spent 1.022 and 0.829 ms. The shared culprit was an enumerated identity
bitplane, even for a symbolic carrier. That is a capability boundary as well as
a cost: it cannot scale to an unenumerable presentation.

The [diagonal experiment](DIAGONAL.md) replaces that boundary with coordinate
equalities, retains native/symbolic/table controls, and strengthens full-product
validation. All fifteen carriers pass its 246 matched cases. Packed512's total
owned-query time drops from 2.258 ms with enumerated identity to 0.717 ms with
its symbolic constructor; dense retains the faster direct finite fill. These
are same-binary controls, not comparisons against an earlier compiled revision. Nine carriers pass sixty-coordinate identity, composition, converse,
residual and closure identities without enumeration. All nine symbolic carriers then pass 27 native long-chain cases. At a million
states per face, the full five-result query takes 0.753 ms in packed64 and
0.964 ms in packed512 in the seven-sample sweep. Those are owned-computation
medians, including setup and publication; exact output bytes match an independent
comparator-circuit oracle. [The symbolic tables](SYMBOLIC-RELATION-MEASUREMENTS.md)
keep this structured one-binding capability test separate from finite join
throughput and record its memory costs.

This verifies successful publication and final-owner lifetime. It does not yet
supply transaction rollback, cancellation, public root admission, per-fact wire
identity or within-arena compaction. Those remain distinct acceptance criteria.

## Coordinate layouts and map-preserving tables

The fifteen-candidate extension compares the same two native relational queries
under three internal orders. The mathematical coordinates, inputs, support and
checked outputs stay identical. The [initial layout tables](BLOCK-BASELINE-LAYOUTS.md)
are retained; the [same-binary map comparison](MAP-MEASUREMENTS.md) isolates
packed-table renaming with eleven samples per case and shuffled serial processes.

For the full 18-coordinate relation query, fresh arena and common outer memo,
the initial same-binary eleven-sample comparison gives:

| Candidate | Face-major (ms) | Bit-major (ms) | Pair-major (ms) | Bit-major retained arena (MB) |
| --- | ---: | ---: | ---: | ---: |
| dense-dispatched | 2.461 | 2.377 | 2.371 | 5.824 |
| bdd-anchored | 2.911 | 1.176 | 1.310 | 0.693 |
| mdd4-pair | 3.015 | 1.202 | 1.145 | 0.858 |
| packed64 | 2.299 | 0.807 | 1.019 | 0.539 |
| packed256 | 2.078 | 1.322 | 1.398 | 0.639 |
| packed512 | 2.267 | **0.741** | 2.225 | 0.510 |
| packed4096 | 11.216 | 2.932 | 2.984 | 0.902 |
| block64 | 2.334 | 1.144 | 1.103 | **0.220** |

These packed rows use the local map path when valid, falling back to canonical
recursive rebuilding otherwise. All fifteen carriers participated in the raw
comparison. The table highlights the strongest finite and symbolic contenders;
sparse/Roaring/run results remain in the generated tables. Dense ignores internal
diagram order, so its three processes are repeated controls. These are measured
medians on a working desktop, not formal confidence intervals.

Two distinct effects are now established. Interleaving corresponding coordinates
reduces the work of this relation fixture. Separately, keeping a renamed table
inside its existing coordinate set avoids unnecessary Shannon rebuilding.
In the same binary and bit-major order, packed512 improves from **1.992 ms to
0.741 ms**, with retained arena estimates falling from **0.801 MB to 0.510 MB**.
Packed4096 improves from **8.010 ms to 2.932 ms**, and from **4.113 MB to 0.902 MB**.
Packed64 improves from **1.010 ms to 0.807 ms**. Bigger tables still do not win
this full relation query; packed512 leads that process.

An independent [seventeen-sample focused repeat](results/tail-focused-repeat.json)
gives **0.794 ms** for packed512, **0.809 ms** for packed64, **1.136 ms** for
block64, and **2.519 ms** for dispatched dense, all in bit-major order.
Packed512's recursive control takes **2.032 ms**. That confirms the large map
improvement and the leading packed pair; the roughly two-percent difference
between packed512 and packed64 is not evidence of a universal leaf-width winner.
The combined generated tables give the two processes equal weight where both
measured a case, rather than retaining whichever process happened to be faster.

The [essential-coordinate table candidate](ESSENTIAL.md) specifies the next
representation experiment: a deterministic local table cut based on exact
Boolean dependence. Its independent reference and standalone raw Rust
prototype pass canonicality/operator checks. It is not yet a complete scoped
carrier or a measured native winner.

The [layout-planning derivation](LAYOUT-PLANS.md) explains why nine tail coordinates
are interesting: three complete `{Xi, Yi, Zi}` orbits survive every required
whole-face permutation in bit-major order. The paired-bit layout cuts the
nine-coordinate tail differently and loses that property. The implementation
checks invariance; it never silently drops a general map or changes the answer.
This derives physical opportunities from the algebra's maps, without asserting
probabilistic independence among coordinates.

The new [block64 representation](BLOCKS.md) uses selector partitions throughout
the graph, with a 16-byte branch and 16-byte selector edge. It retains the least
estimated arena memory in this relation comparison. Its initial full sweep also
constructs the interleaved twenty-pair equality event over `2^40` assignments
in 137 branch/leaf records and 15,144 estimated bytes, including selector edges
and caches. It still hits the resource cap under grouped ordering. Its compact
Coup query takes 12.149 ms in that initial sweep, so its symbolic compression
does not establish good finite query performance. A new
[matched fifteen-candidate Coup repeat](results/tail-coup-repeat.json), using
the current binary and eleven samples, gives **0.661 ms** for dispatched dense,
**1.783 ms** for packed4096, **3.265 ms** for packed512, **4.779 ms** for packed256,
and **11.968 ms** for block64. These are fresh, outer-memo-enabled clover queries
over the same 4,290 legal deals. The new representations do not displace dense
for this finite fixture.

The [eleven-sample modal repeat](results/tail-modal-repeat.json) takes **0.397 ms**
for packed512 in bit-major order and **0.415–0.421 ms** for packed4096 in the two
interleaved orders. Treat those close figures as a leading group, not a stable
small-margin ranking; the earlier process reverses their order. Packed512 retains
**0.387 MB** versus roughly **1.11 MB** for packed4096. Block64 takes about
**1.38 ms** while retaining **0.176 MB**. The dense bit-major control takes
**1.531 ms**. These are the same full-product worlds and checked Must results.

Thus partial-block elimination can cost more even when a grouping makes renaming
cheap. Constructor cost, warm replay and nonuniform-law observation cannot be
inferred from fresh query time either. The [question register](QUESTIONS.md)
preserves those distinctions. The [combined layout tables](LAYOUT-MEASUREMENTS.md)
keep both query families and all three orders explicit.

## Earlier controlled baselines

These retained baseline results use a fresh Event arena with the same outer operation
memo enabled. Times include the real join, Event computation, grouped results
and exact counts. The first six rows use 4,096 bindings; Must uses 128.

| Workload | Leading measured candidate in this mode | Median (ms) |
| --- | --- | ---: |
| Random predicates, 4,096 assignments | Dense, dispatched | 0.700 |
| Coup, 4,290 ordinal deals | Dense, dispatched | 0.680 |
| Same Coup deals, card coordinates | Dense, dispatched | 4.071 |
| Structured predicates, 65,536 assignments | Dense, dispatched with outer memo | 4.919 |
| Sparse predicates, 65,536 assignments | Dense, dispatched; Roaring uses less memory | 4.541 |
| Long runs, 65,536 assignments | Interval runs | 0.679 |
| Relational Must, 262,144 XYZ assignments | Packed 256-bit terminals | 0.650 |

The Coup query falls to **0.160 ms** with a retained operation cache;
its join-only baseline is **0.070 ms**. The matched eleven-candidate modal
repeat gives **0.650 ms** for packed256, **0.733 ms** for packed64, **1.226 ms**
for the four-way diagram and **1.653 ms** for dispatched dense. Packed256's
estimated retained arena is 0.379 MB versus 7.165 MB for dense in that case.
The samples use the same binary and eleven trials. They select a leader for
this fixture, not for every relational program.

Structured predicates also illustrate why the memo setting must be named:
without the common outer memo, packed256 takes 8.392 ms versus 13.475 ms for
dispatched dense; with that memo, dense takes 4.919 ms versus packed256's
8.487 ms. Memoization reverses that comparison.
The sparse case favors dispatched dense on time, while Roaring retains much
less region data. These are operation/workload results, not universal rankings.

The compact and product Coup fixtures contain exactly the same legal deals and
input events. The product presentation has 65,536 encodings but only 4,290 legal
ones. Switching coordinate layouts alone did not make the diagrams win this
query. Under that constrained support, anchored BDD takes
**30.35 ms** versus **51.88 ms** for the paired BDD in the focused repeat,
with arena estimates of **20.9 MB** versus **36.4 MB**. The bitmap prototype retains the
whole product bitplane; a compact admissible-world mapping has its own future
projection/translation costs.

The primary tables use the focused repeat for every case that was repeated,
regardless of its speed, plus the final matched dense kernel/control runs.
Random/overlap and the independent symbolic/initial projection lanes retain the
full sweep where no focused repeat exists. Earlier processes and source/build
snapshots remain linked; the pre-shortcut baseline is not mixed into the final
algorithm comparison. Large pauses in some raw samples are retained.

The newest matched controls for dense-dispatched and packed256 supersede their
older Boolean rows, regardless of whether a newer sample was faster.
The complete [measurement tables](MEASUREMENTS.md) separate fresh versus warm
arenas, outer memo on/off, both join shapes, import time and retained storage.
The raw JSON retains every sample rather than only its median.

“Fresh” means the Event arena starts with imported inputs. Its clone is outside
the timer, and COLT is already built. The timed query includes expression work,
new interning, grouping and exact counts. First native join and image/setup
costs are recorded separately. Input predicate generation is outside import
timings; this is not a complete source-construction benchmark.

“Warm” replays the query with retained regions and caches. All candidates get
the same optional outer operation memo; modal queries also memoize products and
abstractions. Diagram Apply always has its own recursive memo. Disabling the
outer memo is not described as disabling all diagram caching.

The resident join/modal/relation lanes intentionally count inside the timed
query. The newer transport lane separates the source relation program from
restoration and excludes count/law observation from that query phase.
In the current prototypes,
it scans dense words, uses finite container cardinality/run lengths, or traverses
a diagram with a fresh count memo. Thus warm differences include observation
cost, not just the cost of looking up an Event operation. Uniform finite counts
do not benchmark a correlated or parameterized probability law. The separate
exact observation lane below now does so for one declared source fragment.

## Earlier packed-terminal comparisons

The [packed construction](PACKED.md) makes small truth tables the terminal objects
of an otherwise symbolic diagram. It retains one exact canonical decomposition
and the anchored complement invariant. This is a concrete attempt to get both
shared symbolic structure and word-sized arithmetic without silently switching
identity systems.

The initial sweeps show a useful asymmetry. Packed256 takes **5.002 ms** on the
compact Coup Boolean/grouping query, versus **0.680 ms** for dispatched dense
in the new same-binary, eleven-trial control runs. Its modal lead therefore does not make it the
finite Coup winner. Packed64 is also slower on this Boolean fixture. Both packed
candidates still hit the two-million-record cap on grouped twenty-pair equality;
packed terminals do not fix a bad global order.

The full relation lane adds coordinate movement, converse, residuals and closure.
Its current tables are generated from `relations-repeat.json`, with the initial
`relations-sweep.json` retained. It is particularly useful because rebuilding
coordinates across a packed leaf boundary is actually charged. The eleven-trial repeat gives:

| Candidate | Fresh with common memo (ms) | Warm with common memo (ms) | Retained arena estimate (MB) |
| --- | ---: | ---: | ---: |
| packed256 | 2.125 | 0.038 | 0.987 |
| packed64 | 2.403 | 0.046 | 1.091 |
| dense-dispatched | 2.458 | 0.050 | 5.824 |
| bdd-root | 2.915 | 0.056 | 1.477 |
| bdd-anchored | 2.970 | 0.054 | 1.477 |
| mdd4-pair | 2.999 | 0.043 | 1.369 |

Packed256 leads in both independent processes, but its margin over dense is
about 14%, much smaller than the modal gap. It retains about one-sixth as much
estimated arena memory in this fixture. Without the common outer memo, the
initial process takes 4.208 ms for packed256 and 7.043 ms for dispatched dense;
recursive diagram caches are still present. Keep the cache mode explicit.

There are 52 total stabilization iterations across the sixteen groups, and
all eighty results are checked against matrices. A different transition family,
coordinate order, leaf boundary or longer-lived arena may change the ranking.
Stationary warm replay can favor containers with cheap cached cardinality even
when their first query is far slower. These figures do not select a universal
execution backend.

These earlier results justified keeping a packed symbolic manager beside the finite dense
contender in the experiment. They do not justify an automatic mixed manager or
free conversion into a mandatory BDD wire form. The
[new paper review](../../research/representation-search.md) explains why canonical
reduction and renamed structural sharing deserve separate experiments.

## A better invariant emerged from the experiment

The two-root pair is not necessary for constant-time relative complement.
The [anchored construction](ANCHOR.md) stores the side of a complementary pair
that excludes one fixed admissible world, plus the selected polarity.

Every normalized representative is false at that world and outside support.
Transforming the four truth bits makes every internal binary operation map
`(false,false)` to false. Its result therefore automatically stays inside support
and satisfies the same anchor invariant. Boolean Apply needs neither a support
mask nor a new pair record. Equality is exact and complement flips one bit.

That is a useful representation improvement even if a given query is not faster.
The candidate has ordinary BDD growth and can recover a support-masked true root
and renormalize after quantification. Its measured Boolean, relational and
complement costs must be assessed together. We do not claim a new algebra or
a historically novel decision-diagram technique.

## Check the actual machine code before ranking a representation

The [disassembly of the original dense loop](results/dense-raw-op-arm64.s)
contains scalar loads and Boolean operator selection inside every word iteration.
It is not the NEON streaming kernel one might assume from the Rust iterator.
`dense-dispatched` has identical storage, interning, normalization and caches;
it switches on the operation once and instantiates a constant word loop.
The [final dispatched operation](results/dense-dispatched-op-arm64.s) contains
NEON `BIC`, `EOR`, `ORR` and `AND` over sixteen-byte lanes with vector loads/stores;
its scalar tail remains explicit. [Assembly metadata](results/assembly.json)
records the binary hashes. This deliberately isolates an avoidable implementation
cost from the data structure. Its timings are retained alongside the original, not substituted
silently into old results.

The new [packed256 Apply disassembly](results/packed256-apply-relations-arm64.s)
contains paired 128-bit `ORR`, `AND`, `BIC` and `EOR` leaf operations. Its
[table-permutation routine](results/packed256-permute-table-relations-arm64.s)
remains recursive and calls canonical selection; it is not a SIMD shuffle over
the whole graph. The [dense coordinate-permutation kernel](results/dense-permutation-relations-arm64.s)
uses broadword axis swaps with NEON in several branches, rather than a per-world
index loop. [New assembly metadata](results/assembly-relations.json) records exact
symbols, binary/source hashes and extraction commands. Instruction counts include
normalization and other work within each symbol; they are not kernel speedups.

The [fifteen-candidate binary's assembly](results/assembly-tail.json) additionally
retains packed4096 Apply, its permutation dispatcher, and block64 Apply/product.
Packed4096 Apply contains NEON Boolean operations; the local permutation branch
calls the same broadword `Permutation::dense` kernel as the dense carrier.
Block64's selector logic uses scalar masks. Its padded temporary edge arrays
also cause substantial scalar zero-initialization sequences, an explicit
unoptimized cost recorded in [BLOCKS.md](BLOCKS.md). No whole-graph SIMD claim or
fully tuned block-kernel claim is made.

This also limits how broadly to interpret a prototype loss. The Roaring library,
handwritten run/sparse routines and shared diagram engine have different levels
of tuning. Assembly inspection can falsify a supposed representation bottleneck;
it cannot establish that every remaining kernel has reached its best performance.

## Symbolic size is a separate contest

The equality constraint on two twenty-bit words lives over 2^40 assignments:
1,099,511,627,776 possibilities. An interleaved BDD presentation completes with
631 BDD arena nodes, including construction intermediates. Packed64 uses 624
records and packed256 616 in the same interleaved construction; these record
counts have different byte costs. One explicit bitmap
over that domain alone would require 128 GiB, larger than this machine's RAM.

Putting every left coordinate before every right coordinate causes the binary
and grouped four-way prototypes to hit the two-million-node cap at twenty pairs.
These are recorded construction refusals, not successful empty results. The
symbolic timing samples are illustrative single constructions; the strong
evidence here is the size/growth difference, not a microsecond ranking.

This directly challenges a universal fixed-order storage strategy. A compact
execution representation that must normalize into a huge canonical wire BDD
has merely postponed the cost. Persistent identity and translation need their
own measured design. [Knowledge compilation](../../research/algebra-resolution.md)
already distinguishes cheap queries from potentially expensive transformations;
this experiment makes one such failure concrete.

## Exact shared-parameter observation through the same query

The [observation contract](OBSERVATION.md) and [measurement tables](LAW-MEASUREMENTS.md)
add a nonuniform source fragment to all fifteen candidates. Events contract
to exact assignment coefficients by named parameter group, then evaluate under
explicit Beta priors, fixed rational parameters and zero endpoints. The full
Free Join relation program produces 80 canonical output events and 240 exact
conditional observations. Every coefficient and event is checked against an
enumeration oracle; the shared suite independently checks Beta probabilities
against a sequential Pólya urn oracle. Integration preserves shared dependence.

[The matched seven-sample sweep](results/observation-laws.json) passed for all
fifteen candidates. At eighteen outcome coordinates, two parameter groups,
bit-major layout and the common outer memo enabled:

| Candidate | Fresh total, ms | Coefficient contraction, ms | Warm total, ms |
| --- | ---: | ---: | ---: |
| packed512 | 1.297 | 0.421 | 0.545 |
| packed64 | 1.473 | 0.547 | 0.665 |
| block64 | 1.716 | 0.495 | 0.618 |
| bdd-pair | 2.099 | 0.882 | 0.987 |
| packed4096 | 3.576 | 0.408 | 0.538 |
| dense-dispatched | 5.791 | 3.166 | 3.219 |

These are process medians, not confidence intervals or a universal ordering.
Packed512 leads the fresh query in this sweep. Packed4096's quick contraction
does not offset its higher fresh algebra cost; warm totals for the two are
close. Dense's compact-Coup win does not carry over to this structured relational
query. The observation plan uses about 1.15 MB for dense versus about 4 KB for
packed512, separate from Event arenas. Sparse, runs and Roaring currently pay
for dense observation views, so specialized contraction remains a challenger.

The exact answers also differ substantively: one two-group result is `4/21`
under the Beta law versus `13/100` under its plug-in means. Eighty endpoint
observations are undefined despite nonempty evidence. At forty coordinates,
all nine symbolic carriers verify `P(all heads)=1/41` under a shared uniform
Beta parameter, versus `1/2^40` under a fixed fair parameter, without materializing
the world domain. This is additional algebra/observation evidence, not proof of
general parameter-guard solving or native Event persistence.

## Structural transport tests the representation boundary

The [transport experiment](TRANSPORT.md) now exports graph/table views from
all fifteen carriers, checks support-preserving or explicitly restrictive
maps, restores into new owned scopes and reruns the real relation query.
Its 784 finite source/target cases and 81 forty-coordinate symbolic transfers
preserve exact denotations and destination IDs. The nine symbolic exporters
handle the forty-coordinate equality family without enumerating its ambient
`2^40` assignments. Packet sizes there are 1,037–2,043 bytes.

The native lane transports 24 input and 80 output roots for the full Free Join
relation program. It compares every restored Event with explicit bitsets, and
complete common-destination bytes with independently constructed bytes. This
tests owned rebasing and continued algebraic use. The original transport replay
uses a carrier clone with expected roots already restored. The separate
[owned lane](OWNED.md) now tests novel result publication in the actual owner.
Native Event persistence and law descriptors remain production work.

The [initial baseline](TRANSPORT-BASELINE.md) exposed an algorithmic trap: rebuilding
full tables through repeated Boolean ITEs made all five finite-source processes
hit the 60-second process limit. Direct table constructors let every candidate
complete the [seven-sample sweep](results/transfer-direct-sweep.json). The
[same-binary dense control](results/transfer-dense-control.json), at eighteen
coordinates and the bit-major destination, changes checked dense-dispatched
restore from **5,258.641 ms to 0.902 ms**. It returns identical canonical results.
Conversion into packed512 still takes **49.046 ms** in that control because its
table builder gathers individual cells into the physical layout. This motivates
the separate whole-word construction control; it is not evidence of an intrinsic
50 ms lower bound for a symbolic wire representation.

The next [matched dense control](results/transfer-word-dense-control.json) keeps
all three paths in one binary. On the same eighteen-coordinate batch:

| Source → target, bit-major | Recursive restore, ms | Direct-table restore, ms | Word-table restore, ms |
| --- | ---: | ---: | ---: |
| dense-dispatched → dense-dispatched | 5,326.504 | 0.962 | 0.851 |
| dense-dispatched → packed512 | 75.097 | 49.515 | 2.386 |
| packed512 → packed512 | 0.846 | 0.550 | 0.174 |

The first two rows have three samples per mode. The packed row comes from the
[eleven-sample packed control](results/transfer-word-packed-control.json).
Dense uses the same direct constructor in `direct` and `words`; its small
difference is process variation, not another algorithmic improvement. Packed
full-table import permutes axes once, then builds the graph from whole word
blocks. Its same-order local tables also use broadcast/permutation instead of
per-cell gathering. Changing packed512 to face-major still costs about
**0.87–0.91 ms** across these modes: crossing the prefix/tail boundary keeps the
generic fallback. This is a concrete remaining conversion problem, not a reason
to weaken coordinate maps.

All fifteen sources pass the [new seven-sample sweep](results/transfer-word-sweep.json):
82 verified transport cases, with identical common packed512 destination bytes
for all fifteen sources at both sizes. That batch is **25,379 bytes** at eighteen
coordinates. The sweep uses the fixture's declared unconstrained binary-product
constructor. Packed512 setup there is about **0.003 ms**, versus **1.146 ms** when
rebuilding the same full support from fixture bitsets in the matched controls.
These are separate same-binary processes. A full fibre product of constrained
endpoint spaces still has to retain their support; this constructor is not a
general arbitrary-product solver.

[Actual ARM64 extraction](results/assembly-transfer-words.json) shows paired
128-bit loads/stores in whole-leaf construction and NEON `EOR.16B`/`AND.16B`
in the exact bit-axis permutation. The extraction records 79 static vector
Boolean instruction sites in that permutation routine, not a dynamic query
instruction count. Canonical interning and recursive control flow remain.

[The generated phase tables](TRANSPORT-MEASUREMENTS.md) keep source query,
export/encode, decode, target construction and checked restore separate. The
source query excludes count/law observation. Setup from a known full-product
descriptor and setup from fixture bitsets also remain separate experiments.
Canonical batch bytes do not by themselves define canonical individual facts.

The deeper result is algebraic. The [map review](../../research/space-maps.md)
separates faithful Boolean lift, quantifier base change and law preservation.
A copied coordinate is a counterexample to treating all three as one guarantee.
It also derives a map as a total functional relation graph and information
equivalence as the graph composed with its converse. The
[dependency-query derivation](DEPENDENCIES.md) shows that coverage/conflict
summaries are factorized queries returning exact obstruction Events. These are
shared semantic contracts for admission, queries and source construction.

## What this changes in the proposal

1. Preserve exact scoped regions, one-bit complement as the preferred contract,
   relational operations and optional joint laws. Do not identify that contract
   with two stored BDD roots.
2. Let finite Coup execution use a finite canonical manager in the next prototype.
   Do not require every bitmap result to round-trip through a BDD just to get
   resident equality.
3. Keep diagram and run candidates alive where their structure wins. Select a
   representation for a sealed arena before attempting automatic mixing.
4. Treat representation, coordinate layout, operation memoization and output
   observation as separate experimental choices. Density alone cannot choose all
   four. Ordinary join order and internal elimination order are distinct.
5. Charge conversion, admission, updates and persistence before claiming a
   production winner. The lab now measures checked structural conversion and
   limited ownership/replay as well as queries. Native persistent values,
   descriptor resolution and long-lived admission costs remain to establish.
6. Keep map proofs specific: total functionality for Boolean substitution,
   complete fibres for projection rewrites, and pushforward equality for law
   preservation. A single unchecked alignment flag would weaken the algebra.

## Validation, environment and limits

The [dependency bridge](DEPENDENCIES.md) now checks more than the query sink.
All fifteen carriers pass 5,062 pointwise fact-set cases each, 4,096 summary
triples each and 512 mixed updates each. The separate native run matches all
5,062 cases with the actual interval macro, exact-key validator and complete
judge, validating 20,644 citations against original facts. It preserves full
fact identity through finite interval expansion. This is denotational evidence;
native Event storage, ownership and admission are still production work.

The common semantic test exercises **61,440 Boolean cases per candidate** over
all fifteen nonempty four-world supports, plus equality, relative complement,
existential abstraction, multiword/padding cases, exact count, and fused product.
All fifteen candidates pass. Added checks cover all sixteen Boolean operations
across word/padding boundaries, non-product support, projection across prefix/leaf
boundaries, two extra noncontiguous orders per carrier, three full relation layouts,
and forty-coordinate symbolic permutations/quantification. The Rust relation
checker verifies **57,344 residual adjunction cases** across the fifteen candidates,
alongside direct composition,
converse and closure oracles. The initial odd-coordinate MDD permutation bug was
caught before timing and fixed; its source snapshot, failure log and successful
rerun are retained in [the regression record](results/regressions.json). Both
packed permutation modes pass the same shared modules in the debug checker and
in the optimized native engine. Native queries check full grouped bitsets against
independent pointwise oracles outside timing windows, then check counts/checksums
on timed repetitions. The alternate Coup presentations are also compared
assignment by assignment before performance runs.

Recorded host: Apple M2 Max, ARM64, 96 GiB RAM; pinned
`nightly-2026-08-15`, rustc `1.99.0-nightly (d453bdd8f 2026-08-14)`;
release optimization, fat LTO, one codegen unit. No allocation-counter
instrumentation and no explicit `target-cpu=native` override. The real engine
snapshot is `d76d31abca00226bc817a549d7365f1638f4b33c`; source hashes, compiler
metadata and binary hashes accompany the results.

Runs are serial subprocesses with shuffled order and seven timed samples per
normal case. The machine also runs other development work: unrelated builds
were observed during some sweeps. Raw samples include visible scheduling pauses.
Repeated runs are retained, and close results are treated conservatively.
These measurements are not a controlled machine-wide benchmark or formal
confidence intervals. Later focused runs also record host load averages.

Memory columns estimate candidate arenas and caches; they omit some capacity
slack and allocator overhead. Per-process peak RSS also includes the native
engine, oracle, inputs and cloned arenas. All arenas retain intermediates. Node/region caps are not total memory caps;
operation caches and container sizes also contribute.
Caps are one million finite regions, two million diagram nodes and a per-job
timeout. No timeout or resource refusal can become a mathematical answer.

Sparse, Roaring and runs currently convert to a dense view for existential-axis
reduction. Their modal results do not rule out specialized projection algorithms.
Formula/AIG/SAT, general SDD/AND-OR/chain/variable-shift representations, variable-width MDD grouping,
dynamic reordering, general correlated-factor law contraction and event-aware join pruning are not
implemented in this experiment. Local bijective coordinate maps, residuals and finite closure are now exercised;
their full typed production integration is still a separate task. The [question register](QUESTIONS.md) gives
concrete acceptance experiments for those remaining boundaries.

The production engine is unchanged. The runnable laboratory, disposable engine
copy, source snapshots, raw results and reproduction instructions are under
[`event-repr-lab`](README.md). The existing proposal's FD/IND, source-law and
world-relation semantics remain intact.
