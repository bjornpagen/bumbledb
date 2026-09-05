# The Bumbledb TypeScript cookbook

Worked recipes for the successor Effect-native surface: typed schema values,
declared keys and laws, application-owned `Id128` identity, one bounded
native runtime, scoped resources, immutable final-state changes, `Option`
reads, sealed complete results, and one-shot page streams.

Every `ts` fence below is extracted and type-checked against the real
package surface by `test/cookbook-doc.test.ts`; the imports fence here is
prepended to every recipe. The examples are lazy Effect programs — nothing
below runs a database at import time, and measured policies are inputs.

```ts
import { Effect, Option, Result, Stream } from "effect"
import {
	capacity,
	ChangeSet,
	closed,
	contained,
	Db,
	duration,
	f64,
	i64,
	id128,
	Id128,
	interval,
	key,
	mirrors,
	NativeRuntime,
	on,
	query,
	ref,
	relation,
	Scalar,
	schema,
	span,
	str,
	u64,
	v,
	weigh,
	within
} from "@bjornpagen/bumbledb"
import type {
	ApplyOutcome,
	CompleteResult,
	ExecutionPolicy,
	Fact,
	NativeRuntimeOptions,
	QueryReader
} from "@bjornpagen/bumbledb"

declare const work: ExecutionPolicy
declare const runtimePolicy: NativeRuntimeOptions
declare const localPath: string
```

## 1. One schema, typed twice — relations, declared keys, laws

Identity fields are ordinary application-owned `Id128` values: the database
issues no identity and there is no `fresh` mint. Keys are declared
statements; references and capacity are laws over the same fields. The same
declarations, spelled in Rust's `schema!`, produce the same canonical schema
identity.

```ts
const Student = relation("Student", { id: id128, name: str, budget: u64 })
const Attempt = relation("Attempt", {
	id: id128,
	student: id128,
	score: f64,
	units: u64,
	active: interval(i64)
})

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

// Schema construction is pure metadata: the value is inert data, and typing
// follows it — a Fact<typeof Attempt> is the inferred row object.
declare const row: Fact<typeof Attempt>
const scoreIsNumber: number = row.score
const unitsAreExact: bigint = row.units
void [Learning, scoreIsNumber, unitsAreExact]
```

## 2. One runtime layer; explicit create and open

`NativeRuntime.layer(options)` acquires the single bounded native runtime
with scope; reuse ONE layer value so Effect's memoization shares it.
`Db.open` never creates a missing database; `Db.create` refuses existing
authority. Both are scoped acquisitions.

```ts
const Doc = relation("Doc", { id: id128, text: str })
const Docs = schema("Docs", { Doc }, [key(Doc, ["id"])])

const openExisting = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.open(localPath, Docs, work)
		return db.schemaId
	})
)

const createOnce = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.create(localPath, Docs, work)
		return db.schemaId
	})
)

// One boundary; an Effect app provides the layer in its own graph instead.
const layer = NativeRuntime.layer(runtimePolicy)
void [openExisting.pipe(Effect.provide(layer)), createOnce.pipe(Effect.provide(layer))]
```

## 3. Changes: build lazily, normalize once, apply atomically

`ChangeSet.builder` acquires a scoped database-free draft. Ingestion effects
are lazy and re-runnable while the draft is building — each execution reads
the then-current iterable and charges work again. Within ONE change set the
normalization is `(add, remove ∖ add)`: the identical fact's add wins
independent of call order. `finish()` consumes the draft into an immutable,
reusable `ChangeSet`.

```ts
const Task = relation("Task", { id: id128, title: str, done: u64 })
const Tasks = schema("Tasks", { Task }, [key(Task, ["id"])])

const applyOnce = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.open(localPath, Tasks, work)
		const taskId = yield* Id128.random()
		const draft = yield* ChangeSet.builder(Tasks, work)
		yield* draft.insert(Task, [{ id: taskId, title: "write the cookbook", done: 0n }])
		const changes = yield* draft.finish()
		const outcome: ApplyOutcome = yield* db.apply(changes, { ...work, expected: { kind: "any" } })
		switch (outcome.kind) {
			case "accepted":
			case "no-change":
				return outcome.witness
			case "invariant-rejected":
				// Complete statement diagnostics, typed data — never a throw.
				return yield* Effect.fail(outcome.violations)
			case "moved":
				return outcome.current
		}
	})
)
void applyOnce
```

