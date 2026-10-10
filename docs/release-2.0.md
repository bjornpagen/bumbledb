# bumbledb 2.0.0

2.0 is a hard cutover. Every on-disk layout, wire format and checkpoint image
is new, and nothing reads, detects or migrates anything written by 1.x: a
directory that is not a 2.0 store is refused as `NotABumbleDb`. Export 1.x data
with 1.x and load it into a 2.0 store (cookbook recipe 28 shows the shape of
that load).

## Formats

- **One LMDB environment per store**, ordered by LMDB's default comparator, so
  the stock `mdb_*` tools work. Rows are keyed by relation, a 16-byte home (the
  exact scalar key when there is one, the row fingerprint otherwise) and an
  ordinal; one determinant index serves every declared key. The membership
  index is gone.
- **Identity is data, not a path.** The meta records the database id, sequence
  and revision; stores and images are relocatable.
- **A fixed virtual map.** One ceiling (`Options::map_ceiling`, 1 TiB by
  default) is set at open. Address space costs nothing until pages are written.
  A write past it is `Error::Full { ceiling }`; nothing grows, retries or waits
  for readers to drain.
- **Format tags are `bdb.<kind>.v1`**: results, evidence, schemas and every log
  frame. Database directories and checkpoint images are `<name>.bdb`, the lock
  file is `bdb.lock`, and the addon is `bdb.node`. Crate names, npm names, the
  `bumbledb` CLI and environment variables are unchanged.

## Hosted databases

`bumbledb-log` is rebuilt around one rule: the log is the database.

- A database is the create-only sequence of objects `log/{seq}`, written with
  `If-None-Match: *` on an S3 Express directory bucket. There is no mutable
  head, hash chain, epoch or garbage-collection barrier, and nothing under
  `log/` is ever deleted.
- A commands entry names the head its writer judged against and carries each
  command whole with its outcome there. Catching up applies an entry that
  landed right after that head as recorded and judges a later one again where
  it landed. A writer that finds its slot taken writes its batch at the next
  slot at once, joined by the commands queued behind it, so contending writers
  share the log fairly.
- Every entry carries a per-submission nonce. An ambiguous create is resolved
  by reading the object back and comparing bytes, so retries are safe, a
  request is decided once even when two copies of its batch land, and every
  refusal except `Unknown` proves the log does not decide the command.
- A warm submit decides on a read snapshot and takes one round trip. Commands
  that arrive while a write is in flight are committed together, with no timer.
- Checkpoints are automatic, immutable images under `ckpt/` on an S3 Standard
  bucket, verified by a content digest of their rows. A cold open downloads
  the newest image and replays only the tail.
- The core is a sans-IO state machine (`Machine::step`) with deterministic
  fault-injection simulations. The TypeScript package performs its requests
  through the application's own S3 client; `FsStore` runs the same protocol
  on a local directory and `MemStore` in tests.
- The local cache is disposable and opened without sync; a lost tail is
  replayed from the log.

In TypeScript: `Database.make`, `Database.layer` and `Database.pool` (one
database per tenant, closed when idle), `submit(changes, { requestId,
precondition })` returning `Decided` or `Refused`, and reads at `"cached"`,
`"latest"` or `{ atLeast: seq }`.

## Migrations

Migrations ship with the application, like Drizzle or Expo, and are recorded
in the log. `bumbledb generate` writes `migrations/NNNN_<name>/` and a bundled
index; relations whose name and fields are unchanged are copied, so additive
changes need no code. `bumbledb check` fails on an ungenerated schema change
or an edited migration. `onOpen: "migrate"` applies pending migrations in
development; production runs `bumbledb migrate` from the deploy pipeline and
opens with `onOpen: "verify"`, which refuses while a migration is pending.

A migration lands as one entry when the log is quiet, or as Freeze, Migration
and Thaw when writers contend. A rejected migration thaws the log and stays
rejected. Code that runs on an older schema is refused with `SchemaAdvanced`.

## Engine

- **One write outcome.** `WriteOutcome<R>` is `Committed { value, generation,
  changed }`, `Rejected(Violations)` or `Moved { witnessed, current }`, for
  `write`, `write_from`, `apply` and `apply_from`, over one commit path.
- **One error type** with `Error::kind()`. Exceeding an in-memory limit is
  `Error::Capacity` (`ResidentRows`, `DistinctRows`, `Groups`, `ResultBytes`).
- **Schemas are checked at compile time.** `schema!` runs the theory checker
  during expansion and reports each error at its token. `query!` resolves
  relations and fields through the types `schema!` emits. Both live in one
  proc-macro crate, `bumbledb-macros`, built on proc-macro2 and quote.
- **The query image cache is capped** (`Options::image_cache_bytes`, 128 MiB
  by default) with least-recently-used eviction, for 512 MB devices.
- Embedding internals are private; host integrations use `bumbledb::host`.

## SIMD and floats

- Portable kernels (filters, folds, gathers, Allen classification, compaction)
  use `fearless_simd` with runtime level dispatch on x86, and every level is
  checked bit-identical to its scalar twin. The hand-tuned NEON Allen kernels
  stay.
- `f64` values have one NaN and one zero, and NaN equals itself as a value.
  Order follows IEEE: comparisons never match NaN, and `Min` and `Max`
  propagate it. A comparison between an integer column and a float literal is
  rewritten exactly at planning time.
- `Sum` and `Avg` accumulate exactly in a small superaccumulator and round
  once, to nearest even.
- Computed outputs run as a columnar register program over 64-row batches.
- The float-environment guard no longer writes control registers. A query
  with float arithmetic checks once that the environment is the IEEE default
  and refuses with `NonDefaultFloatEnvironment` otherwise.

## TypeScript

- One package, `@bjornpagen/bumbledb`: `"."` for authoring, the `Bumble`
  layer and `Database`; `"./engine"` for the embedded `Db`; the `bumbledb`
  binary. `@bjornpagen/bumbledb-log` is gone.
- Schemas and queries are validated by the engine when they are defined; the
  engine's diagnostics name the authoring relation, field and rule.
- Resources are scoped; there are no `close()` methods. Engine failures are
  one tagged `DbError`.
- Node 26 or later, Effect 4.0.2, pnpm 12.

## Removed

- Any reader, refusal or migrator for 1.x stores, logs, frames or images.
- The second, cursor-based join evaluator and every temporary-LMDB spill tier.
- Elastic map growth.
- Backup, verify and restore. The immutable log, plus versioning on the
  checkpoint bucket, is the safety net.
- Authoring without the native addon.
- `ApplyOutcome`, `ApplyExpected`, `ConditionalWrite`, the `ReadInstance`
  alias, `TenantCache`, the `bumbledb-query` and `bumbledb-query-macros`
  crates, and the `alloc-counter`, `collision-probe` and `ground-off` features
  (`testing` replaces the last two for dependent test suites).

## Toolchain

Rust nightly-2026-10-09, kept current by `scripts/bump-toolchain.sh` and a
weekly canary. CI is `scripts/ci.sh <lane>`: lint, test and addon on every pull
request; Miri (four shards on two Linux runners), musl and the deep sweeps
nightly.

## Not yet measured

2.0 has not been benchmarked. The [last full results](perf/results.md) are for
the 1.3.0 engine. The real S3 lanes and cross-zone Express latency have not
run yet.
