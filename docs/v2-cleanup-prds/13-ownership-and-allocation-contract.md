# PRD 13: Ownership and Allocation Contract

## Goal

Establish one coherent ownership and allocation story for Bumbledb v2.

This PRD is cross-cutting. It should be applied during schema, storage, query, benchmark, and test migrations. The point is not micro-optimization; the point is making ownership obvious and making hot-path allocation intentional.

## Current Problems

The current codebase has several useful optimizations, but the ownership model is inconsistent:

- Public `Row` stores `BTreeMap<String, Value>` at `crates/bumbledb-lmdb/src/storage.rs:35-71`.
- Encoded storage row uses `BTreeMap<String, Vec<u8>>` at `storage.rs:301-341`.
- Query execution wraps `EncodedOwned` in `EncodedValue` at `crates/bumbledb-lmdb/src/query.rs:1153-1178`.
- Query images own encoded columns and durable index chunks at `crates/bumbledb-lmdb/src/query_image.rs:210-275` and later builders.
- Hash trie owns `HashMap<EncodedOwned, HashNode>` at `crates/bumbledb-lmdb/src/hash_trie.rs:83-92`.
- Sorted trie owns row order and level keys at `crates/bumbledb-lmdb/src/sorted_trie.rs:66-81`, `138-149`.
- Bench optimizations introduced several fast paths with local clone/allocation choices instead of one shared policy.

## Ownership Layers

### Public Boundary

Public API owns user data.

Allowed:

```rust
pub struct Row {
    relation: String,
    values: BTreeMap<String, Value>,
}
```

Rationale: public ergonomics matter at the boundary. Users can build rows by field name. This is not a hot internal representation.

Rule: convert public `Row` into compiled field-order representation exactly once at the storage/query boundary.

### Compiled Schema

`StorageSchema` owns compiled metadata derived from `SchemaDescriptor`.

It should own:

- relation name to `RelationId`
- field name to `FieldId`
- index/access name to `AccessId`
- field offsets and widths
- access path prefixes
- constraint compilation
- FK target access ids
- unique access ids

Target shape:

```rust
pub struct StorageSchema {
    descriptor: SchemaDescriptor,
    relations: Vec<CompiledRelationLayout>,
    relation_by_name: BTreeMap<String, RelationId>,
    layouts: Vec<CurrentIndexLayout>,
}
```

Do not repeatedly scan `descriptor.relations` in hot storage/query paths when a compiled map can answer directly.

### Storage Hot Path

Storage hot path owns one encoded tuple per incoming row.

Target:

```rust
pub(crate) struct EncodedTuple {
    relation: RelationId,
    bytes: Vec<u8>,
}
```

Allowed allocations per insert:

- one `Vec<u8>` for encoded tuple
- one key buffer reused or built per access path
- dictionary writes for new string/bytes values

Disallowed hot representations after PRD 07:

- `BTreeMap<String, Vec<u8>>` for encoded rows
- repeated `payload(relation)?` allocations
- current-row payload duplication
- primary-key byte extraction for every secondary index

### LMDB Key Ownership

LMDB owns persisted key/value bytes after put.

During writes, key construction may allocate `Vec<u8>`. Prefer reusable buffers if simple, but do not obscure correctness.

All current access path keys are full tuple encodings, so there is no row payload value. LMDB values for access path keys should remain empty unless a future feature proves otherwise.

### Dictionary Ownership

Dictionary DB owns raw interned string/bytes values.

Hot tuple/index keys store intern IDs only.

No dictionary GC in this pass.

### Query Image Ownership

`QueryImage` is immutable, snapshot-local, and shared by `Arc`.

It owns:

- encoded fixed-width column chunks
- durable index chunks loaded from segments
- relation image metadata
- planner stats cache
- prepared plan cache
- sorted/hash trie caches

Rule: query execution may borrow from `QueryImage` freely but must not mutate relation images.

### Query Execution Ownership

Hot execution should prefer borrowed encoded refs.

Acceptable owned values:

