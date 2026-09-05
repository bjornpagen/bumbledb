# Behavioral obligations (permanent reference)

This page is the authoritative human-readable roster for release
obligations. The machine-readable roster lives in
[`obligation-inventory.json`](obligation-inventory.json). Both are
authored here; nothing is scraped from disposable proposal Markdown.

**Read this correctly:** the 220 names are obligations, not 220 tests or
tasks. One small independent model case may cover several rows; one
qualification report may support many IDs. Every retained assertion must
reject a plausible incorrect implementation — not merely prove that the
program ran. Unresolved obligations stay listed; they are not dropped as
cleanup.

None of these IDs is qualified by documentation transfer. Outcomes stay
**NotRun** until the post-retirement candidate produces discriminating
evidence.

## Namespaces

| Namespace | Examples | Role |
| --- | --- | --- |
| Audit (68) | `REP-001` … `ASS-004` | Historical closure requirements from `audit/` |
| Prior review (78) | `CORE-001` … `REVIEW-004` | Continuity obligations from the source review |
| Parent gate | `G00` … `G16` | Release threshold families |
| Child family (220) | `E-ADMIT`, `PROTO-07`, `APP-MUTATE` | Concrete behavioral schedules |
| Discriminator | `D01` … `D29` | Authored sensitivity cases in [release-gates.md](release-gates.md) |

## Scope that stays mandatory

Pre-1.0 import/compatibility is not supported: `MIG-06` / `PKG-05`
require early untouched refusal, not a legacy engine. Generated
within-family migrations, backup/replay/corruption checks remain
required. `HASH-04` is the selected single BLAKE3 decision plus
actual-input/layout qualification; AEGIS experimentation is optional.
Missing real S3/Graviton is unqualified, not `NotApplicable`.

## Seven commitments

| Commitment | Production representation / deletion |
| --- | --- |
| Compile once | Interned theory/row/scalar program used by storage, judgment and planner; no parallel interpretation |
| Delta-local application work | Exact affected-group indexes and witnesses; independent reference semantics |
| Retention equals ownership | Inseparable admitted capacities, shared token generations, bounded complete scratch/native lifecycle |
| Earn Free Join dividends | Preserve COLT/factoring/SIMD; narrow distinct/existence/fold witnesses; bounded real loops |
| Thin correct authority | Evidence-bearing attempt, coherent writer parent, strict locators and private staged lifecycle |
| One application API | Core primitives imported by log; Effect-only operations; typed generated migrations and actual app |
| Subtract and qualify | Delete superseded mechanisms and pointless tests; exact source/artifact/target evidence |

## Original audit obligations (68)

Names and 220 child schedules are stable coverage identities, not a
requirement for a separate test file per ID.

