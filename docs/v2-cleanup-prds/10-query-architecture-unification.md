# PRD 10: Query Architecture Unification

## Goal

Turn benchmark-driven fast paths into a smaller set of reusable architectural concepts.

This PRD should preserve performance wins while reducing branchiness in `query.rs` and related modules.

## Current State

`crates/bumbledb-lmdb/src/query.rs` has accumulated many side paths:

- Static-empty proofs and caches around query execution.
- Direct storage project before query-image creation.
- Direct count kernels.
- Direct chain kernels.
- LFTJ sorted trie path.
- Hash probe path.
- Mixed hash/LFTJ path.
- Hard-coded JOB/IMDb count factorization.
- Tiny projection sink.
- Global count sink.
- Count-only sink.
- Aggregate sink.

These were useful benchmark optimizations, but many are expressed as branches outside the normal plan model.

## Architectural Target

The query engine should have these concepts:

```text
TypedQuery
  -> NormalizedQuery
  -> LogicalShape
  -> PlanCandidate[]
  -> ExecutionPlan
  -> Executor nodes
  -> Sink policy
```

Fast paths should become plan candidates or executor nodes, not pre-planning special cases.

## Required Module Split

Split `crates/bumbledb-lmdb/src/query.rs` into smaller modules. Exact names can vary, but target structure:

```text
crates/bumbledb-lmdb/src/query/
  mod.rs
  normalize.rs
  value.rs
  cache_key.rs
  planner.rs
  access.rs
  execute.rs
  sinks.rs
  static_empty.rs
  factorized_count.rs
  explain.rs
```

If a directory module is too disruptive, first split into sibling files:

```text
query.rs
query_normalize.rs
query_planner.rs
query_access.rs
query_sinks.rs
```

Final state should not be a 10k-line `query.rs`.

## Required Access Abstraction

Introduce a generic access abstraction over storage/query-image/trie/hash sources.

Recommended shape:

```rust
pub(crate) enum AccessSource<'a> {
    Storage(StorageAccess<'a>),
    RelationImage(ImageAccess<'a>),
    SortedTrie(&'a SortedTrieIndex),
    HashTrie(&'a HashTrieIndex),
}

pub(crate) trait AccessProbe {
    fn exists(&self, prefix: &[EncodedRef<'_>]) -> Result<bool>;
    fn count(&self, prefix: &[EncodedRef<'_>]) -> Result<usize>;
    fn rows<'a>(&'a self, prefix: &[EncodedRef<'_>]) -> Result<RowSetRef<'a>>;
}
```

Do not force all sources into one trait if lifetimes become too complex. The required architectural point is to centralize prefix encoding/probing/scanning so direct/hash/LFTJ paths do not duplicate atom logic.

## Required Plan Nodes

Replace side-branch concepts with plan-level nodes or candidates.

Recommended `NodeImpl` cleanup in `free_join.rs`:

```rust
pub enum NodeImpl {
    AccessScan,
    PrefixProbe,
    SortedLeapfrog,
    HashProbe,
    FactorizedCount,
    Empty,
}
```

The exact names may differ. The current variants are at `crates/bumbledb-lmdb/src/free_join.rs:66-83`:

```rust
SortedLeapfrog
HashProbe
Hybrid
VectorLoop
ExistenceCheck
Product
AggregateSink
```

Delete variants that are not actually implemented as plan nodes. Do not keep scaffolding for future compiled plans unless used.

## Static Empty

Current static-empty logic should become an `Empty` plan candidate with proof diagnostics.

Rules:

- Generic relation/literal contradiction proof belongs here.
- Hard-coded JOB relation-name proofs must be removed or generalized.
- Static-empty cache keys remain structural query-shape keys.

Acceptance:

- No static-empty function checks hard-coded relation names like `CompanyType`, `InfoType`, `MovieCompanies`, `MovieInfoIdx`, `Keyword`, or `MovieLink`.

## Factorized Count

Current benchmark count special cases should become generic factorized-count rules.

Target examples:

- Star count: central variable with independent existence/count side relations.
- Bridge count: two endpoint variables with independent side relation counts.
- Prefix count: bound prefix against a full-covering access path.

Generic rule shape:

```rust
pub(crate) struct FactorizedCountPlan {
    pub drivers: Vec<AccessPrefix>,
    pub multipliers: Vec<AccessPrefixCount>,
    pub guards: Vec<AccessPrefixExists>,
}
```

Do not preserve relation-name-specific JOB code once generic factorization exists.

## Sinks

Current sinks live at `crates/bumbledb-lmdb/src/query.rs:8730-9500`.

Required cleanup:

- Projection is always set semantics. Delete `ProjectPlan.set_semantics` from `free_join.rs:128-135`.
- Keep tiny projection optimization, but hide it behind a `ProjectSink` implementation detail.
- Keep global count optimization, but make empty count semantics explicit.
- Keep encoded min/max optimization if tests cover it.
- Remove `CountOnlySink` if the same behavior can be expressed as a sink mode.

Recommended sink policy:

```rust
pub(crate) enum SinkMode {
    Materialize,
    CountRowsOnly,
}

pub(crate) enum OutputSink {
    Project(ProjectSink),
    Aggregate(AggregateSink),
}
```

`CountRowsOnly` should be a mode of the same sink semantics, not a separate semantic implementation.

## Encoded Value Ownership

Current private wrapper:

```rust
struct EncodedValue {
    encoded: EncodedOwned,
}
```

It lives at `query.rs:1153-1178`. It is now mostly a wrapper around `EncodedOwned`.

Required cleanup:

- Either remove `EncodedValue` and use `EncodedOwned` directly.
- Or make `EncodedValue` meaningful by supporting borrowed/owned forms.

Preferred final shape:

```rust
pub(crate) enum EncodedValue<'a> {
    Borrowed(EncodedRef<'a>),
    Owned(EncodedOwned),
}
```

Only do this if lifetime complexity is manageable. Otherwise, remove the wrapper.

## Cache Keys

Replace any stringly cache key for hash/direct indexes with typed structs.

Current query image hash trie cache uses `BTreeMap<String, Arc<HashTrieIndex>>` at `query_image.rs:221`.

Target:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HashTrieKey([u8; 32]);
```

Key inputs must include:

- schema fingerprint
- tx id or image key
- relation id
- access id or field order
- leaf mode
- scope relevant to loaded fields/indexes

## Tests Required

- Direct storage/project results equal planned query-image results.
- Count-only output count equals materialized output row count for projection queries.
- Global count empty behavior matches PRD 09 decision.
- Static empty generic proof returns same result as full execution.
- No static-empty proof relies on JOB-specific relation names.
- Factorized count generic rule matches materialized execution on JOB and non-JOB shapes.
- LFTJ durable-index path remains covered.
- Hash and mixed executors produce identical output for representative joins.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- Bench smoke for non-JOB preset.
- Bench smoke for practical JOB preset if dataset is available.
- Grep for hard-coded JOB relation names inside generic query planning/execution code returns no matches, except benchmark definitions.

## Completion Criteria

- `query.rs` is split or substantially reduced.
- Fast paths are represented as plan candidates/nodes/sink modes.
- Projection set semantics have a single implementation.
- Aggregate count semantics are explicit and tested.
- No benchmark-specific relation-name special cases remain in the engine.
