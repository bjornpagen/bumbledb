# 32 — One internal Node boundary; no C product

Execution routing: P06 bridge safety; P13 C deletion/artifacts/platforms; P00 manifests/CI/pins; P12 independent consumers. C09/C12 define artifact identity. See [work packets](62-work-packets.md) for source ownership and complete deliverables.

Status: proposed 1.0 replacement, not an implementation or qualification result. The supported public languages are Rust and TypeScript for the core, and TypeScript for the log. This chapter intentionally replaces the earlier proposal to retain a public C API.

## Delete the whole C surface

During implementation, remove the entire C product: crates/bumbledb-c, public headers and exported C database functions, generated header tooling, C examples/smoke programs, C-only tests and CI jobs, package/build/release targets, manifests, documentation and stale imports/references. There is no deprecated compatibility library, optional C feature, hidden supported header, or public C facade underneath the log. Remove old C artifacts from future release inputs; do not delete historical released artifacts or preserved audit evidence. Node's required internal N-API linkage still exists; that implementation detail is not a reusable public C database SDK.

This is not permission to delete matching engine/Node safety coverage. SDK-013's immortal C callback tombstones disappear with the C surface, but the shared native owner/lock leak and lifecycle schedules must remain as Rust/Node regressions. Keep source audit evidence intact. The release deletion gate proves that the old product was actually removed, rather than merely renamed or left excluded from a workspace test.

The Rust core remains public. Internal Rust log crates own history, receipts, S3 authority, recovery and generated-plan execution, without becoming a public Rust log SDK. TypeScript calls this one implementation through N-API. There is no second protocol state machine, another storage engine, or generic native plugin framework.

The public Rust core crate and its transitive dependencies remain free of log/AWS code. For Node 1.0, select **one native platform artifact per supported platform/version**, containing core plus the internal log implementation. Both TypeScript packages import one exact-version loader/runtime and capability registry; no separate log addon, optional native flavor or cross-addon pointer bridge is shipped. Core-only Node users therefore download log/transport code too—an explicit binary-size cost to measure, not a claim of zero cost. Importing the core starts no S3 client, credential resolution, network activity or log maintenance workers; ordinary core async resources start only when needed. A separate slim Node distribution is not part of 1.0.

The core public barrel exports core primitives only. The log wrapper can access an exact-version private integration entry in that shared native package, not a new stable plugin ABI. Both packages use the same core `ChangeSet`, query, scalar/schema, result and read-capability implementations; TypeScript re-export aliases, if any, cannot create new brands/classes/codecs. Duplicate addon copies are foreign runtimes: matching package versions do not prove pointer ownership, and their handles refuse before use. No independently compiled engines exchange internal pointers.

## Node capabilities own and release actual resources

A JavaScript wrapper contains a small opaque native capability and a live/spent/closing state, not a lifetime-erased pointer to a Rust callback scope. Native owner registration and generation checks prevent a retained wrapper from accidentally reaching a successor resource. Bounded slot bookkeeping may be used internally; its layout is not a public C ABI.

Owner close stops admission, revokes idle child capabilities, drains in-flight operation leases and drops the native environment before releasing directory exclusion. Retaining a closed JS object must not retain an engine Arc, mapping, file descriptor or lock. GC finalizers are only a backstop; Effect scope finalization/early close works with GC delayed or disabled.

Every operation checks the exact owner/slot/borrow generation and resource kind, then acquires a scoped native operation lease before registry locks are released. Close cannot free an operation's data underneath it. No bookkeeping lock stays held through query execution, transport, filesystem calls or user code. Repeated close joins the same close operation; a deadline returns CloseIncomplete and leaves the owner Closing until actual drain.

TypeScript Scope owns every resource-owning handle, including database-free drafts, changes and commands. No Promise/synchronous disposal alias or GC-dependent ordinary cleanup path exists. Early close is Effect<CloseReport>; failed/incomplete scope cleanup is a structured finalizer defect with retained native Closing ownership. A sealing/apply/submit operation captures its native reference at admission; later disposal drops the caller's capability without freeing the active operation's inputs or silently cancelling its work. Small inert metadata wrappers and copied witnesses/identities remain ordinary synchronous values.

