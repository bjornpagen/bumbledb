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
identity argument tied to the implemented encoder/decoder. [BEVT v1/v2/v3](../docs/event-value-format.md)
now have pinned structural-row, fixed-law and shared-parameter fixtures, ownership/publication
tests and denotational persistence/source proofs. Remaining M2 qualification
includes complete consumer APIs and retained-memory policy.
[BESC v1/v2](../docs/event-source-descriptors.md) now reconstruct fixed finite and
shared-parameter functions, channels and complete mathematical revision receipts,
including impossible outcomes and shared arithmetic admission budgets. Version 2
also carries refinements and restrictions. Scope each passing gate to its format;
separate family tests cover partial endpoints, guard refinement and replayed
parameter-dependent claims.

The [initial TypeScript API](../docs/event-sdk.md) now adds `event`, immutable
owned BEVT carriers, native Effect operations, stored-field generated bindings,
parameters and Pack. Its isolated tests compile/import generated bindings and
exercise native dependency admission and reopen. Pure `fromBytes` checks an
envelope, not mathematical validity; the core decoder remains authoritative.
Supported Event data/query inputs now copy owned wire bytes before cancellable
worker admission; rejection citations also encode on workers. Pure Event/Test,
map and relation query builders and description roundtrips now use the native
grammar. Complete participating context-fault records now cross to TypeScript
with owned canonical payloads. Native `EventDescriptor.admit`/`describe`/`inspect`
now cover all seven structural BEDC kinds, with owned derived projections and
explicit role/support certification. SDK BESC construction now covers fixed
finite functions, checked conditional kernels and replayed revision receipts.
SDK [exact regions, inhabited domains, roots and partial parameter functions](../docs/event-sdk-parameters.md)
now retain disconnected sets, endpoint holes and BESC v2 function transport.
[SDK family sources/functions and owned observations](../docs/event-sdk-families.md)
now construct normalized univariate laws, preserve actual-world witnesses and
expose individual guard refinements. [SDK family dynamics](../docs/event-sdk-family-dynamics.md)
now add channels, restrictions, complete revision inspections/original-prior
pullbacks and common refinement rosters. Complete query APIs and the aggregate
retained-memory gate remain open.

SDK `key`/`on` now retain the contextual full Event as trailing `true`. Type and
runtime shape pairing includes its logical position; class inference and key
lookup include stored columns only. Generated bindings reproduce the full law,
including explicit full keys on closed scalar rosters. Measured partition,
singleton, ground-coverage and reopen tests cover this consumer slice.

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
[Exact Event selections](../docs/event-selections.md) now retain complete
canonical identity through native judgment, Rust byte literals, managed core/log
SDK schemas and generated bindings. Their tests include empty/nonempty zero-mass
values. [Closed Event rosters](../docs/event-ground.md) now retain portable identity,
owned values, checked ground keys/coverage and execution-bound query rows.
Their Rust/SDK, persistence, point-read, Free Join and managed log paths are
covered; native Ground proofs state context retention and explicit binding/codec
faithfulness obligations. Event capacity projections remain guarded;
the historical synchronous descriptor/log-schema handles still refuse Event
literals. Managed history/cache/admin/transition paths now admit copied schema
envelopes on workers, with command recovery and backup/restore integration tests.

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
Complete source/SDK integration remains acceptance work. Nested bound
fixed-point query integration is implemented in the slice below. A surjectivity certificate never substitutes for a complete-fibre-square
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
Complete Event operand diagnostics now preserve stage/rule/head/occurrence
coordinates and canonical expected/offending bytes in the SDK.
[Scoped query binders](../docs/event-query-binders.md) now add nested least/greatest
solutions with sealed carriers, conservative variance and cumulative work budgets.
Captured source plans now construct truth guards and explicitly lift/descend
Events in native/macro/Node/SDK queries, with common bound-predicate rosters.
Certified factoring, dynamic/multiple-source refinement query construction and
complete parameter-region consumers keep M5 open.

## M6 — Exact sources, observation and TypeSafe imports

**Native area:** source registry/constructors, exact arithmetic and contraction,
Probability/Expectation heads and ordinary import metadata. Final Probability
and final integer Expectation heads now have native implementations; typed
observation staging, exact arithmetic and predicate/guard consumers are present.
Exact rational imports and finite/family function payoff query slots use checked
local covers. Multivariate source solving/binding and TypeSafe imports remain open.

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
Family transport is described below; provider provenance remains an explicit
integration obligation. SourceTransport adds 14 reference reports.

