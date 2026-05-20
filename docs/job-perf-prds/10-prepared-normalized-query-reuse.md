# PRD 10: Prepared Normalized Query Reuse

## Status

Proposed.

## Motivation

The runtime currently normalizes every `TypedQuery` on every execution. Normalization clones variable names, input names, relation names, field names, value types, atom fields, predicates, find terms, and output plan structures. It also re-encodes literals each time.

For large join queries this overhead is not the top cost, but for static-empty and direct kernels it dominates the remaining floor.

Examples from the trace:

- q33 sample normalize averages 5.858 us and the residual cached static-empty work is large enough that SQLite wins.
- q16 first execution allocation calls are 61.5% normalization.
- q01 first execution normalization and cache-key work are part of the 82-83 us cached static-empty floor.

This PRD introduces a prepared normalized query object that can be reused across repeated executions.

## Evidence

| Current behavior | Anchor |
|---|---|
| `execute_query` calls `normalize_query` every execution | `crates/bumbledb-lmdb/src/query.rs:1406-1418` |
| `execute_query_count_only` duplicates normalization | `crates/bumbledb-lmdb/src/query.rs:1616-1620` |
| `normalize_query` clones vars, inputs, find terms, atoms, predicates | `crates/bumbledb-lmdb/src/query.rs:7236-7297` |
| `normalize_atom` clones field names, relation names, value types | `crates/bumbledb-lmdb/src/query.rs:7300-7318` |
| `normalize_term` encodes literals through storage/dictionary | `crates/bumbledb-lmdb/src/query.rs:7321-7367` |
| `encode_inputs` allocates a fresh vector every execution | `crates/bumbledb-lmdb/src/query.rs:7399-7423` |
| `TypedQuery` is currently an owned IR without fingerprint/prepared state | `crates/bumbledb-core/src/query_ir.rs:59-70` |
| Datalog typechecker already creates dense IDs and typed relation/field IDs | `crates/bumbledb-core/src/datalog.rs:907-959` |

## Goals

- Add an explicit prepared query representation for repeated execution.
- Reuse normalized structure across materialized and count-only paths.
- Precompute structural query keys from PRD 08.
- Avoid per-execution cloning of names/types/atoms where possible.
- Encode static literals once per snapshot, not on every execution.
- Represent no-input queries with shared empty encoded input state.

## Non-Goals

- Do not replace the Datalog parser/frontend in this PRD.
- Do not implement Logica frontend here.
- Do not add public API stability guarantees.
- Do not preserve old internal normalized-query ownership shapes if a cleaner shape is possible.

## Current Normalized Query Shape

`NormalizedQuery` currently stores owned metadata:

```rust
pub struct NormalizedQuery {
    vars: Vec<NormVar>,
    inputs: Vec<NormInput>,
    atoms: Vec<NormAtom>,
    predicates: Vec<NormPredicate>,
    output: OutputPlan,
    find: Vec<NormFindTerm>,
}
```

Anchors: `crates/bumbledb-lmdb/src/query.rs:74-87` and related structs around `90-134`.

This shape is convenient but expensive when rebuilt per call.

## Proposed Architecture

Introduce:

```rust
pub struct PreparedQuery {
    shape_key: QueryShapeKey,
    shape: Arc<PreparedQueryShape>,
}

pub struct PreparedQueryShape {
    vars: Box<[PreparedVar]>,
    inputs: Box<[PreparedInput]>,
    atoms: Box<[PreparedAtom]>,
    predicates: Box<[PreparedPredicate]>,
    find: Box<[NormFindTerm]>,
    output: OutputPlan,
    result_columns: Arc<[ResultColumn]>,
    flags: PreparedQueryFlags,
}

pub struct SnapshotPreparedQuery {
    shape: Arc<PreparedQueryShape>,
    encoded_literals: Box<[EncodedOwned]>,
    snapshot_key: QueryImageKey,
}
```

Exact names can vary. The key design is split:

- Shape-level data is independent of dictionary IDs and storage snapshot.
- Encoded literals for strings/bytes are snapshot-dependent.
- Runtime inputs remain execution-dependent.

## Where To Build It

There are two options:

### Option A: Runtime-only first

`execute_query` takes `&TypedQuery`, builds or caches `PreparedQuery` internally in the environment.

Pros:

- Smaller public API change.
- Existing benchmark code changes less.

Cons:

- Harder to avoid hash/cache lookup per call.

### Option B: Explicit prepared API

Expose an internal or public method:

```rust
Environment::prepare_query(schema: &StorageSchema, query: &TypedQuery) -> PreparedQuery
ReadTxn::execute_prepared_query(schema: &StorageSchema, query: &PreparedQuery, inputs: &InputBindings)
```

Pros:

- Benchmark can prepare once before loops.
- Future Logica frontend has a clean lowering target.
- Avoids repeated shape construction entirely.

Cons:

- Bigger API break.

Given the user's instruction to avoid tech debt and embrace breaking changes, choose Option B. Keep old `execute_query` temporarily as a wrapper only if needed for tests, then migrate call sites and delete or de-emphasize it.

## Shape Representation Requirements

### IDs First, Names For Diagnostics Only

Prepared runtime structures should use dense IDs:

