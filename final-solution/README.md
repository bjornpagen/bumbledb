# Bumbledb 1.0 — the successor proposal

**A set-semantic application database with a small, excellent LMDB core—and a TypeScript log product that makes it useful in real applications.**

This is the coordinated breaking-release proposal, dated 2026-09-04. It incorporates the complete existing audit, the representation-first essay, the original design intent and the owner's subsequent decisions. It is a design and implementation contract, **not a claim that the rewrite, bug fixes or release qualification have already happened**. The preserved [audit](../audit/README.md) describes the old working tree; this folder selects what replaces it.

## The thesis, taken seriously

Bumbledb's strength is not that it has many features. It is that an application's facts, identities, vocabularies, relations and laws can share one precise model. The engine judges a proposed final state, queries genuine sets and uses LMDB for durable ordered storage and snapshots. Useful proofs and aggressive tests make that small center trustworthy.

The successor extends that discipline to the machinery around the facts. Commands are owned values. A read is an actual pinned snapshot. A borrow is a distinct capability. A published decision has durable identity. A float has one equality and ordering everywhere. A migration has explicit source, target and checked history. These representations remove the need for layers of defensive folklore about which callback, counter, cache, clock or pointer happens to be safe today.

The desired ordinary experience is simple: declare the theory, open the tenant, query it, submit an admitted change, and close resources predictably. Use local LMDB during embedded/local development. Use the TypeScript log when the app needs durable named commands, S3-backed history, backups or migrations. Large databases use disk and get slower as locality worsens; they do not become invalid because they crossed an arbitrary 32 GiB/RAM line.

## The selected architecture

| Layer | Responsibility | Public product |
| --- | --- | --- |
| `bumbledb` | Canonical values, facts/laws, final-state admission, queries, LMDB transactions/snapshots | Rust, TypeScript, C |
| Internal Rust log implementation | Command/receipt machine, LocalHistory or HostedHistory authority, materialization, checkpoint/retention/recovery | Internal implementation, not a supported public Rust/C log SDK |
| `bumbledb-log` TypeScript package | Owned command ergonomics, async lifecycle, repo-local migrations, backup/restore operations and server integration | TypeScript/Node only in 1.0 |
| Application + Alchemy example | Authentication-to-tenant mapping, ordinary infrastructure configuration, explicit migration/cutover, business effects | A small working integration, not a fleet platform |

The core never depends on the log. TypeScript does not implement a second durability protocol. Backup, restore and migration stay at the log layer. A few generic native storage primitives let the log atomically attach its own records without teaching the core about receipts or schema migrations.

### Local and hosted both stay small

Core embedded writes are ordinary admitted LMDB transactions. LocalHistory adds named receipts and history metadata to that same transaction; it needs no remote object tail, epoch collector or periodic full checkpoint merely to reopen.

HostedHistory keeps multiple concurrent writers. Each host owns a local LMDB materialization; all writers arbitrate through one tenant HEAD on S3. Immutable decision objects plus a bounded tail over a streamed checkpoint supply recovery. A successful HEAD replacement is publication. Candidate facts remain in a private uncommitted local transaction, so normal readers never see a losing proposal.

This chooses tenant-wide atomicity and one recoverable order over braid vectors and split outcomes. It has a real cost: S3 round trips and a contention domain per tenant. It does not require a new leader service, a second storage engine or a permanent privileged writer.

## The breaking decisions that matter

