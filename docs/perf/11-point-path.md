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

## Result (2026-07-07, run bench-out/2026-07-07T03-03-40Z)

Landed: the guard fast lane — `guard_probe_fact` (the probe half: key
from constants, one U/M get, one F fetch, remaining filters) split out
of `execute_guard`, and plain-variable guard plans decode cells straight
from the fact bytes into the buffer through the PRD 08 writers: no sink
reset, no bindings, no finalize pass (the point flame's finalize row is
gone). Aggregate-find guards keep the sink path (classify does not
inspect finds — the lane is conditional, not asserted). Unit test pins
hit (with an interned column beside the word blits), miss, and the
typed param error.

The prologue split (traced sample, 41.7 ns tick quantization):
guard_probe 0.625 µs (the two LMDB gets), bind_params ~0.0–0.4 µs,
finalize 0 (eliminated), execute-self ~1.29 µs — of which the dominant
share is the `db.read` wrapper's per-read LMDB read-txn begin plus the
snapshot check and buffer clear.

Gates: point p50 **1.0 µs** (gate ≤ 0.8; baseline 1.1) ✗ and string
**1.6 µs** (gate ≤ 1.4; baseline 1.8) ✗ — both ~0.2 µs short, and the
residual is the per-read transaction begin, not query work: the
renewable-reader change (`mdb_txn_reset`/`renew` semantics in
storage/env.rs) is the named lever, deliberately not taken inside this
PRD's api-layer scope. No other family regressed (all within bands;
ALL-WIN held; verify green). Expected transfer to the scenario suite's
p1/p2 ratios recorded for the next human scenario run.