## 4. Keyed reads are Options

`get` reads through the relation's primary declared key. A missing key is
`Option.none` — never a fake I/O error, never a nullable row.

```ts
const Person = relation("Person", { id: id128, name: str })
const People = schema("People", { Person }, [key(Person, ["id"])])

const lookup = (reader: QueryReader<typeof People>, personId: Id128) =>
	Effect.gen(function* () {
		const found = yield* reader.get(Person, { id: personId }, work)
		return Option.isSome(found) ? found.value.name : "unknown"
	})
void lookup
```

## 5. Queries are reusable typed values

`v(R)` mints typed variables; reusing one across `match` records is the
join. The `find` record names the answer columns, and parameters infer from
use. A template contains no tenant rows or live snapshot — execute it
against any matching-schema reader.

```ts
const Author = relation("Author", { id: id128, name: str })
const Book = relation("Book", { id: id128, author: id128, pages: u64 })
const Library = schema("Library", { Author, Book }, [
	key(Author, ["id"]),
	key(Book, ["id"]),
	contained(on(Book, "author"), on(Author, "id"))
])

const booksBy = query(Library).rule((r) => {
	const { id, author, pages } = v(Book)
	const { name } = v(Author)
	return r
		.match(Book, { id, author, pages })
		.match(Author, { id: author, name })
		.where(r.eq(name, r.param("name")))
		.find({ id, pages })
})

const readBooks = (reader: QueryReader<typeof Library>, name: string) =>
	Effect.scoped(
		Effect.gen(function* () {
			const result = yield* reader.execute(booksBy, { name }, work)
			return yield* result.collect({ maxBytes: work.resultBytes }, work)
		})
	)
void readBooks
```

## 6. Grouped exact aggregates

Aggregates fold the group's distinct full bindings: keep the identity-bearing
`id` in the binding set so equal scores in different rows both contribute.
Exact float `sum`/`mean` are deterministic with one final rounding.

```ts
const Sample = relation("Sample", { id: id128, series: id128, value: f64 })
const Series = schema("Series", { Sample }, [key(Sample, ["id"])])

const stats = query(Series).rule((r) => {
	const { id, series, value } = v(Sample)
	return r.match(Sample, { id, series, value }).find({
		series,
		total: r.sum(value),
		mean: r.mean(value),
		points: r.count()
	})
})
void stats
```

## 7. Nonrecursive composition — a template is a relation expression

A typed query template of the same schema splices as a derived stage:
`v(imported)` mints variables for its head columns and `match(imported, …)`
joins them. Naming materializes nothing.

```ts
const Reading = relation("Reading", { id: id128, sensor: id128, value: f64 })
const Sensor = relation("Sensor", { id: id128, label: str })
const Telemetry = schema("Telemetry", { Reading, Sensor }, [
	key(Reading, ["id"]),
	key(Sensor, ["id"]),
	contained(on(Reading, "sensor"), on(Sensor, "id"))
])

const perSensor = query(Telemetry).rule((r) => {
	const { id, sensor, value } = v(Reading)
	return r.match(Reading, { id, sensor, value }).find({ sensor, mean: r.mean(value) })
})

const labeled = query(Telemetry).rule((r) => {
	const { sensor, mean } = v(perSensor)
	const { label } = v(Sensor)
	return r
		.match(perSensor, { sensor, mean })
		.match(Sensor, { id: sensor, label })
		.find({ label, mean })
})
void labeled
```

## 8. Bounded results: capped collect, one-shot page stream

`collect({ maxBytes }, work)` is database-enforced total materialization and
leaves the result available. `pages({ pageBytes }, work)` is a ONE-SHOT consuming
stream over the completed result: the first run moves the backing into a
private scoped cursor; a second run refuses. Every element is one owned page
array — pages, not rows — delivered after complete evaluation.

