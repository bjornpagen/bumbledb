# Decisions and remaining questions

This is the active question register. “Decided” fixes behavior for implementation;
it is not a claim that code exists. Older question lists are historical inputs,
not additional release requirements.

## Semantic and database questions: decided

| Question | Decision and reason | Contract |
| --- | --- | --- |
| What is Event? | Exact subset of one named inhabited finite universe; caller supplies the worlds. | [Value](proposal.md#the-value) |
| Is universe size part of identity? | Yes: `(UUID, count)` is the complete descriptor. No hidden global name registry. | [Value](proposal.md#the-value) |
| How do rows share a universe? | Explicit Event values plus parent-row containments; no model type parameter. | [Coup](coup.md#schema-and-its-actual-guarantees) |
| Are empty/full storable? | Yes. Empty is a value and cannot silently erase its row. | [Algebra](proposal.md#algebra-and-equality) |
| Can equality compare foreign scopes? | Yes, false; equal values include their complete descriptor. | [Algebra](proposal.md#algebra-and-equality) |
| Should a pair predicate throw on foreign scopes? | No. Classify DifferentUniverse so predicate placement remains safe. | [Classifier](queries.md#a-total-classifier-that-the-planner-may-move) |
| Fifteen relations or sixteen? | Fifteen occupancy cases within a universe; one foreign case over all Event values. Full masks use all sixteen bits. | [Classifier](queries.md#a-total-classifier-that-the-planner-may-move) |
| Does the signature replace the region? | No; higher-order witnesses and construction require actual members. | [Classifier](queries.md#a-total-classifier-that-the-planner-may-move) |
| Can constructors combine foreign scopes? | No; validate operands and report a typed mismatch. No implicit product or translation. | [Expressions](queries.md#small-explicit-value-expressions) |
| Is the first carrier symbolic? | No. Dense finite bitmap plus polarity and canonical zero; limits are explicit. | [Representation](storage.md#owned-value-and-resident-representation) |
| Is storage interned persistently? | No. Inline canonical row payload; generation-local query interning only. | [Rows](storage.md#durable-rows-inline-without-a-new-object-store) |
| How are hashes trusted? | They select buckets; exact content checks decide identity. | [Ownership](storage.md#owned-value-and-resident-representation) |
| Do Event FDs key exact sets or points? | Points, like interval FDs. Query equality still compares exact sets. | [Schema](proposal.md#the-schema-language) |
| Does an empty source need an IND target? | Pointwise coverage is vacuous, but a separate scalar IND can require the row. | [Admission](proposal.md#complete-admission-rules) |
| What happens on universe mismatch in a constraint? | Typed statement failure for that selected scalar group, even at empty/full. | [Admission](proposal.md#complete-admission-rules) |
| How are deletes checked? | Recompute affected final groups from surviving facts using existing indexes. | [Admission](proposal.md#complete-admission-rules) |
| Can interval uniqueness be reused? | Not unconditionally. Initially keep general Event-key fanout and existing safe witnesses. | [Uniqueness](proposal.md#where-interval-assumptions-must-change) |
| Multiple Event fields? | Allowed in a relation; at most one final region per FD/IND projection. | [Boundaries](proposal.md#deliberate-surface-boundaries) |
| Closed data and capacities? | Follow current variable-sized closed-data restriction; no Event duration or point-count capacity. | [Boundaries](proposal.md#deliberate-surface-boundaries) |
| When do Event expressions run? | For complete surviving body bindings; explicit stages define failure boundaries. | [Expressions](queries.md#small-explicit-value-expressions) |
| What does Pack emit? | One union per present group; preserve all-empty, do not invent absent groups. | [Pack](queries.md#pack-and-result-ownership) |
| Is ordinary recursion expanded? | No. Existing projection-only rules may carry values; new aggregation/constructors remain nonrecursive. | [Expressions](queries.md#small-explicit-value-expressions) |
| Are query results streamed during evaluation? | No. Preserve complete-result sealing and transactional page delivery. | [Lifetime](storage.md#host-and-execution-lifetime) |
| Does WorkContext bound memory? | No. Cancellation, checked allocation and explicit cost reporting only. | [Resources](storage.md#resource-honesty) |
| Is a new persistent index required? | No. Native exact predicates first; optional measured candidate directory later. | [Indexes](queries.md#index-and-kernel-work-after-correctness) |
| Can old experimental BEVT payloads be imported? | No automatic compatibility. Use a distinct finite-value family. | [Encoding](storage.md#proposed-finite-event-encoding) |
| Must every existing database migrate? | No. Preserve non-Event fingerprints and physical layout if the stated additive design holds. | [Compatibility](storage.md#compatibility-and-transport) |
| Do log command results gain Event? | No; that record intentionally remains scalar-only. Normal rows/results carry Event. | [Transport](storage.md#compatibility-and-transport) |
| What does a model score become? | An ordinary supplied fact. Joint membership/coupling requires caller information, not invented probabilities. | [Coup model boundary](coup.md#where-a-model-fits) |

## Integration questions: answer with bounded native probes

These are remaining implementation uncertainties, not invitations to redesign
the algebra. Each has a preferred approach, a test and a stopping rule.

| Probe | Preferred approach | Evidence required before proceeding |
| --- | --- | --- |
| P1: resolver generation and image width | Extend the concrete generation owner; ordinary two-word token comparison and separately pinned payloads. | S1 reopen/rotation/foreign-parameter tests and an early native Event filter. Stop if tokens can escape their owner or accidentally enter interval dispatch. |
| P2: checked construction across theory/core/Node | Immutable theory value plus bounded checked builder; core cancellation and existing owned queue capture. | S1 round-trip fixtures and mutation/lifetime checks, completed across hosts in S5. Freeze tag/family bytes. No new admission service or global quota layer. |
| P3: delta judge integration and diagnostics | Event case in shared compiled metadata; rescan affected final groups and use current citation machinery. | S2 full-versus-delta oracle including mixed scopes, empty source-only groups and replacement transactions. If a generic interval helper assumes endpoints, split that concrete case. |
| P4: group spill and output ownership | Mutable group unions, canonical partial-union spill, result-owned payloads. | S4 forced spill/repeated group merge and native owner release checks. Token-only spill or dangling result handles fail the gate. |
| P5: additive format compatibility | Inline field payload, append unused type/row tags, preserve existing fingerprints and layout. | S1 old-store fixture and codec review; S5 replay/transition and independent consumer fixtures. Any required incompatible change gets an explicit revised compatibility plan first. |

These probes belong inside the implementation sequence. They do not require a
new research lab. A failed assumption must yield a specific revised seam or
format decision, not automatic permission to build another subsystem.

## Empirical choices that do not block the contract

- Whether block unions reduce candidate visits enough to repay their build and
  retained-memory costs; exact scanning is always the fallback.
- How much batching, pair memoization and NEON improve end-to-end workloads;
  choose bounded concrete caches only when measured.
- How much inline disk duplication matters for real schemas; no object catalog
  is presumed necessary before measuring it.
- Whether later applications need a symbolic carrier. A concrete workload must
  justify it and include canonical export/equality costs.

## Deferred rather than unresolved

Native graph closure has a specified logarithmic-round schedule, but is a
separate query feature. Rooted/dynamic variants need their own workload and
scope. Event does not need closure to know its members.

Coordinate maps, quantified modalities, residuals, model/source adapters,
probability laws, parameter arithmetic, general fixed-point programs, strategy
synthesis, belief-memory and inference services are outside this implementation.
The external solver plus database architecture remains available without
embedding those systems in a value owner.

No new Lean development is required to choose these database contracts. Reuse
specific existing denotational results when helpful, and test native integration
where Lean did not prove it. The type is ready for a bounded implementation once
this design is accepted; it is not yet production-qualified.
