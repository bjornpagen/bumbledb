# Implementation sequence and decision experiments

Research is closed; the [handoff](RESEARCH-HANDOFF.md) records its conclusions.
The sequence below is future implementation work. The shelved participation-index
experiment is not an acceptance gate for beginning that work.

Revision 0.7. Items below remain proposed production work. The
[temporary Rust laboratory](experiments/event-repr-lab/REPORT.md) now measures
competing Event carriers through a disposable copy of the actual Free Join engine.
Its test sink is not native Event schema, admission or persistence support.
The sequence follows the data dependencies; it is not a request for approvals.

## 1. Canonical Boolean and event core

Choose the resident carrier using the lab's target-workload results before fixing
its layout. The BDD candidates use fixed coordinate order, canonical Apply and
either paired or anchored normalization; finite candidates intern canonical
complemented regions directly. `BoolOp4` and exact scoped identity are shared
contracts, while `BddRef` and `RegionPair` are representation-specific.
Implement empty/full, scoped equality, complement, union, intersection,
difference, XOR, ITE, cardinality buckets, structural tests, and deterministic
witness extraction. The Boolean core is a representation dependency, not the
public algebra's final capability boundary.

Acceptance: compare against independent bitsets for every Boolean operation,
all small support masks, formulas differing outside support, permutation/rebuild
wire stability, and complements. Unknown/invalid references must never reach
the kernels. Add resource/cancellation refusal during construction.

## 2. Finite sources and guarded source extension

Implement unmeasured structural spaces, explicit admissibility, named categorical
and finite conditional sources, invalid-code support,
copy versus fresh outcome, checked joint extension, and alignment/lifting. Adopt
fixed exact rational laws first, then polynomial shared-parameter laws with
explicit endpoint guards. General semialgebraic source construction requires an
exact solver contract; keep unsupported constructors explicit rather than
pretending that sampling or floating tolerance validates them.

Acceptance: card conservation, repeated shared bias, copy's impossible
disagreement, two different biases, optional decisions, and p=0/p=1 cases.
Check a legal zero-mass outcome remains nonempty; an explicit Supported(law)
view can exclude it. Probability on unmeasured spaces returns MissingLaw.
Test restricted and extended scopes, including extension that fails to project
to the old support. Constructor limits do not create a different Event type.

Support-totality, projection-square completeness and law-pushforward equality
need separate certificates. Retain copied-coordinate counterexamples and
same-marginals/different-coupling cases. The [map review](research/space-maps.md)
settles that distinction; the Rust transport lab implements only finite binary
support embeddings so far.

## 3. Owners, canonical rows, and persistence

Add the event structural type, two ordinary word columns, binding slots, owner
collections, scratch-stage support, canonical descriptor/graph serialization,
verified object resolution, and transaction publication/rebasing. Prove stable
logical bytes separately from cheap resident equality. Never serialize arena IDs.

Acceptance: read after transaction/session release, resident→scratch→result,
reopen with shuffled allocation order, concurrent publication interning races,
rollback, corrupt/missing objects, stale scope tokens, cancellation and memory
exhaustion. Check logical diagnostic order after a different resident load order.
Save and reload empty/full Event fields without losing their facts or owners;
complement a reloaded empty result and compare its full scoped identity.

The [transport lab](experiments/event-repr-lab/TRANSPORT.md) now checks a shared
graph/table codec, cross-carrier rebase, coarse batch ownership and native query
replay. Its packet is an ordered batch, not a finished canonical fact encoding.
Use its phase-separated timings to include conversion and support-proof costs
when comparing a common wire carrier with each resident choice.
The word-table path reuses exact coordinate permutations and whole-word leaf
construction. The owned-key experiment also registers both polarities of a
canonical complement pair, preserving one-bit complement across publication.
The [owned computation lane](experiments/event-repr-lab/OWNED.md) now verifies
novel results in the retained arena, with no query-manager clone, across all
fourteen carriers. Individual fact bytes and native owner/transaction integration
still need explicit acceptance tests.

## 4. Pointwise admission

Generalize the existing pointwise domain dispatch without borrowing interval
endpoint ordering. Add event union/overlap/coverage witnesses, exact target-key
resolution for typed full-space faces, and final-state transaction checking.
The [native dependency review and experiment](experiments/event-repr-lab/DEPENDENCIES.md)
supplies a compositional `(covered, conflict)` summary: both are Events, and
the merge implements pointwise contributor counts saturated at two. A tree
recomputes these summaries after deletion instead of subtracting lost coverage.
Track contributing fact IDs so union witnesses can produce deterministic native
violation reports. A persistent union tree or support counts are needed for
incremental deletion: subtracting a removed event from a union is wrong when
another fact still covers the same worlds.

