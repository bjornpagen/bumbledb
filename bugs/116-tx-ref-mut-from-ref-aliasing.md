# bdb_tx_ref::transaction yields &mut WriteTx from &self; cross-thread use aliases it
- id: 116
- severity: medium
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/db.rs
- status: fixed (2026-08-13)

## Summary
`bdb_tx_ref::transaction` is `fn(&self) -> &mut WriteTx` (`clippy::mut_from_ref` expect). Each `bdb_tx_*` entry `ref_in`s the same `bdb_tx_ref` and reborrows `&mut` to the erased transaction. Same-thread sequential FFI calls are sound (each entry’s `&mut` drops before return). The C ABI pointer is still shareable: a callback that hands `bdb_tx_ref*` to another OS thread can create two live `&mut WriteTx` values, and `Cell<bool> alive` is a data race against `invalidate`. There is no “in_tx_op” flag analogous to `in_write`.

## Evidence
- `transaction(&self) -> BridgeResult<&mut WriteTx<…>>` with SAFETY: “the callback protocol is synchronous single-thread, so this is the only reference” (`cpp/bridge/src/db.rs:172-195`).
- `alive: Cell<bool>` (`:73, 86`) — not `AtomicBool`. `in_write` on `bdb_db` *is* atomic (`:52, 239-248`), showing the authors know the db handle may be hit from two threads; the tx ref is not given the same treatment.
- `bdb_tx_insert` and friends are `extern "C"` (`:549-566`) and can be entered from any thread that captured `bdb_tx_ref*`.
- C++ `WriteTx` is non-copyable and lives on the trampoline stack (`cpp/src/db/tx.cc:20-24`), which helps the dialect but not a raw `bdb_tx_ref*` shared into a `std::thread` inside the callback.

## Why this is a bug
Two `&mut WriteTx` or a `Cell` data race is UB. The single-thread claim is a comment. A callback that does `std::thread t([&]{ tx_insert(...); }); tx_insert(...); t.join();` is representable in C++20 even if dialect AGENTS.md forbids `std::thread` in `src/` — `foreign/` raii and raw ABI users are not the dialect.

## How to trigger / repro sketch
Inside a write callback (raw ABI or raii `db.write`): spawn a thread that calls `bdb_tx_insert` on the same `transaction` pointer while the original thread also inserts. TSan.

## Spec / docs notes
Header: callbacks are “synchronous, on the calling thread.” It does not say the pointer is not `Send`. Dialect forbids raw threads; the C ABI does not.

## Related
- 112 (prepared execute, same missing exclusive)
- 100 (Node actually *does* cross threads, on purpose)

## Verification (2026-08-12)

**Verdict:** confirmed (was `likely`). Severity unchanged (medium). **Dropped:** same-thread nested aliasing as a current bug. Each `extern "C"` `bdb_tx_*` builds `&mut WriteTx` for that call only; it drops before return, so sequential re-entry on one thread does not stack two `&mut`s. No engine hook currently re-enters C while one is live.

**Trace:** `transaction(&self) -> &mut WriteTx` (`db.rs:172-195`); `alive` is `Cell<bool>` (`:73, 86`); `bdb_tx_insert` `ref_in` then `transaction()?` (`:549-566`). Contrast `enter_write`’s `AtomicBool`.

**Why the remaining claim holds:** The SAFETY comment’s “this is the only reference” is a protocol assumption, not a runtime exclusive. Sharing the C pointer across threads during the callback is representable and is aliasing UB / a `Cell` data race. Same exclusive-access gap as 112, for the lexical tx capability.

## Resolution (2026-08-13)

`alive` is `AtomicBool`. `in_op` makes `transaction()` exclusive; a second concurrent call is `MISUSE`. The SAFETY comment names that exclusive.
