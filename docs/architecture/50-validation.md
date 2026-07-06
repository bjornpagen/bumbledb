# 50 — Validation

**Status ledger (2026-07-03).** In-repo today: the negative-validation corpus, the
deterministic property/golden content as unit tests, the randomized
executor-vs-nested-loop differential family, kill-during-commit crash injection, the
concurrent reader/writer families (incl. the pinned-at-T reads), the ETL family, the
allocation gate (allocations *and* deallocations), and the EXPLAIN family (cover
choice + batching engaged). Also in-repo now (`crates/bumbledb-bench`,
docs/architecture/50-validation.md): the SQLite oracle (`bumbledb-bench verify`), the IR→SQL
translator, and the ledger benchmark (`bumbledb-bench bench`). **The oracle was
built as 2-way agreement plus hand-written goldens, not the 3-way reference-engine
design below:** the translator's output is pinned byte-for-byte against
hand-written SQL for every family, and those goldens arbitrate engine-vs-SQLite
disagreements — the reference engine's tie-breaking role is filled by a human
reading the semantics docs against the golden, at the cost of a third independent
executor. The deviation stands until a disagreement the goldens cannot arbitrate
appears; then the reference engine gets built. Still external and unbuilt: the
reference engine, the versioned golden corpus artifact, and fuzz targets proper.
The benchmark is built; **the performance claim is pending a human L-scale ALL-WIN
run** — nothing here makes it. The first S-scale report (2026-07-03, FAIL:
fk_walk/balance/string lost) drove the perf work (landed the
same week; the PRD process is retired — git history has it): selection-level probes, the view-memo LRU, the finalize intern
memo, dense COLT iteration, magnitude-first covers, honest planner
cardinalities, fullfsync parity, and store compaction — each enforced by the
structural tripwires in `crates/bumbledb-bench/src/tripwires.rs`, never by
wall clock. The old report is stale evidence; the claim is re-earned by a
human re-run of `scripts/bench.sh`.

The old repo's best asset was its correctness discipline; the worst was its gate
theater. We keep the former and refuse the latter.

## The oracle

**SQLite is the external correctness oracle** — never infrastructure. Every benchmark
and golden query is executed against SQLite and bumbledb's result set must equal
SQLite's **exactly, by value**, before any timing claim.
**Decision.** **Alternative:** reference-engine-only validation. **Why it lost:** an
independent, battle-tested implementation catches whole bug classes a same-author
reference shares. **Reverses if:** never.

