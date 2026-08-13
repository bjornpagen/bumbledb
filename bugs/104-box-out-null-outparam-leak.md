# box_out then failed out() leaks the Box (engine, row sets, prepared queries)
- id: 104
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/lib.rs, cpp/bridge/src/db.rs, cpp/bridge/src/query.rs
- status: open (do not fix)

## Summary
Owned handles are minted with `box_out` ( `Box::into_raw` ) and then written through `out(ptr, value)`. If the out-pointer is null, `out` returns `Misuse` and drops `value` — but `value` is only a `*mut T`, so the `Box` is leaked. For `bdb_db_create`/`open`/`ephemeral` that leak includes an open LMDB environment and its exclusive lock. The same pattern leaks `bdb_prepared` and `bdb_row_set`.

## Evidence
- `box_out` is `Box::into_raw(Box::new(value))` (`cpp/bridge/src/lib.rs`).
- `out` on null: `return Err(Fail::Misuse)` after taking `value: T` by value; dropping a raw pointer does not `from_raw`.
- `open_with` always `box_out`s a live `bdb_db` *then* `out(out_db, …)` (`cpp/bridge/src/db.rs`).
- Same for `bdb_db_prepare`, `bdb_tx_get` / `bdb_snapshot_get` (Some branch), `bdb_snapshot_scan`.
- C++ wrappers pass a real `T**`; the leak is on the public C ABI misuse path the header still defines (`null required pointer` → MISUSE, not abort).

## Why this is a bug
Misuse is supposed to be a programming error with no allocated error object — not an immortal LMDB lock. A caller that passes a null `out_db` after a successful open (or a fuzzer hitting the ABI) leaves the store locked until process exit. Row-set/prepared leaks grow with each call.

## How to trigger / repro sketch
```
bdb_db *db = NULL;
bdb_error *err = NULL;
bdb_db_create(path, &spec, NULL /* out_db */, &err); // status MISUSE, env leaked
// second create/open of the same path: EnvironmentLocked / lock file busy
```
Also `bdb_snapshot_scan(snap, rel, NULL, &err)` after a successful scan allocation.

## Spec / docs notes
Header: MISUSE means “no error is allocated.” It does not say “the operation is rolled back.” Create/open already opened the env before the out-param write.

## Related
- 105 (bulk_load commits then fails the out-param)
- 102 (handle lifetime)
