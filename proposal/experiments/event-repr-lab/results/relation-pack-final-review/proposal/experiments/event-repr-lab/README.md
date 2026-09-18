# Temporary Event representation laboratory

This experiment reopens the representation choice. Preserve the Event semantics;
let measured workload performance decide which implementations deserve to survive.
Nothing here adds an Event type to the production engine.

The latest completed [readout round](results/readout-final-audit.json) is frozen.
The [equal-cofactor normalization control](COMPLETION.md) is also complete:
all checks pass, but it saves no retained storage and does not repair the caps.
[Direct ternary ITE and answer-root reachability](TERNARY-COMPLETION.md) are now
measured: all four former cap cases complete, and the difficult readout improves
4.35-fold. Small answer graphs still sit in large arenas.
[Dependency-directed support gates](PROJECTION-GATES.md) now pass 74 comparative
processes: the repeated difficult query falls from 392.599 to 10.758 ms, while
packed takes 1.467 ms. All 32 ownership/symbolic acceptance processes pass.
[Local completion](LOCAL-COMPLETION.md) now passes 110 comparative processes;
the repeated difficult query falls from 10.659 to 2.247 ms, versus packed's
1.587 ms. Eight further Lean reports establish the locality rule, including
repaired environments. [Direct completed projection](FUSED-PROJECTION.md)
now passes five shared suites, 81 comparative processes and sixteen native
acceptance processes. Repeated measurements favor staged local completion;
its direct competitor preserves the same canonical answers but takes longer.

The [current design review](../../design-review.md) connects those findings to
the memory layout and public algebra. The [storage-closure proof](../../event-storage.md)
also resolves the earlier nonempty-field restriction and identifies the precise
nonemptiness premise needed by the planner.

Read [the findings](REPORT.md), [generated measurements](MEASUREMENTS.md), and
[the question register](QUESTIONS.md). All timings have retained raw samples.
The [layout tables](LAYOUT-MEASUREMENTS.md) compare three orders of the same
queries; the [map-control tables](MAP-MEASUREMENTS.md) isolate packed renaming.

`prepare.py` creates a disposable engine copy in `.scratch/`. It uses the real
canonical image builder, COLT, Free Join plan lowering/validation, and executor.
A test-only sink evaluates candidate event operations on complete bindings.
The only engine-copy changes are a test module and lab dependencies; the source
manifest records hashes of the original code. Removing `.scratch/` removes all
copied source and build products; scripts, candidate code and results remain.

Initial competitors: dense bitmap, complemented sorted sparse set, complemented
Roaring bitmap, complemented interval runs, support-masked BDD root, paired BDD
roots, and a paired four-way decision diagram. An eighth candidate emerged
during the experiment: the [anchored single-root BDD](ANCHOR.md). A ninth benchmark
candidate, `dense-dispatched`, keeps exactly the dense representation and selects
its Boolean word kernel outside the loop; it isolates an assembly-level confound.
Four [packed-terminal diagrams](PACKED.md), `packed64`, `packed256`, `packed512`
and `packed4096`, test different truth-table sizes beneath symbolic prefixes.
The fourteenth candidate, [block64](BLOCKS.md), places word-sized selector
partitions throughout the diagram. All finite lanes use exactly the
same admissible worlds and input predicates, including empty/full/complements.

Measure constructor cost, actual Free Join + expression + grouped union,
repeated queries with and without a common operator memo, relational elimination,
and output observation. Keep the ordinary join baseline and memory estimates.
Fresh runs begin with input-only arenas; repeated runs retain learned nodes and
caches. Result verification is outside timed windows and uses a direct bitset
oracle. The special symbolic scaling lane avoids materializing all worlds.

This is an intuition-building experiment. Finite-world benchmarks do not settle
general semialgebraic source solving, persistence, incremental admission,
concurrency, or forecast calibration. A missing capability is recorded rather
than compared as though it were a faster implementation of the same operation.
The two Coup presentations describe the same 4,290 legal deals: compact ordinal
indices and four four-bit card coordinates with explicit admissibility. Raw
`worlds` records presentation size; `admissible_worlds` records support cardinality
in the extended fixture runner.


The modal lane also runs through actual Free Join. It composes two transition
relations, hides their middle coordinate, computes domain and bad preimages,
and constructs a nonvacuous universal precondition (`Must`). Its oracle directly
enumerates transitions. The complement lane measures explicit relative negation
with random constrained support, including its import and first-use costs.

