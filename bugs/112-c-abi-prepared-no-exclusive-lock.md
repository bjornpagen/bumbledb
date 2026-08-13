# C ABI does not serialize PreparedQuery; concurrent execute is a data race on !Sync scratch
- id: 112
- severity: high
- confidence: confirmed
- area: ffi
- components: cpp/bridge/src/answers.rs, cpp/bridge/src/query.rs, crates/bumbledb/src/api/prepared.rs, cpp/foreign/raii.cc
- status: fixed (2026-08-13)

## Summary
`PreparedQuery` is `!Sync` and mutates per-execution scratch under `&mut self`. `bdb_snapshot_execute` takes a raw `*mut bdb_prepared` and `mut_in`s it with no handle mutex, generation, or “in_execute” flag (unlike `bdb_db`’s `in_write`). Two threads calling execute (or execute vs destroy) on the same prepared pointer race the plan/sink/bindings. C++ `prepared_handle` is move-only, which avoids sharing in the dialect, but the C ABI and any raw `bdb_prepared*` copy are unprotected.

## Evidence
- Engine: “Not shareable across threads”; `PhantomData<Cell<()>>` (`crates/bumbledb/src/api/prepared.rs`).
- `bdb_snapshot_execute`: `let prepared = mut_in(prepared)?` then `execute_args(&mut prepared.prepared, …)` (`cpp/bridge/src/answers.rs`).
- `bdb_prepared_destroy` is a separate entry with `box_in` — concurrent destroy vs execute is `from_raw` while the other thread holds `&mut`.
- Writes got a bridge-level `AtomicBool` specifically so the engine assertion never fires (`cpp/bridge/src/db.rs` `enter_write`). Prepared execute has no analog.
- raii `prepared_handle::execute` is non-const but has no lock (`cpp/foreign/raii.cc`).

## Why this is a bug
`!Sync` is a soundness constraint, not a style hint. The C ABI is a shareable `bdb_prepared*` (memcpy of the pointer). Concurrent `bdb_snapshot_execute` is undefined behavior: racing `Vec`s in the sink, COLT scratch, bind memos. Destroy-during-execute is a heap UAF. The Node bridge at least uses `RefCell` (see 100 for why that is still UB across threads); the C bridge uses nothing.

## How to trigger / repro sketch
Two threads, one `bdb_prepared*`, two live snapshots (or the same snapshot used unsafely):
`bdb_snapshot_execute` in both. TSan on the cdylib. Destroy variant: thread 2 `bdb_prepared_destroy` while thread 1 is in execute.

## Spec / docs notes
Export comments: “one execution at a time; the handle is not thread-shareable.” That is a comment, not a runtime check. Nested writes were considered worth a typed error; this was not.

## Related
- 100 (Node’s version of the same exclusive-access problem)
- 102 (destroy-while-in-use)

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (high).

**Trace:** Engine: “Not shareable across threads”; `Cell` marker (`crates/bumbledb/src/api/prepared.rs:176-187, 521-522`). `bdb_snapshot_execute` `mut_in`s the prepared pointer with no handle mutex or in-execute flag (`cpp/bridge/src/answers.rs:117-125`). `bdb_prepared_destroy` is a separate `box_in` (`query.rs:500-504`). Writes got `AtomicBool in_write` / `enter_write` so the engine assertion never fires (`db.rs:52, 239-248`). Export comment: “one execution at a time; the handle is not thread-shareable” (`query.rs:449-452`; header `:881`). raii `prepared_handle::execute` is non-const and unlocked (`raii.cc:388-390`). C++ `prepared_handle` is move-only, which avoids sharing in the dialect, not at the C ABI.

**Why it holds:** `!Sync` is a soundness constraint. A copyable `bdb_prepared*` plus two threads in `bdb_snapshot_execute` (or execute vs destroy) is a data race on plan/sink/bindings scratch, or a heap UAF on destroy-during-execute. Nested writes were given a typed error; this exclusive-access problem was left as a comment.

## Resolution (2026-08-13)

`bdb_prepared.in_execute` is an `AtomicBool`; concurrent execute is `EnvironmentLocked`, destroy while executing is `MISUSE` without `from_raw`. Leftover: flag-then-`from_raw` is best-effort against a racing destroy, the same class as `enter_write`.
