# Event implementation ledger

The complete revision 0.9 proposal is the target. This ledger records native
implementation evidence; it does not replace or reduce the M0–M8 acceptance
gates in `proposal/implementation-plan.md`. No release is authorized by this work.

Base: `d76d31ab` (public v1.3.1 plus removal of scratch allocation-count tests).
Implementation branch: `codex/event-algebra`. Owned core commit: `fca4ac667`.

| Gate | Status | Evidence / remaining work |
| --- | --- | --- |
| M0 contract baseline | Passed before native edits; current proposal audited separately | Historical revision 0.8 readiness record is preserved; revision 0.9 adds native identity and bounded fixed-point semantics |
| M1 owned canonical core | Implemented | Shared `bumbledb-event` crate; completed functions, essential tables, fallible construction, legal counts, saturation, checked alignment/registry, ownership and native cancellation |
| M2 structural type / persistence | Native structural and fixed-law slices implemented; full gate open | Event values/macros, canonical rows, image words, parameters/sets/literals, Free Join equality, spill and owned results; raw Node wire uses checked BEVT bytes. Finite structural BEDC descriptors now reconstruct checked owners; BEVT v2 retains checked finite laws; complete SDK, function/kernel transport and parameterized source descriptors remain |
| M3 dependencies / admission | Native field and contextual full projections implemented; full gate open | Pointwise overlap, union coverage, full partitions, final-state repair/deletion, empty-key lookup refusal and conservative planner witnesses work. Event selections, capacity positions and Event-valued closed rosters remain guarded |
| M4 faces / maps / relations | Finite structural relation core implemented; integration gates remain | Full face/fibre products, complete square certificates, membership-FD roles, converse, composition, modalities, both residuals, graph/readout conversion and safe diagram inspection/reconstruction. BEDC transport and typed relation queries are implemented; SDK and parameter/source integration remain |
| M5 query / Pack / Free Join | Complete-binding Event/Test, relation/readout programs and Pack implemented; full gate open | Boolean/ITE/cardinality, structural tests, six readout operators, typed relation operators and Star with captured faces/products, grouped union, scratch claims, staging and occurrence fault sets; general fixed-point binders, SDK diagnostics and certified factoring remain |
| M6 sources / inference | Fixed finite laws, channels and revisions implemented; full gate open | Exact arithmetic, normalized density partitions, owned conditional observations, BEVT v2, weighted images, checked conditional channels, conditioning/likelihood/Jeffrey revisions and owned receipts; shared parameters, solver capabilities, TypeSafe adapters and query observation heads remain |
| M7 information / fixed points | Finite information, partition, expectation, fixed-point and fully observed strategy core implemented; other gates open | Readouts, indexed partitions, typed programs, closure, enabled actions, uniform permissions and ranked/safe policies; signed host expectations; belief-memory construction and complete query/source integration remain |
| M8 qualification | Pending | Full battery, packaged Rust/TypeScript consumers, executable Coup fixtures, malformed descriptors, integrated performance and ARM64 evidence |
| Final commit / push | Pending | Complete implementation and curated evidence committed and pushed; no version bump, tag or release |

## Core and equality

`Value` belongs to the theory crate and must not depend on the storage engine.
The shared Event crate owns values, canonical managers and mathematical
operations; the engine supplies cancellation through `Control`/`WorkContext`.

An owned Event retains its manager. Rust `Eq`/`Hash` is scoped handle identity.
Independent owners require checked `align_to`; it is not an extensional global
comparison. Canonical persistent bytes identify the named source, original support,
designated law if present, and supported membership. A query registry aligns equivalent decoded values into
one namespace, so ordinary word equality implements Event equality there. Equal
counts or probabilities do not identify Events. Public order comparisons refuse.

## Native storage and ownership

[BEVT v1/v2](event-value-format.md) is embedded as a length-prefixed Event field in
canonical rows. The field occupies two ordinary resident words, not an interval
endpoint pair. Whole-fact deduplication uses canonical bytes, never resident keys.
Generated facts containing Event are owned and `Clone`; they are not `Copy`.
Event host-newtype syntax is not implemented.

Each typed or dynamic decoded row aligns its own Event fields through a lazy
registry. Independent rows remain independently owned and require explicit
alignment for standalone algebra. Query outputs use the execution registry and
own their returned Events, including pages and spill results after the snapshot,
executor and database are dropped. Missing/stale/unregistered keys refuse.
Database decode paths with a WorkContext retain cancellation; manual
`RowReader::new` and the legacy no-work scan use the uncancellable core control.

