import assert from "node:assert/strict"
import { readFileSync } from "node:fs"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import { snapshotData } from "#immutable.ts"
import {
	ChangeSet,
	Db,
	decodeBoundaryField,
	decodeRows,
	describeQuery,
	Event,
	encodeBoundaryField,
	encodeRows,
	event,
	type Infer,
	key,
	mirrors,
	NativeRuntime,
	on,
	query,
	queryFromDescription,
	relation,
	rowShape,
	schema,
	u64,
	v
} from "#index.ts"
import { lower } from "#lower.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { internalSchemaBindings, internalSchemaSnapshot } from "#schema-file.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Region = relation("Region", { id: u64, value: event })
const Marker = relation("Marker", { value: event })
const Theory = schema("Events", { Region, Marker }, [key(Region, ["id"])])
const shape = rowShape(Theory, Region)
const scan = query(Theory).rule((r) => {
	const row = v(Region)
	return r.match(Region, row).find(row)
})
const packed = query(Theory).rule((r) => {
	const row = v(Region)
	return r.match(Region, row).find({ value: r.pack(row.value) })
})
const joined = query(Theory).rule((r) => {
	const row = v(Region)
	return r.match(Region, row).match(Marker, { value: row.value }).find(row)
})
const parametrized = query(Theory).rule((r) => {
	const row = v(Region)
	return r.match(Region, { id: row.id, value: r.param("event") }).find({ id: row.id })
})
const paramSet = query(Theory).rule((r) => {
	const row = v(Region)
	return r.match(Region, { id: row.id, value: r.inSet("events") }).find({ id: row.id })
})

function typePins(value: Event) {
	const inferred: Infer<typeof event> = value
	void inferred
	// @ts-expect-error Bytes are not an Event value.
	const wrong: Infer<typeof event> = new Uint8Array()
	void wrong
	query(Theory).rule((r) => {
		const row = v(Region)
		// @ts-expect-error Event values have equality, not an ordering.
		return r.match(Region, row).where(r.lt(row.value, row.value)).find(row)
	})
	query(Theory).rule((r) => {
		const row = v(Region)
		// @ts-expect-error An Event is not an additive scalar.
		return r.match(Region, row).find({ value: r.sum(row.value) })
	})
}
void typePins

function fixture(): Uint8Array {
	const hex = readFileSync(new URL("./fixtures/event-v2-source.hex", import.meta.url), "utf8").trim()
	return Uint8Array.from(Buffer.from(hex, "hex"))
}
function decoded(bytes: Uint8Array) {
	return Result.getOrThrow(Event.fromBytes(bytes))
}
function bytes(value: Event) {
	return Buffer.from(Event.toBytes(value)).toString("hex")
}

