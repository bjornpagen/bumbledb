# 60 — Implementation scope and acceptance milestones

Status: implementation stopped and preserved; proposal refactored for an orchestrated restart, 2026-09-04. All agents are stopped. The incomplete source checkpoint is `4b127782`; read [the frozen handoff](../implementation/06-frozen-implementation-handoff.md). This documentation pass does not extend implementation or run verification. It commits/pushes the preserved work and executable proposal, not a release.

Start the next campaign with [PROMPT.md](../PROMPT.md). Read [61 — orchestration](61-orchestration-and-dependency-graph.md), [62 — packets](62-work-packets.md), [63 — shared contracts](63-shared-interface-contracts.md) and [64 — final verification](64-final-verification-and-handoff.md). They own scheduling, file ownership and execution timing. M0–M8 below remain **scope/acceptance families**, not serial implementation queues or permission to run tests early.

## Rules for the rewrite

- Preserve the dated audit and counterexamples. Port their safety properties before deleting the old mechanisms; an API disappearing is not by itself a regression pass.
- Break the representation once, deliberately. Do not maintain old braid publication, fresh counters, dictionary encodings and new semantics simultaneously inside the core to ease a pre-1.0 compatibility burden the owner explicitly rejected.
- Keep shared interfaces and file ownership explicit while implementing broad packets concurrently. Temporary integration breakage is allowed; declaration scaffolds do not count as completion. No per-agent commits or implementation-phase checkpoint churn. The orchestrator owns final-stage candidate/evidence commits and push.
- Every packet names its deletions, normative contract, proof/test changes and measured cost. If it adds a new authority or independently mutable cache, justify that addition against [01](01-representation-first.md).
- Native Rust implements semantics and the log machine once. The public log product is TypeScript-only; the core supports Rust/TypeScript. Delete all public C code, headers, examples and release machinery; preserve applicable native-safety tests for Rust/Node.
- TypeScript schema declarations generate migration plan/history data. There are no user-authored migration callbacks, manual coverage lists, JavaScript purity framework, core migration DSL or fleet control plane.
- Read baseline source/evidence and author benchmarks/models/regressions from F0 onward, but **execute no tests, typechecks, builds, linters or probes until F3**, after all implementation is integrated. Chapters 40–41 remain binding; physical layout/hash choices freeze after final-phase measurements, with affected code and goldens requalified. Preserve baseline revisions so deleted hot paths can still be compared.

## Acceptance families, implemented through a parallel graph

```text
F0: real shared contracts + exclusive write ownership + authored model/test plans
                           |
F1: engine/storage/query || history/recovery || native/SDK/migrations
    + independent proof/model, adversarial, package/app and performance authors
                           |
F2: all selected behavior integrated + source review + complete coverage mapping
                           |
F3: final builds/tests/probes -> repairs -> format freeze -> full qualification
                           |
F4: final Git handoff (release promotion needs separate authorization)
```

Chapter 61 gives the actual per-packet dependency graph, including contract-ready versus implementation-ready handoffs. A real per-user application slice is authored/integrated alongside the engine, including reopen and named retry/lost-response recovery. Source review protects warm Free Join while bounded fallback is built; measurements run only in F3. No SDK or log lane waits for an entire predecessor milestone's tests before starting.

M0–M6 exit properties below are evaluated during F3, not at intermediate packet handoff. Before then, all test execution status remains NotRun. Map exact test names to the single chapter 70 ledger. Never label an entire G-family passed because a substrate test passed historically. `PKG-07B` alone is post-promotion distribution verification, completed only after separately authorized M8.

## M0 — Freeze the contract and make failures durable evidence

Create the implementation's explicit guarantee/test inventory from [50](50-audit-closure-matrix.md) and [70](70-test-and-release-gates.md). Preserve original fixture source, observed outputs and artifact limitations. For each old failure, express the successor property in an independent harness or model even where the old public API is removed.

Freeze the finite scalar/law/query roster, same-command add-wins normalization, integer/float interval denotations, canonical float quotient, integer aggregate overflow semantics, concrete 128-bit entity values, command grammar, three history coordinates, local versus hosted publication, and generated migration plan/history contract. Use small golden examples as the first executable specification. Maintain one issue-to-test ledger, not separate inconsistent “done” lists in each language. Freeze hash roles/widths only after the explicit collision/threat model and preformat cost probes in chapter 41; no hash equals fact identity.

