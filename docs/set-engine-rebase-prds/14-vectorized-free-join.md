# PRD 14: Vectorized Free Join Execution

## 01. Status

Not started.

## 02. Severity

High performance architecture.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRD 11 and PRD 13 first.

The implementer must preserve scalar fallback.

The implementer must validate every batch size against exact result sets.

The implementer must not add parallel execution.

## 04. Dependency Order

PRD 11 is mandatory because vectorization must execute FreeJoinPlan nodes.

PRD 13 is mandatory for lazy access sources that can expose batch operations.

PRD 09 and PRD 10 should be complete so sinks can accept partial payload events.

PRD 15 depends on vectorized cost metrics.

PRD 16 depends on vectorization counters.

## 05. Problem Statement

Current join execution is scalar.

It opens an iterator.

It finds one key.

It binds one value.

It evaluates predicates.

It recurses.

This creates poor locality and high overhead.

The Free Join paper proposes vectorized execution by batching iterate and probe work.

Bumbledb has no real vectorized join execution today.

Some sink methods mention batch-like projection, but the join itself remains scalar.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/free_join.rs`.
- `crates/bumbledb-lmdb/src/query_access.rs`.
- `crates/bumbledb-lmdb/src/query_image.rs`.
- `crates/bumbledb-lmdb/src/sorted_trie.rs` if sorted trie gets batch seek.
- `crates/bumbledb-lmdb/src/hash_trie.rs` if hash trie gets batch probe.

Relevant current regions:

- `query.rs:5585-5664` for scalar LFTJ recursion.
- `query.rs:8068-8203` for sink batch hooks that are not full join vectorization.
- `query_access.rs:7-32` for narrow access probe abstraction.
- `hash_trie.rs:207-234` for prefix exists/count/facts.
- `sorted_trie.rs` iterator methods for scalar seek/open/next.

## 07. Target Behavior

Free Join node execution can process batches of keys or bindings.

Batch size is deterministic and internally configurable.

Batch size one is equivalent to scalar execution.

Batch execution groups probes by access source.

Batch execution filters failed probes before deeper recursion.

Batch execution feeds projection sinks with multiple result facts when safe.

Batch execution feeds aggregate domain events with multiple events when safe.

Result sets remain exact and canonical.

## 08. Research Context

Vectorized execution improves locality by processing blocks of values.

The Free Join paper applies this to generalized iterate/probe plans.

Instead of recursing immediately for each key, the executor collects a batch.

It probes all batch keys against the next access source.

It removes failed bindings.

It proceeds with the survivors.

This avoids repeatedly bouncing between unrelated memory regions.

Bumbledb's columnar query image and COLT structures should support this naturally.

## 09. Definitions

Batch is a bounded collection of partial bindings or node keys.

Batch key is the encoded value or key group used for a probe.

Batch survivor is a partial binding that passed all probes at the current node.

Scalar fallback is the existing one-binding execution path.

Batch sink is a projection or aggregate sink method accepting several payload events.

Batch size one is the canonical correctness baseline.

## 10. Invariants

Batch execution must produce exactly the same result set as scalar execution.

Batch execution must not depend on nondeterministic ordering for correctness.

Batch execution must not emit duplicate result facts except through explicit set sinks.

Batch execution must preserve aggregate domain dedup across batches.

Batch execution must preserve snapshot stability.

Batch execution must preserve predicate semantics.

Batch execution must preserve overflow behavior.

Batch execution must preserve error behavior.

## 11. Batch Representation Plan

Add an internal `BindingBatch` or equivalent.

Store encoded values in columnar vectors where practical.

Store validity mask or survivor list.

Store current depth metadata.

Store projected payload readiness metadata if PRD 09 is complete.

Store aggregate event readiness metadata if PRD 10 is complete.

Avoid cloning large encoded values repeatedly.

Use small fixed-width encodings efficiently.

Keep a simple fallback representation for initial implementation.

## 12. Access API Plan

Extend access abstraction with batch operations.

Add `probe_batch` for hash/GHT sources.

Add `iter_batch` for cover iteration.

Add `seek_batch` only if sorted trie can support it safely.

Batch APIs may default to scalar loops initially.

At least one real implementation must avoid pure scalar dispatch by the end of this PRD.

Expose fallback counters.

## 13. Executor Plan

Select a node.

Collect up to batch size cover keys or partial bindings.

For each probe in the node, compute batch probe keys.

Probe the access source for all batch keys.

Remove failed bindings.

Apply ready predicates in batch.

Emit ready projection payloads in batch if PRD 09 supports it.

Emit ready aggregate events in batch if PRD 10 supports it.

Recurse or loop on survivors.

Fallback to scalar when a node implementation lacks batch support.

## 14. Predicate Batch Plan

Evaluate fixed-width comparisons over vectors where simple.

Start with scalar predicate evaluation over batch survivors if necessary.

Keep counters for vector-capable and scalar predicate paths.

Do not introduce approximate comparison behavior.

Do not change encoded ordering rules.

Do not add architecture-specific SIMD in this PRD.

## 15. Projection Batch Plan

Projection sink must accept a batch of encoded projected facts.

It must dedup across previous batches.

It must count seen, inserted, and duplicate result facts accurately.

Batch emission must preserve final canonical output ordering.

Batch emission may insert into a set and sort at finish.

Do not require every variable to be bound if PRD 09 supports early projection.

## 16. Aggregate Batch Plan

Aggregate sink must accept batch domain events.

It must dedup domain events across batches.

It must preserve overflow behavior.

It must preserve min/max behavior.

It must avoid decoding unnecessary values for count-domain.

It must interoperate with PRD 10 early aggregate events.

## 17. Required Counters

Add `vector_batches_started`.

Add `vector_batches_completed`.

Add `vector_batch_values_in`.

Add `vector_batch_values_survived`.

Add `vector_probe_batches`.

Add `vector_probe_keys`.

Add `vector_scalar_fallbacks`.

Add `vector_predicate_batches`.

Add `vector_sink_batches`.

Expose configured batch size in plan diagnostics.

## 18. Required Correctness Tests

Batch size one equals scalar output.

Batch size two equals scalar output.

Batch size ten equals scalar output.

Default batch size equals scalar output.

Projection duplicates across batch boundaries dedup correctly.

Aggregate domains across batch boundaries dedup correctly.

Predicate filters across batch boundaries remain correct.

Prepared query output remains correct.

Snapshot stability remains correct.

## 19. Required Performance Tests

Add focused join fixture with enough facts to create multiple batches.

Assert vector batch counters are non-zero.

Assert scalar fallback counter is not equal to all probe work for at least one path.

Assert exact result correctness before checking counters.

Wall-clock timing improvement is desirable but not mandatory for this PRD.

The mandatory metric is real batched probe or iteration work.

## 20. Configuration Requirements

Batch size must have a deterministic default.

Batch size must be controllable in tests.

Do not expose a public unstable API unless necessary.

Do not use environment variables for core correctness tests.

Do not make batch size affect result correctness.

Batch size zero must be rejected or treated as one.

## 21. Passing Criteria

At least one Free Join execution path performs real batch probe or batch iteration.

Batch size one matches scalar behavior.

Multiple batch sizes produce identical result sets.

Projection and aggregate sinks dedup across batches.

Vectorization counters prove batched work.

Scalar fallback remains correct.

The global validation gate passes.

The query-focused validation gate passes.

## 22. Failure Modes

Only batching final output while join remains scalar is a failure.

Changing result sets based on batch size is a failure.

Dropping aggregate domain events across batches is a failure.

Duplicating projected facts across batches without counting is a failure.

Introducing nondeterministic correctness behavior is a failure.

Adding architecture-specific SIMD here is a failure.

## 23. Non-Goals

Do not add ARM NEON in this PRD.

Do not add parallel execution.

Do not change storage layout.

Do not build a full cost model.

Do not remove scalar fallback.

Do not change public query API.

## 24. Completion Notes

Document supported vectorized node implementations.

Document fallback cases.

Keep batch-size differential tests permanent.

This PRD turns Free Join from scalar recursion into a batchable execution framework.
