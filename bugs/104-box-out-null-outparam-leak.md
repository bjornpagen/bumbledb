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

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (high).

**Trace:** `box_out` is `Box::into_raw(Box::new(value))` (`cpp/bridge/src/lib.rs:231-233`). `out` on null takes `value: T` by value and returns `Fail::Misuse` (`:211-214`); `T` here is `*mut bdb_db` / `*mut bdb_prepared` / `*mut bdb_row_set`, so drop does not `from_raw`. `open_with` always mints a live `bdb_db` then `out(out_db, box_out(...))?` (`db.rs:312-319`) — same after a successful `Db::create`/`open`/`ephemeral`. Same pattern: `bdb_db_prepare` (`query.rs:486-492`), `bdb_snapshot_scan` (`db.rs:757`), Some-branch of `bdb_tx_get` / `bdb_snapshot_get` (`:638-644, 724-729`). C++ wrappers pass a real `T**`; the leak is the public C ABI null-required-pointer path the header still defines as MISUSE.

**Why it holds:** MISUSE is supposed to allocate no error object, not leave an immortal LMDB env/lock. A null `out_db` after a successful open is a programming error the ABI explicitly classifies as MISUSE, so the engine work must not `into_raw` before the out-param is checked (or the raw pointer must be `from_raw`’d on that error).
