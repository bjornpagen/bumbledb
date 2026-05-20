# PRD 12: Documentation and Final Gates

## Goal

Finalize the v2 cleanup by rewriting product docs, deleting obsolete planning artifacts if desired, and enforcing repository-wide acceptance gates.

This PRD is the end of the cleanup pass. It must leave the repository internally consistent and free of v1 design concepts.

## Documentation Updates

### Rosetta Stone

Rewrite `docs/ROSETTA_STONE.md` to describe v2 accurately.

Required updates:

- Product thesis says typed query IR now, Logica later.
- Remove “Datalog-only database”.
- State Datalog frontend was deleted intentionally.
- State no SQL, no server, LMDB-only still hold.
- State relation contents are sets of full tuples.
- State duplicate exact insert is idempotent.
- State no primary key concept exists.
- State one covering unique constraint owns canonical tuple access.
- State foreign keys target named unique constraints.
- State identities are nominal typed values.
- State `Code` type does not exist.
- State open numeric domains use `U64`.
- State closed domains use `Enum`.
- State backwards compatibility and migrations remain out of scope.

### Historical Docs

`docs/job-trace-analysis/*` may remain as historical artifacts.

Add a short note to `docs/job-trace-analysis/00-overview.md` if desired:

```text
Note: This analysis predates the v2 schema/query cleanup. It refers to Datalog and primary-key internals that no longer exist after the v2 cleanup pass.
```

### Old PRD Docs

`docs/job-perf-prds/00-roadmap.md` is historical. Either:

- keep it with a historical note, or
- move it under a historical docs directory.

Do not let old perf PRD docs be mistaken for current architecture guidance.

## Repository Rejection Sweep

Run repository searches and remove active code references to banned concepts.

Banned active code terms:

```text
datalog
Datalog
PrimaryKeyDescriptor
GeneratedIdDescriptor
RelationKind
ValueType::Id
ValueType::Ref
ValueType::Code
Value::Id
Value::Ref
Value::Code
with_ref_foreign_keys
IndexKind::Primary
IndexKind::Ref
ComponentRole::Identity
KeyValues
NS_CURRENT_ROW
NS_UNIQUE_GUARD
current_row_key
primary_bytes
encode_primary_key
```

Allowed references:

- This PRD directory.
- Historical docs explicitly marked historical.
- Changelog text explaining deletion.

No active Rust code may contain these concepts.

## Format and Check Gates

Final required commands:

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

Benchmark smoke commands:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob
```

If JOB dataset exists:

```sh
cargo run -p bumbledb-bench --release -- --preset job --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb
```

If these are too slow, run documented smoke subsets and record the skipped full command in the final implementation summary.

## Required Semantic Tests

Ensure tests exist for all of these final behaviors:

- Every relation must have exactly one covering unique constraint.
- FK targets named unique constraints.
- FK type mismatch is rejected.
- Identity type mismatch is rejected by query builder.
- Exact duplicate insert is idempotent.
- Unique prefix conflict fails.
- Delete exact existing tuple succeeds.
- Delete absent exact tuple is non-error.
- Restrict delete blocks referenced target tuple.
- Projection output is distinct.
- Count semantics on empty input match PRD 09.
- Query builder creates dense IDs.
- Query shape hashes include identity type metadata.
- Query image builds without primary access path.
- Storage decodes rows from non-covering-logical access paths because all physical paths cover the tuple.
- SQLite comparison uses set semantics.

## Required Public API Review

Review public exports in:

- `crates/bumbledb/src/lib.rs`
- `crates/bumbledb-core/src/lib.rs`
- `crates/bumbledb-lmdb/src/lib.rs`

Remove exports that expose deleted internals.

Recommended public core exports:

```rust
pub mod encoding;
pub mod query_ir;
pub mod query_builder;
pub mod schema;
```

No `datalog` module.

## Compatibility Statement

Add explicit docs saying:

```text
Bumbledb v2 is not compatible with v1 databases. Open with a mismatched storage format or schema fingerprint fails. There is no migration path except ETL into a new database.
```

If storage format changed, ensure `STORAGE_FORMAT_VERSION` at `crates/bumbledb-lmdb/src/lib.rs:64` is bumped.

If only schema fingerprint changed but storage format did not, still confirm old v1 schemas fail through fingerprint mismatch.

## Final Implementation Summary Template

The final implementing agent should report:

```text
Summary:
- Deleted Datalog frontend and parser fuzz target.
- Replaced schema with v2 explicit constraints and typed identity model.
- Replaced primary/current-row storage with covering tuple access paths.
- Enforced set insert/delete semantics.
- Migrated tests/benchmarks to typed query IR builders.
- Split/simplified query architecture and removed benchmark-specific engine branches.

Validation:
- cargo fmt --all --check: PASS
- cargo check --workspace --all-targets --all-features: PASS
- cargo clippy --workspace --all-targets --all-features -- -D warnings: PASS
- cargo test --workspace --all-features: PASS
- cargo check --manifest-path fuzz/Cargo.toml: PASS
- non-JOB benchmark smoke: PASS/SKIPPED with reason
- JOB benchmark smoke: PASS/SKIPPED with reason

Compatibility:
- v1 schema/database compatibility intentionally removed.
- Migration path is ETL into a new database.
```

## Passing Requirements

- All format/check/test/fuzz gates pass.
- Required semantic tests exist and pass.
- Rejection sweep has no active code hits.
- `docs/ROSETTA_STONE.md` accurately describes v2.
- Final summary records benchmark smoke status.

## Completion Criteria

- The codebase no longer carries v1 schema/frontend/storage concepts.
- The documentation no longer describes Datalog or primary-key semantics as current design.
- The repository is ready for future Logica frontend work on top of typed query IR.
