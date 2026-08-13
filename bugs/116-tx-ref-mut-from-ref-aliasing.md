# bdb_tx_ref::transaction yields &mut WriteTx from &self; nested FFI entries alias it
- id: 116
- severity: medium
- confidence: likely
- area: ffi
- components: cpp/bridge/src/db.rs
- status: open (do not fix)

## Summary
`bdb_tx_ref::transaction` is `fn(&self) -> &mut WriteTx` (`clippy::mut_from_ref` expect). Each `bdb_tx_*` entry `ref_in`s the same `bdb_tx_ref` and reborrows `&mut` to the erased transaction. That is sound only if no two such `&mut`s exist at once. The write callback is C: it can call `bdb_tx_insert` from two OS threads, or (on one thread) hold a pointer obtained some other way. There is no “in_tx_op” flag analogous to `in_write`. `Cell<bool> alive` is also non-atomic, so concurrent `invalidate` vs `transaction` is a data race.

## Evidence
- `transaction(&self) -> BridgeResult<&mut WriteTx<…>>` with SAFETY: “the callback protocol is synchronous single-thread, so this is the only reference” (`cpp/bridge/src/db.rs`).
- `alive: Cell<bool>` — not `AtomicBool`. `in_write` on `bdb_db` *is* atomic, showing the authors know the db handle may be hit from two threads; the tx ref is not given the same treatment.
- `bdb_tx_insert` and friends are `extern "C"` and can be entered from any thread that captured `bdb_tx_ref*`.
- C++ `WriteTx` is non-copyable and lives on the trampoline stack (`cpp/src/db/tx.cc`), which helps the dialect but not a raw `bdb_tx_ref*` shared into a `std::thread` inside the callback (the raii `TxBody` is just a lambda; nothing stops it from spawning).

## Why this is a bug
Two `&mut WriteTx` or a `Cell` data race is UB. The single-thread claim is a comment. A callback that does `std::thread t([&]{ tx_insert(...); }); tx_insert(...); t.join();` is representable in C++20 even if dialect AGENTS.md forbids `std::thread` in `src/` — `foreign/` raii and raw ABI users are not the dialect.

## How to trigger / repro sketch
Inside a write callback (raw ABI or raii `db.write`): spawn a thread that calls `bdb_tx_insert` on the same `transaction` pointer while the original thread also inserts. TSan. Same-thread nested aliasing is harder because each extern call’s `&mut` drops before return — unless a future engine hook re-enters C.

## Spec / docs notes
Header: callbacks are “synchronous, on the calling thread.” It does not say the pointer is not `Send`. Dialect forbids raw threads; the C ABI does not.

## Related
- 112 (prepared execute, same missing exclusive)
- 100 (Node actually *does* cross threads, on purpose)
