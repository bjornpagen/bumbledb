# PRD 09: Module Boundaries And Public API

## Status

Draft. This PRD should be implemented after the core schema/query/storage concepts are stable enough to avoid churn.

## Problem

Several files are too large and mix conceptual layers. The internal LMDB crate publicly re-exports many implementation details. The public facade crate is minimal and partly stale. Datalog naming leaks into runtime code.

## Goals

- Split large modules along real architecture boundaries.
- Keep public API narrow and intentional.
- Keep LMDB internals private unless needed by benchmarks/tests.
- Remove stale Datalog language assumptions from public comments and runtime modules.
- Make future Logica frontend replacement mechanical.

## Non-Goals

- No public stable API promise.
- No backward compatibility.
- No macro/schema DSL implementation.
- No Logica parser implementation.

## Current Code References

- `crates/bumbledb-core/src/schema.rs` combines descriptors, layout generation, canonical fingerprinting, and tests.
- `crates/bumbledb-core/src/datalog.rs` combines parser, AST, typechecker, typed IR, and tests.
- `crates/bumbledb-lmdb/src/storage.rs` combines schema wrapper, rows, values, writes, reads, segments, encoding, dictionary, and tests.
- `crates/bumbledb-lmdb/src/query.rs` combines IR, planning, direct kernels, LFTJ, hash probe, aggregation, output sinks, and tests.
- `crates/bumbledb-lmdb/src/lib.rs` re-exports many internals.
- `crates/bumbledb/src/lib.rs` is a minimal facade with stale comments.

## Required Module Shape

Target core modules:

```text
bumbledb-core/src/schema/mod.rs
bumbledb-core/src/schema/types.rs
bumbledb-core/src/schema/constraints.rs
bumbledb-core/src/schema/indexes.rs
bumbledb-core/src/schema/validation.rs
bumbledb-core/src/query_ir.rs
bumbledb-core/src/datalog.rs       // temporary frontend only
bumbledb-core/src/encoding.rs
```

Target LMDB modules:

```text
bumbledb-lmdb/src/storage/schema.rs
bumbledb-lmdb/src/storage/value.rs
bumbledb-lmdb/src/storage/keys.rs
bumbledb-lmdb/src/storage/write.rs
bumbledb-lmdb/src/storage/read.rs
bumbledb-lmdb/src/storage/constraints.rs
bumbledb-lmdb/src/storage/segments.rs
bumbledb-lmdb/src/query/normalize.rs
bumbledb-lmdb/src/query/direct.rs
bumbledb-lmdb/src/query/planner.rs
bumbledb-lmdb/src/query/hash_probe.rs
bumbledb-lmdb/src/query/lftj.rs
bumbledb-lmdb/src/query/output.rs
bumbledb-lmdb/src/query/explain.rs
```

The exact file names can vary, but the boundaries must exist.

## Public API Direction

The top-level `bumbledb` crate should eventually expose:

- `Database`
- `Schema`/schema descriptors or generated schema handles
- `ReadTxn`
- `WriteTxn`
- typed row/query APIs
- clean error types

The top-level crate should not expose:

- LMDB raw concepts
- query image internals
- trie internals
- planner internals
- Datalog parser internals

The `bumbledb-lmdb` crate may remain public within the workspace but should reduce accidental API surface.

## Implementation Plan

1. Complete PRDs 01 to 08 enough that names are stable.
2. Move query IR to core first.
3. Split schema modules.
4. Split storage modules.
5. Split query modules.
6. Narrow re-exports in `bumbledb-lmdb/src/lib.rs`.
7. Update benchmark/test imports.
8. Update comments in public facade.
9. Add module-level documentation describing boundaries.

## Strict Passing Criteria

- No production LMDB module imports `bumbledb_core::datalog`.
- `query.rs` is split into bounded modules with direct/planner/LFTJ/hash/output separated.
- `storage.rs` is split into bounded modules with write/read/constraints/segments separated.
- `schema.rs` is split or internally organized so validation/types/constraints/index layout are distinct.
- Public facade comments no longer claim Datalog is future/stage-only in stale ways.
- Re-export surface is intentionally reviewed and documented.
- All tests and benchmarks compile after import updates.

## Verification Commands

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
