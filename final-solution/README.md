# Bumbledb 1.0 — the successor proposal

**A set-semantic, Free Join application database with a small, excellent LMDB core—Rust and TypeScript, embedded or S3-backed, one student/user at a time.**

This is the coordinated breaking-release proposal, dated 2026-09-04. It incorporates the complete existing audit, the representation-first essay, the original design intent and the owner's subsequent decisions. It is a design and implementation contract, **not a claim that the rewrite, bug fixes or release qualification have already happened**. The preserved [audit](../audit/README.md) describes the old working tree; this folder selects what replaces it.

## The thesis, taken seriously

Bumbledb's strength is not that it has many features. It is that an application's facts, identities, vocabularies, relations and laws can share one precise model. The engine judges a proposed final state, queries genuine sets and uses LMDB for durable ordered storage and snapshots. Useful proofs and aggressive tests make that small center trustworthy.

The successor extends that discipline to the machinery around the facts. Commands are owned values. A read is an actual pinned snapshot. A borrow is a distinct capability. A published decision has durable identity. A float has one equality and ordering everywhere. A migration has explicit source, target and checked history. These representations remove the need for layers of defensive folklore about which callback, counter, cache, clock or pointer happens to be safe today.

The desired ordinary experience is simple: declare the theory, open the tenant, query it, submit an admitted change, and close resources predictably. Use local LMDB during embedded/local development. Use the TypeScript log when the app needs durable named commands, S3-backed history, backups or migrations. Large databases use disk and get slower as locality worsens; they do not become invalid because they crossed an arbitrary 32 GiB/RAM line.

## The selected architecture

| Layer | Responsibility | Public product |
| --- | --- | --- |
| `bumbledb` | Canonical values, facts/laws, final-state admission, queries, LMDB transactions/snapshots | Rust and TypeScript; entire public C API deleted |
| Internal Rust log implementation | Command/receipt machine, LocalHistory or HostedHistory authority, materialization, checkpoint/retention/recovery | Internal implementation, not a supported public Rust/C log SDK |
| `bumbledb-log` TypeScript package | Owned command ergonomics, async lifecycle, repo-local migrations, backup/restore operations and server integration | TypeScript/Node only in 1.0 |
| Application + Alchemy example | Authentication-to-tenant mapping, ordinary infrastructure configuration, explicit migration/cutover, business effects | A small working integration, not a fleet platform |

The core never depends on the log. TypeScript does not implement a second durability protocol. Backup, restore and migration stay at the log layer. A few generic native storage primitives let the log atomically attach its own records without teaching the core about receipts or schema migrations.

The API boundary is deliberately porous: the log imports the core's schemas, IDs/scalars, query templates, parameters, sealed `ChangeSet`, canonical codecs, read interface and results. It adds a command envelope and published-history metadata, not a second way to describe facts or query them. Both TypeScript packages use one native runtime; the Rust core remains log/AWS-independent. [34](34-sdk-syntax-and-composition.md) is the side-by-side Rust/core-TS/log-TS syntax review.

### Local and hosted both stay small

Core embedded writes are ordinary admitted LMDB transactions. LocalHistory adds named receipts and history metadata to that same transaction; it needs no remote object tail, epoch collector or periodic full checkpoint merely to reopen.

HostedHistory keeps multiple concurrent writers. Each host owns a local LMDB materialization; all writers arbitrate through one tenant HEAD on S3. Immutable decision objects plus a bounded tail over a streamed checkpoint supply recovery. A successful HEAD replacement is publication. Candidate facts remain in a private uncommitted local transaction, so normal readers never see a losing proposal.

This chooses tenant-wide atomicity and one recoverable order over braid vectors and split outcomes. It has a real cost: S3 round trips and a contention domain per tenant. It does not require a new leader service, a second storage engine or a permanent privileged writer.

## The breaking decisions that matter

