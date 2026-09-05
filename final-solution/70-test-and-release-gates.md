# 70 — The complete 1.0 test and release contract

**Status: required future work. None of the successor gates below is claimed passed by writing this proposal.** The previous audit's 2,049 passing Rust tests, 209 selected SDK tests, and 277 conformance cases describe the audited 0.x tree, not 1.0 qualification.

The owner's release rule is binding: flesh out the missing suite, fix every known issue, and pass every required gate before 1.0. This chapter makes that rule executable without inventing another testing platform. Keep Cargo/nextest, Lean, the Node runner, ordinary subprocesses, small independent models, and the existing measurement discipline. Add a small release-result manifest/check, not a workflow engine.

## Green is a complete evidence set

Each release gate returns one of `Passed`, `Failed`, `NotRun`, or `NotApplicable(scope_reason)`. Only `Passed` satisfies a required cell. A missing environment variable, zero matched tests, a timed-out process, an ignored test without its designated lane, or an unavailable runner is **not** a pass.

`NotApplicable` is for declared scope, such as an ARM-specific instruction assertion on x86—not a way to excuse a failed portable correctness test. Every required property must have a designated applicable lane. The release checker rejects missing cells, stale revisions, wrong artifact hashes, and unresolved audit obligations.

Each result names source revision, specification revision, toolchains, OS/architecture/libc/filesystem/backend, feature set, test inventory, executed/skipped counts and reasons, random seeds, and artifact digest. Timings in correctness logs are not performance claims.

No correctness defect can be waived by lowering a test count, changing a golden to the observed bug, renaming an error, disabling a test, or calling a failed environment unsupported after the run. Supported scope is chosen before qualification. Obsolete 0.x feature tests can be replaced only with a documented specification change and preserved safety coverage.

## Existing machinery to preserve and correct

The current repository already has meaningful layers:

- `scripts/battery.sh`: workspace format/lint/tests, feature checks, Lean/conformance, SDK build/test/type/lint and packed import.
- `scripts/check.sh`: documentation tests, allocation gates, feature combinations and observational harness checks.
- `scripts/lean.sh` and the conformance corpus: abstract semantics plus empirical cross-implementation comparison.
- Separate `ts/crate` and `crates/bumbledb-c` checks exist in the audited tree; retain the native Node lane and remove the C crate/lane with affirmative absence checks.
- `.github/workflows/ci.yml`, `bumbledb-log.yml`, and the old `c-abi.yml` identify existing macOS ARM64/Amazon Linux lanes. Port applicable platform/native properties before deleting the C workflow; qualify actual x86 Vercel Node deployment too.
- `scripts/miri.sh`: selected low-level code under native and cross-interpreted targets.
- Independent naive/SQLite oracles, compile-fail tests, kernel/disassembly checks, allocation measurement and packaged-import tests.

Fix the gaps rather than discarding this investment. Current CI explicitly skips S3 smoke with exit zero when credentials are absent; release qualification must expose that as `NotRun`. Miri filters and root workspace checks do not cover all FFI or real LMDB behavior. Path-filtered CI is useful during development, but a release runs the full declared matrix regardless of changed paths. Build/source tests must not accidentally use an old native artifact from an ignored directory.

## Gate inventory

| ID | Required evidence | Primary scope |
| --- | --- | --- |
| G00 | Complete specification/audit/test traceability and approved breaking changes | Every known issue and every public guarantee |
| G01 | Clean pinned builds, lint, types, docs, features and dependencies | Rust, Lean, TS; all published artifacts and complete C removal |
| G02 | Canonical scalar/row/schema/command codecs, especially floats | Bytes and cross-language value identity |
| G03 | Admission, final-state constraints, diagnostics and proof correspondence | Engine semantic correctness |
| G04 | Query denotation, optimizer equivalence, error outputs and arithmetic | All admitted query forms |
| G05 | RAM/disk/scratch equivalence and larger-than-memory operation | No mandatory whole-relation RAM input or 32 GiB cliff |
| G06 | LMDB transaction, snapshot, resize and local durability schedules | Core physical state and opaque attachment coherence |
| G07 | Independent log history model and deterministic concurrency schedules | Single tenant publication authority |
| G08 | Actual backend contract qualification | OS ownership, filesystem durability and real S3 conditions |
| G09 | Named commands, witnesses, concrete application IDs and receipt retirement | Application-visible intent and retry semantics |
| G10 | GC, checkpoint, backup/restore and migration drills | `bumbledb-log` only |
| G11 | Lifecycle and ownership, Rust/Node native safety, repeated resource reclamation | Borrow/owner/operation/result handles |
| G12 | Work, queue, scratch, memory and cancellation behavior | Bounded host execution without a database-size cap |
| G13 | Fresh native packages, platform/ABI compatibility and real consumers | Registry-shaped release artifacts and examples |
| G14 | Untrusted input, authority boundaries and corruption refusal | Parsers, request adapters, cache/provenance isolation |
| G15 | Measured performance envelope and regression decisions | Warm, cold, mutation, larger-than-memory and hosted paths |
| G16 | Complete exact-artifact release packet | No release tag/publication before all required cells pass |

