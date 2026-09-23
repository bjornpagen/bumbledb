# Implementation plan and acceptance gates

This is a plan for implementing a type, not a resumed implementation goal.
Production source remains at `d76d31ab`. The completed deliverable is the
[Coup example](coup.md) running through normal Rust and TypeScript database
paths with exact values, constraints, native predicates and owned results.

The [decision register](decisions.md) fixes the semantics. The
[source audit](source-audit.md) identifies the existing mechanisms to extend.
Do not restore the historical Event implementation wholesale.

## S0 — Freeze the contract and its small oracle

Before runtime work, make these explicit and independently checkable:

- Descriptor/value identity and canonical zero/dense polarity representation.
- Fifteen same-universe signatures plus DifferentUniverse; total mask negation.
- Whole-value equality versus pointwise key/coverage semantics.
- Empty rows, absent groups, constructor failure domains and Pack participation.
- Exact wire bytes and the unchanged non-Event compatibility baseline.

The documentation revision supplies finite reference checks. During
implementation, carry the relevant small cases into the existing conformance
suite; do not start another research package or proof-count target. Use a
plain set-of-worlds oracle, not the bitmap functions under test.

**Gate:** no unresolved semantic branch in the decision register. Proposed
payload/tag fixtures are frozen during S1 before a durable Event is published.
No implementation milestone depends on new probability or SAT research.

## S1 — Prove the value fits the existing database paths

Add the checked theory value/type and concrete core operations. Extend the
schema field/macro, canonical codec, resident field layout and generation
resolver. Establish ownership through image loading, parameters and results.
Use inline durable payloads; no new persistent database or object catalog.

Build a tiny stored relation with an Event field, a scalar key, equality lookup,
and one native INTERSECTS comparison. Wire that comparison into the actual
Free Join node before building the rest of the operator surface. This forces
token layout, scope classification, parameter resolution and predicate placement
to agree early.

**Acceptance:**

- Equal values from different construction orders insert as one fact; forced
  hash collisions do not collapse unequal facts.
- Equality queries work after reopen, cross-store logical import and cache
  rotation. Different universes remain unequal at empty/full.
- A previously unseen structural parameter is evaluated, not treated as an
  equality miss. Changed parameters between prepared runs change the answer.
- The INTERSECTS plan runs before a later fanout atom; execution visit counts
  confirm pruning at that point. Scalar results alone do not prove placement.
- Held results survive query/image release and a concurrent writer snapshot.
- Non-Event rows, fingerprints, stores and public fixture bytes remain unchanged.
- Malformed/old Event payloads fail checked parsing before publication.

**Questions this gate must resolve:** exact generation ownership, typed two-word
layout dispatch, theory/core builder layering, and the native host's owned input
handoff. If the current seam requires a broader change than described in
[storage](storage.md), document that concrete obstacle before expanding scope.

Each commit must leave the workspace building. Exhaustive enum consumers may
need typed temporary refusals while a boundary is unfinished; no panic, silent
blob reinterpretation, or wildcard “supported” answer. Such refusals prevent
calling the whole type complete.

## S2 — Keys, containments and transactional integration

Extend schema validation and the shared compiled projection table with an
Event region case. Preserve scalar routing, exact target-key resolution,
selection semantics, region-position restrictions and statement-local universe
alignment. Use the existing final-state judge and affected-group visitor.
Initially recompute affected groups; no persistent coverage maintenance.

**Acceptance:**

- Independent finite-set cases agree with both full and incremental judges.
- Coup's Slot mirrors and both Holding keys pass; overlap, gap and foreign-scope
  transactions fail with valid existing-style row/statement citations.
- Insert/delete cancellation, idempotent reinsertion, replacement in one delta,
  target deletion and surviving coverage all have correct final-state outcomes.
- Empty rows remain stored, obey scalar INDs and retain distinct payloads. Two
  distinct empty facts sharing a pointwise key cannot license a unique probe
  or incorrect aggregate deduplication removal.
- Different universes in unrelated groups coexist; mismatches within a
  participating group fail even if every contribution is empty.
- Selected-out rows do not enter an IND; source-only empty groups behave as
  specified; target-key declarations are checked exactly.
- Closed Event fields, mixed/multiple region projections and Event duration/
  capacity misuse receive deliberate typed schema refusals.