- **Canonical facts are exact.** Full canonical bytes decide logical tuple equality; hashes only find candidates. Close unchecked scalar/codec construction and remove the mandatory immortal text dictionary.
- **Floats are first-class.** Binary64 has one canonical NaN, one zero, total relational order and explicit casts. Scalar arithmetic follows a guarded deterministic contract. Set aggregates use exact accumulation and one rounding, independent of plan/spill order. This is not decimal money or unrestricted real-number algebra.
- **LMDB is the actual baseline.** Elastic 64-bit maps, ordered cursors and one RAM-to-temporary-LMDB scratch abstraction support execution beyond memory. Warm SIMD/Free Join kernels remain optional, measured accelerators.
- **One command means one final set effect.** Within a command, exact duplicates disappear and adding/removing the same exact fact uses canonical add-wins normalization. Separate commands retain their authoritative order. Read-dependent application logic uses an explicit whole-state witness.
- **Receipts resolve business uncertainty.** Commit, no-change, precondition failure and invariant rejection are named terminal outcomes. Timeouts after dispatch can be unknown. Retained receipt epochs support exact retries; expired IDs permanently refuse re-execution instead of becoming new requests.
- **Fresh entities need no allocator service.** Command-local placeholders resolve from winning incarnation/decision/ordinal into nominal 28-byte IDs. Existing IDs remain ordinary data after restore; only newly issued IDs use the new lineage.
- **Ownership actually releases resources.** One owner, independent borrows, bounded workers and generation-tagged managed/C handles replace GC-owned environments, shared-release mistakes and immortal callback tombstones. Legal Rust page borrows are never revoked underneath the caller.
- **Retention is explicit.** Current recoverable state plus named restore points; hosted deletion uses epoch-qualified objects and a real publication barrier. No default 90-day/PITR clock promise. Independent backups are separate verified bytes under separate policy.
- **TypeScript is the log's product surface.** Drop public Rust/C log APIs while keeping the native implementation and its tests. Migration files live in the application's repo and run through an explicit checked workflow; ordinary requests check compatibility instead of racing migrations.
- **Nightly is a tool.** Retain useful SIMD/try blocks, use fallible allocation APIs where they help, pin the toolchain and prove/test boundaries. No requirement for experimental specialization/coroutine/allocator frameworks.

The compact interval representation stays: nonempty half-open discrete intervals, with the maximum integer reserved as their ray endpoint. Churn is permitted, not mandatory. The audit's constructor defect is fixed at construction rather than expanding every interval kernel without an application need.

## The TypeScript application experience

The new [migration and application chapter](33-typescript-migrations-and-apps.md) specifies a Drizzle/Expo-inspired **workflow**, not SQL syntax or a mobile runtime promise: an ordered migration bundle in the repo, immutable checked history, explicit TypeScript transforms and `migrate()` during development/deployment.

Migration freezes a selected source, transforms into a staged new incarnation, validates its complete theory/state, then returns a still-frozen verified target binding and activation reference for explicit application cutover. Failures preserve the source and operation evidence. Ordinary Next.js requests use the configured compatible binding and never trigger hidden production schema changes. Local development is easy; production downtime and cutover are explicit per tenant.

The example targets supported Node/server runtimes with native binaries and enough local disk. Alchemy provisions the ordinary resources and permissions the example uses. Authentication, per-tenant binding ownership and cross-service business effects remain application responsibilities. A clean `next build` and a real deployed request/migration path are release gates, not a README promise.

## What we deliberately do not build

No replacement for LMDB, remote page-tree engine, generic storage/plugin framework, automatic tenant sharder, fleet migration scheduler, online dual-write migration service, general CRDT layer, cross-tenant transaction, arbitrary query language, browser/Edge/Expo runtime or universal authentication product.

Semilattice laws are used where they apply: set deduplication/union, monotone frontiers and proved incremental reasoning. Keys and upper capacities are not generally union-closed; deletes and read-dependent intentions need more than union. Exact numeric accumulator merge is associative/commutative over disjoint binding partitions, **not idempotent**. [02](02-concurrency-and-semilattices.md) spells out the boundary and counterexamples.

These scope cuts are the point. The test suite may be extensive; the production mechanisms should remain few.

## Read the proposal

For the decision path, read **00 → 01 → 02 → 10 → 20 → 33 → 60 → 70**. The other chapters specify the exact subsystem contracts.