## G00 — Preserve the issue and contract evidence

Use [50 — Audit closure matrix](50-audit-closure-matrix.md) as a finite work inventory. For each ID record the selected representation, which old mechanism disappears, a falsifiable successor property, permanent test names, and fix/review evidence. Reproductions remain in `audit/`; do not rewrite the dated observations.

The original regression should fail against the audited behavior where practical. If the old API is deleted, retain a baseline reproduction plus successor compile/API and behavioral tests proving the dangerous capability no longer exists. “We deleted the test because the method was renamed” is not closure.

Also cover non-indexed observations in the FFI/packaging and hosting reports: payload error classification, package mutation, ABI compatibility, actual Linux runtime, base64 HTTP bodies, method validation, and sensitive diagnostic output. This matrix is a minimum, not a ceiling on new discoveries.

## G01–G03 — The trusted representation boundary

### Builds and surface compatibility

- Clean-checkout builds use the pinned nightly and lockfiles, not a developer's prebuilt native library. Core default, supported feature combinations, log without S3, and log with its S3 feature compile and test.
- Include the separately built native Node crate and all Rust consumers. Prove the C crate, public headers/exports, packaging hooks, examples, workspace references and dedicated workflow are gone. Test public examples as downstream consumers, including compile-fail invalid capability use.
- Regenerate native descriptors and schema/plan artifacts into staging and compare; no in-place mutation of the release source tree is necessary.
- Reject unsupported storage/protocol families before any cleanup or write. A reset numeric format version under new magic must not accept old v1/v8 fixtures.
- Qualify one shared Node artifact/runtime per supported platform, used by both core and log packages. Rust core dependency checks remain log/AWS-free; importing the core Node package starts no transport or log maintenance. Duplicate runtime handles refuse safely rather than crossing addon pointers.

### Canonical values

For every scalar and tuple encoding assert round trip, canonical idempotence, exact equality/hash agreement, lexicographic key-order agreement where promised, truncation refusal, width limits and typed wrong-schema rejection.

Float fixtures must include both zeros; the smallest/largest subnormals; the normal/subnormal boundary; adjacent representable values; maximum finite values; both infinities; many positive/negative signaling/quiet NaN encodings; halfway rounding; overflow; underflow; and integer-cast boundaries near 2^53, i64 limits and u64 limits. Compare **canonical bits**, not approximate host numbers or ordinary JSON serialization.

Run Rust/Node ingestion, storage/reopen, key lookup, membership, grouping, joins and result marshaling against the same corpus on Apple Silicon, ARM Graviton and x86 Vercel's declared CPU/runtime floor. TypeScript's `number`, JSON's lack of nonfinite values and the host FPU environment are separate concerns; none may choose a different database equality relation.

Float property tests use a simple independent bit/rational reference, not the production conversion routine. Force wrong-rounding and FTZ/DAZ host environments where supported. Verify numerical execution establishes/restores its contract and that foreign host settings cannot silently change persisted or returned results.

Public supported ingestion must not turn malformed interval/bool/text/byte/float rows into successful corrupt state. Force digest collisions with a test-only tiny/constant hash and verify full-value equality still decides fact identity and constraint results, including long values above LMDB's key-size bound.

Float intervals require dense numeric denotation fixtures, not enumeration of representable floats: adjacent finite endpoints, gaps between adjacent endpoint values, left/right rays, both infinities, NaN refusal, canonical zero and bound parameters. Allen masks, coverage, packing and pointwise laws use exact endpoint order. Nonfinite point membership is false; unbounded measure and overflow of a bounded length are different errors. `FixedInterval<F64>`/float-width interval compression and float capacity weights remain refused; ordinary F64 and float intervals have fixed-size canonical payloads. Local fingerprints and authoritative content digests have separate width/domain/golden/collision tests; shortening a fingerprint cannot remove exact comparison.

### Admission and laws