| Audit ID | Required successor disposition | Blocking families |
| --- | --- | --- |
| `REP-001` | Delete vacant per-braid publication slots. A never-reused HEAD and epoch-bound staged dependencies are the only hosted publication path | G07/G08/G10; `PROTO-01`; GC barrier |
| `REP-002` | Delete scalar-sum ordering of vector recovery floors. One tenant order; checkpoint boundary must be an authenticated ancestor of the current tip | G07/G10 |
| `REP-003` | Delete clock-equality retention mode and implicit 90-day promise. Retain explicit named roots; release is an explicit authority operation | G10 |
| `REP-004` | Delete the entire database entity allocator and FreshRef representation. Applications seal concrete 128-bit IDs | G08/G09; `PROTO-04`/`PROTO-11` |
| `REP-005` | Delete expiring filesystem mutation leases. Kernel-held local ownership or actual S3 conditional replacement enforces authority | G06/G08/G11 |
| `REP-006` | Delete writer-number fencing of a shared allocator | G07/G09 |
| `REP-007` | Scratch is staging evidence, not remote deletion authority. Only rooted epoch GC may remove objects | G10 |
| `REP-008` | Bind checkpoint publication to the actual captured/revalidated head and ancestor suffix | G07/G10 |
| `REP-009` | Acquire process-lifetime directory ownership before any cleanup or cache recovery | G06/G11 |
| `REP-010` | Remove separately persisted filesystem body/fence authority. LocalHistory uses one LMDB transaction; hosted HEAD uses qualified atomic CAS | G06/G08 |
| `REP-011` | Canonical collision-safe local names plus checked origin/database/incarnation binding before adoption | G10/G11/G14 |
| `REP-012` | Remove historical token-file chains and predecessor scans | G11/G12 |
| `REP-013` | Replace slot-1 rescans and process-local ages with epoch inventory/mark/sweep and durable bounded progress | G10/G12 |
| `REP-014` | Capture one coherent snapshot; publish it with a validated current suffix without recompacting on every intervening write | G06/G10/G15 |
| `REP-015` | Every refresh captures a finite tip; every wait/replay/resolve consumes an execution budget | G07/G12; `PROTO-13`/`PROTO-18` |
| `REP-016` | Log returns a published read capability, never the underlying writable core owner | G07/G11 |
| `REP-017` | Hold ownership through operation drain, native close and protected cleanup; only then release | G08/G11 |
| `REP-018` | Typed ObjectRef verifies kind/epoch/length/digest; snapshot and replay certificates bind exact boundaries | G02/G06/G10/G14 |
| `REP-019` | Deletion progress is durable and retryable; failed deletion never discards sole discovery evidence | G10 |
| `REP-020` | Remove split commit: one tenant command is atomic. Terminal named receipt contains one outcome | G07/G09; `PROTO-10` |
| `ENG-001` | Remove public unchecked interval construction; all typed/constant/dynamic inputs use checked canonical primitives | G02/G03; `E-VALUE` |
| `ENG-002` | Replace trusted safe raw `Fact` codecs with typed field encoding or checked external bytes | G02/G03; `E-CODEC` |
| `ENG-003` | OwnedSnapshot holds one real read transaction; content, generation and attachment derive from it | G06; `E-SNAPSHOT` |
| `ENG-004` | Remove core escaped reservations and log FreshRefs; entity IDs are ordinary owned input | G09; `E-NO-RESERVE` |
| `ENG-005` | Judge the proposed relation/multimap independently of physical unique-index installation | G03; `E-ADMIT` |
| `ENG-006` | Remove mandatory immortal text dictionary; live tuples own canonical text | G03/G06/G10; `E-TEXT` |
| `ENG-007` | Remove fresh-ID burn machine; do not flatten infrastructure failure into semantic rejection | G03/G06/G09 |
| `ENG-008` | Remove hidden no-sync constructors from ordinary production capability | G01/G06 |
| `QRY-001` | Results are owned complete values or an explicit error; scratch is not a published answer | G04/G12 |
| `QRY-002` | Primary warm Free Join/selective index paths, bounded LMDB fallback and one RAM/LMDB scratch map | G04/G05/G12/G15 |
| `QRY-003` | One public execution policy carried through all bindings, with effective values observable | G01/G12 |
| `SDK-001` | One Rust machine owns immutable unresolved attempts; a new command cannot overwrite them | G07/G09 |
| `SDK-002` | Shared lifecycle authority checked inside queued operations; close revokes admission | G07/G11 |
| `SDK-003` | Core ChangeSet owns/normalizes inputs during bounded Effect ingestion | G02/G07/G14 |
| `SDK-004` | Every acquisition returns a distinct generation-bound idempotent borrow | G11 |
| `SDK-005` | Owner/pool close accounts for opening operations as real owned work | G11 |
| `SDK-006` | Remove expiring directory renewal from ownership; process-lifetime OS lock and explicit capability revocation | G08/G11 |
| `SDK-007` | Deterministic native owner close, with explicit outstanding operation/snapshot policy | G11/G12 |
| `SDK-008` | Same root fix as `REP-016`: published read capability without raw writable Db | G01/G07/G11 |
| `SDK-009` | No arbitrary async callback under the serialized commit gate | G01/G07/G11 |
| `SDK-010` | Same root fix as `REP-015`: finite targets and propagated WorkContext | G07/G12 |
| `SDK-011` | Cache/query budgets account for real resources; pressure uses LMDB | G05/G11/G12 |
| `SDK-012` | Same root fix as `REP-012`: no lifetime lease-token chain | G11/G12 |
| `SDK-013` | Delete the entire public C API and its callback/tombstone mechanism | G00/G11/G13; `FFI-*` |
| `SDK-014` | Private uncommitted LMDB candidate plus published snapshot capability | G06/G07 |
| `SDK-015` | No callback replay, placeholder allocator or refill retry machine | G01/G09 |
| `SDK-016` | Verify configured origin and authoritative incarnation before using cache, pending work, or scratch | G10/G11/G14 |
| `ARCH-001` | Explicit ExactState precondition for read-dependent commands; blind set writes remain explicit | G07/G09 |
| `ARCH-002` | Tenant total order replaces product-of-braid-prefix read semantics | G07/G09 |
| `ARCH-003` | Named commands and retained receipt epochs; permanent refusal after retirement | G09/G10 |
| `ARCH-004` | Database/incarnation, command/decision/state identity and local origin binding are separate | G09/G10/G14 |
| `ARCH-005` | One internal Rust log state machine; TypeScript-only public log product; Rust/TS core only | G07/G13 |
| `ARCH-006` | No braids-as-row-sharding story; one tenant is a write authority | G15 |
| `OPS-001` | TypeScript schema SDK generates canonical migration plan/history; log owns freeze, staged execution, admission and one final new incarnation | G10/G13 |
| `OPS-002` | Log backup/restore primitives and an independent protected recovery root in the deployment runbook | G08/G10 |
| `OPS-003` | Immutable blob first, reference/receipt commit second; application outbox and idempotent effect dispatcher | G09/G10/G13 |
| `OPS-004` | Actual owner/borrow/resource boundaries and host authentication-to-tenant mapping | G11/G12/G14 |
| `OPS-005` | Explicit cached/refreshed/minimum-decision reads, published snapshot provenance and typed unavailable state | G07/G10/G12 |
| `OPS-006` | Structured counters/status already at command, snapshot, owner and GC boundaries | G10/G11/G14 |
| `PERF-001` | No compulsory relation-sized image rebuild after each write; preserve primary warm Free Join and bounded disk fallback | G05/G15 |
| `PERF-002` | Distinguish map size, file size, resident cache, plans/results and work; deterministic release and LMDB-backed scratch | G05/G11/G12/G15 |
| `PERF-003` | Count complete named-decision path, retries and checkpoint costs, not one winning PUT | G15 |
| `PERF-004` | Coherent streamed checkpoint with validated suffix and bounded progress | G06/G10/G15 |
| `PERF-005` | One bounded worker adapter for all core/log TypeScript work through Effect | G11/G12/G15 |
| `ASS-001` | Rewrite actual semantic proof premises; old braid theorem is not used to certify the new log | G03/G07 |
| `ASS-002` | Independent history model (`history_model.rs`), deterministic failures and real process/backend tests | G07/G08/G10 |
| `ASS-003` | Rewrite current docs/examples for selected contract; label historical research | G01/G13 |
| `ASS-004` | Preserve audit evidence and add immutable fix/test/decision records | G00/G16 |