Acceptance: overlapping distinct facts with identical payload, full and partial
partitions, gaps, empty branches versus nonempty zero-mass branches,
deletions with remaining overlap, batches that
repair each other's intermediate violations, and unchanged unrelated groups.
Existing scalar containments and capacities retain their meaning.
Empty-valued facts contribute no pointwise coverage/conflict but still obey
scalar INDs, exact fact equality and ordinary capacities. Test missing scalar
targets even when the Event is empty; retain owner validation before shortcuts.

## 5. Free Join and nonrecursive event stages

Extend the pure-data IR and macro with Event/Test and descriptor import heads. Validate
and align all event operands before rewrites. Bind compiled event programs and
retained owners; evaluate them on complete Free Join bindings; extend Pack to
one event per group. Keep explicit empty seeds and total query event values.
Connect exact measurement in a later stage. Ordinary negation remains anti-probing.

Acceptance: every [Coup query](coup/queries.rs) against the direct-world oracle;
duplicate paths; differing events with equal probability; complement after Pack;
scope mismatch hidden behind a constant; repeated evidence; and impossible
conditioning. Import [advanced Coup queries](coup/algebra-queries.rs) as the next
acceptance fixtures. Ordinary recursive feedback remains unchanged; dedicated
finite fixed-point stages arrive in step 8.
The [storage-closure proof](event-storage.md) adds a planner obligation: two
distinct empty-valued facts can have equal complete pointwise keys. Require
nonemptiness evidence before reusing the interval distinctness/unique-binding
shortcut, or use an independently sufficient ordinary key/full-fact identity.
Compare projected results and scalar counts with and without that evidence.

## 6. Exact law contraction

Implement event/law joint traversal with stochastic sum versus logical guard
selection and skipped-coordinate accounting. Memoize by source, event root,
position, and retained parameter context. Add exact rational/polynomial kernels,
positive-evidence results, and only the supported bound-solving constructors.

Acceptance: nonuniform correlated laws, categorical unused codes, skipped outcome
bits, guard refinement that must not double mass, complement/evidence identity,
Coup Tax posterior, and old/new hand transitions. Approximate display paths may
be added only with explicit error contracts; they cannot decide equality or
admission and must not replace exact outputs silently.

The [observation laboratory](experiments/event-repr-lab/OBSERVATION.md) now
implements a first exact fragment across fourteen carriers: explicitly grouped
conditional-iid binary sources, shared Beta parameters, fixed rational laws,
unused draws and zero-evidence conditionals after actual Free Join. Its
coefficient array is a baseline for comparison with normalized polynomial
circuits and general factored laws, not the complete source/guard solver.

## 7. World faces and relational product

Build owned FaceLayout, CoordinateMap, product-support and RelProductPlan
descriptors. Start with parameter-preserving finite coordinates; establish one
common complete guard refinement before projection. Implement relation views,
identity/tests, converse, composition, domain/range, May/All/Must, and both
residuals. Validate signatures before evaluation, including empty operands.
Reordering through a map must rebuild canonical nodes when necessary.
Construct identity from corresponding-coordinate equalities; never require
enumerating a symbolic state product just to prepare a relation program.
The [diagonal experiment](experiments/event-repr-lab/DIAGONAL.md) implements
that boundary and rejects symmetric supports that are not full products.
The [legal-relation investigation](experiments/event-repr-lab/LEGAL-RELATIONS.md)
adds a distinct input-role obligation: scoped abstraction of all undeclared
scratch faces must leave the Event unchanged. This is the membership FD on its
declared faces, not a test that their raw essential bits vanish. Carry that
checked role through legal operators; reject a third-face Event offered as a
binary relation even when the owner has full support. Generalize the product
constructor to legal state domains with unused bit codes and, subsequently,
domains indexed by a retained environment.

Acceptance: exhaustive small explicit relations; composition and residual
adjunctions; target/source support asymmetry; invalid face aliasing; shared
parameters whose witnesses cannot be selected independently; partial-transition
dead ends; new result-space ownership through query staging and serialization.
Expose safe graph/face/map views and the same pure-data IR to library consumers.
The hypothetical Coup action inputs must carry their rule-completion contract.

