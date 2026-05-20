# PRD 08: Structural Query Cache Keys

## Status

Proposed.

## Motivation

The current cache key for prepared plans and static-empty proofs is a debug-formatted string:

```rust
fn prepared_plan_cache_key(query: &NormalizedQuery) -> Option<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(format!("{query:?}").as_bytes());
    Some(hasher.finalize().to_hex().to_string())
}
```

Anchor: `crates/bumbledb-lmdb/src/query.rs:1771-1774`.

This is slow, allocates, and is not an intentional semantic contract. It also forces caches to use `String` and ordered tree structures.

Structural keys are required before we can cleanly implement:

- Early static-empty cache lookup.
- Prepared normalized query reuse.
- Compact direct/static plans.
- LFTJ atom cache cleanup.
- Query-image scoped cache identity.

## Evidence

| Finding | Anchor |
|---|---|
| Prepared/static key hashes debug string and returns hex string | `crates/bumbledb-lmdb/src/query.rs:1771-1774` |
| Static-empty cache stores `BTreeSet<String>` | `crates/bumbledb-lmdb/src/query_image.rs:93-103`, `202-215` |
| Prepared-plan cache methods accept `&str`/`String` | `crates/bumbledb-lmdb/src/query_image.rs:189-200` |
| LFTJ atom cache builds hand-written string with hex literals | `crates/bumbledb-lmdb/src/query.rs:5774-5825` |
| Direct cache keys are strings too | `crates/bumbledb-lmdb/src/query.rs:3370-3374` |
| q33 cached static-empty still loses to SQLite due to query pipeline overhead including key/cache work | `docs/job-trace-analysis/08-job_q33_linked_series_companies.md:27-68` |

## Goals

- Replace debug-format cache keys with stable structural fixed-size keys.
- Replace `String` cache keys in prepared-plan and static-empty caches.
- Replace LFTJ atom cache string keys.
- Make keys versioned and schema-aware where needed.
- Avoid heap allocation during key construction for common small queries.
- Delete old string key path rather than keeping it as a fallback.

## Non-Goals

- Do not implement prepared normalized query reuse in this PRD.
- Do not implement static-empty early lookup in this PRD.
- Do not change query semantics.
- Do not make cache keys public API.

## Proposed Key Types