## Prior-review obligations (78)

The fourth column is a discriminator or inherited gate, not a claim that
only that one test is sufficient.

| ID | Required closure / source repair to preserve | Discriminators |
| --- | --- | --- |
| `CORE-001` | Shared field validity including fixed widths and intervals; repaired walker needs all decoder paths | D04/D19 |
| `CORE-002` | Bounded exact competitor visitor; landed key streaming still needs integration | D03/D04 |
| `CORE-003` | DecodedRow cannot shed charge through owning extraction | D01 |
| `CORE-004` | Reserve before growth, bounded no-output polls, propagate final quantum | D01/D08 |
| `CORE-005` | All sink banks and retained pools own real capacity | D01/D09/D11 |
| `CORE-006` | Shared bounded text generation and complete nonresident resolver | D02 |
| `CORE-007` | Every derived/negative/recursive consumer supports sealed scratch backing | D09 |
| `CORE-008` | Admit collection/page conversion before growth or cursor advance | D12/D25 |
| `CORE-009` | Transactional retry-safe retained and physical scratch accounting | D03 |
| `CORE-010` | Choose cursor regime before resident u32 bounds; no global DB-size limit | D07/D09 |
| `CORE-011` | Affected containment/capacity groups consume compiled adjacency | D04 |
| `CORE-012` | Intern useful compact projections; qualify real persisted keys | D04 |
| `CORE-013` | One store cache, safe generations and actual retention accounting | D02 |
| `CORE-014` | Pinned same-thread read cannot deadlock map resize | D07 |
| `CORE-015` | Actual freshness owner/full metadata check, not Copy marker | D06/D26 |
| `CORE-016` | Private staging, complete (not empty-delta incremental) final judgment and absent-or-complete installation | D06 |
| `CORE-017` | Explicit map ceiling is respected, not rounded above request | D07 |
| `CORE-018` | Current proof premises and empirical refinement (Bridge + correspondence; no dyn/wording census) | G03/G04 |
| `CORE-019` | Test semantics/locality, not mandatory inefficient full-table spill | D04 |
| `CORE-020` | One explicit scratch capability; no TypeId error reflection | D03 |
| `CORE-021` | Canonical witness selection before truncation and replay | D05 |
| `CORE-022` | Bound-key fallback seeks, early stop and one row layout | D10 |
| `CORE-023` | Wide Pack stable exact tokens/order plus unambiguous narrow mode | D11 |
| `CORE-024` | Exclusive owned scratch setup and failure cleanup | D03 |
| `CORE-025` | Preserve source repairs; verify consumers and current artifacts | D04/D07/D08 |
| `LOG-001` | State-specific certainty across every admin failure | D13 |
| `LOG-002` | Retirement validates captured parent under writer | D14 |
| `LOG-003` | Preserve repaired same-writer replay; qualify races | D14 |
| `LOG-004` | Control revision advances independently of decision/data | D14 |
| `LOG-005` | Coherent retained frontier; no loss proof after receipt retirement | D15 |
| `LOG-006` | Historical precondition derived from command/exact predecessor | D05/D14 |
| `LOG-007` | Nonempty-required cold hydration/resume uses private builder | D06 |
| `LOG-008` | Restore staging, correct genesis-root artifacts and no ready partial state | D06/D17 |
| `LOG-009` | Native chunk flow bounded transitively | D17 |
| `LOG-010` | Migration transform/state memory and work use core bounded substrate | D17/D20 |
| `LOG-011` | Receiving and waits carry caps/deadlines, not post-body checks | D17 |
| `LOG-012` | Preserve bounded listing; durable canonical progress, no prefix rescans | D17 |
| `LOG-013` | Strict direct locators including actual 49-byte encoding | D16 |
| `LOG-014` | Immutable S3 create exactness and retry evidence | D17/G08 |
| `LOG-015` | Preserve transactional local root registry; concurrency/crash evidence | D14/G10 |
| `LOG-016` | Every successful HEAD change strictly advances control revision | D14 |
| `LOG-017` | Receipt retirement works with no application decision advance | D14 |
| `LOG-018` | Finite cache tail defaults and work on actual opens | G11/G12 |
| `LOG-019` | Every acquire/close continuation retains a cleanup owner | D18/D24/D29 |
| `LOG-020` | Inspection reports unknown/unavailable truth, no invented zero health | D13/D15 |
| `LOG-021` | Bounded receipt/prune/ancestry visitors under coherent ownership | D14/D17 |
| `LOG-022` | Create versus explicit adopted genesis binds actual initial state | D06 |
| `LOG-023` | Shared transport runtime and actual refreshing credential chain | D17/D18 |
| `LOG-024` | Actual S3/IAM evidence; mocks cannot qualify backend | G08/D17 |
| `LOG-025` | Canonical rejection survives physical remint/spill | D05 |
| `LOG-026` | Preserve Rust command certainty repairs across adapters/retirement | D13/D15 |
| `LOG-027` | Preserve complete initial migration judgment on every path | D06/D20 |
| `LOG-028` | Preserve identity, directory fencing and stale capability refusal | D18/G14 |
| `LOG-029` | Never discard decided receipt when optional diagnostics fail | D13 |
| `TS-001` | Actual submitted work reaches snapshot/query/get execution | D07 |
| `TS-002` | Native registry owns payloads, close independent of GC | D18 |
| `TS-003` | Each partial acquisition owned before next interruption | D18 |
| `TS-004` | Bound host length/cell/chunk work before conversion | D07/D12 |
| `TS-005` | Bounded real native/JS result delivery and event-loop work | D12 |
| `TS-006` | Draft failures are terminal and close is joined | D07/D18 |
| `TS-007` | Cumulative draft budget includes all chunks and finish | D07 |
| `TS-008` | Fixed-worker owned resource tables replace parked/thread-per-session reactors | D18 |
| `TS-009` | Real prepared session reuse and joined independent close | D08/D18 |
| `TS-010` | Addon unavailable during pure metadata import | D22 |
| `TS-011` | One public Effect API; intentional shared internal seam | D07/D22 |
| `TS-012` | Shared scalar grammar with known query kinds and honest unresolved migration fields | D19/D27 |
| `TS-013` | Mandatory bound snapshots and full mapping compile before effects | D20 |
| `TS-014` | Kernel-held repository exclusion with stable inode, joined I/O, immutable staged files and one manifest commit | D21/D28 |
| `TS-015` | Bounded aggregate receiving, interruption/durability correctness | D21 |
| `TS-016` | Canonical boundary values, row types/arity and fixed widths | D19/G02 |
| `TS-017` | Typed operational failures, complete rejection details, Cause preserved | D13/D19 |
| `TS-018` | Real native-ledger-shaped application; sibling read-only | D22 |
| `TS-019` | Honest Notes/Alchemy/Rust examples, real witness and migration prefix | D07/D22 |
| `TS-020` | Actual fresh packed native artifacts, correct evidence and useful tests | D23/G13 |
| `REVIEW-001` | Count admitted benchmark work only; preserve landed rejection-unwrapping repair and verify truth | G15/D04 |
| `REVIEW-002` | Adjudicate historical failures/copied assumptions using independent tiny oracle; no pinned test census | D04/D11/G04 |
| `REVIEW-003` | One fresh build input, exact release evidence, no duplicate/stale runner work | D23/G13 |
| `REVIEW-004` | Current public docs/usage, correct dedup/durability/API/format claims | D22/G01 |