Resolver generations reclaim unreferenced text independently. **All registered
Events and canonical descriptors remain strongly retained until the generation
is released.** The default registry limit is 2,000,000 distinct Events; that is
an entry bound, not a byte budget. Images and executions pin their generation.
Cache rotation cannot invalidate retained results. Memory-pressure qualification
and a complete retained-byte policy remain acceptance work; eviction alone does
not reclaim Event history in a live generation.

The raw Node bridge transports canonical BEVT `Uint8Array` values, tagged `event`
where a tag is required. The worker serializes returned Events under its work
context; malformed input refuses in the shared core decoder. This is the native
wire foundation. The high-level TypeScript field/value/operation API and generated
SDK bindings are still pending; bindings generation refuses unsupported Event
schemas explicitly. The log JSON value spelling is `{"event":"<BEVT hex>"}`.
Command result records retain their existing scalar-only contract.

## Native dependency admission

An Event key routes on its ordinary scalar fields and rejects intersection
between distinct whole facts. Event containment compares each source region
with the union of matching target regions. Exact declared target keys are still
required. Canonical BEVT values hold the union/conflict summaries in the existing
scratch maps; independently decoded owners are checked and aligned before any
empty/full shortcut. A group retains its context even when its target is absent
and every source Event is empty. Different scalar groups may use different spaces.

Incremental admission rebuilds the complete final state of each affected Event
statement. This handles deletions and atomic insert-before-delete repairs;
affected-group scheduling is a future optimization. Conflict citations include
only facts meeting the conflict region, ordered by canonical whole-fact bytes.
Domain-law rejections and operational/context failures remain distinct. The
complete participating-operand fault set is implemented for Event/Test and Pack
heads as described below; typed relation programs retain their M5 gate.

The compiled `EventKeyMayBeEmpty` witness cannot authorize unconditional result
distinctness. Single-result keyed lookup refuses an empty Event unless the
supplied fields include an independently sufficient scalar key. Ordinary queries
continue to return empty-valued facts and deduplicate equal projected Events.
Cross-owner lookup equality uses canonical bytes. The small reference `MapState`
now also deduplicates Event meaning; insertion is fallible and reports whether
a fact was new, with a cancellable `insert_with_control` variant.

[Contextual `true`](event-projections.md) is now a typed projection term across
Rust macros, named/resolved descriptors, schema JSON, raw Node transport,
fingerprints, compiled metadata and admission. It establishes full coverage
without storing an Event owner. Full/full requires actual target presence;
full keys imply scalar uniqueness under the admitted nonempty-world contract.
Existing field-only fingerprints and permuted Event containment positions remain
unchanged. Closed scalar rosters may participate through declared full keys.
Event selections, capacity and Event-valued closed rosters remain guarded;
complete SDK authoring still awaits the later consumer milestone.

## Checked finite maps

The [native map contract](event-maps.md) supplies owned arbitrary Boolean
readouts between checked source/target spaces. Pullback uses simultaneous graph
substitution; image masks original support and abstracts source coordinates as
soon as remaining readouts no longer use them. Maps may copy, discard or combine
coordinates. Each side can contain 62 coordinates without a combined workspace.

Construction checks every readout and original target support. The separate
surjectivity certificate checks the exact support image, without asserting
complete projection-square fibres or preservation of a designated law. Map
results are ordinary owned Events and already persist/join using BEVT identity;
BEDC now carries checked finite map descriptors, including measured endpoint
Events. Captured map/face/product query heads are implemented below. Structural
map certificates make no law-pushforward assertion.

## Lean evidence and remaining correspondence

The proposal suite was rerun: **246 reports / 30 files, 160 axiom-free**.
The [current native semantic run](event-evidence/native-conditional-source-semantics/check.json) adds
**304 reports / eighteen files**, using only permitted standard Lean axioms.
Persistence proves support-relative identity and the registry-key law under
explicit admission/roundtrip premises. FixedPoint proves bounded stabilization,
least/greatest extremality, uniform shared environments and counterexamples for
negation and environment mixing. No `sorryAx` is accepted.

