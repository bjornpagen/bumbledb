# The Bumbledb TypeScript cookbook

Worked recipes for the Effect-native surface: typed schema values,
declared keys and laws, application-owned `Uuid` identity, one bounded
native runtime, scoped resources, immutable final-state changes, `Option`
reads, sealed complete results, and one-shot page streams.

Every `ts` fence below is extracted and type-checked against the real
package surface by `test/cookbook-doc.test.ts`; the imports fence here is
prepended to every recipe. The examples are lazy Effect programs — nothing
below runs a database at import time.

```ts
import { Effect, Option, Result, Stream } from "effect"
import {
	alternatives,
	bool,
	capacity,
	ChangeSet,
	closed,
	closedId,
	Compute,
	contained,
	Db,
	describeQuery,
	duration,
	f64,
	i64,
	uuid,
	Uuid,
	interval,
	key,
	mirrors,
	NativeRuntime,
	on,
	query,
	queryFromDescription,
	ref,
	relation,
	Scalar,
	schema,
	select,
	str,
	u64,
	v,
	weigh,
	within
} from "@bjornpagen/bumbledb"
import type {
	ApplyOutcome,
	CompleteResult,
	Fact,
	FloatIntervalValue,
	IntervalValue,
	QueryReader
} from "@bjornpagen/bumbledb"

declare const localPath: string
```

## 1. One schema, typed twice — relations, declared keys, laws

Identity fields are ordinary application-owned `Uuid` values: the database
issues no identity and there is no `fresh` mint. Keys are declared
statements; references and capacity are laws over the same fields. The same
declarations, spelled in Rust's `schema!`, produce the same canonical schema
identity.

```ts
const Student = relation("Student", { id: uuid, name: str, budget: u64 })
const Attempt = relation("Attempt", {
	id: uuid,
	student: uuid,
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

`NativeRuntime.layer()` acquires the single native runtime
with scope; reuse ONE layer value so Effect's memoization shares it.
Optional configuration controls workers, outstanding jobs/handles, and cleanup
reporting. Defaults are up to four workers, 128 queued jobs, 128 cleanup reports,
64 directory owners, 1,024 native handles, and a five-second cleanup report window.
Operations use ordinary allocation without byte/row/work quotas or execution
deadlines. Effect interruption requests cooperative cancellation. Runtime
inspection reports outstanding work, not memory usage.
Database `inspect().storage` reports virtual map extent, populated file length,
non-free LMDB pages, and allocated disk blocks (null when unavailable).
None of these measures process heap usage or resident RAM. Log history
inspection uses the same `StorageInspection` fields.
`Db.open` never creates a missing database; `Db.create` refuses existing
authority. Both are scoped acquisitions.

```ts
const Doc = relation("Doc", { id: uuid, text: str })
const Docs = schema("Docs", { Doc }, [key(Doc, ["id"])])

const openExisting = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.open(localPath, Docs)
		return db.schemaId
	})
)

const createOnce = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.create(localPath, Docs)
		return db.schemaId
	})
)

// One boundary; an Effect app provides the layer in its own graph instead.
const layer = NativeRuntime.layer()
void [openExisting.pipe(Effect.provide(layer)), createOnce.pipe(Effect.provide(layer))]
```

## 3. Changes: build lazily, normalize once, apply atomically

`ChangeSet.builder` acquires a scoped database-free draft. Ingestion effects
are lazy and re-runnable while the draft is building — each execution reads
the then-current iterable. Within ONE change set the
normalization is `(add, remove ∖ add)`: the identical fact's add wins
independent of call order. `finish()` consumes the draft into an immutable,
reusable `ChangeSet`.

```ts
const Task = relation("Task", { id: uuid, title: str, done: u64 })
const Tasks = schema("Tasks", { Task }, [key(Task, ["id"])])

const applyOnce = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.open(localPath, Tasks)
		const taskId = yield* Effect.sync(() => crypto.randomUUID())
		const draft = yield* ChangeSet.builder(Tasks)
		yield* draft.insert(Task, [{ id: taskId, title: "write the cookbook", done: 0n }])
		const changes = yield* draft.finish()
		const outcome: ApplyOutcome = yield* db.apply(changes, { expected: { kind: "any" } })
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

For a noncommitting check, `db.judge` runs the same admission procedure and
aborts its private candidate on the native worker. It briefly takes the
writer; it does not hold a writer session open in JavaScript. Admitted and
rejected judgments include the actual base and net proposed fact counts.
Operational errors stay in the Effect error channel.

