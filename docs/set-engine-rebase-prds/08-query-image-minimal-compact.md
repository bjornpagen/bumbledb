# PRD 08: Query Image Minimal And Compact

## 01. Status

Not started.

## 02. Severity

High performance and memory architecture.

## 03. Owner Model

This PRD is designed for one implementer.

The implementer must complete PRDs 06 and 07 first.

The implementer must add byte-count diagnostics before claiming memory reduction.

The implementer must preserve snapshot correctness.

The implementer must not make query images depend on redundant full fact access keys.

## 04. Dependency Order

PRD 06 is mandatory because query image columns must not depend on full `fact_set` access keys.

PRD 07 is mandatory because access paths and guard paths must be separated first.

PRD 09 depends on minimal images for efficient projection execution.

PRD 10 depends on minimal images for aggregate-domain execution.

PRD 13 depends on compact images for lazy GHT/COLT.

PRD 16 depends on the diagnostics added here.

## 05. Problem Statement

Query images currently load too much.

For every queried relation, the image scope includes all fields.

For every queried relation, the image scope includes all access paths.

Access image bytes copy full durable access keys.

Those bytes include repeated fixed namespace prefixes.

Those bytes include relation IDs repeated per entry.

Those bytes include access IDs repeated per entry.

Those bytes include fact identity suffixes even when not needed.

The result is unnecessary memory use and cache pressure.

The query image should be a plan-specific snapshot, not a full relation export.

## 06. Code Map

Primary files:

- `crates/bumbledb-lmdb/src/query_image.rs`.
- `crates/bumbledb-lmdb/src/query.rs`.
- `crates/bumbledb-lmdb/src/storage.rs`.
- `crates/bumbledb-lmdb/src/storage_schema.rs`.

Relevant current regions:

- `query.rs:2579-2580` for broad query image scope.
- `query_image.rs:116-173` for scope structures.
- `query_image.rs:1431-1465` for column construction.
- `query_image.rs:1468-1535` for access image construction.
- `query_image.rs:707-721` for relation access image bytes.
- `query_image.rs:1239-1319` for image cache.

## 07. Existing Behavior

`query_image_scope_for_query` selects all fields and all indexes for every relation in the query.

The image builder builds columns for all relation fields.

The image builder scans every access path for the relation.

For each access path, it copies every full access key into a contiguous byte vector.

The copied bytes include fixed durable prefixes.

The copied bytes use durable encoded length as in-memory encoded length.

The image cache stores the resulting image by schema, tx id, and scope key.

Because normal scopes are broad, cache memory can grow quickly.

## 08. Target Behavior

The image scope must be selected from query requirements and plan requirements.

Only fields needed for projection, aggregate domains, aggregate measures, predicates, and selected access paths should be included.

Only access paths needed by the chosen plan should be copied.

Access image entries should be compact in-memory entries.

Fixed durable prefixes should not be repeated per entry.

Fact identity suffix should be included only if the executor needs fact identity from that access image.

Full debug image construction may still exist as an explicit full scope.

Normal query execution must not use full relation scope by default.

## 09. Research Context

The Free Join paper emphasizes column-oriented layout and lazy trie construction.

Query images are Bumbledb's snapshot-local column store.

If query images always load all fields and all access paths, later COLT work starts from a bloated base.

Set-engine execution should know which variables and domains are needed.

That knowledge should drive image scope.

Projection queries often need only a small subset of fields.

Aggregate-domain queries often need domain fields and one measure field.

Existential atoms may need only semijoin access paths.

## 10. Desired Invariants

Query image scope is explicit.

Query image scope is deterministic.

Query image scope is no wider than required by the selected physical plan.

Attempting to access a non-scoped field is an internal error in tests.

Attempting to access a non-scoped access path is an internal error in tests.

Compact access entry length is separate from durable access key length.

Access component offsets refer to compact entry bytes.

Snapshot correctness is unchanged.

Schema fingerprint and tx id remain part of cache identity.

Scope key remains part of cache identity.

## 11. Scope Selection Plan

Add a planner phase that determines required fields.

Fields are required if projected.

Fields are required if used by aggregate group variables.

Fields are required if used by aggregate domain variables.

Fields are required if used by aggregate measure variables.

Fields are required if used by local or cross-atom predicates.

Fields are required if needed to build selected access keys.

Fields are required if needed to verify repeated-variable equality.

Fields are not required merely because they exist in a queried relation.

Add a planner phase that determines required access paths.

Access paths are required if selected by the physical plan.

Access paths are not required merely because they exist in schema.