## Reproduce

From the repository root:

```sh
# Copies the four production crates into a disposable workspace and records hashes.
python3 proposal/experiments/event-repr-lab/prepare.py

# Optimized native engine test binary; correctness then serial benchmark processes.
python3 proposal/experiments/event-repr-lab/run.py --output my-sweep.json

# Reuse that binary; a changed Rust source or engine snapshot is rejected.
python3 proposal/experiments/event-repr-lab/run.py --no-build --lane join --candidate dense --scenario coup_4290 --output my-coup.json
python3 proposal/experiments/event-repr-lab/run.py --no-build --lane modal --output my-modal.json
python3 proposal/experiments/event-repr-lab/run.py --no-build --lane relations --layout all --output my-layouts.json
python3 proposal/experiments/event-repr-lab/run.py --no-build --lane laws --layout bit-major --output my-laws.json
python3 proposal/experiments/event-repr-lab/laws.py my-laws.json --output MY-LAWS.md
python3 proposal/experiments/event-repr-lab/run.py --no-build --lane transport --layout bit-major --output my-transport.json
python3 proposal/experiments/event-repr-lab/transfers.py my-transport.json --output MY-TRANSPORT.md
python3 proposal/experiments/event-repr-lab/analyze.py my-sweep.json --output MY-MEASUREMENTS.md

# Same semantic modules, small debug crate; correctness only, no native timings.
python3 proposal/experiments/event-repr-lab/check.py --output my-check.json

# Two packed renaming algorithms in the same binary, shuffled serial jobs.
python3 proposal/experiments/event-repr-lab/map_sweep.py --output my-map-controls.json
python3 proposal/experiments/event-repr-lab/layouts.py my-map-controls.json --output MY-LAYOUTS.md
```

Use a new output name for each run: its log directory is deliberately exclusive.
`--seed` changes process order; `--trials` sets samples; `--timeout` sets the
per-process wall-clock limit. `--verify-only` runs the common semantic checks;
`--build-only` prepares the binary without starting any benchmarks.
The experiment also has `elimination`, `symbolic`, `complement`, and `relations` lanes.
The `relations` lane includes coordinate permutations, converse, residuals,
finite closure, eventual May, and one-step Must after the real join/grouping stage.
The `laws` lane applies that program and then exact evidence-conditioned
observations under shared Beta parameters, fixed parameters and zero endpoints;
see [its source and arithmetic contract](OBSERVATION.md) and [measurements](LAW-MEASUREMENTS.md).
`--layout all` compares face-major, bit-major and paired-bit orders. `--packed-map
recursive` selects the original packed-table rebuilding path; `local` uses a
table-axis permutation when the tail's coordinate set is preserved. These are
two algorithms over the same representation, selectable in the same binary.
Only one timing sweep should run at once, with no concurrent compilation.

The `transport` lane exports the full relation program's inputs and outputs,
restores them into new scopes/layouts, and reruns the native query using actual
destination scope tokens. Export, decode, scope setup and checked restore are
timed separately. It also tests packed512/bit-major as a common destination.
[The transport contract](TRANSPORT.md) distinguishes structural packets from
canonical fact bytes, and support maps from quantifier/law certificates.
`--transfer-import all` compares recursive, direct-table and word-permutation
import in the same executable; `both` retains the earlier direct/recursive
pair. `--transfer-setup product` uses the declared full-product constructor;
`fixture` retains support construction from fixture bitsets. Keep setup mode
separate when comparing complete transfer times.

The `owned` lane imports only inputs and publishes new query outputs in a
mutable borrow of the destination owner; [the contract](OWNED.md) and
[retained baseline](OWNED-BASELINE.md) document the tested lifetime path.
`--identity all` compares native, symbolic and enumerated diagonal constructors
in the same executable. The `symbolic-relations` lane runs the complete relation
program on non-wrapping counters with up to a million states per face, without
world-table construction. See [the diagonal experiment](DIAGONAL.md).

