# One database, two TypeScript packages, one application story

Status: selected public usage contracts for the replacement swarm, 2026-09-05. C1/C7/C8 and L12–L18 settle the native/scalar/generation corrections. Specimens remain acceptance targets, not claims of compilation or execution. No sibling application was modified.

## Decision

Keep the barbell: a small, fast Rust machine and a high-level Effect 4 TypeScript surface. Do not build an application framework in between them. Rust owns data admission, compiled schemas, execution, resource accounting, native ownership, local storage and the log protocol. TypeScript owns typed authoring, Effect composition, a thin checked transport boundary and framework integration.

The log is not a newer competing database API. It is the core database with durable command identity and a publication/lifecycle envelope. A user should learn schema, query, changes and results once. Adding S3 must not require a second schema DSL, row codec, query reader, change builder, scalar evaluator, cancellation mechanism or runtime.

The current tree is meaningfully closer to this than the old application integration: `Db`, `ChangeSet`, `QueryReader`, scoped completed results, `LocalHistory`/`HostedHistory`, command certainty, generated migration tooling and a Next.js/Alchemy example exist. The remaining work is not another wholesale API invention. It is making these concepts real all the way through their native boundaries, removing duplicate authorities and proving a packed downstream application can use them.

## The public vocabulary

| Concept | Owns / means | Must not become |
|---|---|---|
| Schema and query values | Pure, typed, reusable metadata | An import-time native operation or a live tenant handle |
| `Schema.compile` | Checked canonical schema identity and compiled native theory | A second schema interpreter in TypeScript |
| `ChangeSet.builder` | Scoped, cumulative, single-flight construction of an exact final delta | A transaction callback or arbitrary executable migration |
| `ChangeSet` | Immutable normalized changes, independently retained | Mutable row arrays the caller can edit after sealing |
| `Db` | Core local database owner | A log history with optional identity fields |
| `Snapshot` | One pinned coherent generation and its witness | A database-global prepared-query singleton |
| `QueryReader` | Core `get` and complete `execute` capability | A transport-specific query wrapper |
| `ExecutionSession` | Optional snapshot-bound reusable execution resources | A boolean alias that recompiles on every call |
| `CompleteResult` | A successfully completed answer set with owned RAM/scratch backing | A partially evaluated result masquerading as a complete set |
| `History` | Published state, named commands, recovery and receipts | An application tenant router or a second row store |
| `HistoryBorrow` | One scoped borrow from a shared history owner | Permission to close another request's owner |
| `Command` / `CommandRef` | Sealed intent / small durable recovery coordinate | A retry closure or a newly minted ID on each retry |
| `TenantCache` | One native registry under count/byte pressure | A JS LRU, TTL authority, connection framework or fleet scheduler |

The common interfaces and errors are imported literally from core. Do not re-export them under log aliases. There is one exact peer core version and one exact Effect version, **4.0.0-rc.112**, for the first converged release. There is no public C API and no `@superbuilders/errors` dependency.

### Pure declarations versus operational work

Pure means no native loading, filesystem access, hashing, canonical compilation, row iteration, Promise execution or runtime acquisition. Schema/query/scalar constructors may validate bounded local authoring structure. Expensive validation and graph lowering belong to a budgeted operation. Invalid authoring syntax can produce a clear authoring error; caller-supplied operational input must enter the typed failure channel.

All operations are lazy Effects. There is no Promise twin, sync twin, `AsyncDisposable` twin, raw callback API or SDK-level `runPromise`. Framework handlers and executable CLI entry points are the legitimate places to run Effects. A CLI executable may use Promise machinery internally; exporting its `async cli()` alongside the SDK operation API is unnecessary surface.

The root TypeScript export can remain convenient for pure and operational concepts **if its imports do not load the addon**. Defer the addon load to `NativeRuntime` acquisition. Authoring-only consumers must import a schema on a machine without a native package installed.

## Ownership and bounded work are part of the API

