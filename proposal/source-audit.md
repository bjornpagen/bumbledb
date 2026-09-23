# Source audit and retained research

Audit baseline: `d76d31abca00226bc817a549d7365f1638f4b33c`.
Historical source: `79c46299a71edd7a8c0fd9e48bef5369bdec2548`.
This is a design audit, not a claim that the new type is implemented.

## Search coverage

Searched all 543 restored documents/manifests for identity, dependency, query,
storage, host, resource and unresolved-question terms. Searched 1,370 tracked
text files across crates, TypeScript packages, docs and scripts for type
boundaries, constraint dispatch, query operators, ownership, formats and host
codecs. Hits guided the targeted source reads below. This is not a claim to
have reread every document line, every paper, or every test in the repository.

No new external-paper results are asserted. Retained literature reviews and
source manifests supply the mathematical background; current code determines
integration requirements. Historical source/evidence links may refer to files
not restored, as the [reference index](reference/README.md) explains.

## Findings that changed the plan

| Current source | Finding | Proposal consequence |
| --- | --- | --- |
| [Theory value](../crates/bumbledb-theory/src/value.rs), [type definition](../crates/bumbledb-theory/src/schema.rs) | One shared value sum; field widths describe resident layouts. | Add a concrete value/type, preserve layer direction, audit exhaustive consumers. |
| [Canonical rows](../crates/bumbledb/src/canonical.rs), [row lookup](../crates/bumbledb/src/storage/store/rows.rs) | Rows own variable-sized payloads; fingerprints only route exact comparisons. | Inline durable Event bytes, no persistent dictionary, collision tests. |
| [Text resolver](../crates/bumbledb/src/image/intern.rs), [generation owner](../crates/bumbledb/src/work/cache.rs) | Generation-scoped tokens and separately pinned immutable owners. | Reuse that lifetime model, including rotation and reclamation. |
| [Image widths](../crates/bumbledb/src/image.rs), [encoding](../crates/bumbledb/src/encoding.rs) | Interval WordPair dispatch is not a generic two-word type. | Event needs typed ordinary multiword equality plus its own classifier. |
| [Store format](../crates/bumbledb/src/storage/store/format.rs), [schema tags](../crates/bumbledb/src/schema/wire.rs), [fingerprint](../crates/bumbledb/src/schema/fingerprint.rs) | Physical layout 7, schema family v6; type tags and row tags are distinct. | Keep existing bytes stable; freeze an additive Event encoding and verify old stores. |
| [Schema validation](../crates/bumbledb/src/schema/validate.rs), [compiled theory](../crates/bumbledb/src/schema/compiled.rs) | Exact target-key matching, last/one-region restrictions, closed text refusal, scalar capacity rules. | State Event's corresponding validation rules rather than relying on width alone. |
| [Final-state judge](../crates/bumbledb/src/schema/judge.rs) | Full and affected-group paths use shared compiled access; interval cases sweep endpoints. | Add Event summaries to those paths; do not send Events into interval sweeps. |
| [Distinctness](../crates/bumbledb/src/plan/fj/provably_distinct.rs) | Complete interval uniqueness relies on nonempty values. | Initially retain fanout/dedup for complete Event keys. |
| [IR](../crates/bumbledb/src/ir.rs), [placement](../crates/bumbledb/src/plan/fj/validate.rs), [view predicates](../crates/bumbledb/src/image/view.rs) | Allen residuals bind early and have constant-column forms. | Native Event predicates must have that placement; total scope classification makes movement safe. |
| [Overlap enumeration](../crates/bumbledb/src/exec/run/overlap_leaf.rs), [Allen kernels](../crates/bumbledb/src/exec/kernel/allen.rs) | Candidate access and exact residual checks are separate; optimization has eligibility rules. | Exact Event path first, scoped optional block directory with mask-specific pruning. |
| [Aggregate spill](../crates/bumbledb/src/exec/sink/aggregate/spill.rs), [row folding](../crates/bumbledb/src/exec/sink/aggregate/fold_row.rs) | Interval claims spill by representation limits; errors stay sticky until finalize. | Explicit Event-union state/merge/ownership, no invented byte-budget guarantee. |
| [WorkContext](../crates/bumbledb/src/work.rs), [complete results](../crates/bumbledb/src/api/prepared/result.rs) | Cancellation only; full evaluation before paged delivery. | Correct the previous “existing budgets” claim; no partial publication or streaming claim. |
| [Reach driver](../crates/bumbledb/src/api/prepared/reach.rs), [query IR](../crates/bumbledb/src/ir.rs) | One-self-atom, projection-only frontier expansion. | Graph doubling is separate work, not something already expressed by RecStep. |
| [Schema macro](../crates/bumbledb-macros/src/lib.rs), [query macro](../crates/bumbledb-query-macros/src/lib.rs) | Explicit field grammar and literal masks; staged query IR is the existing extension route. | Small Event field/predicate/head additions; no independent language. |
| [TS fields](../ts/src/fields.ts), [values](../ts/src/values.ts), [query lowering](../ts/src/query/lower.ts), [wire capture](../ts/crate/src/db_wire/codec.rs) | Typed host validation and owned input capture precede queued work. | Distinct Event value, buffer mutation tests, ownership probe rather than assumed worker placement. |
| [Native tags](../ts/crate/src/tags.rs), [schema files](../crates/bumbledb-log/src/schema_file.rs), [bindings](../crates/bumbledb-log/src/bindings.rs), [JSON](../crates/bumbledb-log/src/json.rs) | Multiple exact consumers of the shared type/value grammar. | Include tags, generated bindings, round-trip descriptions and replay in completion gates. |
| [Command-result codec](../crates/bumbledb/src/canonical/result.rs) | Intentionally scalar-only; intervals are refused. | Event rows are supported without expanding that unrelated result language. |

