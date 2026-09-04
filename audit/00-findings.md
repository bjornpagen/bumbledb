# Consolidated finding register

Date: 2026-09-04. Status of every entry: **open audit observation; no implementation fix made by this campaign**.

Use the detailed report as the authority for evidence, source locations, triggering conditions, and proposed regressions. This register groups work without replacing those qualifications.

## Severity and evidence

- **P1:** fix or explicitly narrow the contract before relying on the affected correctness, durability, isolation, or availability guarantee. A P1 contract decision is not automatically a demonstrated implementation defect.
- **P2:** important correctness edge, operational/scaling limitation, or contract/hardening gap. Not “optional forever.”
- **Reproduced:** an executed probe demonstrated the stated observation. A simulated transport outcome or injected clock is marked separately.
- **Static:** source establishes a concrete control-flow, ownership, adversarial schedule, or complexity argument, but the failure was not executed.
- **Contract/design:** an intentional or missing capability with product consequences. Resolve through an explicit contract and tests, not necessarily a new feature.

No P0 is assigned. Severity is conditional on the deployment and API involved, not a claim that every user immediately loses data.

## First repair packages

| Order | Invariant to establish | Findings to close together | Why isolated patches are insufficient |
| --- | --- | --- | --- |
| 1 | A published receipt remains part of recoverable history | REP-001/002/003/004/006/007/008/018; ENG-003 | Log retirement, allocation, snapshot identity, and retention share the same authority boundary |
| 2 | One command has one immutable meaning; ordinary reads see only published state | SDK-001/003/014/015; REP-016 / SDK-008 | Hiding direct writes does not hide candidates; copying bytes does not resolve an earlier pending command |
| 3 | Only the live tenant/database owner can read, mutate, recover, or delete its state | REP-005/009/010/011/017; SDK-002/004/005/006/007/013/016 | A logical close without native release or a lease check without an enforced fence is incomplete |
| 4 | Publicly admitted data and outcomes satisfy the stated semantic contract | ENG-001/002/004/005/007/008; QRY-001 | Trusted value paths, provisional identity, complete diagnostics, and failure outputs need explicit boundaries |
| 5 | Work, memory, cleanup and application retries are bounded and operable | Remaining P2 entries; ARCH/OPS requirements | Per-tenant hosting must bound total cost and define ambiguous outcomes, not just final row semantics |

These are work packages, not a strict instruction to finish all of package 1 before starting package 2. Agree on shared invariants first, then implement independent repairs in parallel. The detailed [roadmap](60-roadmap.md) names dependencies and acceptance criteria.

## Replication, persistence, and recovery

Source: [10 — Replication/storage](10-replication-storage.md); executed evidence: [11](11-replication-test-evidence.md).

