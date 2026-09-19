import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	ChangeSet,
	Db,
	Event,
	EventDescriptor,
	type EventFibreDescription,
	EventMemory,
	type EventMemoryDescription,
	type EventMemoryInspection,
	ExactPolynomial,
	ExactRational,
	event,
	NativeRuntime,
	ParameterDomain,
	ParameterRegion,
	PolynomialSigns,
	query,
	relation,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const id = (n: number) => new Uint8Array(32).fill(n)
const bytes = (value: EventMemory) => Buffer.from(EventMemory.toBytes(value))
const evbytes = (value: Event) => Buffer.from(Event.toBytes(value))
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}
const region = (source: Event, width: number, codes: readonly number[]) =>
	Effect.gen(function* () {
		let result = yield* Event.empty(source)
		const bits: Event[] = []
		for (let i = 0; i < width; i++) bits.push(yield* Event.coordinate(source, BigInt(i)))
		for (const code of codes) {
			let cell = source
			for (const [i, bit] of bits.entries())
				cell = yield* Event.and(cell, code & (1 << i) ? bit : yield* Event.complement(bit))
			result = yield* Event.or(result, cell)
		}
		return result
	})
const recipe = () =>
	Effect.gen(function* () {
		const source = yield* Event.space(id(150), 3n)
		const env = yield* Event.space(id(151), 0n)
		const base = { source, target: env, readouts: [] }
		const product: EventFibreDescription = { identity: id(152), left: base, right: base, reversed: false }
		const pairs = yield* EventDescriptor.inspect(yield* EventDescriptor.admit({ kind: "fibre", product }))
		assert.equal(pairs.kind, "fibre")
		assert.ok("space" in pairs)
		const actions = []
		for (const edges of [
			[
				[0, 2],
				[1, 3]
			],
			[
				[2, 4],
				[3, 5]
			],
			[
				[4, 6],
				[5, 7]
			],
			[
				[4, 7],
				[5, 6]
			]
		] as const) {
			actions.push({
				product,
				region: yield* region(
					pairs.space,
					6,
					edges.map(([s, t]) => s + 8 * t)
				)
			})
		}
		const observations = []
		for (const codes of [[0, 1, 4, 5], [2], [3], [6], [7], []]) observations.push(yield* region(source, 3, codes))
		return { source, given: yield* region(source, 3, [0, 1]), observations, actions } satisfies EventMemoryDescription
	})
function at<T>(values: readonly T[], index: number): T {
	const value = values[index]
	assert.ok(value !== undefined)
	return value
}
function successor(graph: EventMemoryInspection, state: number, action: number, observation: number) {
	const edge = graph.states[state]?.transitions.find(
		(edge) => edge.action === action && edge.observation === observation
	)
	assert.ok(edge)
	return edge.target
}

