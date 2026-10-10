# bumbledb

An embedded relational database for Rust and TypeScript.

Schemas and queries are typed values, not SQL strings. Relations are sets, and
every write is checked against the schema's laws in its final state before it
commits. Joins run on Free Join over in-memory column images with SIMD kernels,
on top of LMDB. The same database can run serverless, as a log of commands in
S3 read through a disposable local cache.

[Release notes](docs/release-2.0.md)
· [TypeScript guide](ts/README.md)
· [Cookbook](docs/cookbook.md)
· [Design principles](docs/design/representation-first.md)
· [Benchmarks](docs/perf/results.md)

## Why bumbledb

- **Laws instead of scattered checks.** Keys, containments, interval keys,
  coverage and capacity windows are declared once. A rejected write names
  every violated law and cites the offending facts.
- **Time is a value type.** Half-open intervals are built in, Allen's 13
  interval relations are one predicate with a bitmask, and "no two bookings
  overlap" is a one-line key.
- **Exact values.** Integer arithmetic is checked, `f64` has one NaN and one
  zero, and `Sum` and `Avg` are exact and round once.
- **In-process speed.** Point reads take microseconds. Filters, folds and
  Allen classification are SIMD kernels.
- **Serverless without a server.** A hosted database is an immutable log in
  S3: one conditional `PUT` per commit, automatic checkpoints, and migrations
  that ship with your code.
- **Small.** The hardware target is a Raspberry Pi Zero 2 with 512 MB of RAM.

## Rust

A schema, a committed write, a rejected write, a typed query and a keyed read:

```rust
use bumbledb::{Db, WorkContext, WriteOutcome};

bumbledb::schema! {
    pub Ledger;

    closed relation Status as StatusId = { Open, Frozen };

    relation Holder { id: u64 as HolderId, name: str }
    relation Account {
        id: u64 as AccountId,
        holder: u64 as HolderId,
        status: u64 as StatusId,
        balance: i64,
    }

    Holder(id) -> Holder;
    Account(id) -> Account;
    Account(holder) <= Holder(id);
    Account(status) <= Status(id);
}

let dir = std::env::temp_dir().join(format!("readme-{}.bdb", std::process::id()));
let db = Db::create(&dir, Ledger, WorkContext::new())?.expect("the empty ledger admits");

db.write(WorkContext::new(), |tx| {
    tx.insert([&Holder { id: HolderId(1), name: "ada" }])?;
    tx.insert([
        &Account { id: AccountId(10), holder: HolderId(1), status: Status::Open.id(), balance: 250 },
        &Account { id: AccountId(11), holder: HolderId(1), status: Status::Frozen.id(), balance: 90 },
    ])?;
    Ok(())
})?
.expect("the write commits");

// An account whose holder does not exist violates `Account(holder) <= Holder(id)`.
let orphan = db.write(WorkContext::new(), |tx| {
    tx.insert([&Account { id: AccountId(12), holder: HolderId(2), status: Status::Open.id(), balance: 0 }])
})?;
assert!(matches!(orphan, WriteOutcome::Rejected(_)));

let open_balances = bumbledb::query!(Ledger {
    (name, balance) | Account(holder: h, status == Open, balance), Holder(id: h, name),
                      balance >= ?floor;
});
let mut prepared = db.prepare(&open_balances, WorkContext::new())?;
let rows = db.read(WorkContext::new(), |frame| {
    let answers =
        frame.execute_collect(&mut prepared, &open_balances.bind(bumbledb::params! { floor: 100i64 }))?;
    let balance = frame.get(AccountById { id: AccountId(11) })?.map(|account| account.balance);
    Ok((answers.len(), balance))
})?;
assert_eq!(rows, (1, Some(90)));

drop(prepared);
drop(db);
std::fs::remove_dir_all(&dir)?;
Ok::<(), Box<dyn std::error::Error>>(())
```

`schema!` checks the schema at compile time and reports each error at the
offending token. `query!` resolves relations and fields through the types
`schema!` generates.

### Intervals

A key whose last field is an interval forbids overlaps, not just duplicates.
Queries compare intervals with one `Allen` predicate and a mask over the 13
basic relations (`INTERSECTS`, `COVERS`, `DISJOINT`, `MEETS` or any union):