| ID | Priority | Observation | Evidence / important condition |
| --- | --- | --- | --- |
| REP-001 | P1 | Old writer recreates a retired log slot and returns invisible `Published` success | Reproduced with actual Writer/Replica after checkpoint and GC; publication needs enforced retirement authority |
| REP-002 | P1 | Scalar checkpoint sum permits a recovery component to move behind collected history | Reproduced with valid incomparable snapshots and fresh recovery |
| REP-003 | P1 | Clock equality switches retention into immediate deletion of prior checkpoints | Reproduced, including default newly opened Checkpointer; no malicious clock needed |
| REP-004 | P1 | Store-level ambiguity proof grants equal counter ranges to two callers | Reproduced helper composition with injected S3-style ambiguity; no real AWS fault run |
| REP-005 | P1 | Expiring lease check followed by rename is not a fenced filesystem CAS | Static pause/resume schedule; includes lease-head regression race |
| REP-006 | P1 | Numeric writer identity permanently fences lower-ID healthy counter allocators | Reproduced unchanged retries, stopped by bounded harness; not ordinary contention |
| REP-007 | P1 | Scratch recovery deletes a published checkpoint still reachable as a predecessor | Reproduced crash-residue/advanced-head state |
| REP-008 | P1 | Delayed checkpoint retains stale backlink and skips an intervening publication | Reproduced; retained history becomes undiscoverable through the spine |
| REP-009 | P1 | Failed competing open can clean up the live owner's scratch before exclusivity | Static live-publisher schedule |
| REP-010 | P1 | Filesystem body and fencing generation persist separately | Static crash/error schedule; missing/bad generation can fall back to zero |
| REP-011 | P1 | Case-distinct logical keys/tenant paths alias on case-insensitive filesystems | Reproduced on this Mac, including actual cross-tenant cache exposure; platform-specific trigger |
| REP-012 | P2 | Each local lease acquisition scans lifetime predecessor history | Static complexity: linear per acquisition, quadratic cumulative work |
| REP-013 | P2 | GC repeats scans from slot 1 and lacks durable object age state | Static complexity/retention design gap; overlaps REP-003's underlying retention redesign |
| REP-014 | P2 | Checkpoint publication can require a whole-database quiet window | Static progress condition; sustained writes and synchronous join can obstruct maintenance/shutdown |
| REP-015 | P2 | Catch-up, wait, and recovery have no work/deadline/cancellation boundary | Static public API/liveness limitation; SDK-010 is the TS companion |
| REP-016 | P1 | Replicated read handles expose the underlying write-capable database | Static Rust public API, TS counterpart reproduced as SDK-008 |
| REP-017 | P1 | Tenant eviction releases ownership before closing/deleting protected state | Static successor race |
| REP-018 | P2 | Checkpoint digest identity is not verified; replay can skip checkpoint boundary audit | Static integrity checks missing; not a demonstrated honest-store data-loss event |
| REP-019 | P2 | Failed orphan deletion loses its discovery record | Static failure schedule; also fails across loss of the local host |
| REP-020 | P2 | Split commit discards earlier successful receipts after a later infrastructure error | Static API result loss; earlier publication may remain durable |

## Semantic engine and query execution

Sources: [20 — Engine](20-engine-semantics.md), [21 — Query](21-query-runtime.md); external harness: [22](22-engine-test-evidence.md).

| ID | Priority | Observation | Evidence / important condition |
| --- | --- | --- | --- |
| ENG-001 | P1 | Public safe unchecked interval constructor admits a value ordinary reads reject as corruption | Reproduced; generated hidden constructor is downstream-callable |
| ENG-002 | P2 | Extensible safe `Fact` codec is treated as a canonical-value certificate | Reproduced malformed custom codec; trusted-implementer contract gap, not a Rust UB claim |
| ENG-003 | P1 | Concurrent compaction captures metadata and rows from different snapshots | Reproduced in ten of ten copies; catalog ownership separately prevents foreign-witness reuse |
| ENG-004 | P1 | ID exposed inside an interrupted closure can be reissued after reopen | Reproduced abrupt exit; no committed-ID reuse shown; resolve provisional versus durable-reservation contract |
| ENG-005 | P2 | Complete key diagnostics omit conflicts among fresh-refused proposed rows | Reproduced; transaction still rejects |
| ENG-006 | P2 | Deleting text and compacting retains the live dictionary entry | Reproduced; intentional append-only policy with space/erasure consequences |
| ENG-007 | P2 | Semantic rejection hides an error persisting escaped-ID burn | Static; in-process pending marks mitigate later writes but not the omitted failure signal |
| ENG-008 | P2 | Hidden no-sync constructors are public in ordinary builds | Static capability boundary; normal default opens still sync |
| QRY-001 | P2 | Aggregate error leaves plausible partial results in reusable Answers | Reproduced; define all-or-error or explicitly marked partial-result contract |
| QRY-002 | P1 | Tuple budget checks happen after materialization and do not bound general query resources | Static; P1 for shared hosts, no destructive OOM experiment run |
| QRY-003 | P2 | Documented host-settable query budgets have no public setters | Static public surface mismatch |

## TypeScript, native boundaries, and tenant hosting

Sources: [30 — SDK/hosting](30-sdk-hosting.md), [31 — FFI/packaging](31-ffi-packaging.md); reproduction sources: [32](32-sdk-test-evidence.md).

