# 00 — Product

## What this is

Bumbledb is an embedded, typed, schemaful, **set-semantic** relational database over
LMDB, built by and for one user (Bjorn Pagen) and his applications. It is Postgres's
relational elegance with the parts its owner hates removed: no SQL, no bag semantics,
no nulls, no layer cake — and a constraint system Postgres cannot follow:
**invariants are two judgments about queries** (functionality and containment,
`30-dependencies.md`), judged once per commit against the transaction's final state,
which makes totality of sum types, conditional reference targets, and pointwise
temporal keys *statable* — and makes the SQL constraint zoo (unique, foreign key,
primary key, check, exclusion, cascade, restrict, deferrable) **deleted
vocabulary**, each word replaced by a derivation. BCNF is a modeling discipline the
owner enforces; temporality is not a discipline but a type (`Interval`,
`10-data-model.md` — the sixth and last; a vocabulary is a closed relation,
never a type).

The bet: take one good algorithm (Free Join), one elegant relational core (typed
sets + dependency judgments), one storage engine (LMDB), and push those decisions to
their logical extreme — measuring how much performance falls out of refusing to
generalize.

## Design philosophy

**Representation over control flow.** The biggest lever is the shape of the data, not
the cleverness of the code (Brooks → Pike → Raymond → Torvalds). When a case shows up
that wants a branch, a flag, or a mode, the first question is what representation would
make the case inexpressible. Illegal states unrepresentable; parse, don't validate.

**Hard structural typing.** A type is an encoding, and nothing else. Nominal typing is
rejected everywhere in the engine (owner ruling): names live in the host
language, where rustc polices them for free. See `10-data-model.md`.

**Generality of representation, discipline of acceptance.** Dependencies are stored
in the same IR queries are; the *representation* could express far more than the
engine accepts. What is accepted is a closed vocabulary where **every statement
carries an O(log n)-per-touched-fact enforcement plan or is rejected at
declaration** (`30-dependencies.md`) — the same law the performance work lives
under: an optimization that cannot cite its number does not ship, and a constraint
that cannot cite its plan does not validate.

**Rust, for allocation control.** Allocation churn is the failure mode the
zero-allocation contract (`40-execution.md`) exists to kill, and Rust makes the
contract verifiable.
**Decision:** Rust. **Alternative:** Zig/C++ offer comparable allocation control.
**Why it lost:** the owner's applications are Rust; the host language *is* the query
composition layer and the nominal-typing layer, so it must be the applications'
language. **Reverses if:** never — owner axiom.

## Owner and workload

- Single user, embedded in his Rust applications. Not a product, not a server, no
  external API-stability obligations. **Compatibility is never a design input**:
  when the design improves, the format, surface, and vocabulary break in one
  release, and data is ETL'd forward or regenerated.
- Workload shape: ledger-like, highly normalized app data — many narrow relations,
  many joins, point lookups by key, reference walks, time-range and interval
  queries, balance-style aggregates, and **scheduling** (ledger-adjacent
  calendars: interval claims, discriminated-union RSVPs, room exclusion — its
  measured form is the calendar benchmark family, `60-validation.md`).
  Read-heavy. The shape is measured, not assumed:
  two of the owner's production schemas were censused (a 74-table Postgres app and a
  payroll SQLite app); their entire type usage collapses onto the six types plus
  the closed-relation vocabulary form, their
  query operators onto this IR, and their app-enforced invariants onto the two
  judgments. The census drove the feature set; nothing shipped without a sighting
  in it — and it cuts as well as it admits: every byte-shaped sighting split into
  reuse-shaped text (`str`, interned) or identity-shaped digests (`bytes<N>`,
  inline), with variable-width binary carrying genuine reuse sighted zero times,
  so that type died and `bytes<N>` took its roster seat.
- **Write design point:** writes are bursty and batched — one write transaction per
  burst; the design assumes **≥100 query executions per committed write generation**.
  Continuous high-frequency commits are out of the envelope (they would defeat the
  image cache by design).
- **Latency budget:** p99 ≤ **10 ms** per warm prepared-query execution at scale L,
  on the canonical machine. No scale-L corpus has been generated yet, so the budget
  is informational at S and binds only when L exists. The first execution after a commit may additionally
  pay an image rebuild; rebuild spikes are exempt from the gate but reported by the
  benchmark. O(n) time-range, membership, and overlap scans must fit this budget or
  the range/stabbing-accelerator OPEN item triggers.
- **Scale axiom, in numbers:** ≤10⁷ facts total, ≤1 GB LMDB file, ≤2 GB peak process
  working set (LMDB pages + columnar images + arenas), minimum machine 16 GB Apple
  Silicon. Data beyond RAM is a non-goal; the hot representation is decoded images, so
  beyond-RAM behavior degrades sharply and no design decision may lean on mmap grace.

## Concurrency, process, and durability model

- **One process.** Multi-process access to one database is out of the envelope in v0
  (LMDB would permit it, but the environment-scope image cache and counter batching are
  process-local). Protected: every open takes an exclusive advisory lock on
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
  latency budget is violated at scale L on real workloads after single-core
  optimization is exhausted. An S-scale budget miss does not arm this trigger.