## 8. Information and sealed finite fixed points

Implement observation partitions as ordinary relations or checked equivalence
relation views. Add Star and positive least/greatest predicate programs on
validated finite stable state presentations. Retain monotonicity, carrier,
and parameter-environment certificates; detect stabilization by canonical
identity. Action quantification uses explicit availability and visibility.

The [native readout laboratory](experiments/event-repr-lab/PREFIX-READOUTS.md)
now measures constructive Possible/Guaranteed/Ambiguous outputs after actual
Free Join. Its [Lean laws](experiments/event-repr-lab/INFORMATION-ALGEBRA.md)
provide exact FD conditions for uniform filter movement and information order.
The [equal-cofactor control](experiments/event-repr-lab/COMPLETION.md) now rules
out that shortcut as a remedy for the retained growth. Keep it and staged Apply
as controls for [direct ternary ITE](experiments/event-repr-lab/TERNARY-COMPLETION.md),
then retain the measured [dependency-directed gates](experiments/event-repr-lab/PROJECTION-GATES.md)
as a separate control: fixed ITE falls from 392.599 to 10.758 ms, while packed
still takes 1.467 ms. The [local-completion control](experiments/event-repr-lab/LOCAL-COMPLETION.md)
now passes 110 comparative processes and reduces its matched query from 10.659
to 2.247 ms. Test [direct completed projection](experiments/event-repr-lab/FUSED-PROJECTION.md)
as a separate operation; its source witnesses and target selectors have distinct
roles, and existential abstraction forbids the substitution-only complement
memo convention.
Measure reachable answer
graphs separately from nodes retained by caches and prior intermediate work. Keep canonical reimport caps separate from timed
query failures; complete output and original-support checks remain required.

Acceptance: all small observation partitions, evidence-empty cells, refinement,
noncommuting constrained saturations, closure versus direct graph traversal,
dead ends and avoiding cycles, uniform-action versus hidden-world choice, and
rank-respecting reachability strategies. Refuse a continuous transformation or
an unbounded transcript without a justified finite quotient. Native query rec
does not acquire any new implicit stochastic fixed-point meaning.

## 9. Observables, source revisions, and inference experiments

Add exact signed Expectation with value/event aggregation, evidence-domain
ownership, overlap and coverage validation. Support finite observable maps,
count partitions, witnesses and checked sufficient explanations. Implement
Jeffrey revision, likelihood channels/reweighting, and law constraints with
separate receipts and explicit context maps. Add source-function derivatives
only on supported exact differentiable fragments with retained domains.

Acceptance: duplicate value branches, missing coverage, uncertain target/certain
cost, shared-parameter robust dominance, observation value with the maximum
outside hidden-state aggregation, posterior-versus-likelihood counterexample,
same observation reused, zero-prior target refusal, and explicit zero-mass
structural possibilities surviving a law update.

For a later authorized inference experiment, collect pre-action Coup judgments
on focused actor information cases and retain complete Noul/Choice/Score outputs.
Compare thresholded answers, marginal-only answers with unknown coupling, and
the adopted joint event source. Separate calibration and import-assumption
quality from exact algebraic consistency. No API calls are part of this revision.

## 10. Measure the representation before choosing fast paths

| Workload | Required dimensions | What to record |
| --- | --- | --- |
| Boolean core | Constants, repeated roots, parity, equality chains, irregular formulas | Nodes visited/created, memo hit rate, unique-table probes, bytes and wall time |
| Coup states | 2–6 players; card- and trajectory-based coordinate orders; increasing finite history | Support/law/event sizes separately; cold/warm construction and query time |
| Query overlap | Few versus many paths to the same event; many distinct overlapping events | Binding count versus canonical event count; Pack allocation and retained memory |
| Dense execution | Cofactors with 4–10 variables and explicitly finite scenario bitmaps | Compilation/reconstruction cost plus kernel time; reuse count needed to pay back |
| Sources | Fixed rational, shared polynomial, guard endpoints, uncertain couplings | Weight DAG/coefficient growth, support compilation, contraction, bound cost |
| Relational product | Sparse/dense transitions, face orders, quantifier sets, shared guards | Lift, conjunction, abstraction and reconstruction costs; peak and final nodes |
| Information / strategies | Coarse/fine cases, visible versus hidden actions, increasing finite memory | Partition/arena growth, fixed-point iterations, witness/rank extraction |
| Lifetime | Long-lived games, aborted writes, repeated snapshots, forced spill | Arena retention, owner counts, scratch bytes, compaction cost |

