# Native and SDK lanes L12–L17

## Common dispatch preamble — send with every lane

Read your full section, attached C/D clauses and source findings; use [60](60-cursor-execution.md) for exclusive paths. Implement the actual capability→operation→output→cleanup chain. Shared declarations/exports belong to the coordinator. Request another owner’s caller adaptation explicitly; do not create a wrapper to bypass it.

No product checks, builds, typechecks, package hooks, lint/format checks, benchmarks or commits during fanout. Author regression schedules now. Handoff is ReadyForIntegration, with changed paths, supplied declarations, consumed owners, removed predecessors, authored tests, verification NotRun and named unresolved seams. Escalate a conflicting contract with evidence instead of redesigning privately.

## L12 — Fixed-worker resource tables, not parked sessions

**Read:** C2/C4/C7; TS-002/003/008/009; runtime.rs worker/run_lane_command, session.rs open/host functions, lanes/registry/owners and runtime_wire. Own runtime.rs, runtime/** and runtime_wire.rs with their inline tests. L13 owns db_wire; L14 owns log_wire; L16 owns TS declarations.

**Outcome:** one-worker open/read/close works; many idle snapshots do not consume one thread each; reachable JS wrappers cannot prevent actual native drain. Ready at F0 on L07 owned-read contract.

**Implement:** each configured worker owns a resource table as ordinary event-loop state. Snapshot entries hold owned pinned read plus worker-local prepared state, not borrowed callback stack frames. Capability routes to worker/kind/id/generation/runtime. Borrow the entry for one job and return to scheduler; no ready_rx blocking. Construct/register snapshot and report ready before any user operations, retaining cleanup ownership if delivery fails. Remove unused JS-driven WriteSession/HostWrite ABI; sealed apply/submit keeps the whole writer operation inside a native job.

Global state only schedules/routes under short locks. No payload work/global registry lock or permanent tombstones. Reserve before insertion; rollback payload/count/bytes together. Busy/closing resources reject new work and drain after current operation. Coalesced close flags are existing admitted obligations, not QueueFull-prone new jobs. Wake sleeping workers for every source. Run heavy destruction off JS and release directory fences last. Preserve bounded fairness and truthful incomplete-close reports.

**Outputs:** real capability/resource access, cancellation/output transfer and joined close interfaces to L13/L14/L16. L07’s owned read is required; no unsafe Send around ReadInstance. Request exact removed writer declarations from L16.

**Delete:** session-long reactors, thread-per-session legacy, readiness channels waiting on same pool, global with_payload closure, wrapper-owned RetainedGuard, orphan-on-admission failure, indefinite revoked rows and rejectable teardown dispatch.

**Acceptance:** D18/D24/D29 with workers=1, sleeping pool, snapshots>workers, reachable JS tokens, saturated queue, failed admission, abandoned output, long create/revoke history and small neighboring tenant. Counters must match actual release; do not zero counters as cleanup.

## L13 — Core native API, cumulative drafts and transactional delivery

**Read:** C2/C4/C7/C8; CORE-008 and TS-001/004/005/006/007; db_wire.rs snapshot/session/result/draft/apply, marshal.rs, L05 delivery ticket. Own db_wire.rs/db_wire/**/marshal.rs and their tests.

**Outcome:** all core addon operations use real bounded owners; one native pull never consumes undelivered rows. Ready at F0 on L05/L12 declarations, even while bodies are incomplete.

**Implement:** replace JS-held payload Arcs/guards with worker resource capabilities. Snapshot and execution-session APIs use L07/L12’s owned read/frame and preserve actual prepared reuse. Each operation has fresh WorkContext; draft accumulation keeps its independent cumulative input/work/deadline through finish and becomes terminal on failure.

For collection/paging: preflight bounded conversion from a core ticket, reserve overlapping buffers and register output, then commit cursor position once. A next row that does not fit ends a nonempty page; an oversized first row refuses unchanged. Cancellation/resource failure before commit aborts and drops the entire admitted output. Terminal backing error closes. Queued output retains charge until transfer/drain. Bound cell/string/byte work before copying and never hide full-result conversion behind a result handle.

**Outputs:** unchanged selected public verbs with real ownership to L16; shared published snapshot constructor to L14; exact ticket adaptations to L05. Report every old read/write/result bypass removed.

