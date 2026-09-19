import assert from "node:assert/strict"
import { readFileSync } from "node:fs"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	ChangeSet,
	Db,
	Event,
	EventDescriptor,
	type EventDescriptorDescription,
	type EventDescriptorInspection,
	EventExpr,
	type EventFibreDescription,
	type EventMapDescription,
	event,
	NativeRuntime,
	query,
	RelationExpr,
	relation,
	schema,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const identity = (n: number) => new Uint8Array(32).fill(n)
const bytes = (value: EventDescriptor) => Buffer.from(EventDescriptor.toBytes(value))
const eventBytes = (value: Event) => Buffer.from(Event.toBytes(value))
function inspected<K extends EventDescriptorInspection["kind"]>(value: EventDescriptorInspection, kind: K) {
	assert.equal(value.kind, kind)
	return value as EventDescriptorInspection & { readonly kind: K }
}
const setup = Effect.gen(function* () {
	const states = yield* Event.space(identity(211), 1n)
	const environment = yield* Event.space(identity(212), 0n)
	const toEnvironment: EventMapDescription = { source: states, target: environment, readouts: [] }
	const product: EventFibreDescription = {
		identity: identity(213),
		left: toEnvironment,
		right: toEnvironment,
		reversed: false
	}
	const fibre = yield* EventDescriptor.admit({ kind: "fibre", product })
	const inspection = inspected(yield* EventDescriptor.inspect(fibre), "fibre")
	return { states, environment, toEnvironment, product, fibre, inspection }
})

test("descriptor construction is lazy and pure transport remains addon-free", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const input = Buffer.from("BEDC\x01")
	const value = Result.getOrThrow(EventDescriptor.fromBytes(input))
	const programs = [
		EventDescriptor.admit({ kind: "fibre" } as never),
		EventDescriptor.describe(value),
		EventDescriptor.inspect(value)
	]
	assert.equal(programs.length, 3)
	input.fill(0)
	assert.deepEqual(bytes(value), Buffer.from("BEDC\x01"))
	assert.equal(nativeBindingIsLoaded(), false)
})

test("all seven structural kinds admit, describe, inspect and roundtrip through owned BEDC", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: EventDescriptorDescription | undefined
	let retainedInspection: EventDescriptorInspection | undefined
	let original: Buffer | undefined
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const { states, toEnvironment, product, fibre, inspection } = yield* setup
				assert.equal(yield* Event.count(inspection.space), 4n)
				const left = yield* EventDescriptor.describe(inspection.left)
				const right = yield* EventDescriptor.describe(inspection.right)
				assert.equal(left.kind, "surjective")
				assert.equal(right.kind, "surjective")
				assert.ok("map" in left && "map" in right)
				const region = yield* Event.coordinate(inspection.space, 0n)
				const descriptions: readonly EventDescriptorDescription[] = [
					{ kind: "map", map: left.map },
					{ kind: "surjective", map: left.map },
					{ kind: "faces", identity: identity(214), environments: [toEnvironment, toEnvironment, toEnvironment] },
					{ kind: "fibre", product },
					{ kind: "relation", product, region },
					{ kind: "composition", identity: identity(215), st: product, tu: product, su: product },
					{ kind: "square", product, left: left.map, right: right.map }
				]
				for (const description of descriptions) {
					const descriptor = yield* EventDescriptor.admit(description)
					const data = yield* EventDescriptor.describe(descriptor)
					assert.equal(data.kind, description.kind)
					assert.deepEqual(bytes(yield* EventDescriptor.admit(data)), bytes(descriptor))
					const result = yield* EventDescriptor.inspect(descriptor)
					assert.equal(result.kind, description.kind)
					switch (result.kind) {
						case "map":
						case "surjective":
							assert.deepEqual(eventBytes(result.source), eventBytes(inspection.space))
							assert.deepEqual(eventBytes(result.target), eventBytes(states))
							assert.deepEqual(result.readouts.map(eventBytes), [eventBytes(region)])
							break
						case "faces":
							assert.equal(yield* Event.count(result.space), 8n)
							assert.equal(result.projections.length, 3)
							assert.equal(result.environments.length, 3)
							for (const projection of result.projections)
								assert.equal((yield* EventDescriptor.describe(projection)).kind, "surjective")
							break
						case "fibre":
							assert.deepEqual(eventBytes(result.space), eventBytes(inspection.space))
							assert.deepEqual(bytes(result.left), bytes(inspection.left))
							break
						case "relation":
							assert.deepEqual(eventBytes(result.region), eventBytes(region))
							assert.deepEqual(bytes(result.product), bytes(fibre))
							assert.deepEqual(eventBytes(result.input), eventBytes(states))
							assert.deepEqual(eventBytes(result.output), eventBytes(states))
							break
						case "composition":
							assert.equal(yield* Event.count(result.workspace), 8n)
							assert.equal(result.products.length, 3)
							assert.equal(result.projections.length, 3)
							for (const pair of result.products) assert.deepEqual(bytes(pair), bytes(fibre))
							break
						case "square": {
							const joint = inspected(yield* EventDescriptor.inspect(result.joint), "surjective")
							assert.deepEqual(eventBytes(joint.source), eventBytes(inspection.space))
							assert.deepEqual(eventBytes(joint.target), eventBytes(inspection.space))
							assert.deepEqual(bytes(result.product), bytes(fibre))
							break
						}
					}
					if (description.kind === "fibre") {
						original = bytes(descriptor)
						retained = data
						retainedInspection = result
						if (data.kind === "fibre") data.product.identity.fill(0)
						assert.deepEqual(bytes(descriptor), original, "editable data does not mutate the carrier")
						retained = yield* EventDescriptor.describe(descriptor)
					}
				}
				// Independent BEDC grammar fixture, assembled without the native codec.
				const u64 = (n: number) => {
					const b = Buffer.alloc(8)
					b.writeBigUInt64LE(BigInt(n))
					return b
				}
				const blob = (b: Buffer) => Buffer.concat([u64(b.length), b])
				const map = yield* EventDescriptor.admit({ kind: "map", map: toEnvironment })
				assert.deepEqual(
					bytes(map),
					Buffer.concat([
						Buffer.from("BEDC\x01\x00"),
						blob(eventBytes(toEnvironment.source)),
						blob(eventBytes(toEnvironment.target)),
						u64(0)
					])
				)
			})
		)
	} finally {
		await runtime.dispose()
	}
	assert.ok(retained && retainedInspection && original)
	assert.equal(retainedInspection.kind, "fibre")
	const next = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		assert.deepEqual(bytes(await next.runPromise(EventDescriptor.admit(retained))), original)
	} finally {
		await next.dispose()
	}
})