The lab compares sixteen Rust candidates: dense, sparse, Roaring,
runs, masked BDD, paired BDD, four-way DD, anchored BDD, diagrams with
64-/256-/512-/4,096-bit terminal tables, six-coordinate selector blocks,
tables over six/nine exact essential coordinates,
and a matched dense kernel variant that hoists operator dispatch and emits NEON. It measures real
Free Join, relational Must, explicit complement, elimination, and symbolic order
growth. A further lane tests coordinate movement, converse, residuals and
finite closure after actual Free Join/grouping. Read its raw samples and
qualifications before using the winners.
Formula/SAT, bounded/general SDD, chain-reduced and variable-shift diagrams
remain additional matched experiments, including
eventual equality, quantification and observation costs. A favorable tiny NEON
loop does not establish query throughput.

The [classifier lane](experiments/event-repr-lab/CLASSIFY.md) now compares direct
support-aware occupancy, four materialized Venn cells and signature reuse through
an owner-validated Free Join computed-stage sink. Its production counterpart
still needs native Event residual placement and batching. Allen classifies
endpoint pairs once and applies a supplied mask by table lookup; Event's fifteen
nonempty signatures offer an analogous finite predicate surface, but computing
them can require traversing shared structure. Preserve constructive witness
Events as a separate output contract. Any shared SIMD lookup core must accept
all sixteen table entries: the current Allen-specific wrapper initializes only
codes 0 through 12, while Event signatures 13 through 15 are valid.

The [derived signature calculus](experiments/event-repr-lab/SIGNATURE-CALCULUS.md)
also settles the finite-composition question. It supplies a sound associative
table for pruning Event relationship constraints and a finite-scope
counterexample to strong composition. Exact region reasoning must remain
available when constraints need more than pair occupancy. Neither table closure
nor a favorable `TBL` instruction probe proves complete Event admission.

The essential-table experiment includes scalar and word algorithms in one
binary and a derived path using constructor certificates and cached structural
counts. Compare the full query, publication, observation and retained bytes;
removing irrelevant table axes must not remove ambient draws or alter their
source-law coupling. Its first 163 native cases pass, while the new carriers
trail fixed-tail controls. Keep that negative evidence when choosing the next
representation experiment.

The [matched compact stores](experiments/event-repr-lab/SLABS.md) now pass
105 native processes with identical logical counts in 146 layout pairs. Smaller
records and single-copy payloads reduce retained bytes, but do not settle fresh
latency. The subsequent [word-classifier control](experiments/event-repr-lab/WORD-CLASSIFIER.md)
uses the normal form's constant-or-table invariant at a bounded local cube.
Its 144 matched pairs preserve complete histograms, resident storage and pair
caches; the nine-sample repeat confirms large first-use gains under both stores,
including ordered cuts with missing cells. Retain fixed-tail/dense controls:
they still lead several queries, and direct classification is not universally
faster than constructing cells. The separate [bounded-scratch control](experiments/event-repr-lab/SCRATCH.md)
now preserves the allocated-word path while replacing alignment buffers.
General renamed operands and planner
placement remain separate experiments. A predicate-only shortcut is a different
output contract from the complete histogram.

The [Lean proof suite](experiments/event-repr-lab/LEAN.md) now also proves
essential-coordinate minimality, cofactor characterization, exact projection,
complement invariance and raw coordinate elimination. Its map proofs establish
that every possibility query survives inverse image iff the support image is
exactly the target support. This also preserves pair occupancy and reflects
supported equality; a restriction cannot reuse an unrestricted signature without
further proof. Its base-change theorem separately proves the exact complete-fibre
criterion for quantifier-square rewrites, including nonvacuous universals. Every
compatible pair needs a lift, but uniqueness is unnecessary. Keep those
capabilities separate from law certificates, and key rewrite proofs to the
particular square and admitted supports. The suite supplies denotational invariants; Rust arena
canonicality and machine-code refinement retain their own proof obligations.