Two [essential-table carriers](ESSENTIAL.md) bring the matrix to sixteen.
Their leaf cut follows exact Boolean dependence, including scattered coordinates.
The [native baseline](ESSENTIAL-BASELINE.md) compares scalar/word algorithms
without merging their timings. `essential_sweep.py` runs matched serial jobs;
`essential_comparison.py` retains the algorithm and phase boundaries.
`--essential-kernel derived` selects constructor certificates, normalized
operation memos and cached structural cardinalities. Its
[matched sweep and focused repeat](ESSENTIAL-MEASUREMENTS.md) improve time and
retained bytes but leave the existing controls ahead on fresh queries. Scalar
and word controls remain available in the same implementation.
`essential_prototype_check.py` verifies the shared raw normal form separately.

The [compact storage experiment](SLABS.md) compares original enum/key storage
with sixteen-byte records, one table-word slab and exact fingerprint chains.
`--essential-layout enum|slab|both` selects matched storage in `run.py`.
`slab_sweep.py` includes both stores and same-binary fixed-tail/dense controls;
`slab_comparison.py` requires unchanged logical counts and retains split memory
estimates. Legacy table generators reject slab results rather than silently
merging the two stores. Shared checkers accept `--essential-layout enum|slab`.

```sh
python3 proposal/experiments/event-repr-lab/slab_sweep.py --output my-slabs.json
python3 proposal/experiments/event-repr-lab/slab_comparison.py my-slabs.json --output MY-SLABS.md --record my-slab-comparison.json
```

The [direct classifier experiment](CLASSIFY.md) reads support-aware relationship
signatures without constructing four Events. Eight carriers have direct kernels;
the remaining carriers supply the constructive control. `classify_check.py`
retains raw-arena checks; the shared checker adds scoped ownership and symbolic
checks. `classify_sweep.py` compares actual Free Join computed-stage filtering,
including owner locking, input validation and predicate selectivity. Direct
queries must leave their actual retained arena unchanged. This is distinct
from native planner pushdown and batched residual kernels.

The [borrowed-word control](WORD-CLASSIFIER.md) keeps that exact readout and
traversal while replacing local assignment enumeration with aligned word planes.
`--occupancy-kernel scalar|words|both` selects the classifier independently of
the constructive `--essential-kernel`. `words_sweep.py` compares both stores,
both classifiers, materialized cells and fixed-tail/dense controls. Its ordered
cut fixture forbids signature 15, adding queries that must prove missing cells.
Matching planes are borrowed; alignment currently allocates temporary vectors.
The [retained measurements](WORD-CLASSIFIER-MEASUREMENTS.md) cover 96 passing
processes, 576 cases and 144 matched pairs, with independently shuffled repeats.
The gains do not select the essential carrier; dense and packed controls still
lead several fresh queries. Actual ARM64 is retained for both classifier modes.

```sh
python3 proposal/experiments/event-repr-lab/words_sweep.py --output my-words.json
python3 proposal/experiments/event-repr-lab/words_comparison.py my-words.json --output MY-WORDS.md --record my-words-comparison.json
```

The [residual-normalization experiment](REUSE.md) separates canonical source
cofactors, bounded aligned-plane reuse and output working order. All use the
same canonical resident Event form. Source normalization changes traversal and
retains cofactor nodes; plane caching must leave canonical insertion and final
arena bytes unchanged. The native comparisons keep both full relation programs
and isolated products, with packed/dense controls.

```sh
python3 proposal/experiments/event-repr-lab/reuse_sweep.py --output my-reuse.json
python3 proposal/experiments/event-repr-lab/reuse_comparison.py my-reuse.json --output MY-REUSE.md --record my-reuse-comparison.json
```

The [scoped contraction](SCOPED-PRODUCT.md) preserves every intermediate support
restriction while fusing mapped operands. Its `scoped-products` lane returns
sixteen grouped Events on full, legal-product, copied-face and asymmetric
supports. It checks each fresh and warm output pointwise outside timing. The
stronger [legal relation roles](LEGAL-RELATIONS.md) need separate workspace and
membership-dependency certificates; the lower kernel does not assume them.

```sh
python3 proposal/experiments/event-repr-lab/scoped_sweep.py --output my-scoped.json
python3 proposal/experiments/event-repr-lab/scoped_comparison.py my-scoped.json --output MY-SCOPED.md --record my-scoped-comparison.json
```

`--support`, `--storage`, `--candidate`, `--layout`, `--normalize` and `--reuse`
select matched controls in this sweep. Preserve the distinction from the
eighty-output relation program. The standalone `run.py --lane scoped-products`
also honors `--product`; the default support is `legal-product`.

