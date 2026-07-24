# 00 — Product

## What this is

Bumbledb is an embedded, typed, schemaful, **set-semantic** relational database over
LMDB, built by and for one user (Bjorn Pagen) and his applications. It is Postgres's
relational elegance with the parts its owner hates removed: no SQL, no bag semantics,
no nulls, no layer cake — and a constraint system Postgres cannot follow:
**invariants are judgments about queries** (functionality and containment, plus
the cardinality-window extension form,
`30-dependencies.md`), judged once per commit against the transaction's final state,
which makes totality of sum types, conditional reference targets, and pointwise
temporal keys *statable* — and makes the SQL constraint zoo (unique, referential,
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
- **Write design point:** writes are batched — one write transaction per logical
  burst — but **no write-frequency assumption remains**: the old "writes are
  bursty and rare" and its "≥100 query executions per committed write
  generation" amortization are RETRACTED (the incremental-images wave) — they
  were workload assumptions, never measurements, and the taught idioms (the
  cookbook's delete+insert recipes, derive diffs) are steady and often
  delete-bearing. Steady insert-only commit streams are in-envelope:
  copy-on-append image maintenance prices the next read at O(delta) plus one
  column memcpy, not a full rebuild. Delete-bearing commits still pay the
  per-relation rebuild on next read — priced and measurable (the delete-bearing
  cold lane), accepted until a real workload demands the mask fork (the
  recorded decider: the filter-mask twin).
- **Latency budget:** p99 ≤ **10 ms** per warm prepared-query execution at scale L,
  on the canonical machine. No scale-L corpus has been generated yet, so the budget
  is informational at S and binds only when L exists. The first execution after a commit may additionally
  pay image maintenance; the spike is exempt from the gate but reported by the
  benchmark — with copy-on-append the exempt spike on delete-free commits is
  an O(delta) tail decode plus one O(relation) column memcpy (the write-design
  bullet above, stated the same way; the memcpy is the recorded cost the slab
  follow-on removes and is hundreds of milliseconds at ceiling scale — the
  trace counters report appends vs builds), while the
  delete-bearing rebuild spike's size at large scale is unmeasured and stays
  PENDING RE-TRUE until the delete-bearing cold lane reports (a ceiling-scale
  rebuild is seconds-order by arithmetic, `50-storage.md`).
  O(n) time-range, membership, and overlap scans must fit this budget or
  the range/stabbing-accelerator OPEN item triggers.
- **Scale axiom, in numbers:** ≤10⁷ facts total, ≤1 GB LMDB file, ≤2 GB peak process
  working set (LMDB pages + columnar images + arenas), minimum machine 16 GB Apple
  Silicon. The 32 GiB map (`50-storage.md`) is the hard capacity ceiling, not the
  design point — headroom above this axiom, never a license to lean on it.
  Data beyond RAM is a non-goal; the hot representation is decoded images, so
  beyond-RAM behavior degrades sharply and no design decision may lean on mmap grace.
  **The map ceiling no longer tracks this axiom** (the incremental-images wave's
  32 GiB ruling): the fixed 32 GiB durable map (`50-storage.md`) is the
  never-resize wall — headroom above the validated envelope, not a new
  working-set target. These numbers remain the VALIDATED scale (every corpus,
  margin, and latency claim was earned at or below them); a store pushed toward
  the ceiling leaves the validated envelope, its memory story is
  `50-storage.md` § memory discipline (peak ≈ 2–3× the decoded live payload,
  which only a RAM class well above the store's payload holds), and nothing at
  that scale is measured yet — the missing big-store witness is recorded, not
  implied away.

## Concurrency, process, and durability model

- **One writing process.** The lock law is a writer law (ruled 2026-07-23, R17):
  one WRITING handle per path. Multi-process write access to one database is out
  of the envelope (LMDB would permit it, but the environment-scope image cache and
  counter batching are process-local). Protected: every writing constructor takes
  an exclusive advisory lock on `<dir>/bumbledb.lock` for the handle's lifetime,
  so a second writer — another process, or a second `Db` on the same path in this
  one — fails loudly at open time (`EnvironmentLocked`, a writer-vs-writer
  refusal) instead of corrupting derived state silently. Readers hold no lock and
  open `MDB_RDONLY`: a read-only environment can corrupt nothing, so archival
  reads work on read-only media, restored snapshots, and mounted backups, with no
  carve-outs (`50-storage.md`, `70-api.md` § exhume). The multi-process closure
  is exactly the write surface — recorded as closed there.
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
- **Durability: fsync per commit on durable stores** (LMDB defaults). A committed
  posting survives power loss — it's a ledger. The law's scope is the durable store
  KIND: no sync mode exists on a durable store, and none may be born —
  `NOSYNC`/`WRITEMAP`/`MAPASYNC` are not expressible through the durable
  constructors. SQLite is benchmarked at `synchronous=FULL` for fairness.
  **The carve-out (a decision, not a mode): the ephemeral store kind.**
  `Db::ephemeral` births a store whose `_meta` carries an ephemeral-kind marker and
  whose environment carries `NOSYNC` (`50-storage.md` § the ephemeral store
  kind; the retired `WRITEMAP` half of the original flag set is the ruling-1
  retraction recorded there): a different store KIND with a different constructor
  and an on-disk marker —
  never a flag on `create`/`open` — so the cross-open is a typed refusal and no
  durable store can quietly lose its guarantee. The sighting is the ephemeral
  relational engine: staging stores judged before ETL into a durable store, analysis
  working sets, scratch stores — the small-commit shape where `NOSYNC`-only
  measures **27–52x over durable-on-SSD** through the real constructor on the
  same device, 43–70x for the full staging pattern (ephemeral-on-ramdisk vs
  durable-on-SSD), 3.1–3.5x over a plain-ramdisk durable store, with a
  1.1–1.6x device tax (per-session bands across three interleaved R6
  sessions, the Measure phase 2026-07-19, the R6 lane of
  `crates/bumbledb/tests/ramdisk_phase_r.rs`; the measurement artifact
  retired with the 2026-07-20 pin swap, `6d5560a8` — a git-history record,
  not a tree path; the retired `WRITEMAP|NOSYNC` band was
  ~75–90x / ~4.2–4.4x / 1.0–1.1x). The owner's doctrine, recorded
  verbatim: "everything we can do to make dogfooding easier is upgraded to a
  feature." **Alternative:** an ephemeral constructor gated on a RAM-backed-device
  precondition. **Why it lost:** the KIND carries the no-machine-crash-durability
  claim, not the device — ephemeral-on-SSD is legitimate (a machine crash loses an
  ephemeral store by definition, so no lie is possible), and a device precondition
  would smuggle device identity into the API's truth conditions. **Reverses if:** a
  workload demands durable semantics from a scratch store — which is spelled
  `Db::create` plus ETL, so never.

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
- **60–120 GB/s memory bandwidth**: the old inference here — "sequential
  scan+decode of a 100 MB relation is single-digit milliseconds, the
  quantitative reason the image-cache design (`40-execution.md` D1) is sound at
  this scale" — is PENDING RE-TRUE: it was bandwidth arithmetic, never a
  measurement of the decode-bound build path, and "this scale" moved 32× with
  the map ceiling, where the soundness argument is copy-on-append maintenance,
  not build speed (`50-storage.md` § the image cache carries the full
  re-truing record; the bandwidth numbers themselves are measured and stand,
  `docs/reference/apple-silicon-performance.md`).
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
- **The unsafe policy.** The workspace root denies `unsafe_code` for every
  engine crate; `unsafe` — including `core::arch` intrinsics and inline asm —
  is sanctioned in an explicit allowlist of modules and nowhere else. Each
  site carries `#[expect(unsafe_code, reason = …)]` (or a module-inner
  `#![allow(unsafe_code)]` where the whole module is the sanction) with its
  safety invariant written at the site. The allowlist, in three categories:
  - **Compute kernels**: `exec/kernel.rs`'s kernel modules (`neon`, `fold`,
    `filter`, `compact`, `prefetch`), `exec/colt/gather.rs` (gather/probe
    paths), `exec/wordmap.rs` with its submodules (slab probe paths), and
    `image/decode.rs` (the columnar decode kernels). The kernel law: **every
    unsafe path has a safe portable reference implementation, and a property
    test asserts bit-identical results across randomized inputs including
    boundary shapes** (empty, single, odd lengths, lane-multiple ±1). The
    differential oracle stays the outer gate; the property tests are the
    inner one.
  - **Boundary and instrument unsafe** — sites where the unsafety IS the
    foreign contract, carrying a documented safety invariant in place of a
    reference twin: `storage/env/open_env.rs` (the one raw-LMDB-open
    chokepoint — heed marks env opening and flag-setting unsafe; the
    capacity contract's preallocation sites are retired, cleanup-0.5.0
    ruling 1), `obs/fastclock.rs` (the trace-only fast clock),
    `alloc_counter.rs` (the feature-gated counting `GlobalAlloc` behind the
    allocation gate), and — in the bench crate —
    `bumbledb-bench/src/clockproxy.rs` (the register-only asm cycle proxy).
  - **Test scaffolding** — fixture-building unsafe inside test code only,
    inline-reasoned at each site, never on a shipped path:
    `tests/alloc_census.rs` (the census `GlobalAlloc`), `tests/api.rs` and
    `src/storage/env/tests.rs` (foreign/raw heed env fixtures),
    `tests/ramdisk_phase_r.rs` (the measurement scratch env),
    `src/exec/kernel/tests.rs` and `src/exec/colt/tests/pins.rs` (the
    kernel/layout pin rigs).

  One surface sits outside the workspace wall and carries its own:
  `ts/crate` denies `unsafe_code` and `unsafe_op_in_unsafe_fn` in its own
  lint table with per-site reasons on the napi FFI sites (cleanup-0.5.0
  ruling 12). (The detached `fuzz/` crate was the other, until the
  fuzzing apparatus was deleted — `60-validation.md` § the deletion
  record.)

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
image-cache design (`50-storage.md`) recovers the paper's environment at a cost
copy-on-append maintenance keeps at an O(delta) tail decode plus one O(relation)
column memcpy on delete-free commits (the old
amortize-by-write-rate argument is retracted with the write design point above);
and the ordered B-tree is what makes
pointwise keys and coverage walks O(log n) neighbor probes instead of new index
structures. **Reverses if:** traced image-rebuild cost exceeds the latency budget
despite caching, or LMDB's write amplification dominates batched commits.

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
selection checks). **Alternative (refused):** a stable/nightly split
per surface — a dual toolchain is a dual truth: two codegens, two sets
of measured margins, two CI stories. **Why one nightly:** a single dated pin
keeps reproducibility while deleting the
split before it exists, and nightly features are adopted only as dividends —
where they delete code, never because they exist. **The deliberate-move rule:**
the pin moves only as an explicit PRD-sized action that carries the `#[ignore]`d
microbench re-earn session with it (codegen changes invalidate every pinned
margin); it never floats and never moves implicitly.

## Dependencies (crates)

The engine crates (`bumbledb`, `bumbledb-macros`) depend on exactly `heed` and
`blake3` — nothing else, ever, without an owner decision. The theory member
`bumbledb-theory` (the engine-free vocabulary: `Value`, `Interval`, the Allen
masks, the descriptor/spec surface and its one lowering — `70-api.md` § the
facade ruling) carries ZERO dependencies; `bumbledb` and `bumbledb-macros`
both depend on it, hosts never name it (the `bumbledb` re-exports are the
permanent public API). The benchmark/oracle
member `bumbledb-bench` is the one quarantined exception: it may hold `rusqlite`
(bundled — the system SQLite is irrelevant and the version pinned) and **nothing
else**; argument parsing, JSON emission, statistics, and randomness are hand-rolled
there. The quarantine is one-directional: nothing in the engine may ever depend on
the bench crate. The downstream sugar member is a facade split: `bumbledb-query`
is the host surface (the `query!` re-export plus the `order` module — host-side
answer ordering; `70-api.md` § host-side sugar) and `bumbledb-query-macros` is
the proc-macro mechanics behind it (hosts still spell `bumbledb-query`). Both
carry zero foreign dependencies and sit under the same one-directional law:
hosts may depend on them; nothing in the engine ever does.

## Non-goals and deleted vocabulary

SQL. Server mode. Network protocol. Engine-side query syntax — text, builder, or
macro (the former OPEN item, superseded by the sharper ruling: queries are pure-data
IR permanently; sugar is downstream-package territory in any language, lowering to
IR — `20-query-ir.md`). Nulls. Floats in
persistent data. Bag semantics. Nominal typing. Runtime DDL. Migrations (ETL into a
new database is the schema-change story; export surface in `70-api.md`). Async API.
Multiple writers. Multi-process write access (the lock law is a writer law,
R17 — readers are lockless). Data beyond RAM. Intra-query parallelism.
Encryption/access control. Compatibility with any prior on-disk format. A deductive
database / logic-programming runtime: queries are query-sized programs against a
theory-governed store, never the unit of an application — Turing-completeness lives
in the host. Engine recursion exists under exactly this ruling: stratified
fixpoints over query-sized programs, capped (`MAX_PREDICATES`) and
budgeted, never a rule-program runtime (`20-query-ir.md` § engine
recursion, the ruling that survived the campaign whole); the closure idiom
remains the covenant for what the caps and the chain-window fence keep
outside, not a workaround.

**Built, quarantined:** the JS binding is the in-tree TypeScript SDK
(`@bjornpagen/bumbledb`, `ts/`), landed on the quarantine shape that was
recorded for it — the napi bridge is a downstream crate on the bench-crate
precedent (`ts/crate/`, outside the Cargo workspace; it holds the N-API
dependency; the engine never depends on it; no engine decision leans on its
existence), marshaling schemas and IR as data in, result copies out
(`70-api.md` § the TypeScript SDK).

**Deleted vocabulary** (each word's replacement, one line, normative in
`30-dependencies.md` and `10-data-model.md`): *primary key* → the fact is its own
identity; *unique* → functional dependency statement; *referential constraint* → containment
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
whose elements are ground axioms; the type died when the schema macro began emitting
closed-relation handles, as recorded by `10-data-model.md`'s obituary, and the
value-type roster is six); *rule program / stored rules* → the host loop over
prepared queries (queries are host data, assembled per prepare); *magic sets /
demand transformation* → the host seeds the frontier (`20-query-ir.md` § engine
recursion).

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
   engine** — earned at scale S by the committed
   `bench-out/campaign-2026-07-23/` artifacts (report provenance rev
   `1e9d39ad`, 2026-07-24: verify-stamped, `all_win: true` on every gated
   family, corpora regenerated under the fixed RNG per R20), and re-voided
   by any format or semantics change until re-run. The "ratchet" is a manually
   re-run report per meaningful change — not a CI gate.
3. **Allocation:** a warm prepared-query execution within a seen (data generation,
   parameter envelope) performs **zero heap allocations** (and zero deallocations)
   excluding a caller-provided result buffer — scratch is a monotone high-water,
   allocating only on strictly larger intermediates — asserted by a counting
   allocator under the protocol defined in `40-execution.md`. Enforcement
   is `scripts/check.sh` (the checked-in gate suite, run before every commit),
   executed verbatim by CI's check lane (`.github/workflows/ci.yml`) on
   macos-arm64 and x86_64-linux.
4. **Docs stay true** (stated intent, mechanized as rules 3/5 in
   `docs/architecture/README.md`: mechanisms name readers; code that contradicts a doc
   amends the doc in the same change).

**Decision: the primary benchmark is ledger-shaped.** **Alternative:** JOB.
**Why it lost:** chasing a benchmark that is not the workload drags the
architecture toward analytics machinery while the basics go unbuilt — the benchmark
quietly *becomes* the product thesis. JOB may run as a stress suite, never as the
ratchet. **Reverses if:** never — the thesis defines the benchmark, not vice versa.
