# PRD 10: Encoded Tuples And GHT API

## Purpose

Introduce tuple-key primitives and the paper's GHT interface. This is the execution abstraction used by COLT and the formal Free Join executor.

## Dependencies

- PRD 03.
- PRD 09.

## Scope

- Encoded tuple representation.
- Tuple schema representation.
- GHT trait/interface.
- Unit tests for tuple hashing, equality, ordering if needed, and width handling.
- Integration with base-image columns derived from LMDB snapshots, not direct durable GHT storage.

## Required Types

- `TupleSchema`: ordered variables or field IDs plus value types.
- `EncodedTuple`: owned tuple key made of fixed-width encoded values.
- `EncodedTupleRef`: borrowed tuple key view if useful.
- `GhtSource` or `GhtNode` interface with relation/atom occurrence metadata, `vars`, `iter`, `get`, and key-count estimate.
- `GhtIterator` yielding tuple values, not only scalar field values.

## Required Behavior

- Tuple keys support one or more values.
- Tuple keys support fixed encoded widths 1, 8, and 16.
- Strings and bytes compare/probe by intern ID, not raw variable-width bytes.
- Tuple construction from a base image offset is deterministic.
- Tuple construction assumes base image offsets are dense positions in immutable vectors loaded from `L/C` LMDB namespaces under one read transaction.
- Tuple construction from current bindings is deterministic.
- `get(tuple)` can return no child without error.
- Key-count estimates can distinguish exact map count from offset-vector estimate later used by COLT.

## Technical Direction

- Reuse existing fixed-width encoding and `EncodedOwned` where possible, but do not force tuple keys through scalar-only APIs.
- Avoid variable-length hot tuple keys. Strings/bytes are 8-byte intern IDs.
- Keep tuple keys private to query execution modules unless a strong test-support need exists.
- Do not revive the old sorted `TrieIter` LFTJ baseline as the GHT API.

## Non-Goals

- Do not implement COLT force here.
- Do not implement the Free Join executor here.
- Do not add durable tuple-key indexes here.
- Do not read directly from LMDB inside generic tuple/GHT tests except through PRD 09 base-image fixtures.

## Acceptance Criteria

- Multi-field tuple keys can be built from encoded columns.
- Multi-field tuple keys can be built from current variable bindings.
- Tuple equality and hashing are byte-stable and type-width-safe.
- Tuple schema rejects width/type mismatches.
- GHT interface is tuple-key based and separate from any future scalar fast path.
- Tests prove tuple keys for `(x)`, `(x, y)`, and mixed 1/8/16-byte values work.

## Required Tests

- Tuple key single-field equality/hash.
- Tuple key multi-field equality/hash.
- Tuple key with enum, serial, string intern ID, and bytes intern ID.
- Schema width mismatch rejection.
- Binding-to-tuple construction.
- Offset-to-tuple construction from base image.
- GHT mock implementing `iter` and `get` for tests.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb encoded_tuple --all-features
cargo test -p bumbledb-lmdb ght --all-features
cargo check --workspace --all-targets --all-features
```
