# 50 — Every known audit issue has a successor obligation

**All rows are specified work, not implemented fixes or closed findings.** The 1.0 release gate requires the implementation, named regressions, supported-platform evidence and independent closure review. The original [audit register](../audit/00-findings.md) remains the record of what was observed.

The audit has 47 indexed implementation observations; several share a root cause and some are explicit contract/design limitations. Architectural, operational, performance and assurance IDs are also covered below. Removing an obsolete mechanism is a valid breaking design choice only when its replacement passes the safety property the old defect exposed.

Gate IDs refer to [70 — Test and release gates](70-test-and-release-gates.md); chapter-specific `E-*`, `F-*`, `PROTO-*`, GC/recovery and SDK gates refine those obligations. During implementation add exact permanent test names and fix commits to each row. Do not change a row to closed merely because the proposal has a paragraph about it.

## Replication and physical history

| Audit ID | Successor disposition | Blocking property / gates |
| --- | --- | --- |
| REP-001 | Delete vacant per-braid publication slots. A never-reused HEAD and epoch-bound staged dependencies are the only hosted publication path | A paused old writer cannot publish behind retained history or reference collected objects. G07/G08/G10; PROTO-01 and GC barrier schedules |
| REP-002 | Delete scalar-sum ordering of vector recovery floors. One tenant order; checkpoint boundary must be an authenticated ancestor of the current tip | Fresh recovery after collection reconstructs the resulting state at its captured tip and every receipt promised by current retention; it need not retain every historical decision object forever. G07/G10 |
| REP-003 | Delete clock-equality retention mode and implicit 90-day promise. Retain explicit named roots; release is an explicit authority operation | Restart/clock change cannot delete a retained restore point. G10; breaking policy documented, not silently weakened |
| REP-004 | Delete separate counter-object allocation. Resolve fresh placeholders from the winning decision; preserve uncertainty in the adapter | No two successful commands issue the same generated entity identity; equal bytes do not prove ownership. G08/G09; PROTO-04/11 |
| REP-005 | Delete expiring filesystem mutation leases. Kernel-held local ownership or actual S3 conditional replacement enforces authority | Suspended owner cannot coexist with a successor mutating its protected state. G06/G08/G11 |
| REP-006 | Delete writer-number fencing of a shared allocator | Arbitrary caller identity cannot strand a healthy request in unchanged retries; every contention loop has a bounded outcome. G07/G09 |
| REP-007 | Scratch is staging evidence, not remote deletion authority. Only rooted epoch GC may remove objects | Crash after publication, later checkpoint, reopen: every retained dependency remains recoverable. G10 |
| REP-008 | Bind checkpoint publication to the actual captured/revalidated head and ancestor suffix; no stale backlink pretending to be full retained history | Every retained root has a discoverable complete closure under reversed candidate publication. G07/G10 |
| REP-009 | Acquire process-lifetime directory ownership before any cleanup or cache recovery | An opener refused ownership makes no changes to the active owner's files or remote objects. An owned later-failing hydration must preserve/release its explicitly registered hold correctly. G06/G11 |
| REP-010 | Remove separately persisted filesystem body/fence authority. LocalHistory uses one LMDB transaction; hosted HEAD uses qualified atomic CAS | No crash leaves published bytes with a downgraded authority generation. G06/G08 |
| REP-011 | Canonical collision-safe local names plus checked origin/database/incarnation binding before adoption | Case aliases, same-schema tenants and changed origins cannot serve or mutate one another's state. G10/G11/G14 |
| REP-012 | Remove historical token-file chains and predecessor scans | Repeated ownership/open cycles have cost tied to current work, not lifetime acquisition count. G11/G12 |
| REP-013 | Replace slot-1 rescans and process-local ages with epoch inventory/mark/sweep and durable bounded progress | Repeated GC resumes safely, survives restart, and does not rescan all retired history forever. G10/G12 |
| REP-014 | Capture one coherent snapshot; publish it with a validated current suffix without recompacting on every intervening write | Checkpoint completes under sustained permitted load or reports a bounded resource condition, not an unobservable quiet-window loop. G06/G10/G15 |
| REP-015 | Every refresh captures a finite tip; every wait/replay/resolve consumes an execution budget | Concurrent writers or unreachable positions cannot occupy a host indefinitely. G07/G12; PROTO-13/18 |
| REP-016 | Log returns a published read capability, never the underlying writable core owner | Unlogged writes are not expressible through a replicated read handle; losing candidates never leak. G07/G11; also SDK-008 |
| REP-017 | Hold ownership through operation drain, native close and protected cleanup; only then release | Old disposal cannot erase or close a successor's directory/environment. G08/G11 |
| REP-018 | Typed ObjectRef verifies kind/epoch/length/digest; snapshot and replay certificates bind exact boundaries | Wrong object identity, overshot boundary, corrupt chain or foreign schema refuses before data is exposed. G02/G06/G10/G14 |
| REP-019 | Deletion progress is durable and retryable; failed deletion never discards sole discovery evidence | Partial failure/host loss leaves an auditable resumable cleanup path. G10 |
| REP-020 | Remove split commit: one tenant command is atomic. Terminal named receipt contains one outcome | No successful prefix can disappear from a later error result; a multi-relation command is all-or-none. G07/G09; PROTO-10 |