### One owner graph, not counters beside unrelated handles

The native registry must own actual payloads and their transitions, including retained results, cursors, drafts, changes and commands. A JS handle is a checked capability into that registry. Its runtime identity, generation, kind and state are validated natively. Removing a TypeScript export is not an authority check.

Each acquired intermediate resource is registered for cleanup before another interruptible step. Directory acquire → database open cannot be one interruptible acquisition with a finalizer installed only after both steps. A native operation must transfer an output into a registered scoped owner atomically with respect to cancellation; if delivery is abandoned or decoding fails, native ownership still has a drain path.

`close()` returns `closed`, `incomplete` with bounded obligations, or `failed`. Closed means the addressed ownership obligations have actually been reclaimed or deliberately transferred. A counter reaching zero in a subset of registries is not proof. Repeated close joins the same transition. Scope finalizers surface incomplete/failed cleanup as `CloseFailure` in the Cause while retaining the original operation failure; they never turn uncertain cleanup into success. Hard process death still requires normal crash recovery.

| Resource | Release rule |
|---|---|
| Runtime | Stops admission; cancels/drains work and all owned native payloads; reports actual remainder |
| Database/history owner | Stops new owner work; joins descendants; releases directory ownership last |
| Tenant borrow | Releases exactly its borrow; never closes the shared history |
| Snapshot | Closes its pinned lease after dependent sessions/jobs drain |
| Session | Stops its own admission and joins its own work; leaves the parent snapshot usable |
| Draft | `Building → Busy → Building`, or terminal `Spent/Closing/Closed`; failure never leaves a hidden partial builder reusable |
| Changes/command | Immutable until released; submission retains an independent native lease for its duration |
| Complete result | Independent of snapshot/session after sealing; retained charges travel with backing |
| Result pages | First run consumes the result into one private cursor; EOF, early take, interruption and downstream failure all drain it |

Retain an execution session only if it actually owns reusable prepared plans, typed bindings and safe generation-bound buffers. Its generation pin and its operation ledger are different things: a session does not donate its first call's budget/deadline to later calls. A fixed worker owns a table of snapshots and prepared state and temporarily borrows an entry for each job. Idle snapshots do not park worker stacks. Use the existing owned LMDB snapshot with a short read frame instead of a session-long read callback. Retain affinity only for genuinely worker-local state; delete the unused JS-driven writer-session ABI. Opening one snapshot must work with one configured worker.

### Budget from the first byte to the last delivery

An execution policy bounds input, work units, rows, working capacity, scratch capacity, result capacity and monotonic time. Tenant/runtime aggregate reservations additionally bound retained capacity. Their ledgers must connect to real allocation owners. No stage is allowed to copy an entire value and then discover that the value exceeded its budget.

The host boundary needs the same discipline as Rust: cheap length checks before string scans and byte copies; schema/query node/depth limits before lowering growth; input chunks constrained by both rows and bytes; cooperative event-loop turns between chunks; one large cell either takes an explicitly bounded cell path or refuses. A 64 KiB chunk condition checked only between rows is not a 64 KiB bound. Arbitrary user getters/iterators cannot be preempted by the SDK; document that fact and bound SDK-controlled work around each call.

Draft construction has one cumulative lifetime budget and deadline, including all insert/delete calls and finish. A fresh native operation per chunk must not reset the draft's allowance. Submission retry attempts share one submission budget. Result delivery is new work with a new delivery deadline: it must not inherit an expired execution deadline. A result's retained backing remains charged independently until released.

Select explicit delivery work: `collect({ maxBytes }, work)` and `pages({ pageBytes }, work)`. These start fresh bounded delivery operations; the completed result keeps its separate retained-backing reservation. Do not reuse the executing call's deadline or add a public cursor API. Native delivery is one transaction across all internal rows: a ticket advances only after the complete admitted output is registered. If a further row will not fit, deliver the accumulated nonempty page; do not discard it through a later error. The maximum is a conservative owned-conversion byte allowance covering native transfer buffers and a documented JS representation envelope, not just wire payload bytes; report substrate/RSS overhead separately rather than claiming exact JS heap prediction.