## Historical findings retained, narrowed or rejected

| Historical material | Disposition |
| --- | --- |
| [Dependency review](reference/proposal/experiments/event-repr-lab/DEPENDENCIES.md) | Retain coverage/conflict summary, exact target keys, final-state deletion reasoning and real conflict citations. No persistent summary tree is required. |
| [Storage semantics](reference/proposal/event-storage.md) | Retain empty-value closure and the uniqueness counterexample. Remove law owners and contextual full projection machinery. |
| [Signature calculus](reference/proposal/experiments/event-repr-lab/SIGNATURE-CALCULUS.md), [classifier](reference/proposal/experiments/event-repr-lab/CLASSIFY.md) | Retain direct occupancy readouts and higher-order limitations. The new foreign class is a database-boundary decision; old fifteen-bit table/proof claims are not automatically proofs of the revised whole-type behavior. |
| [Representation report](reference/proposal/experiments/event-repr-lab/REPORT.md), [word measurements](reference/proposal/experiments/event-repr-lab/WORD-CLASSIFIER-MEASUREMENTS.md) | Retain dense Coup motivation and representation-aware word kernels. They do not qualify the new native predicate path or select a universal winner. |
| [Pack measurements](reference/proposal/experiments/event-repr-lab/PACK-NATIVE-MEASUREMENTS.md) | Retain distributive factoring with scalar separation/presence premises and a cost choice. It is an optional later optimization. |
| [Representation literature](reference/proposal/research/representation-search.md) | Retain canonicality/export-cost cautions. SDDs, diagram maps and decomposition choices do not become initial backends. |
| [Compact counting](reference/proposal/research/compact-counting.md) | Retain the boundary: compact structural data does not guarantee cheap probability readout. No counting/probability solver in the type. |
| [Decomposition and reachability](reference/proposal/research/block-decomposition.md) | Retain the distinction between rooted reachability and materializing full closure. Do not transplant a symbolic fixed-point algorithm into the first Event slice. |
| [Old native query contract](reference/docs/event-queries.md) | Retain present groups and stage ownership. Reject complete-binding-only structural predicates as sufficient integration; reject captured maps/programs and global fault collection. |
| [Old wire formats](reference/docs/event-value-format.md) | Retain portability/canonicality obligations. Do not import BEVT source laws, graph codecs or parameter admission; the finite carrier has its own family. |
| [Old implementation plan](reference/proposal/implementation-plan.md), [decision list](reference/proposal/decisions.md) | Audit prompts only. Their M0–M8 requirements, parameter/source APIs and strategy/memory systems are not active tasks. |
| [Saved Typesafe primitives](reference/proposal/research/event-algebra/typesafe-primitives.md) | Context for caller-supplied judgments. No claim that a marginal Noul specifies joint Event membership. |

The remaining archived iterations, reviews, maps, source-law, information,
parameter and proof reports were included in the topic search. Their distinct
systems are explicitly deferred in the [decision register](decisions.md), rather
than silently forgotten or converted into release obligations.

## Verification boundary

The restored reference tree remains byte-for-byte historical. Active links,
anchors, manifest hashes and unchanged baseline production files are checked
for this revision. No old benchmark battery or Lean suite was rerun.

Independent finite reference checks accompany the design: occupancy/masks,
polarity and proposed byte encoding, coverage/conflict summaries, the 4,290-world
Coup fixture, and the graph-doubling recurrence against per-world reachability.
These check specified mathematics and example data. They do not compile the
proposed macro syntax or establish native storage, performance, memory safety,
cache invalidation, or ABI compatibility. Those obligations have named gates
in the [implementation plan](implementation-plan.md).

The ad hoc reference run passed: 15,376 Event pairs (including foreign scopes),
43,648 binary possibility checks, complement/converse checks for all 65,536
masks, 594 codec round-trips and ten malformed-value refusals, 4,096 admission
triples, and the Coup partitions/15 seeded role groups/567-world steal result.
The closure recurrence matched a per-world Warshall oracle on 4,096 exhaustive
two-vertex cases, 1,024 sampled graphs and ten chain/roster cases. A maximal-u64
universe with a zero payload was checked without allocating its world set.
These small reference runs are not an implementation or a new test framework.