The [finite relationship calculus](SIGNATURE-CALCULUS.md) derives all pair
predicates and a composition envelope from support occupancy. Its checked
counterexample proves that the fifteen-class table is not strong composition
for finite Event scopes. `signature_calculus.py` retains the table and exact
finite checks; it is not a native performance benchmark.

[Lean proofs](LEAN.md) now establish the signature's exact information boundary
and symmetries, a finite-scope composition counterexample, and the anchored
representation's identity/complement/Boolean-Apply laws. They also establish
essential-coordinate minimality, exact projection and raw quantification, plus
the exact support-image condition for preserving possibility and pair readouts.
The base-change proofs additionally establish the complete-fibre condition for
existential, universal and nonvacuous rewrites, without demanding unique lifts.
Borrowed-product proofs establish contraction through the joint image of two
views and show why separate marginal coverage or root-only caches lose meaning.
Cofactor proofs show why canonical residuals can remove additional coordinates;
supported-dependence proofs show why a unique least logical mask need not exist
on coupled support.
Legal-relation proofs connect input roles to supported functional dependencies,
establish laws on legal domains indexed by a retained environment, and prove
why role and support certificates cannot replace one another. A further scoped
map theorem gives sufficient conditions for removing the extra support gates.
`verify_lean.py` checks 195 central theorem reports. It uses
the locally pinned installed Lean toolchain, checks reported theorem axioms and
retains hashes. These are denotational proofs; the Rust implementation remains
covered by separate differential and native checks. Do not compile Lean during
a timing sweep.

The isolated [difference-basis screen](DIFFERENTIAL.md) compares Shannon,
positive/negative Davio and a fixed mixed schedule under two Apply kernels.
It preserves both Coup's preference for Shannon and bilinear parity's much
smaller Davio output graphs. The [counting extension](COUNTING.md) adds direct
sparse-polynomial construction, exact raw uniform counts, and a circuit-based
counterexample to inferring cheap observation from a small input graph.
These are structural screens with independent oracles, not native Free Join
timings. Eighteen new Lean reports cover the basis equations, arbitrary-width
coefficient trees, and the character-sum identities behind the counting review.

The subsequent [inline unary-chain screen](UNARY-CHAINS.md) implements a competing
sixteen-byte Shannon record with up to ten packed letters. It passes 24 matched
structural processes, post-collection operations, and 12.6 million Boolean
comparisons. Nine additional Lean reports establish label composition and
uniform-count laws. Collected graphs shrink, but one-bit kernels rebuild the
same retained intermediate counts; a block-consuming kernel remains the next
discriminating test.

The [fixed-decoder experiment](FIBRE-RETRACTION.md) adds minimum-repair and
prefix-preserving policies at both local cutoffs. Original support stays separate
from completed Event roots. Boolean operations, certified maps/projections,
read-only count/law contraction and cross-owner transport keep their distinct
contracts. Seven prefix-construction reports prove suffix fibre coverage and
exact reflection of prefix membership FDs. The shared Rust checks additionally
compare every nonempty three-bit domain with an independent repair algorithm.

```sh
python3 proposal/experiments/event-repr-lab/prefix_sweep.py --candidate prefix512 retraction512 essential512 packed512 dense-dispatched --storage slab --layout bit-major face-major --domain below holes fibred --mode views --gates certified --trials 5 --output my-prefix.json
python3 proposal/experiments/event-repr-lab/prefix_comparison.py my-prefix.json --output MY-PREFIX.md --record my-prefix-comparison.json
```

The native comparison uses the same executable for every policy and control.

The [product-fibre count experiment](FACTOR-COUNTS.md) tests another consequence
of the same membership dependencies: omit an unused whole state face during
counting and restore its exact legal multiplicity. It keeps original support,
resident Events and both count modes identical, with four Lean reports and
four-mode differential Rust checks. Shared-parameter laws keep their own
contraction. The sweep explicitly records `joint` versus `faces` observation;
use its collector so those modes cannot be silently combined.

