# bumbledb 2.0.0

2.0 rebuilds the hosted log, the TypeScript SDK and much of the engine. It is a
hard cutover: every on-disk layout, wire format and checkpoint image is new, and
nothing written by 1.x is read, detected or migrated. A directory that is not a
2.0 store is refused as `NotABumbleDb`. To move data, export it with 1.x and
load it into a 2.0 store (cookbook recipe 28 shows the load).

## Hosted databases

`@bjornpagen/bumbledb-log` is gone. Hosted databases are part of the main
package, and the protocol is rebuilt around one rule: the log is the database.

- **Create-only log.** A database is the sequence of objects `log/{seq}` on an
  S3 Express directory bucket, each written with `If-None-Match: *`. Nothing
  under `log/` is overwritten or deleted. The mutable head, hash chain, epochs,
  garbage-collection barriers and receipt retirement are gone.
- **Entries carry their decisions.** A batch records the head its writer judged
  against and every command with its outcome there. Catching up applies a batch
  that landed right after that head as recorded and judges a later one again
  where it landed.
- **One round trip per commit.** A warm submit decides against a read snapshot
  and creates the next entry. Commands that arrive while a write is in flight
  commit together in the next entry, with no timer.
- **Contention.** A writer that loses its slot (412 or 409) rewrites its batch
  at the next slot at once, judged again with any commands queued behind it. A
  writer moves past at most one refused slot it has not read; an empty slot
  left below the head is filled by a copy of the entry above it, by any reader.
  Every command is decided exactly once; writers are not guaranteed an equal
  share of commits.
- **Safe retries.** Every entry carries a per-submission nonce. An ambiguous
  `PUT` is settled by reading the slot back and comparing bytes, so a request
  is decided once even when two copies of its batch land. Every refusal except
  `Unknown` proves the log does not decide the command. Reads are retried with
  backoff; `PUT`s are never retried blindly.
- **Checkpoints.** Every 256 entries a writer stores an immutable,
  digest-verified image under `ckpt/` on an S3 Standard bucket and keeps the
  newest 3. Keys sort newest first, so a cold open lists one key, reads the log
  tail while the image downloads, and replays only that tail.
- **Sans-IO core.** The protocol is a Rust state machine (`Machine::step`)
  tested with deterministic fault-injection simulations. The TypeScript package
  sends its requests through the application's own `S3Client`
  (`@aws-sdk/client-s3` is an optional peer). `FsStore` runs the same protocol
  on a local directory and `MemStore` in memory, with fault injection.
- **Disposable cache.** The local LMDB cache is opened without sync; a lost
  tail is replayed from the log. A batch whose recorded outcome does not match
  its replay stops the database with `Diverged`.

## Migrations

Migrations ship with the application, as with Drizzle, and are recorded in the
log.

- `bumbledb generate --schema <file>#<export> --name <name>` writes
  `migrations/NNNN_<name>/` and a bundled index. Relations whose name and
  fields are unchanged are copied, so additive changes need no code; a
  `populate` step computes anything new.
- `bumbledb check` fails on a schema change without a generated migration, or
  on an edited one.
- `bumbledb migrate --config <file>` applies pending migrations from the
  deploy pipeline. Development opens with `onOpen: "migrate"`; production
  opens with `onOpen: "verify"`, which refuses while a migration is pending.
- A migration lands as one entry when the log is quiet. After 3 stale
  attempts, the migrator writes a Freeze with a 5-minute lease, measured by
  the store's clock, then the Migration, then a Thaw; any writer may thaw an
  expired freeze. A rejected migration thaws the log and stays rejected.
  Migration images live under `mig/` on the checkpoint bucket.
- Code built for an older schema is refused with `SchemaAdvanced`.

## TypeScript

- **One package,** `@bjornpagen/bumbledb`, with two entry points: `"."` for
  authoring, the `Bumble` layer and `Database`, and `"./engine"` for the
  embedded `Db`. It ships the `bumbledb` CLI.
- **`Database`:** `Database.make`, `Database.layer` and `Database.pool` (one
  database per tenant, closed after 5 idle minutes). `submit(changes,
  { requestId, precondition })` returns `Decided` with a receipt or `Refused`.
  `read("cached" | "latest" | { atLeast: seq })` returns a reader with `seq`
  and `revision`. `resolve(requestId)` returns the receipt, and `head` the log
  head. `Database.requestId()` mints a `RequestId`.
- **Validated authoring.** `schema()` and `query()` are compiled by the native
  addon when they are defined, and refusals throw `AuthoringError` naming the
  relation, field and rule. Authoring therefore needs the addon.
- **Scoped resources.** There are no `close()` methods.
- **Errors.** Every error is an Effect tagged error. Engine failures are one
  `DbError { operation, reason }`. `NativeLoadError` reports the `target`.
  `SchemaId` is branded.
- **Renamed:** `NativeRuntime` to `Bumble`, `NativeRuntimeOptions` to
  `BumbleOptions`, `SchemaDeclaration` to `Schema`, `CoreWitness` to `Witness`
  (in `./engine`).
- **Moved to `./engine`:** `Db`, `Snapshot`, `DbInspection`, `ApplyOutcome`
  and `JudgeOutcome`. `Db.apply` and `Db.judge` take `(changes, expected?)`,
  and outcomes are tagged `Committed { generation, changed }`, `Rejected` or
  `Moved`.