test("memory construction is lazy; pure transport owns bytes without loading the addon", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const input = Buffer.from("BEBM\x01")
	const value = Result.getOrThrow(EventMemory.fromBytes(input))
	input.fill(0)
	const programs = [EventMemory.admit({} as never), EventMemory.describe(value), EventMemory.inspect(value)]
	assert.equal(programs.length, 3)
	const copied = EventMemory.toBytes(value)
	copied.fill(0)
	assert.deepEqual(bytes(value), Buffer.from("BEBM\x01"))
	assert.equal(EventMemory.isMemory(value), true)
	assert.equal(EventMemory.isMemory({}), false)
	assert.equal(Result.isFailure(EventMemory.fromBytes(Buffer.from("BEDC\x01"))), true)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("a reopened recipe remembers the secret after its screen is erased and supplies ordinary database Events", async () => {
	const retained = await run(
		Effect.gen(function* () {
			const data = yield* recipe()
			const memory = yield* EventMemory.admit(data)
			const described = yield* EventMemory.describe(memory)
			assert.deepEqual(bytes(yield* EventMemory.admit(described)), bytes(memory))
			const graph = yield* EventMemory.inspect(memory)
			const start = graph.initial[0]
			assert.equal(typeof start, "number")
			assert.ok(start != null)
			assert.equal(graph.initial[5], null)
			const left = successor(graph, successor(graph, start, 0, 1), 1, 0)
			const right = successor(graph, successor(graph, start, 0, 2), 1, 0)
			assert.notEqual(left, right)
			assert.equal(graph.states[left]?.observation, graph.states[right]?.observation)
			assert.deepEqual(evbytes(at(graph.states, left).possible), evbytes(yield* region(data.source, 3, [4])))
			assert.deepEqual(evbytes(at(graph.states, right).possible), evbytes(yield* region(data.source, 3, [5])))
			assert.notEqual(successor(graph, left, 2, 3), successor(graph, right, 2, 4))
			return { memory, graph }
		})
	)
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const reopened = Result.getOrThrow(EventMemory.fromBytes(EventMemory.toBytes(retained.memory)))
				const graph = yield* EventMemory.inspect(reopened)
				assert.deepEqual(graph.initial, retained.graph.initial)
				assert.deepEqual(
					graph.states.map((s) => [evbytes(s.possible), s.transitions]),
					retained.graph.states.map((s) => [evbytes(s.possible), s.transitions])
				)
				const Memory = relation("Memory", { id: u64, possible: event })
				const Expected = relation("Expected", { id: u64, possible: event })
				const Theory = schema("MemoryTransport", { Memory, Expected }, [])
				const db = yield* Db.create(storeDir("memory-transport"), Theory)
				const draft = yield* ChangeSet.builder(Theory)
				yield* draft.insert(
					Memory,
					graph.states.map((s, i) => ({ id: BigInt(i), possible: s.possible }))
				)
				yield* draft.insert(
					Expected,
					retained.graph.states.map((s, i) => ({ id: BigInt(i), possible: s.possible }))
				)
				yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
				const scan = query(Theory).rule((r) => {
					const row = v(Memory)
					return r.match(Memory, row).match(Expected, row).find(row)
				})
				const snapshot = yield* db.snapshot()
				const rows = yield* (yield* snapshot.execute(scan, {})).collect()
				assert.equal(rows.length, graph.states.length)
			})
		)
	)
})

test("memory admission rejects bad contexts, incomplete observations, forged graph fields and malformed envelopes", async () => {
	await run(
		Effect.gen(function* () {
			const data = yield* recipe()
			const empty = yield* Event.empty(data.source)
			const foreign = yield* Event.space(id(159), 3n)
			for (const bad of [
				{ ...data, source: empty },
				{ ...data, given: foreign },
				{ ...data, observations: [empty] },
				{ ...data, observations: [data.source, data.source] },
				{ ...data, actions: [{ ...at(data.actions, 0), region: empty }] },
				{ ...data, states: [] }
			])
				assert.ok(yield* Effect.flip(EventMemory.admit(bad)))
			const input = yield* EventMemory.admit(data)
			const encoded = EventMemory.toBytes(input)
			const bad = new Uint8Array(encoded.length + 1)
			bad.set(encoded)
			assert.ok(yield* Effect.flip(EventMemory.inspect(Result.getOrThrow(EventMemory.fromBytes(bad)))))
			const none = yield* EventMemory.admit({ ...data, given: empty })
			assert.equal((yield* EventMemory.inspect(none)).states.length, 0)
			const runtime = yield* runtimeHandle()
			// Raw bridge checks the same schema even when SDK authoring is bypassed.
			assert.ok(
				yield* Effect.flip(
					nativeOperationWith(
						"memory raw refusal",
						(cb) => dbNative.runtimeEventMemory(runtime, "admit", { states: [] } as never, cb),
						dbNative.runtimeBytesTake,
						(v) => v
					)
				)
			)
			const fiber = yield* Effect.forkChild(Effect.forever(EventMemory.inspect(input)))
			yield* Effect.yieldNow
			yield* Fiber.interrupt(fiber)
			assert.equal(Exit.isFailure(yield* Fiber.await(fiber)), true)
			assert.equal((yield* EventMemory.inspect(input)).initial[0], 0)
		})
	)
})