## Idiomatic Effect 4, not Effect-shaped Promise code

Use the installed RC's `Context.Service`, `Layer.effect`, `Effect.fn`, `Effect.gen`, scoped acquisition and `ManagedRuntime`. Create one runtime at the application boundary. Let Layer memoization share it. A request supplies its abort signal only to that boundary; SDK operations translate fiber interruption to native cancellation and tracked drain. There is no second `AbortSignal` on every database call.

The application service should expose the concrete schema type, not erase it behind `AnySchema`. Use `Option` for a missing keyed fact. Use the typed error channel for read/admission/transport failures where publication has not begun. Match stable structured reason variants, not English error strings. Defects mean broken invariants or unexpected host failures, not ordinary invalid work policy.

For operations that may publish, certainty belongs in the success value. `Effect<SubmitOutcome, never>` does **not** mean interruption cannot happen: interruption and finalizer failures remain in the Cause. The caller retains the command reference before dispatch and can resolve it after interruption. Never use a generic catch-all to turn interruption into `not-submitted`.

No per-row fibers, Effects, schema services, proxies or getters in returned rows. Pull and convert bounded batches into ordinary stable-shape records. The abstractions are there to describe ownership and errors, not to insert runtime layers into a hot tuple loop.

Pinned references from the preceding source-backed review (L16 must read the installed RC guidance again before lifecycle edits; this proposal pass did not execute Effect code):

- `ts/package.json:51` and `ts-log/package.json:57` pin Effect `4.0.0-rc.112`; this review does not borrow Effect 3 lifecycle assumptions.
- `ts/node_modules/effect/AGENTS.md` supplies this installed version's integration guidance.
- `ts/node_modules/effect/ai-docs/src/01_effect/05_resources/10_acquire-release.ts` explains scoped resource acquisition; the actual implementation at `src/internal/effect.ts:3971` uses `restore(acquire)` when interruptible and installs the finalizer only after the complete acquisition succeeds. That behavior is the basis of TS-003.
- `ts/node_modules/effect/src/internal/effect.ts:1145` and `:1163` show that `Effect.callback`'s returned async finalizer is interruption cleanup, not an unconditional successful-result release.
- `ts/node_modules/effect/ai-docs/src/04_integration/10_managed-runtime.ts` demonstrates `Context.Service`, `Layer.effect`, shared memoization and `ManagedRuntime` at a framework boundary. No SDK-level Promise twin is required.

## Public usage specimens

**Everything in this section is target syntax.** It is a design acceptance specimen, not a claim that these fences compile against the current package. Core TypeScript spellings intentionally track the existing surface; the proposed Rust snapshot/apply/typed-parameter surface does not yet match the callback-based public Rust API. The eventual packed-consumer gates must compile and run the final versions without private imports, casts to force schemas or hand-written protocol bytes.

### Shared TypeScript schema, query and policy

```ts
import {
  bool, contained, id128, key, on, query, relation, schema, str, v,
  type ExecutionPolicy
} from "@bjornpagen/bumbledb"

export const Note = relation("Note", { id: id128, text: str, pinned: bool })
export const Attachment = relation("Attachment", { id: id128, note: id128, object: str })
export const Notes = schema("Notes", { Note, Attachment }, [
  key(Note, ["id"]),
  key(Attachment, ["id"]),
  contained(on(Attachment, "note"), on(Note, "id"))
])

export const notesByPin = query(Notes).rule((r) => {
  const { id, text, pinned } = v(Note)
  return r.match(Note, { id, text, pinned })
    .where(r.eq(pinned, r.param("pinned")))
    .find({ id, text, pinned })
})

// Example settings, not benchmark-derived universal defaults.
export const requestWork: ExecutionPolicy = {
  inputBytes: 1_048_576n,
  workingBytes: 16_777_216n,
  scratchBytes: 67_108_864n,
  resultBytes: 4_194_304n,
  rows: 100_000n,
  workUnits: 10_000_000n,
  timeout: "2 seconds"
}
```