- **Canonical facts are exact.** Full canonical bytes decide logical tuple equality; hashes only find candidates. Close unchecked scalar/codec construction and remove the mandatory immortal text dictionary.
- **Every stored byte earns its place.** Use 16-byte exact-checked local fingerprints, retain 32-byte authoritative commitments, and measure the physical indexes behind the recorded 2.3–2.45× SQLite space gap. TigerBeetle's AES-accelerated AEGIS-128 checksum is a pre-format benchmark candidate, not an unmeasured blanket replacement for BLAKE3. [41](41-storage-and-hashing.md) gives the byte accounting and collision budgets.
- **Floats are first-class.** Binary64 has one canonical NaN, one zero, total relational order and explicit casts. Sum and mean are included: exact accumulation and one rounding, independent of plan/spill order. `Interval<F64>` and float-bound parameters are included too. This is not decimal money or unrestricted real-number algebra.
- **Measures are ordinary laws, not weighted relations.** Keep exact count/weight/duration constraints over each parent's distinct matching child facts. Normalize equivalent supported spellings; remove cosmetic ban tables. Zero total is not absence, and summed duration is not pointwise occupancy.
- **Queries compose as relations.** Nonrecursive aggregate results can feed later queries. Names are bindings, not compulsory materializations. Preserve distinctness, aggregate/error and rounding boundaries; the only recursive component remains positive and linear, with no value creation or aggregation through its cycle.
- **Free Join remains central.** Warm application queries and selective indexed access are primary paths, not expendable optimizations. Elastic LMDB maps, ordered cursors and one RAM-to-temporary-LMDB scratch abstraction supply a complete bounded fallback beyond memory. The engine should slow with poor locality, not hit an arbitrary size ceiling.
- **One command means one final set effect.** Within a command, exact duplicates disappear and adding/removing the same exact fact uses canonical add-wins normalization. Separate commands retain their authoritative order. Read-dependent application logic uses an explicit whole-state witness.
- **Receipts resolve business uncertainty.** Commit, no-change, precondition failure and invariant rejection are named terminal outcomes. Timeouts after dispatch can be unknown. Retained receipt epochs support exact retries; expired IDs permanently refuse re-execution instead of becoming new requests.
- **Entity IDs are ordinary data.** Application-owned 128-bit IDs replace FreshRef placeholders and 28-byte issued IDs. Generate once before sealing and retain the bytes across retries. Uniqueness has the usual UUID probability model, not a database issuance theorem; keys still enforce schema laws.
- **C is gone from the product.** Hard-delete the C crate, public ABI, headers, examples and release machinery during implementation. Rust ownership and the internal Node bridge remain fully tested; native safety does not disappear with a public language surface.
- **Ownership actually releases resources.** One owner, independent borrows, bounded workers and bounded managed handles replace GC-owned environments and shared-release mistakes. Legal Rust page borrows are never revoked underneath the caller.
- **Retention is explicit.** Current recoverable state plus named restore points; hosted deletion uses epoch-qualified objects and a real publication barrier. No default 90-day/PITR clock promise. Independent backups are separate verified bytes under separate policy.
- **Users write schemas, not migrations.** The TypeScript SDK builds schema/query AST directly. The schema generator emits canonical migration plans and checked repo-local history; the log executes them. No handwritten migration callbacks, SQL parser, coverage lists or helper-purity framework. Ambiguous rename/backfill/data-loss intent requires declarative input rather than a guess.
- **Nightly is a tool.** Retain useful SIMD/try blocks, use fallible allocation APIs where they help, pin the toolchain and prove/test boundaries. No requirement for experimental specialization/coroutine/allocator frameworks.

The compact interval representation expands coherently: integer domains retain their discrete half-open intervals and maximum-word ray endpoint; `Interval<F64>` denotes continuous numeric ranges with non-NaN endpoints and infinities as bounds only. The endpoint-order operations stay shared. Float duration uses one rounding for bounded length and distinguishes unbounded length from finite overflow. Fixed-width float intervals and floating capacity weights are not added.

Apple Silicon is the first performance target; ARM Graviton and x86 Vercel are canonical portable targets. The [performance contract](40-performance-contract.md) uses `../bumblebench`'s falsifiable M2 Max evidence, not architecture-wide folklore. App-sized joins, admission, post-write reads, tenant churn and cold recovery matter more than warehouse throughput. No specialized Graviton/x86 kernel is required to qualify their correctness.

## The TypeScript application experience