Public core Rust snapshots/views obey actual Rust lifetimes. A legal held guard can delay close or map growth and cause CloseIncomplete/ResizeBlockedByReaders; it is never forcibly invalidated. Managed Node snapshots can be revoked while idle because their native guards remain inside the runtime. This distinction is a safety property, not an opportunity to hide unbounded retained resources.

## Value copying, results and errors

Input copying is itself asynchronous library work: bounded host extraction/copy/check chunks yield to the event loop, and native normalization/encoding/hashing executes on bounded workers. A TypeScript ingestion effect's **successful execution** is its acceptance boundary; callers keep mutable rows, buffers, slices and iterator state stable throughout execution. Effect construction is lazy and accepts nothing. There is no call-entry snapshot guarantee for arbitrarily large JS input. After successful execution the native owner no longer depends on those inputs, and local application/log replay consume the same finalized `ChangeSet` even if a caller violated input stability during copying. Query/key parameters and envelope metadata remain stable until their asynchronous operation settles; their required owned checked representation exists before native transaction/execution entry. No worker dereferences JS objects or moving/detachable host buffers.

Single cells/rows, each copy/conversion chunk and total input have finite charged limits. A huge string/byte cell cannot hide a long synchronous copy inside an otherwise async method. Dynamic/untyped callers receive the same tag/shape/UTF-8/width/range validation as typed callers, including defensive checks for detached/resized backing memory. Refuse raw `SharedArrayBuffer` views in 1.0; callers must first make an explicitly synchronized copy into ordinary unshared input. No guarantee is made that mutations before acceptance are detectable or form one atomic source snapshot.

An ingestion/finalization failure spends its draft, blocks new admission, cancels/drains active work and releases accumulated native ownership before completed failure, or reports incomplete drain with retained accounting, including getter/iterator exceptions and reentrant/overlapping calls. Effect finalizers join the same drain or surface explicit incomplete-drain evidence without releasing live resources. Individual input getters/iterator steps still execute as ordinary JS; library chunk limits cannot preempt arbitrary application code that never returns. These host steps are never automatically replayed and execute outside database transactions; an explicit sequential rerun of an insert/delete effect reads its input again while the draft is building. No sync or async application transaction callback is introduced.

Outbound values are owned arrays or bounded copied pages. No ordinary Node method hands out a view into an LMDB mapping that unrelated close can unmap. A CompleteResult is sealed only after full query execution; asynchronous collect performs a bounded, yielding conversion, while a one-shot core pages Stream consumes its source on first run and transfers sealed storage to one private scoped cursor. No public TS cursor/AsyncIterable twin remains. Page reads and row codecs are asynchronous and size-bounded too. No shared/clone cursor subsystem is needed. A closed spent result cannot close its moved cursor.

Argument tags, UTF-8, fixed widths, lengths, zero-arity rows, allocation arithmetic, schema ownership and result identity are checked before use. Native errors never expose half-initialized handles or a partially completed current answer. Unexpected panics are contained at the native entry boundary and reported as internal failures without unsafely continuing a damaged owner.

Stable error codes and typed outcomes survive async open/query/submit/close. Provider text can be preserved as a redacted cause; retries never parse English messages. A known published receipt survives subsequent local-cache/cleanup failure. OutcomeUnknown is publication uncertainty, not merely an exception labeled retryable.

## IDs and floats stay ordinary checked values

Application Id128 values are 16 bytes natively and 32 lowercase hexadecimal characters in TypeScript. A cryptographic-random value helper may generate one before sealing; it is not an issuer or allocator. Preserve exact input IDs through CAS retry, reopen and generated migrations. No FreshRef, 28-byte decision-derived entity tuple, reservation counter, fresh-map codec or ID-burn transaction remains.