- Compare committed/aborted state and the complete promised set of violated statements with an independent full-state evaluator. A boolean “rejected” is insufficient.
- Cross typed/dynamic/FFI inputs, heap admission and LMDB delta admission, inserts/deletes/replacements/no-ops, multiple constraints, closed vocabularies and temporal boundaries.
- Preserve the fresh-refused-key counterexample's semantic intent even though the old fresh-ID physical mechanism is gone.
- Prove/model only the actual admitted language. Update the Lean mutable-support/closed-relation premise rather than citing the old braid theorem as publication proof.
- Reject `sorry`, unreviewed axioms, missing theorem cases and an empirical bridge that silently regenerates expected outputs from production code. Explicitly document hardware, LMDB and S3 assumptions outside the formal model.
- Grouped-measure tests cover supported alias normalization in Rust/TS/dynamic schema inputs, count as unit weight, zero-weight members, empty child groups for existing parents, missing-parent vacuity plus separate containment, dependent upper bounds and exact duration. Do not preserve cosmetic spelling failures as semantic laws or lower count-existence to containment without its key/admission premises. No pointwise temporal occupancy or weighted-bag semantics is implied.

## G04 — Query semantics and optimizer legality

Run every admitted operator/type pairing through the naive model, optimized engine, optimization-disabled engine and the SQL-compatible subset through SQLite. Assert the legal pairing inventory is populated, not just a large random-case count. Closed relations, disjunction, negation, recursion, temporal operations, grouping and parameter sets must be tested in combination.

Nonrecursive relation composition includes aggregate/computed outputs consumed downstream. Test distinct-student projection then count versus attempt-binding count; equal-valued weights on distinct bindings; inner no-group behavior; inner overflow/cast/measure error followed by an outer false filter; and one-rounding at each stage. Inline/materialized/forced-spill executions must agree on facts and semantic errors. Frozen finite computed predecessor relations may feed the one positive linear closure; rejection tests cover aggregation, value invention, negation and mutual recursion through the cycle. Names alone are not materialization directives.

Float rewrites need explicit negative tests: no reassociation of primitive arithmetic, no implicit FMA, no `x-x -> 0` for arbitrary nonfinite x, no `x/x -> 1`, no comparison substitution based on IEEE `NaN != NaN` when database equality is canonical equality. Equivalent plans and input permutations produce identical exact reduction bits. Cancellation and resource checks must not license a truncated set result.

For sum/mean, include catastrophic cancellation, mixed signs/exponents, all-zero groups, NaN/infinity combinations, very large groups and different merge/spill partitions. Verify exact accumulator merge laws independently and the specified one-rounding result. Min/max follow the selected relational total order; they do not inherit a host library's NaN-skipping behavior accidentally.

Assert the exact result state after each error: overflow, decoding error, foreign plan, bad bind, cancellation, scratch failure and invalid numerical input. Reuse prepared plans/results through success/error/success sequences. A result becomes observable only when the API's promised completeness/provenance is established.

## G05 — Larger than memory is an ordinary execution regime

Every query family runs with RAM acceleration on, forced off, and a tiny cache/scratch budget forcing transitions into temporary LMDB. Compare exact canonical result sets and float bits. Include distinct, grouped reduction, recursive visited/frontier sets, text/bytes and many-to-many joins. Forced transitions occur before, during and after the first result/group/frontier, not only at a large final size.

Use one scratch-map implementation across operators; test its spill/cursor/lifetime behavior thoroughly instead of introducing multiple external algorithms just to test them. A disk fallback may be slower; it must preserve denotation and continue making bounded progress.

Release qualification includes both:

1. **Data substantially exceeds resident memory.** On an isolated Linux runner, constrain memory with an appropriate cgroup and use a physical dataset several times larger. Do not use an address-space limit that prevents a legitimate sparse LMDB map and then call that a RAM test.
2. **Physical database crosses the former 32 GiB boundary.** A dedicated storage-qualified run grows and reopens an actually populated database beyond that boundary (not just a large virtual map or sparse empty file), mutates/query-checks both sides, checkpoints and restores it with bounded memory. A practical minimum fixture is over 40 GiB with an explicitly recorded smaller memory allowance.

Large fixtures use predictable generated data and streaming exact checks so the oracle does not itself require loading everything into RAM. Record page faults, RSS/cgroup usage, file size, virtual mapping size, I/O, result correctness, cancellation latency and temporary-disk cleanup. No universal speed ratio is required beyond memory; no arbitrary database-size rejection is permitted.

