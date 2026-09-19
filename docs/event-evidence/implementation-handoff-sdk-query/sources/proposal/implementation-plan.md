# Event implementation sequence and acceptance gates

**Revision 0.9.** Implement the full [Event contract](proposal.md) in ordered
milestones. The public algebra stays fixed throughout; a milestone is not a
weaker substitute for the final design. Existing research is sufficient to begin.
The shelved participation experiment and further carrier competitions are optional.

The baseline is public BumbleDB v1.3.1. Implementation is underway on
`codex/event-algebra`; the [native ledger](../docs/event-implementation.md) records
the owned core, structural query algebra and fixed finite law/storage slices. This plan specifies the full
remaining obligation, including partially implemented milestones. Keep isolated
implementation commits with each gate's evidence attached. No release is authorized.

## M0 — Contract and reproducible semantic baseline

**Inputs:** current schema/query/source contracts, the pinned Lean toolchain,
Coup fixtures and retained native controls.

**Gate:** run [the readiness verification](semantics/README.md); preserve exact
source/version hashes and distinguish proof, exhaustive example, native behavior
and timing evidence. No benchmark is a proof of semantics. New operators must
have a named contract and oracle before entering native planning.

## M1 — Owned canonical Event core

**Native area:** [shared Event core](../crates/bumbledb-event/src/lib.rs),
[theory values](../crates/bumbledb-theory/src/value.rs), engine binding storage
and WorkContext integration. The theory crate does not depend on the engine.

Use the [completed-function baseline](representation.md): symbolic splits,
essential-coordinate tables at initial cutoff nine, direct ITE and staged
support-aware projection. Retain dense and packed512 as controls. Expose empty,
full, complement, BoolOp4, ITE, equality, structural tests and legal witnesses.
Keep private checked references and two-word row keys. Scope mismatches fail
before constant shortcuts. State and test all coordinate/index capacity limits.

**Gate:** independent bitset comparison over small arbitrary supports; equivalent
formulas outside support; complement involution; forced interner collisions;
owner mismatch behind empty/full; copy versus new outcome identity; result-only
owner lifetime; capacity/cancellation refusal with no partial publication.

**Proof anchors:** Retraction, EssentialCoordinates, Anchored, EventSignatures.
These prove denotations, not correctness of the new Rust arena.

## M2 — Structural type, canonical facts and persistence

**Native area:** theory/schema value types, schema and query macros,
[schema wire](../crates/bumbledb/src/schema/wire.rs), field codecs, store images,
transaction publication, spill and result owners.

Add `event`/`Event` and checked schema metadata. Freeze a versioned canonical
per-fact codec and compatibility policy before committing persisted Event facts.
Compare a deterministic semantic-order graph codec with the retained transport
control, including conversion blowup. Resident IDs, decoder choices and leaf
cutoffs never define persisted fact identity.

**Gate:** same Event bytes after different allocation orders, working orders and
supported carriers; reopen; exact whole-fact deduplication; empty/full roundtrip;
corrupt/missing objects; stale tokens; rollback; concurrent publication races;
results retained after snapshot release; spill and cancellation. Explicitly
refuse unsupported versions. A batch transport result cannot pass this gate.

**Required deliverable:** codec specification, compatibility fixtures and an
identity argument tied to the implemented encoder/decoder. [BEVT v1/v2](../docs/event-value-format.md)
now have pinned structural-row and measured-value fixtures, ownership/publication
tests and denotational persistence/source proofs. Remaining M2 qualification
includes complete consumer APIs, retained-memory policy and parameterized
descriptors required by M6. [BESC v1](../docs/event-source-descriptors.md) now
reconstructs fixed finite functions, channels and complete mathematical revision
receipts, including impossible outcomes and shared arithmetic admission budgets. Scope each passing gate to the supported format;
fixed rational tests cannot qualify future parameter-family descriptors.

The [initial TypeScript API](../docs/event-sdk.md) now adds `event`, immutable
owned BEVT carriers, native Effect operations, stored-field generated bindings,
parameters and Pack. Its isolated tests compile/import generated bindings and
exercise native dependency admission and reopen. Pure `fromBytes` checks an
envelope, not mathematical validity; the core decoder remains authoritative.
Supported Event data/query inputs now copy owned wire bytes before cancellable
worker admission; rejection citations also encode on workers. Pure Event/Test,
map and relation query builders and description roundtrips now use the native
grammar. Descriptor/source construction, complete structured diagnostics and
the aggregate retained-memory gate remain open.

## M3 — Native keys, containments and final-state admission

**Native area:** [schema validation](../crates/bumbledb/src/schema/validate.rs),
[judgment](../crates/bumbledb/src/schema/judge.rs),
[store bridge](../crates/bumbledb/src/storage/store/judge_bridge.rs), planner keys.