## Child families (220)

### Concurrency and set algebra

| Obligation | Required behavior / evidence |
| --- | --- |
| `CONC-01` | Independent finite-state checks and Lean lemmas for set normalization/idempotence and the stated raw-delta commutation condition |
| `CONC-02` | Executable key, grouped count/weighted-capacity and parent-delete/child-insert counterexamples |
| `CONC-03` | Real LMDB parallel readers/writers, held snapshots and resize barriers |
| `CONC-04` | Multiple hosted contenders, lost responses and repeated same IDs; one ordered terminal decision per named command |
| `CONC-05` | Mutable-support theorem premises tested against runtime law analysis |
| `CONC-06` | Measure multiwriter contention; no universal low-latency claim inferred from algebra |

### Core representation and admission

| Obligation | Required behavior / evidence |
| --- | --- |
| `E-DELTA` | All permutations/repetitions of add/remove; normalization idempotent; separate commands ordered |
| `E-VALUE` | Malformed bool, interval, fixed width and F64 rows rejected downstream |
| `E-CODEC` | Custom encoders cannot bless wrong relation, width, padding, text or type bytes |
| `E-SNAPSHOT` | Pause export/copy while concurrent commits proceed; one owned read transaction |
| `E-NO-RESERVE` | No reserve/FreshRef/entity allocator API; application-owned 128-bit bytes survive retries |
| `E-ADMIT` | Conflicting tentative rows report every violated statement; indexed and full judgment agree |
| `E-TEXT` | Unique text churn then delete/export/reopen leaves no live dictionary entry |
| `E-DURABILITY` | Ordinary production cannot select benchmark no-sync by accident |
| `E-VISIBILITY` | Candidate work never observed before publication |
| `E-ORIGIN` | Foreign-environment prepared state and mismatched origin refuse |
| `E-LARGE` | Actual contents exceed old 32 GiB boundary and allowed resident memory separately |
| `E-BRIDGE` | Proof premises name real production representations; discriminating refinement tests |