```rust
use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

bumbledb::schema! {
    pub Uptime;

    relation Service { id: u64 as ServiceId, name: str }
    relation Outage  { service: u64 as ServiceId, window: interval<i64> }

    Service(id) -> Service;
    Outage(service) <= Service(id);
    Outage(service, window) -> Outage;
}

// Services down at instant `t`.
let down_at = bumbledb::query!(Uptime {
    (service) | Outage(service, window: w), ?t in w;
});

// Services whose outage overlaps an incident window: one Allen mask.
let overlapping = bumbledb::query!(Uptime {
    (service, w) | Outage(service, window: w), Allen(w, INTERSECTS, ?incident);
});

let schema = Uptime.descriptor().validate().expect("the schema checks");
for query in [&*down_at, &*overlapping] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

Interval endpoints are `u64`, `i64` or `f64`. Coverage (`S(x, span) <= T(x,
span)`) requires the target intervals to cover each source interval, and
capacity laws can sum durations. The [cookbook](docs/cookbook.md) builds
calendars, effective dating, tax brackets and free-time search from these.

## TypeScript and serverless

The TypeScript SDK is Effect-native: every operation is a lazy, scoped
`Effect`, and the engine validates schemas and queries as you define them. A
`Database` stores its data as an immutable log and reads it through a local
cache. The same code runs on a local directory, on S3 from AWS Lambda, or in
memory in tests:

```ts
import { Effect, Option } from "effect"
import { Bumble, ChangeSet, Database, FsStore, key, type Migrations, query, relation, schema, str, uuid, v } from "@bjornpagen/bumbledb"

const Note = relation("Note", { id: uuid, text: str })
const App = schema("App", { Note }, [key(Note, ["id"])])
const notes = query(App).rule((r) => {
	const { id, text } = v(Note)
	return r.match(Note, { id, text }).find({ id, text })
})

// Generated by `bumbledb generate`: `import { migrations } from "./migrations/index.ts"`.
declare const migrations: Migrations

const program = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Database.make({
			schema: App,
			migrations,
			store: FsStore.make("data/log"),
			cache: { directory: "data/cache" },
			onOpen: "migrate"
		})
		// One request id per intent: a retry submits the same command and is decided once.
		const requestId = Database.requestId()
		const draft = yield* ChangeSet.builder(App)
		yield* draft.insert(Note, [{ id: yield* Effect.sync(() => crypto.randomUUID()), text: "hello" }])
		const outcome = yield* db.submit(yield* draft.finish(), { requestId })
		if (outcome._tag === "Refused" && outcome.refusal._tag === "Unknown") {
			// The submit may or may not have been decided; its receipt says which.
			const receipt = yield* db.resolve(requestId)
			return Option.isSome(receipt) ? receipt.value.outcome._tag : "NotDecided"
		}
		const reader = yield* db.read("latest")
		const rows = yield* (yield* reader.execute(notes, {})).collect()
		return `${rows.length} notes at revision ${reader.revision}`
	})
)