test("parameter memory transport preserves one named unknown across transitions", async () => {
	await run(
		Effect.gen(function* () {
			const name = id(160)
			const p = yield* ExactPolynomial.parameter(name)
			const one = yield* ExactPolynomial.constant(yield* ExactRational.fraction(1n, 1n))
			const domain = yield* ParameterDomain.new(
				yield* ParameterRegion.and(
					yield* ParameterRegion.whereSign(name, p, PolynomialSigns.nonNegative),
					yield* ParameterRegion.whereSign(name, yield* ExactPolynomial.subtract(one, p), PolynomialSigns.nonNegative)
				)
			)
			const source = yield* Event.withParameters(yield* Event.space(id(161), 1n), domain, [])
			const env = yield* Event.withParameters(yield* Event.space(id(162), 0n), domain, [])
			const base = { source, target: env, readouts: [] }
			const product: EventFibreDescription = { identity: id(163), left: base, right: base, reversed: false }
			const pairs = yield* EventDescriptor.inspect(yield* EventDescriptor.admit({ kind: "fibre", product }))
			assert.ok("space" in pairs)
			const flip = yield* Event.xor(yield* Event.coordinate(pairs.space, 0n), yield* Event.coordinate(pairs.space, 1n))
			const given = yield* Event.coordinate(source, 0n)
			const memory = yield* EventMemory.admit({
				source,
				given,
				observations: [source],
				actions: [{ product, region: flip }]
			})
			const graph = yield* EventMemory.inspect(memory)
			assert.equal(graph.states.length, 2)
			assert.deepEqual(evbytes(at(graph.states, 0).possible), evbytes(given))
			assert.deepEqual(evbytes(at(graph.states, 1).possible), evbytes(yield* Event.complement(given)))
			assert.equal(successor(graph, 0, 0, 0), 1)
			assert.equal(successor(graph, 1, 0, 0), 0)
			assert.deepEqual(bytes(yield* EventMemory.admit(yield* EventMemory.describe(memory))), bytes(memory))
		})
	)
})

test("independent BEBM grammar and raw submission preserve owned bytes and operation registration", async () => {
	await run(
		Effect.gen(function* () {
			const source = yield* Event.space(id(164), 0n)
			const empty = yield* Event.empty(source)
			const integer = (n: number) => {
				const b = Buffer.alloc(8)
				b.writeBigUInt64LE(BigInt(n))
				return b
			}
			const blob = (v: Event) => Buffer.concat([integer(Event.toBytes(v).length), Event.toBytes(v)])
			// Independent format authoring: no native memory encoder participates.
			const bytes = Buffer.concat([
				Buffer.from("BEBM\x01"),
				blob(source),
				blob(source),
				integer(2),
				blob(empty),
				blob(source),
				integer(0)
			])
			const value = Result.getOrThrow(EventMemory.fromBytes(bytes))
			const graph = yield* EventMemory.inspect(value)
			assert.deepEqual(graph.initial, [null, 0])
			assert.equal(graph.states.length, 1)
			assert.equal(at(graph.states, 0).observation, 1)
			assert.deepEqual(
				EventMemory.toBytes(yield* EventMemory.admit(yield* EventMemory.describe(value))),
				new Uint8Array(bytes)
			)
			const runtime = yield* runtimeHandle()
			const sourceBytes = Event.toBytes(source)
			const given = Event.toBytes(source)
			const observation = Event.toBytes(source)
			const owned = yield* nativeOperationWith(
				"memory raw copy",
				(callback) => {
					const operation = dbNative.runtimeEventMemory(
						runtime,
						"admit",
						{ source: sourceBytes, given, observations: [observation], actions: [] },
						callback
					)
					sourceBytes.fill(0)
					given.fill(0)
					observation.fill(0)
					return operation
				},
				dbNative.runtimeBytesTake,
				(bytes) => Result.getOrThrow(EventMemory.fromBytes(bytes))
			)
			assert.deepEqual((yield* EventMemory.inspect(owned)).initial, [0])
			for (const input of [new Uint8Array(new SharedArrayBuffer(32)), new Uint8Array(16 * 1024 * 1024 + 1)]) {
				assert.ok(
					yield* Effect.flip(
						nativeOperationWith(
							"memory raw extent",
							(cb) => dbNative.runtimeEventMemory(runtime, "inspect", input, cb),
							dbNative.runtimeEventMemoryTake,
							(v) => v
						)
					)
				)
			}
			let registered = false
			assert.ok(
				yield* Effect.flip(
					nativeOperationWith(
						"memory raw malformed",
						(cb) => {
							const operation = dbNative.runtimeEventMemory(runtime, "inspect", Buffer.from("BEBM\x01"), cb)
							registered = true
							return operation
						},
						dbNative.runtimeEventMemoryTake,
						(v) => v
					)
				)
			)
			assert.equal(registered, true)
		})
	)
})
