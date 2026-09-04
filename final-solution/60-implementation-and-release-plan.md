# 60 — Implement the successor in dependency order

Status: proposed implementation campaign. This documentation phase does not start the rewrite, migrate a tenant, reset a shipped version, or publish a package. Its commit is the reviewed starting point, not 1.0 qualification.

## Rules for the rewrite

- Preserve the dated audit and counterexamples. Port their safety properties before deleting the old mechanisms; an API disappearing is not by itself a regression pass.
- Break the representation once, deliberately. Do not maintain old braid publication, fresh counters, dictionary encodings and new semantics simultaneously inside the core to ease a pre-1.0 compatibility burden the owner explicitly rejected.
- Keep intermediate changes reviewable and buildable where practical. A succession of dependency-ordered commits can implement one large breaking release; there is no requirement to publish transitional formats.
- Every packet names its deletions, normative contract, proof/test changes and measured cost. If it adds a new authority or independently mutable cache, justify that addition against [01](01-representation-first.md).
- Native Rust implements semantics and the log machine once. The public log product is TypeScript-only; no Rust/C log API stabilization is required. Core Rust/TypeScript/C remain supported and tested.
- Repo-local TypeScript migrations are a finite log-layer authoring/runner workflow, not a core migration DSL or a fleet control plane.

## Dependency graph

```text
M0: contract, regression/model skeletons, format families
          |
          +--> M1: canonical values + admission + semantic proofs
          |                |
          +--> M2: LMDB ownership + snapshots + sealed host adjunct
                           |
                    M3: disk-native queries + deterministic floats
                           |
                    M4: one internal log machine + receipts
                           |
                    M5: checkpoints + retained roots + recovery/GC
                           |
                    M6: thin clients + TS migration/app workflow
                           |
                    M7: complete qualification + performance work
                           |
                    M8: exact-artifact release promotion
```

Independent prototypes, test models and binding scaffolds can proceed in parallel. The arrows constrain integration evidence, not who may read or work first. Floats' value/proof work begins in M1 and their executor/FFI qualification continues through M7; it is not postponed until after the engine is frozen. A simple correct cursor path precedes heroic kernel optimization.

## M0 — Freeze the contract and make failures durable evidence

Create the implementation's explicit guarantee/test inventory from [50](50-audit-closure-matrix.md) and [70](70-test-and-release-gates.md). Preserve original fixture source, observed outputs and artifact limitations. For each old failure, express the successor property in an independent harness or model even where the old public API is removed.

Freeze the finite scalar/law/query roster, same-command add-wins normalization, interval ceiling contract, canonical float quotient, integer aggregate overflow semantics, command grammar, three history coordinates, local versus hosted publication, and exact migration history contract. Use small golden examples as the first executable specification. Maintain one issue-to-test ledger, not separate inconsistent “done” lists in each language.

Freeze the initial supported OS/CPU/libc/Node/backend matrix in chapter 32. Core runs natively; the TypeScript log requires a supported Node/native server environment. Browser, Edge, Expo/React Native and WebAssembly support are not implied by a TypeScript API or the migration workflow analogy.

### A clean format break

Use distinct format families for the new core store, log objects, command/receipt codec and portable snapshot framing. The selected human-readable identities are `bumbledb.core.v1`, `bumbledb.log.v1`, `bumbledb.command.v1` and `bumbledb.snapshot.v1`; freeze their exact domain-separated binary magic/tag encodings and golden files in this packet. Within each new family the layout counter starts at **1**. Never recognize a store/object solely by that integer.

Core metadata validates family, codec, schema and store identity before writable open/recovery/cleanup. Log objects validate family, kind, identity and canonical framing before interpretation. Unknown, absent or old-family headers are incompatibility/corruption—not an implicit invitation to initialize an empty database. Creation and opening remain separate operations.

Digests include the applicable domain, schema/type and canonical bytes. Do not accidentally preserve an old schema hash while changing scalar equality or admitting different query behavior. An old-format converter is a separately invoked **log tool**, not an implicit branch in every core read. Physical LMDB portability and logical snapshot portability are separate contracts.

Deliverables: approved golden roster, counterexample inventory, independent tiny evaluator/history-model skeletons, release-result schema, exact family encodings, and the chosen platform list. Exit: G00 inventory complete; initial fixtures demonstrate that the new contracts are distinguishable from the old failures. This is not a claim that all semantic gates already pass.

## M1 — Canonical data and a judge with one meaning

Replace the safe raw-codec trust opening with checked typed field construction and a schema-bound byte parser. Close the interval constructor bypass. Add `F64` as specified: canonical NaN/zero, total order, strict canonical wire encoding, explicit casts, and no float intervals/approximate capacity weights.

Make full canonical tuple equality authoritative, including long values and collision buckets. Remove default global text interning and expose text as ordinary live value bytes. Keep local physical row IDs separate from application values. Test actual LMDB maximum key sizes and large determinants before selecting physical index encodings.

Normalize a command's insert/delete sets once and judge the complete candidate, including refused competing key rows. Install unique indexes only after judgment. Return complete violated-statement IDs with bounded examples or a resource failure—not incomplete rejection disguised as completion. Retain exact final-state set semantics and closed vocabulary distinctions.

