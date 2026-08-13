# Destroying or moving the db handle during its own read/write callback frees the engine under live refs
- id: 102
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/db.rs, cpp/foreign/raii.cc, cpp/src/db/db.cc, cpp/src/db/snapshot.cc
- status: fixed (2026-08-13)

## Summary
Re-entrant writes are refused with a typed error, but `bdb_db_destroy` (and C++ `Db`/`db_handle` move, which destroys the source) is not. A callback that destroys the handle `Box::from_raw`s the `bdb_db` the outer `bdb_db_read`/`bdb_db_write` still borrows. If that `Arc` is the last engine owner, the `Db` (and its LMDB env) is dropped while `Snapshot`/`WriteTx` pointers inside the callback are still live. C++ `Snapshot` also borrows `Db::manifest_` by reference, so moving `Db` inside `read` dangles that too.

## Evidence
- `bdb_db_read` does `let handle = ref_in(db)?` then `handle.db.read(|snap| { … callback … })`. Nothing prevents the callback from calling `bdb_db_destroy(db)` (`cpp/bridge/src/db.rs`).
- `bdb_db_destroy` is `drop(box_in(db)?)` — reclaim of the same allocation `handle` still references.
- `bdb_tx_ref::engine` SAFETY comment admits “destroying a db during its own callback is caller UB the alive flag cannot see.”
- Writes have `enter_write` / `in_write`; destroy and read re-entry do not.
- C++: `Db` is movable (`cpp/src/db/db.cc`); `Snapshot` holds `detail::Manifest const&` into that `Db` (`cpp/src/db/snapshot.cc`). A lambda `[&](Snapshot& snap) { auto stolen = std::move(db); … snap.contains(…) }` moves `manifest_` and can run `bdb_db_destroy` when `stolen` dies, still inside `read`.
- `db_handle` destructor calls `bdb_db_destroy` (`cpp/foreign/raii.cc`).

## Why this is a bug
Use-after-free of `bdb_db`, `Arc<Engine>`, LMDB txn, and (C++ dialect) `Manifest`. The bridge already treats a similar class of mistake (nested write) as a typed `EnvironmentLocked` error; destroy/move is the same class and is silent UB. Prepared-query `Arc` clones only delay the env drop if a prepared handle exists.

## How to trigger / repro sketch
C ABI:
```
bdb_db_read(db, callback, db, err);
// callback: bdb_db_destroy((bdb_db*)context); then bdb_snapshot_contains(...)
```
C++:
```
db.read([&](Snapshot& snap) {
  auto other = std::move(db);
  return snap.contains(...); // manifest_ and/or engine already moved/freed
});
```
Run under ASan.

## Spec / docs notes
C++ AGENTS.md ownership/lifetime rules: a sender/callback must not capture locals that die, and resource classes are unique owners. Moving the owner while a lexical capability borrows it is a lifetime violation the type system does not catch because `read` is `const` but the lambda can still `std::move` the named `Db` variable.

## Related
- 101 (stale ref after callback)
- 104 (related: destroy of other boxed handles)

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (high).

**Trace:** `bdb_db_read` does `let handle = ref_in(db)?` then `handle.db.read(|snap| { … callback … })` (`cpp/bridge/src/db.rs:424-430`). Nothing in that frame prevents the callback from calling `bdb_db_destroy(db)`, which is `drop(box_in(db)?)` (`:368-372`) — `Box::from_raw` of the allocation `handle` still borrows. `bdb_tx_ref::engine` SAFETY already admits “destroying a db during its own callback is caller UB the alive flag cannot see” (`:207-210`). Re-entrant *writes* are refused with typed `EnvironmentLocked` via `enter_write` (`:239-248, 463`); destroy and nested read are not. C++: `Db` is movable (`cpp/src/db/db.cc:180-181`); `Snapshot` / `WriteTx` hold `detail::Manifest const&` into that `Db` (`snapshot.cc:24-27`, `tx.cc:21-24`). `db.read` is `const` but the caller lambda can still `std::move` the named `Db` (`db.cc:201-216`). `db_handle::~` calls `bdb_db_destroy` (`cpp/foreign/raii.cc:641-645`).

**Why it holds:** This is UAF of `bdb_db` (the `Box`) plus, if that `Arc` was last, the engine/LMDB txn under live `Snapshot`/`WriteTx` pointers, plus a dangling `manifest_` after `std::move(db)` inside `read`. Nested write was treated as a recoverable typed error; destroy/move of the same owner is the same class of re-entrancy and is silent UB.

## Resolution (2026-08-13)

`in_read`/`in_write` refuse nested or concurrent read and re-entrant write with `EnvironmentLocked`. Destroy during a live callback returns `MISUSE` without `from_raw`. C++ `db_handle` counts live callbacks; move or destroy while the count is nonzero aborts. Leftover: concurrent C ABI reads on one handle are now typed-refused (single snapshot slot) even though the engine allows concurrent `Db::read`.
