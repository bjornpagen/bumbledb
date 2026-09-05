# 35 — Effect is the TypeScript API

Status: **binding proposed API, not implemented or release-qualified**. This chapter fixes the TypeScript signatures and Effect semantics before implementation resumes. Chapters 30–34 supply domain/runtime detail; this chapter owns their Effect interpretation. Rust keeps ordinary native `Result`, ownership and RAII. No new storage or publication mechanism is introduced.

## One cut, not an adapter

Both TypeScript packages require **Effect `4.0.0-rc.112`**, matching the inspected consumer. Declare it as a required exact `peerDependency` and the same exact `devDependency`; it is not optional, bundled privately, or a second dependency version hidden inside log. Pin core/log/native package versions together. An RC upgrade is an explicit lockfile/API/qualification change, not a wildcard range. The release may select a newer qualified Effect version before shipping; this proposal does not claim RC stability.

Delete the public Promise, synchronous database and disposal surfaces. No `/effect` adapter package, `/sync`, `/promise`, `AsyncDisposable`, thenable handle, or per-call `runPromise` escape. A Promise at a Next.js/CLI **application boundary** is normal; it is not a database API. Effects remain lazy and execute only when the application runs them.

Use the version-matched Effect 4 idioms: `Effect.fn("operation")`, `Effect.gen`, `Context.Service`, `Layer.effect`, `Effect.acquireRelease`, `Effect.scoped`, `Schema.TaggedError`, `Option`, `Result`, and `Stream`. `Layer.effect` owns scoped acquisition in this version; do not copy Effect 3's `Layer.scoped`, `Context.Tag`, `Effect.async`, `Effect.catchAll` or `Either` examples. Use `Effect.callback`, `Effect.catch`, and `Result` where those functions are needed. This is a library, not a custom Effect runtime.

## Pure descriptions; effects at execution boundaries

| Ordinary synchronous values | Effectful work |
| --- | --- |
| `relation`, `schema`, field/law builders, `query(...).rule(...)`, named relation expressions and typed parameters | Native schema admission/compilation/fingerprint, row ingestion/encoding/decoding, query bind/prepare/execute |
| Small deterministic scalar constructors, fixed-size ID parsing, `RequestId.from(id)`, interval constructors | Random ID generation, open/create, snapshot/get/apply, command sealing/submit/resolve |
| Migration intent AST and generated static imports | Migration generation/check/write, source freeze/plan execution/activation/status, backup/restore/retention |
| Already-owned identity, schema ID, witness, stamp, receipt, command reference, result metadata | Inspect, result collection/page transfer, cache acquisition/eviction, native resource close |

Pure AST construction has no native work, schema hashing, hidden I/O or row ingestion. Native compilation is charged and effectful; a schema declaration is not a claim that its theory has already been admitted. Give authoring metadata explicit finite structure limits at admission. User callbacks used to construct AST are pure authoring callbacks, **not transaction callbacks**.

`Id128.fromHex` and checked small interval constructors return `Result.Result<Value, DbError>` on invalid input; programmer-facing AST misuse may throw synchronously but performs no I/O. `Id128.random()` returns `Effect.Effect<Id128, DbError>` using cryptographic entropy, not Effect's noncryptographic test/random service. No random effect runs at module import. Generate IDs once for an original intent, persist them as needed, and never include generation inside a database retry.

Use Effect Schema for small external boundary models and structured diagnostics. Derive row/parameter boundary schemas from the core relation descriptors; do not maintain handwritten parallel field rosters. Effect Schema does **not** replace the canonical Rust value parser, binary formats, schema laws or deterministic float algebra. Its generic JSON number encoding is not Bumbledb's `$f64` codec. Arbitrary Effect Schema transforms/refinements are app code, not persisted laws or migration operators. Bulk core row codecs execute in charged chunks; `Schema.decodeUnknownEffect` alone does not make an arbitrarily large synchronous decoder nonblocking.

## One native service; explicit database capabilities

The core exports `NativeRuntime`, a `Context.Service`, with `NativeRuntime.layer(options)`. Its layer acquires the single bounded native runtime and releases it with scope. Options contain the actual worker/queue/aggregate-resource limits and finite cleanup policy, not S3/binding/schema settings. No imported module starts workers, reads credentials or opens a database. Reuse one layer value in the app graph so Effect's layer memoization shares it. A second independently built runtime configuration in the same addon is refused while the first is live; shutdown/incomplete drain cannot mint a successor runtime. Foreign duplicate-addon handles always refuse.