- `EncodedOwned` for sink keys
- `EncodedOwned` for hash trie keys
- `EncodedOwned` for sorted trie level keys
- small fixed arrays or `SmallVec` for bindings/prefixes

Questionable values requiring audit:

- cloned relation/field names in hot loops
- cloned `NormAtom` or `NormPredicate` per execution
- string cache keys for data structures
- `Vec<RowId>` materialization when a `RowSetRef` stream would do

## Encoded Value Policy

There must be one central fixed-width encoded value abstraction.

Current public-ish owned type:

```rust
pub enum EncodedOwned {
    One([u8; 1]),
    Eight([u8; 8]),
    Sixteen([u8; 16]),
}
```

Current borrowed type:

```rust
pub enum EncodedRef<'a> {
    One(&'a [u8; 1]),
    Eight(&'a [u8; 8]),
    Sixteen(&'a [u8; 16]),
}
```

Final rule:

- Use `EncodedRef<'_>` when reading from images, tuple bytes, or LMDB key slices.
- Convert to `EncodedOwned` only when storing beyond the borrowed scope.
- Do not wrap `EncodedOwned` in another owned wrapper unless it adds real lifetime semantics.

If `EncodedValue` survives, it must be:

```rust
pub(crate) enum EncodedValue<'a> {
    Borrowed(EncodedRef<'a>),
    Owned(EncodedOwned),
}
```

Otherwise delete it and use `EncodedOwned` directly.

## Clone Policy

Clones are allowed when they cross ownership boundaries.

Allowed:

- clone `TypedQuery` into `PreparedQuery` once
- clone schema descriptors during test setup
- clone `EncodedOwned` into sink keys
- clone output `Value` into result rows

Avoid:

- cloning relation names in loops
- cloning field names in loops
- cloning normalized query per execution after prepare
- cloning full rows inside storage write path
- cloning index bytes into temporary structures when a slice is sufficient

## Allocation Telemetry Policy

After PRDs 07 through 10, run allocation-enabled benchmark smoke when feasible.

Compare against the final PRD artifacts if available:

- JOB final: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/final-prd-job.json`
- non-JOB final: `/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/final-prd-nonjob.json`

The cleanup pass may trade a small runtime regression for architectural simplicity, but it must not reintroduce obvious allocation cliffs such as per-row `Vec<Vec<u8>>` in LFTJ builds or per-step heap churn in trie traversal.

## Required Audits

Before final gates, audit these code areas:

- `crates/bumbledb-lmdb/src/storage.rs`: no encoded row `BTreeMap`; no row payload duplication.
- `crates/bumbledb-lmdb/src/storage_schema.rs`: compiled maps exist for relation/layout lookup.
- `crates/bumbledb-lmdb/src/query.rs` or split modules: no stringly cache keys for hot indexes.
- `crates/bumbledb-lmdb/src/query_image.rs`: caches use typed keys, not raw strings.
- `crates/bumbledb-lmdb/src/hash_trie.rs`: row iteration can stream through `RowSetRef` where possible.
- `crates/bumbledb-lmdb/src/sorted_trie.rs`: iterator stack remains stack-backed via `SmallVec`.
- `crates/bumbledb-test-support`: reference model mirrors set semantics, not bag semantics.

## Tests and Checks

Required tests:

- Insert path does not allocate encoded row maps. This may be asserted indirectly by deleting the type.
- Query projection decodes only at sink finish.
- Query count-only path does not materialize output values.
- Query image can serve scoped relation columns without full-schema load.
- Hash trie prefix row iteration does not require `rows_owned` in common executor paths.

Required searches:

```text
BTreeMap<String, Vec<u8>>
rows_owned(
hash_trie_cache: Arc<RwLock<BTreeMap<String
payload(relation)
```

Each remaining hit must be justified or removed.

## Completion Criteria

- Public boundary ownership is ergonomic and explicit.
- Internal storage ownership is field-id/offset based.
- Query image ownership is immutable and shared.
- Query execution ownership is borrowed-first, owned-only-when-needed.
- Hot-path allocation decisions are documented in code structure, not scattered comments.
