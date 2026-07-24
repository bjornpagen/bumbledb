## Answer/scan rows cross the FFI with two avoidable copies per string/bytes cell

category: perf | severity: medium | verdict: CONFIRMED | finder: ts:bridge
outcome: fixed b3e355eb

### Summary

The TS bridge's outward row lanes (prepared execute, `snapshotScan`/`exhumeScan`, `snapshotGet`/`txGet`) clone string and bytes payloads they already own. The conversion `ValueOut::from_value(&Value)` borrows, so owned `Box<[u8]>` payloads are re-allocated and memcpy'd into fresh `String`/`Vec<u8>` buffers, then dropped. On the execute lane this stacks on a second avoidable copy: the engine hands back `Answers` — a flat, fully owned carrier whose byte heap the engine documents as "the single sanctioned allocation site of a warm execution" — and the worker explodes it into per-row `Vec<Value>` with per-cell boxed-slice copies before `rows_out` copies each payload again on the main thread. Net: an executed string cell is copied engine-buffer → `Value` → `ValueOut` → JS, two heap allocations + memcpys (plus one `Vec` per row) more than needed. The code's own doc claims the opposite ("the one-copy crossing"), and the engine side records a zero-allocation crossing discipline that the FFI layer breaks at its own boundary. This is the Rust-for-allocation-control philosophy violated at the exact hot boundary the crate exists for.

### Evidence (all verified in the working tree)

- `ts/crate/src/marshal.rs:943-949` — `pub(crate) fn from_value(value: &Value)` borrows; `Value::String(bytes) => Self::Text(String::from_utf8_lossy(bytes).into_owned())` (allocation + UTF-8 rescan + memcpy on the always-valid payload) and `Value::FixedBytes(bytes) => Self::Bytes(bytes.to_vec())` (allocation + memcpy).
- `ts/crate/src/marshal.rs:998-1002` — `rows_out(rows: Vec<Vec<Value>>)` receives OWNED rows but converts via `row.iter().map(ValueOut::from_value)`, cloning every payload and dropping the originals.
- `ts/crate/src/lib.rs:794` and `ts/crate/src/lib.rs:1141` — `snapshot_get`/`tx_get`: `found.map(|values| values.iter().map(ValueOut::from_value).collect())` on an owned `Vec<Value>`; same avoidable clone.
- `ts/crate/src/lib.rs:544-546` — the doc on `answers_rows` claims "the one-copy crossing: decoded cells to owned values on the worker, natural JS values on the main thread" — contradicted by the second copy in `rows_out` above.
- `ts/crate/src/lib.rs:547-567` — `answers_rows` runs on the worker: per string cell `Value::String(v.as_bytes().to_vec().into_boxed_slice())`, per bytes cell `v.to_vec().into_boxed_slice()`, one `Vec<Value>` per row. This is itself a full intermediate copy of the flat `Answers` carrier.
- `crates/bumbledb/src/api/prepared.rs:107-127` — `Answers { arity: usize, cells: Vec<Cell>, bytes: Vec<u8> }` with `Cell: Copy`; fully owned and Send, so it could cross the mpsc reply channel whole. Its doc: "The byte heap is the single sanctioned allocation site of a warm execution." The per-finalize `ResolveMemo` (prepared.rs:129-145) exists precisely so K answers sharing one string cost one byte copy — economy `answers_rows` then discards by copying per cell.
- `crates/bumbledb/src/api/prepared/execute.rs:415-424` — `execute_collect_args` returns a fresh owned `Answers` per call (`Answers::new()` + `execute_args` into it), so nothing couples the buffer to the worker; moving it across the channel is safe.
- `ts/crate/src/lib.rs:465-474, 666-676, 743-753, 1290-1293` — the scan and execute reply lanes all funnel `Vec<Vec<Value>>` through `rows_out`, so the scan/exhume ETL export pays the `rows_out` clone too.
- `docs/architecture/40-execution.md` (§ the alloc gate, line ~1030) — the engine pins "zero allocation across the crossing" for the parameter latch; the outward crossing has no analogous discipline and violates the same stance.
- Correction to the finder's caveat: the manifest/violation call sites (`ts/crate/src/marshal.rs:1080`, `:1172`) also own their values — both loops destructure by value (`for (name, value) in row.values` / `in fields`) — so a borrowing twin is not needed anywhere; a single consuming `from_value(Value)` covers every call site.

### Bench impact

For N answer rows × K string/bytes cells: eliminates N·K redundant heap allocations + memcpys on every lane through `rows_out`, and on the prepared-execute lane additionally N row-`Vec` allocations plus N·K boxed-slice allocations (the `answers_rows` explosion), by moving owned payloads and/or shipping the flat `Answers` carrier whole. Lanes affected: prepared execute returning str/bytes columns (the primary query read), `snapshotScan`/`exhumeScan` full-relation exports (the documented ETL/derivation read), and `snapshotGet`/`txGet` point lookups. Fixed-width columns (bool/u64/i64/interval) are unaffected — the waste is exactly on the variable-width payloads the engine's memo machinery works hardest to copy only once.

### Suggested fix

1. Make the conversion consuming: `ValueOut::from_value(value: Value)` with `Value::String(bytes) => Text(String::from_utf8(bytes.into_vec()).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))` (payloads are engine-validated UTF-8, so the happy path is a zero-copy move) and `Value::FixedBytes(bytes) => Bytes(bytes.into_vec())`. Change `rows_out` to `row.into_iter()`, `snapshot_get`/`tx_get` to `values.into_iter()`, and the manifest/violation loops to pass `value` by value — every call site already owns its `Value`, so no borrowing twin is needed.
2. Second step (execute lane): add a `SnapReply::Answers(Answers)` reply variant and send the carrier whole across the channel (it is `'static`-owned and Send), decoding cells straight to `ValueOut`/JS on the main thread. This erases the intermediate `Vec<Vec<Value>>` representation entirely — the representation-over-control-flow fix: the engine already built the right carrier; the bridge should stop rebuilding a worse one.
