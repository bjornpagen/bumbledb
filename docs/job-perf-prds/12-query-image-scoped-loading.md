# PRD 12: Query Image Scoped Loading

## Status

Proposed.

## Motivation

PRD 03 removes the per-cell allocation cliff in query-image decoding, but the image builder still loads every relation and every durable index in the schema. The first JOB query demonstrates why this is too broad:

- `job_broad_cast_keyword_company` built a whole-schema image.
- It loaded 21 relations.
- The top three relation image spans were `CharName` 320 ms, `Name` 216 ms, `PersonInfo` 129 ms.
- Those three relations alone consumed 665 ms, 73.97% of image build time.

The long-term design should not require a whole-schema image for every query. Query images should be scoped to the relations, columns, and indexes required by the selected execution strategy.

## Evidence

| Evidence | Anchor |
|---|---|
| Query-image cache key only includes schema fingerprint and tx id | `crates/bumbledb-lmdb/src/query_image.rs:13-20` |
| Query image stores `relations: Vec<RelationImage>` and name map for all loaded relations | `crates/bumbledb-lmdb/src/query_image.rs:91-103` |
| Builder always iterates every schema relation | `crates/bumbledb-lmdb/src/query_image.rs:948-979` |
| Segment relation builder loads all columns and all indexes for a relation | `crates/bumbledb-lmdb/src/query_image.rs:1026-1101` |
| Direct count kernels only need selected relation indexes | `crates/bumbledb-lmdb/src/query.rs:2669-2886` |
| Static-empty proofs need only literal/proof relations and indexes | `crates/bumbledb-lmdb/src/query.rs:1783-2110` |
| q01 report shows a zero-row static proof still paid query-image lookup/build in first query context | `docs/job-trace-analysis/03-job_q01_top_production.md` |

## Goals

- Introduce explicit query-image scopes.
- Include scope in query-image cache identity.
- Build only required relations, columns, and indexes where possible.
- Preserve correctness: missing needed data must fail closed or trigger scoped build expansion.
- Remove the assumption that `QueryImage.relations()` is a full dense vector for all schema relations.
- Keep full-schema image as one explicit scope for tests or broad fallback, not the implicit default.

## Non-Goals

- Do not store borrowed LMDB slices in cached images.
- Do not implement durable-index trie adapters here. That is PRD 13.
- Do not preserve full dense relation vector if it creates tech debt.

## New Concepts

### QueryImageScope

Add:

```rust
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryImageScope {
    relations: BTreeMap<RelationId, RelationScope>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationScope {
    columns: BTreeSet<FieldId>,
    indexes: BTreeSet<AccessId>,
    include_all_columns: bool,
    include_all_indexes: bool,
}
```

If `BTreeMap/BTreeSet` is too allocation-heavy for hot paths, use sorted `Box<[...]>` after construction. Scope construction happens once per prepared query/plan, not per row, so clarity first.

### QueryImageKey

Change from:

```rust
pub struct QueryImageKey {
    pub schema: SchemaFingerprint,
    pub tx_id: u64,
}
```

to:

```rust
pub struct QueryImageKey {
    pub schema: SchemaFingerprint,
    pub tx_id: u64,
    pub scope: QueryImageScopeKey,
}
```

Where `QueryImageScopeKey` is a fixed structural hash or sorted compact scope descriptor.

Full-schema scope must have a stable key such as `QueryImageScopeKey::Full`.

## QueryImage Relation Storage

Current `QueryImage` uses:

```rust
relations: Vec<RelationImage>,
relation_by_name: BTreeMap<String, RelationId>,
```

This assumes relation image at `relations[id]`. Scoped images may omit relation IDs. Replace with one of:

```rust
relations: BTreeMap<RelationId, RelationImage>
```

or:

```rust
relations: Vec<Option<RelationImage>>
```

Given the user's no-tech-debt instruction, prefer a map or compact relation store with explicit missing handling. `Vec<Option<_>>` keeps dense indexing but can hide missing data mistakes.

Recommended:

```rust
relations: BTreeMap<RelationId, RelationImage>
relation_by_name: BTreeMap<String, RelationId>
```

Then change callers from:

```rust
image.relations().get(atom.relation.0 as usize)
```

to:

```rust
image.relation_by_id(atom.relation)
```

This is a breaking but cleaner internal API.

## Scope Derivation

### Generic Free Join Scope

For relation atoms:

