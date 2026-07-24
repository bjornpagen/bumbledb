## Snapshot point reads allocate per call while their WriteTx twins are pooled under an explicit allocation-free contract

category: perf | severity: medium | verdict: CONFIRMED | finder: engine:schema-api

### Summary

Every committed-state point read on `Snapshot` allocates fresh heap buffers per call, while the `WriteTx` siblings of the exact same operations run through pooled scratch under an explicit in-code contract ("point reads are allocation-free, docs/architecture/70-api.md"). The engine demonstrably treats the snapshot point path as hot — the parked-reader cache exists solely to shave `mdb_txn_begin`, documented as "the point path's last fixed cost" — yet leaves one malloc+free (two for `contains_dyn`) in that same path. The representation that erases the allocation already exists in-tree and is even used by the closed-relation arm of these very reads.

### Evidence (all verified against the working tree)

Allocation sites on the snapshot read path:
- `crates/bumbledb/src/api/db/snapshot.rs:235` — `Snapshot::get`: `let mut determinant = Vec::new();` then `key.determinant_read(self, &mut determinant)` (first extend triggers the heap allocation; freed at scope exit — one malloc/free round-trip per probe).
- `crates/bumbledb/src/api/db/snapshot.rs:179` — `Snapshot::get_dyn`: same fresh `Vec::new()` per call.
- `crates/bumbledb/src/api/db/snapshot.rs:145,151` — `Snapshot::contains_dyn`: `let mut refs = Vec::with_capacity(values.len());` (allocates immediately) plus `let mut fact = Vec::new();` filled by `encode_fact` — two allocations per membership probe. (The original finding said three; two is the verifiable count.)

The write-side twins, pooled with an explicit contract:
- `crates/bumbledb/src/api/db/get.rs:224-233` — `WriteTx::get` hand-rolls the scratch discipline: "the determinant must not allocate per call (point reads are allocation-free, `docs/architecture/70-api.md`) — so encode into the taken buffer, restore it", via `std::mem::take(&mut self.scratch)`.
- `crates/bumbledb/src/api/db.rs:419-422` — `WriteTx` carries `scratch: Vec<u8>` and `refs: Vec<ValueRef>` fields for precisely this reuse; `WriteTx::get_dyn` and `contains_dyn` route through `with_scratch`/`encode_dyn` (get.rs:270, get.rs:303).
- `docs/architecture/70-api.md:537` states the doctrine: "point reads are determinant gets (allocation-free, no images, no plans)".

The path is hot by the engine's own posture:
- `crates/bumbledb/src/api/db.rs:293-311` — the parked-reader cache exists so that "the per-read `mdb_txn_begin` (the point path's last fixed cost) is skipped entirely". An engine that engineered away a txn-begin on this path is paying an allocator round-trip on the same path.
- `docs/architecture/70-api.md:513-514` names `db.read(|snap| snap.get(key))` as THE Rust read scope for keyed reads — the loop shape of any read-heavy keyed workload.

The erasing representation already exists in-tree:
- `crates/bumbledb/src/storage/keys.rs:30` — `pub type KeyBuf = [u8; MAX_KEY]` ("Fixed scratch buffer for key writers"); `keys.rs:36-61` — `DeterminantImage` with `DETERMINANT_INLINE = 24` ("the widest common determinant shape stays off the heap"); `keys.rs:205` — `MAX_DETERMINANT_WIDTH = 496` bounds every determinant at declaration, so a stack buffer always suffices.
- `crates/bumbledb/src/api/db/get.rs:116` — `closed_fact_by_determinant`, called from the closed-relation arm of `Snapshot::get`/`get_dyn` themselves (snapshot.rs:192,241), already uses `DeterminantImage::scratch_with_capacity`. The ordinary arm of the same functions heap-allocates.

The asymmetry is test-pinned on one side only:
- `crates/bumbledb/tests/alloc_gate.rs:1108-1149` — `borrowed_struct_gate` asserts `tx.insert` + `tx.get` produce exactly `(0, 0, 0, 0)` allocation events. No equivalent gate exists for `snap.get` anywhere in the alloc gate or census suites — because it would fail today.

### Bench impact

Any read-heavy keyed workload in the documented Rust read shape — `db.read(|snap| { for id in ids { snap.get(id)?; } ... })` — pays one allocator round-trip per probe for a determinant that is 8-24 bytes in the common case (`DETERMINANT_INLINE` was sized to exactly this observation); `contains_dyn` pays two. The write-side twin of the identical operation pays zero, gate-verified. With the parked reader already erasing `mdb_txn_begin`, the malloc/free pair is now among the largest fixed costs left on the committed-state point-get lane. The typed `Snapshot::get` is the sharpest instance: its determinant `Vec` is plausibly the path's ONLY allocation (the WriteTx gate proves encode+probe+fetch+decode allocation-free for the shared machinery, and the typed decode returns a borrowed view). The dyn lanes allocate for their owned `Vec<Value>` output regardless, but their determinant/refs scratch allocations are still avoidable.

This is also a representation-first finding, not just a perf one (`docs/design/representation-first.md`): the inline-or-spill buffer is the representation that erases the allocation, it exists, and half of the same function already uses it.

### Suggested fix

Give the read-path determinant an inline-first representation. The one obstacle is the `Key` trait signature at `crates/bumbledb/src/api/db.rs:239-243`: `fn determinant_read(&self, snap: &Snapshot<'_, Self::Schema>, out: &mut Vec<u8>)` (and `determinant_write` at db.rs:251) hard-code `&mut Vec<u8>` as the output. Options, cheapest first:

1. Stack buffer behind the same signature's replacement: change `out` to a `DeterminantImage` (or a thin writer over `KeyBuf`) — `MAX_DETERMINANT_WIDTH = 496 < 511` guarantees a fixed stack buffer always fits, and `DeterminantImage`'s spill arm keeps the wide case correct. This fixes `get`, `get_dyn` (thread the same type through `encode_determinant_with`, which only needs `extend_from_slice`), and `contains_dyn`'s fact buffer in one move, and makes the typed and dyn lanes symmetric with the closed-relation arm that already does this.
2. Alternatively, interior scratch on `Snapshot` (it is single-closure-scoped), though `Snapshot::get` takes `&self`, so this costs a `RefCell`/`UnsafeCell` — the stack representation is cleaner and matches the in-tree precedent.

Then pin it: add the snapshot twin of `borrowed_struct_gate` (`db.read(|snap| snap.get(item))` at zero allocation events) so the read/write symmetry becomes a gated invariant rather than an aspiration.