### Numeric, interval and proof correspondence

| Obligation | Required behavior / evidence |
| --- | --- |
| `F-CANON` | Canonical NaN `0x7ff8000000000000`, positive zero, and payload/key encodings agree across constructors and wire |
| `F-GOLDEN` | Independent oracle goldens; never derive expected bits from the implementation under test |
| `F-ORDER` | Relational order including NaN; min/max do not silently ignore NaN |
| `F-ARITH` | IEEE nearest-even then canonicalize; no reassociation, FMA or implicit numeric promotion |
| `F-ENV` | Host floating-control save/set/restore on required architectures |
| `F-AGG` | Exact sum/mean limb accumulation, cancellation, ties, subnormals, overflow and empty-vs-zero |
| `F-SET` | Query set grain, distinct entities with equal amounts, union at specified tuple grain |
| `F-OPT-NEG` | Optimizer/fusion negatives: outer filter cannot hide a required inner-stage error |
| `F-CROSS` | Cross-relation laws and compiled adjacency over shared projections |
| `F-WIRE` | One authoritative native/artifact codec; tagged JSON floats; no NaN→null |
| `F-RESOURCE` | Charge exact-accumulator limbs and group capacity before growth |
| `F-PROOF` | Current Lean premises map to live constructors; no deleted-path census as proof |
| `F-INTERVAL` | Half-open nonempty intervals; F64 endpoints; coalesce/gap; overflow/unbounded measure distinction |

### Query execution

| Obligation | Required behavior / evidence |
| --- | --- |
| `Q-ATOMIC` | Results are complete owned values or an explicit error; no apparently complete partial answer |
| `Q-BUDGET` | Public execution policy bounds real decode, keyed reads, execution and delivery |
| `Q-DISK` | Query exceeding RAM continues on disk; deadlines/storage failures stop safely |
| `Q-LARGE-STORE` | Actual >32 GiB populated data, not a sparse mapping with tiny contents |
| `Q-COLLISION` | Constant-hash collisions preserve exact wide keys and do not merge facts |
| `Q-FALLBACK` | Bound-key fallback seeks, early existence stop, one row layout per plan |
| `Q-RECUR` | Restricted positive linear recursion with bounded seen/frontier spill |
| `Q-GROUP` | Grouped overflow is an error; empty input emits no global aggregate row |
| `Q-TEMPORAL` | Interval overlap/pack/length use endpoint-order algorithms |
| `Q-LIFETIME` | Prepared session reuse; generation pin does not donate the first call's deadline |
| `Q-FAIR` | A small tenant progresses alongside a slow tenant under allowed fairness |
| `Q-IR` | Compiled IR/layout is per plan/row visit, not allocation per operand |
| `Q-INJECT` | Injected sink refusal or source error stops later scans/probes beyond the permitted chunk |

### Proof, representation and schedule families

| Obligation | Required behavior / evidence |
| --- | --- |
| `P-KERNEL` | Current proof kernels name live production representations |
| `P-SEMANTIC` | Empirical correspondence exercises current Rust behavior, not deleted filenames |
| `P-FLOAT` | Architecture-specific floating-control verification where required |
| `P-REPRESENTATION` | Recalculated raw row/membership/projection bytes from the actual layout |
| `P-DISK` | Live payload, pages, free blocks, churn and old snapshots reported separately from map/RSS |
| `P-MEMORY` | Retained result/plan/native owner accounting; no uncounted opening storm |
| `P-SCHEDULE` | Deterministic barriers, not arbitrary timing microbenchmarks, for ownership schedules |
| `P-ARTIFACT` | Exact artifact/data/flags/toolchain recorded with any performance claim |
| `P-PERF` | Compact scorecard in [performance.md](performance.md); no ritual overlapping jobs |

### Log authority

