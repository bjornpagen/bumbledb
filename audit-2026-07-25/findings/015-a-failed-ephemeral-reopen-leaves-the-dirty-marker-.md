## A failed ephemeral open leaves the dirty marker armed over a cleanly-synced store; the next open wipes data the kind promised survives restarts

bug | medium | CONFIRMED | storage-v7
outcome: fixed 15342f00

### Summary

`Environment::ephemeral` arms and fsyncs the dirty marker before the flagged reopen and verification, and every fallible call after that point propagates with `?` and no marker cleanup. If any of them fails on a transient (fd/memory pressure, a failed fsync), the marker stays set over a store whose last close was clean and force-synced — and whose data pages the failed open provably never touched. The next `Db::ephemeral` reads the marker as a crash and wipes the store, destroying contents the R18 contract ("contents survive process restarts, never machine crashes", `docs/architecture/50-storage.md:571-572`; law: "an ephemeral store never destroys data it promised to keep") promises to keep. The documented wipe trigger — "power loss, or a process death that never reached clean close" — does not describe this case, and the in-process caller can positively distinguish it.

### Evidence (verified against code)

- `crates/bumbledb/src/storage/env/ephemeral.rs:91-100` — marker armed at 91 (`File::create(&marker)?.sync_all()?`), then four fallible calls with no Err-path cleanup: `sync_dirent_chain(path)?` (92), `open_env(..., Ephemeral)?` (93), `verify_and_open(...)?` (95) / `initialize(...)?` (97). `opened.dirty_marker = Some(marker)` only at 99, on success.
- `crates/bumbledb/src/storage/env.rs:266-278` — the only marker-clearing site is `Drop for Environment`, which runs only on a fully constructed handle; on any error above, the raw `heed::Env` drops without touching the marker.
- `crates/bumbledb/src/storage/env/ephemeral.rs:64-74` — the next open: `crashed = marker.try_exists()?` → removes `data.mdb` and `lock.mdb`, skips the probe (`!crashed` at 81), re-initializes empty.
- `crates/bumbledb/src/api/db/open.rs:67-73` — `Db::ephemeral` is a thin wrapper; no cleanup layer above.
- No-unproven-write proof for the loss case (`has_meta == true`): the probe (ephemeral.rs:82, durable-flagged lane) runs and every refusal fires **before** the marker is armed; `open_env` failing at 93 means the NOSYNC env never existed; in `verify_and_open` (`open.rs:60-98`) the only possible data write is the descriptor back-fill (90-97), which cannot trigger on a store born under the current format — `initialize` writes `META_SCHEMA_DESCRIPTOR` at birth (`create.rs:95`) — and a failed `commit` commits nothing.
- Marker-absent-implies-synced at entry: `Drop` clears the marker only after `force_sync()` succeeds (`env.rs:271-276`), so the store being wiped was proven synced at its last close. Pinned by `a_clean_ephemeral_close_clears_the_marker_and_contents_survive` (`env/tests.rs:55`).

### Failure scenario / impact

Day 1: an ephemeral staging store accumulates a day of judged facts; the process closes cleanly (force_sync, marker cleared, dirent chain synced). Day 2: a resume job calls `Db::ephemeral` under fd pressure; `open_env` fails `EMFILE` at ephemeral.rs:93, **after** the marker fsync. The job logs `Io` and its supervisor reruns it a minute later. The rerun finds the marker set, wipes `data.mdb`, and re-initializes empty — a day of staged facts destroyed by a transient error that never opened the NOSYNC environment, let alone wrote a page. Same outcome for a failure at line 92 (dirent sync) or an `Lmdb`/allocation failure inside `verify_and_open`.

### Suggested fix

Hoist the fallible tail (lines 93-98) into a closure/inner fn; on `Err`, best-effort clear the marker (`remove_file` + `sync_dirent_chain`) before propagating — safe because at every error point in that tail no NOSYNC transaction has committed (open never happened, or the shared body failed before/at its commit, which commits nothing on failure). Keep the marker armed on success exactly as today. Test: plant a failure after the marker is armed (e.g. a fault hook on the flagged open, or a typed verify failure via a schema-fingerprint skew introduced between probe and reopen is not reachable — prefer the fault hook), then pin that a subsequent `Db::ephemeral` preserves the prior contents instead of wiping.