IDs arrive once from the application/request boundary; existing domain strings and content-addressed identifiers need not become Id128. No database allocator or implicit foreign-key entity creation is involved. A key constrains a set of complete facts; replacing a note means removing the observed complete fact and adding the intended complete fact.

### Core TypeScript: explicit creation, one delta, witnessed correction

```ts
import { ChangeSet, Db, type Id128, type QueryReader } from "@bjornpagen/bumbledb"
import { Effect, Option } from "effect"
import { Note, Notes, notesByPin, requestWork } from "./schema.ts"

export const listPinned = Effect.fn("Notes.listPinned")(
  function* (reader: QueryReader<typeof Notes>) {
    const result = yield* reader.execute(notesByPin, { pinned: true }, requestWork)
    return yield* result.collect({ maxBytes: 262_144n }, requestWork)
  },
  Effect.scoped
)

export const createLocal = Effect.fn("Notes.createLocal")(
  function* (directory: string, id: Id128, text: string) {
    const db = yield* Db.create(directory, Notes, requestWork)
    const draft = yield* ChangeSet.builder(Notes, requestWork)
    yield* draft.insert(Note, [{ id, text, pinned: false }])
    const changes = yield* draft.finish()
    return yield* db.apply(changes, { ...requestWork, expected: { kind: "any" } })
  },
  Effect.scoped
)

export const pinLocal = Effect.fn("Notes.pinLocal")(
  function* (directory: string, id: Id128) {
    const db = yield* Db.open(directory, Notes, requestWork)
    const observed = yield* Effect.scoped(Effect.gen(function* () {
      const snapshot = yield* db.snapshot(requestWork)
      return { row: yield* snapshot.get(Note, { id }, requestWork), at: snapshot.witness }
    }))
    if (Option.isNone(observed.row)) return { kind: "missing" } as const
    const draft = yield* ChangeSet.builder(Notes, requestWork)
    yield* draft.delete(Note, [observed.row.value])
    yield* draft.insert(Note, [{ ...observed.row.value, pinned: true }])
    const changes = yield* draft.finish()
    return yield* db.apply(changes, {
      ...requestWork, expected: { kind: "exact", at: observed.at }
    })
  },
  Effect.scoped
)
```

Creation refuses existing authority; open refuses absence or mismatch. Invariant rejection, no-change and a moved witness are distinct normal outcomes. The caller handles them explicitly; there is no callback retry that reexecutes arbitrary application effects. A completed result can leave its source snapshot scope if its own scope stays open; the implementation must transfer ownership, not just extend a TypeScript type.

### Rust core: the same concepts with ordinary RAII

