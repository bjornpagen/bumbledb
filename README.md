# bumbledb

An embedded relational database for Rust and TypeScript.

Schemas and queries are typed values in your code, not SQL strings. Relations
are sets. Every write is judged against the schema's laws in its final state
before it commits. Joins run on Free Join over in-memory column images, with
SIMD kernels underneath and LMDB for storage. The same database can also run
serverless: the log of commands lives in S3, and every process reads through
a disposable local cache.

[Release notes](docs/release-2.0.md)
· [TypeScript guide](ts/README.md)
· [Cookbook](docs/cookbook.md)
· [Design principles](docs/design/representation-first.md)
· [Benchmarks](docs/perf/results.md)

## Why bumbledb

- **Laws, not checks scattered through your code.** Keys, containments,
  interval keys, coverage and capacity windows are declared once in the
  schema. The engine enforces them on the final state of every write. A
  rejection names every violated law and cites the offending facts.
- **Time is a first-class shape.** Half-open intervals are a value type.
  Allen's 13 interval relations are a single query predicate taking a 13-bit
  mask, and "no two bookings overlap" is a one-line key.
- **Exact values.** Integer arithmetic is checked. An `f64` has exactly one
  NaN and one zero, so set identity is well defined. `Sum` and `Avg` are
  exact and round once.
- **In-process speed.** Point reads take microseconds, not network round
  trips. Prepared queries reuse their buffers. Filters, folds and Allen
  classification are SIMD kernels, dispatched at runtime on x86, with
  hand-tuned NEON on ARM.
- **Serverless without a database server.** A hosted database is an immutable
  log in S3: one conditional `PUT` per commit, automatic checkpoints, and
  migrations that ship with your code.
- **Small enough for a Raspberry Pi.** The hardware target is a Pi Zero 2
  with 512 MB of RAM.

## A first look in Rust

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

`schema!` checks the declaration while it compiles. A duplicate field, a key
arrow that names another relation, or a containment whose target matches no
declared key is a compile error at the offending token. `query!` resolves
relations, fields and closed handles through the types `schema!` generates.

## Intervals and Allen masks

A key whose last field is an interval is a *pointwise* key: it forbids
overlaps, not just duplicates. A query compares intervals with one `Allen`
predicate and a mask over the 13 basic relations. Named masks include
`INTERSECTS`, `COVERS`, `DISJOINT` and `MEETS`, and any union of relations
works:

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

// The Rust query notation constructs an AST directly. Down at instant `t` — point membership
// (`in`) is a typing rule.
let down_at = bumbledb::query!(Uptime {
    (service) | Outage(service, window: w), ?t in w;
});

// Overlapping an incident window — one Allen mask, no operator zoo.
let overlapping = bumbledb::query!(Uptime {
    (service, w) | Outage(service, window: w), Allen(w, INTERSECTS, ?incident);
});

