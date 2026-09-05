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
ordinary `bigint`, `number` (for `f64`), `string`, boolean, byte, `Id128`,
and interval values; queries infer their parameter and result types from how
those values are used.

## Platform support

The TypeScript package ships native binaries for **darwin-arm64**
(macOS on Apple Silicon), **linux-arm64**, and **linux-x64**.
The matching `@bjornpagen/bumbledb-<platform>` package is selected
automatically during installation. On another platform, importing the
package returns an error that identifies the running platform and the
available binaries.

## Install

```sh
pnpm add @bjornpagen/bumbledb effect
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
	id128,
	Id128,
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
// application-owned Id128 values — the database issues no identity.
const Student = relation("Student", { id: id128, name: str, budget: u64 })
const Attempt = relation("Attempt", {
	id: id128,
	student: id128,
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
		const db = yield* Db.open(localPath, Learning, work)
		const studentId = yield* Id128.random()
		const attemptId = yield* Id128.random()

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
		const rows = yield* result.collect({ maxBytes: work.resultBytes })
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

- Fields use `bool`, `bytes`, `f64`, `i64`, `id128`, `u64`, `str`, and
  `interval` (`interval(f64)` is the dense float interval); `span` builds
  checked interval values. `relation()` declares stored records, while
  `closed()` declares a fixed enum-like set whose values may carry typed
  columns. `Infer` exposes the resulting TypeScript value type. `Id128` is
  the ordinary application-owned 128-bit identity: 32 lowercase hex,
  generated with the effectful `Id128.random()`, parsed with the pure
  `Id128.fromHex` returning `Result`.
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
  `collect({ maxBytes })` is capped materialization and whose
  `pages({ pageBytes })` is a one-shot consuming `Stream` of owned page
  arrays after complete evaluation.
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

## Cookbook

Successor modeling recipes are translated to the TypeScript API in
[COOKBOOK.md](./COOKBOOK.md). `test/cookbook-doc.test.ts` extracts and
type-checks the document's TypeScript examples.

## Architecture

The SDK is a typed interface to the native engine. Storage, transactions,
queries, constraints, performance results, and the Rust implementation are
documented in the
[Bumbledb repository](https://github.com/bjornpagen/bumbledb).

## License

0BSD