- **The single writer is also the invariant story:** commit-time final-state judgment
  plus WriteTx point reads over the same view (`50-storage.md`, `70-api.md`) means
  check-then-act inside one write transaction cannot race — the class of TOCTOU bug
  the surveyed workloads police with `FOR UPDATE` and app discipline is
  unrepresentable here. Across transactions, read-compute-write is optimistic:
  witnessed by snapshots (`write_from`, `70-api.md` § conditional writes), checked
  in O(1) at commit.
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
  everywhere. Measured refinements: the binding OoO
  window under per-item work is the ~115-entry integer issue queue, not the
  ~630-entry ROB; and dependent flag-µops per gathered load consume miss lanes
  (28 → 14 at four flag-µops per miss) — budget comparison code like cache lines.
- **Port topology decides scalar-vs-NEON** (measured): flag-writing ops
  (`adds/adcs/cmp/csel`) are confined to 3 of
  the 6 integer ALUs, so NEON wins every dense reduction (exact sums 2×, min/max
  2.65× — carry-counted `vcgtq_u64` exactness costs vector ops, not flag ports),
  while deep-OoO scalar remains the shape for irregular control flow. 128-bit NEON
  (no SVE) keeps a closed set of sanctioned kernel
  shapes: fixed-width predicate scans, survivor compaction,
  fold/accumulate kernels — dense sums via carry-counted exact u128 now among
  them — (Sum/Min/Max/Count over batch columns, strided or gathered), gather
  kernels (position-indexed column reads), and software-prefetch passes (`prfm`)
  in two-phase probing. Kernel adoption never changes semantics: Sum stays
  i128-accumulated with one range check at finalization, and every kernel ships
  with a portable reference and a bit-identity differential test. Interval
  conditions introduced no new shape — they lower to two-word compares over the
  start/end column pair (`50-storage.md`).
- **60–120 GB/s memory bandwidth**: sequential scan+decode of a 100 MB relation is
  single-digit milliseconds — the quantitative reason the image-cache design
  (`40-execution.md` D1) is sound at this scale.
- **Unaligned loads are near-free (16 KB pages)**: facts are stored dense, with no
  intra-row padding; alignment is spent only where NEON reads column bases.
- **Columnar data is SoA, 128-byte aligned, with strides padded off 16 KiB
  multiples** (`50-storage.md`; measured): the L1D manages 64 B lines behind
  a 128 B memory-system granule — both numbers are real, at different levels —
  and its set congruence costs at most 1.55× on real lockstep scans. The layout
  hazard that actually matters is prefetch-tracker aliasing on 16 KiB page-number
  bits (4–6× on DRAM lockstep scans); one page of stride padding cures it.
- **TAGE branch prediction (>99%)**: the residual misprediction source is per-tuple
  data-dependent branching — batching converts it into branchless compaction; and the
  hot path contains no indirect dispatch (sinks/counters monomorphized,
  `40-execution.md`).
- Explicit SIMD lives under `#[cfg(target_arch = "aarch64")]`; other 64-bit platforms
  compile and run scalar fallback correctly, with no performance promises. x86 SIMD is
  forbidden.
- **The unsafe policy.** `unsafe` —
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
the kind of subtle, unglamorous correctness surface this project should not own; the
image-cache design (`50-storage.md`) recovers the paper's environment at a cost the
write-rate design point makes negligible; and the ordered B-tree is what makes
pointwise keys and coverage walks O(log n) neighbor probes instead of new index
structures. **Reverses if:** traced image-rebuild cost exceeds the latency budget
despite caching, or LMDB's write amplification dominates bursty commits.

**Decision: Free Join is the execution algorithm.** **Alternative (strong):**
Selinger-planned binary hash joins — for reference-walk-heavy ledger queries they are
the obvious contender, and the paper's own wins concentrate on cyclic/skewed queries.
**Why it lost:** Free Join *contains* binary hash join — a left-deep FJ plan with lazy
COLT executes the same loops binary join would, at the same cost, while the same plan
formalism reaches Generic Join for the cyclic/skew cases free of charge. The unified
plan space is strictly larger for one kernel's complexity, and exploring that space is
the stated point of the project. **Reverses if:** the ledger benchmark shows the FJ
kernel measurably slower than a plain hash join on the same plans.

**Decision: one pinned nightly toolchain, edition 2024.** `rust-toolchain.toml`
names one dated nightly (`nightly-2026-07-12`; the comment block records the
selection checks). **Alternative (refused):** a stable pin with a nightly split
for the fuzz targets — a dual toolchain is a dual truth: two codegens, two sets
of measured margins, two CI stories. **Why one nightly:** cargo-fuzz needs
nightly anyway; a single dated pin keeps reproducibility while deleting the
split before it exists, and nightly features are adopted only as dividends —
where they delete code, never because they exist. **The deliberate-move rule:**
the pin moves only as an explicit PRD-sized action that carries the `#[ignore]`d
microbench re-earn session with it (codegen changes invalidate every pinned
margin); it never floats and never moves implicitly.