## G06 — LMDB is the substrate, not a black box excuse

Test held read snapshots during write, compaction/copy, elastic map resize and close. Rows, local generation and opaque materialization attachment must describe one read transaction. The concurrent-compaction mismatch from ENG-003 must be impossible in the new snapshot API.

Use deliberately small initial maps to trigger many growth events. Test `MDB_MAP_FULL`, external-size adoption where supported, held transactions/cursors, reader-blocked resize, retry of a private candidate, and process death around durable commit. Bounds come from the actual platform/LMDB/filesystem, not a hardcoded product database ceiling.

Inject ENOSPC, read-only filesystem, short write, failed sync, missing/truncated metadata, failed snapshot staging and rename. An error after hosted publication does not change the receipt to rejected. A core local transaction either commits facts plus attachment coherently or does not expose them as committed.

In particular, inject allocation/MAP_FULL/I/O failure **after application judgment and decision hashing, during `HostChanges` sealing**. No remote CAS may have been dispatched. This is not covered merely by a failure before application preparation. The sealed capability cannot amend application facts; a rejected candidate's receipt-only transaction retains the same exclusive writer-session parent.

Qualify the supported filesystem's actual persistence assumptions with subprocess and fault-injection tests. Miri does not model filesystem persistence and cannot substitute for them. At least one release campaign must exercise abrupt process death and the chosen machine-failure/durability simulation; state plainly what remains an assumed substrate guarantee.

## G07–G09 — Histories, not only final convergence

Implement a deliberately small independent reference state machine. It models HEAD versions, immutable objects, decisions, application state, receipts, epochs, retained roots, reads and client-visible outcomes. It does not call production transition helpers or recover expected histories from the same serializer.

Enumerate bounded schedules with at least two writers, one reader, two request IDs, one checkpoint/GC transition and response loss. Add generated longer histories and retain minimized seeds. Deterministic barriers belong around consequential effects: before/after local prepare, upload, conditional dispatch, response receipt, receipt lookup, local apply, metadata commit and close.

Check every observation, not just end state:

- At most one authoritative successor per HEAD version, and no ABA/reuse after maintenance or logical deletion.
- Every terminal receipt has exactly one stable named outcome; every ordinary read is a published prefix meeting its declared freshness/witness request.
- A rejected candidate never becomes visible before final convergence.
- Same ID/same digest retries return the same outcome; same ID/different digest refuses; retired IDs never execute again.
- Two witnessed decrements either enact the intended serial changes or return explicit precondition failure. Blind set-write semantics remain separately tested.
- Entity IDs are generated once outside native submission and copied into sealed commands. Retries, response loss and restore preserve those bytes; no FreshRef/reservation/result-map API survives. Duplicate IDs follow ordinary schema laws, not a claim of collision-free issuance.
- A failed/unknown attempt followed by another request on the same live handle preserves resolution evidence.
- Captured-tip refresh does not chase infinite concurrent work. Every retry loop has a bounded outcome under contention or a stuck peer.

Test `Committed`, definite mismatch and uncertain transport outcomes independently. Cases include error before dispatch, server applies/response lost, another caller wins, original request remains in flight, truncated verification GET, checkpoint advances before resolution, receipt epoch closes/retires, and local LMDB commit fails after remote success.

### Real adapter qualification

For supported local filesystems, suspend an owner longer than any old lease TTL and attempt competing open/mutation. The successor must respect the actual OS lock, not delete or mutate the live owner's state. Resume the old process and verify no stolen ownership interval existed. Test crash/death lock release and directory teardown separately.

For S3, use an explicitly provisioned disposable test prefix and least-privilege test credentials. Exercise actual conditional create/update and opaque ETag handling, response-loss proxies or equivalent fault injection, concurrent writes, repeated reads, multipart/streamed snapshot handling, aborted uploads, deletion, permissions failure and restore. Test the exact bucket/service mode being advertised; an S3-compatible mock does not qualify AWS or another vendor automatically.

Credential absence blocks S3 qualification. Cloud tests must never run against application prefixes or create/delete resources implicitly from ordinary source tests. The release workflow supplies the authorized test scope.

## G10 — Log-layer recovery, retention, backup and migration

The main engine gets snapshot/admission tests, not a migration framework. These operations and their runbooks live in `bumbledb-log`.

The public log product is TypeScript-only. Internal Rust model/adapter tests remain required; public Rust log API tests are not a promised surface and the entire public C product is removed. Rust/TS core behavior remains fully qualified. LocalHistory's one-LMDB authority and independent local restore directories receive their own LOCAL-* crash tests; no hosted tail/epoch envelope is imposed just to reopen a local database.