The SDK now constructs exact BERA scalars and BESC finite functions, including
signed arithmetic, weighted images, strict law designation and owned probability/
expectation observations. Inputs and denominators survive runtime release. One
arithmetic budget covers all embedded measured inputs and the operation; exact
Coup posteriors and signed utility survive persistence/reopen. SDK kernel
construction now checks all parent fibres, including zero-prior states; closure
preserves the supplied prior marginal and exposes its onto parent map. Explicit
conditioning, likelihood and Jeffrey operations retain owned replayed receipts,
complete impossible outcomes and posterior-to-prior translation. A prepared Coup
query composes parent and revision pullbacks after database/runtime release.
Parameter families, provider provenance and native query observation/aggregate
heads remain open; this does not close M6 or M7.

[Named-parameter polynomials](../docs/event-polynomials.md) now supply the
canonical sparse rational arithmetic layer, simultaneous substitution, exact
evaluation, explicit Beta moments and BEPL v1 identity. Twenty-three additional
Lean reports describe the rational-assignment algebra; native tests distinguish
shared prior allocation, independent allocations and endpoint evidence failure.
This is arithmetic groundwork. Subsequent slices below add domains, functions
and source laws; multivariate solving and complete observations remain open.
Parameters remain separate from sampled outcome coordinates. The
[TypeScript polynomial API](../docs/event-sdk-polynomials.md) now exposes native
BEPL authoring, exact algebra, simultaneous substitution, rational evaluation,
explicit Beta moments and owned descriptions through cancellable workers. Five
SDK tests include correlated-prior persistence/reopen and late cancellation;
two native bridge tests cover shared arithmetic and malformed/unused operands.
The [SDK parameter layer](../docs/event-sdk-parameters.md) now adds exact regions,
inhabited domains, numerical roots and partial functions with inherited holes.
The [live family SDK](../docs/event-sdk-families.md) now adds those source/function
builders, exact signed observations and individual guard refinements.
[Family dynamics consumers](../docs/event-sdk-family-dynamics.md) now add channels,
revisions/restrictions and common refinement rosters. Ten SDK tests include exact
piecewise targets, unsupported domains, zero-prior channel rows and persisted
joined posterior queries; one native bridge test checks shared replay arithmetic.
Final [Probability](../docs/event-query-probability.md) and
[Expectation](../docs/event-query-expectation.md) heads are now implemented.
Observation staging/arithmetic and the other full M6 gates remain open.

[Exact root isolation and signs](../docs/event-algebraic-roots.md) now supply the
univariate real-algebraic capability, with checked root descriptions, exact
comparison, bounded Sturm/GCD arithmetic and an independent exact SymPy fixture.
Fifteen additional Lean reports cover algebraic/order obligations; they do not
formalize the complete root solver. This progresses irrational witness support
without substituting a rational grid. Subsequent slices add univariate domains,
guarded functions and source transport; multivariate solving remains open.

[Univariate domains and guarded rational functions](../docs/event-parameter-domains.md)
now retain exact sign regions, inhabited ambient domains and partial-function
arithmetic with original denominators and inherited holes. Sixteen further Lean
reports describe logical-cell and generic-field obligations; native tests retain
the shared-bias ratio's zero-evidence endpoints. This progresses domain/function
semantics. The source layer below adds normalized laws, parameter-aware ownership
and transport. Multivariate solving and query observations remain open.

[Canonical real-algebraic and named-region transport](../docs/event-algebraic-identity.md)
now minimizes boundary polynomials and records ordered real-root indices (BEAR
v1), then essential boundaries and membership for exact regions (BEPR v1).
Every decoded identity is rechecked; bounded factor-search exhaustion is a
refusal, never an irreducibility claim. Independent exact fixtures check 143
numeric identities. Nine Lean reports expose the factor-search and uniqueness
premises; they are not a refinement proof of the arithmetic/byte implementation.
These formats supply stable domain identity for the source layer below.