Canonical f64 values and deterministic sum/mean follow chapter 11. All NaNs canonicalize to 0x7ff8000000000000, signed zero to +0, and each arithmetic node canonicalizes its output. JSON uses the explicit schema-aware float-bit codec, not JSON.stringify's treatment of NaN/infinity.

Interval<F64> uses chapter 11's canonical 16-byte pair, dense continuous interval meaning, finite-point membership, infinite bounds and typed duration failures. Do not substitute representable-float counting or allow float widths/capacity weights by accident.

Native execution establishes/restores the required floating environment at relevant foreign-thread entry: round-to-nearest ties-to-even, gradual underflow and no uncontrolled FTZ/DAZ/FMA behavior. Other native code in a Node host cannot silently change database answers. Nested entries and thread-pool execution must preserve the host environment. This remains necessary without a public C product.

## Bounded scheduling, not synchronous native work on the request loop

Every TypeScript data operation is Effect-only: storage/query/log calls, inspect, change ingestion/finalization, command sealing/canonical hash, row codecs, result/cursor operations, maintenance and resource disposal. They run through chapter 31's bounded native runtime with bounded yielding JS conversion where needed, never an async signature that performs the full workload synchronously before resolving. Queue admission, per-owner active counts, network/FS lanes and result conversion are bounded. Pure small schema/query/scalar/intent constructors, inert layer/effect construction and already-owned metadata access remain synchronous ordinary expressions; substantial native validation/compilation belongs to async admission/generation. There is not an independent runtime per tenant. The prepared LMDB writer remains on its owning worker across a remote attempt; no JavaScript callback executes inside that transaction. Ordinary async ingestion is part of this one API, not a deferred bulk protocol.

Cancellation reaches queue admission, transport/body reads, computation, replay, scratch and result transfer. Interrupting a fiber while untracked native work continues is not completed cancellation. Effect.callback cleanup cancels and joins native work or reports incomplete drain; Cause.Interrupt cannot be mapped to a fabricated NotSubmitted. The host can inspect/drain an interrupted operation; unknown publication retains its command reference.

Execution sessions retain accounted optional caches bound to their actual snapshot/physical identity. Schema-level query templates are immutable and shareable; mutable tenant rows/plans are not hidden inside a global GC-only object. Correct small application queries and larger-than-RAM fallback use the same semantics.

## One checked native package contract

Rust's ABI is not public or stable. The Node bridge is an exact-version internal N-API contract used by the packaged TypeScript wrappers. Do not create a second stable ABI or promise that arbitrary internal Rust types are callable downstream.

A small bootstrap descriptor reports ABI major/minor, package/build revision, feature bitmap, engine format, supported codec/protocol versions, architecture, libc/minimum OS, baseline CPU features and N-API level. Check it before opening or mutating data. Missing artifact and present-but-incompatible artifact are distinct failures.

Packed core/log manifests pin compatible native versions exactly. Both require the same exact Effect 4.0.0-rc.112 peer and dev dependency; no optional adapter, duplicate bundled Effect, v3 API or broad RC range. Core/log import one shared NativeRuntime service and use chapter 35's Effect/Scope/Stream contract. Runtime checks still cover manual installs, linked packages, bundlers and accidentally mixed binaries. Pin the Rust toolchain and dependency locks in release provenance; a compiler upgrade cannot silently change schema/value/protocol meaning.

Patch releases do not change equality, float semantics, ID encoding, generated migration-plan meaning or receipt meaning. Storage/protocol/plan versions are independent of npm semver. Supported upgrades require fixtures; incompatible old stores refuse before mutation and use the log importer.

## Canonical qualification targets