- Publish while a checkpoint streams; advance the tip; publish the old coherent checkpoint with a validated retained suffix. It must not require a whole-database quiet period or re-copy on every conflict.
- Advance a GC epoch while an old writer/checkpointer is paused at each upload/CAS boundary. Old objects cannot be introduced into live history after the deletion barrier.
- Race mark/sweep with new objects, restore-point creation/release, hydration pins and failed deletes. Abandoned staged uploads must remain safely collectible; failed deletion must retain resumable discovery state.
- Capture a root atomically before hydration. Revoke/release that root during hydrate and verify no incomplete/wrong snapshot is returned. A complete local snapshot's reads do not secretly depend on a remote pin still existing.
- Lose all local cache files and replay from a clean directory. Verify every retained terminal receipt and application fact, including post-publication local errors.
- Named restore points preserve their complete checkpoint+tail closure. Explicit release changes their availability contract; restart or clock changes do not release them automatically.
- Recover under another origin/cache mapping. Same schema and same revision with foreign data must refuse or reseed before serving, writing or cleaning up anything.
- Migrate through explicit freeze/export/transform/admit/import/new-incarnation/cutover steps. Kill at every step. Old writers cannot resume into the new history, invalid transformed state cannot become current, and original data is not overwritten by a failed migration.
- Race pre-activation abort against activation and delayed genesis, including an absent target and local final-directory install. Durable terminal target fencing precedes source thaw; uncertainty leaves the source frozen. Deleted authorities have no active recovery root, preserve explicitly retained roots/barrier progress, and eventually permit collection without reopening their namespaces.
- Run the schema generator and installed migration runner end to end: identical declarations produce identical canonical plans; automatic changes need no authored migration; rename/backfill/destruction ambiguity refuses without declarative intent. Check ordered plan/history identity, missing/edited/divergent history, baseline, bounded native evaluation, repeated operation identity, frozen-source recovery and expected-old cutover. Multiple pending plans build one final incarnation while preserving necessary intermediate checks; test composition against sequential denotation, including deduplication and rounding boundaries. No runtime imports user migration callbacks; no ordinary request silently migrates a tenant. Chapter 33 defines the exact child inventory.
- Test 0.x format refusal, offline conversion with the declared matching old reader, new-format round trips, and restoring a backup with a new history identity. Never reset the version counter without a distinguishing format family.
- Restore external blob references as part of the application drill. A relational restore whose required blobs were deleted is not a successful application restore.

There is no default time-window PITR guarantee in the selected 1.0 contract. Removing it is explicit. Any future time-window policy requires additional clock/retention proofs and tests before being advertised; it cannot be smuggled back in as an undocumented helper.

## G11–G12 — Ownership, resource use and cancellation

Both TypeScript packages implement chapter 35's exact Effect 4 contract. Extend existing API-01/04/07/10/12, RUN-01/02/04/10 and FFI-05/07/08 with lazy construction, one-shot mutation reruns, interrupted acquire/late native completion, scope/finalizer-installation races, explicit CloseReport versus CloseFailure defects, and known receipt followed by scope failure. A fiber's Cause.Interrupt is never a fabricated NotSubmitted; the pre-dispatch retained ref resolves after reopen. Full drain or explicitly retained Closing ownership is required, not listener removal. Use Effect 4 TestClock for JS time and real/injected native clocks for native deadlines; do not confuse them.

Completed-result page Streams replace the public TS cursor API. Test first-run consuming transfer, second-run refusal, early take, downstream failure/interruption, EOF, oversized row, escaped scope, close/collect races and scratch reclamation with GC disabled. All rows are complete before page delivery. No per-row Effect work is required. Native workers and OS locks are tested independently of JS wrappers.

Run borrowed-owner state sequences as finite model tests and through real Rust/Node handles. Include double release, stale release after reopen, close while opening, close with queued/in-flight requests, retained writer after close, foreign database/plan, use after callback scope, repeated dispose, leaked client borrow, registry slot reuse and generation exhaustion where relevant.

Verify native environments and locks actually release after the last authorized operation drains—not merely that a wrapper throws `closed`. Repeated open/read/query/close cycles must reach a stable resource envelope. Track file descriptors, mapped files, native owners, temporary LMDBs, timers, threads and memory, not just JavaScript heap. Internal Node diagnostics must not retain payloads with historical operation count.

