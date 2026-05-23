# PRD 13: Lazy GHT And COLT Layer

## 01. Status

Not started.

## 02. Severity

High performance architecture.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRDs 08, 11, and 12 first.

The implementer must build correctness tests against existing eager trie behavior.

The implementer must add counters proving lazy behavior.

The implementer must not replace all eager paths in one unsafe step.

## 04. Dependency Order

PRD 08 is mandatory because COLT should use compact scoped query images.

PRD 11 is mandatory because COLT must plug into executable FreeJoinPlan nodes.

PRD 12 is strongly recommended because factored plans expose the largest lazy-build gains.

PRD 14 depends on this PRD for batchable lazy access sources.

PRD 15 depends on this PRD for lazy build cost estimates.

## 05. Problem Statement

The engine eagerly builds sorted or hash trie structures.

Temporary LFTJ atom plans scan source relation images.

They copy selected values into new columns.

They build sorted trie levels eagerly.

Hash trie indexes build full hash structures eagerly.

This can dominate query time.

The Free Join paper identifies trie-building as a major reason WCOJ implementations can lose to binary plans.

COLT addresses this with lazy, column-oriented trie construction.

Bumbledb needs a lazy GHT/COLT layer over query images.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query_image.rs`.
- `crates/bumbledb-lmdb/src/sorted_trie.rs`.
- `crates/bumbledb-lmdb/src/hash_trie.rs`.
- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/free_join.rs`.

Relevant current regions:

- `sorted_trie.rs:84-112` for eager sorted trie build.
- `hash_trie.rs:304-349` for eager hash trie insertion and recursive count.
- `query.rs:6150-6254` for temporary atom relation and sorted trie build.
- `query_image.rs:910-925` for encoded column access by fact ID.
- `query_image.rs:707-721` for access image bytes.

## 07. Research Context

Free Join's GHT generalizes hash tables and tries.

COLT is a column-oriented lazy trie.

COLT starts from column data and fact identity offsets.

It builds internal trie levels only when iteration or lookup requires them.

If a relation is only iterated as a left cover, no auxiliary trie may be needed.

If a lookup touches only one branch, only that branch should be forced.

This avoids eager preprocessing for relations or branches pruned by earlier probes.

Bumbledb's relation images already provide column-oriented data.

That makes COLT a natural fit.

## 08. Desired GHT Interface

Define a small internal trait for generalized access.

It must support iteration over current-level keys.

It must support lookup by encoded key.

It must support estimating key count.

It must support exact key count when materialized.

It must support descending into child context.

It must support returning fact identity sets or ranges at leaves.

It must avoid copying full encoded facts.

It must work with scoped query images.

It must expose counters for forced work.

## 09. Desired COLT Representation

COLT node can be unforced.

Unforced node stores fact identity range or list plus remaining field order.

Forced node stores map from encoded key to child node.

Leaf node stores fact identities.

Column data remains in `RelationImage`.

Encoded values are read from columns on demand.

Fact identity references must be compact.

Small leaf sets can be inline.

Large leaf sets can be ranges or shared vectors.

No node stores full encoded fact bytes.

## 10. Set Semantics Requirements

Every relation fact appears at most once in a COLT leaf.

Exact duplicate relation facts do not exist by storage contract.

COLT must not introduce duplicate fact identities.

COLT must preserve repeated-variable constraints when used by atom matching.

COLT must preserve literal and input filters.

COLT must preserve predicate behavior.

COLT must produce the same result sets as eager sorted trie and hash trie paths.

## 11. Implementation Plan

Add a new module for lazy GHT/COLT rather than overloading sorted trie immediately.

Implement read-only COLT over `RelationImage`.

Start with one relation and one ordered field list.

Support `iter_keys` at root.

Support `probe` at root.

Support child contexts.

Support leaf fact iteration.

Add integration behind a new `NodeImpl::LazyGht` from PRD 11.

Keep eager LFTJ fallback available.

Do not remove sorted trie in this PRD.

