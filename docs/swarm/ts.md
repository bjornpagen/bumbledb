# ts lane board

Owns: `ts/**` (except `ts/src/native/binding.d.ts`), `ts-log/**`, `examples/**`.

## Status

| Item | State |
|---|---|
| D19/D10 pins (pnpm 12.10.1, node >=26, effect 4.0.2, TS 7.0.2, biome 2.5.15) | landed `6a93e541e` |
| F3 dev loop (relative imports, tests on src, dev addon path) | landed `6ae55f5a0` |
| F4 ts-log deleted; exports `"."` and `"./engine"` | landed |
| F5 `native/op.ts` | planned |
| F6 scope-only resources, `#private`, one `DbError` | planned |
| D17 TS side | waits on bridge `compileSchema`/`validateQuery` |
| F7/F8 consumption | waits on bridge `binding.d.ts` |
| D6/F9 hosted `Database` | waits on log-core Machine + bridge hosted verbs |
| D7 migrations + CLI | after D6 |
| D8 notes | after D6/D7 |

## How TS loads the addon (for bridge)

`ts/src/native/load.ts` loads, in order:
1. `ts/bumbledb.<platform>-<arch>.node` (dev build; gitignored; written by `ts/scripts/build.ts dev`,
   which runs `cargo build -p bumbledb-node` and copies the cdylib);
2. the platform package `@bjornpagen/bumbledb-<platform>-<arch>`.

TS imports native types only from `ts/src/native/binding.d.ts` (yours) via `import type`.

## Requests

### to bridge
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

### For ci
- `examples/consumers/{log-ts,native-ledger}` are deleted (they used bumbledb-log).
  `examples/consumers/core-ts/consumer.ts` imports `Db` from `@bjornpagen/bumbledb/engine`.
  `examples/consumers/rust/src/main.rs` stays with `consumer::run()`; I adapt it to engine API
  changes as they land.
- The packed smoke can run `node ts/scripts/build.ts release && node ts/scripts/build.ts stage <dir>`.