Update Lean from the chosen denotation rather than preserving obsolete theorem premises. Prove the actual mutable support definition; model canonical float normalization/order and exact aggregate bounds. Independent model tests compare values, state transitions and specified errors. Keep unsupported trust boundaries explicit.

Delete: unchecked public constructors, hash-as-equality axiom, default immortal dictionary, order-dependent diagnostics, obsolete fresh-allocation semantic assumptions. Exit: G02/G03 and relevant E/F/P/CONC children pass on new representations; downstream safe API attacks are included.

## M2 — Make LMDB's strengths the default

Build one environment owner with kernel-held directory exclusion acquired before cleanup and released last. Implement coherent owned read snapshots, per-open identity, core-local generation, real native close and the transaction gate required for safe map resize. Remove the 32 GiB constant as product policy; grow geometrically subject to actual platform/disk feasibility.

Implement the private candidate path and the narrow host adjunct: checked prepare/admit → log computes outcome → seal opaque host records → commit or abort. The sealed capability cannot mutate application facts. Metadata-only/no-op and domain-rejection decisions must use the same atomic storage boundary. A failed seal, including MAP_FULL after decision hashing, causes no remote dispatch.

Keep the transaction on its owning worker across a bounded hosted attempt. Map-full before publication aborts/retries immutable work after safe resize; it never reruns application code. Local failure after known hosted publication preserves the published outcome and faults only the materialization.

Delete: expiring local leases, separately committed generation authority, GC-only close, snapshot metadata gathered from multiple source transactions, core fresh reservations and benchmark durability escape hatches in ordinary production API. Exit: G06/G11 substrate obligations and deterministic crash/resize/visibility schedules pass. Larger-data qualification begins here and finishes on complete product artifacts.

## M3 — A complete disk path and optional warm kernels

Implement cursor/index-nested-loop execution as the complete baseline. Build one scratch-map abstraction with charged RAM and temporary-LMDB backing; share it across projection distinct, group binding/state, derived relations, recursive frontiers and sealed results. It is not a second persistent database or an external-sort framework.

Keep compact Boolean query structure instead of mandatory exponential expansion. Charge prepare, parameters, growth, intermediate work, aggregates and output before growth. Publish results only when evaluation/finalization is complete. A consuming result cursor transfers one sealed backing owner; streaming delivery failures do not turn a prefix into a complete answer.

Implement exact integer widened sums and deterministic F64 arithmetic/reductions. Qualify the floating environment guard, architecture/compiler behavior, exact 34-limb accumulation and exact-rational mean rounding. The independent bit/rational oracle must not reuse production numerical helpers. Do not weaken deterministic answers for faster reduction.

Retain warm SIMD/Free Join acceleration only behind correct budget reservation and fallback. Forced scalar/cursor/spill execution must agree with optimized paths, including errors and float bits. Tune after the baseline exists; missing measurements are not an excuse to require a whole relation in RAM.

Delete: mandatory full images, output mutation on failure, hidden unbounded prepare/recursive paths, order-sensitive float reduction, invalid numerical rewrite rules. Exit: G04/G05/G12 semantic/fallback tests pass on small forced-spill fixtures, and controlled large-store campaign is executable.

## M4 — One internal durable history machine

Implement LocalHistory and HostedHistory as explicit variants using the same canonical command resolver/outcome grammar. LocalHistory publishes through one durable LMDB commit. HostedHistory publishes through the tenant HEAD CAS over one immutable symbolic-command decision and a bounded recovery tail. LocalHistory does not simulate that object store.

Add stable command identity, exact-state condition, durable no-change/rejection outcomes, three distinct coordinates and winning-decision fresh identities. Resolve uncertainty by retained receipts; closed/retired command epochs cannot become new commands by absence. Concrete fresh IDs never escape before publication. TypeScript supplies owned command values, not retryable arbitrary host callbacks.

Run the independent history model through client-visible schedules during implementation, not as a final cleanup exercise. Include candidate reads, delayed CAS, lost response, later decisions, checkpoint/receipt movement, live-handle next commands, close and post-publication local failure. Distinguish exact known success, definite precondition failure and unknown transport outcome at the real adapter boundary.

Delete: per-braid slots, vector recovery floors, split commits, writer-ID fencing/counter objects, public raw mutable replica access and the second TypeScript protocol. Exit: G07/G09 and PROTO children pass in deterministic models and real local adapter tests; preliminary real-S3 qualification begins in an explicitly authorized disposable environment.

## M5 — Recovery and deletion that preserve the one history

Implement streamed complete logical checkpoints from one RO snapshot, bounded manifests/buffers, exact suffix rebase and fully verified staging activation. Checkpointing must progress during ongoing writes; it does not restart a whole export on every head move. Expose tail backpressure and maintenance health.

Implement explicit named roots/hydration holds, receipt retirement and hosted epoch barrier/mark/sweep with exact-parent reference introduction. GC cursor/phase changes are fenced by barrier identity; late old uploads are future orphans, not publishable missing dependencies. Local restore points use independent complete export directories and owner-scoped cleanup, not remote epoch GC.

