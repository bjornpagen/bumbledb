# Destroying or moving the db handle during its own read/write callback frees the engine under live refs
- id: 102
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/db.rs, cpp/foreign/raii.cc, cpp/src/db/db.cc, cpp/src/db/snapshot.cc
- status: open (do not fix)

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