**Durability parity under `synchronous=FULL` (docs/architecture/50-validation.md).** Both engines
flush **to media** on the timing machine: LMDB does unconditionally on macOS
(`lmdb-master-sys` `mdb.c:171` — `MDB_FDATASYNC(fd)` is `fcntl(fd, F_FULLFSYNC)`
under `__APPLE__`), while SQLite's default `fullfsync=OFF` issues a plain
`fsync(2)` that macOS does not propagate through the drive cache (the bundled
amalgamation's `unixSync` issues `F_FULLFSYNC` only when the pragma is on). The
bench session therefore pins `PRAGMA fullfsync=ON` and
`PRAGMA checkpoint_fullfsync=ON`, and `FairnessCheck` asserts both — the first
benchmark run's 41× commit_single gap was this asymmetry, not engine work.

**The value mapping is normative** (the v5 oracle parsed CLI text with
`parse().unwrap_or(0)`, silently coercing everything — post-mortem-adjacent; never
again). Comparison uses the **typed rusqlite API**, never CLI text:

| bumbledb | SQLite | note |
|---|---|---|
| Bool | INTEGER 0/1 | |
| U64 | INTEGER | generator constrains oracle-checked data to `< 2^63`; full-range U64 is covered by non-oracle property tests (encode/decode, guards) |
| I64 | INTEGER | |
| Enum | INTEGER (ordinal) | Min/Max never apply (equality-only type) |
| String | TEXT | intern ids decoded to bytes **before** comparison, outside any timed region |
| Bytes | BLOB | never TEXT — DISTINCT distinguishes `X'41'` from `'A'` |

**Projection queries:** `SELECT DISTINCT` over the join with all find variables.
**Aggregate queries (normative template):** the aggregate applied over a
`SELECT DISTINCT <all bound query variables>` subquery — never a bare `GROUP BY` over
the joined bag (which folds witness multiplicity) and never `SUM(DISTINCT x)` (which
folds distinct values). `Count` = `COUNT(*)` over that subquery. **Empty-input global
aggregates:** bumbledb yields the empty set; SQLite yields one NULL/0 row; the harness
rule is that the oracle SQL wraps ungrouped aggregates to drop the empty-input row —
a documented translation rule, not an ad-hoc comparison patch.

**The IR→SQL translator is named infrastructure** with its own tests: hand-written SQL
goldens pin its output for known queries. Arbitration for 3-way disagreements
(engine vs reference vs SQLite): the hand-verified golden answers decide; a
disagreement on a non-golden query becomes a minimized golden before it is "fixed."

**Negative validation** has no oracle (SQLite accepts what we reject): a corpus of
invalid IR with pinned error kinds asserts the validation roster in `20-query-ir.md`.

## The primary benchmark: ledger

Owned here (00-product describes shape; this doc owns the schema):

```
Holder(id serial, name string)
Account(id serial, holder u64→Holder, currency enum)
Instrument(id serial, symbol string)
JournalEntry(id serial, source enum, created_at i64)
Posting(id serial, entry u64→JournalEntry, account u64→Account,
        instrument u64→Instrument, amount i64, at i64)
PostingTag(posting u64→Posting, tag enum)
Org(id serial, name string)
OrgParent(child u64→Org, parent u64→Org)
```

Families: unique-key point lookups; postings for a holder/account over a time range;
entries touching an account set (host-side union convention, documented); multi-hop
joins across holders/accounts/postings/instruments/entries; balance-style aggregates by
account and instrument (in the suite before any "beats SQLite" claim); a cyclic-ish
join for WCOJ honesty; a duplicate-witness projection. Data: seeded, reproducible,
generated at 10⁵–10⁷ facts.

**Protocol (success criterion 2 is measured exactly this way):** SQLite file-backed,
WAL, `synchronous=FULL`, **fully indexed per family** (the honest opponent), prepared
statements reused, `ANALYZE` run; `SELECT DISTINCT` (or the aggregate template)
included in the timed SQL — same semantics both sides; timed region = execution +
result materialization on both sides, decode excluded per the mapping table; warmup
then repeats; statistic = per-family **median**; **every family must win**; warm timing
gates, cold-after-commit reported alongside; canonical machine = the owner's. The suite
is an explicit versioned query list in-repo; **the claim is void until the aggregate
families are in it**. The "ratchet" is a manually re-run report per meaningful change —
not a CI gate. JOB and friends may be run for curiosity; they gate nothing.

## Differential and property tests

- A tiny **in-memory reference engine** (naive loops + BTreeSets, obviously correct)
  executes the same IR; randomized queries over randomized ledger-shaped data must
  agree three ways (engine, reference, SQLite).
- **The generator has a feature-coverage contract, itself asserted:** every IR
  construct provably generated — repeated in-atom variables, self-joins, zero-binding
  atoms, params re-bound across executions, empty relations, every comparison op on
  every legal type, point lookups, the cyclic query, aggregates of every op, and
  **duplicate-witness data that exercises the D2 subtree skip and the aggregate-sink
  binding dedup** (the two places a set-semantics bug would hide).
- Operation-sequence property tests for the write path: random insert/delete/alloc
  interleavings with constraint checks, asserting idempotence, guard consistency, and
  serial monotonicity across commits and aborts.
- Scalar/vectorized (batch-size 1 vs 2/64/256/partial/empty) equality on every fixture.
- **Crash and reopen:** kill-during-commit (LMDB atomicity actually exercised) and
  reopen-after-commit asserting F/M/U/R/Q/S mutual consistency and counter truth —
  the deferred-counter-flush design (`40-storage.md`) makes reopen the only test that
  can catch a never-persisted high-water.
- **Concurrent reader/writer families:** long-lived reader pinned at generation T
  across commits T+1..T+n (its images survive; results stay at T); two readers racing
  to build one image (single shared instance or benign duplicate — per 40's rule);
  rapid write/read interleaving (a reader never sees a mismatched generation — the
  snapshot-sourced tx-id rule under test).
- **ETL family:** bulk-load ≡ sequential-insert equivalence (full-relation set
  equality); explicit-serial/high-water property tests; chunk-boundary and mid-stream
  failure semantics (prior chunks committed, count carried on the error); full
  round-trip (export → fresh database → import → oracle-equal results). Append mode
  no longer exists to misuse (`40-storage.md` records the decision not to build the
  fast path); the misuse-rejection item is closed with it. ETL is the migration
  story; an ETL bug is a data-loss bug.
- **Encoding round-trip fuzzing is retained** (decision: the one fuzz target that
  earns its place — order-preserving encodings and composite guard keys are where a
  boundary bug corrupts sort order silently; i64::MIN, empty bytes, max-length values).
  Executor differential fuzzing is subsumed by the seeded generator above.

## Golden set

Hand-written queries with hand-verified expected results over a fixed dataset — the
anchor when the 3-way differential disagrees. Must cover: duplicate witnesses (the
set-semantics signature), exact projection sets, duplicate insert no-ops, absent delete
no-ops, constraint violations, aggregate folds with collapsing-vs-distinguished
bindings (`10#footgun`), and empty-input aggregates.

## The allocation gate

The one numeric gate: a counting allocator asserts the zero-warm-allocation contract
under the exact protocol defined in `30-execution.md` (single-threaded, N warmups over
a fixed param set, M measured runs, arena growth counted, caller-provided result
buffer). It is a boolean, not a budget file.

## EXPLAIN assertions

One small family: on constructed skew fixtures, EXPLAIN's counted execution asserts
the expected cover choice and that batching engaged — the cheap detector for
correct-but-slow regressions (v5's two flagship failures were exactly that class).
Beyond this, the benchmark's timing is knowingly the only performance detector; stated.

## What we deliberately do not have

Line-count gates. PRD-map checks. Banned-identifier greps. Coverage percentages.
Allocation budget *tables*. Failpoint matrices (the crash/reopen family above replaces
them with fewer, sharper tests). The gate surface is: `cargo fmt` / `clippy -D
warnings` / `cargo test`, the oracle, the differential suite, the allocation boolean,
and the EXPLAIN family. A gate earns its place by catching a real bug class.
