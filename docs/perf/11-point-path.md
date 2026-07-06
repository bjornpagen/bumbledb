# PRD 11 — Point path: the last microsecond

## Purpose

The point family: execute self-time 1.459 µs against 0.625 µs of actual
guard probe — the fixed prologue (snapshot check, buffer clear, sink reset,
bind, span construction, result push) costs more than the lookup. These are
the only two scenario-suite queries that lose to SQLite (p1 1.04×,
p2 1.19×), and the loss is entirely this fixed overhead. Point reads are
also the regime where an embedded system-of-record lives or dies in app
code. Target: point p50 under the SQLite line with margin.

## Technical direction

`api/prepared.rs` (`execute`, guard path), `api/db.rs` (`read` wrapper).
Profile-first discipline: before changing anything, add temporary
fine-grained spans (or cycle counters in a scratch build) across the
prologue segments of a point execute — snapshot check, `out.clear`,
`sink.reset`, `bind_params`, guard key encode, LMDB get, decode+push —
and commit the split into `## Result`. Then attack the measured order,
candidates:

- **GuardProbe fast lane.** The guard plan needs none of the join
  machinery: no sink (`ProjectionSink` reset/iteration for ≤ 1 row), no
  `Bindings`, no finalize pass over a sink. Restructure the GuardProbe arm
  of `execute` to: bind params → encode guard key (reuse the prepared
  `guard_key` buffer, already there) → one LMDB `get` → decode the hit
  row's find columns straight into `ResultBuffer` (through the PRD 08
  cell writers; the intern memo only when a find is interned). The
  `EitherSink` enum stays for join plans; guard plans stop carrying sink
  state entirely (type-level: make the sink `Option`al or split the
  prepared-plan enum's payload — end state, no shim).
- **Prologue costs.**
  - `out.clear()` on a fresh/reused buffer: ensure it is O(arity), not
    O(previous rows) in any hidden way (it should already be; verify).
  - `check_snapshot`: one u64 compare + branch — keep, it is load-bearing
    (hardening 00); make sure it is not behind a call that recomputes
    anything.
  - `bind_params`: for all-word params (no interning), the loop should be
    a copy + type check; confirm no dictionary txn work happens on the
    word-only path.
  - Span construction (`obs::span`) is compile-time empty in production
    builds — confirm the bench binary used for gates measures the
    default-features engine through the timing path (it does — obs builds
    gate on `capturing()`; nothing to change, just do not "optimize" it).
  - `db.read` closure overhead: one LMDB read-txn begin/reset per
    execute. LMDB reader-slot reuse (`mdb_txn_reset`/`renew` semantics)
    already backs `ReadTxn` — verify the per-read cost is slot renew, not
    full begin; if a full begin shows in the split, route repeated
    `db.read` through the renewable reader (storage/env.rs owns this;
    typed invariants unchanged).
- **Decode-and-push**: a point row decode touches `layout()` per field —
  precompute the guard plan's find-column offsets/types at prepare into a
  flat array so the hit path is offset loads, no per-field layout walk.
- **Do not** add caches keyed on parameter values, speculative results,
  or any auxiliary structure — this PRD is strictly fixed-cost removal.

## Passing requirements

1. Functional gates green (guard-path behavior covered by verify's point
   cases and the empty-store pass; add a unit test for miss + hit + typed
   param-mismatch errors through the fast lane).
2. Measured: point p50 ≤ 0.8 µs (baseline 1.1; SQLite 1.4); string p50
   ≤ 1.4 µs (baseline 1.8 — same prologue, plus one dict resolve); the
   prologue split in `## Result` shows the remaining fixed cost itemized.
3. No other family regresses (>5%) — the execute-arm restructure touches
   the shared entry; the join arm's prologue must not grow.

## Out of scope

Scenario-suite point queries as gates (they are not in the ledger bench;
note expected transfer: p1/p2 ratios flip < 1.0 at the next scenario run,
recorded when a human runs it), LMDB internals, guard semantics.
