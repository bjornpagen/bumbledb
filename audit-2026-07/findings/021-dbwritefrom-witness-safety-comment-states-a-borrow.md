## dbWriteFrom witness SAFETY comment argues from a borrow that is never held; the raw `&Snapshot` outlives its argued window by the whole transaction

category: bug | severity: high | verdict: CONFIRMED | finder: r2:concurrency-unsafe-ffi
outcome: fixed 05d34c51 + 5b1c87eb

### Summary

The `unsafe` dereference of the witness snapshot address in `run_tx` (ts/crate/src/lib.rs:943) is justified by a SAFETY comment (lib.rs:931-936) whose two claims are both false as written:

1. **"dbWriteFrom holds the snapshot handle's live borrow"** — it does not. In `db_write_from` the `live(&snap.inner)` Ref is scoped to the block at lib.rs:1067-1073 and is dropped at the closing brace, *before* `spawn_tx(&inner, Some(witness))` at lib.rs:1074. During the entire deref window on the tx thread, no borrow of the snapshot handle exists anywhere.
2. **"the snapshot cannot close before write_from has read its generation"** — true for the prologue (the JS thread is parked in `begin_outcome`'s `recv`, lib.rs:1023-1041, until `serve` sends `TxReply::Ready` at lib.rs:888, and the generation read at crates/bumbledb/src/api/db/write.rs:162 happens before `write_witnessed` runs `serve`), but the reference does not end there. The `&Snapshot<'static>` at lib.rs:943 is a live local for the whole `db.write_from(snap, serve)` call, and `write_from`'s `witness: &Snapshot<'_, S>` parameter (write.rs:151-163) is a Stacked/Tree-Borrows-**protected** function argument for the entire transaction — `write_witnessed` runs the serve loop, parked in `requests.recv()` (lib.rs:892), inside `write_from`'s frame.

Once `Ready` unblocks JS, nothing in the native layer prevents `snapshotClose` (lib.rs:735-738 → `take_handle` at :130 → `SnapWorker::drop` at :526-533, which sends `SnapReq::Close` and joins) — or a GC finalizer of the snapshot `External` — from tearing down the snapshot worker. That makes `run_snapshot`'s `Db::read` closure break (lib.rs:653) and return, destroying the very `Snapshot` whose address `SnapReq::Witness` handed out (`std::ptr::from_ref(snap)`, lib.rs:652). Deallocating memory referenced by a protected argument is undefined behavior in the Rust abstract machine even if the reference is never read again. `TxWorker` (lib.rs:997-1003) holds no tie to the snapshot, so the invariant the comment gestures at is enforced by nothing.

### Evidence (all verified against the code)

- ts/crate/src/lib.rs:931-936 — the SAFETY comment quoted above.
- ts/crate/src/lib.rs:1067-1074 — `let witness = { let snap_worker = live(&snap.inner, "snapshot")?; ... };` then `spawn_tx(&inner, Some(witness))?` — the Ref dies before the spawn.
- ts/crate/src/lib.rs:943-944 — `let snap = unsafe { &*(address as *const Snapshot<'static, SchemaDescriptor>) }; db.write_from(snap, serve)`.
- crates/bumbledb/src/api/db/write.rs:151-163 — `witness` is read only at :156 (`env_instance`) and :162 (`generation`), but remains a live protected parameter while `write_witnessed(Some(witnessed), f)` runs `f` = the entire serve loop.
- ts/crate/src/lib.rs:652 — `SnapReq::Witness => SnapReply::Witness(std::ptr::from_ref(snap) as usize)`: the address is a stack-frame local of the snapshot worker's `Db::read` closure.
- ts/crate/src/lib.rs:735-738, 130-132, 526-533 — `snapshot_close` synchronously joins the worker, ending `Db::read` and destroying the `Snapshot` (thread stack freed on join).
- docs/architecture/70-api.md § conditional writes (cited in write.rs:122-132): the witness is "evidence, never a raw integer a caller could fabricate" — the engine API upholds this, but the FFI layer re-introduces exactly the raw-integer representation the doctrine refused, guarded only by prose.

### Mitigation found (does not refute)

The shipped TS SDK never opens the window: `witnessedAttempt`/`commitWitnessed` (ts/src/db.ts:1202-1307) call `closeScopeState` strictly *after* `txCommit`/`txAbort` on every exit path, including error paths. But this is host-side convention: the native surface is exported and used directly (the repo's own tests call `native.snapshotClose` etc.), and a GC finalizer of a dropped snapshot `External` reaches the same teardown with no call at all. The soundness of an `unsafe` block in the crate must not depend on the discipline of one JS wrapper.

### Failure scenario

Native-surface sequence: `const tx = native.dbWriteFrom(db, snap)` (returns ok; tx thread now parked in the serve loop inside `write_from`'s frame with `witness` protected) → `native.snapshotClose(snap)` joins the snapshot worker, `Db::read` returns, the `Snapshot` and its LMDB read txn are destroyed, the worker's thread stack is freed. At this instant the tx thread holds two dangling `&Snapshot<'static>` (run_tx's local, write_from's protected parameter) — UB per Stacked/Tree Borrows protector rules regardless of further use. Today no code path rereads `witness` after the prologue, so there is no observable corruption; but any future engine change that touches the witness after begin (e.g. re-checking the generation at commit, which the write.rs:134-139 doc discussion explicitly contemplates as a possible stats-surface evolution) becomes a silent use-after-free, and the SAFETY comment would still read as if it were sound.

### Suggested fix

Replace the pointer with a representation that makes the dangling state unrepresentable, preserving the engine's refusal of caller-fabricated integers: have the snapshot worker mint an opaque, moveable witness value — e.g. `Snapshot::witness_token(&self) -> WitnessToken` carrying the env instance id and the witnessed `GenerationId` (mintable only from a live `Snapshot`, no lifetime), plus `Db::write_from_token(WitnessToken, f)` performing the same env-identity and generation compare from the token's fields. `SnapReq::Witness` then replies with a value that crosses threads by move; `run_tx` never sees a `Snapshot` address, the `unsafe` block and its false comment are deleted, and snapshot close order becomes irrelevant to soundness. At minimum, if the pointer stays, the SAFETY comment must be rewritten to state the real (convention-only) argument — JS single-threadedness for the prologue plus the SDK's close-after-tx-end ordering for the remainder — and the tx handle should hold the snapshot worker alive (e.g. an `Arc` to the `SnapWorker`) so the guarantee is structural rather than prose.