## Dependencies (crates)

The engine crates (`bumbledb`, `bumbledb-macros`) depend on exactly `heed` and
`blake3` — nothing else, ever, without an owner decision. The benchmark/oracle
member `bumbledb-bench` is the one quarantined exception: it may hold `rusqlite`
(bundled — the system SQLite is irrelevant and the version pinned) and **nothing
else**; argument parsing, JSON emission, statistics, and randomness are hand-rolled
there. The quarantine is one-directional: nothing in the engine may ever depend on
the bench crate. The downstream sugar member `bumbledb-query` (the `query!` macro,
`70-api.md` § host-side sugar) carries zero foreign dependencies and sits under the
same one-directional law: hosts may depend on it; nothing in the engine ever does.

## Non-goals and deleted vocabulary

SQL. Server mode. Network protocol. Engine-side query syntax — text, builder, or
macro (the former OPEN item, superseded by the sharper ruling: queries are pure-data
IR permanently; sugar is downstream-package territory in any language, lowering to
IR — `20-query-ir.md`). Nulls. Floats in
persistent data. Bag semantics. Nominal typing. Runtime DDL. Migrations (ETL into a
new database is the schema-change story; export surface in `70-api.md`). Async API.
Multiple writers. Multi-process access. Data beyond RAM. Intra-query parallelism.
Encryption/access control. Compatibility with any prior on-disk format.

**Anticipated, not built:** JS/N-API bindings are punted with zero deliverable and
a recorded quarantine shape — a downstream crate on the bench-crate precedent (it
may hold the N-API dependency; the engine never depends on it; no engine decision
may lean on its existence), compiling the application's `schema!` in and marshaling
IR-as-data in, result copies out (`70-api.md` § anticipated bindings).

**Deleted vocabulary** (each word's replacement, one line, normative in
`30-dependencies.md` and `10-data-model.md`): *primary key* → the fact is its own
identity; *unique* → functional dependency statement; *foreign key* → containment
statement; *check constraint* → host newtype constructors; *exclusion constraint* →
functional dependency over an interval position; *cascade* → same-transaction
cluster demolition under final-state judgment; *restrict / no action / deferrable* →
final-state judgment is the only timing; *trigger* → nothing, on purpose; *view* → a function returning atoms;
*materialized view / refresh* → a relation under statements, maintained by
witnessed writes; *null* →
absent fact in a 0..1 child relation; *uuid* → fresh + explicit time columns;
*update / upsert* → delete+insert, with WriteTx point reads for the read-modify-write
idiom; *SELECT FOR UPDATE / row locks / SERIALIZABLE retry* → the generation witness
(snapshot-witnessed `write_from`) plus WriteTx point reads under final-state
judgment — locks protect what you remembered to lock; the witness protects
everything the snapshot saw; *enum* → closed relation (a vocabulary is a relation
whose rows are ground axioms; the type died when the schema macro began emitting
closed-relation handles, as recorded by `10-data-model.md`'s obituary, and the
value-type roster is six).

## Success criteria

1. **Exactness:** exact result-set equality with SQLite on the full validation suite
   wherever SQLite can express the query, and engine/naive-model agreement
   everywhere — queries *and* dependency verdicts — always, before any timing claim.
   The two-oracle construction is normative in `60-validation.md`. **Mechanism:**
   `bumbledb-bench verify` — every family and N randomized queries compared, a stamp
   on success, arbitration bundles on failure; `bench` refuses to time without the
   stamp.
2. **Performance:** beats SQLite on the ledger benchmark under the protocol in
   `60-validation.md`: per-family median, **every family must win**, warm timing gates
   (cold reported, not gated), SQLite fully indexed + prepared + `ANALYZE`d,
   `synchronous=FULL`, `SELECT DISTINCT` included in timed SQL, canonical machine =
   the owner's. **The claim is unearned until the suite runs green on this
   engine.** The "ratchet" is a manually re-run report per meaningful change — not
   a CI gate.
3. **Allocation:** a warm prepared-query execution within a seen (data generation,
   parameter envelope) performs **zero heap allocations** (and zero deallocations)
   excluding a caller-provided result buffer — scratch is a monotone high-water,
   allocating only on strictly larger intermediates — asserted by a counting
   allocator under the protocol defined in `40-execution.md`. Enforcement
   today is `scripts/check.sh` (the checked-in gate suite, run before every commit);
   it becomes a CI gate verbatim when CI exists.
4. **Docs stay true** (stated intent, mechanized as rules 3/5 in
   `docs/architecture/README.md`: mechanisms name readers; code that contradicts a doc
   amends the doc in the same change).

**Decision: the primary benchmark is ledger-shaped.** **Alternative:** JOB.
**Why it lost:** chasing a benchmark that is not the workload drags the
architecture toward analytics machinery while the basics go unbuilt — the benchmark
quietly *becomes* the product thesis. JOB may run as a stress suite, never as the
ratchet. **Reverses if:** never — the thesis defines the benchmark, not vice versa.