[Shared univariate Event sources](../docs/event-parameter-sources.md) now own an
inhabited parameter domain, deterministic guards and optional fibre-normalized
rational-function law. Admission checks every feasible fibre; contraction sums
outcomes without treating guards as random alternatives. Owned conditional
functions retain zero-evidence holes, and BEVT v3 retains the complete validated
source through ordinary schema fields, FD/INDs, storage and both Free Join paths.
Equivalent-partition maps/products retain one actual parameter. SDK v3 carriers
and `atomCount` preserve the distinction between logical cells and actual worlds.
Sixteen further Lean reports state quotient, map and contraction obligations.
Explicit [common guard refinement and lifts](../docs/event-parameter-refinement.md)
now preserve old Events, outcomes and laws while admitting new predicates. Exact
descent checks representability; readout refinement reconstructs checked maps.
Six native tests include an exhaustive 64-Event descent oracle and nine new Lean
reports state the logical projection/denotation obligations. A database consumer
splits a stored partition and retains its family through both Free Join paths.

[Domain inclusions and family conditioning](../docs/event-parameter-conditioning.md)
now capture a smaller inhabited parameter domain without deleting outcomes,
retain its law, and admit checked maps into an appropriately refined prior.
Every source cell must equal a whole target cell; unreachable parameter cases
retain ordinary image/vacuity behavior. Family conditioning solves the exact
positive-evidence domain, revalidates the posterior law and owns a receipt with
the original evidence-mass function. Posterior-zero outcomes remain structurally
possible; all-zero evidence has no posterior. Seven core tests, two persisted
Free Join/FD/IND consumers and fifteen Lean reports cover this slice.

[Family functions and likelihood revision](../docs/event-family-functions.md)
now add exact signed parameter-dependent values on Event cells, with totality
checked on every active fibre. Arithmetic, sign/refinement, weighted images,
law designation and expectations retain the same parameter and source context.
Image reuses the finite symbolic multiplicity kernel; explicit domain inclusions
give zero on unreachable target parameter cases. Likelihood receipts retain the
supplied factor, scale and normalizer, and all outcomes on the positive-normalizer
domain. Eleven tests cover all four-world maps, restricted-support controls,
62-coordinate images, exact payoff functions, repeated factors and explicit
refusals; seventeen Lean reports state generic-field factoring and update laws.
[Family channels and Jeffrey replacement](../docs/event-family-dynamics.md)
now add everywhere-normalized conditional channels, exact numerical FD descent,
prior-marginal preservation, ordered replacement targets and per-cell unsupported
parameter regions. Piecewise targets retain original-prior translations;
zero-target conditionals are skipped without erasing possibilities. Nine tests,
a persisted Free Join consumer and ten additional Lean reports cover this slice.
[Family source transport](../docs/event-source-descriptors.md) now adds BESC v2
for partial/total functions, channels, refinements, restrictions and full replayed
revision receipts, with exact domains and requested impossible-update identities.
Ten core tests, the persisted imported family consumer and twenty Lean reference
reports qualify this transport. Refinement query instructions, scope-changing
maps, multivariate/general function solving, explicit prior integration, TypeSafe
intents and observation query heads remain required M6 work. The univariate
family SDK now transports the existing native functions, channels and receipts.

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
expectation now returns a source-owned signed rational observation. Its
[shared-parameter counterpart](../docs/event-parameter-expectations.md) also
returns an exact signed conditional function with the original payoff, evidence,
numerator and evidence-mass function. Six core tests include 432 independent
finite-oracle cases, exact profitable-case Events and irrational domains; twelve
Lean reports state pointwise cell contraction and conditional definedness.
`FamilyFunction` now admits guarded rational payoff values depending on the
parameter, with the same owned conditional observation contract. Final integer
expectation/coverage integration is implemented. Exact reachable possibility
memory now lowers to the existing action arena; its descriptor/SDK/query
integration and continuous-source certificates remain required by this gate.
Rational and finite/family function payoff imports are now implemented. Nested bound fixed-point query
integration is now implemented.

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
Fully observed finite strategies are covered by those denotations.
[BeliefMemory](../crates/bumbledb-event/semantics/BeliefMemory.lean) now proves
exact possibility updates for complete action/observation histories and the
enabledness/knowledge contracts used by the native reachable memory constructor.
The hidden state must contain all information affecting future transitions.
It does not identify equal-support posterior laws or claim all hidden-goal
winning objectives are equivalent to eventually knowing the goal.
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


## Native final probability query checkpoint

[Final Probability heads](../docs/event-query-probability.md) now cross the Rust
macro, IR, Free Join, owned answer, Node worker and typed SDK boundaries. Pair
identity survives projection/deduplication/grouping; fixed ratios and partial
family functions retain original evidence and exact positive-evidence domains.
Fourteen QueryProbability reference reports extend the native Lean roster to
547 across 33 modules. No Rust refinement or performance proof is claimed.