```ts
const judgeOnce = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.open(localPath, Tasks)
		const draft = yield* ChangeSet.builder(Tasks)
		const id = yield* Effect.sync(() => crypto.randomUUID())
		yield* draft.insert(Task, [{ id, title: "inspect a candidate", done: 0n }])
		return yield* db.judge(yield* draft.finish(), { expected: { kind: "any" } })
	})
)
void judgeOnce
```

Judgment is optional: `apply` always judges for itself. To act on a judgment,
use its base as an exact expected witness and handle `moved`; do not assume
another writer could not change the database between these operations.

Change sets also work as inspectable values without a database. Their
`counts` report distinct requested additions/removals, not the net change
against any store. `records()` yields `{ relation, kind, fact }` records;
the relation name narrows the fact type. Every traversal opens an independent
cursor over the shared native bytes. Early termination, failure and interruption
close it, without consuming the change set. Delivery is bounded to 256 records
and a 64 KiB canonical-byte target per batch; a larger individual row is allowed.

`toBytes()` explicitly copies the complete canonical payload. `fromBytes`
checks its schema fingerprint, framing, scalar values, order and uniqueness;
it refuses malformed bytes instead of silently normalizing them. Input bytes
must remain stable until the Effect exits, and returned bytes are independently
owned. Native byte order is canonical encoding order, not insertion order.

```ts
const inspectAndCompose = Effect.scoped(
	Effect.gen(function* () {
		const draft = yield* ChangeSet.builder(Tasks)
		const id = yield* Effect.sync(() => crypto.randomUUID())
		yield* draft.insert(Task, [{ id, title: "inspect typed changes", done: 0n }])
		const changes = yield* draft.finish()
		const bytes = yield* changes.toBytes()
		const decoded = yield* ChangeSet.fromBytes(Tasks, bytes)
		const combined = yield* changes.compose(decoded)
		const preview = yield* Stream.runCollect(combined.records().pipe(Stream.take(10)))
		return { counts: combined.counts, byteLength: combined.byteLength,
			titles: preview.map((record) => record.fact.title) }
	})
)
void inspectAndCompose
```

`compose` performs a native linear merge without decoding/re-encoding rows.
It is commutative, associative and idempotent: the identical fact's add wins
over its removal within the combined command. Two distinct rows sharing a key
both survive composition and can still be rejected by admission. Applying two
commands sequentially has different semantics: a later removal removes a
previously added fact. Neither input is consumed by composition.

## 4. Keyed reads are Options

`get` takes the exact key descriptor used in the schema and its complete
projected value. Every declared key is usable; there is no implicit primary key.
A missing key is
`Option.none` — never a fake I/O error, never a nullable row.

```ts
const Person = relation("Person", { id: uuid, name: str })
const PersonById = key(Person, ["id"])
const People = schema("People", { Person }, [PersonById])

const lookup = (reader: QueryReader<typeof People>, personId: Uuid) =>
	Effect.gen(function* () {
		const found = yield* reader.get(PersonById, { id: personId })
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
const Author = relation("Author", { id: uuid, name: str })
const Book = relation("Book", { id: uuid, author: uuid, pages: u64 })
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
			const result = yield* reader.execute(booksBy, { name })
			return yield* result.collect()
		})
	)
void readBooks
```

For repeated calls on one snapshot, prepare once. The scoped handle keeps
one compiled plan and reuses its execution buffers; only parameters cross
the native boundary on subsequent calls. One-shot `execute` does not retain
a plan. Preparations share the snapshot's pinned version but close
independently; completed results also remain independent.

Ordinary execution keeps buffers for reuse. If a prepared query will sit idle
after a large operation, `yield* prepared.releaseMemory()` drops its
execution buffers but keeps the compiled query and pinned snapshot. Shared
query caches belong to the database: `yield* db.clearCache()` clears those
separately. Neither operation invalidates live results. Both are optional;
scope closure releases ownership as usual.

```ts
const readTwoAuthors = (reader: QueryReader<typeof Library>) =>
	Effect.scoped(
		Effect.gen(function* () {
			const prepared = yield* reader.prepare(booksBy)
			const first = yield* prepared.execute({ name: "Ursula Le Guin" })
			const second = yield* prepared.execute({ name: "Octavia Butler" })
			return [
				yield* first.collect(),
				yield* second.collect()
			]
		})
	)
void readTwoAuthors
```

Generated queries use the same checked rule, scalar, and execution machinery.
`describeQuery` exposes the logical IR and its names as owned data. Its relation
ordinals refer to the supplied schema's declaration order; variables are local
to each rule. Intermediate names are local labels. The result-field record
passed to `queryFromDescription` must match the actual derived head, including
closed vocabularies, and determines the returned row type. Parameters supplied
to a generated query are checked against their uses during execution.

