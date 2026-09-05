# Requirement continuity: nothing closes by losing its old document

The original 68 audit IDs, all CORE-001–025, LOG-001–029, TS-001–020 and REVIEW-001–004 remain owned. These are obligations, not a count of distinct bugs or passed fixes. Current root mechanisms are in [11](11-core-findings.md), [21](21-log-findings.md) and [31](31-sdk-findings.md); complete paths and discriminators in [60](60-cursor-execution.md)/[70](70-test-and-release-gates.md).

**None is qualified by this proposal review. The owner sets below are traceability/accountability groups, not permission for overlapping writes; exclusive source ownership is in chapters 60–65.** A source repair listed below must be preserved and integrated, not rebuilt needlessly. If source has changed again, inspect the named mechanism. Source-complete, integrated and qualified are separate states.

## Seven commitments and required exits

| Commitment | Production representation / deletion | Owners |
| --- | --- | --- |
| Compile once | Interned theory/row/scalar program used by storage, judgment and planner; no parallel interpretation | L01/L02/L05/L06/L15/L16 |
| Delta-local application work | Exact affected-group indexes and witnesses; independent reference semantics | L01/L02/L05/L06/L07 |
| Retention equals ownership | Inseparable admitted capacities, shared token generations, bounded complete scratch/native lifecycle | L03/L04/L05/L06/L07/L10/L11/L12/L13 |
| Earn Free Join dividends | Preserve COLT/factoring/SIMD; narrow distinct/existence/fold witnesses; bounded real loops | L01/L02/L05/L06 |
| Thin correct authority | Evidence-bearing attempt, coherent writer parent, strict locators and private staged lifecycle | L07/L08/L09/L10/L11/L14 |
| One application API | Core primitives imported by log; Effect-only operations; typed generated migrations and actual app | L12/L13/L14/L15/L16/L17/L18 |
| Subtract and qualify | Delete superseded mechanisms and pointless tests; exact source/artifact/target evidence | L19/L20/L21 + coordinator/all |

## Prior review obligations, current ownership

The fourth column is a new discriminator or inherited gate, not an assertion that only that one test is sufficient. Detailed inherited schedules remain in [permanent behavioral obligations](../docs/reference/behavioral-obligations.md).