test("derived projections and complete composition plans feed real stored Event queries", async () => {
	const Row = relation("Row", { region: event })
	const Theory = schema("DescriptorMoves", { Row }, [])
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const { product, fibre, inspection } = yield* setup
					const plan = yield* EventDescriptor.admit({
						kind: "composition",
						identity: identity(216),
						st: product,
						tu: product,
						su: product
					})
					const a = yield* Event.coordinate(inspection.space, 0n)
					const b = yield* Event.coordinate(inspection.space, 1n)
					const flip = yield* Event.xor(a, b)
					const db = yield* Db.create(storeDir("descriptor-moves"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Row, [{ region: flip }])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const scan = query(Theory).rule((r) => {
						const row = v(Row)
						const rel = RelationExpr.bind(row.region, fibre)
						return r.match(Row, row).find({
							reachable: EventExpr.region(RelationExpr.star(rel, plan)),
							twice: EventExpr.region(RelationExpr.compose(rel, rel, plan)),
							domain: EventExpr.image(row.region, inspection.left),
							range: EventExpr.image(row.region, inspection.right)
						})
					})
					const snapshot = yield* db.snapshot()
					const answers = yield* (yield* snapshot.execute(scan, {})).collect()
					assert.equal(answers.length, 1)
					assert.ok(answers[0])
					assert.equal(yield* Event.isFull(answers[0].reachable), true)
					assert.equal(yield* Event.equal(answers[0].twice, yield* Event.complement(flip)), true)
					assert.equal(yield* Event.isFull(answers[0].domain), true)
					assert.equal(yield* Event.isFull(answers[0].range), true)
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})

test("composition inspection keeps three distinct roles in ST, TU, SU order", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const e = yield* Event.space(identity(222), 0n)
				const s = yield* Event.space(identity(223), 1n)
				const t = yield* Event.space(identity(224), 2n)
				const u = yield* Event.space(identity(225), 1n)
				const environment = (source: Event): EventMapDescription => ({ source, target: e, readouts: [] })
				const pair = (name: number, left: Event, right: Event): EventFibreDescription => ({
					identity: identity(name),
					left: environment(left),
					right: environment(right),
					reversed: false
				})
				const st = pair(226, s, t)
				const tu = pair(227, t, u)
				const su = pair(228, s, u)
				const plan = yield* EventDescriptor.admit({ kind: "composition", identity: identity(229), st, tu, su })
				const result = inspected(yield* EventDescriptor.inspect(plan), "composition")
				assert.equal(yield* Event.count(result.workspace), 16n)
				for (const [index, coordinates] of [
					[0, [0n, 1n, 2n]],
					[1, [1n, 2n, 3n]],
					[2, [0n, 3n]]
				] as const) {
					const product = inspected(yield* EventDescriptor.inspect(result.products[index]), "fibre")
					const projection = inspected(yield* EventDescriptor.inspect(result.projections[index]), "surjective")
					assert.deepEqual(eventBytes(projection.source), eventBytes(result.workspace))
					assert.deepEqual(eventBytes(projection.target), eventBytes(product.space))
					const expected: Buffer[] = []
					for (const coordinate of coordinates)
						expected.push(eventBytes(yield* Event.coordinate(result.workspace, coordinate)))
					assert.deepEqual(projection.readouts.map(eventBytes), expected)
				}
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("nonrectangular shared environments, reversal and measured endpoints survive inspection", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const raw = yield* Event.space(identity(217), 2n)
				const support = yield* Event.or(yield* Event.coordinate(raw, 0n), yield* Event.coordinate(raw, 1n))
				const source = yield* Event.restrict(raw, support)
				const target = yield* Event.space(identity(218), 1n)
				const left: EventMapDescription = { source, target, readouts: [yield* Event.coordinate(source, 0n)] }
				const right: EventMapDescription = { source: target, target, readouts: [yield* Event.coordinate(target, 0n)] }
				const product: EventFibreDescription = { identity: identity(219), left, right, reversed: false }
				const normal = inspected(
					yield* EventDescriptor.inspect(yield* EventDescriptor.admit({ kind: "fibre", product })),
					"fibre"
				)
				assert.equal(yield* Event.count(normal.space), 3n, "shared environment agreement retains correlation")
				const reverse = inspected(
					yield* EventDescriptor.inspect(
						yield* EventDescriptor.admit({ kind: "fibre", product: { ...product, reversed: true } })
					),
					"fibre"
				)
				assert.deepEqual(eventBytes(reverse.space), eventBytes(normal.space))
				assert.deepEqual(bytes(reverse.left), bytes(normal.right))
				assert.deepEqual(bytes(reverse.right), bytes(normal.left))
				const rel = inspected(
					yield* EventDescriptor.inspect(
						yield* EventDescriptor.admit({
							kind: "relation",
							product: { ...product, reversed: true },
							region: reverse.space
						})
					),
					"relation"
				)
				assert.deepEqual(eventBytes(rel.input), eventBytes(target))
				assert.deepEqual(eventBytes(rel.output), eventBytes(source))
				const hex = readFileSync(new URL("./fixtures/event-v2-source.hex", import.meta.url), "utf8").trim()
				const measured = yield* Event.full(Result.getOrThrow(Event.fromBytes(Buffer.from(hex, "hex"))))
				const map = yield* EventDescriptor.admit({
					kind: "surjective",
					map: {
						source: measured,
						target: measured,
						readouts: [yield* Event.coordinate(measured, 0n), yield* Event.coordinate(measured, 1n)]
					}
				})
				const view = inspected(yield* EventDescriptor.inspect(map), "surjective")
				assert.deepEqual(eventBytes(view.source), eventBytes(measured))
				assert.deepEqual(eventBytes(view.target), eventBytes(measured))
				assert.equal(eventBytes(view.source)[4], 2)
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("admission refuses fake certificates, implicit full markers and malformed structural data", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const { states, product, fibre, inspection } = yield* setup
				const a = yield* Event.coordinate(states, 0n)
				const identityMap: EventMapDescription = { source: states, target: states, readouts: [a] }
				const zeroMap = { ...identityMap, readouts: [yield* Event.empty(states)] }
				const restricted = yield* Event.restrict(states, a)
				const foreign = yield* Event.space(identity(220), 1n)
				for (const data of [
					{ kind: "map", map: { ...identityMap, source: a } },
					{ kind: "map", map: { ...identityMap, target: a } },
					{ kind: "map", map: { ...identityMap, target: restricted } },
					{ kind: "map", map: { ...identityMap, readouts: [] } },
					{ kind: "map", map: { ...identityMap, readouts: [yield* Event.coordinate(foreign, 0n)] } },
					{ kind: "surjective", map: zeroMap },
					{ kind: "square", product, left: identityMap, right: identityMap },
					{ kind: "relation", product, region: states },
					{
						kind: "composition",
						identity: identity(221),
						st: product,
						tu: { ...product, left: { ...product.left, source: foreign } },
						su: product
					}
				] satisfies EventDescriptorDescription[]) {
					assert.equal((yield* Effect.flip(EventDescriptor.admit(data))).reason._tag, "Engine", data.kind)
				}
				// A non-onto total map remains a valid map.
				assert.equal(
					(yield* EventDescriptor.describe(yield* EventDescriptor.admit({ kind: "map", map: zeroMap }))).kind,
					"map"
				)
				let getterReads = 0
				const access = {
					kind: "map",
					get map() {
						getterReads++
						return identityMap
					}
				}
				const sparse = new Array<EventMapDescription>(1)
				const detached = identity(2)
				structuredClone(detached, { transfer: [detached.buffer] })
				for (const data of [
					access,
					{ kind: "map", map: identityMap, extra: true },
					{ kind: "fibre", product: { ...product, identity: new Uint8Array(new SharedArrayBuffer(32)) } },
					{ kind: "fibre", product: { ...product, identity: detached } },
					{ kind: "fibre", product: { ...product, reversed: 1 } },
					{ kind: "faces", identity: identity(2), environments: sparse },
					{ kind: "faces", identity: identity(2), environments: Array(4097).fill(product.left) }
				]) {
					assert.equal((yield* Effect.flip(EventDescriptor.admit(data as never))).reason._tag, "InvalidArgument")
				}
				assert.equal(getterReads, 0)
				const malformed = Result.getOrThrow(EventDescriptor.fromBytes(Buffer.from("BEDC\x01")))
				assert.equal((yield* Effect.flip(EventDescriptor.describe(malformed))).reason._tag, "Engine")
				assert.equal((yield* Effect.flip(EventDescriptor.inspect(malformed))).reason._tag, "Engine")
				assert.deepEqual(
					eventBytes(inspected(yield* EventDescriptor.inspect(fibre), "fibre").space),
					eventBytes(inspection.space)
				)
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("native descriptor submission copies owned bytes before workers and refuses shared backing", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const { states } = yield* setup
				const handle = yield* runtimeHandle()
				const source = Event.toBytes(states)
				const target = Event.toBytes(states)
				const readout = Event.toBytes(yield* Event.coordinate(states, 0n))
				const owned = yield* nativeOperationWith(
					"descriptor.raw",
					(cb) => {
						const operation = dbNative.runtimeEventDescriptor(
							handle,
							"admit",
							{ kind: "map", map: { source, target, readouts: [readout] } },
							cb
						)
						source.fill(0)
						target.fill(0)
						readout.fill(0)
						return operation
					},
					dbNative.runtimeBytesTake,
					(result) => Result.getOrThrow(EventDescriptor.fromBytes(result))
				)
				assert.equal((yield* EventDescriptor.inspect(owned)).kind, "map")
				for (const input of [new Uint8Array(new SharedArrayBuffer(32)), new Uint8Array(16 * 1024 * 1024 + 1)]) {
					const result = yield* Effect.result(
						nativeOperationWith(
							"descriptor.raw",
							(cb) => dbNative.runtimeEventDescriptor(handle, "inspect", input, cb),
							dbNative.runtimeEventDescriptorTake,
							(value) => value
						)
					)
					assert.ok(Result.isFailure(result))
				}
				let registered = false
				const failure = yield* Effect.flip(
					nativeOperationWith(
						"descriptor.raw",
						(cb) => {
							const op = dbNative.runtimeEventDescriptor(handle, "inspect", Buffer.from("BEDC\x01"), cb)
							registered = true
							return op
						},
						dbNative.runtimeEventDescriptorTake,
						(value) => value
					)
				)
				assert.equal(registered, true, "semantic refusal runs after operation registration")
				assert.equal(failure.reason._tag, "Engine")
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("cancelled descriptor delivery drops all queued inspection data and permits reuse", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventDescriptor
	try {
		const { fibre } = await runtime.runPromise(setup)
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventDescriptor = (handle, operation, input, callback) =>
			original(handle, operation, input, () => completed.resolve(callback))
		const fiber = runtime.runFork(EventDescriptor.inspect(fibre))
		const lateCallback = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		lateCallback()
		dbNative.runtimeEventDescriptor = original
		assert.equal((await runtime.runPromise(EventDescriptor.inspect(fibre))).kind, "fibre")
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeEventDescriptor = original
		await runtime.dispose()
	}
})