// This executes before any Effect loads the addon.
test("Event envelopes own immutable data and pure codecs never load native work", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const original = fixture()
	const value = decoded(original)
	const stable = bytes(value)
	original.fill(0)
	Event.toBytes(value).fill(0)
	assert.equal(bytes(value), stable)
	assert.equal(snapshotData({ value }).value, value)
	assert.ok(Object.isFrozen(value))
	assert.equal(Event.isEvent({ ...value }), false, "copying a wrapper does not copy its owned encoding")
	const boundary = Result.getOrThrow(encodeBoundaryField(event, value))
	assert.deepEqual(boundary, { $event: stable })
	assert.equal(bytes(Result.getOrThrow(decodeBoundaryField(event, boundary))), stable)
	assert.ok(Result.isFailure(decodeBoundaryField(event, { $event: stable.toUpperCase() })))
	assert.ok(Result.isFailure(decodeBoundaryField(event, { $event: stable, extra: true })))
	assert.ok(Result.isFailure(Event.fromBytes(new Uint8Array(new SharedArrayBuffer(20)))))
	let reads = 0
	const tricky = fixture()
	Object.defineProperty(tricky, "buffer", {
		get() {
			reads++
			throw new Error("user getter")
		}
	})
	assert.equal(bytes(decoded(tricky)), stable)
	assert.equal(reads, 0)
	assert.throws(
		() =>
			query(Theory).rule((r) => {
				const row = v(Region)
				return r
					.match(Region, row)
					.where(r.lt(row.value as never, row.value as never) as never)
					.find(row)
			}),
		/Event values support equality/
	)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("Event executor checks all truth functions, shared contexts and symbolic supports", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const full = yield* Event.space(new Uint8Array(32).fill(190), 2n)
				const a = yield* Event.coordinate(full, 0n)
				const b = yield* Event.coordinate(full, 1n)
				assert.equal(yield* Event.signature(a, b), 15n)
				for (let mask = 0n; mask < 16n; mask++) {
					const result = yield* Event.apply(mask, a, b)
					for (let world = 0n; world < 4n; world++)
						assert.equal(
							yield* Event.contains(result, world),
							(mask & (1n << (((world & 1n) << 1n) | (world >> 1n)))) !== 0n
						)
				}
				const inverse = yield* Event.complement(a)
				assert.equal(yield* Event.disjoint(a, inverse), true)
				assert.equal(yield* Event.isFull(yield* Event.or(a, inverse)), true)
				assert.equal(yield* Event.subset(a, full), true)
				assert.equal(
					yield* Event.equal(yield* Event.ite(a, b, inverse), yield* Event.or(yield* Event.and(a, b), inverse)),
					true
				)
				const restricted = yield* Event.restrict(full, yield* Event.or(a, b))
				assert.equal(yield* Event.count(restricted), 3n)
				const large = yield* Event.space(new Uint8Array(32).fill(191), 62n)
				assert.equal(yield* Event.count(yield* Event.coordinate(large, 61n)), 1n << 61n)
				const empty = yield* Event.empty(full)
				const foreign = yield* Event.empty(large)
				for (const operation of [
					Effect.asVoid(Event.and(empty, foreign)),
					Effect.asVoid(Event.equal(empty, foreign)),
					Effect.asVoid(Event.apply(0n, empty, foreign))
				]) {
					const error = yield* Effect.flip(operation)
					assert.equal(error.reason._tag, "Engine")
				}
				assert.equal((yield* Effect.flip(Event.apply(16n, a, b))).reason._tag, "InvalidArgument")
				// A well-formed envelope header is not a graph/law certificate.
				const malformed = decoded(Uint8Array.from([66, 69, 86, 84, 1]))
				assert.equal((yield* Effect.flip(Event.validate(malformed))).reason._tag, "Engine")
				const measured = decoded(fixture())
				assert.equal(bytes(yield* Event.validate(measured)), bytes(measured))
			})
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("typed Event rows reopen, join, pack and bind owned parameters", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const path = storeDir("event-sdk")
	let retained: Event | undefined
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const full = yield* Event.space(new Uint8Array(32).fill(192), 2n)
					const a = yield* Event.coordinate(full, 0n)
					const b = yield* Event.complement(a)
					const independentlyOwned = decoded(Event.toBytes(a))
					const encoded = yield* encodeRows(shape, [{ id: 1n, value: a }])
					const restored = yield* decodeRows(shape, encoded)
					assert.ok(restored[0])
					assert.equal(bytes(restored[0].value), bytes(a))
					const db = yield* Db.create(path, Theory)
					const changes = yield* ChangeSet.builder(Theory)
					yield* changes.insert(Region, [
						{ id: 1n, value: a },
						{ id: 2n, value: b }
					])
					yield* changes.insert(Marker, [{ value: independentlyOwned }])
					assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					const rows = yield* (yield* snapshot.execute(scan, {})).collect()
					assert.equal(rows.length, 2)
					const matches = yield* (yield* snapshot.execute(joined, {})).collect()
					assert.deepEqual(
						matches.map((row) => row.id),
						[1n]
					)
					assert.ok(matches[0])
					assert.equal(bytes(matches[0].value), bytes(a))
					assert.deepEqual(yield* (yield* snapshot.execute(parametrized, { event: a })).collect(), [{ id: 1n }])
					assert.deepEqual(yield* (yield* snapshot.execute(paramSet, { events: [a, independentlyOwned] })).collect(), [
						{ id: 1n }
					])
					const literal = query(Theory).rule((r) => {
						const row = v(Region)
						return r.match(Region, { id: row.id, value: a }).find({ id: row.id })
					})
					const importedLiteral = queryFromDescription(Theory, describeQuery(literal), { id: u64 })
					assert.deepEqual(yield* (yield* snapshot.execute(importedLiteral, {})).collect(), [{ id: 1n }])
					const imported = queryFromDescription(Theory, describeQuery(packed), { value: event })
					const result = yield* (yield* snapshot.execute(imported, {})).collect()
					assert.equal(result.length, 1)
					assert.ok(result[0])
					retained = result[0].value
					assert.equal(yield* Event.isFull(retained), true)
					const generated = yield* internalSchemaBindings(yield* internalSchemaSnapshot(lower(Theory)))
					assert.match(generated, /db\.event/)
				})
			)
		)
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const reopened = yield* Db.open(path, Theory)
					const snapshot = yield* reopened.snapshot()
					assert.equal((yield* (yield* snapshot.execute(scan, {})).collect()).length, 2)
					const result = yield* (yield* snapshot.execute(packed, {})).collect()
					assert.equal(result.length, 1)
					assert.ok(result[0])
					assert.equal(yield* Event.isFull(result[0].value), true)
				})
			)
		)
		assert.ok(retained)
		assert.equal(await runtime.runPromise(Event.count(retained)), 4n)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
	assert.ok(retained)
	assert.ok(Event.toBytes(retained).length > 5, "returned Event bytes outlive native runtime disposal")
})