The native final-only boundary must still expand to staged observations and
query arithmetic. Rational/function payoff slots, remaining solver/prior/import
work and all other unfinished M0–M8 gates stay required. No release or version bump.


## Native final expectation query checkpoint

[Final Expectation heads](../docs/event-query-expectation.md) collect integer-payoff
rosters through Free Join, validate complete structural coverage on evidence,
and retain exact fixed/family observations. Multiple expectations compose with
ordinary scalar aggregates and the existing single Pack head. Scratch spill,
operand provenance, shared arithmetic and owned SDK results preserve the same
source/evidence contract. Twenty QueryExpectation reference reports extend the
native roster to 567 reports across 34 modules; they are not Rust refinement.
Observation staging/arithmetic, rational/function payoff slots and all other open
M0–M8 requirements remain required. No release, tag or version bump.

## Native reachable belief-memory checkpoint

[BeliefMemory](../docs/event-belief-memory.md) now retains exact possible-world
Events across public actions and observations. It checks all endoroles and
shared environments, requires uniform action enabledness, and closes only
reachable belief Events under explicit resource limits. Compilation produces an
ordinary ActionArena with knowledge/possibility target regions. The native
database fixture stores memory, transition, winning/policy regions through
ordinary dependencies and joins after reopen. Eighteen additional Lean reference
reports bring the native roster to 585 across 35 modules. No Rust refinement or
performance claim is added. Memory descriptor/SDK/query construction, complete Coup consumers and all other
open M0–M8 gates remain. Equal-support posterior laws must remain distinct when
an application uses probabilities to choose actions.


## Native scoped query-binder checkpoint

[Query binders](../docs/event-query-binders.md) now solve nested monotone
least/greatest equations from row inputs, including relation-valued regions.
Full BEVT scopes retain original support/laws/guards; bound predicate depths are
separate from database variables. Complete external-operand admission precedes
iteration. SDK callbacks close hygienically to owned inspectable IR, and
interiors/Pack/tests/final Probability consume ordinary resulting Events.
Shared per-head/binding work budgets cover nested binders and Star; cancellation
and exhaustion publish no approximant. Five native integrations, SDK/Node/parser
consumers and two budget/cancellation unit tests cover this slice. Twenty-one
QueryBinders reference reports bring the native roster to 606 across 36 modules;
these do not verify the native interpreter or SDK closure algorithm.
All other open M0–M8 gates remain required. No release, tag or version bump.


### Explicit univariate prior checkpoint

[Source-owned Beta binding](../docs/event-priors.md) now implements the N07
polynomial moment fragment for total raw univariate observations on full `[0,1]`.
Finite point deviations are retained as exact exception certificates; they do
not change Event equality or fill function holes. Evidence is integrated before
normalization. Native query observations and SDK Event/payoff inputs retain
the original source, raw functions and conditional domains. BESC v2 tag 6
replays explicit source/shape admission, with persistence and both Free Join
paths exercised. PriorBinding adds eighteen reference reports with explicit
analytic premises, not Rust refinement.

Multivariate source binding and all other open M0–M8 gates remain required.
This host binding does not expose a source-changing marginalization map. This does not close M6 or authorize a release.

### Local function-cover checkpoint

[Function covers](../docs/event-function-covers.md) now assemble finite and
parameter-dependent local observables with exact agreement on overlaps and
complete structural coverage of evidence. Owned original patches distinguish
supplied zeros from missing values. All contexts participate, including empty
and redundant patches; zero mass never waives admission. Host masking/gluing,
SDK reconstruction and a persisted database consumer on both Free Join paths
are implemented. Fifteen FunctionCovers Lean reference reports establish the
measure-independent denotation; they do not verify the native implementation.

The consumer glues query-produced patches on the host. Native rational/function
payoff query slots and observation staging/arithmetic remain required. This
foundation does not close M5/M6/M7 or reduce any other M0–M8 acceptance gate.

### Exact column-ratio payoff checkpoint

[`Expectation(Ratio(numerator, denominator), when, given)`](../docs/event-query-expectation.md)
now reads ordinary integer columns into an exact rational payoff. Rust macros,
native `PayoffExpr`, complete-binding Free Join collection, strict Node transport
and owned SDK authoring/description imports share the contract. Equal fraction
presentations union by exact value. Every participating divisor is checked,
including an empty `0/0` patch, after the complete source-context fault pass.