```ts
const Event = relation("Event", { id: id128, at: i64 })
const Feed = schema("Feed", { Event }, [key(Event, ["id"])])

const everything = query(Feed).rule((r) => {
	const { id, at } = v(Event)
	return r.match(Event, { id, at }).find({ id, at })
})

const drain = (result: CompleteResult<{ readonly id: Id128; readonly at: bigint }>) =>
	result.pages({ pageBytes: 65536n }, work).pipe(
		Stream.runForEach((page) =>
			Effect.sync(() => {
				// One owned page array; caller mutation cannot reach native
				// state or another delivered page.
				return page.length
			})
		)
	)
void [everything, drain]
```

## 9. Application identity: generate once, retain, never regenerate

`Id128.random()` is effectful cryptographic entropy — run it once for an
original intent, persist the value with the request, and never regenerate
inside a retry. `Id128.fromHex` is the pure fixed-size parser returning
`Result`.

```ts
const parsed = Id128.fromHex("00112233445566778899aabbccddeeff")
const okOrRefused: boolean = Result.isSuccess(parsed)

const mintOnce = Effect.gen(function* () {
	const id = yield* Id128.random()
	// Persist `id` with the original request BEFORE any database dispatch;
	// a timeout retries the identical intent, never a new identity.
	return id
})
void [okOrRefused, mintOnce]
```

## 10. Witnessed correction: exact expected state

Read under a short scope, keep the copied witness, and apply with
`expected: { kind: "exact", at: witness }`. An intervening net change moves
the apply instead of silently overwriting.

```ts
const Account = relation("Account", { id: id128, balance: i64 })
const Bank = schema("Bank", { Account }, [key(Account, ["id"])])

const correct = (accountId: Id128) =>
	Effect.scoped(
		Effect.gen(function* () {
			const db = yield* Db.open(localPath, Bank, work)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* db.snapshot(work)
					const previous = yield* snapshot.get(Account, { id: accountId }, work)
					if (Option.isNone(previous)) {
						return yield* Effect.fail({ missing: accountId })
					}
					return { previous: previous.value, at: snapshot.witness }
				})
			)
			const draft = yield* ChangeSet.builder(Bank, work)
			yield* draft.delete(Account, [observed.previous])
			yield* draft.insert(Account, [{ ...observed.previous, balance: observed.previous.balance + 1n }])
			const changes = yield* draft.finish()
			return yield* db.apply(changes, { ...work, expected: { kind: "exact", at: observed.at } })
		})
	)
void correct
```

## 11. Exact floats and dense intervals

`f64` is a real schema scalar: NaN canonicalizes to the one quiet NaN,
`-0` to `+0`, and the relational order is total. `interval(f64)` is the
parameterized dense interval — half-open, NaN-free, strictly ordered.
`span` builds checked interval values as `Result`s.

```ts
const Window = relation("Window", { id: id128, confidence: interval(f64), during: interval(i64) })
const Windows = schema("Windows", { Window }, [key(Window, ["id"])])

const discrete = span(0n, 60n)
const dense = span(0.25, 1.5)
const bothChecked: boolean = Result.isSuccess(discrete) && Result.isSuccess(dense)
void [Windows, bothChecked]
```

## 12. Scoped ownership and honest close

Every native resource is scoped; early `close()` is itself an Effect
returning the honest `CloseReport`. A scope finalizer that cannot complete
teardown surfaces a structured `CloseFailure` DEFECT in the Cause — never a
silently swallowed failure, never falsely reclaimed resources.

```ts
const Item = relation("Item", { id: id128, label: str })
const Items = schema("Items", { Item }, [key(Item, ["id"])])

const explicitClose = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.open(localPath, Items, work)
		const report = yield* db.close()
		// `closed` releases this capability's obligations; `incomplete` and
		// `failed` retain native Closing accounting — they are never
		// counted as reclaimed.
		return report.kind
	})
)
void explicitClose
```

## 13. Unresolved field arithmetic is authoring metadata

`Scalar.field("units")` is not a typed program. Builders accept it inside
arithmetic. Native compilation binds it against the verified source
schema — including empty input — before any manifest write or freeze.

```ts
const incrementUnits = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
const asFloat = Scalar.toF64(Scalar.add(Scalar.field("units"), Scalar.u64(1n)))
void [incrementUnits, asFloat]
```