Miri covers applicable pure unsafe Rust; ASan/UBSan/LSan or appropriate platform tools cover the internal Node/native subprocess paths. Buffer pin/copy, thread handoff, cancellation and close must obey that boundary's actual ownership contract. Removing public C eliminates its raw-pointer contract, not unsafe Rust or native lifetime obligations.

Memory pressure trims optional caches and moves scratch work to LMDB rather than rejecting a database solely because it is large. Real disk/address-space exhaustion, an unrepresentable scalar, or a configured request deadline returns a precise error. Queue and worker limits control concurrent work; they are not a hidden database-size cap.

Cancellation is tested inside scans, image/cache construction, grouping, exact float accumulation, recursive rounds, spill-map transitions, snapshot streams, object requests, receipt resolution and shutdown. Use measured maximum checkpoint/work intervals. A JavaScript Promise timeout while native work runs forever is a failure. After remote publication, cancellation reports/retains publication certainty and cannot imply rollback.

Include two-tenant noisy-neighbor tests: one slow/large request must not indefinitely stop the other tenant's progress or lifecycle operations. Worker isolation is a small runtime boundary, not a fleet scheduling product.

## G13–G14 — The artifact and public boundary are the product

Packed TypeScript consumers must infer exact Effect A/E/R, Scope acquisitions, Option get results and typed core/log errors using the required **4.0.0-rc.112** peer dependency. Compile chapters 33–35 against fresh packages. Assert no Promise/sync/AsyncDisposable twin or `/effect` adapter, no core log import, and direct core QueryReader/ChangeSet/page Stream/codec/NativeRuntime reuse by log. Test the app's one ManagedRuntime with concurrent requests and abort at its outer framework boundary; no layer-per-call or second tenant cache. Generation/admin interruption tests use the original stable operation identity. These strengthen existing PKG-03, TS-MIG-10 and APP-03/08, not a parallel release system.

Build exact native packages for the declared darwin-arm64, linux-arm64 and linux-x64 roster (or a deliberately revised prequalified roster). Exercise the oldest declared Node runtime as well as the current supported runtime. Linux artifacts run against their documented libc floor, not just any container with the word Linux on it.

Pack from staging manifests. Install tarballs into empty consumers without workspace links/dev dependencies; import core and log separately; run create/write/read/query/reopen/close and hosted test-backend commands; typecheck downstream declarations and verify no public C artifact/export survives. Assert SDK/native ABI/format compatibility at load. Check mismatched artifacts refuse rather than calling the wrong export layout.

Fuzz bounded parser and boundary inputs: wrong type/width/schema, unknown tags, duplicate/conflicting operations, malformed UTF-8, pathological lengths, noncanonical floats, recursive size limits, object digest/length mismatches and foreign identity. Reinstate an actual corpus-replay/fuzz campaign where useful; a past preference for deleting fuzzing is not a 1.0 correctness constraint.

HTTP/example adapters validate method/path/event shape and size, decode base64 correctly, and require a host-supplied authenticated tenant mapping. Test logs/errors do not emit private fact payloads or credentials by default. This is scoped boundary qualification, not a claim to have built a full authentication product.

The Next.js/Alchemy and x86 Vercel Node examples are release consumers: production build, server-only imports, native asset inclusion, selected Node/libc/CPU floor, actual local-disk envelope, real IAM/credential rotation, local development, generated deployment migration and schema/history mismatch on ordinary open. Client/Edge imports must fail usefully. The Expo/Drizzle analogy does not qualify a React Native or browser runtime. The >40-GiB engine lane runs on fitting hardware, not a serverless scratch directory without that capacity.

Chapter 34's proposed Rust/core-TS/log-TS syntax becomes executable consumer fixtures during implementation. The same core schema, scalar/ID, ChangeSet, query template, typed parameters, QueryReader helper, CompleteResult and value codec must work across the applicable surfaces without application adapters or duplicate brands. Log-only identity/receipt/freshness remains outside core exports; a shared read helper gains no write capability. Generated migration plans invoke the same core query operators and stage semantics. API-12, FFI-08, PKG-03 and TS-MIG-04/07/10 own this evidence; no new SDK testing framework is required.

## G15 — Earn the performance claims without turning them into superstition

Include the shared core/log Effect/V8 envelope from chapters 35/40: native/bridge/Effect/full-app decomposition, warmup, allocation/GC/external-memory plateau, bytes copied, stable versus polymorphic row shapes, event-loop tail delay, bounded page pull and cancellation under saturation. No per-tuple fibers/spans or duplicate log conversion. Correctness plus an Effect return type is not evidence that the main thread stays responsive.

