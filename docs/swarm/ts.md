# ts lane board

Owns: `ts/**` (except `ts/src/native/binding.d.ts`), `ts-log/**`, `examples/**`.

## Status

| Item | State |
|---|---|
| D19/D10 pins (pnpm 12.10.1, node >=26, effect 4.0.2, TS 7.0.2, biome 2.5.15) | landed `6a93e541e` |
| F3 dev loop (relative imports, tests on src, dev addon path) | landed `6ae55f5a0` |
| F4 ts-log deleted; exports `"."` and `"./engine"` | landed |
| F5 `native/op.ts` | landed |
| F6 scope-only resources, `#private`, one `DbError`, `Bumble` layer, branded `SchemaId` | landed |
| D17 TS side (sync compile/validate at definition, mirrored checks deleted, diagnostics named) | landed |
| F7/F8 consumption (generated `binding.d.ts`, JSON inputs, `_tag` outputs) | landed |
| D6 stores (MemStore with faults, FsStore, S3Store), I/O executor, machine driver | landed |
| D6/F9 `Database.make/layer/pool`, submit, consistency, migrations on open | landed |
| D7 `bumbledb generate / check / migrate` | landed |
| D8 notes | after D6/D7 |

## How TS loads the addon (for bridge)

`ts/src/native/load.ts` loads, in order:
1. `ts/bumbledb.<platform>-<arch>.node` (dev build; gitignored; written by `ts/scripts/build.ts dev`,
   which runs `cargo build -p bumbledb-node` and copies the cdylib);
2. the platform package `@bjornpagen/bumbledb-<platform>-<arch>`.

TS imports native types only from `ts/src/native/binding.d.ts` (yours) via `import type`.

## Requests

### to bridge
- R-B4 (D6): the TS side mirrors log-core's `IoRequest`/`IoResponse`/`Op`/`IoResult` with `_tag`
  unions (`ts/src/database/io.ts`): `{ id: bigint, bucket: "Log" | "Checkpoints", key, op }`,
  `op: { _tag: "Get", target: { _tag: "Memory" } | { _tag: "File", path } } | { _tag: "PutIfAbsent",
  body: { _tag: "Bytes", bytes } | { _tag: "File", path } } | { _tag: "List", startAfter: string | null,
  maxKeys } | { _tag: "Delete" }`; responses `{ id, date: bigint | null, result }` with result
  `Body{bytes,lastModified} | Saved{lastModified} | Missing | Created | Occupied | Keys{keys} |
  Deleted | Failed`. If your generated shapes differ, I follow yours. The driver needs, per open
  database: `request(ticket, input) -> Step`, `respond(IoResponse) -> Step`, `close() -> Step`, with
  `Step = { io: IoRequest[], done: { ticket, settled }[] }` (async on the executor is fine).
- R-B1: please put every napi export's types in `ts/src/native/binding.d.ts` (generated, committed).
  TS reads only that file; the hand-written `native.ts`, `runtime-native.ts`, `db-native.ts` and
  ts-log `native.ts` are deleted on the TS side as your outputs land.
- R-B2 (D17): synchronous `compileSchema(specJson: string)` returning a tagged union
  `{ _tag: "Compiled", schema: SchemaHandle, descriptor } | { _tag: "Invalid", diagnostic }` and
  `validateQuery(schema: SchemaHandle, irJson: string)` returning
  `{ _tag: "Valid", query: QueryHandle } | { _tag: "Invalid", diagnostic }` where the diagnostic
  carries rule/atom/find indices plus a stable code, so TS can map back to authoring names.
- R-B3 (D6): the log-core Machine over napi. TS drives `step(input) -> { io: IoRequest[], done?: Outcome }`
  and feeds back `IoResponse { id, ... }`. Please announce the exact verb names/shapes here or on
  your board as soon as they exist; TS will follow them.

### to log-core
- R-L1: please announce the `Machine` input/step/IoRequest/IoResponse/Outcome shapes (U11) and the
  `SubmitOutcome` union on your board; the TS executor mirrors them 1:1.

## API announcements

### Landed
- `@bjornpagen/bumbledb-log` (ts-log) is deleted. `@bjornpagen/bumbledb/internal/log` is deleted.
- `@bjornpagen/bumbledb` exports `"."` (authoring, runtime, errors; `Database`/`Migration` later)
  and `"./engine"` (`Db`, `Snapshot`, `PreparedQuery`, `QueryReader`, `ApplyOutcome`, ...).
- Dev loop: `cd ts && node scripts/build.ts dev` (debug addon at `ts/bumbledb.<platform>-<arch>.node`),
  then `pnpm test` runs `node --test` straight on `src` (no `dist`). `node scripts/build.ts release`
  builds the optimized addon into `npm/<platform>-<arch>/` plus `dist/`;
  `node scripts/build.ts stage <out>` packs the main package (platform packages pinned as optional
  dependencies) and each platform package that holds an addon.
- Deleted from `ts/scripts`: absence-gate, pin, declarations, platform, native-artifact, stage,
  runtime-identities, generate-law-scale, errors. No pack provenance, no version roster.

- `ts/src/native/op.ts`: `call(operation, start, take)`, `drain(operation, start)`,
  `release(operation, start)`, `scoped(operation, acquire, close)`. They replace
  `nativeOperation`/`nativeOperationWith`/`finalizeClose`/`close.ts` and the interrupt stash
  (Effect 4.0.2 keeps the interrupt beside a cleanup defect; a test pins it).

