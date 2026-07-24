## Tx.insert discards the engine's changed-state boolean that already crosses the FFI

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: ts:core
outcome: fixed 58e63af9 (R11)

### Summary

The engine computes a changed-state boolean on every insert and the N-API bridge returns it on every call, but the TS SDK's `Tx.insert` wraps `native.txInsert` in a void thunk and drops the bit, returning only the minted fresh cells. `Tx.delete` returns the same bit from its twin call. The Rust surface pins `insert(&fact) -> bool` (changed-state report), and the SDK's own bridge documentation states the surface "stays bijective with the Rust surface." No recorded ruling in the API doc's decision ledger blesses the asymmetry. A TS host doing idempotent replay or change counting must issue a separate `txContains` FFI round trip before each insert to learn what the insert call already carried back.

### Evidence (all verified against the working tree)

- `ts/src/db.ts:1095-1097` — the drop site: `bridged("bumbledb tx insert", function record() { native.txInsert(txHandle, entry.id, row) })`; the thunk returns void and `insert` returns only the frozen `fresh` record.
- `ts/src/db.ts:1109-1111` — the contrast: `remove` returns `bridged("bumbledb tx delete", ...)` directly, surfacing the boolean.
- `ts/src/db.ts:216-218` — the `Tx` interface: `insert(...): Minted<R>` (no changed bit anywhere in its doc comment); `delete(...): boolean` documented "`true` iff the final state changed."
- `ts/src/native.ts:427` — `txInsert(tx, relationId, values): boolean`, documented "Records an insert into the delta; `true` iff the final state changed."
- `ts/crate/src/lib.rs:1103` — `pub fn tx_insert(tx: &External<TxHandle>, relation: u32, values: Array) -> napi::Result<bool>`: the bit crosses the FFI on every insert today; exposing it costs nothing new.
- `ts/src/marshal.ts:51` — `type Minted<R> = { [K in FreshKeys<R>]: Fact<R>[K] }`: for a relation with no fresh fields, insert returns a frozen empty object, so the changed bit is entirely unrecoverable from the return value.
- `docs/architecture/70-api.md:498` — the frozen Rust surface: "`insert(&fact) -> bool` (changed-state report); `delete(&fact) -> bool`."
- `docs/architecture/70-api.md:504` — the point-reads decision calls `tx.contains` "the `insert`/`delete` return value's read-only sibling" — the spec treats both return values as part of the surface contract, and the TS SDK keeps only one of them.
- `ts/src/native.ts:353-355` — the bridge's stated principle: "the SDK surface stays bijective with the Rust surface." I searched 70-api.md (including the drizzle-law section, the vars-are-values ruling, and the freeze/OPEN-ledger census) and the TS package docs for any ruling that deliberately trades the bool for `Minted`; none exists.

### Failure scenario / cost

Not a wrong-output bug — a parity and FFI-efficiency hole. Note the precise scope: when insert mints an omitted fresh field, the state change is guaranteed (fresh ids are never reissued), so the lost bit matters exactly when the host supplies the full fact — the resupply/idempotent-replay lane. A host replaying a fact log for at-least-once delivery, or counting real state changes during a reconciliation write, must call `tx.contains(relation, fact)` before every `tx.insert` — one extra FFI crossing per fact — to recover a boolean the subsequent `txInsert` call returns and the SDK throws away.

### Suggested fix

Expose the bit the bridge already returns; no engine or crate change needed. Options, in order of surface conservatism: (a) extend the returned object — `insert` returns `{ changed: boolean, ...fresh }` or `{ changed, fresh }`; (b) keep `Minted<R>` and add a parallel spelling that returns the bool for the full-fact lane. Either restores the Rust-surface bijection that `delete` already honors. Since the return value is currently a frozen record whose properties hosts destructure (the census at 70-api.md:1057-1067 records id-map building from insert returns), option (b) or the non-colliding `{ changed, fresh }` shape avoids a fresh-field name collision with a hypothetical `changed` column.