Use actual intended application schemas where available. Cover booking/capacity, knowledge/graph metadata, and event/ledger identity; report each schema's coordination domain and unsupported query shapes.

Measure warm Free Join/indexed reads, first query after insert/replace/delete, cold reopen, beyond-memory cursor execution, scratch spill, cache churn, command sealing with application IDs, single/multiple remote writers, checkpoint/GC overlap and recovery. Include small per-student tenants, not just a single large relation. Report absolute latency distributions, throughput, peak/resident memory, virtual map/file sizes, temporary disk, bytes decoded/copied and object operations per terminal decision.

Chapters [40](40-performance-contract.md) and [41](41-storage-and-hashing.md) define mandatory workload/constant/storage/hash families. Revisit batch/load/prefetch/cache thresholds with falsifiers and in-situ antagonists. Separate machine facts from backend bounds and arbitrary policy. Account for per-namespace live key/value bytes, LMDB page occupancy/free pages, raw/compacted files, OS allocated blocks and indexed SQLite/WAL under the same data/index/durability conditions. Attribute the recorded 2.3–2.45× gap rather than labeling all overhead a Free Join tax. Compare 16-byte exact-checked local fingerprints, full authoritative digests and the AEGIS candidate on actual short-row and bulk inputs before freezing a new algorithm. No hash throughput claim or space-saving percentage is established by these documents.

Retain all runs and configuration; do not select only best medians. Performance baselines compare identical correctness and durability semantics. Host-specific assembly/allocation assertions remain scoped measured claims; they do not replace application benchmarks or force an architecture-wide no-allocation promise.

A result that is slower than 0.x is investigated and accepted only with a documented tradeoff or fixed before release. Do not invent a universal speed target without workloads. The nonnegotiable cliff test is that beyond-memory and beyond-32-GiB data continue to execute correctly through the same public semantics.

## G16 — Release packet and promotion

The final release candidate is built from a clean committed tree. The packet includes:

1. Specification and breaking-change inventory, including float semantics and the new format family.
2. Closed finding matrix with fix/test/review references and no unresolved known defect in supported behavior.
3. Every required G00–G15 matrix result for that revision or a demonstrably identical promoted artifact.
4. Exact source/native/package/binary digests, toolchains, dependency locks and generated schema/plan/native-descriptor provenance.
5. Restore/migration, ownership, real S3 and large-database qualification records.
6. Published workload results and explicit platform/backend/failure-domain limits.
7. Tested installation, diagnosis, backup/restore, migration and rollback instructions.

A small machine check validates completeness and identity before tagging/publishing. It rejects credential-skipped S3, stale native artifacts, surviving public C artifacts or missing Node safety coverage, unrun large-database qualification and open audit rows. Human review checks that the specification was not weakened just to make the packet green.

Separate prepublication qualification from distribution verification to avoid a circular gate. Before public promotion, exact staged artifacts pass all semantic/backend/platform/migration tests plus clean installation and a disposable/private registry publication rehearsal. After authorized publication, download the public registry artifacts, verify digest/pins/install, and only then declare release completion. A distribution mismatch is a release incident, never retroactive permission to ship unqualified code. The actual public registry cannot prove a version available before that version has been uploaded.

The proposal commit is **not** this packet. It starts the implementation and qualification campaign. No `1.0.0` release tag, registry publication, data-layout reset in shipped code, or production migration is performed by this documentation phase.

## Detailed child gates are mandatory inventory, not optional examples

The following cross-index makes the chapter-specific families part of the release packet. Numeric ranges below are inclusive and expand to every integer with the shown zero padding. Each family may contain many tests; a family with zero selected cases fails. All listed properties require evidence in their applicable lane; broad G00–G16 success cannot conceal an unrun child. The release checker also scans chapter-defined families and refuses any new child missing from this index.