test("raw Event bridge refuses unknown operations and shared or forged byte backing", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const handle = yield* runtimeHandle()
				for (const [op, inputs, arg] of [
					["madeUp", [fixture()], 0n],
					["validate", [], 0n],
					["validate", [fixture()], 1n],
					["space", [new Uint8Array(31)], 1n],
					["space", [new Uint8Array(32)], 63n],
					["validate", [new Uint8Array(new SharedArrayBuffer(100))], 0n]
				] as const) {
					const outcome = yield* Effect.result(
						nativeOperationWith(
							"Event.raw",
							(cb) => dbNative.runtimeEvent(handle, op, inputs, arg, cb),
							dbNative.runtimeRowsTake,
							(rows) => rows
						)
					)
					assert.ok(Result.isFailure(outcome))
				}
			})
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("Event dependencies enforce disjointness and joint coverage through ordinary keys and mirrors", async () => {
	const Parent = relation("Parent", { group: u64, condition: event })
	const Child = relation("Child", { group: u64, item: u64, condition: event })
	const Partitions = schema("Partitions", { Parent, Child }, [
		key(Parent, ["group"]),
		key(Parent, ["group", "condition"]),
		key(Child, ["group", "condition"]),
		mirrors(on(Parent, ["group", "condition"]), on(Child, ["group", "condition"]))
	])
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const full = yield* Event.space(new Uint8Array(32).fill(193), 2n)
					const a = yield* Event.coordinate(full, 0n)
					const b = yield* Event.complement(a)
					const db = yield* Db.create(storeDir("event-partitions"), Partitions)
					const initial = yield* ChangeSet.builder(Partitions)
					yield* initial.insert(Parent, [{ group: 1n, condition: full }])
					yield* initial.insert(Child, [
						{ group: 1n, item: 1n, condition: a },
						{ group: 1n, item: 2n, condition: b }
					])
					assert.equal((yield* db.apply(yield* initial.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const overlap = yield* ChangeSet.builder(Partitions)
					yield* overlap.insert(Child, [{ group: 1n, item: 3n, condition: a }])
					const conflict = yield* db.apply(yield* overlap.finish(), { expected: { kind: "any" } })
					assert.equal(conflict.kind, "invariant-rejected")
					if (conflict.kind === "invariant-rejected") assert.equal(conflict.violations[0]?.kind, "functionality")
					const uncovered = yield* ChangeSet.builder(Partitions)
					yield* uncovered.delete(Child, [{ group: 1n, item: 2n, condition: b }])
					const gap = yield* db.apply(yield* uncovered.finish(), { expected: { kind: "any" } })
					assert.equal(gap.kind, "invariant-rejected")
					if (gap.kind === "invariant-rejected") assert.equal(gap.violations[0]?.kind, "containment")
					const malformed = yield* ChangeSet.builder(Partitions)
					const invalid = decoded(Uint8Array.from([66, 69, 86, 84, 1]))
					// The worker's core decoder preserves the mathematical error family.
					assert.equal(
						(yield* Effect.flip(malformed.insert(Parent, [{ group: 2n, condition: invalid }]))).reason._tag,
						"Engine"
					)
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})

test("interrupting completed Event delivery drains the output and leaves the runtime usable", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEvent
	const completed = Promise.withResolvers<() => void>()
	try {
		dbNative.runtimeEvent = (handle, operation, inputs, argument, callback) =>
			original(handle, operation, inputs, argument, () => completed.resolve(callback))
		const fiber = runtime.runFork(Event.validate(decoded(fixture())))
		const lateCallback = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		lateCallback()
		dbNative.runtimeEvent = original
		const restored = await runtime.runPromise(Event.validate(decoded(fixture())))
		assert.equal(bytes(restored), Buffer.from(fixture()).toString("hex"))
		const service = await runtime.runPromise(NativeRuntime)
		const outstanding = await runtime.runPromise(service.inspect())
		assert.equal(outstanding.retained, 0n)
	} finally {
		dbNative.runtimeEvent = original
		await runtime.dispose()
	}
})

test("parameterized Event sources retain their domains through SDK storage and joins", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	const path = storeDir("event-parameter-sdk")
	const hex = readFileSync(new URL("./fixtures/event-v3-parameter-source.hex", import.meta.url), "utf8").trim()
	const value = decoded(Buffer.from(hex, "hex"))
	let retained: Event | undefined
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					assert.equal(bytes(yield* Event.validate(value)), hex)
					const full = yield* Event.full(value)
					assert.equal(yield* Event.atomCount(full), 12n)
					assert.ok(Result.isFailure(yield* Effect.result(Event.count(full))))
					assert.ok(Result.isFailure(yield* Effect.result(Event.mass(full))))
					const endpoint = yield* Event.coordinate(value, 2n)
					const impossibleMeasurement = yield* Event.and(value, endpoint)
					assert.equal(yield* Event.isEmpty(impossibleMeasurement), false)
					assert.equal(yield* Event.count(impossibleMeasurement), 2n)
					const restricted = yield* Event.restrict(full, endpoint)
					assert.equal(yield* Event.count(restricted), 4n)
					assert.equal(Event.toBytes(restricted)[4], 3)
					const db = yield* Db.create(path, Theory)
					const changes = yield* ChangeSet.builder(Theory)
					yield* changes.insert(Region, [{ id: 1n, value }])
					yield* changes.insert(Marker, [{ value: decoded(Event.toBytes(value)) }])
					assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					const rows = yield* (yield* snapshot.execute(joined, {})).collect()
					assert.equal(rows.length, 1)
					assert.ok(rows[0])
					retained = rows[0].value
					assert.equal(bytes(retained), hex)
				})
			)
		)
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.open(path, Theory)
					const snapshot = yield* db.snapshot()
					const rows = yield* (yield* snapshot.execute(joined, {})).collect()
					assert.equal(rows.length, 1)
					assert.ok(rows[0])
					assert.equal(bytes(rows[0].value), hex)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
	assert.ok(retained)
	assert.equal(bytes(retained), hex)
})