| Obligation | Required behavior / evidence |
| --- | --- |
| `PROTO-01` | Never-reused HEAD; paused old writer cannot publish behind retained history |
| `PROTO-02` | Materialized stamp describes exactly its committed facts and host records |
| `PROTO-03` | Certainty is state, not an error-name heuristic; terminal receipt survives later local failure |
| `PROTO-04` | Application-sealed 128-bit IDs; no allocator uniqueness theorem |
| `PROTO-05` | Historical replay derives ExactState from command/exact predecessor |
| `PROTO-06` | Rejection witness selection is canonical, not insertion-token order |
| `PROTO-07` | Proof ladder: matching receipt → decided; covered loss → proved lost; else unknown |
| `PROTO-08` | Changed HEAD plus absence after receipt retirement is not proved loss |
| `PROTO-09` | Independent history model traces intermediate observable snapshots |
| `PROTO-10` | One tenant command is atomic; terminal named receipt contains one outcome |
| `PROTO-11` | Create versus explicit adopted genesis binds actual initial state |
| `PROTO-12` | Control revision advances on every HEAD replacement, independently of data |
| `PROTO-13` | Every wait/replay/resolve consumes an execution budget |
| `PROTO-14` | Strict ObjectRef kind/epoch/length/digest; 49-byte current encoding |
| `PROTO-15` | Recovery/GC/backup stop at the authenticated base; no historical epoch probing |
| `PROTO-16` | Receipt retirement works with no application decision advance |
| `PROTO-17` | Inspection reports unknown/unavailable truth, no invented zero health |
| `PROTO-18` | Finite targets; concurrent writers cannot occupy a host indefinitely |
| `PROTO-19` | Relocated backup uses its manifest and unchanged historical commitments |
| `PROTO-20` | Decided receipt is never discarded when optional diagnostics fail |

### Store, local roots and GC

| Obligation | Required behavior / evidence |
| --- | --- |
| `STORE-01` | Private staging; destination is absent or the complete identified store |
| `STORE-02` | Complete final judgment; empty ChangeSet cannot borrow a lawful-parent premise |
| `STORE-03` | Two installers cannot overwrite each other; cleanup cannot delete the winner |
| `STORE-04` | Zero-row destination with host metadata is not fresh |
| `STORE-05` | One store cache, safe generations, actual retention accounting |
| `STORE-06` | Explicit map ceiling is respected, not rounded above request |
| `STORE-07` | Pinned same-thread read plus MapFull produces bounded progress, not self-deadlock |
| `STORE-08` | OwnedSnapshot holds one real read transaction |
| `STORE-09` | Compiled indexes persist shared projection identity and compact keys |
| `STORE-10` | Adoption binds complete actual initial digest, schema, lineage and snapshot |
| `LOCAL-01` | Transactional local root registry; crash-safe concurrency |
| `LOCAL-02` | Canonical collision-safe local names and origin/database/incarnation binding |
| `LOCAL-03` | Process-lifetime directory ownership before cleanup or cache recovery |
| `GC-01` | Rooted epoch GC only; scratch is not remote deletion authority |
| `GC-02` | Checkpoint boundary is an authenticated ancestor of the current tip |
| `GC-03` | Explicit named roots; no clock-equality retention or implicit 90-day promise |
| `GC-04` | Durable bounded progress; restart does not rescan all retired history forever |
| `GC-05` | Failed deletion never discards sole discovery evidence |
| `GC-06` | Every retained root has a discoverable complete closure |
| `GC-07` | Crash after publication, later checkpoint, reopen: retained dependencies remain recoverable |
| `GC-08` | Capture one coherent snapshot; publish with a validated current suffix |
| `GC-09` | Checkpoint completes under sustained permitted load or reports a bounded resource condition |
| `GC-10` | Protected recovery root cannot be deleted by ordinary GC/writer roles |
| `GC-11` | Receipt/prune/ancestry visitors stay bounded under coherent ownership |
| `GC-12` | Erase tombstones authority honestly about residual copies |
| `GC-13` | Repeated ownership/open cycles have cost tied to current work, not lifetime acquisition count |

### Filesystem, S3, recovery, backup, restore

| Obligation | Required behavior / evidence |
| --- | --- |
| `FS-01` | Kernel-held local ownership; suspended owner cannot coexist with a successor |
| `FS-02` | One LMDB transaction is local authority; no split body/fence files |
| `FS-03` | An opener refused ownership makes no changes to the active owner's files |
| `FS-04` | Old disposal cannot erase or close a successor's directory/environment |
| `FS-05` | Growing-file reads stop at the receiving/aggregate bound on the same opened descriptor |
| `S3-01` | Immutable create exactness and retry evidence on the real provider |
| `S3-02` | Conditional replacement ambiguity, lost response and immutable conflicts |
| `S3-03` | Missing-versus-denied, pagination, redirects/retries and provider refresh |
| `S3-04` | Transport reports observation; it does not decide command absence |
| `S3-05` | Receiving enforces digest and declared body size before full buffering |
| `S3-06` | Shared transport runtime does not share tenant authority; credentials refresh |
| `REC-01` | Fresh recovery reconstructs captured tip and every receipt promised by current retention |
| `REC-02` | Process death at lifecycle boundaries preserves old authority or a resumable matching new target |
| `REC-03` | Cold-open/resume of nonempty-required targets uses the private builder |
| `REC-04` | Recovery/GC/backup/witness traversal stops at the authenticated base |
| `REC-05` | Unresolved publication remains unknown, resolvable with original identity |
| `REC-06` | Independent history model plus real process/backend tests, not only final bytes |
| `REC-07` | Restore after loss of local cache/active namespace from the protected root |
| `BACKUP-01` | Independent verified bytes, not an active-store pointer |
| `BACKUP-02` | Verification is a separate typed read-only step |
| `BACKUP-03` | Relocated backup uses its manifest and unchanged historical commitments |
| `BACKUP-04` | Data-plane credentials cannot delete the protected recovery root |
| `BACKUP-05` | Streamed >RAM backup/results stay bounded |
| `RESTORE-01` | Restore creates a new writable incarnation; it never mutates the source lineage in place |
| `RESTORE-02` | Application Id128 values and applied migration history are preserved |
| `RESTORE-03` | Old bindings keep refusing with a lineage mismatch rather than silently serving the wrong incarnation |