Handle creation captures the runtime capability. Methods on an acquired handle require no repeated runtime lookup; acquiring a child adds only `Scope.Scope`. Database-free native work such as schema compilation or initial draft acquisition requires `NativeRuntime`. This keeps reusable helpers small without an invisible global default runtime. One native registry owns operation leases and shutdown accounting; Effect owns fiber/scope composition. These are different safety responsibilities, not duplicate task authorities.

No `Context.Service` class is generated for every tenant, relation, snapshot or query. An app may define a repository service around its domain operations using `Layer.effect`; it need not define an adapter for Bumbledb. The optional log `TenantCache.make(schema, options)` acquires a scoped `TenantCache<S>` value; `cache.acquire(binding, options)` returns a distinct scoped `HistoryBorrow<S>`. An app service may own that cache in its layer. This preserves the concrete schema type without a generic global service tag that erases it. Use a core `QueryReader<S>` parameter for cross-core/log read helpers. Do not inject an ambient writable tenant into every query expression.

Effect `LayerMap`/`RcMap` was considered. Its keyed scoped lifecycle/idle TTL does not by itself provide the required aggregate byte/disk reservations, independent revocable borrows or native close-incomplete accounting. Keep chapter 31's one bounded **native** registry; no second JS tenant cache, default TTL authority, per-tenant executor or new generic pool is added. Effect services/layers expose the existing ownership, not a replacement cache protocol.

## Core signature roster

This is a specification of the exported shape, not a complete `.d.ts` file. `S` is a declared core schema; `Rel<S>`, `Fact<R>`, `Key<R>`, and query `Params`/`Row` are derived from the existing typed descriptors. Relations/keys/query templates must belong to `S`. No `any`/`unknown` parameter escape is part of the typed API; explicitly unknown wire input uses the checked codecs. All fields of returned metadata and outcomes are readonly.

```ts
import type { Effect, Layer, Option, Scope, Stream } from "effect"

// Static acquisitions / database-free work
NativeRuntime.layer(options): Layer.Layer<NativeRuntime, DbError>
Schema.compile<S>(schema: S, work: ExecutionPolicy):
  Effect.Effect<CompiledSchema<S>, DbError, NativeRuntime>
Db.create<S>(path: string, schema: S, work: ExecutionPolicy):
  Effect.Effect<Db<S>, DbError, NativeRuntime | Scope.Scope>
Db.open<S>(path: string, schema: S, work: ExecutionPolicy):
  Effect.Effect<Db<S>, DbError, NativeRuntime | Scope.Scope>
ChangeSet.builder<S>(schema: S, work: ExecutionPolicy):
  Effect.Effect<ChangeDraft<S>, DbError, NativeRuntime | Scope.Scope>

interface Db<S> {
  readonly schemaId: SchemaId
  snapshot(work: ExecutionPolicy): Effect.Effect<Snapshot<S>, DbError, Scope.Scope>
  apply(changes: ChangeSet<S>, options: ApplyOptions): Effect.Effect<ApplyOutcome, DbError>
  inspect(work: ExecutionPolicy): Effect.Effect<DbInspection, DbError>
  close(): Effect.Effect<CloseReport>
}

interface QueryReader<S> {
  get<R extends Rel<S>>(relation: R, key: Key<R>, work: ExecutionPolicy):
    Effect.Effect<Option.Option<Fact<R>>, DbError>
  execute<P, A>(query: QueryTemplate<S, P, A>, params: P, work: ExecutionPolicy):
    Effect.Effect<CompleteResult<A>, DbError, Scope.Scope>
}

interface Snapshot<S> extends QueryReader<S> {
  readonly witness: CoreWitness
  session(work: ExecutionPolicy): Effect.Effect<ExecutionSession<S>, DbError, Scope.Scope>
  close(): Effect.Effect<CloseReport>
}

interface ExecutionSession<S> {
  execute<P, A>(query: QueryTemplate<S, P, A>, params: P, work: ExecutionPolicy):
    Effect.Effect<CompleteResult<A>, DbError, Scope.Scope>
  close(): Effect.Effect<CloseReport>
}

interface ChangeDraft<S> {
  insert<R extends Rel<S>>(relation: R, rows: Iterable<Fact<R>>): Effect.Effect<void, DbError>
  delete<R extends Rel<S>>(relation: R, rows: Iterable<Fact<R>>): Effect.Effect<void, DbError>
  finish(): Effect.Effect<ChangeSet<S>, DbError, Scope.Scope>
  close(): Effect.Effect<CloseReport>
}

interface ChangeSet<S> {
  readonly schemaId: SchemaId
  close(): Effect.Effect<CloseReport>
}

interface CompleteResult<A> {
  collect(options: { readonly maxBytes: bigint }): Effect.Effect<ReadonlyArray<A>, DbError>
  pages(options: { readonly pageBytes: bigint }): Stream.Stream<ReadonlyArray<A>, DbError>
  close(): Effect.Effect<CloseReport>
}
```