```ts
const Event = relation("Event", { id: uuid, amount: u64 })
const Events = schema("Events", { Event }, [key(Event, ["id"])])
const countEvents = query(Events).rule((r) =>
	r.match(Event, {}).find({ count: r.count() })
)
const description = describeQuery(countEvents)
const totalEvents = queryFromDescription(
	Events,
	{ ...description, columns: ["total"] },
	{ total: u64 }
)
const readTotal = (reader: QueryReader<typeof Events>) =>
	Effect.scoped(Effect.gen(function* () {
		const result = yield* reader.execute(totalEvents, {})
		const rows: readonly { readonly total: bigint }[] = yield* result.collect()
		return rows
	}))
void readTotal
```

Descriptions contain bigint and byte values, so they are not a JSON wire format.
An accepted description is an authored query; native preparation still checks
engine semantics. It carries no snapshot, prepared handle, or result rows.

## 6. Grouped exact aggregates

Aggregates fold the group's distinct full bindings: keep the identity-bearing
`id` in the binding set so equal scores in different rows both contribute.
Exact float `sum`/`mean` are deterministic with one final rounding.

```ts
const Sample = relation("Sample", { id: uuid, series: uuid, value: f64 })
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
const Reading = relation("Reading", { id: uuid, sensor: uuid, value: f64 })
const Sensor = relation("Sensor", { id: uuid, label: str })
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

## 8. Completed results: collect or consume delivery batches

`collect()` explicitly materializes all rows into JavaScript and
leaves the result available. `pages()` is a ONE-SHOT consuming
stream over the completed result: the first run moves the backing into a
private scoped cursor; a second run refuses. Every element is one owned page
array of up to 256 rows — pages, not rows — delivered after complete evaluation.
Paging bounds delivery, not query execution or total result storage. An empty
result emits one empty page; interruption and early termination close the cursor.

```ts
const Event = relation("Event", { id: uuid, at: i64 })
const Feed = schema("Feed", { Event }, [key(Event, ["id"])])

const everything = query(Feed).rule((r) => {
	const { id, at } = v(Event)
	return r.match(Event, { id, at }).find({ id, at })
})

