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
  (LMDB would permit it, but the environment-scope image cache and counter batching are
  process-local). Guarded: every open takes an exclusive advisory lock on
  `<dir>/bumbledb.lock` for the handle's lifetime, so a second handle — another process,
  or a second `Db` on the same path in this one — fails loudly at open time
  (`EnvironmentLocked`) instead of corrupting derived state silently. Recorded as
  closed.
- **Threading doctrine: bumbledb owns zero threads.** The engine never spawns one — no
  background writers, compactors, or build pools; all threads belong to the
  application, extending the host-owns-composition principle to scheduling. Parallelism
  is **inter-query**: many reader threads run their own prepared queries concurrently
  (LMDB MVCC snapshots; immutable `Arc`-shared images make this free), while one thread
  at a time writes. **Within a query, execution is single-threaded** — the parallelism
  is one P-core's ~28 MLP lanes, which the batched executor exists to saturate. Why not
  intra-query threads: at ≤10⁷ facts a single M-series core (3–4 IPC, 60–120 GB/s to
  memory) fits the 10 ms budget with headroom, while partitioned WCOJ needs per-thread
  sinks, dedup merges, and shared arenas — real complexity against the zero-allocation
  contract, bought for latency we don't need. A prepared query is `!Sync`.
  **Decision.** **Alternative:** partition the root cover across cores (per-core
  bindings/sinks, merge at the end). **Why it lost:** above. **Reverses if:** the
  latency budget is violated on real workloads after single-core optimization is
  exhausted.
- **Durability: fsync per commit** (LMDB defaults; no `NOSYNC`/`WRITEMAP`/`MAPASYNC`).
  A committed posting survives power loss — it's a ledger. SQLite is benchmarked at
  `synchronous=FULL` for fairness.

## Target hardware

Apple Silicon M-series is the only performance target; **64-bit only** (enforced at
compile time — 32-bit targets are rejected, `usize` is 8 bytes everywhere, and no design
decision accommodates narrower platforms). Full research notes with sources:
`docs/reference/apple-silicon-performance.md`. The machine model the design exploits:

- **~28–33 MLP lanes per core, bounded by smaller queues first**: the win is many
  *independent* loads in flight — batched execution over dependent pointer-chasing,
  everywhere. Measured refinements (docs/silicon/, bumblebench): the binding OoO
  window under per-item work is the ~115-entry integer issue queue, not the
  ~630-entry ROB; and dependent flag-µops per gathered load consume miss lanes
  (28 → 14 at four flag-µops per miss) — budget comparison code like cache lines.
- **Port topology decides scalar-vs-NEON** (docs/silicon/06, superseding
  scalar-ILP-first): flag-writing ops (`adds/adcs/cmp/csel`) are confined to 3 of
  the 6 integer ALUs, so NEON wins every dense reduction (exact sums 2×, min/max
  2.65× — carry-counted `vcgtq_u64` exactness costs vector ops, not flag ports),
  while deep-OoO scalar remains the shape for irregular control flow. 128-bit NEON
  (no SVE) keeps a closed set of sanctioned kernel shapes (amended by docs/perf/
  and docs/silicon/): fixed-width predicate scans, survivor compaction,
  fold/accumulate kernels — dense sums via carry-counted exact u128 now among
  them — (Sum/Min/Max/Count over batch columns, strided or gathered), gather
  kernels (position-indexed column reads), and software-prefetch passes (`prfm`)
  in two-phase probing. Kernel adoption never changes semantics: Sum stays
  i128-accumulated with one range check at finalization, and every kernel ships
  with a portable reference and a bit-identity differential test.
- **60–120 GB/s memory bandwidth**: sequential scan+decode of a 100 MB relation is
  single-digit milliseconds — the quantitative reason the image-cache design
  (`30-execution.md` D1) is sound at this scale.
- **Unaligned loads are near-free (16 KB pages)**: facts are stored dense, with no
  intra-row padding; alignment is spent only where NEON reads column bases.
- **Columnar data is SoA, 128-byte aligned, with staggered column bases**: L1D is 8-way
  with a 16 KB set stride, so columns scanned in lockstep must not sit at bases
  congruent mod 16 KB — the pathological aliasing case is a 10–20× slowdown
  (`40-storage.md`). (The reference notes carry a flagged 64 B-vs-128 B L1D-line
  contradiction; 128-byte alignment is correct under either reading.)
