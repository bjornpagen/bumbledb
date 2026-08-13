# Stale snapshot/tx “alive” flag cannot implement documented MISUSE; post-callback use is stack UAF
- id: 101
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/db.rs, cpp/bridge/src/tests.rs, cpp/foreign/bumbledb_c.h, cpp/foreign/raii.cc
- status: fixed (2026-08-13)

## Summary
`bdb_snapshot_ref` / `bdb_tx_ref` are stack values inside the Rust `Db::read` / `Db::write` closures. The header, module docs, and a unit test claim that stashing the pointer and using it after the callback returns `BDB_STATUS_MISUSE` via an `alive` flag. After the callback, the stack object is dropped; any later load of `alive` is a use-after-free of that stack slot. The flag can only look like it works while that memory has not been reused.

## Evidence
- Refs are locals in `bdb_db_read` / `write_with`, passed to C as `&snapshot_ref`, then `invalidate()` runs, then the closure returns and the local is dropped (`cpp/bridge/src/db.rs`).
- Header: “stashed ref answers `BDB_STATUS_MISUSE` instead of being replayed” (`cpp/foreign/bumbledb_c.h`).
- The ABI test stashes the pointer and replays it *after* `bdb_db_read` returns, and even comments that “the frame’s memory is gone in principle” (`stale_snapshot_ref_is_misuse` in `cpp/bridge/src/tests.rs`).
- Types are opaque incomplete structs in C, so C cannot copy the bytes; it can only stash the pointer. That is exactly the dangling-stack-pointer case. A heap copy of the struct (if a caller `memcpy`s after obtaining a size by cheating) would keep `alive == true` and then dereference the erased `Snapshot*` — also UAF.

## Why this is a bug
The promised safety property is not implementable with a flag stored in the stack object that dies at callback return. ASan/Miri would flag the test itself. In production, a C or raii.cc caller who keeps `bdb_snapshot_ref const*` past `read()` has a dangling pointer; they may crash, see a “lucky” MISUSE if the bytes still read `false`, or pass the check and use a dangling `Snapshot`.

## How to trigger / repro sketch
1. Raw ABI: in the read callback, save `snapshot` to a global; after `bdb_db_read` returns, call `bdb_snapshot_contains` (the existing unit test already does this).
2. Rebuild the bridge tests with ASan and run `stale_snapshot_ref_is_misuse`.
3. raii layer: `db.read([&](bdb_snapshot_ref const& s) { leaked = &s; return OK; })` then use `*leaked`.
4. The dialect `bdb::Snapshot` is non-copyable, which mitigates accidental copies, but a `Snapshot*` to the trampoline local still dangles.

## Spec / docs notes
Violates the C ABI contract in `bumbledb_c.h` and `cpp/bridge/src/lib.rs` (“lexical capabilities … every use re-checks the alive flag”). A generation counter on `bdb_db` (or heap-allocated refs) would be required to actually return MISUSE.

## Related
- 102 (destroy/move of the owner during the same callback)
- 115 (the unit test is itself UB)

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (high).

**Trace:** `bdb_snapshot_ref` is a stack local inside `handle.db.read` (`cpp/bridge/src/db.rs:428-431`): minted, handed to C as `&raw const *snapshot_ref`, `invalidate()` (sets `alive = false` on that same local, `:155-157`), then the closure returns and the local is dropped. `snapshot()` loads `self.alive.get()` then dereferences `self.snap` (`:136-152`). `bdb_tx_ref` is the same shape in `write_with` (`:467-470, 216-218`). Header still promises a stashed ref “answers `BDB_STATUS_MISUSE` instead of being replayed” (`cpp/foreign/bumbledb_c.h:285-289`). The ABI test stashes the pointer and replays it after `bdb_db_read` returns (`cpp/bridge/src/tests.rs:995-1012`), commenting that “the frame’s memory is gone in principle.” Types are incomplete in C, so the only stash is the pointer.

**Why it holds:** After the callback, the `alive` flag does not exist. A later `bdb_snapshot_contains` / `bdb_db_write_from` is a load of a dropped stack slot, then (if the bytes still look live) a dereference of the erased `Snapshot*`. `invalidate()` can only help while the object is still on the stack — i.e. not for the documented post-callback MISUSE path. C++ `Snapshot` is non-copyable (`cpp/src/db/snapshot.cc:32-34`), which blocks accidental copies, not a stashed `Snapshot*` / `bdb_snapshot_ref const*`.

## Resolution (2026-08-13)

Refs live in heap slots inside `bdb_db`. The callback receives a pointer into that slot; `invalidate()` runs on every closure exit (Drop). A stashed pointer is real `BDB_STATUS_MISUSE`, not stack UAF. Nested `write_from` during read still uses the live slot.