```rust
use bumbledb::{ApplyExpected, ApplyOutcome, ChangeSet, Db, Id128, WorkContext};

bumbledb::schema! {
    pub Notes;
    relation Note { id: id128 as NoteId, text: str, pinned: bool }
    relation Attachment { id: id128 as AttachmentId, note: id128 as NoteId, object: str }
    Note(id) -> Note;
    Attachment(id) -> Attachment;
    Attachment(note) <= Note(id);
}

// Proposed public API: current Rust still uses db.read/db.write closures.
fn create_and_pin(
    directory: &std::path::Path,
    id: NoteId,
    work: &WorkContext,
) -> bumbledb::Result<ApplyOutcome> {
    let db = Db::create(directory, Notes, work)?;
    let mut draft = ChangeSet::builder(Notes, work)?;
    draft.insert([&Note { id, text: "An exact set", pinned: false }])?;
    let changes = draft.finish()?;
    match db.apply(&changes, ApplyExpected::Any, work)? {
        ApplyOutcome::Accepted { .. } | ApplyOutcome::NoChange { .. } => {}
        other => return Ok(other),
    }

    let notes_by_pin = bumbledb::query!(Notes {
        (id, text, pinned) | Note(id, text, pinned), pinned == ?pinned;
    });
    let (correction, witness) = {
        let snapshot = db.snapshot(work)?;
        let mut session = snapshot.session(work)?;
        // The query's generated parameter signature checks this bool tuple.
        let result = session.execute(&notes_by_pin, &(false,), work)?;
        let rows = result.collect(262_144, work)?;
        assert_eq!(rows.len(), 1);
        let previous = snapshot.get::<Note>(&id, work)?.expect("inserted note");
        let mut correction = ChangeSet::builder(Notes, work)?;
        // Borrowed text stays inside the snapshot scope; the builder copies
        // into its own charged canonical storage before these calls return.
        correction.delete([&previous])?;
        correction.insert([&Note { pinned: true, ..previous }])?;
        (correction.finish()?, snapshot.witness())
    };
    db.apply(&correction, ApplyExpected::Exact(witness), work)
}

// The host supplies entropy once; newtypes preserve nominal entity identity.
fn note_id(bytes: [u8; 16]) -> NoteId { NoteId(Id128::from_bytes(bytes)) }
```

The specimen does not return its borrowed `Note` from the snapshot: it constructs the independently owned canonical change set before the read lease ends. Any fact-returning API that permits scope escape must explicitly return an owned fact. The public typed tuple is preferable to users assembling positional untyped `BindValue` arrays. Preserve the macro's concise relational notation and compile-time nominal IDs; do not add a fluent Rust application framework to imitate TypeScript.

Ordinary Rust operations take an explicit bounded `WorkContext`, started by the host from its `ExecutionPolicy`. This is the selected public contract, not a hidden bridge affordance. The example shares a context to bound the whole illustrated workflow; independent calls may start independent contexts while retained capacities keep their proper owner. Delete the effectively unlimited convenience surface; do not retain a second Rust API or silently manufacture a year-long unlimited context for any execution path. One selected API accepts an explicit host policy.

### TypeScript log: identical changes and reader, additional certainty

```ts
import { ChangeSet, type Id128 } from "@bjornpagen/bumbledb"
import {
  Command, LocalHistory,
  type CommandRef, type LocalBinding, type RequestId
} from "@bjornpagen/bumbledb-log"
import { Effect } from "effect"
import { Note, Notes, requestWork } from "./schema.ts"
import { listPinned } from "./core.ts"

export const appendNote = Effect.fn("Notes.appendNote")(
  function* <E, R>(
    binding: LocalBinding,
    requestId: RequestId,
    id: Id128,
    text: string,
    retainBeforeDispatch: (ref: CommandRef) => Effect.Effect<void, E, R>
  ) {
    // Provisioning already initialized this binding from generated data.
    const history = yield* LocalHistory.open(binding, Notes, requestWork)
    const draft = yield* ChangeSet.builder(Notes, requestWork)
    yield* draft.insert(Note, [{ id, text, pinned: false }])
    const command = yield* Command.seal({
      scope: history.identity,
      id: { receiptEpoch: history.receiptEpoch, requestId },
      changes: yield* draft.finish(),
      precondition: { kind: "blind" },
      result: { note: id }
    }, requestWork)

    yield* retainBeforeDispatch(command.ref)
    const outcome = yield* history.submit(command, {
      ...requestWork, attempts: 4, backoff: { baseMillis: 25, capMillis: 250 }
    })
    return { ref: command.ref, outcome }
  },
  Effect.scoped
)

export const readPublished = Effect.fn("Notes.readPublished")(
  function* (binding: LocalBinding) {
    const history = yield* LocalHistory.open(binding, Notes, requestWork)
    const snapshot = yield* history.snapshot({
      ...requestWork, consistency: { kind: "latest" }
    })
    return {
      at: snapshot.stateStamp,
      decisionAt: snapshot.decisionStamp,
      rows: yield* listPinned(snapshot)
    }
  },
  Effect.scoped
)
```