### Migration and erase

| Obligation | Required behavior / evidence |
| --- | --- |
| `MIG-01` | Generate/verify refuses before writing a new authoritative manifest or freezing source |
| `MIG-02` | Every required intermediate source/target is bound and compiled |
| `MIG-03` | Valid prefix retry appends only the intended suffix |
| `MIG-04` | Invalid destination never becomes current; old writer refuses |
| `MIG-05` | Plan edits and history divergence refuse |
| `MIG-06` | Pre-1.0 import/compatibility is early untouched refusal, not a legacy engine |
| `MIG-07` | Kill at each cutover step; process death leaves old authority or resumable matching target |
| `MIG-08` | MapSpill::finish must not reconstruct its entire scratch result as Rows/BTreeMap |
| `MIG-09` | Symbolic field arithmetic constructs; missing/wrong-kind source fields refuse before effects |
| `MIG-10` | Empty-input checking still compiles and refuses invalid mappings |
| `MIG-11` | Activate is an explicit one-time cutover; repeated activation returns the recorded outcome |
| `MIG-12` | Abort before activation fences the target first, then thaws the matching frozen source |
| `MIG-13` | Uncertain cancellation leaves the source frozen; a cancelled operation cannot resume |
| `MIG-14` | No handwritten migration callback escape |
| `ERASE-01` | Ordinary lookups refuse after erase; explicit retained roots remain honored |
| `ERASE-02` | Report is honest about residual copies that require their own erasure decisions |
| `ERASE-03` | Erasure of the protected recovery root is a separate deliberate act |
| `ERASE-04` | Failed deletion never discards sole discovery evidence |
| `OPS-TEST-01` | Persist operator-minted operation ID before the first attempt; resolve uncertainty with the same ID |
| `OPS-TEST-02` | Runbook procedures name the shipped admin API; representative failures remain diagnosable |

### SDK, runtime, FFI, packages

| Obligation | Required behavior / evidence |
| --- | --- |
| `API-01` | One public Effect API; no Promise/sync/disposal twin |
| `API-02` | Pure metadata remains synchronous and addon-free |
| `API-03` | ChangeSet is immutable after seal; caller buffer mutation cannot change persisted meaning |
| `API-04` | QueryReader is the published read capability; no writable Db escapes |
| `API-05` | Certainty lives in the success channel; interruption stays in Cause |
| `API-06` | Typed tagged errors; no wrapper error family in maintained code |
| `API-07` | One ManagedRuntime at the application boundary |
| `API-08` | Explicit WorkContext on ordinary Rust operations; no unlimited twin |
| `API-09` | collect/pages are fresh bounded delivery operations |
| `API-10` | Session close leaves parent usable; repeated close joins one transition |
| `API-11` | Shared scalar grammar across query and generated migration compilation |
| `API-12` | Common interfaces imported from core; no log aliases for the same types |
| `RUN-01` | Native registry owns payloads; close is independent of GC |
| `RUN-02` | Each partial acquisition is owned before the next interruption |
| `RUN-03` | Fixed workers; idle snapshot entries do not park their scheduler |
| `RUN-04` | One-worker open/read/close with many idle snapshots still completes |
| `RUN-05` | Concurrent operations cannot use stale-generation handles |
| `RUN-06` | Abandoned outputs have cleanup owners |
| `RUN-07` | Close revokes new admission and reports honest remaining resources until joined |
| `RUN-08` | No heavy JS-thread destructor or per-session OS-thread growth |
| `RUN-09` | Directory-acquire → Db-open is not one interruptible acquisition with a late finalizer |
| `RUN-10` | Draft failures are terminal; cumulative draft budget includes finish |
| `RUN-11` | Result backing and queued conversion overlap are correctly charged |
| `RUN-12` | Independent caps intersect: maxBytes cannot enlarge work.resultBytes |
| `RUN-13` | Worker table/tombstone count returns to the admitted baseline across cycles |
| `RUN-14` | No payload closure, destructor, I/O or callback under the registry mutex |
| `RUN-15` | Deadline is a deadlock safety net, not the success assertion |
| `FFI-01` | No public C crate, header, example, workflow or ABI artifact |
| `FFI-02` | Node N-API linkage is an implementation detail, not a reusable C SDK |
| `FFI-03` | Duplicate/foreign runtime handles refuse before use |
| `FFI-04` | Missing artifact and incompatible artifact are distinct refusals |
| `FFI-05` | Bootstrap descriptor checked before opening or mutating data |
| `FFI-06` | One exact-version native artifact per platform, shared by core and log |
| `FFI-07` | Importing the core starts no transport or log maintenance work |
| `FFI-08` | Loader refuses any other-version artifact; handshake encodes ABI, format, OS/CPU floors |
| `PKG-01` | Fresh locked builds per platform; provenance records source, flags, locks and digests |
| `PKG-02` | Packing never rewrites the checkout; no prepack/postpack hooks |
| `PKG-03` | Tarball-isolated consumers and chapter-34 fixtures in an empty project |
| `PKG-04` | Canonical target matrix: Apple Silicon, Graviton ARM64, Linux x86-64 Node |
| `PKG-05` | Golden data compatibility; pre-1.0 importer is early refusal |
| `PKG-06` | Affirmative absence of deleted C/public-Rust-log/pack-hook/superbuilders products |
| `PKG-07A` | Pre-promotion: exact staged digests, empty-project installs, pins, simulated partial publication |
| `PKG-07B` | After separately authorized publication: download actual registry artifacts and verify identical digests |