Admission proves the executable Boolean coverage/conflict accumulator against
distinct whole-fact incidence, pointwise-key and containment equivalences,
traversal independence and exact citation membership. It retains counterexamples
for duplicate contributors and subtracting deleted coverage. Seven additional
reports prove contextual full-key uniqueness, partition and presence laws and
show why an empty world space breaks them. These laws assume
one already-aligned legal world type and a duplicate-free fact roster; they do
not establish those native premises by themselves.

These prove reference denotations. They do not verify the Rust graph, encoder,
byte parser, mutexes, allocator, source solver or query compiler.
In particular, a finite-diagram node count is not a finite-world proof. Native
fixed-point admission counts original support and checks typed program variance;
the correspondence of native counting, DAG execution and equality to those
reference premises remains tested, not kernel-verified.

CoordinateMaps adds 26 reports for support admission, Boolean substitution,
both image adjunctions, surjectivity/occupancy, composition and the recursive
image reference with safe abstraction after last use. Its counterexamples keep
decoder aliases, absent fibres, image intersections and prematurely forgotten
readouts separate. These proofs neither certify every native map square nor
verify extraction of raw coordinate dependencies from the Rust graph.

Relations adds 22 reports for full legal products, joint-fibre certificates,
membership-FD descent, shared-witness lowering, both residual adjunctions and
graph/readout admission. The [native relation API](event-relations.md) implements
those finite structural operations. Its actual constructors and plans remain
tested Rust, not a machine-checked refinement of these denotations.

The old readiness record pins preimplementation production source. Do not
rewrite it to claim its isolation audit verifies the current engine. Revision
0.8 documents are preserved before revision 0.9 changes. The new current-proposal
auditor checks documentation/proof evidence without asserting Rust completion.

## Verification of the native slice

- Event core: 15 substantive tests, including exhaustive four-world supports,
  dense symbolic oracles, projection/counts, codec corruption, allocation/order
  changes, collisions, capacity/cancellation and owner lifetimes.
- Native Event storage: ten tests covering canonical dedup/reopen, cross-owner
  equality joins on resident/cursor paths, literals/parameters/set dedup, rollback,
  concurrent publication, malformed bytes, a pinned row fixture, same-row alignment,
  cancellation and retained pages. A separate internal test covers spill/stale keys.
- Rust query macro Event parameter test and native WorkContext test pass.
- Raw Node bridge: all 103 unit tests pass, including two Event tests for owned
  wire roundtrip, malformed/version refusal and worker cancellation. Node crate
  compilation includes every target. TypeScript checking and the wire-tag fixture
  test pass; this does not claim the complete packaged SDK has been qualified.
- Earlier regression run: 1,341 engine unit tests passed, 18 ignored; 25 API and
  five keyed-get tests passed. These precede the final same-row decoder change;
  the focused Event tests exercise that change. Log JSON: seven tests passed.

Formatting, whitespace checks and strict clippy pass for the core/engine and
the separate Node crate. The [native slice record](event-evidence/native-storage-qualification-final/check.json)
pins the checked source state and retained logs. The
[proposal handoff](event-evidence/implementation-handoff-0-9-final/check.json)
separately records documentation, proof and reference checks.
This evidence qualifies the described slice, not the entire proposal. The final
packaged-consumer battery and integrated performance qualification remain M8.
The local proposal laboratory is preserved and will be curated explicitly for
Git; it is not blanket-added or deleted.

The [native admission qualification](event-evidence/native-admission-qualification/check.json)
records the later field-dependency implementation and its checks. Its independent
four-world oracle covers 768 partition cases across full and nonrectangular
supports. Additional tests cover foreign/empty contexts, multiple groups,
canonical duplicates, final-state repair, rollback, deletion, keyed reads,
Free Join projection deduplication, stable citations and reference-state identity.
A separate internal test forces Event summaries through scratch spill, oversized
keys, context refusal and cancellation. This supersedes the earlier native
storage record for the changed admission paths, without rewriting that record.

The [full projection qualification](event-evidence/native-full-projection-qualification/check.json)
extends the field-dependency slice with contextual constants. Fourteen tests
(including three generated closed-enum checks) cover 768 additional partition
cases across three supports, empty/foreign contexts, full-only keys, exact target
resolution, scalar permutations, closed rosters, atomic repair/deletion and
reopening. Schema-file and real N-API tests retain the typed marker and refuse
malformed constants. The [updated handoff](event-evidence/implementation-handoff-full-projection-final/check.json)
audits the current proposal and 246 proposal plus 37 native proof reports.
These records do not close M3, later source/face/query milestones or M8 performance.