## Core semantic engine

| Audit ID | Successor disposition | Blocking property / gates |
| --- | --- | --- |
| ENG-001 | Remove public unchecked interval construction; all typed/constant/dynamic inputs use checked canonical primitives | Safe supported input cannot commit invalid interval/bool/width representation. G02/G03; E-VALUE |
| ENG-002 | Replace trusted safe raw `Fact` codecs with typed field encoding or checked external bytes | A custom integration cannot establish canonicality by merely returning success. G02/G03; E-CODEC |
| ENG-003 | OwnedSnapshot holds one real read transaction; content, generation and attachment derive from it | Concurrent export/copy/checkpoint never mismatches metadata and rows. G06; E-SNAPSHOT |
| ENG-004 | Remove core escaped fresh reservation contract. Log FreshRefs resolve only with a published decision | No prepublication ID can be presented as durably reserved; abrupt exit/retry preserves issued identity. G09; E-NO-RESERVE |
| ENG-005 | Judge the proposed relation/multimap independently of physical unique-index installation | All violated statement IDs are reported for completed judgment, including refused-row conflict permutations. G03; E-ADMIT |
| ENG-006 | Remove mandatory immortal text dictionary; live tuples own canonical text | Deleted text has no independently live dictionary entry and disappears from live-state export; retention/physical erasure remains explicitly log/filesystem policy. G03/G06/G10; E-TEXT |
| ENG-007 | Remove fresh-ID burn machine; do not flatten infrastructure failure into semantic rejection | Every persistence failure remains observable; terminal log outcome retains publication certainty. G03/G06/G09 |
| ENG-008 | Remove hidden no-sync constructors from ordinary production capability; benchmark-only weakening is structurally isolated | Default downstream API cannot silently select benchmark durability; production durability modes are tested. G01/G06 |

## Query execution

| Audit ID | Successor disposition | Blocking property / gates |
| --- | --- | --- |
| QRY-001 | Results are owned complete values or an explicit error; scratch is not a published answer | Every failure family leaves no apparently complete partial current answer, including grouped overflow. G04/G12 |
| QRY-002 | Disk-native execution, optional bounded caches and one RAM/LMDB scratch map; charge work/growth before or at bounded points | A query exceeding RAM continues on disk; deadlines/storage failures stop safely without unbounded native work or hidden truncation. G04/G05/G12 |
| QRY-003 | One public execution policy carried through all bindings, with effective values observable | Downstream users can control request budgets, and tests prove what each limit actually bounds. G01/G12 |

## SDK, native ownership and hosting

| Audit ID | Successor disposition | Blocking property / gates |
| --- | --- | --- |
| SDK-001 | One Rust machine owns immutable unresolved attempts; a new command cannot overwrite them | Failure then next command on the same live handle preserves correct local state and resolvable outcome. G07/G09 |
| SDK-002 | Shared lifecycle authority checked inside queued operations; close revokes admission | Retained writers and queued requests cannot dispatch new publications after closure. G07/G11 |
| SDK-003 | Copy/normalize into a sealed command before asynchronous work; recorder becomes spent | Caller buffer mutation and escaped builders cannot change persisted/applied command meaning. G02/G07/G14 |
| SDK-004 | Every acquisition returns a distinct generation-bound idempotent borrow; borrow disposal returns that borrow | Double/stale release cannot consume another borrow or close the shared owner. G11 |
| SDK-005 | Owner/pool close accounts for opening operations as real owned work | An in-flight open cannot outlive shutdown and return an unowned live replica/timer. G11 |
| SDK-006 | Remove expiring directory renewal from ownership; process-lifetime OS lock and explicit capability revocation | Missing stale token can never be interpreted as successful ownership renewal; paused owner remains fenced by real exclusion. G08/G11 |
| SDK-007 | Deterministic native owner close, with explicit outstanding operation/snapshot policy | Eviction releases actual environment/lock/resources, not just a JS wrapper. G11/G12 |
| SDK-008 | Same root fix as REP-016: published read capability without raw writable Db | Direct accepted unlogged writes cannot disappear on refresh because that capability is absent. G01/G07/G11 |
| SDK-009 | No arbitrary async callback under the serialized commit gate; submission takes sealed data | Nested await cannot wait on a gate held by the same user callback. G01/G07/G11 |
| SDK-010 | Same root fix as REP-015: finite targets and propagated WorkContext | Deadline/cancellation reaches actual native/I/O work, not just Promise waiting. G07/G12 |
| SDK-011 | Cache/query budgets account for real resources; pressure uses LMDB, concurrent admission reserves actual work | No DB-size/RAM hard boundary; no uncounted opening storm or native-memory exemption. G05/G11/G12 |
| SDK-012 | Same root fix as REP-012: no lifetime lease-token chain | Repeated use does not accumulate quadratic cleanup work. G11/G12 |
| SDK-013 | Replace immortal raw callback tombstones with bounded generation-tagged handles; invalidate payload ownership | C read/destroy releases the engine and lock; long callback history has bounded retained resources. G11/G13 |
| SDK-014 | Private uncommitted LMDB candidate plus published snapshot capability | Read-only wrappers never point at a candidate that later rejects. G06/G07 |
| SDK-015 | No callback replay or refill retry machine; log placeholders, immutable operations and receipt results | Side effects cannot be duplicated by a hidden recording retry; old callback API removed/tested. G01/G09 |
| SDK-016 | Verify configured origin and authoritative incarnation before using cache, pending work, or scratch | Equal-schema/equal-revision cache reuse refuses/reseeds before any foreign fact or command crosses scope. G10/G11/G14 |