## 12. Two-Phase Planning Option

The current planner may need relation stats before final plan selection.

If full image stats are required, split image construction.

Phase A can build lightweight relation stats.

Phase A must avoid copying all access bytes.

Phase B builds the plan-specific image.

If two-phase planning is too large, first implement conservative field pruning and access pruning based on current selected plan.

Document any temporary broad scope retained.

## 13. Compact Access Image Plan

Introduce compact encoded length on `RelationIndexImage`.

Store durable prefix once in metadata if needed.

Strip namespace byte from per-entry bytes.

Strip relation ID from per-entry bytes.

Strip access ID from per-entry bytes.

Adjust component offsets to compact entry start.

Store fact identity offset explicitly if present.

Allow access images with no fact identity if only prefix existence/count is needed.

Update `entries_with_prefix`, `prefix_range`, and `component_bytes` to use compact layout.

## 14. Column Construction Plan

After PRD 06, build relation columns from canonical fact owner or a fact iterator.

Do not depend on every field being present in `fact_set` key components.

Only build columns included in scope.

Maintain mapping from schema field ID to scoped column position.

`RelationImage::encoded_bytes` must handle scoped columns correctly.

If a non-scoped field is requested, return `None` or an internal error depending on call site.

Tests should assert the chosen behavior.

## 15. Query Execution Changes

Update all query execution paths to request required scope.

LFTJ must request fields it needs to build atom tries.

Direct paths must request access paths and fields they use.

Projection sink must request projected fields.

Aggregate sink must request group, domain, and measure fields.

Proof-like paths, if reintroduced through Free Join, must request only required fields and access paths.

Do not silently fall back to full images when a scoped field is missing.

## 16. Cache Key Requirements

Scope key must reflect selected fields.

Scope key must reflect selected access paths.

Scope key must reflect whether fact identity suffixes are included in access images.

Scope key must reflect compact layout version if needed.

Two scopes with different field sets must not collide.

Two scopes with different access sets must not collide.

Full scope and query scope must not collide.

## 17. Diagnostics Requirements

Add requested field count per relation.

Add requested access count per relation.

Add encoded column bytes copied.

Add durable access bytes scanned.

Add compact access bytes stored.

Add fixed-prefix bytes avoided.

Add fields omitted count.

Add access paths omitted count.

Expose these through existing query image diagnostics if practical.

Do not expose raw key bytes.

## 18. Required Scope Tests

Projection query needing one field builds only that field for the relation.

Predicate query needing one filter field and one projected field builds exactly those fields.

Aggregate query builds group, domain, and measure fields only.

Existential relation used only for semijoin does not build unused projected columns.

Query requiring a range access includes that access.

Query not requiring a range access omits that access.

Full debug image still includes all fields and accesses when explicitly requested.

## 19. Required Compact Layout Tests

Access image compact entry length is smaller than durable key length for non-empty access path.

Component offsets return correct encoded bytes after prefix stripping.

Prefix cardinality works on compact entries.

Prefix existence works on compact entries.

Range iteration works if range access uses compact entries.

Fact identity lookup works when fact identity is included.

Access image cache key changes when compact layout version changes.

## 20. Required Behavior Tests

All existing query tests pass with scoped images.

Golden examples pass with scoped images.

Prepared plan cache does not reuse a plan with incompatible image scope.

Query image cache distinguishes two different scopes in the same tx id.

Snapshot stability remains correct.

Reopen behavior remains correct.

## 21. Passing Criteria

Normal query execution no longer uses all-fields/all-accesses image scope by default.

Compact access entries strip fixed durable prefixes.

Column images are field-scoped.

Access images are access-scoped.

Diagnostics show bytes avoided for at least one test.

Full explicit image scope remains available for diagnostics/tests.

The global validation gate passes.

The query-focused validation gate passes.

## 22. Failure Modes

Silently building full images for every normal query is a failure.

Copying full durable access keys into compact images is a failure.

Depending on full `fact_set` key fields is a failure after PRD 06.

Returning empty values for missing scoped fields is a failure.

Cache key collision between scopes is a failure.

Breaking snapshot stability is a failure.

## 23. Non-Goals

Do not implement COLT.

Do not implement vectorized execution.

Do not change aggregate semantics.

Do not change public query result shape.

Do not add cache eviction here unless trivial.

Do not add storage compression.

## 24. Completion Notes

Update query image diagnostics docs.

Update tests that assumed all columns are always present.

Document how to request full scope for debug tools.

This PRD prepares the data layout required for lazy GHT/COLT.