The [native map qualification](event-evidence/native-map-qualification/check.json)
adds nine core map tests, including 61,440 exhaustive map/predicate cases and a
bounded 62-coordinate symbolic case, plus two database integration tests for
storage, Free Join equality, ownership and cancellation. The
[map handoff](event-evidence/implementation-handoff-maps/check.json)
reconciles current documents with 246 proposal and 63 native reports. Full M4
and the complete M0–M8 implementation target remain open.

The [native relation qualification](event-evidence/native-relation-qualification-final/check.json)
adds thirteen relation tests and a database persistence/join consumer. It checks
all 4,096 two-state R/Q/V triples for both adjunctions and all 1,024 R/Q/E triples
for the guarded Must law, alongside asymmetric supports, role failures, nonlinear
environments, descriptor mismatches, graph/readout recovery, algebraic identities,
capacities and a 60-bit symbolic workspace. The
[relation handoff](event-evidence/implementation-handoff-relations/check.json)
records the current proposal and 246 proposal plus 85 native proof reports.
This does not close descriptor transport, source/query integration or M8.

The [diagram inspection API](event-inspection.md) now exposes original support,
completed membership, borrowed table words and safe shared-node traversal through
an immutable owned snapshot. Checked reconstruction preserves named context and
legal support across working orders. Borrowed views hold no manager lock. Ten
Inspection reports prove an acyclic signed-graph reconstruction reference under
explicit constructor correspondence; they do not verify native graph copying.
Seven native tests include a separate bitset interpreter, arbitrary supports,
62-coordinate symbolic traversal, independent lifetime and resource refusal.
The [inspection qualification](event-evidence/native-inspection-qualification/check.json)
and [updated handoff](event-evidence/implementation-handoff-inspection/check.json)
record this extension. Query/source integration, descriptor transport and the
remaining M0–M8 gates stay open.

The [finite information API](event-information.md) adds possible/guaranteed/ambiguous
Events, nonvacuous evidence cases, unique FD factorization through onto readouts
and checked indistinguishability relations. Its environment gate prevents the
product from silently changing observation cells. Nineteen Information reports
prove reference lowering, evidence/FD and relation-kernel contracts. Eight native
tests include independent partition oracles and 62-bit symbolic readouts; the
database consumer uses information after reopening and closing its owner.
The [information qualification](event-evidence/native-information-qualification/check.json)
and [information handoff](event-evidence/implementation-handoff-information/check.json)
record this extension. These operations implement part of M7.

The [finite program and fixed-point API](event-fixed-points.md) now adds owned,
inspectable typed instruction DAGs with checked monotone admission. Finite carriers
count original legal worlds; bounded iteration includes exact equality detection
and refuses budgets/cancellation without returning a partial solution. Derived
operations implement CanReach, InevitablyReach, SafeThroughout and Star with
checked endpoint/environment roles and one shared middle witness. Programs has
13 variance/reference reports; Closure has 14 denotational reports; FixedPoint
adds nine legal-roster and detection reports. No arbitrary host callback can
bypass admission.

The [fixed-point qualification](event-evidence/native-fixed-point-qualification/check.json)
records **63 core tests and five doctests**, 1,343 engine tests (18 ignored),
Event integrations, workspace and Node checks, and strict clippy. Eleven new core
tests cover truth functions, finite relational oracles, environment/role refusal,
budgets, ownership and symbolic carriers up to 2^62 worlds. The database consumer
persists computed answers, reopens, joins on resident/cursor paths and retains
owned results after closing the database. The
[fixed-point handoff](event-evidence/implementation-handoff-fixed-points/check.json)
audits that checkpoint against 246 proposal plus 150 native reports. The partition
extension below supplies finite observable rosters. Measured expectation, action availability/uniform strategies,
descriptor transport, full query programs and integrated performance remain open.

The [finite partition API](event-partitions.md) adds an indexed, owned Event
roster with explicit parent coverage and disjointness. Scalar observable labels
stay in ordinary columns. Admission clips to evidence; grouping, shared-world
refinement, pullback, checked readout conversion and complete cardinality buckets
preserve the partition contract and empty positions. Thirteen Partitions reports
prove those reference laws and keep arbitrary image separate from pullback.