- **Removed:** `Schema.compile` and `CompiledSchema`, `describeQuery` and the
  description, IR and spec types, `StorageInspection`, `WriteOptions`,
  `WriteExpected`, `runtimeErrorCodes`, the native error classes and the
  internal log export.
- **Requirements:** Node 26 or later and Effect `^4.0.2`. The addon file is
  `bdb.node`, in one platform package each for darwin-arm64, linux-arm64 and
  linux-x64.

## Engine

- **New store layout.** One LMDB environment with named databases `meta`,
  `rows` and `det`, ordered by LMDB's default comparator, so the stock `mdb_*`
  tools work. A row key is 26 bytes: relation, a 16-byte home (the exact scalar
  key when there is one, the row fingerprint otherwise) and an ordinal. One
  determinant index serves every declared key; the membership index is gone. A
  schema holds at most 65,536 relations.
- **Relocatable identity.** The meta records the database id, sequence and
  revision. `Db::compact` is a compacting LMDB copy that keeps the id.
- **Fixed virtual map.** `Options::map_ceiling` (1 TiB by default) is set at
  open and costs nothing until pages are written. A write past it fails with
  `Error::Full { ceiling }`.
- **Options.** `Db::create_with` and `Db::open_with` take `Options`:
  `map_ceiling`, `durability` (`Durable` or `Cache`) and `image_cache_bytes`
  (128 MiB by default, least-recently-used eviction).
- **One write outcome.** `write`, `write_from`, `apply` and `apply_from` return
  `WriteOutcome<R>`: `Committed { value, generation, changed }`,
  `Rejected(Violations)` or `Moved { witnessed, current }`. A write that
  changes nothing is `Committed { changed: false }`. `apply` no longer takes an
  expected snapshot; `apply_from` is the conditional form.
- **One error type** with `Error::kind()` (`ErrorKind`, formerly
  `ErrorFamily`). Exceeding an in-memory limit is `Error::Capacity`
  (`ResidentRows`, `DistinctRows`, `Groups`, `ResultBytes`); nothing spills to
  disk. New variants include `NotABumbleDb`, `Locked`, `Closed`,
  `ReentrantWriter`, `Full`, `ForeignSchema`, `Cancelled` and `Allocation`; the
  1.x format, sync and hatch variants are gone. A plan the validator cannot
  build is refused as `Unplannable`.
- **Compile-time schemas.** `schema!` runs the theory checker during expansion
  and reports each error at its token. It emits struct constants such as
  `Ledger::Account` with `FieldId` fields, replacing `Ledger::ACCOUNT`-style
  constants. `query!` resolves relations and fields through those types and
  accepts string literals. Both macros live in `bumbledb-macros`.
- **Smaller surface.** `bumbledb::store`, `bumbledb::integration`, `digest`,
  `value`, `schema::judge` and `schema::compiled` are private. Host
  integrations use `bumbledb::host`. `ReadFrame::witness()` is infallible, and
  `verify_store` takes a `WorkContext`.
- **Evaluation.** The cursor-based second join evaluator and every
  temporary-LMDB spill tier are deleted; one rule runner remains. A key
  violation over intervals cites every overlapping row.

## SIMD and floats

- Portable kernels (filters, folds, gathers, Allen classification,
  compaction) use `fearless_simd` with runtime dispatch on x86, and every level
  is checked bit-identical to its scalar twin. The hand-tuned NEON Allen
  kernels stay. The engine crates need no `#![feature]`.
- `f64` order follows IEEE: comparisons never match NaN (1.x sorted NaN above
  infinity), and `Min` now propagates NaN as `Max` did. A value still has one
  NaN and one zero, and NaN equals itself as a value.
- A comparison between an integer column and a float literal is rewritten
  exactly at planning time; comparing two variables of different numeric types
  is refused as `MixedNumeric`.
- `Sum` and `Avg` stay exact, now on a small superaccumulator that rounds once
  to nearest even. Computed outputs run as register programs over 64-row
  batches.
- A query with float arithmetic checks once that the floating-point
  environment is the IEEE default and refuses with `NonDefaultFloatEnvironment`
  otherwise; nothing writes control registers.

## Removed

- Every reader, refusal or migrator for 1.x stores, logs, frames and images.
- Elastic map growth.
- Backup, restore and erase. The immutable log, plus versioning on the
  checkpoint bucket, is the safety net. (`Db::verify_store`, the offline
  consistency sweep, stays.)
- Authoring without the native addon.
- `ApplyOutcome`, `ApplyExpected`, `ConditionalWrite`, `ReadInstance` and
  `CollectionBuilder` in Rust; `TenantCache`; the `bumbledb-query` and
  `bumbledb-query-macros` crates; the `alloc-counter`, `collision-probe` and
  `ground-off` features (`testing` replaces them for dependent test suites).

## Releases and toolchain

- A push to `main` that sets a `ts/package.json` version npm does not have
  releases it: CI publishes all four packages through npm trusted publishing,
  with provenance, and creates the GitHub release. No token is stored.
- Rust nightly-2026-10-09, kept current by `scripts/bump-toolchain.sh` and a
  weekly canary. Every push runs lint, tests on three platforms and the three
  addon builds; Miri (eight shards), the static musl build, the deep sweeps,
  udeps and the NEON assembly check run nightly.

## Not yet measured

2.0 has not been benchmarked. The [last full results](perf/results.md) are for
the 1.3.0 engine. The S3 store passed the conformance suite against real S3
Express and Standard buckets by hand; no CI lane runs against S3, and
cross-zone Express latency has not been measured.