| ID | Priority | Observation | Evidence / important condition |
| --- | --- | --- | --- |
| SDK-001 | P1 | Next live commit overwrites unresolved Pending after failed publication | Reproduced; second acknowledged commit diverges from local state, first recovery evidence lost |
| SDK-002 | P1 | Existing writer can publish after replica disposal | Reproduced |
| SDK-003 | P1 | Caller mutates byte cell across an await: judged/applied and published facts diverge | Reproduced with ordinary Uint8Array; escaped recorder companion path is static |
| SDK-004 | P1 | Shared tenant handle makes double/stale release consume another borrow | Reproduced; async disposal also closes shared replica instead of returning the borrow |
| SDK-005 | P1 | Pool finishes closing while an in-flight open later returns a live replica | Reproduced |
| SDK-006 | P1 | Lost tenant lease is treated as successfully renewed | Reproduced using controlled clock injection; not an actual process-pause test |
| SDK-007 | P1 | Logical disposal does not deterministically close the native database | Static intentional public API choice; actual resource release is not guaranteed by wrapper disposal |
| SDK-008 | P1 | Public replica database accepts unlogged local writes later erased by refresh | Reproduced; same capability defect as REP-016, not an independent root bug |
| SDK-009 | P2 | Async commit callback can await its own replica gate indefinitely | Static reentrancy/deadlock path |
| SDK-010 | P2 | Open/refresh/recovery/wait have no coherent cancellation or work limits | Static; companion/overlap with REP-015 |
| SDK-011 | P2 | Tenant resource budget is advisory, after allocation, and incomplete | Static documented limitation; not hard host admission control |
| SDK-012 | P2 | Local lease cleanup cost grows with lifetime acquisitions | Static; companion/overlap with REP-012 |
| SDK-013 | P1 | C destroy permanently retains engine-owning retired read handles | Static retained/leaked Arc; no C sanitizer or process reopen reproduction run |
| SDK-014 | P1 | Ordinary concurrent read sees candidate that eventually returns rejected | Reproduced; read-only capability by itself does not fix this |
| SDK-015 | P2 | ID cache refill re-executes callback, risking repeated side effects or exhausted iterables | Static intentional mechanism; distinct from conflict rejudgment, which does not rerun the host body |
| SDK-016 | P1 | Reused same-schema local cache serves facts from the wrong tenant namespace | Reproduced across two ordinary processes; initiated by misconfigured reuse, with REP-011 as separate alias trigger |

## Architectural and strategic work—not extra independent bug counts

| IDs | Subject | Decision or deliverable |
| --- | --- | --- |
| ARCH-001 | Schema-valid effects versus read-dependent command intent | Optional published-state witness/expected-fact contract; preserve cheap blind writes |
| ARCH-002 | Braids versus application causality | Explicit valid-prefix/session/read dependency contract; no inferred global transaction |
| ARCH-003 | End-to-end retry identity | Named command receipts and deduplication horizon |
| ARCH-004 | Session provenance and database incarnation | Bind transferred tokens and caches to logical history; validate before waiting/recovery |
| ARCH-005 | Duplicate host transition machines | Shared schedule corpus, then evaluate a pure transition core |
| ARCH-006 | Real schema contention domains | Measure schema components; braids are not row sharding |
| OPS-001–006 | Migrations, recovery authority, blobs/effects, tenant isolation, read status, observability | A tested operating contract, not a claim these are six newly found corruption bugs |
| PERF-001–005 | Image work, retained memory, commit amplification, maintenance progress, host scheduling | Measured warm/cold/hot/fleet workload envelope |
| ASS-001 | Closed-relation braid theorem mapping | Match actual decomposition to actual proof premises; not evidence partitioning is unsound |
| ASS-002 | History-testing coverage | Independent state-machine and process-boundary test campaign |
| ASS-003–004 | Stale specifications and lost audit evidence | Current runbooks plus durable finding/resolution history |

## Closing a finding

Do not close an entry because a nearby test passes or a comment now explains the intended behavior. Record:

1. The exact guarantee chosen, including narrowed supported conditions.
2. A regression that fails on this audited implementation and asserts the desired outcome after repair.
3. Fix commit and affected language/backend variants.
4. Clean-directory recovery, lifecycle, and error-path checks appropriate to the change.
5. Remaining limits and a skeptical closure review.

If the original observation is disproved, retain it with the counterevidence. If it was a deliberate choice, make that choice public and testable. Do not delete the record or silently replace a strong claim with a weaker one.