The [partition qualification](event-evidence/native-partition-qualification/check.json)
records **70 core tests and six doctests**, 1,343 engine tests (18 ignored),
Event integrations, workspace/Node checks and strict clippy. Seven new core tests
cover exhaustive finite oracles, evidence, contexts, readouts, resource refusal,
empty cells and a 62-bit symbolic carrier. The database fixture uses native
pointwise dependencies to admit stored count/value rows and reject a coverage
gap; reopen and owner-lifetime checks preserve explicit empty buckets.
The [partition handoff](event-evidence/implementation-handoff-partitions/check.json)
audits 246 proposal and 163 native reports.

The [action and strategy API](event-actions.md) adds checked Step(state, action,
outcome) roles, enabled choices, controlled predecessors and greatest uniform
one-step permissions on inhabited observation cases. Fully observed reachability
retains first-entry rank partitions and every action that strictly decreases
rank. Continuing safety retains enabled actions that preserve its invariant.
Supplied policies can restrict these choices only while preserving coverage.
No state visibility, probability policy or belief-memory update is inferred.

Nineteen Actions reports prove reference quantifier order, permissions,
rank-bounded termination for every policy choice/outcome, policy restriction and
safety. Three FixedPoint reports prove first-entry uniqueness/coverage. The
[action qualification](event-evidence/native-action-qualification/check.json)
records **78 core tests and seven doctests**, 1,343 engine tests (18 ignored),
Event integrations, workspace/Node checks and strict clippy. It includes
exhaustive two-state policy/permission oracles and a symbolic 20-bit arena.
The [action handoff](event-evidence/implementation-handoff-actions/check.json)
audits 246 proposal and 185 native reports. Belief/information-memory construction,
measured expectation, exact sources/adapters, query/SDK/descriptor integration
and the complete M8 performance/consumer gates remain open.

The [native Event/Test query heads](event-queries.md) now construct owned regions
and six structural predicates in the Rust macro/IR and raw Node descriptor path.
All sixteen truth functions, ITE, complement and fixed-roster cardinality preserve
participating operands through scope admission. Empty results remain values.
Interiors seal before consumption. Complete context-fault sets retain canonical
source/support/value bytes and logical stage/rule/find/leaf identities; DNF
collapse retains every written-rule stamp. Resource and scalar refusals never
claim to be complete fault sets. Existing scalar first-error semantics remain.

Query.lean adds **22 reference reports** for participation, same-world counts,
empty closure, schedule invariance and complete-versus-partial fault reporting.
The [query qualification](event-evidence/native-query-qualification/check.json)
records core/engine/integration/macro regressions, strict workspace/Node clippy,
Node tests, TypeScript checking and actual N-API grammar admission/refusals.
The [query handoff](event-evidence/implementation-handoff-query-heads/check.json)
audits that checkpoint against 246 proposal plus 207 native reports. The Pack
extension below supersedes its query coverage; the evidence is unchanged.


Grouped Event Pack now emits one union per present group, retaining empty values
and all participating written provenance. Its canonical minimum context is
independent of physical join order. Computed group-key faults and Pack context
faults are accumulated together; undefined keys do not create groups, and later
valid keys still participate. Exact claims and wide group keys use native scratch,
with inline claim keys preserving group traversal after spill.

Pack.lean adds **19 reports** for union/presence, canonical minima in a lawful total
order, context admission, provenance, saturation and partial-key participation.
The [Pack qualification](event-evidence/native-pack-qualification/check.json)
records the expanded query integrations, forced spill/cancellation, regressions,
strict workspace/Node clippy and actual Node Pack execution. The
[Pack handoff](event-evidence/implementation-handoff-pack/check.json) audits
246 proposal plus 226 native reports. M5 remains open for face imports, typed
relation programs, SDK diagnostics and certified factoring. Sources, complete
SDK/descriptor transport, partial-observation memory, packaged Coup consumers,
integrated performance and the full M0–M8 acceptance gates remain unfinished.


[BEDC v1 descriptor transport](event-descriptors.md) now captures finite maps,
surjectivity requests, face/pair products, relation regions, composition plans
and complete squares as pure data. Import checks full space markers, reconstructs
original supports and endpoint orientation, and reruns every certificate gate.
Syntax parsing returns no executable certificate. Names remain authored; import
performs no implicit source allocation. Descriptor extents and kernel capacities
are explicit refusals, with cooperative cancellation throughout.