Resolve exact typed target keys and event-valued full constants. Enforce
pointwise conflict and union coverage over distinct whole facts; retain fact
provenance for citations. Recompute contributor summaries on deletion.
Keep scalar roster laws active for empty-valued facts. Require nonemptiness or
an independently sufficient ordinary key for distinctness/unique-binding rewrites.

**Gate:** native interval-judge bridge cases plus actual Event admission; overlap,
gaps, complete/partial partitions, equal-payload distinct facts, empty rows,
zero-mass nonempty rows, absent scalar targets, deletion with shared coverage,
atomic batches repairing intermediate violations, stable final-law citations.
Compare query counts with and without the pointwise-key shortcut.

**Proof anchors:** EventStorage; FiniteMeasure.key_and_coverage and
schema_partition_mass; native Admission's coverage/conflict correspondence and
citation theorem. The source constructor separately validates the law.

**Current slice:** native field projections now implement pointwise keys and
union coverage, rebuilding each affected statement's final state. Empty Event
keys use a conservative planner witness and a checked single-result lookup.
The native ledger retains bitset, transaction, query, citation and spill evidence.
Contextual full projections now also have typed declaration, wire, fingerprint,
compiler and admission support. Their native tests cover absent targets, retained
contexts, arbitrary-support partitions and closed scalar rosters; the native
Admission proofs establish the required nonempty-world laws. See the
[projection contract](../docs/event-projections.md). This does not close M3:
Event selections, capacity positions and Event-valued closed rosters remain
guarded. Zero-mass/nonempty source cases require measured-source integration.

## M4 — Checked faces, maps and the full nonrecursive relation core

**Native area:** new owned face/map/relation descriptors and Event operator IR.

Construct parameter-preserving legal-domain products, role certificates and
symbolic identity. Implement converse, composition, domain/range, May/All/Must,
both residuals and graph/readout views. Keep support-image, complete-fibre and
law-pushforward capabilities separate. Shared environments and middle witnesses
cannot be projected independently. Gates/repair are prepared operation data.

**Gate:** explicit matrix oracle on asymmetric legal domains; role-invalid third
faces; empty endpoints/operands; both residual adjunctions; converse/identity;
shared-environment counterexamples; dead ends; the guarded Must composition rule;
cross-owner maps and newly produced result ownership. Public safe inspection and
construction interfaces must cover the same algebra used by admission.

**Proof anchors:** LegalRelations, ScopedProduct, BaseChange, ViewProduct,
ProjectionGate, LocalCompletion and ModalContract.

**Native progress:** [checked finite maps](../docs/event-maps.md) now support
arbitrary Boolean readouts, simultaneous substitution, symbolic image with
abstraction after last use, both image adjunctions and separate surjectivity
admission. Native CoordinateMaps proofs cover their denotational/reference laws.
The [finite relation core](../docs/event-relations.md) now also implements full
face/fibre products, joint-fibre certificates, membership-FD roles, symbolic
identity/tests, converse, composition, modalities, both residuals and graph/readout
conversion. It has explicit matrix, native storage and semantic evidence.
The [inspection API](../docs/event-inspection.md) now provides immutable snapshots,
borrowed graph/table views and checked reconstruction, with a signed-graph
reference proof and native support/ownership oracles. [BEDC v1](../docs/event-descriptors.md)
now transports finite maps, onto maps, products, relation views, composition plans
and complete squares as inspectable descriptors. Import reconstructs all support,
orientation, environment and fibre certificates; syntax parsing alone admits no
executable object. Eleven Descriptors reports prove reference reconstruction
obligations and counterexamples. Captured map query imports now retain checked
readouts and all six map operations, with exact input/output contexts. Typed
relation trees now retain faces and explicit product plans, including finite Star.
Source/SDK and general bound fixed-point query integration remain acceptance
work. A surjectivity certificate never substitutes for a complete-fibre-square
or law-pushforward certificate.

## M5 — Query heads, Pack and certified Free Join schedules

**Native area:** [query macro](../crates/bumbledb-query-macros/src/lib.rs),
[IR](../crates/bumbledb/src/ir.rs),
[preparation](../crates/bumbledb/src/api/prepared/build.rs),
[computed outputs](../crates/bumbledb/src/api/prepared/computed.rs),
[aggregate sink](../crates/bumbledb/src/exec/sink/aggregate.rs),
[Event Pack claims](../crates/bumbledb/src/api/prepared/computed/pack.rs), planner.

Add explicit Event/Test heads and owned operator programs. Event Pack unions
one bound region per group; dependent heads use separate stages. Preserve empty
values, explicit seeds and ordinary anti-probes. Start with complete bindings;
then integrate certified factoring as an optional costed schedule. The supplied
separator checker is conservative; search/expanded fragments require new proofs.