Add independent backup/restore primitives in the internal log and TypeScript surface. Restore a writable copy into a new incarnation; preserve old entity IDs as data. Test without the original cache/origin and with independent backup permissions. Do not call an active-store pointer an independent backup.

Delete: scratch-as-deletion-authority, age sentinels/default PITR promise, historical slot/token scans, partial hydration accepted as empty state, retries that lose discovery evidence. Exit: G08/G10 and STORE/LOCAL/GC/FS/S3/REC/BACKUP/RESTORE/ERASE children pass in their declared lanes. Real S3 is mandatory for the hosted claim.

## M6 — The application-facing product

Replace native lifetime-erased callback pointers with bounded generation-tagged capabilities where needed. Make TypeScript owners, borrows, commands, snapshots and results explicitly disposable, with small inert wrappers after native close. Keep core Rust guards genuinely lifetime-safe and core C's raw pointer obligations honest. Build bounded asynchronous Node workers so hosted/native work does not block unrelated requests.

Deliver the TypeScript log API, repo-local migration manifest/ordered steps, explicit `migrate()` workflow and server-only Next.js + Alchemy example in [33](33-typescript-migrations-and-apps.md). The migration runner validates history/checksums, freezes source authority, applies explicit TypeScript transforms outside native transactions, stages and admits a new incarnation, verifies it, then returns a still-frozen `ReadyToSwitch` target and activation reference. Activation and application binding cutover are explicit, with a durable activation marker resolving lost responses. Automatic abort/thaw is only available while nonactivation is proven; after activation even no-change decisions can make rollback unsafe. An interrupted migration has a visible operation identity; ordinary app startup never quietly rewrites a production tenant.

The example must work from installed packages with actual deployment runtime, native bundling, IAM attachment, refreshed credentials, authenticated tenant mapping, deadlines and errors. Alchemy provisions only ordinary resources the example needs; it is not a newly invented database control plane. Local development uses LocalHistory. Hosted Node uses S3 plus enough real local disk for materialization. An Edge/ephemeral host without that capacity is explicitly incompatible, not magically repaired by TypeScript types.

Remove public Rust/C log headers/exports/docs. Internal Rust tests remain indispensable; lack of a public Rust SDK is not lack of a tested native machine. Package core C separately with its real ABI/support matrix.

Delete: source-mutating packing hooks, unchecked native-version pairing, event/body auth shortcuts, automatic implicit migrations, and duplicate cross-language log API maintenance. Exit: API/RUN/FFI/PKG plus chapter 33 migration/application children pass, including pristine consumer and production-shaped deployment tests.

## M7 — Close the ledger, then measure the whole product

Run the entire [70](70-test-and-release-gates.md) inventory against fresh artifacts, not only tests filtered by changed source paths. Every detailed child has an executed lane and evidence; a broad green job does not cover missing float/FPU, local migration or GC cases.

Run the physically populated >40 GiB fixture, separately enforced >RAM workload, cross-platform float corpus, process/power-failure qualification, real S3 conditions, backup-to-clean-restore, migration crashes/history divergence, noisy neighbors, stale handles and package mismatch. Count live resources and cancellation-to-quiescence, not only final exceptions.

Measure intended application schemas across warm/cold/after-write/>RAM/multiwriter/maintenance regimes. Compare only equal correctness/durability. Hold every surprising regression for an explicit cost decision or fix. No universal “fastest database” claim is earned by a cherry-picked kernel measurement.

All known supported-behavior defects close with fix, permanent regression and independent review. Removed scope is documented, not hidden as a passing test. Unfinished proof, unsupported required runtime, missing cloud credentials, stale artifacts or an unrun large-data lane leaves qualification incomplete.

## M8 — Promote exactly what passed

Build release artifacts once in staging from the clean candidate revision. Record toolchains, dependency locks, supported targets and all artifact digests. Test those artifacts through isolated installation, API examples, migration drills and backend qualification. Packaging must not rewrite the checkout.

Complete G16 before release authorization/tagging. Publish the tested platform dependencies and packages in the declared order; verify downloaded artifacts equal the staged artifacts and exact native pins resolve. Do not rebuild a different binary during publication. If post-publication installation verification fails, report and repair the release; do not pretend a tag means success.

No source implementation, production migration, cloud resource creation, package publication, or 1.0 tag is authorized by the current proposal-writing phase. The next authorized campaign starts at M0/M1 with these documents and the preserved audit as its contract.

## Stop conditions that protect the design

If the complete checkpoint/one-tenant-CAS cost is unsuitable for the measured application, return to the explicit tradeoff rather than quietly adding braids, remote page trees or a second authority. If a numerical proof or FPU bridge fails, fix the implementation/domain specification before claiming deterministic floats. If the TypeScript migration runner needs an online fleet coordinator, keep 1.0's explicit per-tenant downtime and report the requested expansion separately.

The implementation should become smaller in conceptual machinery even if the test corpus gets much larger. This campaign succeeds when a few mechanisms explain the entire product—and every selected guarantee has evidence.