```sh
python3 proposal/experiments/event-repr-lab/factor_sweep.py --candidate prefix512 retraction512 essential512 packed512 dense-dispatched --storage slab --layout bit-major face-major --domain below holes fibred --mode views --gates certified --trials 5 --output my-factor.json
python3 proposal/experiments/event-repr-lab/factor_comparison.py my-factor.json --output MY-FACTOR.md --record my-factor-comparison.json
```
The [information-readout lane](PREFIX-READOUTS.md) constructs 64 output Events
after real three-input Free Join: original, Possible, Guaranteed and Ambiguous
for each group. It measures suffix, whole-face and non-suffix queries separately,
retains incomplete processes, and checks complete legal-world bitsets outside
timing. The [information laws](INFORMATION-ALGEBRA.md) identify the exact FDs
behind filter movement and information order.

```sh
python3 proposal/experiments/event-repr-lab/readout_sweep.py --candidate prefix512 packed512 dense-dispatched --storage slab --layout bit-major face-major --domain fibred --family suffix-1 non-suffix --faces x --factor faces --trials 3 --output my-readouts.json
python3 proposal/experiments/event-repr-lab/readout_comparison.py my-readouts.json --output MY-READOUTS.md --record my-readout-comparison.json
```

The build uses this repository's pinned Rust toolchain and release profile, and
cached dependencies in offline mode. On the recorded machine, linking the full
engine test binary takes several minutes with sixteen candidates. The runner currently uses macOS
`sysctl` and `/usr/bin/time -l`; port those metadata/RSS calls before Linux use.
No toolchains or global dependencies are installed by these scripts.

## Isolation and limits

`.scratch/` is ignored and disposable; it contains the copied engine and build
products. Deleting it leaves this experiment's code, reports and raw evidence.
The build verifies the original production crate hashes before execution.
The lab module is injected only into the copy, so it can exercise private
Free Join APIs without a production API patch.

Each job owns its process group, which the runner terminates on timeout.
Candidate arenas also refuse growth at one million finite regions or two
million diagram nodes. Resource-cap refusals are retained as such and never
interpreted as empty results. The prototype is single-threaded, retains
intermediate regions, and has no garbage collection or allocation profiler.

`src/carrier.rs` contains the common exact operation contract and matched
constant rules; `finite.rs` and `diagram.rs` contain the competing data
structures; `packed.rs` adds packed-terminal diagrams; `blocks.rs` adds selector
partitions; `native.rs` contains the real engine adapter; `modal.rs` contains
the relational and complement experiments; `relations.rs` exercises coordinate
movement, residuals and fixed points; `relational_core.rs` and
`semantic_checks.rs` contain the shared algebra and differential checks;
`lib.rs` contains fixtures and native routing verification. The interface is experimental, not the public macro
API or the full checked Face/CoordinateMap implementation.

`dependencies.rs` implements a compositional coverage/conflict summary and
update tree. `dependency_native.rs` compares finite Event denotations with
the actual interval macro/validator/judge. This bridge checks dependency
semantics; it does not implement native Event fields. [The source review](DEPENDENCIES.md)
keeps that boundary explicit.

`transfer.rs` defines structural packets, support checks and retained batches;
`transfer_checks.rs` shares finite/symbolic cross-carrier checks with the small
checker; `transfer_bench.rs` adds native query replay and conversion timings.


To regenerate the retained primary tables, using the focused repeat for each
case when one exists:

```sh
python3 proposal/experiments/event-repr-lab/analyze.py --latest sweep-3.json coup-product.json repeat-coup.json repeat-product.json repeat-modal.json repeat-runs.json repeat-structured.json repeat-sparse.json dispatched.json dense-control.json packed64-sweep.json packed256-sweep.json packed-modal-controls.json relations-sweep.json relations-repeat.json relations-dense-join.json relations-packed-join.json block64-initial-sweep.json packed512-initial-sweep.json packed4096-initial-sweep.json tail-map-controls.json tail-focused-repeat.json tail-modal-repeat.json tail-symbolic.json tail-coup-repeat.json
python3 proposal/experiments/event-repr-lab/layouts.py tail-map-controls.json tail-focused-repeat.json tail-modal-repeat.json
python3 proposal/experiments/event-repr-lab/map_comparison.py tail-map-controls.json tail-focused-repeat.json
python3 proposal/experiments/event-repr-lab/audit.py
```