**Delete:** eager per-row advancement within uncommitted batch, wrapper retention authority, unbounded collect calls, duplicate row codecs and old write-session consumer paths.

**Acceptance:** D01/D07/D12/D18/D25 on the real addon: two rows individually fitting but jointly too large, cancellation after first copy, retry, scratch fault terminality, EOF/early take, result beyond snapshot lifetime, queued-output close and cumulative multi-chunk draft exhaustion.

**Do not:** solve data loss by silently returning empty/partial-complete results, disabling retry or adding a public raw cursor.

## L14 — Native log evidence, verified migration and repository fence

**Read:** C4–C8; LOG-029 and TS-013/014; log_wire.rs/admin, migration_wire.rs chain_response/verify_compiled_chain, runtime owner and existing directory fence. Own log_wire.rs/log_wire/**/migration_wire.rs/log.rs and adjacent tests.

**Outcome:** log imports core resource behavior rather than rebuilding it; every migration path compiles complete schema-bound plans; repository generation can hold a real native OS lock across joined TS I/O. Ready at F0.

**Implement:** adapt every snapshot/result/command/admin/backup/restore to L12/L13 registry and L10 bounded materializer. Preserve Decided receipt on optional diagnostics failure; map phase directly from L08 evidence, never English errors. Keep stable command/admin refs before dispatch and honest inspect health. Remove native chunk aggregation and retain abandoned-output owners.

Require all base/intermediate snapshots in verify/append/generate/admin paths. Compile symbolic expressions against exact source/target kinds and laws even for empty data before any artifact commit/freeze. Expose a minimal internal repository-lock Effect bridge over L11’s existing kernel directory exclusion, with an opaque resource capability and idempotent joined release; no second public DB/file-lock API. Register the lock before returning it to JS.

**Outputs:** typed wire/codec/lock declarations to L17; export updates to coordinator/L16 internal subpath. Requests for core payloads go to L13, not copied code.

**Delete:** optional snapshot verification, structurally parsed-but-uncompiled append, certainty-to-generic-error adapters, log-specific reader/result owner twins and whole-tail/chunk arrays.

**Acceptance:** D13/D17/D18/D20/D28: genuine producer diagnostic failure, invalid hashed mapping with zero rows before side effects, concurrent/process-death generation locking, partially acquired native owner cancellation and >RAM restore through addon.

**Do not:** expose writable core Db through published snapshot, make a TS evaluator or use mock wire outcomes as publication proof.

## L15 — One scalar/query grammar with honest unresolved fields

