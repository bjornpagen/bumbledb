# Bumbledb: adversarial audit and architectural direction

**2026-09-04 · current working tree · documentation only**

The core idea deserves to survive this review: a tenant-sized, set-semantic application database whose declared laws drive admission, explanations, query execution, and coordination. The current implementation contains real substance behind that idea. The recommendation is to strengthen it, not restart it.

**The hosted durability and isolation contract is not ready to rely on for irreplaceable application data.** Focused probes found published writes that disappear from fresh replicas, unsafe checkpoint/retention behavior, local/log divergence, speculative reads that escape before rejection, and cross-tenant cache exposure. Several issues are ordinary failure-path or lifecycle behavior, not exotic malformed-input attacks.

This audit contains **47 indexed implementation observations**, plus architectural, operational, performance, and assurance analyses. That is not a claim of 47 independent bugs: three SDK entries explicitly overlap replication findings, and some entries are contract choices or scaling limitations. Each report separates executed reproductions, static schedules, and proposed requirements. See the [consolidated register](00-findings.md).

## Start here

1. [The philosophy and design constitution](01-philosophy.md): what is genuinely distinctive, where the claims overreach, and the coherent product this can become.
2. [Prioritized findings](00-findings.md): the release blockers and the full indexed register.
3. [Architecture and guarantee boundaries](02-architecture-and-guarantees.md): schema validity versus application intent, publication, sessions, and braid independence.
4. [Dependency-ordered roadmap](60-roadmap.md): repair shared invariants first, with explicit exit criteria.
5. [Design decisions to settle](91-decisions.md): recommended defaults and the tradeoffs they commit the project to.

## What changed the assessment

| Executed observation | Why it matters |
| --- | --- |
| An old writer returned `Published` at a collected slot; a fresh checkpoint-seeded replica never saw its fact | Successful publication is not yet a reliable end-to-end recovery obligation: REP-001 |
| A checkpoint moved from `[0,2]` to `[3,0]` after collection; fresh recovery lost acknowledged B history | Recovery floors need componentwise dominance, not a larger sum: REP-002 |
| A newly opened checkpointer with default 90-day retention deleted a predecessor created 1.1 seconds earlier | Retention cannot depend on process-local clock sentinels: REP-003 |
| Mutable input bytes produced different local and published rows; another probe exposed a candidate that later rejected | Immutable command ownership and published read visibility are missing boundaries: SDK-003/014 |
| Opening B with A's same-schema/equal-vector cache served A's facts; case aliases independently caused tenant leakage | Local state needs authoritative database identity and safe directory ownership: SDK-016, REP-011 |
| Concurrent compaction produced data and generation metadata from different snapshots | A snapshot certificate must name exactly the state it contains: ENG-003 |

Other severe findings include duplicate ID ranges under injected S3-style ambiguity, pending-state overwrite, filesystem fencing races, borrowed-handle lifetime errors, and native engine retention. Conditions and evidence limitations are in the reports; the S3 ambiguity probe did **not** contact AWS.

## Detailed reports and evidence

| Area | Report | Supporting evidence / next tests |
| --- | --- | --- |
| Replication, checkpoints, GC, stores, leases | [10 — Replication/storage](10-replication-storage.md) | [11 — Ten executed scenarios and full harness](11-replication-test-evidence.md) |
| Engine admission, identity, compaction, text lifetime | [20 — Engine semantics](20-engine-semantics.md) | [22 — Engine harness](22-engine-test-evidence.md) |
| Query correctness, budgets, execution model | [21 — Query runtime](21-query-runtime.md) | [51 — Regression campaign](51-test-campaign.md) |
| TypeScript writer, visibility, tenants, lifecycle | [30 — SDK/hosting](30-sdk-hosting.md) | [32 — SDK reproduction sources](32-sdk-test-evidence.md) |
| C ABI, native ownership, packaging | [31 — FFI/packaging](31-ffi-packaging.md) | Static ownership analysis; sanitizer/process tests still required |
| Actual application performance envelope | [40 — Performance](40-performance.md) | Proposed workload matrix; no new benchmark claims |
| Proof/test/specification boundaries | [50 — Assurance](50-assurance.md) | [90 — Scope and validation record](90-evidence.md) |
| Per-tenant deployment and operations | [03 — Production contract](03-production-contract.md) | Restore, migration, erasure, failure, and resource drills |

## Positive evidence—and its limit

The workspace-locked Rust run passed **2,049 tests**, with 30 skipped. The reviewers' selected TypeScript/Node suites passed **209 tests**. Lean built and the three-way conformance lane reported **277 cases, zero disagreements**. Formatting and workspace Clippy also passed. Exact commands, artifact provenance, and excluded environments are in [90 — Evidence](90-evidence.md).

Passing those gates is meaningful. It also shows that the missing tests are about histories and boundaries, not simply a shortage of ordinary examples: subsequent use after a failed publish, old owners after retention, readers during speculative writes, namespace changes with equal vectors, and cleanup after historical success.

## Recommended direction

Build one explicit **tenant capsule**: admitted theory, immutable command, published read frontier, recoverable receipt, bound database incarnation, deterministic resource ownership, conservative retention, and bounded host work. Keep blind set writes simple; add explicit preconditions and named-command receipts where application intent requires them.

Do not start with a rewrite, more aggressive concurrency, a new storage engine, or historical footprint optimizations. First make the current publication and recovery promises true. Then test a few real application schemas at cold, warm, hot, and fleet scale.

No production code or existing tests were edited. Existing user changes were preserved. This is a broad multi-reviewer audit with targeted falsification, **not** exhaustive verification or a guarantee that all bugs have been found. Preserve these files as the dated baseline; append resolution evidence rather than deleting findings after a fix.