For hosted operation, change only the binding and constructor to `HostedHistory.open`; the shared read/write language stays identical. An application process normally borrows from `TenantCache` instead of opening a history for every request. The reference callback above is an application boundary to existing durable request state, not a new required database framework/service.

The three submit cases are `decided`, `not-submitted`, and `outcome-unknown`. A decided receipt still distinguishes committed/no-change/precondition-failed/invariant-rejected, and its local-materialization health is separate from durability. Once native code knows the receipt, a subsequent ancillary decode/local-health failure must preserve that knowledge instead of degrading it to unknown. Preserve the native diagnostic-health repair that landed in the latest swarm; verify every after-dispatch adapter rather than assuming a wire type alone establishes this invariant. An unknown outcome means retain and resolve the exact ref; it does not authorize a newly identified command. Reusing a request ID for different sealed bytes must refuse. A witnessed log correction uses the published **StateStamp**, not its decision counter or a core-local witness. Receipt-only decisions may advance decision history without changing application state.

## Generated migrations: familiar workflow, native meaning

The desired experience is Drizzle-style generation, not Drizzle's SQL semantics: edit the current typed schema; supply only ambiguous intent; generate deterministic data; review and commit it; execute an explicit administrative rollout. Normal application opening never synthesizes genesis, generates files, silently migrates data or adopts an unrelated identity.

```ts
// Target spelling: Scalar.bool is the selected shared tagged literal API.
import { Scalar } from "@bjornpagen/bumbledb"
import { backfill, migrationIntent, renameField } from "@bjornpagen/bumbledb-log/schema"
import { Note, Notes } from "./schema.ts"

export const intent = migrationIntent(Notes, [
  renameField(Note, "body", "text"),
  backfill(Note, "pinned", Scalar.bool(false))
])
```

Keep one core scalar AST parameterized by leaf scope. Query variables and admitted source-schema fields are two leaf sources, not two independent arithmetic systems. Known numeric kinds remain distinct: I64 and U64 do not collapse merely because both use host bigint. A source-name leaf has unresolved kind until bound against the verified prior snapshot; it must not be advertised as an already checked program. Use explicit `Scalar.u64`, `Scalar.i64`, `Scalar.f64`, `Scalar.bool` constructors and explicit casts. A caller cannot assert an arbitrary source field's type through `field<T>("name")`. For example, Scalar.add(Scalar.field("units"), Scalar.u64(1n)) must construct useful inert metadata. Do not recursively reject its field leaf during construction. Cached depth/kind summaries keep construction constant-work per node; the native binder resolves all fields and rejects incompatible operators even on zero rows. The generator compiles field references against the verified source snapshot; missing, renamed, incompatible or unrepresentable values refuse before publication.

The repository contains the manifest, every immutable schema snapshot, every plan, a data-only import index and a tiny deployed runtime contract. The generated runtime artifact must include the snapshots needed to bind and compile plans, not just plan strings and digest claims. One Rust verification operation checks canonical schema identities, plan identities, each source/target correspondence, the entire prefix and the required expression typing. TypeScript may present diffs and unresolved intent; it does not declare a structurally parsed snapshot authoritative.

Generation scopes the existing native kernel-held repository/directory lock through the internal codec seam. Its persistent inode is never removed/replaced as stale recovery; process death releases the OS lock. No PID liveness guess, empty-lock reclaim or TTL is used, and same-process callers are excluded too. Register the owner before returning it, join all outstanding file I/O before release, and keep this out of the public application API. Read bounded bytes from the same opened file descriptor, reject invalid UTF-8, enforce an aggregate repository policy, and create unique exclusive temporary files. Publish immutable plan/snapshot content before the manifest and update the manifest atomically under the lock. Never overwrite a recorded entry because another generator raced. File/directory durability failures are explicit; “atomic rename” is not synonymous with “durable transaction.” Re-running an unchanged schema is byte-identical and writes nothing.

