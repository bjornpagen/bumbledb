# Node write-begin can permanently stick tx_open if thread::spawn panics
- id: 106
- severity: medium
- confidence: confirmed
- area: ffi
- components: ts/crate/src/lib.rs
- status: open (do not fix)

## Summary
`spawn_tx` `swap`s `tx_open` to true, then `thread::spawn`s the writer. `std::thread::spawn` panics if the OS cannot create a thread. There is no `TxWorker` yet, so `Drop`/`finish` never clears the flag. Every later `dbWriteBegin` / `dbWriteFrom` on that handle fails with “a write transaction is already open” until process exit.

## Evidence
- `if inner.tx_open.swap(true, AcqRel) { return Err(already open) }` then channels, then `thread::spawn(...)`, then `Ok(TxWorker { … thread: Some(thread), tx_open: Arc::clone(...) })` (`ts/crate/src/lib.rs` `spawn_tx`).
- `TxWorker::finish` / `Drop` are the only `tx_open.store(false)` sites besides successful begin-failure paths that already constructed a `TxWorker`.
- `thread::spawn` documents panic on OS failure (use `Builder::spawn` for `io::Result`).

## Why this is a bug
A transient resource failure (thread limit) latches the single-writer guard forever. The handle is not recoverable without dropping the whole `Db`. The comment on `tx_open` is that a second begin would deadlock — here there is no writer at all.

## How to trigger / repro sketch
Hard to hit in CI. Sketch: lower the process thread cap (or mock spawn) and call `dbWriteBegin`. Catch the panic if the napi layer converts it; then `dbWriteBegin` again — typed “already open” with no live tx. `ulimit`-style thread exhaustion on a machine already near `nproc` is the realistic path.

## Spec / docs notes
None in Lean. Bridge module doc: one write tx per Db handle; this path leaves the guard set with zero transactions.

## Related
- 109 (`take_handle` panic vs typed close)

## Verification (2026-08-12)

**Verdict:** confirmed. Severity unchanged (medium).

**Trace:** `spawn_tx` `swap`s `tx_open` to true (`ts/crate/src/lib.rs:1046-1052`), then creates channels, then `std::thread::spawn` (`:1057`), then constructs `TxWorker` (`:1058-1064`). `TxWorker::finish` / `Drop` are the only `tx_open.store(false)` sites (`:926-927, 934-942`). `begin_outcome` calls `finish()` on moved/failed/died replies (`:1089-1099`), but only after a `TxWorker` exists. `thread::spawn` panics if the OS cannot create a thread (`Builder::spawn` is the `io::Result` spelling). Snapshot open (`db_snapshot`, `:780`) uses spawn without a latch; this finding is the write-guard latch only.

**Why it holds:** A transient thread-create failure leaves the single-writer flag true with no live writer. Later `dbWriteBegin` / `dbWriteFrom` hit the “already open” error until the `Db` handle is dropped. The comment on `tx_open` is that a second begin would deadlock — here there is no writer at all.
