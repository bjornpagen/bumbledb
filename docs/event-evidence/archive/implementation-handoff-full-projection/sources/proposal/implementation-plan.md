# Event implementation sequence and acceptance gates

**Revision 0.9.** Implement the full [Event contract](proposal.md) in ordered
milestones. The public algebra stays fixed throughout; a milestone is not a
weaker substitute for the final design. Existing research is sufficient to begin.
The shelved participation experiment and further carrier competitions are optional.

The baseline is public BumbleDB v1.3.1. Implementation is underway on
`codex/event-algebra`; the [native ledger](../docs/event-implementation.md) records
the owned core and unmeasured storage/binding slice. This plan specifies the full
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
identity argument tied to the implemented encoder/decoder. The unmeasured slice
now has [BEVT v1](../docs/event-value-format.md), a pinned native row fixture,
ownership/publication tests and denotational persistence proofs. Remaining M2
qualification includes complete consumer APIs and the descriptor extensions
required by M4/M6. Scope each passing gate to the supported format; unmeasured
tests cannot qualify future law descriptors.

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

## M5 — Query heads, Pack and certified Free Join schedules

**Native area:** [query macro](../crates/bumbledb-query-macros/src/lib.rs),
[IR](../crates/bumbledb/src/ir.rs),
[preparation](../crates/bumbledb/src/api/prepared/build.rs),
[computed outputs](../crates/bumbledb/src/api/prepared/computed.rs),
[aggregate sink](../crates/bumbledb/src/exec/sink/aggregate.rs), planner.

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

**Proof anchors:** FactorizedPack, ParticipatingValidation, RelationPack,
QuerySeparator. No theorem presently verifies native IR extraction or costing.

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
source solving, Jeffrey/Pearl arithmetic and source-adapter correspondence retain
reference tests and explicit native obligations.

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

**Proof anchors:** InformationReadout, ReadoutMaps, ModalContract and
[FixedPoint](../crates/bumbledb-event/semantics/FixedPoint.lean). The latter proves
the finite iteration bound, least/greatest extremality and the uniform shared
environment rule. Native admission, variance checking, resource behavior and
refinement to that reference iteration remain required. General game strategy
synthesis is not claimed proved.

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