Administrative steps are explicit: initialize generated history for a new binding; inspect status; freeze/migrate a known source to a staged target; inspect the resulting status; activate only a verified ready target; preserve the prior recovery capability as specified by the protocol. Each mutation has a stable caller-retained operation ref before dispatch and an `AdminOutcome`. “Completed as paused” is not “ready to serve.” A later application build carries the expected schema and applied-prefix digest and refuses a mismatched deployment.

Do not add online dual-write migration callbacks, arbitrary JS transforms, a shadow TypeScript evaluator or runtime migration discovery. A public advanced creation artifact constructor can remain if it has a clear use, but the normal onboarding path must be generated `initialize`, without application code importing a private schema codec to fabricate its bytes.

## Next.js + Alchemy without a Bumbledb framework

One server-only module constructs a concrete `Context.Service<Databases, TenantCache<typeof Notes>>`; `Layer.effect` acquires the cache; `NativeRuntime.layer` is provided once; `ManagedRuntime.make` is held for the process lifetime. The request program is scoped, resolves an authenticated principal to a trusted binding, acquires its borrow and does the work. The handler calls `runPromise(program, { signal: request.signal })`. Process shutdown disposes the runtime; hard shutdown is handled by recovery. No runtime per request, per tenant or per query.

```ts
// Target server module; construct this once from immutable deployment config.
import "server-only"
import { NativeRuntime, type NativeRuntimeOptions } from "@bjornpagen/bumbledb"
import { TenantCache, type HistoryBinding, type RuntimeExpectation } from "@bjornpagen/bumbledb-log"
import { Context, Effect, Layer, ManagedRuntime } from "effect"
import { Notes, requestWork } from "./schema.ts"
import { listPinned } from "./core.ts"

export class Databases extends Context.Service<Databases, TenantCache<typeof Notes>>()("app/Databases") {}

export function makeNotesRuntime(native: NativeRuntimeOptions, expected: RuntimeExpectation) {
  const databases = Layer.effect(Databases, TenantCache.make(Notes, {
    maxOpen: 32, budgetBytes: 1_073_741_824n, maintenance: requestWork, expected
  }))
  return ManagedRuntime.make(databases.pipe(Layer.provideMerge(NativeRuntime.layer(native))))
}

export const readTenant = Effect.fn("Notes.readTenant")(
  function* (authenticatedBinding: HistoryBinding) {
    const databases = yield* Databases
    const history = yield* databases.acquire(authenticatedBinding, requestWork)
    const snapshot = yield* history.snapshot({ ...requestWork, consistency: { kind: "latest" } })
    return yield* listPinned(snapshot)
  },
  Effect.scoped
)
// Handler boundary: appRuntime.runPromise(readTenant(binding), { signal: request.signal })
```

The binding registry belongs to the application. Database bindings are trusted configuration, never paths or bucket prefixes derived directly from arbitrary request text. Local development points at durable owned local directories. Hosted bindings point at S3 authority plus disposable local materializations. Distinct active processes need noncolliding owned cache locations; a comment saying “per-process” is not enough. Role credentials use the native refreshable provider chain. Alchemy provisions resources and least-privilege grants; Bumbledb does not store long-lived AWS secrets in its schema or add a JS S3 client.

Qualify Node server runtimes only. Both native packages must be externalized and their matching platform addon included in the built deployment. Current Edullm already externalizes both in `packages/product/nextjs-app/next.config.ts`; retain and test that behavior. No Edge/browser/mobile promise follows from a TypeScript package.

## The real application acceptance target

The current sibling integration is `../edullm/packages/data/native-ledger`, not a historical learner-frontier or explanation package. It still imports the old `descriptorOf`, `openReplica`, `openWriter`, `decodeBatch` and `Replica` model, wraps Promises, manually disposes replicas, scans publication history for receipt provenance and frequently scans complete relations. It is not evidence that the new SDK is consumed successfully.

