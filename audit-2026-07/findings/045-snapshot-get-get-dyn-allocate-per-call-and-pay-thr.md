## Snapshot point reads allocate per call — the flagship keyed-get lane (p5) is the only bench lane not beating SQLite

category: lean-rust-drift | severity: medium | verdict: CONFIRMED | finder: perf:rings

### Summary

`Snapshot::get_dyn` and `Snapshot::get` violate the repo's own point-read doctrine. The WriteTx point-read sibling states the law explicitly and implements it — `crates/bumbledb/src/api/db/get.rs:226-233`: "the determinant must not allocate per call (point reads are allocation-free, `docs/architecture/70-api.md`)", using `std::mem::take(&mut self.scratch)` / restore. The doctrine is anchored at `docs/architecture/70-api.md:537` ("point reads are determinant gets (allocation-free, no images, no plans)"). The Snapshot siblings ignore it: a fresh `Vec::new()` determinant per call, and (in the dyn lane) a fully owned decode allocating several boxes per read. The pinned night bench shows the consequence: `p5_keyed_get` — the 0.5.0 flagship "no query machinery" point read — is the only lane in the scenarios run at SQLite parity, and it is slower than the same probe pushed through the full prepare/plan/COLT machinery.

### Evidence (all verified against the working tree)

- `crates/bumbledb/src/api/db/snapshot.rs:179` — `let mut determinant = Vec::new();` per `get_dyn` call; the typed `get` twin does the same at `snapshot.rs:235`.
- `crates/bumbledb/src/api/db/snapshot.rs:200-205` — the hit decodes through `decode_values(fact, rel.layout(), |id| Ok(Box::from(dict::resolve(&self.txn, id)?)))`: `crates/bumbledb/src/encoding/decode.rs:235-260` collects an owned `Vec<Value>`, allocates a `Box<[u8]>` per string field, and (line 248) another Box per `bytes<N>` field (`Value::FixedBytes(value.as_bytes().into())`). For the bench's `Doc` relation (`key: str`, `payload: bytes<32>`) that is 4 mallocs+frees per hit: determinant Vec, `Vec<Value>`, string Box, payload Box.
- LMDB descents per hit: `dict::lookup_str` (`storage/dict.rs:97-108`), `read::determinant_row` U-get (`storage/read/determinant_row.rs:24`), `read::fetch` F-get (`storage/read/fetch.rs:27-30`) — plus a **fourth** the finder undercounted: `dict::resolve` to materialize the returned key string during decode. SQLite's twin answers the same full-row point SELECT with an index seek + row fetch, strings inline.
- The timed loop: `crates/bumbledb-bench/src/scenarios/run_query.rs:228-235` calls `snap.get_dyn(...)` per sample — every one of those allocations and descents is inside the timing window.
- The pinned numbers (`bench-out/night-2026-07-20/scenarios/scenarios.md:44,47`): `p5_keyed_get` 1.4us vs SQLite 1.4us, ratio **1.00**; `p2_by_key` (query machinery, same string-keyed probe) 0.9us, ratio 0.67; `p1_by_id` ratio 0.50. p5's SQLite lane is the canonical full-column point SELECT (`crates/bumbledb-bench/src/translate.rs:115-151`), so parity is apples-to-apples.
- The pooled twin already exists in the engine: `crates/bumbledb/src/exec/dispatch/execute_key_probe.rs:26-36` takes `key_scratch: &mut Vec<u8>` and reuses `Bindings`; the prepared direct lane pools `self.determinant_key` (`crates/bumbledb/src/api/prepared/execute.rs:333-339`) and its `FixedBytes` arm comments "no temporary heap ... this is the point fast lane" (`execute.rs:359-375`). The zero-alloc probe is written once and correctly — the get surface just doesn't use it.
- Doc check per audit protocol: `docs/architecture/70-api.md:537` is the governing contract; the Snapshot code diverges from it. Not a Free Join / COLT subsystem, so the paper is not implicated.

### One correction to the finder's framing

`p2_by_key` is not literally "the SAME point read": its query projects only `(id, size)` — two fixed-width columns, no output string resolution, no payload copy (`crates/bumbledb-bench/src/scenarios/points.rs:134-148`), while `get_dyn` returns the full 5-field fact. Part of the 0.5us p2→p5 gap is therefore p5's extra output work (the fourth descent + two extra Boxes), not solely the missing scratch discipline. This does not weaken the finding — that extra work IS the per-call allocation overhead at issue, and the ratio-1.00 parity against SQLite's full-row SELECT stands on its own — but "removing the allocations closes the whole gap to p2" is not guaranteed; the dict::resolve descent for the returned string is structural to the owned-`Value` dyn contract.

### Bench impact

`p5_keyed_get` is the lane the 0.5.0 release notes call the flagship, and it is the only scenarios lane not beating SQLite. Per read it pays 4 heap allocations + frees and 4 LMDB descents where the engine's own prepared point lane pays zero allocations, and where the WriteTx sibling proves the scratch discipline is already idiomatic in this codebase. Removing the per-call Vecs and decoding into a reusable out-buffer removes the malloc traffic from a ~1.4us loop; the typed `get` lane (which already decodes to borrowed views — `snapshot.rs:218-221` — and only wastes the determinant Vec) gets the cheapest fix.

### Suggested fix

1. Determinant scratch: `Snapshot` reads are `&self`, so mirror the existing Snapshot precedent of caller-owned mutable state (`Snapshot::execute(&self, ..., out: &mut Answers)`) — either an explicit `get_dyn_into`-style scratch/out parameter or a small pool on the `Db`/read-scope object. The WriteTx take/restore trick is the `&mut self` variant of the same discipline.
2. Output buffer: give `get_dyn` an `Answers`-style reusable out parameter instead of returning fresh `Option<Vec<Value>>` per call (the bench and the TS bridge are loop callers).
3. Unification: `WriteTx::fact_by_determinant`'s committed leg (`get.rs:334-337`) and `Snapshot::get`'s probe (`snapshot.rs:243-246`) and `key_probe_fact` are the same U→F probe with three allocation contracts; routing the get surfaces through the pooled probe deletes the duplicates — representation over reimplementation.