const drain = (result: CompleteResult<{ readonly id: Uuid; readonly at: bigint }>) =>
	result.pages().pipe(
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

`Effect.sync(() => crypto.randomUUID())` is effectful cryptographic entropy — run it once for an
original intent, persist the value with the request, and never regenerate
inside a retry. `Uuid.parse` is the pure fixed-size parser returning
`Result`.

```ts
const parsed = Uuid.parse("00112233-4455-6677-8899-aabbccddeeff")
const okOrRefused: boolean = Result.isSuccess(parsed)

const mintOnce = Effect.gen(function* () {
	const id = yield* Effect.sync(() => crypto.randomUUID())
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
const Account = relation("Account", { id: uuid, balance: i64 })
const AccountById = key(Account, ["id"])
const Bank = schema("Bank", { Account }, [AccountById])

const correct = (accountId: Uuid) =>
	Effect.scoped(
		Effect.gen(function* () {
			const db = yield* Db.open(localPath, Bank)
			const observed = yield* Effect.scoped(
				Effect.gen(function* () {
					const snapshot = yield* db.snapshot()
					const previous = yield* snapshot.get(AccountById, { id: accountId })
					if (Option.isNone(previous)) {
						return yield* Effect.fail({ missing: accountId })
					}
					return { previous: previous.value, at: snapshot.witness }
				})
			)
			const draft = yield* ChangeSet.builder(Bank)
			yield* draft.delete(Account, [observed.previous])
			yield* draft.insert(Account, [{ ...observed.previous, balance: observed.previous.balance + 1n }])
			const changes = yield* draft.finish()
			return yield* db.apply(changes, { expected: { kind: "exact", at: observed.at } })
		})
	)
void correct
```

## 11. Exact floats and dense intervals

`f64` is a real schema scalar: NaN canonicalizes to the one quiet NaN,
`-0` to `+0`, and the relational order is total. `interval(f64)` is the
parameterized dense interval — half-open, NaN-free, strictly ordered.
Interval values are structural records. Their field descriptor validates
endpoint ranges, nonemptiness and fixed width at the boundary. Integer rays
end at the element's maximum; fixed-width intervals cannot be rays.

```ts
const Window = relation("Window", { id: uuid, confidence: interval(f64), during: interval(i64) })
const Windows = schema("Windows", { Window }, [key(Window, ["id"])])

const discrete: IntervalValue = { start: 0n, end: 60n }
const dense: FloatIntervalValue = { start: 0.25, end: 1.5 }
void [Windows, discrete, dense]
```

## 12. Scoped ownership and honest close

Every native resource is scoped; early `close()` is itself an Effect
returning the honest `CloseReport`. A scope finalizer that cannot complete
teardown surfaces a structured `CloseFailure` DEFECT in the Cause — never a
silently swallowed failure, never falsely reclaimed resources.

```ts
const Item = relation("Item", { id: uuid, label: str })
const Items = schema("Items", { Item }, [key(Item, ["id"])])

const explicitClose = Effect.scoped(
	Effect.gen(function* () {
		const db = yield* Db.open(localPath, Items)
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


## 14. Derive slices, measure them, and round the total

An earning range crosses two rate bands. Intersection produces a relation of
nonempty segments, measurement produces exact integer widths, and ordinary
stages combine the results. These are synthetic arithmetic units, not tax policy.

```ts
const Earning = relation("Earning", { id: u64, schedule: u64, span: interval(u64) })
const Band = relation("Band", { id: u64, schedule: u64, span: interval(u64), numerator: u64 })
const Rates = schema("Rates", { Earning, Band }, [key(Earning, ["id"]), key(Band, ["id"])])
const overlaps = query(Rates).rule((r) => {
	const earning = v(Earning)
	const band = v(Band)
	return r.match(Earning, earning).match(Band, { ...band, schedule: earning.schedule }).find({
		earning: earning.id, band: band.id, numerator: band.numerator,
		span: r.intersection(earning.span, band.span)
	})
})
const weighted = query(Rates).rule((r) => {
	const row = v(overlaps)
	return r.match(overlaps, row).find({ earning: row.earning, band: row.band,
		weighted: Compute.multiply(Compute.measure(row.span), row.numerator) })
})
const totals = query(Rates).rule((r) => {
	const row = v(weighted)
	return r.match(weighted, row).find({ earning: row.earning, total: r.sum(row.weighted) })
})
const amounts = query(Rates).rule((r) => {
	const row = v(totals)
	return r.match(totals, row).find({ earning: row.earning,
		amount: Compute.mulDiv(row.total, Compute.u64(1n), Compute.u64(10n), "nearestTiesToEven") })
})
const calculate = Effect.scoped(Effect.gen(function* () {
	const db = yield* Db.create(localPath, Rates)
	const change = yield* ChangeSet.builder(Rates)
	yield* change.insert(Earning, [{ id: 1n, schedule: 1n, span: { start: 80n, end: 140n } }])
	yield* change.insert(Band, [
		{ id: 1n, schedule: 1n, span: { start: 0n, end: 100n }, numerator: 1n },
		{ id: 2n, schedule: 1n, span: { start: 100n, end: 200n }, numerator: 2n }
	])
	yield* db.apply(yield* change.finish(), { expected: { kind: "any" } })
	const snapshot = yield* db.snapshot()
	return yield* (yield* snapshot.execute(amounts, {})).collect()
}))
// [{ earning: 1n, amount: 10n }]: (20*1 + 40*2)/10, rounded once.
void calculate
```

Retain contribution identity through intermediate projections: two distinct
bands contributing the same amount must both count. Projecting only the amount
intentionally deduplicates it. Round after summing when that is the intended
calculation; summing separately rounded contributions can give a different result.
Checked multiplication and aggregate result ranges still apply before the final
`mulDiv`. The query creates no stored slice or copied-width facts.

## 15. Subtract a window, coalesce coverage, and measure

Binary difference returns zero, one, or two maximal nonempty pieces. A subsequent
`pack` stage coalesces overlapping or adjacent coverage per owner; measurement
then sums covered width once.

```ts
const Window = relation("Window", { id: u64, owner: u64, span: interval(i64), excluded: interval(i64) })
const Windows = schema("Windows", { Window }, [key(Window, ["id"])])
const remaining = query(Windows).rule((r) => {
	const row = v(Window)
	return r.match(Window, row).find({ owner: row.owner, span: r.difference(row.span, row.excluded) })
})
const coverage = query(Windows).rule((r) => {
	const row = v(remaining)
	return r.match(remaining, row).find({ owner: row.owner, span: r.pack(row.span) })
})
const widths = query(Windows).rule((r) => {
	const row = v(coverage)
	return r.match(coverage, row).find({ owner: row.owner, span: row.span, width: Compute.measure(row.span) })
})
const coveredWidth = query(Windows).rule((r) => {
	const row = v(widths)
	return r.match(widths, row).find({ owner: row.owner, width: r.sum(row.width) })
})
void coveredWidth
```

For `[0,10)` minus `[3,7)`, the pieces are `[0,3)` and `[7,10)`.
Subtracting the whole input produces no rows. Absence is never a null or an empty
interval. Delivery order is unspecified. Multiple producers in one head form
all combinations; they are not zipped. Their input variables must already be
bound. Use another stage to consume produced intervals, aggregate, or `pack`.
Ordinary joins, Allen predicates, and negation accept those derived stages.
An empty interval producer removes that binding before the same head's scalar
expressions run, regardless of column order. Once a stage produces an arithmetic
failure, subsequent stages cannot turn that failure into an empty answer.

Both operands must share an interval element kind: `i64`, `u64`, or `f64`.
Fixed-width inputs yield general intervals because clipping may change width.
`pack` also returns general intervals: merging adjacent fixed-width inputs can
produce a longer interval. These output types survive description export/import.
Construction compares endpoints and is independent of represented width; output
enumeration still costs work proportional to its cardinality.

`measure` returns `u64` for bounded integer intervals and the native once-rounded
`f64` length for dense intervals. Integer maximum endpoints represent rays.
Unbounded measurement and finite overflow are distinct failures. Clip a ray to a
finite interval before measuring it. Filtering a later stage cannot erase an
upstream arithmetic failure. Exact measure additivity applies to bounded integer
segments, not arbitrary sums of already-rounded floating lengths.

Each difference subtracts one interval. Unioning `A minus B1` with `A minus B2`
does not subtract their combined coverage from `A`.

## 16. Exhaustive alternatives with ordinary laws

Declare each key once. The helper expands a closed roster into ordinary
containment and mirrors statements in roster order. Equivalent independently
constructed descriptors work; JavaScript object identity has no meaning here.

```ts
const Kind = closed("Kind", ["Imported", "Electronic", "Postal"])
const Document = relation("Document", { id: u64, kind: closedId(Kind) })
const Imported = relation("Imported", { document: u64, evidence: str })
const Electronic = relation("Electronic", { document: u64, reference: str })
const Postal = relation("Postal", { document: u64, window: interval(i64) })
const documentKey = key(Document, ["id"])
const arms = {
	Imported: key(Imported, ["document"]),
	Electronic: key(Electronic, ["document"]),
	Postal: key(Postal, ["document"])
}
const Documents = schema("Documents", { Kind, Document, Imported, Electronic, Postal }, [
	documentKey, arms.Imported, arms.Electronic, arms.Postal,
	...alternatives(documentKey, "kind", Kind, arms)
])
void Documents
```

Missing, extra, wrong, or duplicate payloads refuse under the generated laws.
Switch the discriminator and replace its payload in one change: the final state
is checked, independent of insert/delete call order. Composite scalar keys work.
Interval keys cannot establish exactly one payload row because their containment
means pointwise coverage. Payloads themselves can contain intervals.

Each payload keeps its own inferred fields and ordinary relation representation.
Evidence and ownership still need their own laws. The expansion has the same
native descriptor, fingerprint, and admission costs as the identical manual laws.

## 17. Exact integer quotients and explicit rounding

`mulDiv` requires matching integer operands and a strictly positive divisor.
It computes the product exactly in a wider native integer and checks the public
64-bit result range after rounding.

```ts
const Input = relation("Input", { amount: i64 })
const Arithmetic = schema("Arithmetic", { Input }, [])
const quotients = query(Arithmetic).rule((r) => {
	const row = v(Input)
	return r.match(Input, row).find({
		truncated: Compute.mulDiv(row.amount, Compute.i64(1n), Compute.i64(2n), "towardZero"),
		away: Compute.mulDiv(row.amount, Compute.i64(1n), Compute.i64(2n), "nearestTiesAwayFromZero"),
		even: Compute.mulDiv(row.amount, Compute.i64(1n), Compute.i64(2n), "nearestTiesToEven")
	})
})
void quotients
```

For `amount = -5`, the results are `-2`, `-3`, and `-2`. For `-7`, ties-to-even
returns `-4`. Exact quotients do not change. `u64::MAX * 2 / 2` succeeds;
`i64::MIN * -1 / 2` returns `2^62`; `i64::MIN * -1 / 1` refuses final overflow.
Existing checked multiply/divide retain their intermediate-overflow behavior,
so replacing them with `mulDiv` is an explicit semantic choice. All three modes
perform bounded work without an additional scan, index, or JavaScript evaluator.