Review [34's syntax](34-sdk-syntax-and-composition.md) and [35's complete Effect contract](35-effect-typescript-contract.md) before implementation resumes. Freeze the exact Effect 4 dependency, A/E/R, Scope/Stream/error ownership, interruption/finalizer rules, stable intent and V8 conversion budget. No optional Effect adapter or Promise/sync twin. Freeze the core/log import ownership and common read/change interfaces alongside those examples: the log envelopes core changes and imports core operators/results, never records its own fact DSL. Freeze the grouped-measure normal form and typed query-composition boundaries; do not preserve obsolete spelling bans or projection-only interiors just to reuse old fixtures.

Freeze semantic examples and shared declarations before dependent implementation; do not freeze unmeasured physical layout. Author storage/hash/long-key probes now; execute them in F3 before final physical golden bytes. Select one layout and one algorithm per role, remove losing variants, update affected implementation and then qualify persistent encodings. The first authored/integrated vertical slice is canonical insert/judge/query/reopen through LMDB and TypeScript, warm Free Join versus forced disk and LocalHistory named retry; hosted lost-response recovery extends that same slice as the history implementation lands.

Freeze the initial supported OS/CPU/libc/Node/backend matrix in chapter 32: Apple Silicon, ARM Graviton and x86 Vercel Node are canonical targets. Core runs natively; the TypeScript log requires supported Node/native execution and fitting local storage. Browser, Edge, Expo/React Native and WebAssembly support are not implied. ARM/x86 portable correctness is required now; specialized tuning beyond Apple Silicon is not.

### A clean format break

Use distinct format families for the new core store, log objects, command/receipt codec and portable snapshot framing. The selected human-readable identities are `bumbledb.core.v1`, `bumbledb.log.v1`, `bumbledb.command.v1` and `bumbledb.snapshot.v1`; freeze their exact domain-separated binary magic/tag encodings and golden files in this packet. Within each new family the layout counter starts at **1**. Never recognize a store/object solely by that integer.

Core metadata validates family, codec, schema and store identity before writable open/recovery/cleanup. Log objects validate family, kind, identity and canonical framing before interpretation. Unknown, absent or old-family headers are incompatibility/corruption—not an implicit invitation to initialize an empty database. Creation and opening remain separate operations.

Digests include the applicable domain, schema/type and canonical bytes. Do not accidentally preserve an old schema hash while changing scalar equality or admitting different query behavior. An old-format converter is a separately invoked **log tool**, not an implicit branch in every core read. Physical LMDB portability and logical snapshot portability are separate contracts.

Deliverables: approved golden roster, counterexample inventory, independent tiny evaluator/history-model skeletons, release-result schema, exact family encodings, and the chosen platform list. Exit: G00 inventory complete; initial fixtures demonstrate that the new contracts are distinguishable from the old failures. This is not a claim that all semantic gates already pass.

## M1 — Canonical data and a judge with one meaning

Replace the safe raw-codec trust opening with checked typed field construction and a schema-bound byte parser. Close the interval constructor bypass. Add `F64` with canonical NaN/zero, total order, strict canonical bytes, explicit casts, sum/mean and dense-domain `Interval<F64>`. Share endpoint-order kernels; distinguish bounded float measure overflow from unbounded rays. No `FixedInterval<F64>`/float-width interval compression or approximate capacity weights; ordinary F64 and float-interval payloads remain fixed-size.

Make full canonical tuple equality authoritative, including long values and collision buckets. Remove default global text interning and expose text as ordinary live value bytes. Keep local physical row IDs separate from application values. Test actual LMDB maximum key sizes and large determinants before selecting physical index encodings.

Normalize a command's insert/delete sets once and judge the complete candidate, including refused competing key rows. Install unique indexes only after judgment. Return complete violated-statement IDs with bounded examples or a resource failure—not incomplete rejection disguised as completion. Retain exact final-state set semantics and closed vocabulary distinctions.

