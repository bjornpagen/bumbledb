# Receipt audit 0–9 — pair A

Auditor: pair A (independent verifier). Method: read `proposals/settlement/90-traceability.md` rows 0–9 and `00-canon.md`; attempt to **refute** each row against the present tree. A row **passes** only when refutation fails and a landed type or invariant blocks it, with `file:line`.

| Row | Finding (abbrev) | Dissolving move | Verdict | Blocking invariant / refutation evidence |
| --- | --- | --- | --- | --- |
| 0 | Same-candidate publish race clobbers winner `prev` | `prev` inside content hash; document written once | **PASS** | `Checkpoint::digest()` hashes rendered bytes including `prev`; publication is create-only at the digest key — `crates/bumbledb-log/src/manifest.rs:297,361-366,441-444` |
| 1 | Checkpointer publishes forged checkpoint on applied pending | Compaction input is `Settled`; `Pending` cannot compact | **PASS** | `settled_view` / checkpointer match `Chain::Pending { .. } => None`; duty arms only on `Chain::Settled` — `crates/bumbledb-log/src/writer/duty.rs:49-54,66-72`; `crates/bumbledb-log/src/checkpointer.rs:136-138,219-220` |
| 2 | Rust `pid_alive` treats EPERM as dead | `Alive\|Dead\|Unknown`; fenced CAS lease; no `kill(0)` | **PASS** | `Liveness` sum; mutation lock breaks on `Lease::breakable(expiry)` only; no `pid_alive` / `kill(0)` in tree — `crates/bumbledb-log/src/store.rs:178-212`; `crates/bumbledb-log/src/store/fence.rs:238` |
| 3 | Lock-break is read/probe/unlink, not atomic | Lease broken only by expiry through store CAS | **PASS** | Acquire mints next token via exclusive `hard_link`; live lease ⇒ `LeaseBusy::Live`; no probe/unlink break path — `crates/bumbledb-log/src/store/fence.rs:186-216,238-252`; `ts-log/src/store.ts:162-170,321-399` |
| 4 | Rust breaks live lock on EPERM; TS honors it | One lease primitive, one liveness sum | **PASS** | Rust: expiry-only `breakable`; TS: `livenessOf` maps unreadable ⇒ `unknown`, `breakable(unknown) === false` — `crates/bumbledb-log/src/store.rs:200-212`; `ts-log/src/store.ts:61,151-170` |
| 5 | `refresh_braid` skips pending resolution, merges fork | `Chain` is a sum; no reader skips `Pending` | **PASS** | `Chain::Pending` arm; refresh stepper calls `resolve_pending` before catch-up; TS `stepBraid` matches pending slot — `crates/bumbledb-log/src/sidecar.rs:123-131`; `crates/bumbledb-log/src/replica.rs:582-584,801-805`; `ts-log/src/replica.ts:481-486` |
| 6 | Leading BOM decodes differently Rust vs TS | One string grammar, bytes-in/bytes-out | **PASS** | WHATWG `ignoreBOM: true` keeps U+FEFF; both drivers decode `EF BB BF 68 65 6C 6C 6F` to `"\uFEFFhello"` — `ts-log/src/bytes.ts:19`; `crates/bumbledb-log/src/codec.rs:62-65`; `ts-log/test/bytes.test.ts` |
| 7 | Stale writer resurrects retention-swept slot | Below-floor create refused | **PASS** | `below_floor` write-path precondition ⇒ `SlotRetired` before store touch — `crates/bumbledb-log/src/writer/discipline.rs:339-349,199-200`; `ts-log/src/writer.ts:495-498` |
| 8 | Unconditional `rm` lets two processes hold one key lock | Fencing token; two current holders unrepresentable | **PASS** | `Fenced` token on writes; `still_current()` / generation CAS; acquire uses exclusive link, not blind rm — `crates/bumbledb-log/src/store.rs:323-332,389-390`; `crates/bumbledb-log/src/store/fence.rs:293-301`; `crates/bumbledb-log/src/store/fs.rs:319-321`; `ts-log/src/store.ts:293-306` |
| 9 | `chain.json` pending: Rust hex vs TS base64 | One canonical encoding, byte-identical | **PASS** | Sidecar is binary `chain` (not `chain.json`); pending is length-delimited raw batch bytes in both drivers — `crates/bumbledb-log/src/sidecar.rs:10-14,218-224`; `ts-log/src/chain.ts:6-8,108-115`; `crates/bumbledb-log/tests/conformance_v3.rs:260-265` |

## Roll-up

| Verdict | Rows |
| --- | --- |
| PASS | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9 |
| REFUTE | — |
