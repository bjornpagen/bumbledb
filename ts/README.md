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

```sh
pnpm add @bjornpagen/bumbledb@1.0.1 effect@4.0.0-rc.112
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
import type { ExecutionPolicy, NativeRuntimeOptions } from "@bjornpagen/bumbledb"

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
const Learning = schema("Learning", { Student, Attempt }, [
	key(Student, ["id"]),
	key(Attempt, ["id"]),
	contained(on(Attempt, "student"), on(Student, "id")),
	capacity(on(Student, "id"), {
		from: on(Attempt, "student"),
		weight: weigh("units"),
		within: within(0n, ref("budget"))
	})
])

// Measured policies are inputs — the library invents no limits.
declare const runtimePolicy: NativeRuntimeOptions
declare const work: ExecutionPolicy
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
		const db = yield* Db.create(localPath, Learning, work)
		const studentId = yield* Effect.sync(() => crypto.randomUUID())
		const attemptId = yield* Effect.sync(() => crypto.randomUUID())

		const draft = yield* ChangeSet.builder(Learning, work)
		yield* draft.insert(Student, [{ id: studentId, name: "Ada", budget: 10n }])
		yield* draft.insert(Attempt, [
			{ id: attemptId, student: studentId, score: 0.9, units: 1n, active: { start: 0n, end: 60n } }
		])
		const changes = yield* draft.finish()

		const outcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
		if (outcome.kind !== "accepted" && outcome.kind !== "no-change") {
			return outcome
		}
		const snapshot = yield* db.snapshot(work)
		const found = yield* snapshot.get(Student, { id: studentId }, work)
		if (Option.isNone(found)) {
			return outcome
		}
		const result = yield* snapshot.execute(attemptsFor, { student: studentId }, work)
		const rows = yield* result.collect({ maxBytes: work.resultBytes }, work)
		return { outcome, rows }
	})
)

// One boundary for this script; an Effect app supplies the layer in its
// own application graph instead.
void Effect.runPromise(program.pipe(Effect.provide(NativeRuntime.layer(runtimePolicy))))
```

Every `ts` fence in this README is extracted and type-checked against the
real surface by `test/readme.test.ts` — the examples cannot drift.

## Surface

The SDK translates TypeScript values directly into the engine's shared schema
and query representations.

- Fields use `bool`, `bytes`, `f64`, `i64`, `uuid`, `u64`, `str`, and
  `interval` (`interval(f64)` is the dense float interval); `span` builds
  checked interval values. `relation()` declares stored records, while
  `closed()` declares a fixed enum-like set whose values may carry typed
  columns. `Infer` exposes the resulting TypeScript value type. `Uuid` is
  a structural template-literal string, not a nominal brand or a cast helper.
  It represents all 128-bit UUID payloads in canonical text,
  generated with the effectful `Effect.sync(() => crypto.randomUUID())`, parsed with the pure
  `Uuid.parse` returning `Result`.
- `schema()` accepts `key`, `contained`, `mirrors`, and `capacity`
  statements. Keys are declared statements — there is no minted identity.
  `capacity(target, { from, weight?, within })` takes named options;
  `.where` makes a reference conditional, `within` sets a count or
  measurement range, and `weigh` chooses a numeric field or interval
  duration. Harmless equivalent window spellings lower to one canonical
  law; genuinely different meanings still refuse.
- `NativeRuntime.layer(options)` owns the one bounded native runtime;
  provide it once in the app graph. `Db.create` and `Db.open` are scoped
  Effects over that runtime; `open` never creates and `create` refuses
  existing authority. `db.apply(changes, { ...work, expected })` judges one
  immutable final-state change: `accepted`, `no-change`,
  `invariant-rejected` (complete statement diagnostics), or `moved`.
- `ChangeSet.builder(schema, work)` acquires a scoped database-free draft;
  `insert`/`delete` are lazy bounded ingestion effects, `finish()` seals the
  immutable `ChangeSet`. Snapshots satisfy the shared `QueryReader`: typed
  `get` returns `Option`, `execute` returns a sealed `CompleteResult` whose
  `collect({ maxBytes }, work)` is capped materialization and whose
  `pages({ pageBytes }, work)` is a one-shot consuming `Stream` of owned page
  arrays after complete evaluation. Delivery work is fresh: it does not
  inherit the snapshot or execution deadline.
- `query(S).rule(...)` builds typed queries. Reusing a variable created by
  `v(R)` joins records through that value. The builder supports named result
  rows, typed parameters, negation, comparisons, boolean conditions, set
  parameters, interval operations, exact aggregates (`sum`/`mean` over
  `f64` are deterministic with one final rounding), named intermediate
  results, nonrecursive composition of query templates, and linear
  recursive reachability.
- `Scalar.field("units")` is an unresolved source-field leaf.
  `Scalar.add(Scalar.field("units"), Scalar.u64(1n))` authors synchronously
  without native loading. Native schema binding typechecks it, including
  on zero rows. There is no `field<T>` assertion and no JS evaluator.
- Operational failure is the one `DbError` tagged-reason class in the
  Effect error channel; interruption and finalizer problems stay in
  `Cause`. Resource owners are scoped and report honest `CloseReport`s;
  incomplete teardown surfaces as a structured `CloseFailure` defect.

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
