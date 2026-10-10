# bumbledb

An embedded relational database for Rust and TypeScript. Schemas and queries
are typed values in your code, not SQL strings. Relations are sets, joins run
on Free Join over in-memory column images, and storage is LMDB. Every write is
judged against the declared laws of its final state before it commits.

For serverless and multi-process deployments, a `Database` keeps the data as an
immutable log of commands in an object store (S3 Express for the log, S3
Standard for checkpoints) and reads it through a disposable local cache.
Migrations ship with the application code and are recorded in the log.

[Release notes](docs/release-2.0.md)
· [TypeScript guide](ts/README.md)
· [Cookbook](docs/cookbook.md)
· [Benchmarks](docs/perf/results.md)

## The model

- **Relations are sets.** Inserting a fact that exists and deleting one that
  does not are no-ops. There are no nulls: an optional attribute is an absent
  fact in a child relation.
- **Laws are statements.** A schema declares keys (`R(x) -> R`), containments
  (`S(y) <= T(x)`, with selections and `==` for both directions), pointwise
  interval keys, and capacity windows (`T(x) <=[w]{lo..hi} S(y)`). Closed
  relations are fixed vocabularies whose rows are part of the schema.
- **Writes are judged once.** A write is a net change over the committed
  state. The engine judges the proposed final state, so a delete and an
  insert in one write never pass through an invalid intermediate state. A
  rejection names every violated statement and cites the offending facts.
- **Queries are data.** Rules join atoms, filter with comparisons and Allen
  interval masks, negate, aggregate exactly, compose named stages, and
  recurse linearly. Prepared queries reuse their buffers across executions.
- **Values are exact.** Integer arithmetic is checked. An `f64` value has one
  NaN and one zero, so set identity is well defined; query comparisons follow
  IEEE order, where NaN orders against nothing. `Sum` and `Avg` accumulate
  exactly and round once. Intervals are half-open and never empty.

Identity is application-owned: the database issues no ids. Use `uuid` fields
or any declared key.

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
linux-x64; the matching platform package installs automatically.

## Rust

A schema, one write, a rejected write, a typed query and a keyed read:

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

`schema!` checks the declaration at compile time: a duplicate field, a key
arrow naming another relation, or a containment whose target matches no
declared key is a compile error at the offending token. `query!` resolves
relations, fields and closed handles through the theory's generated constants.

## TypeScript

The TypeScript SDK is Effect-native: every operation is a lazy, scoped
`Effect`, and schemas and queries are validated by the engine when they are
defined. A hosted database over a local directory store:

```ts
import { Effect } from "effect"
import { Bumble, ChangeSet, Database, FsStore, key, type Migrations, query, relation, schema, str, uuid, v } from "@bjornpagen/bumbledb"

const Note = relation("Note", { id: uuid, text: str })
const App = schema("App", { Note }, [key(Note, ["id"])])
const notes = query(App).rule((r) => {
	const { id, text } = v(Note)
	return r.match(Note, { id, text }).find({ id, text })
})

declare const migrations: Migrations // generated by `bumbledb generate`

const program = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Database.make({
			schema: App,
			migrations,
			store: FsStore.make("data/log"),
			cache: { directory: "data/cache" },
			onOpen: "migrate"
		})
		const draft = yield* ChangeSet.builder(App)
		yield* draft.insert(Note, [{ id: yield* Effect.sync(() => crypto.randomUUID()), text: "hello" }])
		yield* db.submit(yield* draft.finish(), { requestId: Database.requestId() })
		const reader = yield* db.read("latest")
		return yield* (yield* reader.execute(notes, {})).collect()
	})
)

void Effect.runPromise(program.pipe(Effect.provide(Bumble.layer())))
```

Swap `FsStore` for `S3Store` (through the application's own `S3Client`) to run
the same code on AWS Lambda; see the [TypeScript guide](ts/README.md) and the
[Notes example](examples/notes/README.md), a server-side Next.js application
with per-tenant databases, migrations and an outbox.

## Hosted durability

A hosted database is the create-only sequence of objects `log/{seq}`. Each
object is one entry that decides a batch of commands, or a step of a
migration, and carries the decided change set, so catching up applies effects
without judging them again. Writers race with `If-None-Match: *`; a writer
resolves an ambiguous create by reading the object back and comparing bytes,
so retries are safe, and every refusal except `Unknown` proves the command is
not in the log. Automatic
checkpoint images bound the replay on a cold open. The log core is a sans-IO
state machine in Rust (`crates/bumbledb-log`); the TypeScript package performs
its object-store requests.

## Performance

The last full benchmark suite ran on the 1.3.0 engine (Apple M2 Max,
2026-09-11): a 0.46 µs median point lookup, a 4.96 µs range query, and lower
medians than indexed SQLite in all 32 read families. 2.0 has not been
re-measured yet. See the [full results and their limits](docs/perf/results.md).

![Read latency against indexed SQLite](assets/bench-vs-sqlite.svg)

The hardware target is a Raspberry Pi Zero 2 with 512 MB of RAM; the query
image cache is capped (128 MiB by default, `Options::image_cache_bytes`).

## Development

`scripts/ci.sh <lane>` is the body of every CI job, so a local run matches CI:

```sh
scripts/ci.sh lint    # fmt, clippy, rustdoc, TypeScript lint and typecheck
scripts/ci.sh test    # nextest over the workspace, doctests
scripts/ci.sh addon   # native addon, TypeScript tests, packed-package smoke
```

The [static Linux ARM64 guide](docs/static-linux-arm64.md) covers the fully
static musl build.

## Repository

- `crates/bumbledb`: the engine.
- `crates/bumbledb-theory`: values, intervals, Allen masks and the schema checker.
- `crates/bumbledb-macros`: `schema!`, `query!` and `params!`.
- `crates/bumbledb-log`: the hosted log's sans-IO core and LMDB cache.
- `crates/bumbledb-node`: the Node bridge.
- `crates/bumbledb-bench`: independent oracles and benchmarks.
- `ts/`: the TypeScript SDK; `examples/`: consumers and the Notes application.

## License

[0BSD](LICENSE).
