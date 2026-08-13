# slice_in builds slices without rejecting count*size overflow or unaligned pointers
- id: 108
- severity: medium
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/lib.rs, cpp/bridge/src/value.rs, cpp/bridge/src/schema.rs, cpp/bridge/src/query.rs
- status: open (do not fix)

## Summary
`slice_in` is the only `(pointer, count)` helper. It refuses null when `count != 0`, then `from_raw_parts(ptr, count)`. Rust requires `count * size_of::<T>()` ≤ `isize::MAX` and that `ptr` be aligned for `T`. A hostile or overflowed `size_t` (bulk `row_count`, IR `binding_count`, schema `relation_count`, param `set_len`) is undefined behavior, not `MISUSE`. The helper already treats some contract breaks as `Misuse`, so this is an incomplete validation of the same class.

## Evidence
- `slice_in` (`cpp/bridge/src/lib.rs`): empty count → `&[]`; null+nonzero → `Misuse`; else `from_raw_parts` with no overflow/align check.
- Used for `bdb_value` rows, `bdb_param` arrays, `bdb_row_view` bulk lists, every IR/schema view graph (`query.rs`, `schema.rs`).
- `bdb_value` is a large `repr(C)` struct; `count = SIZE_MAX / 8` is enough to fail the `isize` rule on 64-bit.
- Alignment: a packed or byte-offset C pointer to `bdb_value`/`bdb_condition` violates `from_raw_parts`.

## Why this is a bug
Creating the slice is already UB; iterating it is not required. A fuzzer or a `size_t` wrap in a C caller can take this path without going through the C++ `span` wrappers (which are also just `.data()/.size()`). The bridge’s defensive null check shows the ABI is not “pure caller-UB”; overflow was left out.

## How to trigger / repro sketch
Call `bdb_db_bulk_load(db, rel, some_ptr, (size_t)-1, &committed, &err)` or `bdb_tx_insert(..., values, SIZE_MAX/2, ...)`. Expect crash/UB rather than MISUSE. Confirm with Miri on a unit test that passes a huge count (may need to skip actually mapping that many pages — Miri flags the `from_raw_parts` precondition immediately).

## Spec / docs notes
Header: views are `(pointer, count)` with count naming the allocation. Mis-sized counts are caller bugs; the inconsistency is that null is MISUSE and overflow is UB.

## Related
- 103 (same “documented MISUSE, actual UB” pattern for tags)

## Verification (2026-08-12)

**Verdict:** confirmed (was `likely`). Severity unchanged (medium).

**Trace:** `slice_in` (`cpp/bridge/src/lib.rs:189-207`): `count == 0` → `&[]`; null + nonzero → `Misuse`; else `from_raw_parts(ptr, count)` with no `count.checked_mul(size_of::<T>())` vs `isize::MAX` and no alignment check. Call sites include bulk `bdb_row_view` lists (`db.rs:787`), every IR graph (`query.rs:268, 379, 387-399, 409-424`), schema `relation_count` (`schema.rs:427`), `row_in` of `bdb_value` (`value.rs:174-175`). `bdb_value` is a large `repr(C)` struct; `count = SIZE_MAX` fails the `isize` rule on 64-bit without touching the allocation.

**Why it holds:** `from_raw_parts` preconditions are language UB at slice construction, not at first index. The helper already treats some contract breaks as `Misuse`, so overflow/misalignment are the same class left incomplete. A C `size_t` wrap or a byte-offset pointer is enough; C++ `span` wrappers still pass `.data()/.size()` into this helper.