New Event validation reports the deterministic set of participating operand
faults specified in the proposal. Derive stable semantic diagnostic descriptors;
neither join order nor resident allocation chooses the error. Do not change the
existing scalar evaluator's error contract.

**Gate:** complete/factored/oracle equality of values, presence, multiplicities
and fault sets; stable ordered diagnostics across physical schedules; unmatched
invalid operands; validation after saturation; duplicate bindings; equal masses
on unequal Events; lost middle-state/environment counterexamples; output lifetime
and spill. A residual-intersection aggregate remains distinct from union Pack.

**Proof anchors:** native Query and Pack; laboratory FactorizedPack,
ParticipatingValidation, RelationPack and QuerySeparator. No theorem presently
verifies native IR extraction or costing.

**Native progress:** [complete-binding Event/Test heads](../docs/event-queries.md)
now compile through Rust macros and owned pure-data IR, including all Boolean
truth functions, ITE, fixed-roster cardinality and six structural tests. Scoped
empty, staging, ownership/spill and canonical participating context-fault sets
have native tests. Query.lean adds 22 reference reports. Raw Node descriptors
are admitted and bounded. Event Pack now implements per-group union, canonical
context minima, retained rule provenance, computed group keys and scratch-backed
claims. Four database integrations, forced-spill/cancellation tests and four
actual N-API execution scenarios qualify that slice. Pack.lean adds 19 reference
reports for presence, minima, context admission and partial-key participation.
`EventImport`/`use map` now retain checked BEDC maps through owned query IR.
All six readout operations nest in Boolean/Test/cardinality heads and stages;
static context incompatibility refuses before execution, and dynamic validation
records demands by leaf occurrence across map boundaries. QueryMaps adds 18
scope-indexed reference reports, including whole-program congruence from retained
inputs. Native tests use explicit fibres on complete/coupled supports, mixed
imports, independent owners, Pack staging and actual N-API execution. Retained
face/product imports now supply typed relation views, all Boolean constructions,
identity/tests, converse, composition, both residuals, modalities and finite Star.
QueryRelations proves whole-program transport under legal endpoint bijections,
with shared environments as an explicit instantiation premise. Seven native
query integrations and 31 N-API executions qualify that slice. The SDK now
authors the existing grammar through typed Event/Test/relation builders and
owned BEDC carriers, including query-description imports and staged Pack.
General
Least/Greatest query binders, structured SDK diagnostics and certified factoring
keep M5 open.

## M6 — Exact sources, observation and TypeSafe imports

**Native area:** source registry/constructors, exact arithmetic and contraction,
proposed Probability/Expectation heads and ordinary import metadata.

Begin with finite normalized rational laws and conditional categorical sources.
Retain original support and source identities; distinguish missing law, impossible
evidence, source conflict and unsupported construction. Preserve repeated evidence,
copy versus fresh draws, conditional policy information, and correlation.
Then add shared polynomial parameters, guarded rational functions and exact
semialgebraic constructors with documented solver certificates/capabilities.
The general solver and canonical arithmetic encoding are not supplied by Lean here.

**Gate:** nonuniform correlated sources, skipped outcome multiplicity, guards
without stochastic mass, unused categorical codes, nonempty zero-mass evidence,
conditional complement, full/proper partitions, repeated shared bias, endpoints,
Coup Tax posterior and old/new hand transitions. A conditional observation keeps
its denominator/domain. Never silently rescale invalid model distributions.

For TypeSafe, test each declared import intent separately: new judgment,
conditional forecast, constraints, posterior revision and likelihood. Posterior
Noul is not a likelihood pair; independent API questions do not establish joint
independence. Actor cases exclude inaccessible hidden information. Preserve raw
responses and named observation identities; no production API calls are needed
for fixture acceptance.

**Proof anchors:** FiniteMeasure, FaceCounting and information/map laws. General
source solving and source-adapter correspondence retain explicit native
obligations. Revisions.lean now proves finite rational Jeffrey/Pearl laws and
signed cell contraction; it is a reference, not Rust refinement.

**Native progress:** [Fixed finite rational sources](../docs/event-sources.md)
now implement exact arithmetic, checked joint density partitions, symbolic
contraction, source-owned conditional observations and BEVT v2 identity. Twelve
core tests include 61,440 exhaustive conditional cases and a 62-bit symbolic
law. Database/Node consumers exercise law-sensitive joins, zero-mass dependency
admission, persistence and malformed-law refusal. FiniteDensity adds 20 checked
reports. Finite functions and conditional channels now add weighted image,
per-parent normalization, old-marginal preservation and explicit fresh outcomes.
Eight more core tests include 3,840 arbitrary finite maps, large symbolic images,
categorical encodings and the Coup Tax posterior; a stored query consumer checks
captured measured maps. WeightedMaps adds 18 reports.
[Fixed-law revisions and signed expectations](../docs/event-revisions.md) now
retain mathematical inputs, impossible outcomes and explicit translation. Nine
core tests include 1,280 conditioning/expectation cases and 240 partition revisions;
a persisted Coup consumer checks posterior queries and signed utility after
owner drop. Revisions adds 30 exact rational reports. Parameter
families, solver capabilities, TypeSafe adapters and query observation heads
keep this gate open. BESC v1 function/kernel/revision transport now replays
constructor checks, including full receipt claims and zero-prior channel rows;
SDK construction, parameterized descriptors and provider provenance remain
explicit integration obligations. SourceTransport adds 14 reference reports.

