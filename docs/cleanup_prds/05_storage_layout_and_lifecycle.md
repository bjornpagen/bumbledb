# PRD 05: Storage Layout And Lifecycle

## Status

Draft. This PRD should be implemented alongside or immediately after PRD 04 because index layout changes affect storage layout.

## Problem

Storage currently duplicates current row data in `NS_CURRENT_ROW` and full covering current indexes. Segment publication rebuilds full relation snapshots after each write, old segments are never garbage-collected, history is append-only, dictionary values are append-only, and query-image cache is unbounded.

This is acceptable for experimentation but not for a clean performance foundation.

## Goals

- Make row payload, index payload, history, and segments each have a crisp purpose.
- Keep direct read paths fast.
- Keep query-image build efficient.
- Avoid unbounded cache/lifecycle surprises where easy.
- Preserve LMDB-safe durability.
- Allow storage format breakage.

## Non-Goals

- No migrations.
- No unsafe LMDB durability flags.
- No server/background compactor.
- No vector or FlatBuffer blob policy.
- No as-of query implementation.

## Current Code References

- `NS_CURRENT_ROW`, `NS_CURRENT_TUPLE`, `NS_UNIQUE_GUARD`, `NS_HISTORY` in `storage.rs`.
- `current_row_key`, `current_index_key`, `unique_guard_key` in `storage.rs`.
- `append_relation_segment`, `build_relation_segment`, and `build_index_segment` in `storage.rs`.
- `visible_segments` and segment descriptor loading in `storage.rs`.
- `QueryImageCache` in `query_image.rs`.
- `Environment::storage_diagnostics` in `lib.rs`.

## Required Storage Decisions

### Current Row Store

`NS_CURRENT_ROW` is the authoritative row payload store.

Required behavior:

- Insert writes `NS_CURRENT_ROW` once.
- Replace overwrites `NS_CURRENT_ROW` once.
- Delete removes `NS_CURRENT_ROW` once.
- Direct point lookup can read `NS_CURRENT_ROW` without scanning primary covering index.
- Non-covering index scans can fetch rows from `NS_CURRENT_ROW` by primary identity.

### Current Index Store

`NS_CURRENT_TUPLE` stores physical access paths.

Required behavior:

- Index entries are unique keys with empty values.
- Index keys should not duplicate full rows unless explicitly designed.
- Index entry count metadata remains accurate.
- Index keys include enough identity bytes to fetch rows.

### Unique Guard Store

`NS_UNIQUE_GUARD` remains the enforcement mechanism for unique constraints.

Required behavior:

- Guard key is built from unique fields.
- Guard value is primary identity bytes.
- Replace with same primary can retain unique value.
- Conflicting primary fails atomically.

### History Store

History remains append-only for now.

Required cleanup:

- Document that history is not used for current reads.
- Ensure history records include enough row identity and payload bytes after index layout changes.
- Preserve failpoint coverage around history append.

### Segments

Segments are durable query-image inputs.

Required decisions:

- Keep full-relation segment publication for now if it preserves current benchmark wins.
- Make `segment:visibility:` either used or deleted.
- Expose segment lifecycle diagnostics clearly.
- Do not let single-row writes accidentally dominate non-join benchmarks in acceptance tests.

### Query Image Cache

The cache must become bounded or at least explicitly observable.

Required behavior:

- Diagnostics must expose cached image count, hit/miss/build counts, and build micros.
- Add an eviction policy or a documented explicit non-goal if delayed.
- Repeated writes must not make memory usage invisible.

## Implementation Plan

1. Make row store authoritative in read APIs where direct lookup is possible.
2. Update index write/read for PRD 04 layout policy.
3. Keep unique guard keyspace stable within the new format.
4. Audit history payload shape after row/index changes.
5. Decide whether to remove or read `segment:visibility:`.
6. Add diagnostics for segment meta count, active segment count, closed segment count, and segment bytes.
7. Add query-image cache bound or explicit clear/diagnostic method.
8. Update failpoint tests.

## Strict Passing Criteria

- `get_row` does not require a primary index scan when direct row key is available.
- Non-covering index scans still return correct full rows.
- Insert/replace/delete remain atomic under all existing failpoints.
- Unique guard behavior is unchanged in semantics.
- Segment-backed query image tests pass.
- Reopened database query-image uses durable segments correctly.
- `segment:visibility:` is either used in visible segment selection or removed from writes.
- Storage diagnostics expose enough lifecycle counters to reason about segment/cache growth.

## Verification Commands

```sh
cargo test -p bumbledb-lmdb storage query_image --features test-failpoints
cargo test --workspace --all-features
scripts/bench-focused.sh --fail-gates
```
