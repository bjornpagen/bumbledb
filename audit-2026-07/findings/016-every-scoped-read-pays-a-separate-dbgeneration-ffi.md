## Every TS scoped read pays a second dbGeneration FFI call (and its fault-pairing branch) that the snapshot's Ready reply could carry for free

category: perf | severity: medium | verdict: CONFIRMED | finder: ts:core

### Summary

`db.read(fn)` — and therefore every sugar verb (`db.get`, `db.scan`, `db.contains`, `db.execute`) and every attempt of the witnessed-write loop — makes two native calls before the user callback runs: `dbSnapshot`, then `dbGeneration`. The second call exists only to populate the diagnostic `ReadScope.generation`, opens its own transient LMDB read txn on the engine side, and forces a hand-written fault-pairing branch (`generationForScope`) whose entire purpose is to close the already-open snapshot when the second call throws. The snapshot worker already executes inside `Db::read` where the snapshot's own generation is computable (`ReadTxn::generation`), and it already sends a `SnapReply::Ready` at open — the generation could ride that reply with zero additional crossings. This is a textbook representation-over-control-flow finding: a better representation (snapshot open returns its generation) erases both the extra crossing and the branch.

### Evidence (all verified)

- **Two crossings per read:** `ts/src/db.ts:1002-1005` — `read()` calls `openScopeState()` (→ `native.dbSnapshot`, db.ts:964-965) then `generationForScope(state)` (→ `native.dbGeneration`, db.ts:988-999). `witnessedAttempt` repeats the pair at db.ts:1241-1242, so every retry of `writeWitnessed` pays it too.
- **The integer is purely informational:** `ReadScope.generation` (ts/src/db.ts:246-253) is documented as the scope's witnessed generation; the write path never consumes it — `dbWriteFrom` takes the snapshot *handle* (db.ts:1247), and the bridge witnesses via the snapshot worker's `SnapReq::Witness` address (ts/crate/src/lib.rs:929-944). The bridge itself says so: `db_generation` is "diagnostics; the write-side witness is always the snapshot handle, never this integer" (lib.rs:352-354).
- **The second call opens a whole extra engine read txn:** lib.rs:356-362 → `Db::generation` at `crates/bumbledb/src/api/db/maintain.rs:90` is `self.env.read_txn()?.generation()` — a fresh LMDB reader-slot acquisition per scoped read, doubling read-txn opens for a point read.
- **The free ride exists:** `run_snapshot` (ts/crate/src/lib.rs:606-610) sends `SnapReply::Ready(Ok(()))` from *inside* `db.read(|snap| …)`; `ReadTxn::generation` exists (`crates/bumbledb/src/storage/env/readtxn.rs:15`) and memoizes. The one missing piece: `Snapshot::txn()` is `pub(crate)` (api/db.rs:396-403) and `Snapshot` has no public `generation()`, so the fix includes a one-line engine addition.
- **The branch the representation erases:** `generationForScope` (ts/src/db.ts:979-1000) exists solely for the fault window where `dbGeneration` throws with a snapshot already open — reader-table exhaustion being exactly the state in which it throws — plus the `liveSnapshots` census pairing. With the generation riding the Ready reply, that window is unreachable, not handled.
- **Doctrinal divergence:** `readtxn.rs:6-10` states the reader's generation is "the storage tx id read from `_meta` *inside this snapshot*", the race-closing rule of `docs/architecture/50-storage.md` ("a reader's generation T is the storage tx id read from…", line 584). The TS host instead reads it from a *separate* transient txn and defends atomicity with prose (db.ts:248-252: "nothing can commit between the snapshot open and the generation read", true only because this process holds the sole write handle). Reading it inside the worker's own snapshot restores the spec's rule by construction.
- **Amplifier:** each `dbSnapshot` spawns a dedicated OS thread with channel round-trips (lib.rs:707-731, `std::thread::spawn` at 712), so a `db.get` today is thread-spawn + 4 crossings (snapshot, generation, get, close); the fix removes one crossing and one LMDB txn open.

### Bench impact (corrected)

The finder claimed the point-read benchmark lanes improve. **That part is refuted:** `docs/architecture/61-bench-lanes.md` and `scripts/bench-night.sh` show every Report-class lane is a subcommand of the Rust bench binary (`crates/bumbledb-bench`); no lane runs through the TS bridge. The real impact is TS SDK per-read latency: every point read through the sugar verbs drops from 4 native crossings + 2 LMDB read-txn opens to 3 crossings + 1 open, and every `writeWitnessed` attempt sheds the same. The per-snapshot thread spawn remains the dominant cost of that path (an adjacent, larger issue), which is why severity is medium rather than the finder's high.

### Suggested fix

1. Engine: add `pub fn generation(&self) -> Result<GenerationId>` to `Snapshot` (delegating to `self.txn.generation()` — one line next to the existing `pub(crate) fn txn()` at api/db.rs:400).
2. Bridge: have `run_snapshot` send `SnapReply::Ready(Ok(generation))` (reading `snap.generation()` inside the `db.read` closure), and have `db_snapshot` return the pair — either a `#[napi(object)]` `{ handle, generation }`, or cache the `u64` in `SnapshotHandle` behind a trivially infallible `snapshotGeneration` accessor if marshaling an `External` inside an object proves awkward.
3. Host: `makeScope` takes the pair from open; delete `generationForScope` and its fault-pairing close dance (ts/src/db.ts:979-1000); delete `db_generation`'s role in the read path (it can remain as a standalone diagnostic).

Atomicity improves as a side effect: the generation is read inside the same engine read closure as the snapshot, restoring 50-storage's race-closing rule by representation instead of by the single-writer argument.