| ID | Required closure / source repair to preserve | Owner(s) | Discriminators |
| --- | --- | --- | --- |
| CORE-001 | Shared field validity including fixed widths and intervals; repaired walker needs all decoder paths. | L01/L02 | D04/D19 |
| CORE-002 | Bounded exact competitor visitor; landed key streaming still needs integration. | L01/L02/L03/L04 | D03/D04 |
| CORE-003 | DecodedRow cannot shed charge through owning extraction. | L01/L02/L03/L04 | D01 |
| CORE-004 | Reserve before growth, bounded no-output polls, propagate final quantum. | L03/L04/L05/L06 | D01/D08 |
| CORE-005 | All sink banks and retained pools own real capacity. | L03/L04/L05/L06 | D01/D09/D11 |
| CORE-006 | Shared bounded text generation and complete nonresident resolver. | L03/L04/L05/L06 | D02/D09 |
| CORE-007 | Every derived/negative/recursive consumer supports sealed scratch backing. | L05/L06 | D09 |
| CORE-008 | Admit collection/page conversion before growth or cursor advance. | L05/L06/L12/L13/L15/L16 | D12/D25 |
| CORE-009 | Transactional retry-safe retained and physical scratch accounting. | L03/L04 | D03 |
| CORE-010 | Choose cursor regime before resident u32 bounds; no global DB-size limit. | L05/L06/L07 | D07/D09 |
| CORE-011 | Affected containment/capacity groups consume compiled adjacency. | L01/L02 | D04 |
| CORE-012 | Intern useful compact projections; qualify real persisted keys. | L01/L02 | D04 |
| CORE-013 | One store cache, safe generations and actual retention accounting. | L03/L04/L05/L06/L07 | D02 |
| CORE-014 | Pinned same-thread read cannot deadlock map resize. | L07 | D07 |
| CORE-015 | Actual freshness owner/full metadata check, not Copy marker. | L07 | D06/D26 |
| CORE-016 | Private staging, complete (not empty-delta incremental) final judgment and absent-or-complete installation. | L07/L10/L11 | D06 |
| CORE-017 | Explicit map ceiling is respected, not rounded above request. | L07 | D07 |
| CORE-018 | Current proof premises and empirical refinement, no stale census. | L19/L20/L21 | G03/G04 |
| CORE-019 | Test semantics/locality, not mandatory inefficient full-table spill. | L01/L02/L19/L20/L21 | D04 |
| CORE-020 | One explicit scratch capability; no TypeId error reflection. | L01/L02/L03/L04 | D03 |
| CORE-021 | Canonical witness selection before truncation and replay. | L01/L02/L08/L09 | D05 |
| CORE-022 | Bound-key fallback seeks, early stop and one row layout. | L01/L02/L05/L06 | D10 |
| CORE-023 | Wide Pack stable exact tokens/order plus unambiguous narrow mode. | L03/L04/L05/L06 | D11 |
| CORE-024 | Exclusive owned scratch setup and failure cleanup. | L03/L04 | D03 |
| CORE-025 | Preserve source repairs; verify consumers and current artifacts. | L01/L02/L05/L06/L19/L20/L21 | D04/D07/D08 |
| LOG-001 | State-specific certainty across every admin failure. | L08/L09/L14/L17/L18 | D13 |
| LOG-002 | Retirement validates captured parent under writer. | L08/L09 | D14 |
| LOG-003 | Preserve repaired same-writer replay; qualify races. | L08/L09 | D14 |
| LOG-004 | Control revision advances independently of decision/data. | L08/L09/L14 | D14 |
| LOG-005 | Coherent retained frontier; no loss proof after receipt retirement. | L08/L09/L14 | D15 |
| LOG-006 | Historical precondition derived from command/exact predecessor. | L08/L09 | D05/D14 |
| LOG-007 | Nonempty-required cold hydration/resume uses private builder. | L07/L10/L11 | D06 |
| LOG-008 | Restore staging, correct genesis-root artifacts and no ready partial state. | L07/L10/L11/L14 | D06/D17 |
| LOG-009 | Native chunk flow bounded transitively. | L10/L11/L14 | D17 |
| LOG-010 | Migration transform/state memory and work use core bounded substrate. | L10/L11 | D17/D20 |
| LOG-011 | Receiving and waits carry caps/deadlines, not post-body checks. | L10/L11 | D17 |
| LOG-012 | Preserve bounded listing; durable canonical progress, no prefix rescans. | L08/L09/L10/L11 | D17 |
| LOG-013 | Strict direct locators including actual 49-byte encoding. | L08/L09/L10/L11 | D16 |
| LOG-014 | Immutable S3 create exactness and retry evidence. | L10/L11 | D17/G08 |
| LOG-015 | Preserve transactional local root registry; concurrency/crash evidence. | L08/L09 | D14/G10 |
| LOG-016 | Every successful HEAD change strictly advances control revision. | L08/L09 | D14 |
| LOG-017 | Receipt retirement works with no application decision advance. | L08/L09/L10/L11 | D14 |
| LOG-018 | Finite cache tail defaults and work on actual opens. | L14/L17/L18 | G11/G12 |
| LOG-019 | Every acquire/close continuation retains a cleanup owner. | L12/L13/L14/L17/L18 | D18/D24/D29 |
| LOG-020 | Inspection reports unknown/unavailable truth, no invented zero health. | L14/L17/L18 | D13/D15 |
| LOG-021 | Bounded receipt/prune/ancestry visitors under coherent ownership. | L07/L08/L09/L10/L11 | D14/D17 |
| LOG-022 | Create versus explicit adopted genesis binds actual initial state. | L07/L08/L09/L10/L11 | D06 |
| LOG-023 | Shared transport runtime and actual refreshing credential chain. | L10/L11/L12/L13 | D17/D18 |
| LOG-024 | Actual S3/IAM evidence; mocks cannot qualify backend. | L10/L11/L19/L20/L21 | G08/D17 |
| LOG-025 | Canonical rejection survives physical remint/spill. | L01/L02/L08/L09 | D05 |
| LOG-026 | Preserve Rust command certainty repairs across adapters/retirement. | L08/L09/L14 | D13/D15 |
| LOG-027 | Preserve complete initial migration judgment on every path. | L07/L10/L11 | D06/D20 |
| LOG-028 | Preserve identity, directory fencing and stale capability refusal. | L12/L13/L14/L17/L18 | D18/G14 |
| LOG-029 | Never discard decided receipt when optional diagnostics fail. | L14/L17/L18 | D13 |
| TS-001 | Actual submitted work reaches snapshot/query/get execution. | L12/L13/L15/L16 | D07 |
| TS-002 | Native registry owns payloads, close independent of GC. | L12/L13/L14 | D18 |
| TS-003 | Each partial acquisition owned before next interruption. | L12/L13/L15/L16/L17/L18 | D18 |
| TS-004 | Bound host length/cell/chunk work before conversion. | L15/L16/L17/L18 | D07/D12 |
| TS-005 | Bounded real native/JS result delivery and event-loop work. | L05/L06/L12/L13/L15/L16 | D12 |
| TS-006 | Draft failures are terminal and close is joined. | L12/L13/L15/L16 | D07/D18 |
| TS-007 | Cumulative draft budget includes all chunks and finish. | L12/L13/L15/L16 | D07 |
| TS-008 | Fixed-worker owned resource tables replace parked/thread-per-session reactors. | L12/L13 | D18 |
| TS-009 | Real prepared session reuse and joined independent close. | L05/L06/L12/L13/L15/L16 | D08/D18 |
| TS-010 | Addon unavailable during pure metadata import. | L15/L16/L19/L20/L21 | D22 |
| TS-011 | One public Effect API; intentional shared internal seam. | L15/L16/L17/L18 | D07/D22 |
| TS-012 | Shared scalar grammar with known query kinds and honest unresolved migration fields; canonical literals and native binder/evaluator. | L01/L02/L05/L06/L10/L11/L14/L15/L16 | D19/D27 |
| TS-013 | Mandatory bound snapshots and full mapping compile before effects. | L10/L11/L14/L17/L18 | D20 |
| TS-014 | Kernel-held repository exclusion with stable inode, joined I/O, immutable staged files and one manifest commit. | L17/L18 | D21/D28 |
| TS-015 | Bounded aggregate receiving, interruption/durability correctness. | L17/L18 | D21 |
| TS-016 | Canonical boundary values, row types/arity and fixed widths. | L01/L02/L14/L15/L16 | D19/G02 |
| TS-017 | Typed operational failures, complete rejection details, Cause preserved. | L14/L15/L16/L17/L18 | D13/D19 |
| TS-018 | Real native-ledger-shaped application; sibling read-only. | L17/L18/L19/L20/L21 | D22 |
| TS-019 | Honest Notes/Alchemy/Rust examples, real witness and migration prefix. | L17/L18/L19/L20/L21 | D07/D22 |
| TS-020 | Actual fresh packed native artifacts, correct evidence and useful tests. | L19/L20/L21 | D23/G13 |
| REVIEW-001 | Count admitted benchmark work only; preserve landed rejection-unwrapping repair and verify truth | L19/L20/L21 | G15/D04 |
| REVIEW-002 | Adjudicate historical failures/copied assumptions using independent tiny oracle; no pinned test census | L01/L02/L05/L06/L19/L20/L21 | D04/D11/G04 |
| REVIEW-003 | One fresh build input, exact release evidence, no duplicate/stale runner work | L19/L20/L21 | D23/G13 |
| REVIEW-004 | Current public docs/usage, correct dedup/durability/API/format claims | L17/L18/L19/L20/L21/coordinator | D22/G01 |

