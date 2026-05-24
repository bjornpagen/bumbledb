# PRD 11: COLT Lazy Trie

## Purpose

Implement execution-local Column-Oriented Lazy Trie over immutable relation base images. COLT must implement the GHT API from PRD 10 and must be the correctness fallback for all valid logical atom access.

## Dependencies

- PRD 09.
- PRD 10.

## Scope

- COLT node data structure.
- Offset-vector leaves and lazy hash-map forcing.
- GHT schema construction for one atom occurrence from a formal Free Join plan.
- COLT counters for tests and future explain output.
- Execution over immutable relation base images loaded from real LMDB read snapshots.

## Required Model

A COLT node must contain:

- Atom occurrence or relation source metadata.
- Base image reference.
- Remaining GHT schema levels.
- Current tuple schema `vars`.
- Data as either an offset vector or a hash map from `EncodedTuple` to child node.

The exact implementation may use arenas or node IDs, but observable behavior must match the paper.

## Required Behavior

- `new(base, schema)` starts from all live offsets for the relation/atom occurrence.
- `base` must be the PRD 09 snapshot-local image derived from LMDB `L/C` state; COLT must not open LMDB transactions itself.
- `iter()` over a map returns map keys.
- `iter()` over an offset vector streams tuple values from base columns only when allowed by the current schema semantics.
- `iter()` over a non-streamable offset vector calls `force()` first.
- `get(tuple)` calls `force()` if needed and returns the matching child node if present.
- `force()` groups offsets by current tuple key and creates child nodes with remaining schema.
- Repeated `get()` or `iter()` on an already forced node does not rebuild the map.
- Empty relations and empty subtries work.
- COLT does not mutate cached base images.

## Technical Direction

- Implement execution-local mutable COLT nodes rather than mutating shared cached base images.
- Keep COLT execution-local. Do not persist COLT nodes, maps, or offset vectors back into LMDB.
- Use an arena to avoid borrow-checker problems with child nodes if needed.
- Build GHT schemas from `FreeJoinPlan` atom partitions, not durable indexes.
- Support unbalanced trees and maps/vectors at the same depth.
- Add instrumentation: nodes created, nodes forced, offsets scanned, hash maps built, get calls, misses, iter calls.
- Optional accelerator seeding is out of scope until the core COLT behavior is correct.

## Non-Goals

- Do not implement dynamic cover selection here.
- Do not implement vectorized execution here.
- Do not persist COLT nodes to LMDB.
- Do not require optional physical accelerators for correctness.
- Do not use an in-memory database as a source relation replacement; all source data originates from LMDB-backed base images.

## Acceptance Criteria

- COLT implements the GHT API.
- A relation used only as a directly iterated cover can avoid forcing any hash map.
- A lookup forces only the levels needed to satisfy that lookup.
- Unused deeper levels are not forced.
- Offset vectors reference base image row offsets, not durable fact IDs directly.
- COLT works without any declared physical index.
- Counters prove lazy behavior in tests.

## Required Tests

- Initial COLT contains one all-offset vector.
- Cover-only iteration creates no hash map.
- `get(x)` forces root once and finds child.
- Repeated `get(x)` does not force again.
- Lookup miss returns none and increments miss counter.
- Second-level lookup forces only selected child.
- Empty relation iteration and lookup work.
- Paper clover COLT shape can be built and partially forced as expected.
- COLT output equals an eager reference grouping for small relations.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb colt --all-features
cargo test -p bumbledb-lmdb ght --all-features
cargo check --workspace --all-targets --all-features
```