- Include every relation referenced by atoms.
- Include every field referenced by atoms.
- Include fields needed by local predicates and projection/aggregation.
- Include durable indexes needed for prefix scans or LFTJ indexed-prefix build.

Conservative first implementation can include all columns for referenced relations but only referenced relations. That alone avoids unrelated large JOB relations.

### Direct Count Scope

For `DirectCountPlan` from PRD 07:

- Include relations referenced by direct count plan.
- Include only indexes used by the direct count plan.
- Include columns only if direct count loops relation rows, e.g. `MovieLink.movie` and `MovieLink.linked_movie` in bridge count.

### Static Empty Scope

For no-input static-empty proof:

- Include literal-bearing relation indexes where available.
- Include relations and indexes used by specialized proof helpers.
- If a proof needs row scans, include required columns only.

Long-term, static-empty cached hits should bypass image entirely via PRD 09. Scoped images still matter for cold proof misses.

## Builder Changes

### QueryImageCache

Change:

```rust
get_or_build(txn, schema)
```

to:

```rust
get_or_build(txn, schema, scope)
```

or provide both:

```rust
get_or_build_full(txn, schema)
get_or_build_scoped(txn, schema, scope)
```

Do not let old no-scope path remain the default in query execution.

### QueryImageBuilder

Change:

```rust
QueryImageBuilder::new(txn, schema).build()
```

to:

```rust
QueryImageBuilder::new(txn, schema, scope).build()
```

In `build`, iterate scope relations, not every schema relation.

### RelationImageBuilder

Change builder to accept `RelationScope`.

For segment path:

- Load only scoped columns.
- Load only scoped indexes.
- Validate required descriptors exist.

For current-index fallback:

- Extract only scoped columns from primary index.
- If scoped indexes are requested but no segment exists, either build current-index bytes for the requested indexes or report unavailable. The current fallback sets `indexes: Vec::new()`, which will break direct kernels on non-segment images. Because this code is unstable, implement proper requested-index fallback rather than preserving missing index behavior.

## Missing Scope Behavior

If code requests a relation/field/index not included in scope:

- Return a clear internal error in development.
- Do not silently treat it as empty.
- Long-term, a planner can request scope expansion, but initial implementation should derive correct scope upfront.

## Acceptance Criteria

- Query execution no longer implicitly builds full-schema image.
- `QueryImageKey` includes scope.
- Query image relation access is explicit by relation ID/name and handles missing scope loudly.
- Direct count queries can build index-only scoped images.
- Static-empty cold proofs can build proof-scoped images.
- Full-schema image remains available only as explicit fallback/test path.

## Tests

### Unit Tests

- Scope key differs for different relation sets.
- Full scope and scoped image for same tx id do not collide.
- Scoped image with one relation contains only that relation.
- Missing relation access returns clear error/none as designed.
- Scoped relation loads only requested columns.
- Scoped relation loads only requested indexes.
- Direct count scope contains required indexes and columns.
- Static-empty proof scope contains proof relations and indexes.

### Integration Tests

- JOB `job_broad_movie_info_star` works with direct-count scoped image.
- JOB `job_q09_voice_us_actor` works with relation-scoped LFTJ image.
- Existing storage/query image tests updated to explicitly request full scope.

Run:

```sh
cargo test -p bumbledb-lmdb query_image
cargo test -p bumbledb-lmdb query
cargo test --workspace --all-features
```

## Benchmark Gates

Run:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_broad_cast_keyword_company \
  --query job_broad_movie_info_star \
  --query job_q09_voice_us_actor
```

Gates:

- `job_broad_cast_keyword_company` image build should not load all 21 relations unless the chosen plan truly requires all 21.
- Direct count queries should avoid large unrelated relations entirely.
- q09 correctness remains intact.
- No query should regress due to missing scoped data.

## Risks

- Many call sites assume dense `relations()[id]`. This PRD intentionally breaks that assumption.
- Planner stats may assume relation stats for all image relations. Change it to request stats only for query atoms.
- Partial images can create false static-empty/direct results if missing relation/index is interpreted as empty. Fail closed.
- Cache cardinality can grow with scopes. Add diagnostics and eventual eviction if needed.

## Definition Of Done

- Query image cache identity includes scope.
- Query images can be built for a subset of relations/columns/indexes.
- Query execution derives and requests scopes explicitly.
- Full-schema image is no longer the default query path.
- JOB scoped-image benchmarks pass and show relation-load reductions.