void Effect.runPromise(program.pipe(Effect.provide(Bumble.layer())))
```

- `submit` returns `Decided` with a receipt (`Committed`, `NoChange`,
  `PreconditionFailed` or `InvariantRejected`), or `Refused`. Every refusal
  except `Unknown` proves the command was not decided.
- `precondition: reader.revision` makes a submit conditional on nothing having
  committed since that read.
- Reads take `"cached"`, `"latest"` or `{ atLeast: seq }`.
- `Database.pool` opens one database per tenant on demand and closes idle ones.

### How the hosted log works

- **The log is the database.** Entries are objects `log/{seq}` in an S3
  Express bucket, each created with `If-None-Match: *` and never overwritten
  or deleted.
- **One `PUT` per commit.** Commands that arrive while a write is in flight
  commit together in the next entry; there is no batching timer.
- **Contention.** A writer that loses a slot rewrites its batch, judged again,
  at the next free slot. Every command is decided exactly once, but writers
  are not guaranteed an equal share of commits.
- **Safe retries.** Each entry carries a nonce; after an ambiguous `PUT` the
  writer reads the object back and compares bytes.
- **Fast cold starts.** Checkpoints are immutable, digest-verified images in
  an S3 Standard bucket. A cold open downloads the newest one and replays
  only the tail.
- **Tested as a state machine.** The protocol core (`crates/bumbledb-log`)
  does no I/O and runs under deterministic fault-injection simulations. The
  TypeScript package sends its requests through your `S3Client`; `FsStore`
  and `MemStore` run the same protocol locally and in memory.

### Migrations

Migrations are generated next to the schema, bundled with the application and
recorded in the log:

```sh
bumbledb generate --schema src/schema.ts#App --name add_tags   # writes migrations/NNNN_add_tags/
bumbledb check --schema src/schema.ts#App                      # fails on an ungenerated change
bumbledb migrate --config bumbledb.config.ts                   # run from the deploy pipeline
```

Relations that keep their name and fields are copied, so additive changes need
no code. Development opens with `onOpen: "migrate"`. Production runs
`bumbledb migrate` from the deploy pipeline and opens with `onOpen: "verify"`,
which refuses while a migration is pending. Code built for an older schema is
refused with `SchemaAdvanced`.

The [Notes example](examples/notes/README.md) is a server-side Next.js app
with per-tenant databases, migrations and an outbox.

## The model

- **Relations are sets.** Inserting an existing fact or deleting a missing one
  is a no-op. There are no nulls: an optional attribute is a fact in a child
  relation.
- **Identity belongs to the application.** The database issues no ids; use
  `uuid` fields or any declared key.
- **Laws:**

  | Law | Notation | Means |
  |---|---|---|
  | Key | `R(x) -> R` | `x` identifies one fact |
  | Containment | `S(y) <= T(x)` | every `y` appears as some `x`; `==` holds both ways |
  | Pointwise key | `R(x, span) -> R` | facts sharing `x` never overlap in time |
  | Coverage | `S(x, span) <= T(x, span)` | target intervals cover each source interval |
  | Capacity | `T(x) <=[w]{lo..hi} S(y)` | the weighted count per target stays in the window |

  Closed relations are fixed vocabularies whose rows are part of the schema.
- **Writes are judged on their final state.** Every write returns
  `Committed`, `Rejected(violations)` or `Moved` (its snapshot changed).
- **Queries are data.** Rules join, filter with comparisons and Allen masks,
  negate, aggregate exactly, compose named stages and recurse linearly.
- **Floats follow IEEE order.** Comparisons never match NaN, `Min` and `Max`
  propagate it, and an integer column compared with a float literal is
  rewritten exactly.

## Performance

The last full benchmark suite ran on the 1.3.0 engine (Apple M2 Max,
2026-09-11): a 0.46 µs median point lookup, a 4.96 µs range query, and lower
medians than indexed SQLite in all 32 read families.

**2.0 has not been benchmarked yet.** See the [1.3.0 results and their
limits](docs/perf/results.md).

![Read latency against indexed SQLite](assets/bench-vs-sqlite.svg)

The query image cache is capped at 128 MiB (`Options::image_cache_bytes`), and
the memory map has a fixed 1 TiB virtual ceiling (`Options::map_ceiling`) that
costs nothing until pages are written. A query over an in-memory limit fails
with `Error::Capacity`; nothing spills to disk.

## Fit

Good for embedded and edge apps, per-tenant or personal databases with one
writer each, serverless backends that should cost nothing idle, and domains
whose rules are worth stating as laws. Not for many concurrent writers on one
database, working sets much larger than memory, or ad-hoc SQL.

## Status

No stability guarantees yet, and not qualified for production.

- 2.0 is a hard cutover: every format is new, and a 1.x store is refused as
  `NotABumbleDb`.
- The S3 store passed the conformance suite against real S3 Express and
  Standard buckets, run by hand. CI covers the protocol with the in-memory and
  filesystem stores and fault-injection simulations, not S3.

## Install

Rust builds from Git with the nightly toolchain in
[`rust-toolchain.toml`](rust-toolchain.toml):

```toml
[dependencies]
bumbledb = { git = "https://github.com/bjornpagen/bumbledb" }
```

TypeScript needs Node 26 and Effect 4. The package ships native addons for
darwin-arm64, linux-arm64 and linux-x64:

```sh
pnpm add @bjornpagen/bumbledb effect
```

## Development

`scripts/ci.sh <lane>` is the body of every CI job:

```sh
scripts/ci.sh lint    # fmt, clippy, rustdoc, cargo-deny, TypeScript lint and typecheck
scripts/ci.sh test    # nextest and doctests
scripts/ci.sh addon   # native addon, TypeScript tests, packed-package smoke
scripts/ci.sh deep    # nightly: adds the wide sweeps in `deep` test modules
scripts/ci.sh miri_1  # nightly: one of four Miri shards
```

Rust is nightly, kept current by `scripts/bump-toolchain.sh` and a weekly
canary. Tests are deterministic; timings are reports, never gates. Changes
follow the [representation-first design principles](docs/design/representation-first.md).
A release is a version bump in `ts/package.json` pushed to `main`
([ts/PUBLISHING.md](ts/PUBLISHING.md)). The [static Linux ARM64
guide](docs/static-linux-arm64.md) covers the static musl build.

## Repository

- `crates/bumbledb`: the engine.
- `crates/bumbledb-theory`: values, intervals, Allen masks and the schema checker.
- `crates/bumbledb-macros`: `schema!`, `query!` and `params!`.
- `crates/bumbledb-log`: the hosted log's protocol core and its LMDB cache.
- `crates/bumbledb-node`: the Node bridge.
- `crates/bumbledb-bench`: independent oracles (a naive evaluator, SQLite differentials) and benchmarks.
- `ts/`: the TypeScript SDK and the `bumbledb` CLI.
- `examples/`: consumers and the Notes app.
- `proposals/`: research proposals.

## License

[0BSD](LICENSE).