| Document | Contents |
| --- | --- |
| [00 — Design contract](00-design-contract.md) | Owner requirements, binding choices, layers and subtraction ledger |
| [01 — Representation first](01-representation-first.md) | How the supplied philosophy changes the model and the machine |
| [02 — Concurrency and semilattices](02-concurrency-and-semilattices.md) | Multiwriter judgment, lawful cheap wins and explicit counterexamples |
| [10 — Semantics and engine](10-semantics-and-engine.md) | Canonical facts, admission, intervals, text, snapshots, LMDB growth and sealed host records |
| [11 — Floats](11-floats.md) | Exact binary64 value/arithmetic/aggregate/wire contract and proofs |
| [12 — Query execution](12-query-execution.md) | Disk-native baseline, scratch, bounded work, optional kernels and atomic results |
| [13 — Lean and Rust](13-lean-and-rust.md) | Actual proof premises, empirical bridge, unsafe islands and concrete nightly policy |
| [20 — Durable protocol](20-durable-protocol.md) | Authority, command/receipt identity, ambiguity, fresh IDs and local/hosted specialization |
| [21 — Storage and retention](21-storage-and-retention.md) | Streamed checkpoints, exact suffix rebase, named roots, safe GC and backend qualification |
| [22 — Recovery and migrations](22-recovery-and-migrations.md) | Crash histories, independent backup, restore, migration/cutover and erasure |
| [30 — Client APIs](30-client-apis.md) | Public core versus TS-only log, command/read/result/error contracts |
| [31 — Tenant runtime](31-tenant-runtime.md) | Ownership, native close, cache identity, bounds, workers and host integration |
| [32 — FFI and packaging](32-ffi-and-release-packaging.md) | Core C/Node safety, artifact/ABI/platform matrix and clean release staging |
| [33 — TypeScript migrations and apps](33-typescript-migrations-and-apps.md) | Repo-local migration API and Next.js + Alchemy integration |
| [50 — Audit closure matrix](50-audit-closure-matrix.md) | Every indexed bug/limitation plus architecture, operations, performance and assurance disposition |
| [60 — Implementation plan](60-implementation-and-release-plan.md) | Dependency-ordered rewrite, clean format break, deletions and exit gates |
| [70 — Test and release gates](70-test-and-release-gates.md) | Complete evidence matrix, detailed child gates and exact-artifact promotion |
| [90 — Provenance and review](90-provenance-and-review.md) | Source/history inputs, cross-review, actual evidence and unverified obligations |

## How this becomes 1.0

The implementation starts by freezing canonical examples, new format families and regression/model inventories. It then builds values/admission and LMDB ownership, the complete disk executor, the internal log machine, recovery/retention, and the TypeScript application workflow. Optimizations earn their place through equivalence and measurement.

The old audit has **47 indexed implementation observations**, plus architectural, operational, performance, assurance and unindexed boundary issues. Every one has an explicit successor obligation in [50](50-audit-closure-matrix.md). None is marked fixed by writing prose.

Before promotion, every required qualification gate must pass on the exact candidate artifacts and selected platform/backend matrix. That includes fresh Node and core C builds, independent semantic/history models, float/FPU bit parity, real S3 conditions, crash/restore/migration schedules, native resource reclamation, a physically populated database above 40 GiB, and a separate enforced larger-than-memory workload. Skips, missing credentials, zero matched tests and stale binaries cannot count as qualification. After authorized publication, verify the downloaded artifacts and clean install before declaring the release complete.

Full streamed checkpoints, single-tenant HEAD contention, inline text, exact numerical reductions and offline migration all have real costs. They must be measured against the intended apps before claiming the design is fast enough. A failure of those measurements calls for a visible design decision, not an undisclosed second architecture.

This proposal is ambitious about quality and conservative about mechanism count. Its definition of “amazing” is concrete: a model applications can trust, a backend used well, predictable ownership and recovery, strong performance in the intended regime, correct slowdown beyond memory, and evidence behind every shipped claim.
