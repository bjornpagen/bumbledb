## Witnessed-write `&Snapshot` dangles for the whole transaction if the snapshot closes before commit

category: bug | severity: high | verdict: CONFIRMED | finder: ts:bridge
outcome: fixed 5b1c87eb + 05d34c51 + a0365a14

### Summary

The witnessed-write entry in the N-API bridge turns a raw usize into a `&'static Snapshot` and passes it as the argument to `Db::write_from`. The engine only *reads* the witness at entry, but as a function argument the reference is protected for the entire `write_from` call — and the serve loop that processes every `txInsert`/`txCommit` runs *inside* that call. The published native sequence `dbWriteFrom(db, snap)` → `snapshotClose(snap)` → `txCommit(tx)` deallocates the Snapshot (a stack local on the snapshot worker thread, which `snapshotClose` joins) while the tx worker's protected reference still dangles. Deallocating memory with an active argument protector is undefined behavior under both Stacked Borrows and Tree Borrows even when the reference is never dereferenced again, and the `'static` lifetime erasure guarantees zero compile-time signal if the engine ever grows a post-entry witness use (e.g. a commit-time generation re-check, which the code structure invites). The `SAFETY` comment justifying the unsafe block is factually wrong about the code as written.

### Evidence (all verified against the working tree)

- **The fabricated reference and its scope** — `ts/crate/src/lib.rs:929-945`: `run_tx` does `let snap = unsafe { &*(address as *const Snapshot<'static, SchemaDescriptor>) }; db.write_from(snap, serve)`. The reference is `write_from`'s argument, so it lives in that frame until `write_from` returns.
- **The transaction runs inside that frame** — `crates/bumbledb/src/api/db/write.rs:151-163`: `write_from(witness: &Snapshot<'_, S>, f)` reads the witness exactly twice at entry (`env_instance()` check at :156, `witness.txn().generation()?` at :162) and then calls `self.write_witnessed(Some(witnessed), f)` — the closure `f` is the bridge's serve loop (`ts/crate/src/lib.rs:886-927`), which blocks on `requests.recv()` for every tx verb until commit/abort. The witness reference remains a live, protected argument the whole time.
- **The SAFETY comment's borrow claim is stale** — `ts/crate/src/lib.rs:1067-1074` (`db_write_from`): the `live(&snap.inner, "snapshot")?` Ref is scoped to the witness-address block and drops at the closing brace on :1073, *before* `spawn_tx(&inner, Some(witness))` on :1074. The comment at :931-936 ("dbWriteFrom holds the snapshot handle's live borrow and blocks on the begin verdict") is half true: only the begin-verdict block is real, and it covers only the entry reads — `TxReply::Ready` is sent at :888, after `write_from` has read the generation.
- **The pointee is a dead thread's stack after close** — `crates/bumbledb/src/api/db/read.rs:30-41`: the `Snapshot` is a local of `Db::read` on the snapshot worker thread (`run_snapshot`, `ts/crate/src/lib.rs:606-660`; the witness address is `std::ptr::from_ref(snap)` at :652). `snapshot_close` (`ts/crate/src/lib.rs:735-738`) unconditionally takes the handle; `SnapWorker::drop` (:526-533) sends `SnapReq::Close` and **joins the thread**, so the Snapshot local is destructured (read.rs:41) and its stack deallocated. `SnapshotHandle` carries no link to `tx_open` or any open witnessed transaction — nothing refuses the sequence.
- **The SDK happens to avoid it; the native surface does not** — `ts/src/db.ts:1202-1219`: `commitWitnessed` calls `native.txCommit` (:1205) before `closeScopeState(state)` (:1215/:1219). But the raw native functions are the published surface and tests already call `native.snapshotClose` directly (e.g. `ts/test/bughunt.test.ts:505-519`); no test covers the close-before-commit ordering.
- **Spec check** — `write.rs:121-138` (doc for `write_from`, citing `docs/architecture/70-api.md` § conditional writes) declares the witness must be "evidence, never a raw integer a caller could fabricate or stale-cache (the recorded refusal)." The bridge ships the witness across a channel as exactly a raw integer address (`SnapReq::Witness` → `SnapReply::Witness(usize)`, lib.rs:500/510/652). The bridge violates the engine's own recorded doctrine at the representation level — this is the representation-first finding underneath the UB.

### Failure scenario

Raw FFI (or any future SDK refactor that frees the reader slot early to unpin LMDB pages — a plausible optimization, since `write_witnessed` itself drops the parked reader at write.rs:190-195 for exactly that reason):

```js
const r = native.dbWriteFrom(db, snap)   // tx worker enters write_from; Ready received
native.snapshotClose(snap)               // snapshot thread joins; Snapshot stack local deallocated
native.txInsert(r.tx, ...)               // tx worker still inside write_from, holding &Snapshot into freed stack
native.txCommit(r.tx)
```

Today: deterministic formal UB — a protected `&Snapshot` argument whose pointee is deallocated mid-call (Miri's "deallocation while item is strongly protected"), plus an LLVM `dereferenceable`-attributed argument pointing at freed memory. No wrong read occurs with the current engine because both witness reads complete before `Ready` is sent. After any engine change that touches `witness` after entry — a commit-time re-check being the obvious candidate — this becomes a real use-after-free reading a dead thread's stack, with no compile-time signal thanks to the `'static` erasure.

### Suggested fix

Reify the witness as data instead of a cross-thread borrow (the fix the engine's own "evidence, never a raw integer" contract points at): have `Snapshot` mint an opaque `Witness` value — env instance id + generation, constructible only by a `Snapshot`, so it stays non-fabricable by construction — and change the signature to `Db::write_from(witness: Witness, f)`. The engine already reduces the reference to exactly that pair at entry (write.rs:156, :162), so this is a pure representation change with no behavioral delta. The bridge then sends a plain value over the channel; `SnapReq::Witness`, the usize address, the `'static` transmute, and the entire unsafe block at lib.rs:929-944 are deleted. If the engine signature cannot change, at minimum confine the raw pointer to the snapshot worker (read env-instance + generation there and ship values), so no reference ever outlives the JS thread's blocked window.