Keep one exact nonnegative grouped-measure law with count as unit weight. Normalize supported aliases within its canonical representation; preserve parent-domain empty-group zero, zero-weight facts and target/source key premises. Do not replace indexed law enforcement with arbitrary query assertions or add time-varying occupancy. Port old syntax-ban tests into canonical-equivalence or genuine semantic-refusal tests as appropriate.

Update Lean from the chosen denotation rather than preserving obsolete theorem premises. Prove the actual mutable support definition; model canonical float normalization/order and exact aggregate bounds. Independent model tests compare values, state transitions and specified errors. Keep unsupported trust boundaries explicit.

Delete: unchecked public constructors, hash-as-equality axiom, default immortal dictionary, order-dependent diagnostics, obsolete fresh-allocation semantic assumptions. Exit: G02/G03 and relevant E/F/P/CONC children pass on new representations; downstream safe API attacks are included.

## M2 — Make LMDB's strengths the default

Build one environment owner with kernel-held directory exclusion acquired before cleanup and released last. Implement coherent owned read snapshots, per-open identity, core-local generation, real native close and the transaction gate required for safe map resize. Remove the 32 GiB constant as product policy; grow geometrically subject to actual platform/disk feasibility.

Implement the private candidate path and the narrow host adjunct: checked prepare/admit → log computes outcome → seal opaque host records → commit or abort. The sealed capability cannot mutate application facts. Metadata-only/no-op and domain-rejection decisions must use the same atomic storage boundary. A failed seal, including MAP_FULL after decision hashing, causes no remote dispatch.

Keep the transaction on its owning worker across a bounded hosted attempt. Map-full before publication aborts/retries immutable work after safe resize; it never reruns application code. Local failure after known hosted publication preserves the published outcome and faults only the materialization.

Delete: expiring local leases, separately committed generation authority, GC-only close, snapshot metadata gathered from multiple source transactions, core fresh reservations and benchmark durability escape hatches in ordinary production API. Exit, evaluated in F3: G06/G11 substrate obligations and deterministic crash/resize/visibility schedules pass. Larger-data harnesses are authored with this work and qualified on complete product artifacts.

## M3 — Excellent application queries with a complete bounded fallback

Preserve Free Join and selective indexed access as primary application paths. Implement cursor/index-nested-loop execution as the complete bounded fallback and simple independent comparison path, not as a universal warm-query replacement. Build one scratch-map abstraction with charged RAM and temporary-LMDB backing; share it across projection distinct, group binding/state, derived relations, recursive frontiers and sealed results. A fitting warm query should not pay temporary-LMDB writes merely for architectural uniformity. This is not a second persistent database or an external-sort framework.

Keep compact Boolean query structure instead of mandatory exponential expansion. Charge prepare, parameters, growth, intermediate work, aggregates and output before growth. Publish results only when evaluation/finalization is complete. A consuming result cursor transfers one sealed backing owner; streaming delivery failures do not turn a prefix into a complete answer.

Replace projection-only interior special cases with typed relation-expression dependencies, including nonrecursive aggregate outputs consumed by later queries. Preserve set grain, empty-group, rounding and stage-error boundaries through any inlining/pushdown; naming alone does not force full materialization. Keep one positive linear recursive component with projection-only feedback. Frozen finite predecessor relations may contain computed values; no aggregation, value invention or negation participates in the cycle. Reuse the same core representation/operators in generated migration plans.

Implement exact integer widened sums and deterministic F64 arithmetic/reductions. Qualify the floating environment guard, architecture/compiler behavior, exact 34-limb accumulation and exact-rational mean rounding. The independent bit/rational oracle must not reuse production numerical helpers. Do not weaken deterministic answers for faster reduction.

Retain warm SIMD/Free Join behind correct budget reservation and fallback. Forced scalar/cursor/spill execution must agree with optimized paths, including errors and float bits. Author matched comparisons now and measure changes against preserved warm/post-write/cold baseline revisions in F3. Tune batch, prefetch, table load and scratch thresholds through final-phase adversarial sweeps; neither missing measurements nor a fast kernel excuses requiring a whole relation in RAM.

Delete: mandatory full images, output mutation on failure, hidden unbounded prepare/recursive paths, order-sensitive float reduction, invalid numerical rewrite rules. Exit: G04/G05/G12 semantic/fallback tests pass on small forced-spill fixtures, and controlled large-store campaign is executable.