The pre-shortcut baseline is `sweep-1.json`; the common-shortcut revision is
`sweep-2.json`; `sweep-3.json` adds the anchored challenger; the subsequent files
record alternate Coup coordinates, focused repeats, and the matched dense
kernel comparison. Source snapshots and their build metadata are retained for
each revision. `FASTPATH-MEASUREMENTS.md` retains the earlier stage's tables;
`MEASUREMENTS.md` is the current comparison. `--latest` uses the last supplied
process for a case, regardless of whether it was faster, rather than selecting
the minimum across runs. Omitting that option aggregates process medians.

The packed extension's [design proof](PACKED.md),
[primary-source adjudication](../../research/representation-search.md), and
[regression record](results/regressions.json) retain the evidence behind this
iteration. `inspect_assembly.py` extracts named operations from the exact current
binary and records their hashes; run it outside a timing sweep. Verification
failures are saved before the runner exits, and source edits during compilation
are rejected before new build metadata is published.

The block extension adds [three primary papers](../../research/block-decomposition.md),
three physical coordinate layouts, and a [retained initial layout sweep](BLOCK-BASELINE-LAYOUTS.md).
The shared checker and the native binary run identical carrier/algebra modules.
`check.py` does not replace the native executor routing checks or establish
native performance. Do not edit Rust sources while either build is running.
`inspect_assembly.py --tag new-name` preserves each assembly extraction under a
new tag; existing artifacts are never silently overwritten.
Use `--scope observation` to extract the actual masked-popcount contraction
kernel. `results/assembly-dependency.json` retains its ARM64 `CNT`/`ADDV` sites.
Use `--scope transport` for whole-word leaf construction and bit-axis
permutation; `results/assembly-transfer-words.json` retains that revision.

The [unary block follow-up](UNARY-BLOCKS.md) passes 48 structural processes,
but paired projection can retain more nodes. The
[shared-exit experiment](RANGES.md) adds grouped Apply and passes 64 structural
processes. It helps a deliberate shared-exit family strongly, while saving few
Apply states on Coup.

Its [complete scoped adapter](RANGE-NATIVE.md) passes symbolic, packet, owner
and shared-parameter checks. The
[native comparison](RANGE-NATIVE-MEASUREMENTS.md) passes 56 configurations:
grouped fresh Coup medians do not beat the matched one-bit control.

[Factoring grouped Event results](FACTORIZED-PACK.md) now has a separate native
implementation: branch scans and the summary join all use Free Join. Six
carriers pass 5,376 differential executions. The
[main comparison](PACK-NATIVE-MEASUREMENTS.md) retains 144 configurations and
72 matched schedule pairs. Every fresh query improves; several warm fanout-two
cases lose to staging. The representation and original range binary remain
unchanged. Twelve separate Lean reports establish the rewrite, its exact
join-dependency, group-presence and participating-validation conditions, and
typed relational analogues. These reports are separate from the central 195.
The [crossover follow-up](PACK-CROSSOVER-MEASUREMENTS.md) adds 48 configurations,
eleven samples each and reversed schedule order for the repeated controls.
Every fanout-one case loses; the large fanout-eight gains survive the repeat.

The Pack tree and executable are isolated under `pack-native-src/` and
`.scratch/pack-engine/`. To reproduce into new output names, build with
`pack_native_build.py`, then run `pack_native_sweep.py` with `--phase correctness`,
`--phase smoke`, and `--phase comparison` in that order. Each phase requires
`--output <new-name>.json`. Run no compilation, Lean or second benchmark beside
the timing phases. `pack_native_analysis.py` produces matched comparisons and
tables; `pack_native_audit.py` checks the frozen evidence.

The [relational Pack follow-up](RELATIONAL-PACK.md) adds typed composition and
residual aggregation with actual normalized-IR separator checking. Six carriers
pass 11,904 differential executions, while six additional Lean reports prove
the scalar assignment gluing and residual aggregation laws. The main sweep
and reversed-order repeat retain 448 configurations. Every fresh matched pair
improves; small warm memo-enabled cases lose to staging. Packed composition's
preferred working layout reverses after branch reduction.

This tree and executable are isolated under `relation-pack-src/` and
`.scratch/relation-pack-engine/`. `relation_pack_build.py` builds the comparison;
`relation_pack_sweep.py --phase correctness|smoke|comparison --output <new-name>.json`
runs serial phases. `relation_pack_analysis.py` checks outputs and writes matched
tables; `relation_pack_audit.py` checks frozen sources, samples and proofs.