let schema = Uptime.descriptor().validate().expect("the schema checks");
for query in [&*down_at, &*overlapping] {
    bumbledb::ir::validate::validate(&schema, query).expect("the query validates");
}
```

**Intervals:**
- **Endpoints** can be `u64`, `i64` or `f64`; fixed-width intervals store only their start.
- **Coverage** (`S(x, span) <= T(x, span)`) holds when the union of the target intervals covers each source interval.
- **Capacity** can sum durations.

The [cookbook](docs/cookbook.md) builds calendars, effective-dated
configuration, disjoint covers, tax brackets and free-time coalescing from
these pieces.

## TypeScript and serverless

The TypeScript SDK is Effect-native. Every operation is a lazy, scoped
`Effect`, and the engine validates schemas and queries as they are defined. A
`Database` keeps its data as an immutable log in an object store and reads it
through a local cache. The same code runs against a local directory, against
S3 on AWS Lambda, or in memory in tests:

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

**Submits and reads:**
- **`submit` outcomes.** It returns `Decided` with a receipt (`Committed`, `NoChange`, `PreconditionFailed` or `InvariantRejected`), or `Refused`. Every refusal except `Unknown` proves the log does not decide the command.
- **Optimistic writes.** A `precondition: reader.revision` makes a submit conditional on nothing having committed since that read.
- **Read consistency.** Reads take `"cached"`, `"latest"` or `{ atLeast: seq }`.
- **Per-tenant databases.** `Database.pool` opens one database per tenant on demand and closes idle ones.

### How the hosted log works

1. **The log is the database.** A database is the sequence of objects `log/{seq}` in an S3 Express directory bucket, each created with `If-None-Match: *`. Nothing under `log/` is ever overwritten or deleted.
2. **One `PUT` per commit.** A warm submit decides against a read snapshot and creates the next entry. Commands that arrive while a write is in flight commit together in the next entry, with no batching timer.
3. **Fair under contention.** An entry names the head its writer judged against and carries each command whole with its outcome there. A writer that finds its slot taken writes its batch at the next slot at once, joined by the commands queued behind it, while it reads the winner, so every writer races for every slot. Catching up applies an entry that landed right after its head as recorded and judges a later one again where it landed.
4. **Safe retries.** Each entry carries a per-submission nonce. After an ambiguous `PUT`, the writer reads the object back and compares bytes. A request is decided once, even when two copies of its batch land.
5. **Bounded cold starts.** Checkpoints are automatic, immutable images on an S3 Standard bucket, verified by a digest of their contents. A cold open lists checkpoints and reads the log in the same round trip, fetches the tail while the newest image downloads, and replays only that tail.
6. **Sans-IO core.** The protocol is a Rust state machine (`crates/bumbledb-log`) tested with deterministic fault-injection simulations. The TypeScript package runs its requests through your own `S3Client`. `FsStore` runs the same protocol on a local directory, and `MemStore` runs it in memory.

### Migrations ship with your code

Migrations work the way they do with Drizzle on Expo: they are generated next
to the schema, bundled with the application, and recorded in the log.

```sh
bumbledb generate --schema src/schema.ts#App --name add_tags   # writes migrations/NNNN_add_tags/
bumbledb check --schema src/schema.ts#App                      # fails on an ungenerated change
bumbledb migrate --config bumbledb.config.ts                   # run from the deploy pipeline
```

**Migration rules:**
- **Additive changes need no code.** Relations that keep their name and fields are copied, and a `populate` step computes anything new.
- **Development:** `onOpen: "migrate"` applies pending migrations when the database opens.
- **Production:** run `bumbledb migrate` from the deploy pipeline and open with `onOpen: "verify"`, which refuses while a migration is pending.
- **Old code:** code built for an older schema is refused with `SchemaAdvanced`.

The [Notes example](examples/notes/README.md) is a server-side Next.js
application with per-tenant databases, migrations and an outbox.

## The model

- **Relations are sets.** Inserting a fact that exists and deleting one that does not are no-ops. There are no nulls: an optional attribute is an absent fact in a child relation.
- **Identity belongs to the application.** The database issues no ids; use `uuid` fields or any declared key.
- **Laws are statements:**

  | Law | Notation | Means |
  |---|---|---|
  | Key | `R(x) -> R` | `x` identifies one fact |
  | Containment | `S(y) <= T(x)` | every `y` appears as some `x`; `==` holds both ways; selections narrow either side |
  | Pointwise key | `R(x, span) -> R` | facts sharing `x` never overlap in time |
  | Coverage | `S(x, span) <= T(x, span)` | target intervals jointly cover each source interval |
  | Capacity | `T(x) <=[w]{lo..hi} S(y)` | the weighted count per target stays in the window |

  Closed relations are fixed vocabularies whose rows are part of the schema.
- **Writes are judged once, on the final state.** A delete and an insert in the same write never pass through an invalid intermediate state. Every write returns one `WriteOutcome`: `Committed`, `Rejected(violations)` or `Moved` (when the write was conditioned on a snapshot that has since changed).
- **Queries are data.** Rules join atoms, filter with comparisons and Allen masks, negate, aggregate exactly, compose named stages and recurse linearly.
- **Floats follow IEEE order.** Comparisons never match NaN, and `Min` and `Max` propagate it. Comparing an integer column with a float literal is rewritten exactly at planning time.

## Performance

The last full benchmark suite ran on the 1.3.0 engine (Apple M2 Max, 2026-09-11):
- a 0.46 µs median point lookup;
- a 4.96 µs range query;
- lower medians than indexed SQLite in all 32 read families.

**2.0 has not been benchmarked yet.** It rebuilds the hosted log and much of the engine. See the [full 1.3.0 results and their limits](docs/perf/results.md).

![Read latency against indexed SQLite](assets/bench-vs-sqlite.svg)

**Memory:**
- The query image cache is capped at 128 MiB by default (`Options::image_cache_bytes`).
- The memory map has a fixed 1 TiB virtual ceiling (`Options::map_ceiling`) that costs nothing until pages are written.
- A query that exceeds an in-memory limit fails with `Error::Capacity`; nothing spills to disk.

## When to use it

**It fits:**
- embedded and edge applications;
- tools and services where microsecond reads matter;
- per-tenant or personal databases with one writer each;
- serverless backends that should cost pennies when idle;
- domains whose rules are worth stating as laws.

**It doesn't fit:**
- many concurrent writers on one database;
- working sets much larger than memory;
- ad-hoc SQL or BI access. Use a server database for those.

## Status

bumbledb has no stability guarantees yet.

- **No upgrade path from 1.x.** 2.0 is a hard cutover: every format is new, and a 1.x store is refused as `NotABumbleDb`.
- **The S3 path is not proven against AWS yet.** The S3 test lanes (SeaweedFS on every pull request, AWS S3 Express and Standard on main) have not run against real buckets.
- **Not qualified for production.** Treat the hosted mode accordingly.

## Install

Rust consumers build from Git with the nightly toolchain pinned in
[`rust-toolchain.toml`](rust-toolchain.toml):

```toml
[dependencies]
bumbledb = { git = "https://github.com/bjornpagen/bumbledb" }
```

TypeScript requires Node 26 and Effect 4:

```sh
pnpm add @bjornpagen/bumbledb effect
```

The npm package ships native addons for darwin-arm64, linux-arm64 and
linux-x64, and the matching platform package installs automatically.

## Development

`scripts/ci.sh <lane>` is the body of every CI job, so a local run matches CI:

```sh
scripts/ci.sh lint    # fmt, clippy, rustdoc, TypeScript lint and typecheck
scripts/ci.sh test    # nextest over the workspace, doctests
scripts/ci.sh addon   # native addon, TypeScript tests, packed-package smoke
```

**Rules for contributors:**
- **Toolchain:** Rust nightly, kept current by `scripts/bump-toolchain.sh` and a weekly canary.
- **Tests:** deterministic. Timings are reports, never test gates.
- **Design:** every change follows the [representation-first design principles](docs/design/representation-first.md). Change the data so a special case disappears, rather than adding a branch to handle it.

The [static Linux ARM64 guide](docs/static-linux-arm64.md) covers the fully static musl build.

## Repository

- `crates/bumbledb`: the engine.
- `crates/bumbledb-theory`: values, intervals, Allen masks and the schema checker.
- `crates/bumbledb-macros`: `schema!`, `query!` and `params!`.
- `crates/bumbledb-log`: the hosted log's sans-IO core and its LMDB cache.
- `crates/bumbledb-node`: the Node bridge.
- `crates/bumbledb-bench`: independent oracles (a naive evaluator and SQLite differential tests) and benchmarks.
- `ts/`: the TypeScript SDK and the `bumbledb` CLI.
- `examples/`: consumers and the Notes application.
- `proposals/`: research proposals.

## License

[0BSD](LICENSE).
