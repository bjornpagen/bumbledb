# Inbound repr(C) enums and bools are matched without validating discriminants (UB, not MISUSE)
- id: 103
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/value.rs, cpp/bridge/src/query.rs, cpp/bridge/src/schema.rs, cpp/bridge/src/lib.rs, cpp/bridge/src/db.rs
- status: fixed (2026-08-13)

## Summary
The C ABI protocol documents that an unknown enum tag is `BDB_STATUS_MISUSE`. Every inbound marshal path instead `match`es a `#[repr(C)]` enum (or reads a Rust `bool`) directly. An out-of-range C `enum` or a `bool` that is not 0/1 is undefined behavior in Rust *before* any `Misuse` can be returned. The same applies to `bdb_callback_control` returned from the caller’s C function pointer.

## Evidence
- Protocol: “unknown enum tag” → `BDB_STATUS_MISUSE`, no error allocated (`cpp/bridge/src/lib.rs` module doc; `cpp/foreign/bumbledb_c.h`).
- `value_in` matches `view.kind` exhaustively with no integer fallback (`cpp/bridge/src/value.rs`). Same for `bdb_param_kind`, `bdb_term_kind`, `bdb_condition_kind`, `bdb_cmp_op_kind`, `bdb_head_op`, schema spec kinds, etc.
- `call_read_callback` / `call_write_callback` return the C function’s `bdb_callback_control` and then match `Ok` vs `Abort` (`cpp/bridge/src/db.rs`).
- C `enum` is an `int`-sized bag of integers; cbindgen emits unscoped C enums. Rust `#[repr(C)] enum` may only be 0..N-1. Rust `bool` may only be 0 or 1; `bdb_value.bool_value` is a `bool` field in a POD the caller fills.
- No `try_from` / discriminant range check exists anywhere in the bridge.

## Why this is a bug
Invalid discriminants are not a recoverable `Misuse`; they are immediate UB (wrong jump table, LLVM assuming impossible values, or a `bool` load of `2`). Easy to hit with uninitialized `bdb_value` on the C side, a corrupted buffer, or a callback that returns `(bdb_callback_control)2`. The documentation promised a defined status.

## How to trigger / repro sketch
1. Zero a `bdb_value` then set `kind = (bdb_value_kind)99` (or `memset` to 0xFF) and pass it to `bdb_tx_insert`.
2. Read callback: `return (bdb_callback_control)2`.
3. Set `bool_value = (bool)2` with `kind = BOOL` (C allows this; assigning through `_Bool` may saturate, but a raw byte write into the struct does not).
4. Run under Miri (bridge unit tests can construct the invalid repr with `transmute` if needed).

## Spec / docs notes
Directly contradicts the generated header’s boundary protocol. C++ dialect wrappers usually value-initialize structs (kind 0 = Bool), so the SDK happy path is fine; the C ABI and any uninitialized stack struct are not.

## Related
- 108 (another “we null-check but do not validate size/tag” hole in `slice_in`)

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (high).

**Trace:** Rust module doc lists “an unknown enum tag” as `BDB_STATUS_MISUSE` (`cpp/bridge/src/lib.rs:22-25`). No `try_from` / discriminant range check exists in the bridge. `value_in` exhaustively matches `view.kind` (`cpp/bridge/src/value.rs:140-170`); `bdb_value.bool_value` is a Rust `bool` (`:96`). Same pattern: `bdb_param_kind` (`:309-310`), `bdb_term_kind` (`query.rs:254-260`), `bdb_head_op` (`:282-291`), `bdb_find_term_kind` (`:314-325`), `bdb_cmp_op_kind` (`:338-349`), `bdb_condition_kind` (`:368-372`), schema spec kinds (`schema.rs:253-268` and following). `call_read_callback` returns the C function’s `bdb_callback_control` and the outer match is `Ok` vs `Abort` only (`db.rs:432-437, 471-476`). cbindgen emits unscoped C enums (`cpp/foreign/bumbledb_c.h:40-57` etc.). Note: the *generated* header protocol blurb (`:8-14`) names null / stale ref / index, not “unknown enum tag”; the promise lives in the Rust module doc that cbindgen is generated from.

**Why it holds:** A `#[repr(C)]` enum may only hold 0..N−1; a Rust `bool` may only be 0 or 1. An out-of-range C tag or a `0x02` in `bool_value` is UB at the `match` / bool load, before any `Misuse` can be returned. That contradicts the documented MISUSE lane. C++ dialect value-initialization makes the SDK happy path `kind == 0`; the raw C ABI and uninitialized stack structs do not.

## Resolution (2026-08-13)

Inbound tags are `u32`/`u8` with `tag_in`/`bool_in` before any `match`. Callbacks return `u32`, then `tag_in`. cbindgen regenerated; unknown tags and bool bytes other than 0/1 are `MISUSE`.