`Layer` in the roster is Effect's `Layer` type; `Schema.compile` is the **core** namespace, imported as `BumbleSchema` when Effect Schema is also in scope. `CompiledSchema<S>` is bounded detached immutable descriptor data plus canonical `schemaId`, not a tenant handle or separate user-authored schema. It needs no native finalizer. Open/create/build compile via the same implementation; prior compilation is optional, not a mandatory prepare ceremony. No log `descriptorOf` or log fingerprint helper remains. A query template contains schema-bound logical AST; execution validates/lowers it natively under budget.

`ApplyOptions` extends core work policy with `expected: { kind: "any" } | { kind: "exact"; at: CoreWitness }`. `ApplyOutcome` remains `accepted | no-change | invariant-rejected | moved` in **A**. Operational failure is `DbError` in **E**; no generic `Result` around every effect. Missing key is `Option.none`, not a fake I/O error or nullable row. Effects returning resources expose `Scope.Scope`; TypeScript cannot prove linear lifetimes, so runtime spent/identity checks remain necessary.

`ExecutionPolicy` uses exact nonnegative `bigint` byte/row/work counters and a bounded monotonic duration `timeout: Duration.Input` (Effect's type) for the operation, converted once at admission into a native deadline. Worker/queue counts are checked positive safe integers in runtime options. Zero work limits mean no work allowed, not unlimited. Policy has no public `AbortSignal`, parallel cancellation token or callback. Fiber interruption supplies cancellation; native work budgets are still enforced even when callers mask interruption. Ingest calls share the draft's aggregate input/working/spill budget; they do not each reset it. Teardown has the runtime's separate reserved cleanup envelope so exhausting work cannot prevent release. Values are measured configuration, not new database-size constants.

`encodeRows(rowShape, rows, work)` and `decodeRows(rowShape, input, work)` return `Effect<owned data, DbError, NativeRuntime>`. Typed row shape comes from a relation or query output. They share the core canonical codec; untrusted input cannot inject native capabilities. Binding parameters is ingestion, not unrestricted synchronous copying hidden in an AST helper. Core encoding used by log and migrations stays the same implementation.

### Streams replace the TypeScript cursor facade

`pages` describes a **one-shot consuming stream over a completed result**, not a live query stream. Its first execution atomically spends the result and moves its backing storage into a private cursor owned by the stream's scope. Construction alone does not spend it. Successful `collect` leaves the result available; failed collection due to an output cap leaves its sealed backing available for pages. Cursor/storage corruption faults that backing. Concurrent collect/transfer or two transfers refuse before touching it; no implicit parallel cursor.

Use `Stream.unwrap` with scoped cursor acquisition and `Stream.paginate`/a pull channel internally. In this RC, paginate emits the elements of its returned array: return a singleton array containing the owned page to emit **pages**, not accidentally individual rows. No public `intoCursor`, `next`, `AsyncIterable`, clone or second streaming API in TypeScript. Rust retains its consuming `into_cursor` API. `Stream.take`, upstream failure, downstream failure and interruption close/drain the private cursor; its terminal EOF cleanup is identical. A stream constructed but never run leaves the original result's scope responsible. A stream run after that scope closes fails `ClosedHandle`. A second run after transfer fails `SpentHandle`, not silently returning EOF or rerunning the query.

Each stream element is an owned **page array** capped in bytes, not an Effect per tuple. A row exceeding pageBytes refuses explicitly; empty result emits no pages. Output is an unordered set; page boundaries/order are not a stable seek token. Full execution/admission and exact aggregates finish before the first page. Stream backpressure bounds delivery, not query work already done. `Stream.runCollect` can still collect every page into app memory; use `collect` for a database-enforced total materialization cap. App effects over rows remain application work; a JS `Stream.runFold` is not a deterministic Bumbledb float aggregate.

## Log signature roster: the same core, with durable identity

```ts
LocalHistory.open<S>(binding: LocalBinding, schema: S, options: LocalOpenOptions):
  Effect.Effect<History<S>, LogError, NativeRuntime | Scope.Scope>
HostedHistory.open<S>(binding: HostedBinding, schema: S, options: HostedOpenOptions):
  Effect.Effect<History<S>, LogError, NativeRuntime | Scope.Scope>
Command.seal<S>(input: CommandInput<S>, work: ExecutionPolicy):
  Effect.Effect<Command<S>, LogError, Scope.Scope>

interface History<S> {
  readonly identity: DatabaseIdentity
  readonly receiptEpoch: ReceiptEpoch
  snapshot(options: ReadOptions): Effect.Effect<PublishedSnapshot<S>, LogError, Scope.Scope>
  submit(command: Command<S>, options: SubmitOptions): Effect.Effect<SubmitOutcome>
  resolve(ref: CommandRef, work: ExecutionPolicy): Effect.Effect<ResolveOutcome, LogError>
  inspect(work: ExecutionPolicy): Effect.Effect<HistoryInspection, LogError>
  close(): Effect.Effect<CloseReport>
}

interface PublishedSnapshot<S> extends QueryReader<S> {
  readonly identity: DatabaseIdentity
  readonly decisionStamp: DecisionStamp
  readonly stateStamp: StateStamp
  readonly freshness: Freshness
  session(work: ExecutionPolicy): Effect.Effect<ExecutionSession<S>, DbError, Scope.Scope>
  close(): Effect.Effect<CloseReport>
}

interface Command<S> {
  readonly ref: CommandRef
  close(): Effect.Effect<CloseReport>
}
```

`CommandInput<S>` is exactly `{ scope, id, changes, precondition, result }` from chapter 30; it retains the change's captured runtime, never loads a second one. A `HistoryBorrow<S>` exposes the same read/submit/resolve/inspect and metadata members as History, but `release(): Effect<CloseReport>` instead of owner `close`. Its scope releases only that borrow; it cannot close the registry's shared owner. `TenantCache.make` returns `Effect<TenantCache<S>, LogError, NativeRuntime | Scope>`; the cache exposes `acquire(binding, options): Effect<HistoryBorrow<S>, LogError, Scope>`, `inspect(work): Effect<CacheInspection, LogError>`, `evict(binding): Effect<CloseReport, LogError>` and `close(): Effect<CloseReport>`. Eviction of a borrowed/active slot refuses rather than revoking another request. Bindings are discriminated local/hosted data; neither constructor accepts the other backend's fields. Different tenant schemas use separately constructed typed caches, not casts through one untyped schema cache.

Creation is explicit: `LocalHistory.create`/`HostedHistory.create` have the open signature plus required stable creation identity and a checked initialization artifact. They never fabricate applied migration history. Ordinary apps get their initial binding through the chapter 33 generated-plan `initialize` operation. Existing history open never initializes on missing/error.

`ReadOptions` extends `ExecutionPolicy` only with chapter 30's consistency sum. `SubmitOptions` adds a finite native publication-attempt limit and native backoff bounds; it accepts no Effect `Schedule` or user retry callback. The native protocol already owns catch-up/CAS retries. Retries consume one total operation budget, not a fresh deadline per attempt. No whole-workflow automatic retry or row callback is introduced.

`submit` is `Effect<SubmitOutcome, never>` because ordinary failure is represented **once** in its certainty union: decided(receipt, localHealth), not-submitted(ref, error), outcome-unknown(ref, error). Invalid/closed/foreign handles supplied dynamically refuse before dispatch using not-submitted when an authentic ref is available; a forged capability with no authentic ref is programmer misuse/a defect, never a forged receipt. Other malformed public input is typed at its parser/seal boundary. `never` in E does **not** mean an uninterruptible or defect-free fiber. A rejected/precondition-failed receipt is a durable value, not an Effect failure to retry. `resolve` returns found/not-recorded-at/command-epoch-closed/receipt-expired-unknown in A and operational `LogError` in E. Frozen/expired/read-back semantics remain chapter 30's exactly.

### Laziness, reruns and stable intent

Every call constructs a lazy effect. Acquisitions run again if the effect is run again; queries rerun against their still-live snapshot; core apply can perform another local transaction. There is no hidden memoization of database calls. Draft mutation **execution** is one-shot: each returned insert/delete/finish effect has an execution latch; a second/concurrent execution fails `SpentOperation` and drains that draft if still building. Distinct sequential calls may ingest more rows. Finish spends the draft. Sealed changes/commands are immutable and reusable while open: retrying submit is allowed and refers to the identical command, not a rebuilt intent. Do not put a whole build/open/generate workflow under `Effect.retry`.

Input stability is required from the start of an ingestion effect's execution through its Exit; constructing that effect does not freeze input. After successful ingestion, later mutation cannot affect the owned draft. Host copying yields between bounded chunks; limits include each cell/row. SharedArrayBuffer views are refused. Getter/iterator errors become typed input failures, never untracked partial drafts; arbitrary user getters cannot be preempted. Failure/interruption spends the draft and initiates tracked drain. The exact finalized native bytes are used for both local application and log replay. No second JS row walk occurs during sealing/submission.

Before dispatch, copy `command.ref` and retain it with the original intent using the application's existing request/job persistence. A reference resolves uncertainty but cannot reconstruct a missing payload. Cross-process **resubmission** requires the original sealed meaning: `Command.encode(command, work): Effect<Uint8Array, LogError>` and `Command.decode(bytes, schema, work): Effect<Command<S>, LogError, NativeRuntime | Scope>` use the same bounded versioned native command codec, not a new journal. Decode checks the supplied core schema against the encoded identity before producing typed facts. Encoding yields capped owned bytes; a payload over the configured cap refuses. Caller input remains stable during decode. Persisting/exporting the command is an app choice; retention still obeys receipt epochs. Ref/schema/stamp parsers are bounded core/log boundary codecs, not casts from HTTP strings.

### Interruption is not a rollback or a returned decision

Use `Effect.callback` at the native operation boundary. Registration reserves and records ownership before dispatch; completion resumes at most once. Its interruption cleanup signals native cancellation **and joins or explicitly reports incomplete drain**. Dropping the JS listener, racing a Promise against a timeout or aborting only the HTTP fetch is not native cancellation. Long native calls, I/O bodies and scratch loops observe the same native operation context.

Mask only bounded registration, ownership transfer and finalizer-installation handshakes with `Effect.uninterruptibleMask`. Long acquisition, copy, catch-up, query and S3 operations stay interruptible. Effect 4 `acquireRelease` defaults to uninterruptible acquisition; use its interruptible option with a cancellation-safe acquisition bridge, or explicit masking/restoration plus guaranteed cleanup of late success. Do not mask the entire open or submit. The bridge itself must reclaim a resource if completion loses the race with interruption before the scope receives it.

| Event | What the caller can observe | What remains true |
| --- | --- | --- |
| Native work timeout while fiber is live, before publication dispatch | not-submitted | This invocation did not dispatch; earlier invocations may have |
| Lost publication response / native timeout after dispatch | outcome-unknown, or decided if proven | The original ref and exact intent remain the recovery coordinate |
| Effect timeout or fiber interruption | Interruption/timeout in the fiber's Cause; **no guarantee that A is delivered** | Cancel/drain locally; remote publication may still have happened |
| Known receipt, then local cache or cleanup failure | Receipt retained; local-health/close problem separate | No change from decided to rejected or unknown |

The library cannot force a successful `SubmitOutcome` into an interrupted fiber. It must retain observed evidence in the tracked operation until reconciled/drained; the retained ref resolves after reopen even if the HTTP response or enclosing scope fails. It must not claim the caller received a receipt merely because native code observed it. Applications requiring response mapping inspect `Exit` at their outer boundary and resolve the already retained ref. No catch-all maps interruption/defects to `not-submitted`. A scope finalizer does not retire an epoch, undo a write, thaw a migration or activate a target.

## Close reports and finalizers: do not swallow the hard case

All resource owners are scoped, even when they also expose early `close()`. Early close starts/joins the same stored close transition using the runtime's configured finite cleanup envelope. Repeated close is idempotent. A borrow's release uses the same rules but never closes another borrow. There is no public disposal protocol or promise method.

```ts
type CloseReport =
  | { readonly kind: "closed" }
  | { readonly kind: "incomplete"; readonly outstanding: OutstandingWork }
  | { readonly kind: "failed"; readonly error: DbError }
```

Core owns this vocabulary; log transport/shutdown detail is bounded diagnostic data, not an imported `LogError` in the core. `closed` means this capability's obligations are released; closing a command/change drops its reference, not another operation's retained reference. Incomplete/failed never counts as reclaimed resources. The native registry retains the Closing owner and lock/accounting until actual completion; a successor cannot reuse its directory. Diagnostics remain available through runtime/cache inspect or another explicit close on the inert closing handle. Every branch has bounded evidence, not an unbounded list of retained payloads.

Effect finalizers have E = never. Their policy is fixed: run the native close; on incomplete/failed, surface a structured `CloseFailure` **defect in the finalizer Cause**, and preserve native Closing accounting. Do not use catch-and-log, `orDie` on an erased unknown cause, or report success while work runs detached. An explicit caller can inspect `close()`'s report, but the scope still reports unresolved cleanup instead of silently ignoring it. Teardown attempts use reserved capacity. A stuck OS call cannot be guaranteed to stop on a deadline; the report is not a successful cancellation. Hard termination requires the documented host process boundary and recovery, not a simulated library timeout.

An enclosing `Effect.scoped` can therefore fail after a known receipt. The app must not equate its final Exit with the database's terminal decision. Copy/persist the ref before submit; retain an observed receipt in the existing app response/job state when required; report cleanup failure separately and use resolve on uncertainty. Tests must cover this exact scenario, not just success-path finalization.

## Error shape and diagnostics

Use core `DbError` and log-only `ProtocolError` as `Schema.TaggedError` classes. Each owns a **tagged reason** generated from its stable native code roster, with operation and bounded structured detail. The reason's `_tag` is the specific code; family/retry guidance is derived from that roster, not independently stored booleans. `Effect.catchTag("DbError", ...)` handles core failures; `Effect.catchReason("DbError", "ResourceLimit", ...)` is the documented Effect 4 pattern for a selected reason (ResourceLimit is the selected resource-budget reason; finer resource details live in its bounded payload). Log preserves core errors unchanged: `LogError = DbError | ProtocolError`, not `BumbledbFailure { cause: unknown }`. Any displayed `.code` accessor is derived from the reason tag, never another mutable authority.

Catching one reason does not generally eliminate the parent DbError type in this RC; preserve the inferred E or use the documented unwrapReason/complete handling when intentional. Operational errors are in E; semantic admission/receipt outcomes remain tagged data in A. Use `Result` only for genuinely pure fallible parsing; `Option` for absent key lookup. Use `Cause`/`Exit` for defects, interruption and finalizer problems, not an extra return envelope on every method. Expected native panics are not business outcomes; contain them, fault the owner and surface Internal according to the checked bridge, without continuing damaged state.

Use normal Effect tracing/log context around operation/batch boundaries. `Effect.fn` names remain stable and spans carry redacted operation/cost metadata, not rows, query parameters, credentials or tenant IDs by default. No per-cell Effect allocation/span, custom logger, global telemetry service or mandatory OTLP dependency. Native operation metrics feed the existing bounded inspect result; trace context is explicit bounded metadata across the bridge, not a native dependency on the JS fiber object.

## Migration/admin APIs are Effects too

`@bjornpagen/bumbledb-log/schema` owns only pure intent constructors. `@bjornpagen/bumbledb-log/migrations` owns `generateMigrations`, `checkMigrations`, `migrationStatus`, `initialize`, `migrate`, `activateMigration`, `abortMigration`. Generation/check take schema/intent/repository options and return `Effect<GenerationReport, LogError, NativeRuntime>`; generation writes reviewed repo-local plan data, check writes nothing. Initialize/migrate take generated plans plus stable operation identity and binding/resource options, never callbacks. All durable operations retain a small immutable operation reference **supplied/derived before dispatch**, not only returned on success.

Status is `Effect<MigrationStatus, LogError, NativeRuntime>`. Mutating admin operations return `Effect<AdminOutcome<Value>, never, NativeRuntime>`, where the tagged arms are `completed { ref, value }`, `not-started { ref, error }`, and `outcome-unknown { ref, error }`. The ref is the operation's existing protocol identity (for migration: source identity + stable operation ID), not a new journal or new receipt system. Typed options carry that ref before execution. Malformed untyped refs fail their small boundary parser before this API; forged opaque capabilities are misuse defects, as with submit.

Not-started proves this invocation performed no authoritative mutation; it does not erase prior invocations. All preflight input/manifest/drift/resource failures use this arm. Completed means the **reported transition/status is known**, not necessarily successful migration: migrate's value is `up-to-date { binding }`, `ready-to-switch { deploymentBinding, activationRef }`, or `paused { error, sourceState }`. A failure after a proven source freeze is completed(paused) and leaves it frozen. If any freeze/genesis/cancellation/activation publication remains uncertain, use outcome-unknown until status resolves it; never return paused while concealing unresolved authority. Activation returns its verified activation/current-access report; abort returns its verified cancellation/source-access report. Initialize returns the verified initial binding. Chapter 22 remains authoritative about transition order, replay and race results. Fiber interruption remains Cause, as with submit; status uses the original operation reference.

The same Effect ownership/certainty rule covers existing checkpoint, named restore-point pin/release, receipt-epoch closure/retirement, GC, backup, verifyBackup, restore and erase operations. Read-only inspect/verification has typed E. Maintenance references use the already specified database/epoch/root/barrier identity; a deterministic bounded GC/checkpoint pass does not gain a new per-invocation persistent operation ID or journal. Restore/migration retain their specified operation identities. Streams/files are scoped and native work is bounded. These are language wrappers over each operation's existing report, not a new shared durable state machine or an assertion that all maintenance has command receipts. Retained-root/history/admin records remain their existing protocol authority. No new core backup/migration export is added.

## Effect-native and V8-aware, at the same time

The hot unit is **one native operation or page**, not one Effect per tuple, field, constraint or join binding. Effect describes ownership and sequencing; Rust performs normalization, admission, Free Join, float aggregation and storage. Moving row loops from Rust into elegant-looking Effect combinators is a regression in architecture, not an idiomatic improvement.

- Use stable named operations with `Effect.fn`; capture services once at acquisition and keep handles monomorphic. Do not create Context services, layers, tagged error instances, fiber runtimes or spans in row loops. Defaults for tracing carry no row data; use the consumer's configured tracer rather than installing one. Measure traced/untraced overhead separately.
- Compile schema-specific conversion metadata once per admitted schema/runtime identity. Emit ordinary records in declared field order with the same fields/shapes across pages. Avoid Proxy row wrappers, dynamic property deletion, per-cell closures, boxed scalar classes and generated runtime JavaScript/eval. This is a bounded converter descriptor, not a second query compiler. No claim that every schema can share one V8 hidden class.
- Keep idiomatic primitives: number for f64, bigint for exact integers, canonical hex for Id128 and owned Uint8Array for bytes. Do not mix number/bigint in one declared field or replace exact integers with unsafe doubles for a benchmark. Float canonicalization remains native; string IDs and bigint/boxed numeric allocations have measurable cost. They are not “zero-copy.”
- Copy only at ownership boundaries. After successful checked ingestion, Command.seal retains native changes and never reconstructs JS rows. Never reencode the same accepted command on each CAS retry. Input chunks are bounded host-to-native messages, not one N-API call per field. Worker callbacks deliver bounded chunks; they never dereference moving JS objects. No borrowed LMDB page enters V8.
- Result pages are owned ordinary arrays/records; TypeScript readonly is an API type, not a deep-freeze pass over every returned fact. Caller mutation cannot affect native state or another delivered page. Immutable small metadata/AST may be frozen where appropriate, but not unbounded row graphs at effect construction. Reusing arrays between pages would violate ownership and is forbidden.
- Charge host extraction, strings, buffers, output records and queue-retained payloads, not only Rust buffers. Reject oversize cells using safe cheap length bounds before costly UTF-8 conversion/copy; exact encoding is subsequently charged. Raw JS AST/row input is not recursively cloned by Effect construction. Temporary per-schema converter caches are bounded and released with their actual owners.
- Yield copying/conversion after charged chunks through a scheduler path that allows **Node event-loop turns**, timers and sockets to progress. A chain of already-resolved Promises or microtasks is not evidence of fairness; neither is sprinkling Effect.yieldNow without measurement of its scheduler behavior. Native completion callbacks do not synchronously decode an entire million-row answer. Individual hostile user getters remain outside library preemption guarantees.
- Keep one private cursor behind a page Stream with pull-based backpressure and no prefetch queue by default. No automatic `Stream.mapEffect` for each row and no `concurrency: "unbounded"`. If a measured application needs parallel page processing, its explicit bound is charged against app/runtime headroom. Stream combinators do not bypass the engine's work cap.
- Native protocol owns publication retry/backoff; Effect Schedule is available to application policy but is not nested into each native retry loop. Effect request batching/caching (`RequestResolver`) is **not** enabled automatically: snapshot identity, set grain and error/rounding boundaries cannot be inferred from equal-looking query objects. No query-result cache is introduced by adopting Effect.
- Use Effect Schema's compiled/reused boundary decoders for small external data. Bulk rows are admitted once by the checked native boundary with bounded host shape extraction, not parsed through a fresh full-array schema and then normalized again by log. This does not permit unchecked wire bytes: transport, native and application trust boundaries retain their own distinct necessary checks.

Qualification compares a native/direct-bridge baseline, Effect-wrapped native operations and the full app path on identical semantics. Record bytes copied, allocations per operation/page, live heap/GC pause and retained external memory, event-loop p50/p95/p99/max delay, queue wait, native time, conversion time and end-to-end latency. Exercise cold/warm/post-write/small-result/large-ingestion/page-stream/concurrent-tenant paths. Observe JIT warmup and shape polymorphism; do not assert a V8 optimization from coding style alone. Inspect opt/deopt/GC diagnostics when a measured regression warrants it; do not make unstable V8 flags part of the shipped API. Apple Silicon is the first measured tuning target; qualified Node 24/26 versions and portable Graviton/x86 paths retain the same semantics.

No numeric slowdown budget is fabricated here. M0 records the matched baseline and M7 reports deltas with the chapter 40 method; material regressions require an explicit decision. Effect overhead is real and acceptable only with evidence at the intended application granularity. It does not justify keeping a second non-Effect production surface as a permanent benchmark escape hatch.

## Consumer cutover, grounded in Edullm

Read `packages/bronze-accessor/src/accessor.ts`: immutable typed query inputs plus one Effect-returning service are the right aesthetic. Do not copy its unbounded transport concurrency, whole-body buffering or synchronous hashing as database performance policy.

Read the actual `packages/learner-frontier/src/store/{theory,open,commands,port,receipt}.ts` and generic `packages/conception-engine/src/storage/bumbledb/runtime.ts`. Required downstream direction:

- Delete generic `tryPromise`/`try` wrappers, symbol-wrapped duplicate database owners and manual AsyncDisposable adapters. Acquired core/log handles already carry typed effects and Scope.
- Replace log `descriptorOf(LearnerSchema)` with effectful core `Schema.compile` at initialization/build admission; expose its detached schema ID to pure app fingerprint construction. No database work at schema import.
- Replace `replica.db.read` full scans with the core QueryReader/query AST. Keep `judgeLearnerStoreRows` as application-specific semantic judgment; it is not a second engine validator to port into core.
- Replace reserved u64 event IDs with retained app Id128 data in the successor schema and generated migration. A request/response digest remains a business identity, not automatically a truncated random request ID.
- Replace `writer.commit(callback)` and whole-request `Effect.uninterruptible` with scoped core change construction, sealing, a retained command ref and typed submit/resolve. Receipt lookup handles delivery retries within retained epochs; the response-occurrence key/request-digest rule still prevents conflicting business submissions, including across different command IDs or expired receipts. Do not delete those business laws.
- Keep residence/auth/publication binding, admin provisioning and domain response mapping app-owned. Core not-found/missing authority must still become the app's “not provisioned” refusal, never implicit genesis. No edits to Edullm are part of this proposal pass.

## Qualification additions under the existing gates

No new gate framework. Extend API-01/04/07/10/12, RUN-01/02/04/10, FFI-05/07/08, PKG-03, TS-MIG-10 and APP-03/08:

1. Exact RC declarations/inference compile in packed consumers: Effect A/E/R, Scope requirements, Option lookup, core QueryReader reused by both histories, one native layer, no Promise/sync/disposal/twin exports. Pure schema/query modules perform no native work.
2. Cold laziness; mutation between effect construction and execution; successful acceptance then mutation; repeated and concurrent mutation-effect execution; one-shot result-stream consumption; escaped scopes and foreign runtime handles. No unchecked typed-cast escape.
3. Interruption at registration/completion/finalizer-installation/ownership transfer; abandoned acquisition; queued and running native operations. Retained wrappers with GC disabled release real resources or report Closing, never false quiescence.
4. Unknown publication before/after Effect timeout; known receipt followed by finalizer defect; retained ref resolves after reopen; submission's E=never does not hide Cause.Interrupt. A negative test catches accidental whole-open/whole-submit masking.
5. Completed-result pages with backpressure, early take, downstream failure, EOF, oversize row, second run and close race. No query prefix exposed before completion; scratch reclaimed without GC, copied pages independent.
6. One scoped app runtime/cache across concurrent Next requests; request AbortSignal only at ManagedRuntime's outer run boundary; HMR replacement drains or refuses. No layer-per-query worker growth, duplicate cache or unbounded stream buffering.
7. Event-loop delay and allocation/throughput for Effect boundary overhead on small Apple Silicon queries, large ingestion/conversion and noisy-neighbor cancellation. Measure at batch/page granularity; “Effect is async” alone proves nothing about blocking.
8. Migration freeze/genesis/activation/abort interruption via actual effects; operation identity retained outside retry; no implicit thaw or plan regeneration. Generated-schema and explanation-store-shaped examples run without generic wrappers.

Use version-compatible `@effect/vitest` and `effect/testing` TestClock for TS fiber/layer/schedule tests when implementing the cutover, within the existing single test command. Native real-time cancellation, OS locks, S3 ambiguity and performance require real harnesses/clocks as well; advancing TestClock cannot advance a Rust OS clock or prove remote fencing. Preserve full Rust parallel test coverage and all 220 existing release families. No package/install/source/test implementation changes occur in this documentation pass.

## Version-matched documentation inspected

The installed `../edullm/node_modules/effect/package.json` identifies `4.0.0-rc.112`; its shipped `AGENTS.md` explicitly recommends its own source/AI docs over unrelated documentation. Read that guide and the shipped examples for service/layer composition, references, acquired resources, LayerMap, ManagedRuntime integration, tagged/reason errors, Schema, streams, NodeRuntime, and Effect tests. Inspected `Effect.ts` acquireRelease/callback/scoped contracts and `LayerMap.ts`/`Stream.ts` public types. These are Effect 4 documentation/source facts, not Bumbledb qualification.

Repository-relative evidence paths are external sibling inputs, not required files to ship: `../edullm/node_modules/effect/ai-docs/src/01_effect/`, `03_stream/`, `04_integration/10_managed-runtime.ts`, `09_testing/`; Effect upstream source is [Effect-TS/effect](https://github.com/Effect-TS/effect). Do not silently substitute latest website/Effect 3 examples for the pinned RC. The implementation must typecheck exact examples against its locked dependency before claiming the API exists.
