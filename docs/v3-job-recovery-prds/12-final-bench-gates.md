# PRD 12: Final Bench And Gates

## Goal

Run final validation for the full v3 pass, prove q09/q24 recovery, and verify that the type/FK simplification did not break non-JOB correctness.

This is the final PRD in this suite.

## Explicit Non-Goals

- No backwards compatibility validation.
- No old database migration validation.
- No v1/v2 benchmark behavior preservation.
- No accepting q09/q24 regressions as known failures.
- No relaxing final grep rejection gates.

## Required Format And Test Gates

Run:

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

All must pass.

## Required Rejection Gates

Active Rust code must not contain:

```text
ValueType::Uuid
Value::Uuid
UuidBytes
encode_uuid
decode_uuid
IdentityAllocation
IdentityValue
ValueType::Identity
Value::Identity
Application
allocation:
ValueType::Code
Value::Code
PrimaryKeyDescriptor
GeneratedIdDescriptor
RelationKind
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
datalog
Datalog
```

Allowed references only:

- historical docs clearly marked historical
- this PRD directory before deletion
- generated target files outside source control

## Required Benchmark Runs

### Non-JOB

Run:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-final-nonjob.json
```

Expected:

- correctness passes
- all gates pass or any remaining perf warnings are documented
- Bumbledb should remain close to current non-JOB story

### JOB 10k

Run:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-final-job-10k.json
```

Expected:

- Bumbledb wins at least `8/8` or every loss has a concrete RCA.
- q09 Bumbledb avg under `3000us`.
- q24 Bumbledb avg under `1000us`.
- q09 beats SQLite.
- q24 beats SQLite.
- no row mismatch.
- all benchmark gates pass with `--fail-gates` if the harness supports it.

### Practical JOB Default

Run if feasible:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-final-job-default.json
```

If too slow, explicitly report it as skipped with reason. Do not fake completion.

## Required Summary

The final implementation summary must include:

```text
Summary:
- Recovered q09/q24 through generic structural optimizer rules.
- Deleted UUID primitive.
- Encoded enums as bytes.
- Collapsed nominal identity to serial.
- Added native compound FK support over exact unique-key tuples.
- Updated reference/SQLite/test support.

Validation:
- fmt: PASS
- check: PASS
- clippy: PASS
- tests: PASS
- fuzz check: PASS
- non-JOB benchmark: PASS with artifact path
- JOB 10k benchmark: PASS with artifact path
- practical JOB default: PASS or SKIPPED with reason

Compatibility:
- No backwards compatibility.
- No migrations.
- Existing databases require ETL into a new database.
```

## Completion Criteria

- All PRDs in this directory except `00-roadmap.md` have been implemented and deleted.
- This final PRD is deleted after passing.
- If the user asks for the same loop as last time, remove `00-roadmap.md` and the empty directory after all PRDs complete.