Descriptors.lean adds **eleven reference reports**, ten axiom-free, for legal-code
reconstruction and the support/role/environment premises it requires. The
[descriptor qualification](event-evidence/native-descriptor-qualification/check.json)
records seven new core tests, the strengthened database reopen/join consumer and
workspace/Node regressions. The
[descriptor handoff](event-evidence/implementation-handoff-descriptors/check.json)
audits 246 proposal plus 237 native reports. This implements finite host transport;
query face imports, SDK consumers, measured-source descriptors, programs/strategy
transport and all remaining M0–M8 gates retain their scope.

Captured readout queries now retain checked `EventImport` owners and their
canonical BEDC data. Rust `use map` and raw Node map descriptors expose pullback,
image, universal/nonvacuous image and possible/guaranteed regions. Map boundaries
supply input and output contexts; every syntactic leaf occurrence retains its
own context demand. Incompatible declared outputs refuse before body execution.
Independent equivalent owners align explicitly, and no query operator infers
source identity, independence or a new probability law.

QueryMaps.lean adds **18 reports**, 11 axiom-free, for scope-indexed evaluation,
whole-program congruence from demanded inputs, complete occurrence validation
and map denotations. The [readout qualification](event-evidence/native-query-map-qualification/check.json)
records five new database integrations, compile-fail coverage, strict workspace
and Node checks, TypeScript parser tests and eight actual map execution cases.
The [readout handoff](event-evidence/implementation-handoff-query-maps/check.json)
audits 246 proposal plus 255 native reports. These remain reference proofs with
explicit Rust correspondence obligations. Full relation query programs, SDK
diagnostics, certified factoring and the other M0–M8 gates remain open.

Typed relation query programs now retain `use faces`/`use product` imports.
The pure-data `RelationExpr` supplies binding, identity/tests, Boolean operations,
converse, composition, both residuals and finite Star. Event views expose Region,
Domain, Range, May, All, Must and Post. Composition/residual/Star syntax requires
the authored product plan; no result namespace or shared workspace is invented.
Static role checks compare actual environment readouts and original support.
Converse retains coordinate orientation; equivalent independently named products
reindex through the checked host algebra. Per-occurrence context validation
precedes all value evaluation, including dead ends and constant results.

QueryRelations.lean adds **eleven reports**, eight axiom-free, proving typed
whole-program transport under legal endpoint bijections, both residuals, finite
paths and modal transport. The [relation-query qualification](event-evidence/native-query-relation-qualification/check.json)
records seven database integrations, 4,096 complete and 512 asymmetric/coupled
product triples, role/ownership/fault checks, all 31 actual N-API relation scenarios
and eight malformed/context refusals. The
[relation-query handoff](event-evidence/implementation-handoff-query-relations-final/check.json)
audits 246 proposal plus 266 native reports. General bound fixed-point query
programs, complete SDK diagnostics, measured sources, certified factoring and
the other full-plan gates remain open. No performance ranking or release follows.

## Exact finite laws and measured persistence

The [finite source API](event-sources.md) now designates normalized nonnegative
rational joint densities over Event regions. Equal densities merge canonically;
zero-mass admissible worlds remain structural possibilities. Contraction uses
legal counts, including skipped outcome bits, and returns exact rationals.
Conditional observations retain their source, numerator and evidence mass;
undefined conditioning remains distinct from MissingLaw and empty Events.

BEVT v2 carries the designated finite law; version 1 is unchanged. Alignment,
diagram rebuilding, registry identity, storage and Free Join include the law.
Structural restriction explicitly produces an unmeasured context. Products
remain unmeasured, and map certificates do not establish law pushforward.
Arithmetic has explicit conservative bit/step limits; law pieces and canonical
payload bytes have separate bounds. These are not total retained-memory quotas.

FiniteDensity adds 20 reports (six axiom-free) for count/density contraction,
regrouping, complement, evidence and skipped-outcome multiplicity. The module
uses common-denominator natural numerators and an exhaustive legal-world roster;
it does not verify the bigint library, graph counts or BEVT v2 parser. Twelve
new core tests and three database integrations include 61,440 conditional
observations, a 62-bit symbolic law, law-sensitive joins, zero-mass dependency
admission/rollback, malformed sources, cancellation and independent ownership.
Actual N-API execution checks measured values and refuses unnormalized laws.

