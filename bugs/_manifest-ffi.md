# FFI / unsafe / allocation audit manifest (ids 100–117)

Number range 100–199. One finding per file. Bashed 2026-08-13: all 17 survivors fixed.

Verified 2026-08-12 against C/C++/Rust source. Deleted 110 and 118 (see `_rebuttals-ffi.md`). No new ids.

| file | severity | confidence | one-line summary |
|---|---|---|---|
| 100-napi-prepared-cross-thread-aliasing.md | critical | confirmed | NAPI ships a `PreparedQuery` address to a worker while JS still holds `RefMut`; aliasing UB on every execute. |
| 101-stale-snapshot-ref-is-stack-uaf.md | high | confirmed | Snapshot/tx `alive` flag lives in a stack object; post-callback use is UAF, not MISUSE. |
| 102-destroy-db-during-callback-uaf.md | high | confirmed | `bdb_db_destroy` / C++ `Db` move during read/write frees the engine under live snapshot/tx/manifest refs. |
| 103-repr-c-enum-and-bool-unchecked.md | high | confirmed | Inbound `#[repr(C)]` enums and `bool`s are matched without discriminant checks; docs promised MISUSE. |
| 104-box-out-null-outparam-leak.md | high | confirmed | `box_out` + null `out()` leaks `Box` (LMDB env, prepared, row sets). |
| 105-bulk-load-null-out-committed.md | high | confirmed | Bulk load can commit (or partially commit) then return MISUSE if `out_committed` is null. |
| 106-napi-tx-open-stuck-on-spawn-panic.md | medium | confirmed | `tx_open` stays true if `thread::spawn` panics; later writes refuse forever. |
| 107-unguarded-extern-panic-wall.md | low | confirmed | Several externs skip `catch_unwind`; panic into `-fno-exceptions` C++ is UB. (Downgraded: no current panic site.) |
| 108-slice-in-count-overflow.md | medium | confirmed | `slice_in` / `from_raw_parts` does not reject `count*size` overflow or misalignment. |
| 109-napi-take-handle-refcell-panic.md | medium | confirmed | `take_handle` uses panicking `borrow_mut`; re-entrant close unwinds into napi. |
| 111-cpp-answer-value-borrow-escape.md | medium | confirmed | `cell()` / `Value` copies borrowed string/bytes pointers with no lifetime. |
| 112-c-abi-prepared-no-exclusive-lock.md | high | confirmed | C `bdb_prepared*` has no exclusive lock; concurrent execute/destroy races `!Sync` scratch. |
| 113-cpp-exception-through-rust-callback.md | low | confirmed | A throwing C++ callback unwinds through Rust; `catch_unwind` cannot catch it. (Downgraded: in-tree `-fno-exceptions`.) |
| 114-store-error-overwrites-without-free.md | low | confirmed | Reused `bdb_error**` without destroy leaks the previous error. |
| 115-stale-ref-test-is-uaf.md | info | confirmed | `stale_snapshot_ref_is_misuse` dereferences a dropped stack ref (green-washes 101). |
| 116-tx-ref-mut-from-ref-aliasing.md | medium | confirmed | `transaction(&self) -> &mut WriteTx` plus non-atomic `alive`; cross-thread callback use is UB. (Rewritten: dropped same-thread nested aliasing.) |
| 117-moved-from-error-kind-is-panic.md | low | confirmed | Moved-from `error_handle::kind()` returns `PANIC` instead of aborting. |

## Counts
- critical: 1
- high: 6
- medium: 5
- low: 4
- info: 1
- total: 17

## Surfaces covered
- `cpp/bridge` C ABI (all `extern "C"` exports, `guard`, `box_out`/`box_in`, callbacks, IR/schema/value marshal)
- `cpp/foreign` (generated `bumbledb_c.h`, raii, program/param wire)
- `cpp/src` dialect Snapshot/Db/WriteTx/answers decode
- `ts/crate` NAPI (workers, prepared pointer, RefCell, tx_open)
- Engine unsafe used *through* those bridges (PreparedQuery `!Sync`/transmute drop order, `execute_args` clear, `EscapedIdBurn`)
- Engine kernel/wordmap/image `unsafe`: reviewed; no confirmed FFI-crossing defect filed (Copy-only MaybeUninit, documented decode bounds)

## Deliberately not filed
- `PreparedQuery<'static>` transmute in both bridges: drop order (`prepared` then `Arc`) and `schema: &Schema` into Arc-pinned `Db` look sound.
- Node `dbWriteFrom` `Witness` value (the old 018 `&'static Snapshot` path).
- Double-destroy of C handles (documented once-only; C++ RAII).
- Engine COLT/kernel `get_unchecked` (in-engine invariants, not an FFI ownership transfer).
- Inbound `as_str`/`slice_in` caller-chosen `'a` with no current escape (deleted 118).
- Execute carrier “cleared first” on failure (deleted 110; documented on C ABI and `execute_into`).