- **TAGE branch prediction (>99%)**: the residual misprediction source is per-tuple
  data-dependent branching — batching converts it into branchless compaction; and the
  hot path contains no indirect dispatch (sinks/counters monomorphized,
  `30-execution.md`).
- Explicit SIMD lives under `#[cfg(target_arch = "aarch64")]`; other 64-bit platforms
  compile and run scalar fallback correctly, with no performance promises. x86 SIMD is
  forbidden.
- **The unsafe policy (amended by the performance suite, docs/perf/).** `unsafe` —
  including `core::arch` intrinsics and inline asm — is sanctioned in an explicit
  allowlist of kernel/hot modules and nowhere else: `exec/kernel.rs`,
  `exec/colt.rs` (gather/probe paths), `exec/wordmap.rs` (slab probe paths),
  `exec/run.rs` (leaf/batch paths), `image.rs` (decode kernels), and `obs.rs`
  (the trace-only fast clock). Each carries `#[allow(unsafe_code)]` at module or
  item level with a comment naming this policy; the crate denies `unsafe_code`
  everywhere else. The law, extended from kernel.rs: **every unsafe path has a
  safe portable reference implementation, and a property test asserts
  bit-identical results across randomized inputs including boundary shapes**
  (empty, single, odd lengths, lane-multiple ±1). The differential oracle stays
  the outer gate; the property tests are the inner one.

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

## Dependencies

The engine crates (`bumbledb`, `bumbledb-macros`) depend on exactly `heed` and
`blake3` — nothing else, ever, without an owner decision. The benchmark/oracle
member `bumbledb-bench` is the one quarantined exception: it may hold `rusqlite`
(bundled — the system SQLite is irrelevant and the version pinned) and **nothing
else**; argument parsing, JSON emission, statistics, and randomness are hand-rolled
there. The quarantine is one-directional: nothing in the engine may ever depend on
the bench crate.

## Non-goals

SQL. Server mode. Network protocol. Text query language (OPEN). Nulls. Floats in
persistent data. Bag semantics. Nominal typing. Runtime DDL. Migrations (ETL into a new
database is the schema-change story; export surface in `60-api.md`). Async API. Multiple
writers. Multi-process access. Data beyond RAM. Intra-query parallelism.
Encryption/access control. Compatibility with v1–v5 formats.

## Success criteria

1. **Exactness:** exact result-set equality with SQLite on the full validation suite,
   always, before any timing claim. The oracle construction — value mapping, aggregate
   template, U64 rule — is normative in `50-validation.md`. **Mechanism:**
   `bumbledb-bench verify` (docs/architecture/50-validation.md) — every family and N randomized
   queries compared as multisets, a stamp on success, arbitration bundles on failure;
   `bench` refuses to time without the stamp.
2. **Performance:** beats SQLite on the ledger benchmark under the protocol in
   `50-validation.md`: per-family median, **every family must win**, warm timing gates
   (cold reported, not gated), SQLite fully indexed + prepared + `ANALYZE`d,
   `synchronous=FULL`, `SELECT DISTINCT` included in timed SQL, canonical machine = the
   owner's. The claim is **void until the aggregate families are in the suite**. The
   "ratchet" is a manually re-run report per meaningful change — not a CI gate.
   **Mechanism:** `bumbledb-bench bench` — the gate verdict,
   budget lines, and artifacts; the aggregate families (balance, stats) are in the
   suite, so the claim awaits only the human L-scale ALL-WIN run.
3. **Allocation:** a warm prepared-query execution performs **zero heap allocations**
   (and zero deallocations) excluding a caller-provided result buffer, asserted by a
   counting allocator under the protocol defined in `30-execution.md`. Enforcement
   today is `scripts/check.sh` (the checked-in gate suite, run before every commit);
   it becomes a CI gate verbatim when CI exists.
4. **Docs stay true** (stated intent, mechanized as rules 3/5 in
   `docs/architecture/README.md`: mechanisms name readers; code that contradicts a doc
   amends the doc in the same change).

**Decision: the primary benchmark is ledger-shaped.** **Alternative:** JOB, as v2–v5
used. **Why it lost:** chasing JOB dragged the old architecture toward analytics
machinery while the basics went unbuilt; the benchmark quietly *became* the product
thesis (post-mortem §00). JOB may return as a stress suite, never as the ratchet.
**Reverses if:** never — the thesis defines the benchmark, not vice versa.