### Generated history and applications

| Obligation | Required behavior / evidence |
| --- | --- |
| `TS-MIG-01` | Same-process and different-process generators cannot overwrite winner artifacts |
| `TS-MIG-02` | Kernel-held repository lock; lock body may be empty or garbage and is irrelevant |
| `TS-MIG-03` | Kill after durable file/sync/manifest step; retry finds previous history or the complete committed chain |
| `TS-MIG-04` | Lock remains owned until in-progress I/O is joined; no late write after a successor begins |
| `TS-MIG-05` | New generator acquires the same persistent inode without deleting/replacing it |
| `TS-MIG-06` | A stale token's repeated release cannot unlock a successor |
| `TS-MIG-07` | Bounded aggregate receiving; growing-file reads stop at the cap |
| `TS-MIG-08` | Invalid UTF-8 is rejected and the descriptor is closed |
| `TS-MIG-09` | Assertions use actual recorded plans/snapshots, not file existence |
| `TS-MIG-10` | Keep test inputs beside the test; no replacement fixture tree |
| `APP-01` | No native library, database file, secret or admin plan enters a client/public bundle |
| `APP-02` | Next.js serverExternalPackages and explicit platform tracing includes |
| `APP-03` | Inspect the emitted server unit for the `.node` file and execute it in the target environment |
| `APP-04` | Cold materialization measured on the deployed runtime, not assumed from Node compatibility |
| `APP-05` | HostedHistory local directory on ephemeral hosts is disposable materialization |
| `APP-06` | Concurrent-tenant FD/memory/disk budget measured per instance |
| `APP-07` | Credentials refresh through the supported provider chain; static keys are never committed |
| `APP-08` | Migration cutover is an admin job with budgets — never a request hook or worker startup path |
| `APP-FAST` | Resident application read scorecard; preparation measured separately from execution |
| `APP-MUTATE` | Mutation/read pair: insert/replace/delete/no-change/rejection then the prepared read |
| `APP-NUMERIC` | Exact bits and errors before any numeric timing |
| `APP-LARGE` | Actual >RAM and >32 GiB datasets |
| `APP-TENANTS` | Many idle users, opening storms, slow tenant plus small neighbor |
| `APP-TARGETS` | Apple Silicon, real Graviton ARM64 and Linux x86-64 Node cells |
| `APP-METHOD` | Record CPU, OS/libc, toolchain, artifact, flags, memory limits, durability, load and raw distributions |
| `APP-MAGIC` | Classify constants as representation bound, host policy or measured crossover; no public autotuner |

### Storage bytes and hashing

| Obligation | Required behavior / evidence |
| --- | --- |
| `SPACE-01` | Recalculate raw index/payload bytes from the actual layout; fits-the-backend is eligibility, not a cost model |
| `SPACE-02` | Compare SQLite with matched facts/laws/indexes/durability; separate fresh, churn and pinned-snapshot sizes |
| `HASH-01` | 16-byte fingerprints only narrow an exact comparison |
| `HASH-02` | 32-byte commitments authenticate object/history identity |
| `HASH-03` | Constant-hash tests prove collisions affect work, not truth |
| `HASH-04` | Selected BLAKE3 decision plus actual-input/layout qualification; AEGIS remains optional research |

## Parent gates

See [release-gates.md](release-gates.md) for G00–G16, D01–D29, runner
order and evidence identity. The inventory lists the same IDs for
machine checking. G03/G04/G07 and D04/D05/D19/D26 bind
`lean/correspondence.md` case ids; independent oracles are
`judge_final_state`, `staged.rs`, and `history_model.rs` — not the
production planner. L20 executable subset is `correspondence::OWNED_CASES`
in `bumbledb-bench` (not `scripts/lean.sh`).