The new [migration and application chapter](33-typescript-migrations-and-apps.md) specifies a Drizzle/Expo-inspired **workflow**, not SQL syntax or a mobile runtime promise: edit typed schema declarations, generate and review the plan/history artifacts, then run `migrate()` during development/deployment. Runtime consumes data; it never imports user migration functions.

Migration freezes a selected source, executes the generated pending plan into one final staged incarnation, validates its complete theory/state, then returns a still-frozen verified target binding and activation reference for explicit application cutover. Necessary intermediate checks are preserved without publishing a whole new database for every file. Failures preserve the source and operation evidence. Ordinary Next.js requests use the configured compatible binding and never trigger hidden production schema changes. Production downtime and cutover are explicit per tenant; generated does not mean zero-cost or online.

The examples target Apple Silicon development, Graviton/Lambda and x86 Vercel Node runtimes with qualified native binaries and enough local disk. Cold hosted opens materialize the tenant locally; serverless ephemeral storage limits the deployment's tenant envelope, not the LMDB engine's database size. Alchemy provisions ordinary resources and permissions. Authentication, per-tenant binding ownership and cross-service business effects remain application responsibilities. A clean `next build` and real deployed request/migration paths are release gates, not README promises.

## What we deliberately do not build

No replacement for LMDB, remote page-tree engine, analytics warehouse, public C ABI, generated-ID authority, generic storage/plugin framework, automatic tenant sharder, fleet migration scheduler, online dual-write migration service, general CRDT layer, cross-tenant transaction, textual query language, arbitrary migration-code runner, browser/Edge/Expo runtime or universal authentication product.

Semilattice laws are used where they apply: set deduplication/union, monotone frontiers and proved incremental reasoning. Keys and upper capacities are not generally union-closed; deletes and read-dependent intentions need more than union. Exact numeric accumulator merge is associative/commutative over disjoint binding partitions, **not idempotent**. [02](02-concurrency-and-semilattices.md) spells out the boundary and counterexamples.

These scope cuts are the point. The test suite may be extensive; the production mechanisms should remain few. The final scope also defers snapshot compression/native snapshot-image acceleration, auxiliary large-command objects, async bulk-command ingestion, blocking TypeScript adapters and background cache warming. LocalHistory persists receipts/current state without a duplicate command-body log. None is needed to deliver the selected application contract.

## Read the proposal

For the decision path, read **00 → 01 → 02 → 10 → 20 → 33 → 34 → 60 → 70**. Chapter 34 is the owner's syntax checkpoint before implementation; the other chapters specify the exact subsystem contracts.

| Document | Contents |
| --- | --- |
| [00 — Design contract](00-design-contract.md) | Owner requirements, binding choices, layers and subtraction ledger |
| [01 — Representation first](01-representation-first.md) | How the supplied philosophy changes the model and the machine |
| [02 — Concurrency and semilattices](02-concurrency-and-semilattices.md) | Multiwriter judgment, lawful cheap wins and explicit counterexamples |
| [10 — Semantics and engine](10-semantics-and-engine.md) | Canonical facts, admission, intervals, text, snapshots, LMDB growth and sealed host records |
| [11 — Floats](11-floats.md) | Exact binary64 value/arithmetic/aggregate/wire contract and proofs |
| [12 — Query execution](12-query-execution.md) | Primary Free Join/index paths, bounded disk fallback, scratch and atomic results |
| [13 — Lean and Rust](13-lean-and-rust.md) | Actual proof premises, empirical bridge, unsafe islands and concrete nightly policy |
| [20 — Durable protocol](20-durable-protocol.md) | Authority, command/receipt identity, ambiguity, application IDs and local/hosted specialization |
| [21 — Storage and retention](21-storage-and-retention.md) | Streamed checkpoints, exact suffix rebase, named roots, safe GC and backend qualification |
| [22 — Recovery and migrations](22-recovery-and-migrations.md) | Crash histories, independent backup, restore, migration/cutover and erasure |
| [30 — Client APIs](30-client-apis.md) | Public core versus TS-only log, command/read/result/error contracts |
| [31 — Tenant runtime](31-tenant-runtime.md) | Ownership, native close, cache identity, bounds, workers and host integration |
| [32 — FFI and packaging](32-ffi-and-release-packaging.md) | Complete C deletion, internal Node safety, platform matrix and clean release staging |
| [33 — TypeScript migrations and apps](33-typescript-migrations-and-apps.md) | Schema-generated migration plans and Next.js/Alchemy/Vercel integration |
| [34 — SDK syntax and composition](34-sdk-syntax-and-composition.md) | Side-by-side Rust core, TypeScript core and TypeScript log examples; literal primitive reuse and shared read helpers |
| [40 — Performance contract](40-performance-contract.md) | M2 Max evidence, application workload matrix and portable target qualification |
| [41 — Storage and hashing](41-storage-and-hashing.md) | Indexed-SQLite storage gap, physical byte accounting, TigerBeetle AEGIS and right-sized hashes |
| [50 — Audit closure matrix](50-audit-closure-matrix.md) | Every indexed bug/limitation plus architecture, operations, performance and assurance disposition |
| [60 — Implementation plan](60-implementation-and-release-plan.md) | Dependency-ordered rewrite, clean format break, deletions and exit gates |
| [70 — Test and release gates](70-test-and-release-gates.md) | Complete evidence matrix, detailed child gates and exact-artifact promotion |
| [90 — Provenance and review](90-provenance-and-review.md) | Source/history inputs, cross-review, actual evidence and unverified obligations |

