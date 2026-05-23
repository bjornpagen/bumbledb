# PRD 01: LFTJ Atom Cache Correctness

## 01. Status

Not started.

## 02. Severity

Critical correctness.

## 03. Owner Model

This PRD is designed for a single implementer.

The implementer must write failing tests first.

The implementer must not alter unrelated optimizer behavior.

The implementer must not delete LFTJ.

The implementer must preserve cache reuse for genuinely identical atom restrictions.

## 04. Dependency Order

This is the first PRD in the rebase suite.

No later query-cache, trie, optimizer, or Free Join PRD may proceed while this bug remains open.

PRD 02 may be implemented in parallel only if PRD 01 tests are not changed.

PRD 13 depends on the cache-key contract established here.

PRD 15 depends on cache diagnostics remaining meaningful after this fix.

## 05. Problem Statement

The LFTJ atom trie cache can return a trie whose contents were filtered for a different query predicate.

The current atom trie build path can filter facts using local comparison predicates.

The current atom trie cache key does not include those local comparison predicates.

The current atom trie cache key does not include encoded input values used by local comparison predicates.

This means the trie cache can turn one query's local restriction into another query's silent data loss.

This is not an observability bug.

This is not a performance bug.

This is a correctness bug.

No final result-set deduplication can repair missing candidates.

No aggregate domain deduplication can repair missing candidates.

No benchmark timing should be trusted if the same snapshot can reuse the wrong filtered trie.

## 06. Code Map

Primary file: `crates/bumbledb-lmdb/src/query.rs`.

Cache lookup starts near `build_lftj_atom_plan`.

The cache key is constructed by `lftj_atom_cache_key`.

Temporary atom tries are built by `build_lftj_sorted_trie`.

Local comparison filtering is performed by `atom_local_comparisons_pass_slots`.

Durable sorted trie fast path is skipped when local comparisons exist.

The skip is correct today because durable trie images are not predicate-specific.

The cache key is incorrect because temporary tries can be predicate-specific.

Relevant current regions:

- `query.rs:5897-5906` for cached atom trie build.
- `query.rs:5963-5964` for local-comparison durable fast-path rejection.
- `query.rs:6150-6191` for temp relation scan, filter, and copy.
- `query.rs:6428-6470` for local comparison evaluation.
- `query.rs:6473-6521` for atom trie cache-key hashing.

## 07. Existing Behavior

The executor normalizes a query.

The planner chooses a variable order.

The LFTJ builder builds one atom plan per relation atom.

Each atom plan is backed by a sorted trie.

The sorted trie may be reused from the query image cache.

If no durable trie can be used, a temporary relation image is built.

That temporary relation image scans source facts.

During scan, atom-local inputs and literals can filter candidate facts.

The filtered facts are copied into column builders.

The copied columns are sorted into a trie.

That trie is then cached.

The key hashes relation, atom fields, variable order, inputs and literals in terms.

The key does not hash local comparison predicates.

The cache can therefore conflate two different filtered source sets.

## 08. Concrete Failure Case

Create relation `R(id, x)`.

Insert facts with `x = 1, 2, 3, 4, 5, 6, 7, 8, 9`.

Run query A in one read snapshot: find `id` where `R(id, x)` and `x < 5`.

Run query B in the same read snapshot: find `id` where `R(id, x)` and `x < 9`.

Both queries can share relation atom shape.

Both queries can share variable order.

Both queries can share relation image transaction ID.

If the cache key ignores the predicate bound, query B may reuse query A's trie.

The facts where `5 <= x < 9` disappear.

The output is a strict subset of the correct result set.

The bug is silent unless an exact differential test catches it.

## 09. Input-Backed Failure Case

Create prepared query Q: find `id` where `R(id, x)` and `x < $limit`.

Execute Q with `$limit = 5`.

Execute Q with `$limit = 9`.

Both executions share query shape.

Both executions may share query image snapshot.

The encoded input value changes.

The atom trie contents change.

The atom trie cache key must change.

If only input ID is hashed, the cache is incorrect.

If encoded input value is hashed, the cache can distinguish them.

## 10. Research Context

Generic Join and LFTJ require each trie to be a faithful representation of its relation restriction.

Free Join generalizes this by letting nodes iterate and lookup through relation-specific access structures.

Any cached access structure must represent a precise logical object.

The logical object can be an unfiltered relation projection.

The logical object can be a relation projection filtered by static local predicates.

The logical object can be a relation projection filtered by encoded input values.

Those logical objects are not interchangeable.

Set semantics make this more important, not less.

A missed candidate value may eliminate an entire result fact.

There is no multiplicity compensation later in the pipeline.

Correct cache identity is a prerequisite for lazy GHT and COLT work.

## 11. Definitions

An atom-local predicate is a comparison predicate where every variable operand belongs to the same relation atom being considered.

An atom-local predicate may also contain input operands.

An atom-local predicate may also contain literal operands.

A cross-atom predicate is a comparison predicate with variables from more than one relation atom.

A build-time predicate is any predicate that actually filters facts during temporary atom trie construction.

A cache-shaping predicate is any build-time predicate whose result can change trie contents.

Only cache-shaping predicates belong in the atom trie cache key.

Execution-time predicates do not belong in the atom trie cache key.

## 12. Invariants

The trie cache key must include every value that can affect cached trie contents.

The trie cache key must exclude values that cannot affect cached trie contents.

The same query, same inputs, same snapshot, and same local predicates must hit cache.

