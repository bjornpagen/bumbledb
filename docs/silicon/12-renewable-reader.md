# PRD 12 — Renewable reader: the last 0.2 µs of the point path

## Purpose

perf-PRD 11 took the point path to 1.0 µs against a 0.8 gate and named
the residual precisely: the `db.read` wrapper's per-execute LMDB read-txn
begin dominates what remains (guard probe itself is 0.625 µs; finalize is
gone; the prologue is lean). LMDB's own design provides the fix:
`mdb_txn_reset`/`mdb_txn_renew` keep a reader slot warm so a "new"
read transaction is a slot renewal, not a full begin. Point reads are the
regime where an embedded system-of-record lives or dies in app code, and
p1/p2 are the only two scenario queries still losing to SQLite (1.04×,
1.16×) — both by exactly this fixed cost. Every execute of every family
pays the same toll, so the win is broad and small everywhere, large where
it matters.

## Technical direction

`crates/bumbledb/src/storage/env.rs` (ReadTxn, Db read entry).

- **Read the current shape first.** Establish what `db.read` does today:
  full `mdb_txn_begin(RDONLY)` per call, or already reset/renew? The
  perf-PRD 11 result says the cost profile matches a full begin (or a
  renew that still re-validates too much). Commit the before/after split
  in `## Result` (traced prologue, the perf-PRD-11 methodology).
- **Cached reader slot.** The `Db` (or its storage handle) owns one
  cached read transaction: on `read()` entry, `mdb_txn_renew` it; on
  exit, `mdb_txn_reset`. Concurrency discipline follows the existing
  ownership model — read the current `Db` threading contract and match
  it: if `Db` is single-threaded by construction (`!Sync`), one slot
  suffices; if reads may be concurrent, the slot is per-handle or guarded
  by the existing synchronization (do NOT add a new lock on the read
  path; if the contract requires one, the slot is per-thread via the
  handle the caller already holds).
- **Snapshot semantics are load-bearing.** `mdb_txn_renew` observes the
  latest committed snapshot — same visibility as a fresh begin. The
  engine's snapshot check (the u64 compare from hardening-00) stays
  exactly as is. Write tests that prove: (a) a write committed between
  two reads is visible to the second (renewed) read; (b) a prepared
  query's snapshot-mismatch error still fires identically across renew;
  (c) error paths (typed param errors, misses) leave the slot renewable
  (no poisoned-slot state — reset must run on every exit path, including
  errors; RAII guard, not manual calls).
- **Reader-table hygiene.** A reset-but-never-freed slot pins the reader
  table entry, not the snapshot — that is LMDB's intended usage. Test:
  10,000 executes do not grow the reader table (assert via
  `mdb_env_info`/reader list if exposed, else via stable map-size
  behavior under interleaved writes — pick the strongest observable the
  bindings expose and document it).

## Passing requirements

1. Measured (vs post-11, min-of-5): point p50 ≤ 0.8 µs (the inherited
   perf-PRD-11 gate, closed at last; baseline 1.0); string p50 ≤ 1.4 µs
   (baseline 1.6).
2. Broad transfer: every read family's p50 improves or holds (each pays
   one begin today); commit_batch and write paths unchanged ±2% (writers
   are untouched).
3. The traced point prologue split in `## Result` shows the txn-begin
   share reduced ≥ 50%.
4. Snapshot-semantics tests (a)–(c) green; reader-table hygiene test
   green; verify green (its interleaved read/write cases now exercise
   renew); zero-alloc holds.
5. Scenario-suite transfer (p1/p2 < 1.0×) noted as expected in
   `## Result` for the next human scenario run — not gated here (suite
   runs are human-owned).

## Out of scope

Write transactions (unchanged); multi-reader pooling beyond the ownership
model the code already has; any LMDB flag changes (NOMETASYNC etc. —
durability posture is settled elsewhere).