## How this becomes 1.0

The implementation starts with canonical semantic examples, distinct format families and regression/model inventories. Physical golden bytes freeze after the targeted storage/hash/long-key probes, not before measurements exist. Values/admission, LMDB ownership and a real application vertical slice anchor parallel work on Free Join/fallback execution, the internal log machine, recovery/retention and schema generation. Performance measurements start early, before deleting existing hot paths; equivalence and cost must justify both additions and deletions.

The old audit has **47 indexed implementation observations**, plus architectural, operational, performance, assurance and unindexed boundary issues. Every one has an explicit successor obligation in [50](50-audit-closure-matrix.md). None is marked fixed by writing prose.

Before promotion, every required qualification gate must pass on the exact candidate artifacts and selected platform/backend matrix. That includes fresh Rust/Node builds, affirmative C-removal checks, independent semantic/history models, float/FPU and interval parity, generated migration plans, real S3 conditions, crash/restore schedules, native resource reclamation, a physically populated database above 40 GiB, and a separate enforced larger-than-memory workload on adequately provisioned hosts. Vercel is tested within its actual runtime/storage envelope, not assigned a fictitious 40-GiB scratch disk. Skips, missing credentials, zero matched tests and stale binaries cannot count as qualification. After authorized publication, verify downloaded artifacts and clean installation before declaring release completion.

Full streamed checkpoints, single-tenant HEAD contention, inline text, exact numerical reductions and offline migration all have real costs. They must be measured against the intended apps before claiming the design is fast enough. A failure of those measurements calls for a visible design decision, not an undisclosed second architecture.

This proposal is ambitious about quality and conservative about mechanism count. Its definition of “amazing” is concrete: a model applications can trust, a backend used well, predictable ownership and recovery, strong performance in the intended regime, correct slowdown beyond memory, and evidence behind every shipped claim.

## Final consistency verdict

**GO for implementation; not yet qualified for release.** The final cross-review reconciled migration abort versus activation/genesis, terminal deletion versus GC roots, receipt lookup versus new-command admission, shared-worker costs, and client health/ownership. The corrections use the existing authority, transaction and ownership mechanisms; they do not introduce another service or protocol. Existing gate families now explicitly exercise those edges.

The owner's subsequent measure/query-composition decisions are integrated, and chapter 34 presents proposed SDK syntax for owner review before proceeding. That review does not claim these names already compile against 0.x. No source rewrite begins as part of this documentation phase.

Start with one end-to-end slice: canonical facts → final-state judgment → LMDB → Free Join/query → TypeScript → reopen, followed by LocalHistory named retry and hosted lost-response recovery. Force the disk path against the warm path early. The remaining uncertainty is implementation/proof/performance evidence—not an invitation to expand the feature list. A new production mechanism must displace existing machinery or satisfy a selected contract that the existing mechanisms cannot; another paragraph or test family is not sufficient justification.