| Chapter | Complete child family inventory | Parent gates |
| --- | --- | --- |
| [02 concurrency](02-concurrency-and-semilattices.md) | `CONC-01` through `CONC-06` | G03/G04/G06/G07/G09/G15 |
| [10 engine](10-semantics-and-engine.md) | `E-DELTA`, `E-VALUE`, `E-CODEC`, `E-SNAPSHOT`, `E-NO-RESERVE`, `E-ADMIT`, `E-TEXT`, `E-DURABILITY`, `E-VISIBILITY`, `E-ORIGIN`, `E-LARGE`, `E-BRIDGE` | G01/G02/G03/G06/G09/G11 |
| [11 floats](11-floats.md) | `F-CANON`, `F-GOLDEN`, `F-ORDER`, `F-ARITH`, `F-ENV`, `F-AGG`, `F-SET`, `F-OPT-NEG`, `F-CROSS`, `F-WIRE`, `F-RESOURCE`, `F-PROOF`, `F-INTERVAL` | G02/G03/G04/G05/G11/G12/G13 |
| [12 queries](12-query-execution.md) | `Q-ATOMIC`, `Q-BUDGET`, `Q-DISK`, `Q-LARGE-STORE`, `Q-COLLISION`, `Q-FALLBACK`, `Q-RECUR`, `Q-GROUP`, `Q-TEMPORAL`, `Q-LIFETIME`, `Q-FAIR`, `Q-IR`, `Q-INJECT` | G04/G05/G06/G11/G12/G15 |
| [13 assurance](13-lean-and-rust.md) | `P-KERNEL`, `P-SEMANTIC`, `P-FLOAT`, `P-REPRESENTATION`, `P-DISK`, `P-MEMORY`, `P-SCHEDULE`, `P-ARTIFACT`, `P-PERF` | G00–G15 as the linked lane specifies |
| [20 protocol](20-durable-protocol.md) | `PROTO-01` through `PROTO-20` | G07/G08/G09/G11/G12/G15 |
| [21 storage](21-storage-and-retention.md) | `STORE-01` through `STORE-10`; `LOCAL-01` through `LOCAL-03` | G05/G06/G08/G10/G11/G12/G15 |
| [21 collection](21-storage-and-retention.md) | `GC-01` through `GC-13` | G07/G08/G10/G12 |
| [21 real backends](21-storage-and-retention.md) | `FS-01` through `FS-05`; `S3-01` through `S3-06` | G06/G08/G10/G11/G14 |
| [22 recovery](22-recovery-and-migrations.md) | `REC-01` through `REC-07` | G06/G07/G09/G10/G11 |
| [22 backup/restore](22-recovery-and-migrations.md) | `BACKUP-01` through `BACKUP-05`; `RESTORE-01` through `RESTORE-03` | G05/G08/G10/G14 |
| [22 migration](22-recovery-and-migrations.md) | `MIG-01` through `MIG-14` | G08/G10/G13/G14 |
| [22 erasure/operations](22-recovery-and-migrations.md) | `ERASE-01` through `ERASE-04`; `OPS-TEST-01` through `OPS-TEST-02` | G08/G10/G11/G14 |
| [30 public APIs](30-client-apis.md) | `API-01` through `API-12` | G01/G02/G04/G05/G07/G09/G11/G12/G13/G14 |
| [31 runtime](31-tenant-runtime.md) | `RUN-01` through `RUN-15` | G05/G06/G08/G10/G11/G12/G13/G14/G15 |
| [32 native ABI](32-ffi-and-release-packaging.md) | `FFI-01` through `FFI-08` | G01/G02/G04/G11/G12/G13/G14 |
| [32 packages](32-ffi-and-release-packaging.md) | `PKG-01` through `PKG-06`; `PKG-07A`, `PKG-07B` | G01/G13/G16; PKG-07B public-distribution verification after authorized promotion |
| [33 TypeScript migrations](33-typescript-migrations-and-apps.md) | `TS-MIG-01` through `TS-MIG-10` | G02/G05/G08/G10/G12/G13/G14 |
| [33 application integration](33-typescript-migrations-and-apps.md) | `APP-01` through `APP-08` | G08/G10/G11/G12/G13/G14/G15 |
| [40 application performance](40-performance-contract.md) | `APP-FAST`, `APP-MUTATE`, `APP-NUMERIC`, `APP-LARGE`, `APP-TENANTS`, `APP-TARGETS`, `APP-METHOD`, `APP-MAGIC` | G00/G04/G05/G06/G11/G12/G13/G15 |
| [41 storage/hashing](41-storage-and-hashing.md) | `SPACE-01` through `SPACE-02`; `HASH-01` through `HASH-04` | G00/G02/G03/G04/G05/G13/G14/G15 |

The TypeScript migration/application families are as mandatory as engine/protocol families. A checked history file alone does not qualify migration, and a successful local Next build alone does not qualify native deployment or the attached AWS permissions.

In the implementation result manifest, one row per expanded child records exact test names, applicable platforms/backends, evidence revision/artifact, executed case count, outcome and review reference. This is a small data file checked by the release script—not a new test orchestration service.
