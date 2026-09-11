# @bjornpagen/bumbledb

This package is the TypeScript interface to the
[Bumbledb](https://github.com/bjornpagen/bumbledb) embedded relational
database. Schemas and queries are typed TypeScript values rather than SQL
strings, while storage, admission, transactions, and query execution run in
the native engine.

The API is **Effect-native**: every database operation constructs a lazy
[`Effect`](https://effect.website) and every native resource is scoped.
There is no Promise, synchronous, or disposal twin. Pure schema/query/scalar
construction and reads of already-owned metadata stay ordinary synchronous
expressions with no hidden native work. The package requires Effect
`4.0.0-rc.112` exactly, as a peer dependency.

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
automatically during installation. Schema and query authoring do not load
the addon. Attempting native database work on an unsupported platform fails
with a diagnostic identifying the running platform and available binaries.
Node 24 or newer is required; Edge and browser runtimes are unsupported.
Linux artifacts target Amazon Linux 2023 / glibc 2.34 or newer. Their
correctness CI is distinct from the Apple Silicon performance measurements.

## Install

This guide follows the 1.3.1 source API. Use the guide from the Git tag matching
your installed package. GitHub release tarballs and npm publication are separate;
the npm command below applies once that version is published.

```sh
pnpm add @bjornpagen/bumbledb@1.3.1 effect@4.0.0-rc.112
```

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
	Db,
	f64,
	i64,
	uuid,
	interval,
	key,
	NativeRuntime,
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

		const outcome = yield* db.apply(changes, { expected: { kind: "any" } })
		if (outcome.kind !== "accepted" && outcome.kind !== "no-change") {
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
void Effect.runPromise(program.pipe(Effect.provide(NativeRuntime.layer())))
```

Every `ts` fence in this README is extracted and type-checked against the
real surface by `test/readme.test.ts` — the examples cannot drift.

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
- `NativeRuntime.layer()` owns the shared native runtime with sensible defaults;
  provide it once in the app graph. `Db.create` and `Db.open` are scoped
  Effects over that runtime; `open` never creates and `create` refuses
  existing authority. `db.apply(changes, { expected })` judges one
  immutable final-state change: `accepted`, `no-change`,
  `invariant-rejected` (complete statement diagnostics), or `moved`.
  `db.judge(changes, { expected })` uses the same admission path but aborts
  the private candidate. It returns `admitted` or `invariant-rejected` with
  the actual base witness and net additions/removals, or `moved` without
  judging. Both use `WriteOptions`; judgment never guarantees a later apply.
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
- `describeQuery(q)` returns owned logical IR, result names, intermediate
  table names, and parameter names. `queryFromDescription(S, description,
  resultFields)` checks a generated description through the same rule builder,
  scalar grammar, lowering, and execution paths. The result-field record is
  checked against the derived head and infers the returned row type; it is
  not an unchecked cast. Generated parameters are validated by their uses at
  execution. Ordinals refer to the supplied schema's ordered declarations;
  variable ordinals are local to each rule. Native preparation still checks
  engine semantics. The description contains plain values, including bigint
  and byte arrays, rather than native resources or a JSON encoding.
  Query `data` and `schema`, and compiled-schema inspection properties,
  return detached snapshots: editing their byte buffers cannot change the
  query or compilation. Checked immutable branches may be shared. Structural
  declarations with byte payloads are copied when consumed; JavaScript byte
  buffers themselves are mutable and are never treated as immutable cache keys.
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