- `ObjectStore` (`ts/src/database/io.ts`): `get(bucket, key, target)`, `putIfAbsent(bucket, key, body)`,
  `list(prefix, startAfter, maxKeys)` and `delete(key)` (the last two only exist for `Checkpoints`).
  `MemStore.make({ clock?, fault? })`, `FsStore.make(root)` (temp + fsync + `link` + dir fsync),
  `S3Store.make({ client, log: { bucket }, checkpoints: { bucket }, prefix })` over the app's
  `S3Client` (optional peer `@aws-sdk/client-s3`, loaded on first use; reads the `Date` header).
- `execute(store, IoRequest, { timeout, readRetry })` -> `IoResponse`: reads retry, a PUT reports
  its first non-200 (`Occupied` for 412, else `Failed`), list/delete on `Log` are refused.
- `Driver.make(port, store, { concurrency, executor })` (scoped): one mailbox, one machine step at a
  time, IoRequests run concurrently and feed back as responses; tickets settle exactly once; scope
  close sends the machine `Close`.
- `pnpm --dir ts run test:s3` runs the ObjectStore conformance suite against the CI S3 contract
  (`BUMBLEDB_S3_*`); the regular suite runs the same conformance against MemStore, FsStore and
  S3Store over an in-process fake S3.

- F6: `NativeRuntime` is renamed `Bumble` (`Bumble.layer(options)`, `BumbleOptions`); its service
  has `inspect()` only. No resource has `close()`: Db, Snapshot, PreparedQuery, CompleteResult,
  ChangeSet and ChangeDraft are released by their scope. Errors live in `ts/src/errors.ts`:
  `AuthoringError`, `NativeLoadError`, `DbError { operation, reason }`, `CloseFailure` (all
  `Schema.TaggedError`); `SdkInvariantError`, `NativeOperationError` and `NativeReportedError` are
  gone (contradictions are `DbError` with reason `Internal`). `SchemaId` is a branded string.

- D17/F7/F8: `schema()` compiles with `compileSchema` when it is defined and `query(...).rule(...)`
  validates with `validateQuery` when it is built; engine refusals throw `AuthoringError` naming the
  schema's statements/relations and the query's tables, finds, params and variables. TS keeps only
  checks the engine cannot see (relation identity, record keys, class joins, lowering shape).
  `Schema.compile`/`CompiledSchema`, `describeQuery`/`queryFromDescription` and the TS IR parser are
  deleted. All native types come from `ts/src/native/binding.d.ts` through `ts/src/native/addon.ts`.
- Public outcome unions are the generated wire types: `Db.apply(changes, expected?)` and
  `Db.judge(changes, expected?)` (`expected` is a `Witness`, omitted means current) return
  `ApplyOutcome` (`Committed | NoChange | Rejected | Moved`) and `JudgeOutcome`
  (`Admitted | Rejected | Moved`); violations are `ViolationOut`; `ChangeRecord.kind` is
  `"Add" | "Remove"`; `DbInspection` is `{ schemaId, generation, diskBytes, retainedOperations }`.
  `DbError.reason` mirrors the addon's `RuntimeError` union; `CloseFailure.report` is `CloseOut`.

- Hosted (`ts/src/database/database.ts`), exported from the package root:
  - `Database.make({ schema, migrations, store, cache: { directory }, onOpen: "migrate" | "verify", tuning? })`
    (scoped) and `Database.layer(Database.tag<S>("Key"), options)`; `Database.pool({ ...options,
    store: (tenant) => ObjectStore, cache: (tenant) => { directory }, idleTimeToLive? })` with
    `pool.get(tenant)`.
  - `db.submit(changes, { requestId, precondition? })` -> `SubmitOutcome` =
    `Decided { receipt } | Refused { refusal }` (the generated `SettledOut` arms);
    `db.read("cached" | "latest" | { atLeast: seq })` -> scoped `QueryReader` plus `seq`;
    `db.resolve(requestId)` -> `Option<ReceiptOut>`; `db.head`.
  - `Migration.make({ id: "NNNN_name", hash: 64 hex, from?, to, populate? })`; relations that keep
    name and fields are copied by default; `populate({ from: QueryReader<From>, into: ChangeDraft<To> })`
    adds rows; the initial migration's `populate` seeds a newly created database once (after every
    migration ran; request id from its hash). Readers carry `seq` and `revision`. `onOpen: "verify"` fails with `DbError` reason `Engine { kind: "MigrationPending" }`.
  - `RequestId` (32 hex digits) and `Database.requestId()`.

- `bumbledb` bin (`ts/src/bin.ts`, `dist/bin.js`): `generate --schema <file>#<export> --name <name>
  [--migrations dir]` writes `NNNN_name/{schema.json, schema.ts, migration.ts}` and `index.ts`
  (`export const migrations = [...] as const`); `check --schema ...` verifies hashes, the index and
  that the schema has no ungenerated change; `migrate --config <file>` opens the config's database
  (default export: the app's `Database` options) with `onOpen: "migrate"`. A migration hash is
  sha256 over `id + "\n" + schema.json`. `@effect/platform-node` is no longer a dependency.

### For ci
- `examples/consumers/{log-ts,native-ledger}` are deleted (they used bumbledb-log).
  `examples/consumers/core-ts/consumer.ts` imports `Db` from `@bjornpagen/bumbledb/engine`.
  `examples/consumers/rust/src/main.rs` stays with `consumer::run()`; I adapt it to engine API
  changes as they land.
- The packed smoke can run `node ts/scripts/build.ts release && node ts/scripts/build.ts stage <dir>`.
