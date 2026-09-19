import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
	ChangeSet,
	closed,
	contained,
	Db,
	Event,
	EventExpr,
	event,
	key,
	NativeRuntime,
	on,
	query,
	relation,
	Schema,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const copy = (value: Event) => Result.getOrThrow(Event.fromBytes(Event.toBytes(value)))
const catalog = (a: Event, b: Event, empty: Event) =>
	closed(
		"Catalog",
		["A", "B", "Empty"],
		{ group: u64, when: event, tag: u64 },
		{
			A: { group: 7n, when: a, tag: 9n },
			B: { group: 7n, when: b, tag: 9n },
			Empty: { group: 7n, when: empty, tag: 9n }
		}
	)

test("closed Event authoring compares complete owned carriers without native work", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const value = Result.getOrThrow(Event.fromBytes(Buffer.from("BEVT\x01")))
	const Catalog = catalog(value, value, value)
	const independentlyOwned = catalog(copy(value), copy(value), copy(value))
	assert.doesNotThrow(() => schema("Ground", { Catalog }, [key(independentlyOwned, ["group", "when"])]))
	assert.equal(nativeBindingIsLoaded(), false)
	// @ts-expect-error the implicit id key cannot be duplicated as an Event key
	assert.throws(() => key(Catalog, ["id"]), /closedness already materializes/)
})

test("closed Events retain schema identity, coverage and query results across runtime release", async () => {
	const path = storeDir("event-closed")
	let runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let saved: readonly [Event, Event, Event] | undefined
	let retained: readonly Event[] = []
	const Observation = relation("Observation", { id: u64, group: u64, when: event })
	const theory = (values: readonly [Event, Event, Event]) => {
		const Catalog = catalog(...values)
		const t = schema("Ground", { Catalog, Observation }, [
			key(Catalog, ["group", "when"]),
			key(Observation, ["id"]),
			contained(on(Observation, ["group", "when"]), on(Catalog, ["group", "when"]))
		])
		return { Catalog, t }
	}
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const full = yield* Event.space(new Uint8Array(32).fill(129), 2n)
					const a = yield* Event.coordinate(full, 0n)
					const b = yield* Event.complement(a)
					const empty = yield* Event.empty(full)
					saved = [copy(a), copy(b), copy(empty)]
					const { Catalog, t } = theory([a, b, empty])
					assert.equal((yield* Schema.compile(t)).schemaId, (yield* Schema.compile(theory(saved).t)).schemaId)
					assert.equal((yield* Effect.flip(Schema.compile(theory([a, a, empty]).t))).reason._tag, "Engine")
					const db = yield* Db.create(path, t)
					const seed = yield* ChangeSet.builder(t)
					yield* seed.insert(Observation, [
						{ id: 1n, group: 7n, when: copy(a) },
						{ id: 2n, group: 7n, when: copy(empty) }
					])
					assert.equal((yield* db.apply(yield* seed.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const invalid = yield* ChangeSet.builder(t)
					yield* invalid.insert(Observation, [{ id: 3n, group: 8n, when: full }])
					assert.equal(
						(yield* db.apply(yield* invalid.finish(), { expected: { kind: "any" } })).kind,
						"invariant-rejected"
					)
					const joined = query(t).rule((r) => {
						const c = v(Catalog)
						const o = v(Observation)
						return r
							.match(Observation, o)
							.match(Catalog, { id: c.id, when: o.when })
							.find({ id: c.id, region: o.when, complement: EventExpr.complement(o.when) })
					})
					const snapshot = yield* db.snapshot()
					const answers = yield* (yield* snapshot.execute(joined, {})).collect()
					assert.deepEqual(answers.map((row) => row.id).sort(), ["A", "Empty"])
					retained = answers.map((row) => row.complement)
				})
			)
		)
		await runtime.dispose()
		runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
		assert.ok(saved)
		const { Catalog, t } = theory([copy(saved[0]), copy(saved[1]), copy(saved[2])])
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					assert.deepEqual((yield* Effect.all(retained.map(Event.count))).sort(), [2n, 4n])
					const db = yield* Db.open(path, t)
					const snapshot = yield* db.snapshot()
					const q = query(t).rule((r) => {
						const row = v(Catalog)
						return r.match(Catalog, row).find({ id: row.id, when: row.when, tag: row.tag })
					})
					const rows = yield* (yield* snapshot.execute(q, {})).collect()
					assert.equal(rows.length, 3)
					assert.ok(rows.every((row) => row.tag === 9n))
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})