Raw fraction claims remain in scratch until finalization. One exact arithmetic
budget normalizes every roster, admits all partitions, then contracts every
probability and expectation. The existing owned result types, supplied zero
buckets, conditional parameter holes and atomic append rollback are retained.
Native tests include the full four-world payoff/evidence oracle for rational
values on both Free Join paths, persistence and signed/unsigned extremes. Six
additional QueryExpectation Lean reports cover presentation invariance and
participating definedness; they do not verify the Rust collector or normalizer.

This is the exact ratio-of-columns fragment. Arbitrary-precision rational
literals, functional payoff query slots, observation staging/arithmetic and all
other open M0–M8 gates remain required. No release, tag or version bump.


## Native function-payoff query checkpoint

[Final Expectation heads](../docs/event-query-expectation.md) now accept owned
arbitrary-precision rational constants and total finite/family function imports.
Relational regions assemble checked local covers: exact pointwise agreement on
overlaps and structural coverage on evidence precede contraction. Mixed rosters
promote scalar and finite functions explicitly while preserving the source.
Actual parameter boundary worlds and zero-mass worlds remain obligations.

Rust `use payoff`/`Payoff(import)`, strict copied Node ingress, shared import
admission budgets, pure SDK authoring/description imports and owned cover results
are connected. Faults identify imported functions without invented variables.
Thirty-two QueryExpectation reports now connect functional admission to coverage
and a pointwise FD; the native roster is 651 across 38 modules. They do not
verify the Rust/SDK implementation. Family result equality is presentation
identity; checked numerical equivalence remains an explicit operation.

Observation staging/arithmetic, multivariate/general solving and source binding,
parameter-changing query maps, TypeSafe provenance/visibility, retained-memory
policy, capacity projections, certified factoring, complete program/strategy/
memory transport, full Coup and integrated performance/ARM64 qualification remain
required. The full M0–M8 goal stays active. No release, tag or version bump.


## Observation-stage checkpoint

[Observation stages](../docs/event-observation-stages.md) now carry completed
probability and expectation values through Rust interiors. Query types separate
observation values from the producer/fold roles that created them. Free Join,
projection, antijoins and grouping use full observation identity, retaining
source/evidence/function ownership, zero-mass worlds and parameter holes.
Producer errors precede consumers; one exact observation budget spans stages
and the final head. Scratch inheritance and owned outputs preserve execution
boundaries. Eighteen new reference reports bring the native roster to 669 across
39 modules, without claiming Rust refinement or performance qualification.

The typed SDK now has a query value domain for completed observations, distinct
from stored schema fields. Imported variables, named interiors, same-kind identity
joins, grouping, antijoins, projection-only recursion, description replay and
owned decoding retain this domain independently of producer/fold roles. Scalar,
Event and external-parameter coercions refuse. SDK consumers preserve exact
source/evidence/function identity, family holes, closed-owner result lifetime
and producer errors before downstream filtering.

Numerical observation arithmetic remains required. All other open M0–M8 gates
above remain unchanged. No release, tag or version bump.


## Partial numerical host foundation

[Exact observation numbers](../docs/event-observation-numbers.md) now implement
source-retaining native host arithmetic on completed observations, with exact
partial functions and true/false/undefined predicates. Operands remain in a
bounded owned derivation; explicit numerical-domain restriction retains the
original source/evidence. Twenty-six new PartialNumbers reference reports state
the domain/partition/provenance contracts without claiming native refinement.

This is groundwork for numerical query instructions, not completion of the
observation arithmetic gate. Pure-data IR, macro authoring, owned query staging,
identity/transport and SDK integration remain required, alongside explicit
predicate guard/refinement and every other open M0–M8 obligation. No release,
tag or version bump.

## Portable numerical derivation checkpoint

Native BENO v1 imports now replay the complete numerical expression and original
observation leaves, with indexed payoff rosters and exact source/component
identity. All source/coverage admission, contraction and arithmetic share the
caller's counter. The parser rejects malformed/noncanonical data and obeys
envelope/source/expression limits; explicit stacks enforce the depth boundary.
The native reference roster is now 703 reports across 41 modules, including
eight abstract replay contracts. No Rust codec refinement or performance claim
is made.

This closes the host numerical transport prerequisite. Numerical query IR,
heads, execution-owned identity/staging, macro/Node/SDK authoring and result
decoding are still required, as are predicate guards/refinements and all other
open M0–M8 gates. No release, tag or version bump.