Different local predicate literals must miss cache.

Different local predicate encoded input values must miss cache.

Different comparison operators must miss cache.

Different comparison value types must miss cache.

Different local predicate variable mappings must miss cache.

Cross-atom predicates must remain execution-time predicates.

Unfiltered atom tries must retain the old cache efficiency as much as possible.

## 13. Implementation Requirements

Add a helper that discovers local predicates for an atom.

The helper must accept `NormalizedQuery`, `NormAtom`, and `EncodedInputs`.

The helper must inspect `query.predicates`.

The helper must map atom variable IDs to atom fields.

The helper must reject predicates containing a variable not present in the atom.

The helper must keep predicates containing only local variables, inputs, and literals.

The helper must be deterministic.

The helper must produce stable ordering independent of map iteration.

The helper must not allocate large structures per fact.

The helper must be usable by both cache-key construction and build-time filtering.

## 14. Cache Key Requirements

Hash a version tag for the new key format.

Hash relation ID.

Hash atom field IDs.

Hash atom field value types.

Hash atom term kinds.

Hash literal encoded values from atom terms.

Hash input encoded values from atom terms.

Hash variables in variable-order position.

Hash every cache-shaping predicate.

For each predicate, hash operator.

For each predicate, hash comparison value type.

For each operand, hash operand kind.

For variable operands, hash atom-local variable identity or field identity.

For input operands, hash input ID and encoded input value.

For literal operands, hash encoded literal value.

Do not hash pointer addresses.

Do not hash display strings if structural data is available.

Do not rely on `Debug` formatting.

## 15. Filtering Requirements

The set of predicates used by `atom_local_comparisons_pass_slots` must match the set included in the cache-shaping key.

If a predicate cannot be evaluated from atom slots, it must not filter atom trie construction.

If a predicate uses unsupported encoded comparison logic, preserve current behavior but document whether it filters or is deferred.

The build path must not evaluate cross-atom predicates.

The execution path must still evaluate cross-atom predicates when all operands are bound.

## 16. Acceptable Simpler Design

It is acceptable to remove build-time atom-local filtering entirely.

If chosen, temporary atom tries become unfiltered by comparison predicates.

The cache key no longer needs predicate fingerprints for trie contents.

Execution must apply all predicates during binding.

This may regress performance.

If chosen, add counters showing extra predicate evaluations.

If chosen, add a TODO-free issue note in docs explaining why cache correctness was prioritized over filtering.

This simpler design must still pass all correctness tests in this PRD.

## 17. Required Tests

Add a literal-bound cache poisoning regression.

Add an input-bound cache poisoning regression.

Add a prepared-query input-bound regression.

Add a same-filter cache-hit regression.

Add a different-filter cache-miss regression.

Add a cross-atom predicate exclusion regression.

Add a no-local-predicate cache-hit regression.

Add an aggregate query regression if the aggregate path uses the same atom trie cache.

Add a projection query regression because projection is the most common path.

Add a test that compares cached execution with forced cold execution if a test hook exists.

## 18. Test Data Requirements

Use small deterministic facts.

Use at least nine facts so different bounds produce distinguishable non-empty differences.

Use encoded numeric values that compare correctly by the existing encoded comparison path.

Use at least one input-backed test to prove encoded input values are hashed.

Use at least one literal-backed test to prove literal values are hashed.

Use at least one equality predicate and one range predicate if practical.

Avoid relying on output ordering unless the result set guarantees ordering.

Assert exact result facts.

Assert cache counters where deterministic.

## 19. Diagnostics Requirements

Existing `sorted_trie_cache_hits` must still increment for identical cached atom tries.

Existing `sorted_trie_cache_misses` must increment for different local filters.

If new counters are added, include filtered atom trie builds.

If new counters are added, include unfiltered atom trie builds.

If new counters are added, include local predicates hashed.

Do not add noisy per-fact diagnostics.

Do not expose predicate literal values in public logs unless already encoded and safe.

## 20. Passing Criteria

The literal-bound poisoning test fails before the fix.

The literal-bound poisoning test passes after the fix.

The input-bound poisoning test fails before the fix.

The input-bound poisoning test passes after the fix.

Identical local filters still reuse cache.

Different local filters do not reuse cache.

Cross-atom predicates are not included in atom trie content keys.

No result facts disappear because of stale filtered trie contents.

The global validation gate passes.

The query-focused validation gate passes.

## 21. Failure Modes

Hashing input ID but not encoded input value is a failure.

Hashing predicate ID but not predicate structure is a failure.

Hashing cross-atom predicates into a single-atom trie key is a failure.

Disabling all trie cache hits is a failure unless the simpler design explicitly documents and justifies it.

Relying on final output deduplication is a failure.

Changing aggregate results while fixing projection cache behavior is a failure.

Changing variable ordering to avoid the cache bug is a failure.

Adding a compatibility option for old cache behavior is a failure.

## 22. Non-Goals

Do not redesign LFTJ execution.

Do not introduce COLT.

Do not change storage layout.

Do not change result-set ordering.

Do not implement optimizer cover choice.

Do not add a new public query option.

Do not weaken cache diagnostics.

## 23. Completion Notes

When complete, update this PRD status if the project process tracks completed PRDs.

Record any new cache-key version tags in code comments.

Record any intentional performance regression if the simpler design is chosen.

Keep this PRD as a permanent regression contract until the cache architecture is replaced by PRD 13.
