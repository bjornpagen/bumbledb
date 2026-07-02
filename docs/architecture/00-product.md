# 00 — Product

## What this is

Bumbledb is an embedded, typed, schemaful, **set-semantic** relational database over
LMDB, built by and for one user (Bjorn Pagen) and his applications. It is Postgres's
relational elegance with the parts its owner hates removed: no SQL, no bag semantics, no
nulls, no layer cake. BCNF-normalized typed relations with no way out of it.

The bet: take one good algorithm (Free Join), one elegant relational core (typed sets),
one storage engine (LMDB), and push those decisions to their logical extreme — measuring
how much performance falls out of refusing to generalize.

## Design philosophy

**Representation over control flow.** The biggest lever is the shape of the data, not
the cleverness of the code (Brooks → Pike → Raymond → Torvalds). When a case shows up
that wants a branch, a flag, or a mode, the first question is what representation would
make the case inexpressible. Illegal states unrepresentable; parse, don't validate;
minimal branching and special-casing everywhere — in the engine, the IR, and the schema.

**Rust, specifically for allocation control.** Memory allocation churn was an Achilles
heel of the five discarded implementations. Rust is here so allocation can be managed
aggressively and *verified* (see the allocation contract in `30-execution.md`).

## Owner and workload

- Single user, embedded in his Rust applications. Not a product, not a server, no
  external API stability obligations.
- Workload shape: ledger-like, highly normalized app data — many narrow relations, many
  joins, point lookups by unique key, FK walks, time-range scans, balance-style
  aggregates. Read-heavy; writes are single-writer and comparatively rare.
- **Scale axiom: data fits in RAM.** Design envelope is up to ~100s of MB. Behavior
  beyond RAM is a non-goal; LMDB's mmap keeps us from falling off a cliff, but no design
  decision may be justified by >RAM workloads.
- Latency: interactive. Queries serve application logic, not batch pipelines.

## Target hardware

Apple Silicon M-series is the only performance target. Full research notes with sources:
`docs/reference/apple-silicon-performance.md`. The relevant profile:

- ~28 lanes of memory-level parallelism, ~630-entry ROB: the win is many *independent*
  loads in flight. Batched execution over dependent pointer-chasing.
- 128-bit NEON only (no SVE): SIMD is for filter scans and survivor compaction over
  fixed-width columns, not the primary lever.
- 128-byte cache lines: columnar data is SoA, 128-byte aligned.
- Explicit SIMD lives under `#[cfg(target_arch = "aarch64")]`; other platforms compile
  and run scalar fallback correctly, with no performance promises. x86 SIMD is forbidden.

## Non-goals

SQL. Server mode. Network protocol. Text query language (for now — see `20-query-ir.md`).
Nulls. Floats in persistent data. Bag semantics. Runtime DDL. Migrations (ETL into a new
database is the schema-change story). Async API. Multiple writers. Data beyond RAM.
Encryption/access control (application's job). Compatibility with the v1–v5 formats.

## Success criteria

1. Exact result-set equality with SQLite (`SELECT DISTINCT` oracle) on the full
   validation suite, always, before any timing claim.
2. Beats SQLite on the ledger benchmark (`50-validation.md`) — that suite, not JOB, is
   the ratchet.
3. Prepared-query execution allocates zero heap memory in steady state, enforced in CI.
4. These documents still describe the actual system six months from now.

## Decision: single primary benchmark is ledger-shaped

**Alternative:** optimize against JOB (IMDb join-order benchmark), as v2–v5 did.
**Why it lost:** JOB is an analytics benchmark; chasing it dragged the old architecture
toward machinery the owner's workload never needed, while the basics (point reads,
aggregates) went unbuilt. The benchmark quietly *became* the product thesis. JOB may
return later as a stress suite, never as the ratchet.
