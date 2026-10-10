# @bjornpagen/bumbledb

This package is the TypeScript interface to the
[Bumbledb](https://github.com/bjornpagen/bumbledb) embedded relational
database. Schemas and queries are typed TypeScript values rather than SQL
strings, while storage, admission, transactions, and query execution run in
the native engine.

The API is **Effect-native**: every database operation constructs a lazy
[`Effect`](https://effect.website) and every native resource is scoped.
There is no Promise, synchronous, or disposal twin. Schema, query and scalar
construction stay ordinary synchronous expressions; a schema or query is
validated by the engine when it is defined. The package requires Effect
`^4.0.2` as a peer dependency.

Relation declarations describe their fields, and the statements passed to
`schema()` connect those fields into typed keys and references. Values remain
ordinary `bigint`, `number` (for `f64`), `string`, boolean, byte, `Uuid`,
and interval values; queries infer their parameter and result types from how
those values are used.

Relations, closed rosters, keys, selections, and constraints are checked
structural descriptions. Schema construction owns copies of declarations.
Independently constructed equivalent declarations work in queries, codecs,
writes, and key lookups; ordered fields and enum handles must agree.

Individual fields expose the same validation and JSON codecs as rows:
`fieldSchema(field)` returns a host-value Effect Schema,
`decodeBoundaryField(field, input)` decodes strict JSON-boundary data, and
`encodeBoundaryField(field, value)` encodes it. Both boundary operations return
`Result`; their value type is `Infer<typeof field>`. Compose field schemas into
ordinary Effect Schema records for application inputs.

## Platform support

The TypeScript package ships native binaries for **darwin-arm64**
(macOS on Apple Silicon), **linux-arm64**, and **linux-x64**.
The matching `@bjornpagen/bumbledb-<platform>` package is selected
automatically during installation. Defining a schema or a query loads the
addon and validates the definition in the engine, so an invalid definition
throws `AuthoringError` where it is written. On an unsupported platform the
first definition fails with a diagnostic naming the running platform and the
available binaries. Node 26 or newer is required; Edge and browser runtimes
are unsupported.
Linux artifacts target Amazon Linux 2023 / glibc 2.34 or newer. Their
correctness CI is distinct from the Apple Silicon performance measurements.

## Install

```sh
pnpm add @bjornpagen/bumbledb effect
```

`@aws-sdk/client-s3` is an optional peer dependency, needed only for `S3Store`.

The package has two entry points and one binary. `@bjornpagen/bumbledb`
holds authoring (relations, schemas, queries, change sets), the `Bumble`
runtime layer and the durable `Database`. `@bjornpagen/bumbledb/engine` is
the embedded `Db` for single-process use, tests and benchmarks. The
`bumbledb` binary generates, checks and applies migrations.

## Quick start

Declare relations, connect their fields with keys and references, build a
change set, apply it, and query the admitted state. Parameters and result
rows are inferred; a failed constraint check is a typed apply outcome, never
a thrown exception.

```ts
import { Effect, Option } from "effect"
import {
	capacity,
	ChangeSet,
	contained,
	f64,
	i64,
	uuid,
	interval,
	key,
	Bumble,
	on,
	query,
	ref,
	relation,
	schema,
	str,
	u64,
	v,
	weigh,
	within
} from "@bjornpagen/bumbledb"
import { Db } from "@bjornpagen/bumbledb/engine"

// Relations describe stored records. Identity fields are ordinary
// application-owned Uuid values — the database issues no identity.
const Student = relation("Student", { id: uuid, name: str, budget: u64 })
const Attempt = relation("Attempt", {
	id: uuid,
	student: uuid,
	score: f64,
	units: u64,
	active: interval(i64)
})

// Keys are declared statements; references and capacity are laws.
const StudentById = key(Student, ["id"])
const Learning = schema("Learning", { Student, Attempt }, [
	StudentById,
	key(Attempt, ["id"]),
	contained(on(Attempt, "student"), on(Student, "id")),
	capacity(on(Student, "id"), {
		from: on(Attempt, "student"),
		weight: weigh("units"),
		within: within(0n, ref("budget"))
	})
])

declare const localPath: string

// Queries are reusable typed values: reusing a v(R) variable is the join,
// the find record names the answer columns, and params are typed by use.
const attemptsFor = query(Learning).rule((r) => {
	const { id, student, score, units, active } = v(Attempt)
	return r
		.match(Attempt, { id, student, score, units, active })
		.where(r.eq(student, r.param("student")))
		.find({ id, score })
})

const program = Effect.scoped(
	Effect.gen(function* () {
		// First use: create explicitly. Use Db.open for an existing store.
		const db = yield* Db.create(localPath, Learning)
		const studentId = yield* Effect.sync(() => crypto.randomUUID())
		const attemptId = yield* Effect.sync(() => crypto.randomUUID())

		const draft = yield* ChangeSet.builder(Learning)
		yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 10n }])
		yield* draft.insert(Attempt, [
			{ id: attemptId, student: studentId, score: 0.9, units: 1n, active: { start: 0n, end: 60n } }
		])
		const changes = yield* draft.finish()

		const outcome = yield* db.apply(changes)
		if (outcome._tag !== "Committed") {
			return outcome
		}
		const snapshot = yield* db.snapshot()
		const found = yield* snapshot.get(StudentById, { id: studentId })
		if (Option.isNone(found)) {
			return outcome
		}
		const result = yield* snapshot.execute(attemptsFor, { student: studentId })
		const rows = yield* result.collect()
		return { outcome, rows }
	})
)

// One boundary for this script; an Effect app supplies the layer in its
// own application graph instead.
void Effect.runPromise(program.pipe(Effect.provide(Bumble.layer())))
```

Every `ts` fence in this README is extracted and type-checked against the
real surface by `test/readme.test.ts` — the examples cannot drift.

## Durable databases

`Database` keeps a database as an immutable log in an object store and reads
it through a local cache that is only a cache. The store is `S3Store` (the log
in an S3 Express directory bucket, checkpoints in a Standard bucket, through the
app's `S3Client`), `FsStore` (a durable local directory) or `MemStore` (tests).
A cold open fetches up to 32 log objects at once, and every new connection
resolves DNS on one of libuv's 4 threads. Where lookups are slow, start Node
with `UV_THREADPOOL_SIZE=64` or give the `S3Client` an agent that caches
lookups.

Migrations are bundled with the code. `bumbledb generate --schema
src/schema.ts#App --name <name>` writes `migrations/NNNN_<name>/` and
`migrations/index.ts`; add a `populate` step to a generated `migration.ts`
when rows must be computed. Relations that keep their name and fields are
copied unchanged. `bumbledb check --schema src/schema.ts#App` fails when the
schema has an ungenerated change or a migration was edited.
`onOpen: "migrate"` creates the database and runs pending migrations;
`onOpen: "verify"` refuses to open while any is pending, so production runs
`bumbledb migrate --config <file>` from the deploy pipeline first.

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

`submit` returns `Decided` with a receipt (`Committed`, `NoChange`,
`PreconditionFailed` or `InvariantRejected`) or `Refused` with the reason;
every refusal but `Unknown` proves the command is not in the log. A submit
with `precondition: reader.revision` is decided only if nothing committed after
that read. `read` takes `"cached"`, `"latest"` or `{ atLeast: seq }`.
`Database.layer(Database.tag<typeof App>("App/Database"), options)` provides
one database to an app graph, and `Database.pool({ ...options, store:
(tenant) => ..., cache: (tenant) => ... })` opens one per tenant on demand and
closes it when idle.

## Surface

The SDK translates TypeScript values directly into the engine's shared schema
and query representations.

- Fields use `bool`, `bytes`, `f64`, `i64`, `uuid`, `u64`, `str`, and
  `interval` (`interval(f64)` is the dense float interval). Interval values
  are plain `{ start, end }` records checked against their field. `relation()` declares stored records, while
  `closed()` declares a fixed enum-like set whose values may carry typed
  columns. `Infer` exposes the resulting TypeScript value type. `Uuid` is
  a structural template-literal string, not a nominal brand or a cast helper.
  It represents all 128-bit UUID payloads in canonical text,
  generated with the effectful `Effect.sync(() => crypto.randomUUID())`, parsed with the pure
  `Uuid.parse` returning `Result`.
- `schema()` accepts `key`, `contained`, `mirrors`, and `capacity`
  statements. Keys are declared statements — there is no minted identity.
  `capacity(target, { from, weight?, within })` takes named options;
  `select(relation, { field: value })` makes a reference conditional,
  `closedId(roster)` supplies a field that accepts named enum handles,
  `within` sets a count or
  measurement range, and `weigh` chooses a numeric field or interval
  duration. Harmless equivalent window spellings lower to one canonical
  law; genuinely different meanings still refuse.
- `Bumble.layer()` owns the shared native runtime with sensible defaults;
  provide it once in the app graph. `Db.create` and `Db.open` are scoped
  Effects over that runtime; `open` never creates and `create` refuses
  existing authority. `db.apply(changes, expected?)` judges one immutable
  final-state change: `Committed` (with the generation, and `changed: false`
  when the state already held it), `Rejected` (complete statement
  diagnostics), or `Moved` when `expected`, a snapshot's opaque `witness`, is
  no longer current. `db.judge(changes, expected?)` uses the same admission
  path and stores nothing: `Admitted` with net additions and removals,
  `Rejected`, or `Moved` without judging. A judgment never guarantees a later
  apply.
- `ChangeSet.builder(schema)` acquires a scoped database-free draft;
  `insert`/`delete` are lazy bounded ingestion effects, `finish()` seals the
  immutable `ChangeSet`. It exposes `schemaId`, `counts`, and `byteLength`.
  `records()` is a reusable, bounded stream of relation-name-discriminated
  facts; each traversal owns an independent scoped cursor. `toBytes()`
  explicitly materializes canonical bytes; `ChangeSet.fromBytes(schema, bytes)`
  checks them without normalizing malformed input. `left.compose(right)`
  merges native records into one add-wins command without decoding rows.
  Composition is commutative, associative, and idempotent, not sequential
  replay. Its counts describe requested distinct actions; only `judge`
  measures their effect against a store.
- Snapshots satisfy the shared `QueryReader`: typed
  `get(keyDescriptor, keyValues)` chooses an explicit declared key and returns
  `Option`; `execute` returns a sealed `CompleteResult` whose
  `collect()` explicitly materializes all rows and whose
  `pages()` is a one-shot consuming `Stream` of owned page
  arrays after complete evaluation. Each page contains up to 256 rows; this
  bounds delivery rather than streaming execution. Effect scope closes the
  cursor on completion, failure, or interruption.
- `query(S).rule(...)` builds typed queries. Reusing a variable created by
  `v(R)` joins records through that value. The builder supports named result
  rows, typed parameters, negation, comparisons, boolean conditions, set
  parameters, interval operations, exact aggregates (`sum`/`mean` over
  `f64` are deterministic with one final rounding), named intermediate
  results, nonrecursive composition of query templates, and linear
  recursive reachability.
- Operational failure is the one `DbError` tagged-reason class in the
  Effect error channel; interruption and finalizer problems stay in
  `Cause`. Resource owners are scoped and report honest `CloseReport`s;
  incomplete teardown surfaces as a structured `CloseFailure` defect.
  `error.message` contains the operation and reason code for safe default
  display. Inspect `error.reason` explicitly for engine details, or
  `InvalidArgument.detail` for authoring text and available structured
  diagnostics such as `MissingField`, `UnknownField`, and `InvalidValue`.
  Those details can contain application names or values and should not be
  copied into public logs indiscriminately.

## Cookbook

Modeling recipes are translated to the TypeScript API in
[COOKBOOK.md](./COOKBOOK.md). `test/cookbook-doc.test.ts` extracts and
type-checks the document's TypeScript examples.

## Architecture

The SDK is a typed interface to the native engine. Storage, transactions,
queries, constraints, performance results, and the Rust implementation are
documented in the
[Bumbledb repository](https://github.com/bjornpagen/bumbledb).

## License

0BSD