| Target | Required policy |
| --- | --- |
| Apple Silicon macOS arm64 | Canonical development/performance target; initially macOS 14 minimum plus current supported macOS. Qualify native close/lock/mmap/FPU behavior and useful small per-user application workloads. |
| AWS Graviton Linux arm64 | Supported portable correctness target with real deployment tests; initial glibc 2.34/AL2023 baseline. No separate Graviton tuning project before measurements justify one. |
| Vercel Node Linux x86-64 | Supported application target inside a measured deployment envelope. Qualify the actual emitted x64 artifact, current runtime/libc/CPU, local-disk/FD/memory limits, warm concurrency and full cold recovery. No special x86 tuning claim yet. |
| Node versions | Node 24 is the common deployment baseline. Node 26 may be separately qualified on hosts that offer it; do not claim Vercel Node 26 when its supported-version list does not. |
| Not claimed | Edge/Worker/browser/mobile runtimes, musl/Alpine, Windows, macOS x64 and 32-bit targets without separate artifacts and complete qualification. |

These are support requirements, not claims of tests passed in this proposal. Publish exact tested versions and binary digests. For managed hosts that roll minor/patch versions, record the runtime observed during qualification and fail clearly on incompatibility.

Use a correct baseline CPU path: x86-64/SSE2 and ARMv8-A subject to the platform ABI. Optional optimized paths require runtime detection and equivalence tests; universally shipped binaries must not be compiled for the release machine's native CPU. Apple Silicon is the initial tuning focus, not a different database semantics.

Vercel Node is not Vercel Edge. Node compatibility does not by itself prove native packaging, mmap behavior, durable local storage or enough /tmp space. HostedHistory uses local files as disposable materialization, with S3 authority. LocalHistory needs genuinely durable owned storage and must not be advertised as durable on an ephemeral function filesystem. See chapter 33 for the concrete envelope and external evidence.

## Build and publish without mutating the checkout

Build from controlled source into an isolated staging tree. Generate packed manifests and exact native pins there; preserve unrelated optional dependencies. No prepack/postpack hook rewrites the developer's package.json or depends on an interrupted post-hook repairing it.

Keep source tests, native builds and release-pack tests distinct. Source tests do not silently clean/build/pack/publish. Release tests intentionally build fresh artifacts and report their identity. A wrapper test against yesterday's native binary is not a clean release qualification.

Enumerate every remaining Rust/Node crate, feature and target explicitly, including bridge crates outside the root workspace. Removing the C product is affirmative removal from that inventory. Ordinary docs/examples never depend on unpublished packages to pass a pre-promotion gate.

Pre-promotion qualification installs the exact staged tarballs in empty projects and a disposable/private registry, checks pins/file allowlists and simulates partial publication. It needs no public registry version that does not exist yet. No publication is authorized by this document.

After separately authorized promotion, download actual public-registry artifacts, verify the same digests and perform clean remote installation. Failure is a release incident and blocks declaring completion, not a retroactive green qualification report. Publish tested immutable files; never rebuild an untested native library during promotion.

## Required boundary and release gates

Existing gate IDs remain stable where their Rust/Node safety obligation survives; C-only harness requirements are replaced by explicit surface deletion, not quietly marked passed.

