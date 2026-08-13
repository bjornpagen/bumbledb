# Inbound repr(C) enums and bools are matched without validating discriminants (UB, not MISUSE)
- id: 103
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/value.rs, cpp/bridge/src/query.rs, cpp/bridge/src/schema.rs, cpp/bridge/src/lib.rs, cpp/bridge/src/db.rs
- status: open (do not fix)

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