## Architecture: settle the questions, not just the symptoms

| Audit ID | Decision | Blocking evidence |
| --- | --- | --- |
| ARCH-001 | Explicit ExactState precondition for read-dependent commands; blind set writes remain explicit | Two-decrement/ABA/no-op/rejection schedules; stable named precondition outcome. G07/G09 |
| ARCH-002 | Tenant total order replaces product-of-braid-prefix read semantics | No accidental cross-braid causality claim or split outcome. Cross-tenant work remains outside one transaction. G07/G09 |
| ARCH-003 | Named commands and retained receipt epochs; permanent refusal after retirement | Crash-before-response, duplicate request, mismatched digest and expired-ID schedules. G09/G10 |
| ARCH-004 | Database/incarnation, command/decision/state identity and local origin binding are separate | Foreign token/cache, rebirth, restore and migration tests. G09/G10/G14 |
| ARCH-005 | One internal Rust log state machine; TypeScript-only public log product | One reference history corpus through internal Rust and public Node surface; no separately handwritten TS protocol or public Rust/C log API to maintain. Core C remains separately tested. G07/G13 |
| ARCH-006 | No braids-as-row-sharding story; one tenant is a write authority | Measure real-schema hot-tenant contention; resident placement does not change authority. G15 |

## Operations, kept at the log/host boundary

| Audit ID | Minimal disposition | Blocking evidence |
| --- | --- | --- |
| OPS-001 | Repo-local TypeScript migration API in log using checked history, freeze, coherent export, admission and new incarnation; no core migration framework | Kill at each cutover step; invalid destination never current; old writer refuses; migration-file edits/history divergence refuse. G10/G13; chapter 33 |
| OPS-002 | Log backup/restore primitives and an independent protected recovery root in the deployment runbook | Restore after loss of local cache/active namespace; verify credentials/policy separate from ordinary GC. G08/G10 |
| OPS-003 | Immutable blob first, reference/receipt commit second; application outbox and idempotent effect dispatcher pattern | Missing blob, orphan upload, lost acknowledgment and restored-reference drill. G09/G10/G13 |
| OPS-004 | Actual owner/borrow/resource boundaries and host authentication-to-tenant mapping | Cross-tenant cache/handle attacks and noisy-neighbor/lifecycle tests. G11/G12/G14 |
| OPS-005 | Explicit cached/refreshed/minimum-decision reads, published snapshot provenance and typed unavailable state | Missing history is never empty; captured-tip and timeout behavior tested. G07/G10/G12 |
| OPS-006 | Structured counters/status already at command, snapshot, owner and GC boundaries—not an observability service | Tests verify certainty, progress and redaction fields; runbook diagnoses representative failures. G10/G11/G14 |

## Performance and assurance