## M4 — One internal durable history machine

Implement LocalHistory and HostedHistory as explicit variants using the same canonical concrete-command/outcome grammar. LocalHistory publishes through one durable LMDB commit. HostedHistory publishes through the tenant HEAD CAS over one immutable command decision and a bounded recovery tail. LocalHistory does not simulate that object store.

Add stable command identity, exact-state condition, durable no-change/rejection outcomes and three distinct coordinates. Resolve uncertainty by retained receipts; closed/retired command epochs cannot become new commands by absence. Application IDs are already concrete sealed values, preserved through retries/restores with no allocation result mapping. TypeScript supplies owned commands, not retryable arbitrary host callbacks.

Author the independent history model and client-visible schedules during implementation; execute them in the final F3 campaign. Include candidate reads, delayed CAS, lost response, later decisions, checkpoint/receipt movement, live-handle next commands, close and post-publication local failure. Distinguish exact known success, definite precondition failure and unknown transport outcome at the real adapter boundary.

Delete: per-braid slots, vector recovery floors, split commits, writer-ID fencing/counter objects, public raw mutable replica access and the second TypeScript protocol. Exit, evaluated in F3: the implemented command/publication portion of G07/G09 and PROTO passes in deterministic models and real local adapter tests; checkpoint/GC histories and whole-product performance remain recorded M5–M7 acceptance dependencies. Real-S3 qualification executes only in F3 in an explicitly authorized disposable environment.

## M5 — Recovery and deletion that preserve the one history

Implement streamed complete logical checkpoints from one RO snapshot, bounded manifests/buffers, exact suffix rebase and fully verified staging activation. Checkpointing must progress during ongoing writes; it does not restart a whole export on every head move. Expose tail backpressure and maintenance health.

Implement explicit named roots/hydration holds, receipt retirement and hosted epoch barrier/mark/sweep with exact-parent reference introduction. GC cursor/phase changes are fenced by barrier identity; late old uploads are future orphans, not publishable missing dependencies. Local restore points use independent complete export directories and owner-scoped cleanup, not remote epoch GC.

Add independent backup/restore primitives in the internal log and TypeScript surface. Restore a writable copy into a new incarnation; preserve old entity IDs as data. Test without the original cache/origin and with independent backup permissions. Do not call an active-store pointer an independent backup.

Delete: scratch-as-deletion-authority, age sentinels/default PITR promise, historical slot/token scans, partial hydration accepted as empty state, retries that lose discovery evidence. Exit: the implemented storage/recovery portion of G08/G10 and STORE/LOCAL/GC/FS/S3/REC/BACKUP/RESTORE/ERASE passes in its available lanes; migration, packaged deployment and full workload qualification remain explicit M6–M7 dependencies. Real S3 is mandatory for the hosted claim.

## M6 — The application-facing product

Delete the public C API and replace any remaining internal lifetime-erased callback pointers with bounded capabilities where needed. Make TypeScript owners, borrows, drafts, changes, commands, snapshots and results Effect-scoped, with small inert wrappers after native close. Finalizers report incomplete/failed close in Cause; one-shot page Streams replace the public TS cursor facade. Keep Rust guards genuinely lifetime-safe. Build one bounded native runtime behind a core Context.Service/Layer so all core/log work avoids blocking unrelated requests. Use version-matched Effect 4 idioms, batch/page granularity and stable V8 data shapes; no generic consumer wrappers or second JS tenant cache.

Deliver the schema/query SDK, TypeScript log API, schema-diff generator, canonical repo-local plans/history, explicit `migrate()` workflow and server-only Next.js/Alchemy/Vercel examples in [33](33-typescript-migrations-and-apps.md). Users write schemas, never imperative migration modules. Generation requires declarative intent where rename/backfill/destruction is ambiguous. The native log executor validates the generated chain, freezes source authority, evaluates the pending batch with necessary intermediate checks, stages one final destination, admits/verifies it, then returns a still-frozen `ReadyToSwitch` binding and activation reference. Do not publish/rebuild a complete intermediate incarnation per file when plan composition can preserve the same meaning. Activation and binding cutover remain explicit; a durable activation marker resolves lost responses. Abort must irreversibly fence target activation/delayed genesis before thawing the matching source; observing nonactivation is insufficient. Ordinary app startup never quietly rewrites a production tenant.