## Original audit obligations

The following required dispositions are preserved from the reviewed packet, with current owners. Names and 220 child schedules in the permanent machine inventory are stable coverage identities, not a requirement for a separate test file per ID. No production check in this review executed the inventory.

| Audit ID | Required successor disposition | Blocking property / obligation families | Owner(s) |
| --- | --- | --- | --- |
| REP-001 | Delete vacant per-braid publication slots. A never-reused HEAD and epoch-bound staged dependencies are the only hosted publication path | A paused old writer cannot publish behind retained history or reference collected objects. G07/G08/G10; PROTO-01 and GC barrier schedules | L08/L09/L10/L11/L14/L17/L18 |
| REP-002 | Delete scalar-sum ordering of vector recovery floors. One tenant order; checkpoint boundary must be an authenticated ancestor of the current tip | Fresh recovery after collection reconstructs the resulting state at its captured tip and every receipt promised by current retention; it need not retain every historical decision object forever. G07/G10 | L08/L09/L10/L11/L14/L17/L18 |
| REP-003 | Delete clock-equality retention mode and implicit 90-day promise. Retain explicit named roots; release is an explicit authority operation | Restart/clock change cannot delete a retained restore point. G10; breaking policy documented, not silently weakened | L08/L09/L10/L11/L14/L17/L18 |
| REP-004 | Delete the entire database entity allocator and FreshRef representation. Applications seal concrete 128-bit IDs | No reservation/counter ownership claim exists; retries preserve entity bytes, duplicate keys are judged normally, and UUID uniqueness is not misrepresented as absolute. G08/G09; PROTO-04/11 | L08/L09/L10/L11/L14/L17/L18 |
| REP-005 | Delete expiring filesystem mutation leases. Kernel-held local ownership or actual S3 conditional replacement enforces authority | Suspended owner cannot coexist with a successor mutating its protected state. G06/G08/G11 | L08/L09/L10/L11/L14/L17/L18 |
| REP-006 | Delete writer-number fencing of a shared allocator | Arbitrary caller identity cannot strand a healthy request in unchanged retries; every contention loop has a bounded outcome. G07/G09 | L08/L09/L10/L11/L14/L17/L18 |
| REP-007 | Scratch is staging evidence, not remote deletion authority. Only rooted epoch GC may remove objects | Crash after publication, later checkpoint, reopen: every retained dependency remains recoverable. G10 | L08/L09/L10/L11/L14/L17/L18 |
| REP-008 | Bind checkpoint publication to the actual captured/revalidated head and ancestor suffix; no stale backlink pretending to be full retained history | Every retained root has a discoverable complete closure under reversed candidate publication. G07/G10 | L08/L09/L10/L11/L14/L17/L18 |
| REP-009 | Acquire process-lifetime directory ownership before any cleanup or cache recovery | An opener refused ownership makes no changes to the active owner's files or remote objects. An owned later-failing hydration must preserve/release its explicitly registered hold correctly. G06/G11 | L08/L09/L10/L11/L14/L17/L18 |
| REP-010 | Remove separately persisted filesystem body/fence authority. LocalHistory uses one LMDB transaction; hosted HEAD uses qualified atomic CAS | No crash leaves published bytes with a downgraded authority generation. G06/G08 | L08/L09/L10/L11/L14/L17/L18 |
| REP-011 | Canonical collision-safe local names plus checked origin/database/incarnation binding before adoption | Case aliases, same-schema tenants and changed origins cannot serve or mutate one another's state. G10/G11/G14 | L08/L09/L10/L11/L14/L17/L18 |
| REP-012 | Remove historical token-file chains and predecessor scans | Repeated ownership/open cycles have cost tied to current work, not lifetime acquisition count. G11/G12 | L08/L09/L10/L11/L14/L17/L18 |
| REP-013 | Replace slot-1 rescans and process-local ages with epoch inventory/mark/sweep and durable bounded progress | Repeated GC resumes safely, survives restart, and does not rescan all retired history forever. G10/G12 | L08/L09/L10/L11/L14/L17/L18 |
| REP-014 | Capture one coherent snapshot; publish it with a validated current suffix without recompacting on every intervening write | Checkpoint completes under sustained permitted load or reports a bounded resource condition, not an unobservable quiet-window loop. G06/G10/G15 | L08/L09/L10/L11/L14/L17/L18 |
| REP-015 | Every refresh captures a finite tip; every wait/replay/resolve consumes an execution budget | Concurrent writers or unreachable positions cannot occupy a host indefinitely. G07/G12; PROTO-13/18 | L08/L09/L10/L11/L14/L17/L18 |
| REP-016 | Log returns a published read capability, never the underlying writable core owner | Unlogged writes are not expressible through a replicated read handle; losing candidates never leak. G07/G11; also SDK-008 | L08/L09/L10/L11/L14/L17/L18 |
| REP-017 | Hold ownership through operation drain, native close and protected cleanup; only then release | Old disposal cannot erase or close a successor's directory/environment. G08/G11 | L08/L09/L10/L11/L14/L17/L18 |
| REP-018 | Typed ObjectRef verifies kind/epoch/length/digest; snapshot and replay certificates bind exact boundaries | Wrong object identity, overshot boundary, corrupt chain or foreign schema refuses before data is exposed. G02/G06/G10/G14 | L08/L09/L10/L11/L14/L17/L18 |
| REP-019 | Deletion progress is durable and retryable; failed deletion never discards sole discovery evidence | Partial failure/host loss leaves an auditable resumable cleanup path. G10 | L08/L09/L10/L11/L14/L17/L18 |
| REP-020 | Remove split commit: one tenant command is atomic. Terminal named receipt contains one outcome | No successful prefix can disappear from a later error result; a multi-relation command is all-or-none. G07/G09; PROTO-10 | L08/L09/L10/L11/L14/L17/L18 |
| ENG-001 | Remove public unchecked interval construction; all typed/constant/dynamic inputs use checked canonical primitives | Safe supported input cannot commit invalid interval/bool/width representation. G02/G03; E-VALUE | L01/L02/L03/L04/L07 |
| ENG-002 | Replace trusted safe raw `Fact` codecs with typed field encoding or checked external bytes | A custom integration cannot establish canonicality by merely returning success. G02/G03; E-CODEC | L01/L02/L03/L04/L07 |
| ENG-003 | OwnedSnapshot holds one real read transaction; content, generation and attachment derive from it | Concurrent export/copy/checkpoint never mismatches metadata and rows. G06; E-SNAPSHOT | L01/L02/L03/L04/L07 |
| ENG-004 | Remove core escaped reservations and log FreshRefs; entity IDs are ordinary owned input | No database issuance capability remains; abrupt exit/retry preserves the sealed application's ID bytes. G09; E-NO-RESERVE | L01/L02/L03/L04/L07 |
| ENG-005 | Judge the proposed relation/multimap independently of physical unique-index installation | All violated statement IDs are reported for completed judgment, including refused-row conflict permutations. G03; E-ADMIT | L01/L02/L03/L04/L07 |
| ENG-006 | Remove mandatory immortal text dictionary; live tuples own canonical text | Deleted text has no independently live dictionary entry and disappears from live-state export; retention/physical erasure remains explicitly log/filesystem policy. G03/G06/G10; E-TEXT | L01/L02/L03/L04/L07 |
| ENG-007 | Remove fresh-ID burn machine; do not flatten infrastructure failure into semantic rejection | Every persistence failure remains observable; terminal log outcome retains publication certainty. G03/G06/G09 | L01/L02/L03/L04/L07 |
| ENG-008 | Remove hidden no-sync constructors from ordinary production capability; benchmark-only weakening is structurally isolated | Default downstream API cannot silently select benchmark durability; production durability modes are tested. G01/G06 | L01/L02/L03/L04/L07 |
| QRY-001 | Results are owned complete values or an explicit error; scratch is not a published answer | Every failure family leaves no apparently complete partial current answer, including grouped overflow. G04/G12 | L03/L04/L05/L06/L12/L13/L15/L16 |
| QRY-002 | Primary warm Free Join/selective index paths, bounded LMDB fallback and one RAM/LMDB scratch map; charge growth before or at bounded points | A query exceeding RAM continues on disk; deadlines/storage failures stop safely without unbounded native work or hidden truncation. G04/G05/G12/G15 | L03/L04/L05/L06/L12/L13/L15/L16 |
| QRY-003 | One public execution policy carried through all bindings, with effective values observable | Downstream users can control request budgets, and tests prove what each limit actually bounds. G01/G12 | L03/L04/L05/L06/L12/L13/L15/L16 |
| SDK-001 | One Rust machine owns immutable unresolved attempts; a new command cannot overwrite them | Failure then next command on the same live handle preserves correct local state and resolvable outcome. G07/G09 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-002 | Shared lifecycle authority checked inside queued operations; close revokes admission | Retained writers and queued requests cannot dispatch new publications after closure. G07/G11 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-003 | Core ChangeSet owns/normalizes inputs during bounded Effect ingestion; success establishes acceptance; log seals only its history envelope around that same value | Caller buffer mutation and escaped builders cannot change persisted/applied command meaning; no duplicate log recorder or scalar codec. G02/G07/G14 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-004 | Every acquisition returns a distinct generation-bound idempotent borrow; borrow disposal returns that borrow | Double/stale release cannot consume another borrow or close the shared owner. G11 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-005 | Owner/pool close accounts for opening operations as real owned work | An in-flight open cannot outlive shutdown and return an unowned live replica/timer. G11 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-006 | Remove expiring directory renewal from ownership; process-lifetime OS lock and explicit capability revocation | Missing stale token can never be interpreted as successful ownership renewal; paused owner remains fenced by real exclusion. G08/G11 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-007 | Deterministic native owner close, with explicit outstanding operation/snapshot policy | Eviction releases actual environment/lock/resources, not just a JS wrapper. G11/G12 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-008 | Same root fix as REP-016: published read capability without raw writable Db | Direct accepted unlogged writes cannot disappear on refresh because that capability is absent. G01/G07/G11 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-009 | No arbitrary async callback under the serialized commit gate; submission takes sealed data | Nested await cannot wait on a gate held by the same user callback. G01/G07/G11 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-010 | Same root fix as REP-015: finite targets and propagated WorkContext | Deadline/cancellation reaches actual native/I/O work, not just fiber waiting; interruption cleanup joins or records incomplete native drain. G07/G12 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-011 | Cache/query budgets account for real resources; pressure uses LMDB, concurrent admission reserves actual work | No DB-size/RAM hard boundary; no uncounted opening storm or native-memory exemption. G05/G11/G12 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-012 | Same root fix as REP-012: no lifetime lease-token chain | Repeated use does not accumulate quadratic cleanup work. G11/G12 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-013 | Delete the entire public C API and its callback/tombstone mechanism; preserve the ownership property at Rust/Node boundaries | Affirmative crate/header/export/workflow/consumer removal checks; Node close/drain releases the engine/lock and long operation history has bounded resources. G00/G11/G13; FFI-* | L12/L13/L14/L15/L16/L17/L18 |
| SDK-014 | Private uncommitted LMDB candidate plus published snapshot capability | Read-only wrappers never point at a candidate that later rejects. G06/G07 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-015 | No callback replay, placeholder allocator or refill retry machine; concrete immutable operations and receipts | Side effects and entity generation cannot be duplicated by hidden recording retry; old callback API removed/tested. G01/G09 | L12/L13/L14/L15/L16/L17/L18 |
| SDK-016 | Verify configured origin and authoritative incarnation before using cache, pending work, or scratch | Equal-schema/equal-revision cache reuse refuses/reseeds before any foreign fact or command crosses scope. G10/G11/G14 | L12/L13/L14/L15/L16/L17/L18 |
| ARCH-001 | Explicit ExactState precondition for read-dependent commands; blind set writes remain explicit | Two-decrement/ABA/no-op/rejection schedules; stable named precondition outcome. G07/G09 | L01/L02/L08/L09/L14/L17/L18 |
| ARCH-002 | Tenant total order replaces product-of-braid-prefix read semantics | No accidental cross-braid causality claim or split outcome. Cross-tenant work remains outside one transaction. G07/G09 | L01/L02/L08/L09/L14/L17/L18 |
| ARCH-003 | Named commands and retained receipt epochs; permanent refusal after retirement | Crash-before-response, duplicate request, mismatched digest and expired-ID schedules. G09/G10 | L01/L02/L08/L09/L14/L17/L18 |
| ARCH-004 | Database/incarnation, command/decision/state identity and local origin binding are separate | Foreign token/cache, rebirth, restore and migration tests. G09/G10/G14 | L01/L02/L08/L09/L14/L17/L18 |
| ARCH-005 | One internal Rust log state machine; TypeScript-only public log product; Rust/TS core only | One reference history corpus through internal Rust and public Node surface; no separately handwritten TS protocol or public Rust log API, and no public C product anywhere. G07/G13 | L01/L02/L08/L09/L14/L17/L18 |
| ARCH-006 | No braids-as-row-sharding story; one tenant is a write authority | Measure real-schema hot-tenant contention; resident placement does not change authority. G15 | L01/L02/L08/L09/L14/L17/L18 |
| OPS-001 | TypeScript schema SDK generates canonical migration plan/history; log owns freeze, staged execution, admission and one final new incarnation | Generation refuses unresolved ambiguity; no handwritten callback escape; kill at each cutover step, invalid destination never current, old writer refuses, plan edits/history divergence refuse. G10/G13; the SDK migration contract | L08/L09/L10/L11/L14/L17/L18/L19/L20/L21 |
| OPS-002 | Log backup/restore primitives and an independent protected recovery root in the deployment runbook | Restore after loss of local cache/active namespace; verify credentials/policy separate from ordinary GC. G08/G10 | L08/L09/L10/L11/L14/L17/L18/L19/L20/L21 |
| OPS-003 | Immutable blob first, reference/receipt commit second; application outbox and idempotent effect dispatcher pattern | Missing blob, orphan upload, lost acknowledgment and restored-reference drill. G09/G10/G13 | L08/L09/L10/L11/L14/L17/L18/L19/L20/L21 |
| OPS-004 | Actual owner/borrow/resource boundaries and host authentication-to-tenant mapping | Cross-tenant cache/handle attacks and noisy-neighbor/lifecycle tests. G11/G12/G14 | L08/L09/L10/L11/L14/L17/L18/L19/L20/L21 |
| OPS-005 | Explicit cached/refreshed/minimum-decision reads, published snapshot provenance and typed unavailable state | Missing history is never empty; captured-tip and timeout behavior tested. G07/G10/G12 | L08/L09/L10/L11/L14/L17/L18/L19/L20/L21 |
| OPS-006 | Structured counters/status already at command, snapshot, owner and GC boundaries—not an observability service | Tests verify certainty, progress and redaction fields; runbook diagnoses representative failures. G10/G11/G14 | L08/L09/L10/L11/L14/L17/L18/L19/L20/L21 |
| PERF-001 | No compulsory relation-sized image rebuild after each write; preserve primary warm Free Join and bounded disk fallback | First-read-after-insert/replace/delete; hot/cold/forced-disk equivalence, retained hot-path assembly/allocation gates and in-situ costs. G05/G15; the performance contract | L01/L02/L03/L04/L05/L06/L07/L10/L11/L12/L13/L19/L20/L21 |
| PERF-002 | Distinguish map size, file size, resident cache, plans/results and work; deterministic release and LMDB-backed scratch | >RAM and >32 GiB datasets, tenant churn, retained result/plan/native owner accounting. G05/G11/G12/G15 | L01/L02/L03/L04/L05/L06/L07/L10/L11/L12/L13/L19/L20/L21 |
| PERF-003 | Count complete named-decision path, retries and checkpoint costs, not one winning PUT | Requests/bytes/time per terminal outcome at single/multiple writers; no old footprint speed claim. G15 | L01/L02/L03/L04/L05/L06/L07/L10/L11/L12/L13/L19/L20/L21 |
| PERF-004 | Coherent streamed checkpoint with validated suffix and bounded progress | Continuous-write checkpoint success, cancellation, peak disk/RAM and catch-up tests. G06/G10/G15 | L01/L02/L03/L04/L05/L06/L07/L10/L11/L12/L13/L19/L20/L21 |
| PERF-005 | One bounded worker adapter for all core/log TypeScript work through Effect; only Rust keeps blocking calls | Event-loop/cross-tenant progress and cancellation under slow workloads. G11/G12/G15 | L01/L02/L03/L04/L05/L06/L07/L10/L11/L12/L13/L19/L20/L21 |
| ASS-001 | Rewrite actual semantic proof premises; old braid theorem is not used to certify the new log | Closed vocabulary/mutable-support model tests; real correspondence obligations in the current proof bridge. G03/G07 | L01/L02/L08/L09/L17/L18/L19/L20/L21 |
| ASS-002 | Independent history model, deterministic failures and real process/backend tests | G07/G08/G10 with client-visible traces, not only final bytes | L01/L02/L08/L09/L17/L18/L19/L20/L21 |
| ASS-003 | Rewrite current docs/examples for selected contract; label historical research | Compiled downstream examples and source/artifact compatibility matrix. G01/G13 | L01/L02/L08/L09/L17/L18/L19/L20/L21 |
| ASS-004 | Preserve audit evidence and add immutable fix/test/decision records | G00/G16; no retirement-by-deleting-evidence | L01/L02/L08/L09/L17/L18/L19/L20/L21 |

## Scope narrowing and permanent coverage

Pre-1.0 import/compatibility is not supported: MIG-06/PKG-05 require early untouched refusal, not a legacy engine. Generated within-family migrations, backup/replay/corruption checks remain required. HASH-04 requires the selected single BLAKE3 decision and actual-input/layout qualification; AEGIS experimentation is optional and not an implementation lane. No existing semantic/resource/backend guarantee is narrowed by deleting its old test.

The 220 child schedules and 17 parent gates already live in `docs/reference/behavioral-obligations.md` and `obligation-inventory.json`. L19/L20/L21 reconciles them with D01–D29 and current permanent contracts without duplicating report hierarchies. Remove any required S3/Graviton “not applicable” bypass; missing prerequisites are unqualified. Do not scrape this disposable Markdown or invent placeholder Passed evidence.

Every claim of closure links the real production/test changes and exact qualified candidate in the final evidence index/review. A worker cannot drop an architectural P2, essential negative test, or target requirement as “cleanup later.” Owner-approved product changes must be explicit and propagated through every affected contract.