| Audit ID | Successor disposition | Blocking evidence |
| --- | --- | --- |
| PERF-001 | No compulsory relation-sized image rebuild after each write; disk-native baseline and bounded optional acceleration | First-read-after-insert/replace/delete; hot/cold/forced-disk equivalence and costs. G05/G15 |
| PERF-002 | Distinguish map size, file size, resident cache, plans/results and work; deterministic release and LMDB-backed scratch | >RAM and >32 GiB fixtures, tenant churn, retained result/plan/native owner accounting. G05/G11/G12/G15 |
| PERF-003 | Count complete named-decision path, retries and checkpoint costs, not one winning PUT | Requests/bytes/time per terminal outcome at single/multiple writers; no old footprint speed claim. G15 |
| PERF-004 | Coherent streamed checkpoint with validated suffix and bounded progress | Continuous-write checkpoint success, cancellation, peak disk/RAM and catch-up tests. G06/G10/G15 |
| PERF-005 | Small bounded worker adapter for hosted native calls; explicit blocking embedded API | Event-loop/cross-tenant progress and cancellation under slow workloads. G11/G12/G15 |
| ASS-001 | Rewrite actual semantic proof premises; old braid theorem is not used to certify the new log | Closed vocabulary/mutable-support model tests; real correspondence obligations in chapter 13. G03/G07 |
| ASS-002 | Independent history model, deterministic failures and real process/backend tests | G07/G08/G10 with client-visible traces, not only final bytes |
| ASS-003 | Rewrite current docs/examples for selected contract; label historical research | Compiled downstream examples and source/artifact compatibility matrix. G01/G13 |
| ASS-004 | Preserve audit evidence and add immutable fix/test/decision records | G00/G16; no retirement-by-deleting-evidence |

## Additional observations that must not fall between indexed rows

These were discussed in the audit without separate implementation IDs. They are included to avoid treating the count of 47 as the full scope.

| Source observation | Required action / gate |
| --- | --- |
| Public JS/native version mismatch can load before a compatibility assertion | Versioned ABI handshake and artifact mismatch refusal; G13 |
| Prepack changes the source manifest and cleanup is not exact restoration | Stage immutable package inputs; interruption leaves source unchanged; G01/G13 |
| Source tests can use stale installed native output | Fresh-build and tarball-isolated consumer gates, artifact provenance; G01/G13/G16 |
| Linux package label broader than its actual libc/CPU floor | Declare and run the exact minimum runtime matrix; G13 |
| Semver peer compatibility is not protocol compatibility | Version-family and supported cross-release reopen/replay/mismatch fixtures; G02/G10/G13 |
| Synchronous native calls can block unrelated JS work | Hosted async worker boundary; explicit embedded blocking contract; G11/G12 |
| HTTP example ignores base64/event/method distinctions | Minimal correct request decoding/validation tests; no new web framework; G13/G14 |
| Example lacks an application authorization boundary | Host-supplied trusted tenant resolver required/documented and tested; no invented built-in auth platform; G14 |
| Example's intended IAM role is not necessarily attached; captured credentials can expire | Inspect deployed function role and authentication policy; rotate actual credentials during native operations. G08/G13/G14; RUN-13, S3-03 and chapter 33 deployment tests |
| Errors collapse certainty or expose tenant facts in logs | Stable structured certainty/error family and redacted defaults; G09/G14 |
| Prepared objects retain high-water memory and are environment-owned | Schema-level immutable plan versus environment-bound execution state; trim/release/fallback tests; G04/G11/G12 |
| Hash-as-logical-equality was an unproved exactness shortcut | Exact candidate comparison under forced collisions; G02/G03/G04 |
| Whole remote snapshots and commands can be buffered before limits | Bounded owned command admission; streamed checkpoints/materialization; G05/G10/G12 |

## New requirements from the 1.0 request

| Requirement | Deliverable and release evidence |
| --- | --- |
| Full first-class floats | [11](11-floats.md), all scalar/query/schema/client/codec surfaces, F-* and G02/G04/G13 |
| LMDB larger than memory; no 32 GiB product cliff | [10](10-semantics-and-engine.md), [12](12-query-execution.md), elastic maps, disk-native paths, G05/G06 |
| Aggressive elimination of casework | [01](01-representation-first.md), one owner per mechanism, explicit subtraction review in G00 |
| Small core, no infrastructure expansion | Core/log dependency boundary, reuse LMDB for persistent and scratch ordered maps, manual host/runbook responsibilities |
| Nightly where useful | [13](13-lean-and-rust.md), verified feature/toolchain ledger, nightly repin gates; G01/G04/G11/G15 |
| All missing tests pass before release | [70](70-test-and-release-gates.md), exact-artifact G16 packet |
| TypeScript log and repo-local migration workflow | [33](33-typescript-migrations-and-apps.md), checked migration history, staged transforms, native server-only Next.js + Alchemy example; G10/G13/G14 |
| Commit and push proposal before coding | Documentation-only commit; no release tag, format mutation, source fix or production migration in this phase |

## Closure record format

For each implementation work item append: audit IDs, chosen guarantee, fix commit, exact regression names, evidence artifacts, applicable platforms/backends, reviewer challenge and resolution. Status transitions are `Specified → Implemented → Qualified → Closed`; failed or missing evidence moves it back, not around the gate. A newly discovered defect is added to the matrix immediately and blocks release under the same rule.

The test matrix is intentionally larger than the mechanism inventory. A small production core can and should have a much larger independent body of evidence.