## S3 — Complete native predicates and structural value operations

Complete all masks, converse/complement behavior, constant/column and
column/column forms, membership, equality-set parameters, negated atoms,
projection and grouping. Keep pair classification total across scopes. Add
small Event computed heads through ordinary stages, with iterative depth checks
and the declared evaluation domain for partial constructors.

**Acceptance:** all masks and binary construction truth tables agree with the
finite oracle; atom-order/DNF variants give the same filtered result; empty,
full, tail-word and foreign cases pass. A three-way common-world query must
reject the pairwise-overlap counterexample. Partial construction failures are
not hidden by constant folding or downstream filters. No body bindings produce
no constructed values. Changing non-Event comparison semantics is a regression.

## S4 — Pack, spill and complete result delivery

Add Event union groups to the aggregate sink, using mutable accumulators and
one final canonicalization per resident group. Merge spilled partial unions
through the existing scratch facility. Keep group presence, universe checks
and output ownership explicit. Do not retain every historical claim just to
recover descriptors after saturation.

**Acceptance:** seeded and unseeded groups, all-empty groups, no-input global
Pack, duplicates across written rules, wide group keys, and forced repeated
spill partitions match the reference union. A foreign final input after full
must still fail. Canceled execution produces no sealed result; canceled delivery
does not advance a cursor. Save a computed result and use it in another query
after dropping its original query and snapshot.
Include Event-valued group/dedup keys after their source stage is released;
pinning the resolver generation alone must not be mistaken for payload ownership.

Inspect live ownership in the forced-spill test: persisting tokens while pinning
all payloads does not prove that group state was released. This gate promises
correct spill behavior, not a global process-memory quota.

## S5 — Complete public boundaries and compatibility

Finish Rust macro/type inference and structural IR, TypeScript fields/values,
query lowering/description replay, native marshaling, diagnostics, schema-file
rendering and generated bindings. Thread Event through change sets, log row
replay, checkpoint/backup/restore and existing schema transition paths. Retain
the log's scalar-only command-result gate.

**Acceptance:**

- The same schema/query runs through Rust and typed TypeScript, with identical
  canonical Event bytes and results.
- Independently authored fixtures test row and Event formats, including malformed
  flags, lengths, tail bits, zero-count worlds and unsupported families.
- Mutating a JS input/output byte array cannot change an admitted value or a
  queued write. Cancellation and runtime release preserve current ownership rules.
- Schema/query descriptions round-trip; generated bindings retain Event type
  identity; error categories are represented in the native and SDK rosters.
- Baseline non-Event stores/histories remain readable. Event-bearing logical
  replay and transition succeed; schema changes never reinterpret old rows.
- Pack, result ownership and predicates pass the packaged consumer checks,
  not merely in-tree unit tests.

Run the repository's appropriate complete battery after integration. Do not
rerun it repeatedly without a relevant change or failure. Historical Lean
proofs may justify retained laws, but do not verify these native boundaries.

## Performance qualification and the stopping point

After S1–S5, measure the complete Coup workload and a small set of counter-shapes:
selective overlap, saturated overlap, disjointness, mixed universes, many repeated
regions and many unique regions. Include tiny and multiword universes, wide
scalar groups, updates, result delivery, first use and reuse.

Report canonical row bytes, resident payload bytes, token/cache metadata and
query/result ownership separately. Measure non-Event and Allen regressions.
Compare the native scan to an optional block-union directory, charging build
cost. Compare scalar to NEON only within the same correct execution path.
Index thresholds, pair memo capacity and vector width are empirical questions;
none changes the public algebra. Disable optimizations that lose.

**Done means:** native Event values, dependencies, predicate placement,
constructive stages, Pack, persistence and supported hosts pass these gates.
Stop there. Faster SIMD, factoring, symbolic carriers and the separate
[closure proposal](queries.md#separate-query-proposal-event-labelled-reachability)
are not reasons to prolong the type implementation.

## Documentation-stage verification

The first restart verified all 543 restored reference files against their Git
blobs; this revision rechecks provenance and active links. Ad hoc finite checks
cover the revised classifier, canonical polarity/encoding, admission summary,
Coup worlds and graph-doubling recurrence. Results are recorded in the
[source audit](source-audit.md#verification-boundary). No native Event code,
new benchmark result or new Lean proof is claimed by this documentation work.
