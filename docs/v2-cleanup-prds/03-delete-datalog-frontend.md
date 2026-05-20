# PRD 03: Delete the Datalog Frontend

## Goal

Delete the Datalog language frontend entirely.

After PRD 02, all real callers should use the typed query IR builder. This PRD removes the now-dead parser/typechecker module, parser fuzz target, public exports, and Datalog terminology in executable code.

## Current State

- `crates/bumbledb-core/src/lib.rs:6` exports `pub mod datalog;`.
- `crates/bumbledb-core/src/datalog.rs:1-1568` implements parser, AST, typechecker, errors, and tests.
- `fuzz/Cargo.toml:45-50` declares `fuzz_datalog_parser`.
- `fuzz/fuzz_targets/fuzz_datalog_parser.rs` fuzzes the parser.
- Several comments in `crates/bumbledb-lmdb/src/query.rs` and `crates/bumbledb-lmdb/src/free_join.rs` still say Datalog even though execution consumes `TypedQuery`.

## Required Changes

### Remove Module Export

Delete from `crates/bumbledb-core/src/lib.rs`:

```rust
pub mod datalog;
```

Keep:

```rust
pub mod encoding;
pub mod query_ir;
pub mod schema;
```

If PRD 01 added `query_builder.rs`, export it:

```rust
pub mod query_builder;
```

### Delete Datalog Source

Delete:

```text
crates/bumbledb-core/src/datalog.rs
```

Do not leave a stub. Do not leave compatibility aliases. Do not move parser tests elsewhere.

### Delete Parser Fuzz Target

Delete:

```text
fuzz/fuzz_targets/fuzz_datalog_parser.rs
```

Remove from `fuzz/Cargo.toml`:

```toml
[[bin]]
name = "fuzz_datalog_parser"
path = "fuzz_targets/fuzz_datalog_parser.rs"
test = false
doc = false
bench = false
```

### Remove Datalog Wording From Code

Replace code comments such as:

```rust
/// Executor-friendly normalized Datalog query.
```

with:

```rust
/// Executor-friendly normalized typed query IR.
```

Known anchors:

- `crates/bumbledb-lmdb/src/query.rs:76`
- `crates/bumbledb-lmdb/src/query.rs:1523`
- `crates/bumbledb-lmdb/src/query.rs:1764`
- `crates/bumbledb-lmdb/src/free_join.rs:133`
- `crates/bumbledb-test-support/src/workloads.rs:3`
- `crates/bumbledb-lmdb/src/benchmark.rs:11-19`

### Keep Historical Docs Intact For Now

Do not edit `docs/job-trace-analysis/*` in this PRD. Those files are historical artifacts and can keep old words until PRD 12.

## Non-Goals

- No Logica parser.
- No query text parser of any kind.
- No hidden Datalog feature flag.
- No compatibility import path like `bumbledb_core::datalog::TypedQuery`.

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- Repository grep for `bumbledb_core::datalog` returns no matches.
- Repository grep for `parse_and_typecheck` returns no matches.
- Repository grep for `fuzz_datalog_parser` returns no matches.
- Repository grep for `pub mod datalog` returns no matches.

## Completion Criteria

- `datalog.rs` is gone.
- All callers use `query_ir` and query builder APIs.
- The public core API no longer exposes a Datalog namespace.
- Fuzz manifest no longer references deleted files.