## M7 — Information, observables and finite fixed points

**Native area:** checked observation descriptors, dedicated sealed finite stages,
action/availability operators and explanatory witnesses.

Implement Possible/Guaranteed/Ambiguous, value/event partitions, expectation,
Star and validated monotone least/greatest fixed points. Keep ordinary recursion
unchanged. Finite-path laws do not prove algorithm termination: validate the
finite carrier and implement a bounded stabilization argument. Controlled choices
and hidden-state quantifiers follow declared visibility and information updates.

**Gate:** coarse/fine/coupled information cases; observer FDs; uniform-action
counterexample and greatest permission relation; legal witnesses; unavailable
actions; dead ends and avoiding cycles; reachability versus inevitability versus
invariant safety; capacity/cancellation. An unbounded game transcript is not a
finite presentation. Supply a termination/refinement argument for the actual
fixed-point implementation before this gate passes.

**Current slice:** [native information operations](../docs/event-information.md)
now return possible, guaranteed and ambiguous Events through checked readouts,
with explicit nonvacuous evidence cases. Onto readouts admit unique FD factors;
indistinguishability relation construction checks that the observation determines
the pair environment. Independent partition oracles and native Information
proofs cover this slice. [Sealed finite programs](../docs/event-fixed-points.md)
now implement typed host instruction DAGs, positive variance admission, exact
least/greatest detection, CanReach/InevitablyReach/SafeThroughout and Star.
The carrier counts original legal support; Star counts legal pairs and preserves
one environment. [Indexed finite partitions](../docs/event-partitions.md) now
admit parent-relative observable rosters, count buckets, value grouping,
same-world refinement and checked readout conversion. Scalar labels remain
ordinary columns and empty cells retain their identity. The
[action layer](../docs/event-actions.md) now supplies enabled choices, controlled
predecessor programs, inhabited uniform permissions and fully observed
ranked-reach/continuing-safety strategies. Policy restriction preserves state
coverage and every retained progress/invariant witness. Host finite-function
expectation now returns a source-owned signed rational observation. Query
expectation/coverage integration, belief/information-memory construction,
continuous-source certificates and general
bound fixed-point query
integration remain required by this gate.

**Proof anchors:** InformationReadout, ReadoutMaps, ModalContract and
[FixedPoint](../crates/bumbledb-event/semantics/FixedPoint.lean),
[Programs](../crates/bumbledb-event/semantics/Programs.lean) and
[Closure](../crates/bumbledb-event/semantics/Closure.lean). These prove the finite
iteration bound, legal-roster count, early detection/extremality, typed variance
soundness, closure/reachability/safety/forcing denotations and uniform shared
environments. Native admission, graph equality/counting, DAG evaluation and
resource behavior remain tested correspondence obligations. Actions additionally
proves rank-bounded termination for every retained choice/outcome, policy
restriction, greatest uniform permissions and enabled invariant safety.
Fully observed finite strategies are covered by those denotations; adequate
belief-memory construction for repeated partial observation is not supplied.
Partitions additionally proves clipping/admission, grouping/refinement/pullback,
readout codes and the complete count-roster recurrence; native vector/index
and kernel correspondence remains explicit.

## M8 — Integrated qualification and performance

Run the repository correctness battery after native integration, including schema
identity/compatibility, transactions, storage and packaged Rust/TypeScript consumers.
Execute the complete Coup schemas and basic/advanced query fixtures against the
independent world oracle. Verify malformed external descriptors cannot bypass
owner, role, support or source admission.

Measure one native lane at a time: construction, fresh query, warm query,
canonical publication, exact observation, conversion and owner retention. Include
small/large groups, duplicate paths, skew, unmatched rows, adversarial orders,
shared parameters and memory pressure. Retain dense/packed controls and losses.
Record actual resource limits; distinguish estimated retained bytes, live graph
size and process RSS. ARM64/NEON inspection covers named leaf kernels only.

**Release gate:** all applicable milestones pass with linked evidence. The
complete algebra remains the target; partial implementation must name unsupported
capabilities explicitly and cannot advertise the whole Event proposal as shipped.
No additional open-ended representation competition is required.