## Numerical query checkpoint

The [numerical query contract](../docs/event-observation-numbers.md) now exposes
completed numbers in native IR, Rust macros, the raw Node bridge and typed SDK.
The complete derivation is canonicalized before aggregation; execution-owned
one-word identities preserve sources, evidence and expression structure through
Free Join, grouping, antijoins, staged imports and projection-only recursion.
A shared arithmetic budget survives stage boundaries and observation contraction.
Owned BENO outputs and exact partial functions survive result paging and owner
closure. Consumers cover all numerical operators/components and domain imports,
undefined holes, malformed inputs, downstream filter refusals and spill rollback.

Ten new `NumberQueries` reference reports state grouping/count preservation under
faithful canonical encoding and cumulative primitive charging; they do not prove
native code or performance. The reference roster is 713 reports / 42 modules.
Predicate/parameter-region query integration and explicit guard/refinement
construction remain required. Every other open M0–M8 gate remains in force.
No release, tag or version bump.

## Native predicate guard checkpoint

[Owned observation predicates](../docs/event-observation-predicates.md) now retain
strict partial Boolean composition, exact truth partitions, explicit domains
and complete numerical origins. BENP v1 replays the whole typed expression under
combined numerical/predicate bounds and one exact-work counter. Native guard
construction either uses existing cells exactly or explicitly refines the source
while preserving actual worlds and its law. A persisted query-number consumer
checks full partition admission and both Free Join paths after owner closure.
Nineteen PredicateGuards reference reports bring the native roster to 732 across
43 modules; they do not prove the native solver, codec or query engine.

Predicate-valued query IR/staging/imports, query guard/refinement instructions and
Node/SDK consumers remain required. All other open M0–M8 gates stay in force.
No release, tag or version bump.


## Predicate query checkpoint

[Observation predicates](../docs/event-observation-predicates.md) now have native
IR/macro/Node/SDK query heads, explicit possibility/universality/totality tests,
complete BENP identity staging, imports, grouping and joins. Shared exact budgets,
combined predicate/numerical shape limits, producer errors, spills and answer
rollback are covered by native and cross-language consumers. Fourteen
PredicateQueries reference reports bring the roster to 746 across 44 modules;
they establish strict definedness and shared-witness quantifier laws, not Rust
refinement or performance.

Query Event guard/refinement construction and parameter-region consumers remain
open, as do all other unsatisfied M0–M8 requirements. The existing host guard
API is not evidence for those query opcodes. The goal remains active; no release,
tag or version bump.


## Captured guard query checkpoint

[Explicit guard queries](../docs/event-observation-predicates.md) now interpret
predicates on a fully admitted captured source. Holds, Fails and Undefined return
ordinary Event fields; Lift and exact Descend bridge a named deterministic
parameter refinement. Existing-source plans refuse unresolved cells. Native IR,
Rust macros, strict raw Node grammar, typed SDK authoring and description replay
share the operation contract. The five heads preserve source/law, undefined
endpoints and possible zero-mass outcomes through staging, persistence and both
Free Join paths. Empty guards remain values; transport context faults retain
written occurrences and producer errors survive later filters.

Five additional PredicateGuards reference reports establish partition and exact
descent laws, bringing the native roster to 751 reports across 44 modules.
They do not prove the native solver, codecs, planner or performance. Captured
single-source plans do not invent dynamic source identities or compute a common
refinement across unrelated row predicates. Those query consumers, multivariate
solving/source binding and every other unsatisfied M0–M8 obligation remain open.
No release, tag or version bump.


## Common bound-predicate refinement checkpoint

[Guard queries](../docs/event-observation-predicates.md) now resolve an explicit
nonempty predicate roster in one captured source. Canonical truth-region ordering
keeps the common presentation invariant under roster reordering and duplication,
while every written predicate retains its domain checks and numerical origin.
Native, macro, Node and SDK consumers construct ordinary Event cases, compose
them over one parameter assignment and retain independent undefined regions.
Existing-source refusal and exact lift/descent apply to the complete roster.

Five new reference reports characterize the common observation partition and
its refinement universal property; the native roster is 756 reports / 44 modules.
Native/SDK tests establish executable correspondence for the supported univariate
fragment, not solver completeness, minimum representation size or performance.
Dynamic/multiple-source query refinement and all other unsatisfied M0–M8 gates
remain open. The goal stays active; no release, tag or version bump.