- relation ID,
- field ID,
- variable ID,
- input ID,
- predicate ID,
- atom ID.

Names should be stored only where output/explain needs them. Prefer `Arc<str>` or `Box<str>` if names remain stored.

### Literals

Prepared shape stores typed literal values or typed literal descriptors, not dictionary-encoded IDs.

At snapshot execution:

- encode numeric/bool/uuid/decimal literals without heap where possible,
- dictionary string/bytes literals use current snapshot lookup,
- cache encoded literals under `{shape_key, tx_id}`.

This avoids stale dictionary IDs across snapshots.

### Inputs

No-input queries should use a shared empty encoded input container:

```rust
static EMPTY_ENCODED_INPUTS: EncodedInputs = ...
```

If static initialization is awkward because `EncodedInputs` owns `Vec`, use `EncodedInputs { values: Vec::new() }` only once inside prepared query execution and borrow it.

Long-term, replace `EncodedInputs { values: Vec<EncodedOwned> }` with `Box<[EncodedOwned]>` or `SmallVec`.

## Implementation Plan

### Step 1: Introduce Prepared Shape Builder

Refactor `normalize_query` into two phases:

```text
TypedQuery -> PreparedQueryShape
PreparedQueryShape + ReadTxn snapshot -> SnapshotPreparedQuery
```

Current `normalize_query` does both shape normalization and literal encoding. Split them.

### Step 2: Add Prepared Execution Methods

Add methods near `ReadTxn::execute_query`:

```rust
pub fn execute_prepared_query(
    &self,
    schema: &StorageSchema,
    query: &PreparedQuery,
    inputs: &InputBindings,
) -> Result<QueryOutput>;

pub fn execute_prepared_query_count_only(...);
```

Then make old `execute_query` build a temporary prepared query and call the prepared method, or migrate all internal call sites.

### Step 3: Update Benchmark Harness

In `crates/bumbledb-bench/src/main.rs:807-810`, currently:

```rust
let typed = parse_and_typecheck(...)?;
let inputs = InputBindings::from_values(...);
```

Add:

```rust
let prepared = bumble_env.prepare_query(&bumble_schema, &typed)?;
```

Then use `execute_prepared_query` for correctness, warmups, and samples.

This makes benchmark loops measure execution, not repeated runtime normalization.

### Step 4: Update Static Empty And Direct Count PRDs

PRD 09 should use prepared shape key for early no-input static-empty cache.

PRD 07 should use prepared shape flags to detect direct count shapes without re-inspecting owned strings.

Prepared flags can include:

- `has_inputs`
- `has_predicates`
- `has_literals`
- `is_global_count`
- `maybe_factorized_count`
- `maybe_movie_link_bridge_count`
- `maybe_static_empty_candidate`

Do not rely only on flags for correctness. Execution planners must still validate exact relation/index conditions.

### Step 5: Delete Redundant Runtime Normalization

After call sites are migrated:

- Remove repeated `normalize_query` from hot execution path.
- Keep a test helper or wrapper only if needed.
- Ensure `execute_query_count_only` does not duplicate normalization logic.

## Acceptance Criteria

- Benchmark loops prepare the typed query once and execute prepared query repeatedly.
- Repeated execution does not clone relation names, field names, variables, and value types on every call.
- Static-empty and direct-count paths can access a stable `QueryShapeKey` before heavy runtime work.
- `execute_query` wrapper, if retained, is clearly not the optimized path.
- Existing correctness tests pass.

## Tests

### Unit Tests

- Prepared query shape matches old normalized query for simple projection.
- Prepared query shape matches old normalized query for aggregate count.
- Prepared query shape includes literals and predicates correctly.
- Encoding literals per snapshot preserves missing-dictionary behavior.
- Prepared no-input query uses empty inputs without allocation-heavy work.
- Repeated prepared execution reuses same shape key.

### Integration Tests

- Existing Datalog/SQLite comparison tests use prepared execution or wrapper.
- JOB smoke queries still match SQLite row counts.

Commands:

```sh
cargo test -p bumbledb-core
cargo test -p bumbledb-lmdb query
cargo test -p bumbledb-test-support sqlite_comparison
cargo test --workspace --all-features
```

## Benchmark Gates

Run q33, q16, q01 after PRD 09/10 combination:

```sh
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query job_q01_top_production \
  --query job_q16_character_title_us \
  --query job_q33_linked_series_companies
```

Expected:

- Normalization spans disappear or become prepare-only for benchmark samples.
- q33 beats SQLite.
- q01/q16 do not regress.

## Risks

- Caching encoded string literals across snapshots can be wrong. Keep encoded literals snapshot-scoped.
- Prepared shape must include schema fingerprint or be tied to a schema object. A typed query from one schema must not execute against another compatible-looking schema.
- Tests that inspect exact explain metadata may need updates because names move out of hot structures.
- API churn is intentional; do not keep duplicate optimized and unoptimized public paths long-term.

## Definition Of Done

- There is an explicit prepared query shape.
- Benchmark uses prepared execution for repeated samples.
- Runtime execution no longer normalizes every call on the hot path.
- Literal encoding is snapshot-safe.
- Static-empty and direct-count PRDs can rely on stable structural keys and prepared flags.
