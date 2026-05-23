# PRD 01: Delete Public Facade Crate

## Status

Not started.

## Objective

Remove the placeholder `bumbledb` crate from the workspace unless it becomes a real facade in this PRD. The current facade exposes opaque transaction tokens and placeholder errors instead of the real fact/query API. That is worse than no facade.

## Problem

The facade crate is public API noise. It duplicates `bumbledb-lmdb::Environment` without exposing insert, delete, scan, schema verification, or query execution. It preserves a future abstraction layer that does not currently exist.

## Required Direction

Delete the crate entirely.

Do not rewrite it into a richer facade here. A future public facade can be added later when the core API is stable.

## Implementation Steps

1. Remove `crates/bumbledb` from workspace members.
2. Delete `crates/bumbledb/Cargo.toml` and `crates/bumbledb/src/lib.rs`.
3. Remove facade-specific tests from the workspace.
4. Remove any dependency references to the facade crate.
5. Update docs that name `bumbledb` as a public facade if present.

## Passing Criteria

- `cargo metadata` no longer lists a `bumbledb` package.
- No Rust source imports the facade crate.
- The workspace builds and tests without facade tests.
- No placeholder public facade errors remain.

## Failure Modes

- Keeping empty opaque transaction wrappers is failure.
- Keeping the crate but making it re-export everything from LMDB is failure.
- Adding compatibility aliases is failure.

## Validation

Run the global validation gate.

## Completion

Delete this PRD and commit.