Preserve the application's real contracts while changing its adapter:

- `NativeCommand` operation/receipt identity and exact replay rules;
- one terminal fact per slot and the slot's conflict rules;
- observation identity such as `(source, reportToken)`;
- reference/lineage provenance and content-addressed artifact IDs;
- atomic claim/admit/domain facts and intentionally retained outbox behavior.

A log receipt removes the need to rediscover publication by scanning the complete braid, but does not replace these business keys. Do not truncate domain IDs into Id128 or generate new request IDs during retry. Queries replace scans only where they preserve the application's exact set grain and failure semantics. A read-dependent change carries an expected state witness; a pre-read in application code alone is not isolation.

Edullm's current documented product scope is local development/production with isolated tutor hosts; it is not authorization to introduce multitenancy there. The separate Notes example exercises the desired per-user hosted product. The app's instructions prohibit adding tests in Edullm. Put the SDK acceptance scenario beside the existing consuming tests/examples in Bumbledb, using a small faithful native-ledger schema/workflow; request separate app-owner authorization for its eventual implementation/check gate. Do not create a new fixture, exhaust or implementation-report hierarchy. Necessary golden/input data belongs beside the test that consumes it.

The release journey is packed core + packed log + real addon → schema import without addon acquisition → generated migration history → explicit initialize → reopen → native write/read → stable-ref retry/resolve → checked backfill/rename/seed migration → activate → reopen under new runtime contract. Run the same core read helper on a local core snapshot and a published snapshot, without casts/adapters. Qualify hosted publication/recovery separately against an actual supported S3 environment; a mocked store is not that gate.

## Package seam and subtraction

Use `@bjornpagen/bumbledb/internal/log` for the minimum exact-version bridge the log implementation genuinely needs: checked operation submission, shared resource constructors/accessors, compiled schema identity, row/value codec and shared addon access. It is unsupported application surface, **not a security boundary**. Ship the declarations it needs; do not use `stripInternal` to erase required types and produce an unusable published log package. Keep runtime kind/owner/generation validation even when every TS consumer is trusted.

Move public-root raw handles, mutable capability maps, callback dispatch and wire rosters there. Remove unused legacy synchronous addon functions rather than merely hiding them from docs. Derive protocol/value/error rosters from one authority where practical; keep an independent adversarial decoder test, not hand-written production twins. Rust core stays log/S3-free, while the single Node addon can depend on Rust log and AWS internals. Do not split the packages into separate native executors.

Preserve meaningful tests: canonical-value counterexamples, typed foreign-schema/key refusals, native finalizer/cancel races, failed-admission atomicity, private-output completion, one-shot page transfer, publication uncertainty, real migrations and packed downstream consumers. Consolidate duplicate type-shape/export roster tests into one declaration gate. Delete obsolete Promise/replica/allocator compatibility tests when their surfaces are removed. Keep mock adapter tests only when they isolate a real certainty/Cause/ownership distinction that is hard to drive through native code; they cannot stand in for native evidence.

Current real-bridge and adversarial tests are valuable. README fences typechecked with source path aliases are useful authoring checks, not release packaging evidence. The release gate must use installed packed declarations and binaries on supported macOS ARM64, Linux ARM64 and Linux x64 environments, with wrong-version/missing/unloadable addon cases and a no-addon pure import case. Report platform qualification honestly, not as inferred from a successful local source import.

## Done means

The user can build the same application in Rust core, TypeScript core and TypeScript log without learning competing primitives; interrupt it without losing ownership or publication certainty; evolve it through reviewed generated data; and deploy it under explicit per-tenant budgets whose actual enforcement is measured. The SDK chapter is complete only when the findings below are closed with their discriminating acceptance evidence and the final specimens run as downstream consumers.