**Read:** C1/C8; TS-012/016 and selected scalar/interval semantics in chapter 01. Own ts/src/scalar.ts, query/**, query.ts if present, fields.ts and spec.ts. Own ts/test/scalar-algebra.test.ts, computed-find.test.ts, comparison-pairing.test.ts and parse-query-ir.test.ts; other TS tests belong L16.

**Outcome:** query operators and migration arithmetic share actual constructors/roster; source-field backfill is usable without a caller inventing types. Ready at F0 on the shared leaf/AST declaration.

**Implement:** generic leaf-scoped node representation with one tagged literal grammar and operator roster. Query variable leaves derive known schema kinds. Migration field-name leaves carry unresolved kind until native binding. Arithmetic over unresolved leaves builds inert AST; fully known incompatible kinds refuse. Preserve I64/U64/F64 distinction, explicit casts and cached bounded depth; avoid recursive whole-tree validation at each node. Keep normal stable plain metadata objects, no native loading/proxies/evaluation. Native compiler receives exact one grammar, not a second flattened hand-maintained interpretation.

**Outputs:** authoring constructors and lowering to L14/L16/L17, with example Scalar.add(Scalar.field("units"), Scalar.u64(1n)). L14 owns semantic binding; L17 passes every required snapshot.

**Delete:** duplicate QueryNode/MigrationNode rosters, generic field<T>, unconditional field-node rejection, arbitrary phantom result assertions and re-walk-on-every-constructor logic.

**Acceptance:** D19/D27: valid field arithmetic/casts author synchronously addon-free, then execute through query and generated migration. Wrong source field/kind and invalid literal-only operations refuse at their promised boundary; native empty-source compile still refuses invalid plans. Known query I64/U64 mixing fails static/type checks without any/casts.

**Do not:** pretend a symbolic expression is already typechecked, blanket-disable native validation, or require users to handwrite snapshot/parser bytes.

## L16 — Effect core surface and scoped resource use

**Read:** C7/C8, chapter 30, TS-001–011/016/017. Before edits read the installed Effect 4.0.0-rc.112 AGENTS/integration documentation, acquire-release/callback finalizer behavior and ManagedRuntime examples cited in chapter 30. Own ts/src/** except L15 paths and coordinator index.ts; own remaining ts/test/**.

**Outcome:** pure authoring stays synchronous/addon-free; every operation is a lazy Effect using the one native ownership graph; no Promise/sync API remains. Ready at F0; implement declarations/cleanup against L12/L13.

**Implement:** scope each acquired intermediate before another interruptible step, including directory→Db→snapshot and operation→registered output. Treat callback cleanup correctly for successful completion versus interruption. Preserve original Cause plus close failures and truthful incomplete drain. ChangeSet chunks share lifetime budget and become terminal after failure; host cells/strings are checked before SDK-controlled copies with bounded yields. Result pages are one-shot scoped Stream with min(page/maxBytes, work.resultBytes); no old deadline reuse. Core QueryReader serves published snapshots literally.

Remove deleted writer/session ABI declarations. Keep intentional internal/log subpath minimal and exact-version, with real types shipped; it is not a security boundary. No package import loads addon until runtime acquisition.

**Outputs:** actual core operations/internal exports to L17/L18; changed wire signatures to L12/L13/coordinator. The pinned Effect version is not changed casually.

**Delete:** superbuilders/errors everywhere in owned TS, raw root handles/dispatch, unused old writer verbs, Promise/sync/disposal twins and duplicated ownership wrappers.

**Acceptance:** D07/D12/D18/D22/D25. Type/behavior tests author now; final real-addon runs retain JS tokens, interrupt every partial acquisition gap, close early Stream and verify no duplicate/drop or orphan. No-addon import test must make native loading unavailable, not merely import successfully with it installed.

**Do not:** use SDK-level runPromise, swallow Cause/finalizer failures, per-row Effects/getters/proxies or a new host runtime per tenant.

## L17 — Thin Effect log and transactional generated history

**Read:** C1/C5–C8; TS-013–019; ts-log machine/surface/bridge and migrations fsops/generate/repo/native/codec. Own ts-log/src/** except index.ts and all ts-log/test/**. L18 owns README/examples; L14 native semantics.

**Outcome:** publication certainty survives Effect composition; normal Drizzle-shaped generation is exclusive, bounded and resumable; log reuses core primitives. Ready at F0 on L14 lock/chain and L15 scalar declarations.

**Implement:** use exact imported QueryReader/changes/results/policy/runtime, scoped native resources and typed Effect errors. Preserve stable recovery refs, terminal receipt health and unknowns. No migrations at import/open/request. Generator acquires native kernel-held persistent-inode lock before repository reads/cleanup and holds it until all I/O settles. Remove PID/alive/stale unlink logic. Read same-FD bounded chunks with fatal UTF-8 and aggregate allowance. Write immutable uniquely owned artifacts no-clobber; verify identical existing content; durably commit manifest last; repair derived index/runtime contract idempotently. Cancellation must join pending writes before lock release.

Pass all snapshots and symbolic source-field AST to mandatory native chain compile before publishing artifacts or freezing. Empty source is not a shortcut. Keep typed ambiguous rename/drop/backfill/seed intent and deterministic unchanged no-write behavior.

**Outputs:** real generated initialization/migration workflow to L18, exact native requests to L14, core seam corrections to L16.

**Delete:** duplicated scalar/read/changes/runtime APIs, stat→whole-read, stale-file lock heuristics, partial recorded-file overwrites, optional compile and public async CLI twin. Keep framework runners only at executable boundaries.

**Acceptance:** D13/D15/D18/D20/D21/D27/D28. Same/cross-process generator exclusion, death after each durable step, growing reads, interruption while promise I/O continues, field arithmetic backfill, edited/missing snapshots and packed reopen under expected prefix. Use actual native lock/compile for acceptance, not scripted codec alone.

**Do not:** invent filesystem transactions/leases, arbitrary JS data transforms, silent migration discovery or another TS log engine.
