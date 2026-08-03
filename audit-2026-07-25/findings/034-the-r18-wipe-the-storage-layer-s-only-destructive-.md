## The R18 crash-wipe is the storage layer's only silent lifecycle branch — every benign lifecycle act fires an obs event, the one destructive act does not

observability | low | CONFIRMED | storage-v7
outcome: fixed 15342f00

### Summary

`Environment::ephemeral` implements the R18 crash contract: if the dirty marker exists at open, the possibly-torn store is never opened — `data.mdb` and `lock.mdb` are deleted and the store is re-initialized fresh. This is the only destructive act in the storage lifecycle, and it fires no obs event. Meanwhile every benign or recoverable lifecycle act in the same estate is instrumented: durable create's birth dirent chain (`CREATE_DURABLE`), compaction's durability chain (`COMPACT_DURABLE`), and transient commit-sync retries (`COMMIT_SYNC_RETRY`, whose in-code doctrine reads "never silent"). An operator reading a trace cannot distinguish "reopened existing ephemeral store" from "wiped it and started fresh" — the returned `Environment` carries no indicator either.

### Evidence (verified)

- `crates/bumbledb/src/storage/env/ephemeral.rs:66-74` — the crashed branch removes `data.mdb` and `lock.mdb` with no obs call. `grep 'obs::' ephemeral.rs` → zero hits. The subsequent open (lines 93-99) sets only `opened.dirty_marker = Some(marker)`; nothing distinguishes the fresh-init arm from the verified-reopen arm to a caller or trace.
- `crates/bumbledb/src/storage/env/create.rs:42-47` — `obs::event(CREATE_DURABLE, Category::Storage, 2, 0)` fires after the birth dirent chain, establishing the open-time storage-lifecycle event pattern.
- `crates/bumbledb/src/api/db/maintain.rs:63-68` — `COMPACT_DURABLE` fires after compaction's dirent chain (finding cited this as `maintain.rs`; the call site is under `api/db/`, not `storage/env/` — the substance is unaffected).
- `crates/bumbledb/src/storage/commit/write.rs:28` (doc) + `write.rs:46-53` (code) — "Each retry is an obs event (`COMMIT_SYNC_RETRY`), never silent" — the explicit in-repo doctrine for destructive-adjacent transients. The wipe is strictly more consequential than a retry.
- `crates/bumbledb/src/obs.rs:271-285` — the names block carries `COMMIT_SYNC_RETRY` (271), `COMPACT_DURABLE` (279), `CREATE_DURABLE` (285); no ephemeral, wipe, or open name exists anywhere in `obs.rs`.
- Only-silent-branch claim verified: across `crates/bumbledb/src/storage/env/`, only `create.rs` contains obs calls.
- Zero-cost-off by construction: `obs::event` is `#[cfg(feature = "trace")]` (`obs.rs:611-612`) with a `#[cfg(not(feature = "trace"))]` no-op stub; the wipe is open-time, nowhere near the join loops, so `docs/architecture/40-execution.md`'s measured-mechanisms doctrine is satisfied.

### Failure scenario / impact

An ephemeral store reopens empty after a host reboot — correct R18 behavior, the marker was set and the wipe destroyed nothing the kind promised to keep. The on-call engineer sees a day's facts gone, pulls the trace, and finds no create/wipe/open event to anchor on: the trace of a wiped-and-reinitialized store is indistinguishable from a verified reopen. Hours go into ruling out the write path, or the loss is misattributed to a commit/judgment bug. Severity is low (no data-correctness issue — the wipe semantics themselves are correct and ruled), but the diagnostic cost of the silence is real and the fix is one already-priced call.

### Suggested fix

One `obs::event(names::EPHEMERAL_WIPE, Category::Storage, files_removed, 0)` inside the crashed branch at `ephemeral.rs:66-74` (count the successful `remove_file`s for arg0). For symmetry with `CREATE_DURABLE`, an `EPHEMERAL_OPEN` event distinguishing the fresh-init and verified-reopen arms (e.g. arg0 = 1 verified / 0 fresh) is free at the same call shape around lines 94-98. Names documented in the `obs.rs` names block beside `CREATE_DURABLE`; test rides the existing `api/db/trace_tests.rs` pattern (cf. the `COMPACT_DURABLE` assertion at trace_tests.rs:321), which currently has zero ephemeral coverage.