Use fixed-size BLAKE3 output as the underlying key.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct QueryShapeKey([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct LftjAtomKey([u8; 32]);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DirectKernelKey([u8; 32]);
```

Using `BTreeMap` with fixed keys is acceptable initially. Later we can switch to `HashMap` if useful. The largest immediate win is eliminating debug strings and `String` allocation.

## Key Versioning

Every structural key must begin with a domain/version tag:

```text
bumbledb.query_shape.v1
bumbledb.lftj_atom.v1
bumbledb.direct_kernel.v1
```

Hash input must include only semantic fields, never debug formatting or diagnostics.

## Query Shape Key Contents

Hash these fields in deterministic order:

- Schema fingerprint or schema-local version.
- Variables in dense ID order: ID, logical type, optional name if output names are semantic for cache reuse.
- Inputs in dense ID order: ID, logical type, optional name.
- Find terms in order: projection variable IDs and aggregate function/variable/type.
- Clauses in normalized order: relation atoms and comparisons.
- For relation atoms: relation ID, field ID, term tag, variable/input ID or encoded literal bytes.
- For comparisons: predicate ID/order, operator, operand tags, variable/input IDs or encoded literal bytes, value type.
- Output plan shape.

Do not include:

- Runtime timings.
- Planner estimates.
- Cache diagnostics.
- Trace/explain strings.
- Allocation counters.

## Value Type Encoding

Use a structural value-type encoder. Do not hash `Debug` output for `ValueType`.

Recommended helper near query key code:

```rust
fn hash_value_type(hasher: &mut blake3::Hasher, ty: &ValueType) { ... }
```

Cases must cover all current `ValueType` variants from `crates/bumbledb-core/src/schema.rs`:

- `Bool`
- `U64`
- `I64`
- `Id { name }`
- `Ref { target }`
- `TimestampMicros`
- `Decimal { scale }`
- `Uuid`
- `String`
- `Bytes`
- `Enum { name }`
- `Code { name }`

If names are schema-dependent, either hash their bytes or hash schema-local domain IDs if available. Current descriptors mostly store names, so hash names for now.

## Encoded Literal Safety

`NormalizedQuery` stores encoded literals as `EncodedOwned`. For strings/bytes, encoding depends on dictionary IDs in the current database snapshot.

Important invariant:

- A `QueryShapeKey` over `NormalizedQuery` is snapshot-dependent if it includes encoded string/bytes literal IDs.
- That is acceptable for image-local prepared/static caches because `QueryImageKey` is already `{schema, tx_id}`.
- For future frontend-level prepared queries, store unencoded literal shape separately and cache encoded literals per snapshot.

This PRD should key runtime caches after normalization. PRD 10 will split reusable query shape from snapshot-local encoded literals.

## Cache Changes

### Prepared Plan Cache

Current methods in `QueryImage`:

- `cached_prepared_plan(&self, key: &str)` at `query_image.rs:189-191`.
- `insert_prepared_plan(&self, key: String, ...)` at `query_image.rs:193-200`.

Change to:

```rust
pub(crate) fn cached_prepared_plan(&self, key: QueryShapeKey) -> Result<Option<Arc<ExecutionPlan>>>;
pub(crate) fn insert_prepared_plan(&self, key: QueryShapeKey, plan: ExecutionPlan, build_micros: u64) -> Result<Arc<ExecutionPlan>>;
```

PreparedPlanCache should store:

```rust
RwLock<BTreeMap<QueryShapeKey, PreparedPlanEntry>>
```

### Static-Empty Cache

Current:

```rust
static_empty_queries: Arc<RwLock<BTreeSet<String>>>
```

Change to:

```rust
static_empty_queries: Arc<RwLock<BTreeSet<QueryShapeKey>>>
```

or merge with prepared cache later in PRD 09/11.

### LFTJ Atom Cache

Current `lftj_atom_cache_key` returns `String` at `query.rs:5774-5819`.

Change to return `LftjAtomKey`.

Hash contents:

- Relation ID.
- Variables in atom-trie variable order.
- Atom fields in field ID order as stored in `NormAtom`.
- Term tags and variable/input IDs.
- Encoded literal/input bytes for bound terms.
- Field IDs for each variable position.

### Direct Kernel Cache Keys

Current direct cache key helpers at `query.rs:3370-3374` produce strings. Replace with structural keys if they are still used after PRD 07.

## Implementation Plan

### Step 1: Add Key Types And Hash Helpers

Add to `query.rs` or a new internal `query_key.rs` module.

Prefer a new module if `query.rs` becomes unwieldy, but avoid overengineering.

Helpers:

- `hash_u8`
- `hash_u16`
- `hash_u64`
- `hash_usize_as_u64`
- `hash_bytes_len_prefixed`
- `hash_value_type`
- `hash_encoded_owned`
- `hash_norm_term`
- `hash_norm_operand`

Length-prefix variable-length bytes and strings to avoid ambiguous concatenation.

### Step 2: Replace `prepared_plan_cache_key`

Replace:

```rust
fn prepared_plan_cache_key(query: &NormalizedQuery) -> Option<String>
```

with:

```rust
fn query_shape_key(schema: &StorageSchema, query: &NormalizedQuery) -> QueryShapeKey
```

If schema fingerprint is already in `QueryImageKey`, still include it in key or keep it explicit in caller cache identity. Prefer including it to make keys robust if moved outside image-local caches later.

### Step 3: Update Query Image Caches

Update `QueryImage` methods and `PreparedPlanCache` types in `query_image.rs`.

Ensure diagnostics still expose counts/hits/misses/builds. They do not need to expose keys.

### Step 4: Update Static Empty Call Sites

In `execute_query` and `execute_query_count_only`, update:

- cache lookup at `query.rs:1456-1459`.
- cache insert at `query.rs:1489-1492`.
- duplicate count-only path at `query.rs:1637-1668`.

### Step 5: Update LFTJ Atom Cache

Update:

- `build_lftj_atom_plan` at `query.rs:5430-5446`.
- `lftj_atom_cache_key` at `query.rs:5774-5819`.
- `QueryImage::cached_sorted_trie` at `query_image.rs:218-307` to accept `LftjAtomKey`.

If `cached_sorted_trie` is also used by non-LFTJ code, either generalize to a new `TrieCacheKey` enum or introduce a sorted-trie key newtype.

## Tests

### Unit Tests

- Same normalized query produces same `QueryShapeKey` across repeated construction.
- Changing relation ID changes key.
- Changing field ID changes key.
- Changing literal changes key.
- Changing comparison operator changes key.
- Changing aggregate function changes key.
- Changing output order changes key.
- Changing input type/name changes key.
- Same LFTJ atom and inputs produce same `LftjAtomKey`.
- Changing encoded input value changes `LftjAtomKey`.
- Key construction performs no heap allocation for a small no-literal query if allocation telemetry can be tested locally; otherwise rely on code review.

### Integration Tests

Existing prepared-plan cache tests must pass or be updated to fixed-key semantics.

Run:

```sh
cargo test -p bumbledb-lmdb query
cargo test --workspace --all-features
```

## Benchmark Gates

This PRD should mostly affect small/static/direct repeated query overhead.

Run:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q33_linked_series_companies \
  --query job_q16_character_title_us \
  --query job_movie_link_bridge
```

Expected:

- No correctness changes.
- q33 and q16 allocation calls should drop modestly before PRD 09.
- Direct query plan-cache overhead should drop modestly.

Do not expect the big q33 win until PRD 09 moves the static-empty lookup earlier.

## Risks

- Missing one semantic field in a key can cause incorrect cache hits. Tests must intentionally mutate every query component.
- Including too much non-semantic data reduces cache hits but is safer than false hits. Prefer conservative inclusion.
- Encoded string literals are dictionary/snapshot dependent. Keep runtime keys image-local until PRD 10 introduces split prepared shape.
- Changing cache map key type may require broad but mechanical edits.

## Definition Of Done

- No production `format!("{query:?}")` cache key path remains.
- Prepared-plan and static-empty caches use fixed structural keys.
- LFTJ atom cache uses a fixed structural key.
- Tests cover key equality and inequality for all relevant query components.