The examples must work from installed packages with actual deployment runtime, native bundling, IAM attachment, refreshed credentials, authenticated tenant mapping, deadlines and errors. Alchemy provisions ordinary resources, not a new database control plane. Local development uses LocalHistory. Hosted Node uses S3 and fitting local disk for materialization. Qualify Vercel's ephemeral-disk/cold-open envelope explicitly; larger tenants remain supported on provisioned hosts. Edge is a different unsupported runtime, not a synonym for Vercel Node.

Remove public Rust log exports/docs and the entire C crate/header/export/packaging/workflow/example surface. Internal Rust/Node tests remain indispensable. No compatibility shim, dormant C feature or separate C artifact remains.

Ship one Node platform artifact/runtime with core and internal log capabilities. Keep the public Rust core free of log/AWS dependencies and keep core-only Node imports free of transport/maintenance initialization. Delete log-owned scalar tags/row lifting/change recorders and core-barrel log exports. Prove that the same core change/query/parameter/result objects and read helpers work unchanged through both TypeScript packages, with foreign-runtime handles refused rather than pointer-bridged.

Delete: source-mutating packing hooks, unchecked native-version pairing, event/body auth shortcuts, automatic implicit migrations, and duplicate cross-language log API maintenance. Exit: API/RUN/FFI, pre-promotion PKG and chapter 33 migration/application tests run against fresh staged consumers and production-shaped deployments; M7 closes their whole-product evidence. Post-publication `PKG-07B` is not a pre-promotion exit condition.

## M7 — Close the ledger and complete whole-product measurement

Run the entire [70](70-test-and-release-gates.md) inventory against fresh artifacts, not only tests filtered by changed source paths. Every detailed child has an executed lane and evidence; a broad green job does not cover missing float/FPU, local migration or GC cases.

Run the physically populated >40 GiB fixture, separately enforced >RAM workload, cross-platform float corpus, process/power-failure qualification, real S3 conditions, backup-to-clean-restore, migration crashes/history divergence, noisy neighbors, stale handles and package mismatch. Count live resources and cancellation-to-quiescence, not only final exceptions.

Measure intended application schemas across warm/cold/after-write/>RAM/multiwriter/maintenance regimes. Compare only equal correctness/durability. Hold every surprising regression for an explicit cost decision or fix. No universal “fastest database” claim is earned by a cherry-picked kernel measurement.

All known supported-behavior defects close with fix, permanent regression and independent review. Removed scope is documented, not hidden as a passing test. Unfinished proof, unsupported required runtime, missing cloud credentials, stale artifacts or an unrun large-data lane leaves qualification incomplete.

## M8 — Promote exactly what passed

Build release artifacts once in staging from the clean candidate revision. Record toolchains, dependency locks, supported targets and all artifact digests. Test those artifacts through isolated installation, API examples, migration drills and backend qualification. Packaging must not rewrite the checkout.

Complete G16's **pre-promotion packet**, including `PKG-07A`, before release authorization/tagging. Publish the tested platform dependencies and packages in the declared order; then complete `PKG-07B` by verifying downloaded artifacts equal the staged artifacts and exact native pins resolve. Only then is G16/release completion final. Do not rebuild a different binary during publication. If post-publication installation verification fails, report and repair the release; do not pretend a tag means success.

The current phase authorizes preservation/push and proposal refactoring only. A future invocation of PROMPT.md starts the F0–F4 implementation campaign. It does not authorize production migration, cloud provisioning, package publication, version bump or a 1.0 tag. Missing required external qualification remains a release blocker, not a reason to waive a gate.

## Stop conditions that protect the design

If the complete checkpoint/one-tenant-CAS cost is unsuitable for the measured application, return to the explicit tradeoff rather than quietly adding braids, remote page trees or a second authority. If a numerical proof or FPU bridge fails, fix the implementation/domain specification before claiming deterministic floats. If the TypeScript migration runner needs an online fleet coordinator, keep 1.0's explicit per-tenant downtime and report the requested expansion separately.

The implementation should become smaller in conceptual machinery even if the test corpus gets much larger. This campaign succeeds when a few mechanisms explain the entire product—and every selected guarantee has evidence.