The [finite-law qualification](event-evidence/native-finite-law-qualification/check.json)
and [finite-law handoff](event-evidence/implementation-handoff-finite-laws-final/check.json)
record that slice with 246 proposal plus 286 native reports. It left conditional
channels to the later implementation below. Shared parameters, solver-backed
constructors, source revisions, TypeSafe imports and owned query observations
remained at that checkpoint. Later sections record subsequent work; full M0–M8
qualification is not complete.


## Conditional sources through exact weighted images

[FiniteFunction and FiniteKernel](event-functions.md) now supply canonical signed
rational functions, pointwise algebra, pullback and symbolic weighted image.
The image kernel masks support and sums each source coordinate exactly once,
after its last remaining readout use. It preserves copied-readout correlation
and supports 62-bit source/target spaces without a combined coordinate arena.

Conditional-law admission checks unit row mass at every legal parent, including
zero-prior rows. Closing under an explicit prior retains its old marginal and
returns a measured extension plus the parent readout/surjectivity certificate.
An FD check detects dependence on information omitted by an actor readout.
Fresh outcome extensions remain distinct from copying or reusing an Event.

WeightedMaps adds 18 reports (four axiom-free). Eight core tests check 3,840
four-world maps, signed values, categorical encodings, symbolic images, scope
and resource faults. The native Coup channel obtains 92/147 and 5/21 after Tax;
a database consumer reopens those worlds and computes translated intersections
through captured map query heads on both execution paths.

The [conditional-source qualification](event-evidence/native-conditional-source-qualification/check.json)
and [conditional-source handoff](event-evidence/implementation-handoff-conditional-sources/check.json)
record 246 proposal plus 304 native reports. That checkpoint left source revisions,
shared parameters, solver capabilities, TypeSafe adapters and observation heads open;
function/kernel SDK and descriptor transport also remain explicit work.


## Fixed-law revisions and signed expectations

[Explicit revisions](event-revisions.md) now distinguish Event conditioning,
nonnegative likelihood factors and Jeffrey posterior replacement. Each retains
its prior and mathematical receipt; successful outcomes retain a checked
identity-on-worlds translation. Zero evidence/normalizers and unsupported positive
targets are owned impossible outcomes. Neither zero posterior mass nor revision
changes structural admissibility. Likelihood scale remains visible; Jeffrey
revision has no intrinsic evidence probability.

Finite-function expectation now retains the signed payoff, evidence, numerator
and evidence mass. It checks contexts and missing laws even for zero payoffs.
The total function's authored zero default does not discharge a query observable's
separate grouping, distinct-value disjointness and coverage obligations.

Revisions adds 30 exact rational Lean reports; the native suite now has nineteen
modules. The new proofs cover normalized/nonnegative revisions, likelihood scale/composition, target
masses, within-cell ratios, same-partition idempotence and signed contraction.
Nine core tests include 1,280 conditioning/expectation cases, 240 partition
revisions, symbolic 62-bit support, owned outcomes and explicit refusals. The
persisted Coup consumer translates old Events into the posterior on resident
and cursor paths, and measures signed utility after external owners drop.

The [revision qualification](event-evidence/native-revision-qualification-wakeup/check.json)
and [current handoff](event-evidence/implementation-handoff-revisions-final/check.json)
record 246 proposal plus 334 native reports. These are denotational proofs,
not Rust extraction or a performance result. Receipt/function/kernel transport,
parameter families, TypeSafe adapters, query observations/expectation, SDK
completion and the other M0–M8 gates remain open. No release follows.

The first [revision qualification attempt](event-evidence/native-revision-qualification/check.json)
exposed a Node runtime cleanup race: supervisor reclamation removed a cancelled
operation without waking workers waiting for its owner. Reclamation now notifies
those waiters for both successful and error outputs. A focused concurrency test
[passes with the wakeup and fails when it is removed](event-evidence/native-revision-runtime-wakeup-checked/check.json).
The failed run remains evidence; the final qualification above includes this fix.

An earlier negative-control attempt revealed that `wait_timeout_while` can
observe the changed predicate only at its deadline and still report success.
The regression now checks the direct wait timeout, so it actually distinguishes
a delivered wakeup from a deadline poll. Both attempts are retained.