## 12. Lazy Forcing Rules

Do not force a node until lookup or key iteration requires it.

If the node's remaining fields are a suffix needed only for leaf fact scanning, iterate facts directly.

If a lookup requests a key, force only the current node.

If a child is never touched, do not force it.

If an earlier Free Join probe rejects a branch, do not build deeper branch levels.

If a cover relation is only streamed, do not build a hash map for it.

## 13. Interaction With Compact Query Images

COLT must read scoped columns.

If a requested field is not scoped, planning is wrong and must fail in tests.

COLT may use compact access images for pre-sorted or keyed traversal.

COLT should prefer existing compact access image when it exactly matches needed key order.

COLT should fall back to column-driven forcing when no compact access exists.

Do not rebuild full relation images inside COLT.

## 14. Interaction With Factoring

Factored plans should force fewer COLT branches.

Add a test where factoring plus COLT avoids building a deeper level.

COLT counters should make that visible.

Do not make COLT depend on factoring for correctness.

Unfactored plans must still execute correctly.

## 15. Required Counters

Add `lazy_ght_instances`.

Add `lazy_ght_nodes_forced`.

Add `lazy_ght_root_forces`.

Add `lazy_ght_child_forces`.

Add `lazy_ght_keys_materialized`.

Add `lazy_ght_fact_ids_scanned`.

Add `lazy_ght_bytes_copied`.

Add `lazy_ght_eager_builds_avoided` if estimable.

Keep existing sorted trie counters for fallback.

## 16. Required Unit Tests

COLT over a one-field relation iterates keys correctly.

COLT over a two-field relation probes root correctly.

COLT child context iterates second-level keys correctly.

COLT leaf returns exact fact identities.

COLT does not force child nodes that are never queried.

COLT preserves duplicate-free fact identity sets.

COLT handles empty relation image.

COLT handles singleton relation image.

## 17. Required Query Tests

Single-relation projection works through COLT path.

Two-relation join works through COLT path.

Prefix lookup query works through COLT path.

Range-like plan falls back if COLT does not support range yet.

Factored clover-like query forces fewer nodes than eager build.

Results match eager sorted trie execution.

Prepared query works through COLT path.

Snapshot stability remains correct.

## 18. Required Differential Tests

Run existing representative queries with eager path and COLT path if test hook exists.

Compare exact result sets.

Include projection queries.

Include aggregate queries if aggregate execution can use COLT safely.

Include cyclic queries.

Include acyclic chain queries.

Include duplicate existential witness fixtures.

## 19. Performance Gate

Add a focused query where eager temp trie builds many levels.

COLT must force fewer nodes than eager equivalent.

COLT must copy fewer encoded bytes than eager temp relation build.

Exact result correctness remains mandatory.

Wall-clock improvement is desirable but not the only passing metric.

The primary passing metric is avoided materialization measured by counters.

## 20. Passing Criteria

Lazy GHT/COLT exists as an executable access implementation.

COLT stores fact identity references, not full encoded facts.

COLT forces nodes on demand.

At least one tested query executes through COLT.

At least one tested query proves avoided eager build work.

Eager fallback remains correct.

The global validation gate passes.

The query-focused validation gate passes.

## 21. Failure Modes

Building full trie levels at COLT construction time is a failure.

Copying full encoded facts into COLT nodes is a failure.

Returning duplicate fact identities is a failure.

Silently reading non-scoped columns is a failure.

Deleting eager fallback before COLT is fully covered is a failure.

Claiming performance improvement without counters is a failure.

## 22. Non-Goals

Do not implement vectorized batches.

Do not implement ARM NEON.

Do not delete sorted trie.

Do not implement full optimizer cover-cost selection.

Do not change storage layout.

Do not change public APIs.

## 23. Completion Notes

Document which query shapes use COLT.

Document fallback cases.

Keep eager-vs-COLT differential tests permanent.

This PRD supplies the lazy physical structure required by the Free Join rebase.