| Gate | Required assertion |
| --- | --- |
| FFI-01 Lifecycle | Rust/Node create/read/query/close/reopen with retained wrappers and GC disabled; millions of bounded cycles plateau. No native owner, mapping, FD, lock or historical callback tombstone persists. Port shared SDK-007/013 schedules. |
| FFI-02 Capability safety | Wrong owner/kind/generation, double close, slot reuse, capacity/exhaustion, escaped borrow/snapshot and close during lookup refuse without wrong-resource access. Internal bookkeeping remains bounded. |
| FFI-03 Boundary memory | Applicable Rust Miri/sanitizer/fuzz tests and a real Node addon harness cover valid-memory boundary tags/lengths, UTF-8, overflow, zero arity and allocation failure. No native unwind or half-initialized output escapes. No public C harness/product is retained to satisfy this row. |
| FFI-04 Threads | Parallel Node/native read/apply/close/cancel, worker affinity, nested entry and poisoned/faulted owner paths. Race tooling where supported; no freeing active data or deadlock through bookkeeping locks. |
| FFI-05 Owned values | Mutable input after successful ingestion cannot affect work; mutation during yielded copying, buffer detachment/resize and hostile dynamic shapes cannot violate memory safety or local/replay identity. No call-entry snapshot or detection guarantee is invented. Failed/cancelled/reentrant/overlapping builder calls spend construction and drain/release ownership or explicitly report incomplete drain without GC; no partial reuse. Result/page copies survive their documented owner lifetime; the one-shot TS page Stream spends/transfers once and closes on take/failure/interruption. Close/reuse/cancel cannot expose a mapping-backed JS view or free active input. |
| FFI-06 Floats | Golden bits, foreign-thread rounding/FTZ/DAZ, nested guard restoration, scalar/interval edge cases and sum/mean order/spill equivalence on all canonical targets. |
| FFI-07 Async bridge | Queue/active cancellation, JS getter/iterator throw, worker failure, open errors, unresolved submit during close, bounded input/output conversion and draft cleanup under saturation. Declarations and runtime export inventory expose only Effect work, Scope acquisitions and page Streams: no Promise/sync/AsyncDisposable twin. Exact A/E/R and finalizer/interruption behavior are tested against the pinned RC. Measure event-loop delay through ingestion, large-cell rejection, finalization/canonical hash, query and row codecs; Effects around blocking work fail. No erased error kind or untracked forever-running native task. |
| FFI-08 Native mismatch | Wrong version/features/N-API/format/codec/architecture/libc/CPU or foreign duplicate-runtime handles refuse before store mutation; distinguish absent from incompatible artifacts. |
| PKG-01 Reproducible inputs | Fresh locked Rust core/internal log builds and the single full-capability Node artifact per supported platform at the pinned toolchain. Rust core dependency checks show no log/AWS dependency; Node core-only import starts no transport/maintenance work. Provenance identifies source revision/dirty state, toolchain, targets, flags, locks and digests. |
| PKG-02 Clean staging | Interrupt manifest/build/pack at each phase; source stays unchanged, unrelated optional deps survive and staging retry is coherent. |
| PKG-03 Isolated install | Actual tarballs in empty npm/pnpm projects; ESM/types/native load/core/log/generated-plan examples run without monorepo paths or source aliases. Same core ChangeSet/query/QueryReader/CompleteResult passes through both packages without user adapters; exactly one shared native runtime and compatible Effect dependency graph loads. Exercise both packages with the same core service/layer and no generic wrapper. Missing platform dependencies and deliberately duplicated incompatible runtime handles fail clearly. |
| PKG-04 Canonical targets | Actual artifacts execute on Apple Silicon, Graviton and Vercel Node x64. Verify supported Node/OS/libc/CPU, cold/warm paths, binary tracing and baseline fallbacks. Portable correctness required everywhere; unmeasured tuning claims forbidden. |
| PKG-05 Data compatibility | Golden stores/history/commands/generated plans reopen/replay on supported upgrades. Incompatible old/downgrade cases refuse before writes, with explicit log import requirements. |
| PKG-06 Whole C deletion/public contract | C crate, headers, symbols, examples, tests, workflows, release targets, dependencies/imports and public documentation are removed from the implementation/release tree; preserved audit/history is exempt. Rust/TS declarations/examples match remaining exports. No public Rust log SDK; no core migration/log dependency. Shared Rust/Node regression coverage remains. |
| PKG-07A Pre-promotion proof | Exact staged digests, empty-project/private-registry installs, pins, package allowlists and simulated partial publication pass before public promotion. Missing required qualification is incomplete, not skipped-green. |
| PKG-07B Distribution proof | After authorized promotion, actual registry-download digests/pins/clean installs match qualification. Failure blocks release completion and triggers an incident. |

## Audit disposition

C-specific pointer/tombstone compatibility is removed rather than preserved. Engine/Node lifetime, input-ownership, async error, native reclamation, float and packaging counterexamples remain obligations. Deleting an interface does not erase a shared bug; a new version, a compile pass or this proposal is not closure evidence.
