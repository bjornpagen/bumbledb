# 00 — Product

## What this is

Bumbledb is an embedded, typed, schemaful, **set-semantic** relational database over
LMDB, built by and for one user (Bjorn Pagen) and his applications. It is Postgres's
relational elegance with the parts its owner hates removed: no SQL, no bag semantics, no
nulls, no layer cake. BCNF-normalized typed relations — a modeling discipline the owner
enforces, not a property the schema layer checks (declared FDs are not validated; "no
way out" refers to nulls/blobs/EAV being unrepresentable, which they are).

The bet: take one good algorithm (Free Join), one elegant relational core (typed sets),
one storage engine (LMDB), and push those decisions to their logical extreme — measuring
how much performance falls out of refusing to generalize.

## Design philosophy

**Representation over control flow.** The biggest lever is the shape of the data, not
the cleverness of the code (Brooks → Pike → Raymond → Torvalds). When a case shows up
that wants a branch, a flag, or a mode, the first question is what representation would
make the case inexpressible. Illegal states unrepresentable; parse, don't validate.

**Hard structural typing.** A type is an encoding, and nothing else. Nominal typing is
rejected everywhere in the engine (owner ruling, 2026-07-02): names live in the host
language, where rustc polices them for free. See `10-data-model.md`.

**Rust, for allocation control.** Allocation churn was an Achilles heel of the five
discarded implementations (post-mortem §32–33). Rust makes the zero-allocation contract
(`30-execution.md`) verifiable.
**Decision:** Rust. **Alternative:** Zig/C++ offer comparable allocation control.
**Why it lost:** the owner's applications are Rust; the host language *is* the query
composition layer and the nominal-typing layer, so it must be the applications'
language. **Reverses if:** never — owner axiom.

## Owner and workload

- Single user, embedded in his Rust applications. Not a product, not a server, no
  external API-stability obligations.
- Workload shape: ledger-like, highly normalized app data — many narrow relations, many
  joins, point lookups by unique key, FK walks, time-range scans, balance-style
  aggregates. Read-heavy.
- **Write design point:** writes are bursty and batched — one write transaction per
  burst; the design assumes **≥100 query executions per committed write generation**.
  Continuous high-frequency commits are out of the envelope (they would defeat the image
  cache by design).
- **Latency budget:** p99 ≤ **10 ms** per warm prepared-query execution at the top scale
  point, on the canonical machine. The first execution after a commit may additionally
  pay an image rebuild; rebuild spikes are exempt from the gate but reported by the
  benchmark. O(n) time-range scans must fit this budget or the range-accelerator OPEN
  item triggers.
- **Scale axiom, in numbers:** ≤10⁷ facts total, ≤1 GB LMDB file, ≤2 GB peak process
  working set (LMDB pages + columnar images + arenas), minimum machine 16 GB Apple
  Silicon. Data beyond RAM is a non-goal; the hot representation is decoded images, so
  beyond-RAM behavior degrades sharply and no design decision may lean on mmap grace.

## Concurrency, process, and durability model

- **One process.** Multi-process access to one database is out of the envelope in v0
  (neither supported nor guarded; LMDB would permit it, but the environment-scope image
  cache and counter batching are process-local). Recorded as closed.
- Within the process: one writer at a time, many concurrent reader threads (LMDB MVCC).
- **Query execution is single-threaded** per query; intra-query parallelism is a
  non-goal (matches the paper's system). A prepared query is not shareable across
  threads (`30-execution.md`).
- **Durability: fsync per commit** (LMDB defaults; no `NOSYNC`/`WRITEMAP`/`MAPASYNC`).
  A committed posting survives power loss — it's a ledger. SQLite is benchmarked at
  `synchronous=FULL` for fairness.

## Target hardware

Apple Silicon M-series is the only performance target. Full research notes with sources:
`docs/reference/apple-silicon-performance.md`. The relevant profile:

- ~28 lanes of memory-level parallelism, ~630-entry ROB: the win is many *independent*
  loads in flight. Batched execution over dependent pointer-chasing.
- 128-bit NEON only (no SVE): SIMD is for filter scans and survivor compaction over
  fixed-width columns, not the primary lever.
- Columnar data is SoA, **128-byte aligned**. (The reference notes carry a flagged
  internal contradiction on L1D line size, 64 B vs 128 B; 128-byte alignment is correct
  under either reading since it implies 64-byte alignment, and matches L2/SLC lines.)
- Explicit SIMD lives under `#[cfg(target_arch = "aarch64")]`; other platforms compile
  and run scalar fallback correctly, with no performance promises. x86 SIMD is forbidden.

**Decision:** Apple-Silicon-only performance target. **Alternative:** portable
performance posture. **Why it lost:** there are no other consumers; portability spends
design effort on hardware nobody runs. **Reverses if:** the owner's fleet changes.

## Load-bearing platform decisions

**Decision: LMDB is the only durable backend.** **Alternative (strong):** in-memory
tables + append-only WAL/snapshot file — at this scale it would make the durable
representation identical to the paper's execution environment, deleting Deviation D1 and
the image cache entirely. **Why it lost:** LMDB gives crash-safe atomic commits, real
MVCC read snapshots, and a battle-tested B-tree for free; a hand-rolled WAL is exactly
the kind of subtle, unglamorous correctness surface this project should not own, and the
image-cache design (`40-storage.md`) recovers the paper's environment at a cost the
write-rate design point makes negligible. **Reverses if:** traced image-rebuild cost
exceeds the latency budget despite caching, or LMDB's write amplification dominates
bursty commits.

**Decision: Free Join is the execution algorithm.** **Alternative (strong):**
Selinger-planned binary hash joins — for FK-walk-heavy ledger queries they are the
obvious contender, and the paper's own wins concentrate on cyclic/skewed queries.
**Why it lost:** Free Join *contains* binary hash join — a left-deep FJ plan with lazy
COLT executes the same loops binary join would, at the same cost, while the same plan
formalism reaches Generic Join for the cyclic/skew cases free of charge. The unified
plan space is strictly larger for one kernel's complexity, and exploring that space is
the stated point of the project. **Reverses if:** the ledger benchmark shows the FJ
kernel measurably slower than a plain hash join on the same plans.

## Non-goals

SQL. Server mode. Network protocol. Text query language (OPEN). Nulls. Floats in
persistent data. Bag semantics. Nominal typing. Runtime DDL. Migrations (ETL into a new
database is the schema-change story; export surface in `60-api.md`). Async API. Multiple
writers. Multi-process access. Data beyond RAM. Intra-query parallelism.
Encryption/access control. Compatibility with v1–v5 formats.

## Success criteria

1. **Exactness:** exact result-set equality with SQLite on the full validation suite,
   always, before any timing claim. The oracle construction — value mapping, aggregate
   template, U64 rule — is normative in `50-validation.md`.
2. **Performance:** beats SQLite on the ledger benchmark under the protocol in
   `50-validation.md`: per-family median, **every family must win**, warm timing gates
   (cold reported, not gated), SQLite fully indexed + prepared + `ANALYZE`d,
   `synchronous=FULL`, `SELECT DISTINCT` included in timed SQL, canonical machine = the
   owner's. The claim is **void until the aggregate families are in the suite**. The
   "ratchet" is a manually re-run report per meaningful change — not a CI gate.
3. **Allocation:** a warm prepared-query execution performs **zero heap allocations**
   excluding a caller-provided result buffer, enforced in CI by a counting allocator
   under the protocol defined in `30-execution.md`.
4. **Docs stay true** (stated intent, mechanized as rules 3/5 in the README: mechanisms
   name readers; code that contradicts a doc amends the doc in the same change).

**Decision: the primary benchmark is ledger-shaped.** **Alternative:** JOB, as v2–v5
used. **Why it lost:** chasing JOB dragged the old architecture toward analytics
machinery while the basics went unbuilt; the benchmark quietly *became* the product
thesis (post-mortem §00). JOB may return as a stress suite, never as the ratchet.
**Reverses if:** never — the thesis defines the benchmark, not vice versa.