The [borrowed-product brief](experiments/event-repr-lab/VIEW-PRODUCT.md) now gives
the mapped kernel's semantic and experimental contract. Read both operands
through their maps while retaining their exact joint compatibility, project the
shared witness, and publish only the canonical result. The original five joint-image Lean reports
prove the denotational fusion and its necessary joint-image condition. The Rust
prototype handles pending pins when source and destination orders differ;
the existing classifier's constant-or-table stopping invariant does not transfer
automatically. The first native comparison returns exactly the same complete
relation results but loses on speed, and output fusion can increase partial
outputs. The [factorial control](experiments/event-repr-lab/VIEW-ORDER.md) now
separates assignment/word readout from output-late/fused traversal. The output
bijection and witness-reindexing laws also have Lean proofs. Charge canonical
publication and all resident operation caches in each control.

The [source-normalization and reuse control](experiments/event-repr-lab/REUSE.md)
compares canonical source cofactors against pending pins, independently of the
plane cache and output working order. Keep complete full-program and isolated
product lanes, both stores, both cutoffs, and existing packed/dense controls.
Cache on/off must preserve exact construction and retained byte counts. Charge
normalization intermediates and distinguish cache-held bytes from total peak
workspace. Independent repeats, owned publication and symbolic closure are
acceptance gates before choosing a policy.

The [supported-dependence proofs](experiments/event-repr-lab/SUPPORTED-DEPENDENCE.md)
rule out a unique least logical dependency mask on arbitrary coupled support.
Retain the raw-function mask invariant. The [scoped adapter](experiments/event-repr-lab/SCOPED-PRODUCT.md)
now admits arbitrary support by preserving witness and intermediate-output
gates; it has the exact meaning of the staged clipped operations. This does not
grant relation laws to arbitrary coupled support. The laboratory now admits
certified legal-domain products and checked input roles, handling unused codes
and rejecting symmetric coupled supports. Preserve its separate support/role
obligations when integrating production handles; test cross-owner publication
and failure before adopting the internal capability API. More general alignment still needs
the joint-image certificate; product support does not provide a probability law.

Inspect generated ARM64 as the existing Allen work does. Gate only named leaf
symbols with justified instruction properties. Do not demand flag-free general
Apply or pretend NEON has a general indexed gather. Add batch compaction and
constant-operand paths after profiles identify their value.

## Design review findings resolved in this revision

The [native Pack experiment](experiments/event-repr-lab/FACTORIZED-PACK.md)
now supplies a second query schedule above unchanged Event carriers. Its fixed
clover constructor gives the exact Cartesian-branch certificate. Native branch
scans and a native summary join preserve present empty groups, whole-row
deduplication and represented multiplicities; validation follows participating
rows and preserves unordered fault sets. The differential suite passes 5,376
runs. Main and crossover sweeps retain 192 configurations and show both large
gains and small-query regressions.

Before general planner integration, derive the separator certificate from
normalized variable incidence and check residuals/anti-probes. Keep presence
separate from the Event fold, specify diagnostic ordering, and estimate staging
against complete traversal. Typed composition and residual Pack laws are checked
separately in Lean; implement and test their support, interface and witness
obligations rather than treating Boolean distributivity as sufficient.

The subsequent [relational prototype](experiments/event-repr-lab/RELATIONAL-PACK.md)
now checks a supplied scalar separator on the actual normalized query and
executes composition/union and residual/intersection programs through native
branch scans and summary joining. Legal faces, relation-role admission and
source environments remain checked. The new differential suite passes 11,904
runs. Its conservative checker rejects local filters, point bindings, derived
occurrences and global residuals/anti-probes; extending this fragment requires
tracking their actual variable incidence. Search and costing remain separate
from certificate validity.

- Admissible-world equality requires exact support-relative canonicalization;
  the experiments test several valid representations of that invariant.
- Probability zero cannot establish a forbidden world; supported-law views
  must be requested explicitly.
- The algebra exposes predicates and relations, not only a probability getter:
  faces, domain, residuals, and finite closure are required interfaces.
- One-bit relative complement does not require storing both BDD roots: finite
  canonical orientation and the anchored-root construction also establish it.
- Canonical resident identity is different from wire identity: rebuild in a
  deterministic order and retain semantic coordinate descriptors.
- Flat two-word values need transitive owners through spill and returned results.
- A faithful support extension does not automatically commute with projection
  or preserve a designated probability law; those proofs have separate roles.
- Parameter guards select logical cases; they never acquire stochastic mass.
- Event FD/IND operations share Boolean primitives but cannot reuse temporal sweeps.
- A removed row's event cannot simply be subtracted from aggregated coverage.
- Dense representations can be canonical resident carriers for finite worlds.
  Cross-carrier conversion and persistence must still preserve exact identity.
  Unknown dependence cannot be replaced with marginal multiplication.
