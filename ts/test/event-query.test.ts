import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import { snapshotData } from "#immutable.ts"
import {
	bool,
	ChangeSet,
	Db,
	describeQuery,
	Event,
	EventDescriptor,
	EventExpr,
	type EventFind,
	type EventOperandFault,
	EventTest,
	event,
	key,
	NativeRuntime,
	type QueryRow,
	type QueryRuleScope,
	query,
	queryFromDescription,
	RelationExpr,
	relation,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Claim = relation("Claim", { id: u64, a: event, b: event })
const Theory = schema("EventQueries", { Claim }, [key(Claim, ["id"])])
const constructed = query(Theory).rule((r) => {
	const row = v(Claim)
	return r.match(Claim, row).find({
		id: row.id,
		region: EventExpr.and(row.a, EventExpr.complement(row.b)),
		safe: EventTest.subset(row.a, row.b)
	})
})
const packed = query(Theory).rule((r) => {
	const row = v(constructed)
	return r.match(constructed, row).find({ region: r.pack(row.region) })
})
const missing = query(Theory).rule((r) => {
	const row = v(packed)
	return r.match(packed, row).find({ region: EventExpr.complement(row.region) })
})

function typePins(value: Event) {
	const answer: QueryRow<typeof constructed> = { id: 1n, region: value, safe: false }
	const region: Event = answer.region
	const safe: boolean = answer.safe
	void region
	void safe
	const row = v(Claim)
	// @ts-expect-error Event algebra reads Event columns, not scalar variables.
	EventExpr.complement(row.id)
	// @ts-expect-error A host Event is a literal value, not a body-bound operand.
	EventExpr.complement(value)
	// @ts-expect-error A roster must establish its context.
	EventExpr.exactly(0n)
	// @ts-expect-error A test is not an Event expression.
	EventExpr.complement(EventTest.isEmpty(row.a))
	query(Theory).rule((r) =>
		r.match(Claim, row).find({
			// @ts-expect-error A relation's roles require an explicit Event view.
			region: RelationExpr.identity(null as never)
		})
	)
}
void typePins

function number(value: number): Buffer {
	const out = Buffer.alloc(8)
	out.writeBigUInt64LE(BigInt(value))
	return out
}
function blob(value: Uint8Array): Buffer {
	return Buffer.concat([number(value.length), value])
}
function descriptor(bytes: Uint8Array): EventDescriptor {
	return Result.getOrThrow(EventDescriptor.fromBytes(bytes))
}
function value(bytes: Uint8Array): Event {
	return Result.getOrThrow(Event.fromBytes(bytes))
}

// Tiny canonical full-support fixtures are independent of native operations.
// Coordinates are little-endian world bits; a low-edge sign owns complement.
function region(identity: number, coordinates: number, bits: number): Buffer {
	const nodes: Buffer[] = []
	const unique = new Map<string, number>()
	const visit = (coordinate: number, world: number): number => {
		if (coordinate === coordinates) return (bits >> world) & 1
		let low = visit(coordinate + 1, world)
		let high = visit(coordinate + 1, world | (1 << coordinate))
		if (low === high) return low
		const sign = low & 1
		low ^= sign
		high ^= sign
		const key = `${coordinate}/${low}/${high}`
		let ref = unique.get(key)
		if (ref === undefined) {
			const node = Buffer.alloc(9)
			node[0] = coordinate
			node.writeUInt32LE(low, 1)
			node.writeUInt32LE(high, 5)
			nodes.push(node)
			ref = 2 * nodes.length
			unique.set(key, ref)
		}
		return ref ^ sign
	}
	const root = visit(0, 0)
	const header = Buffer.alloc(52)
	header.write("BEVT")
	header[4] = 1
	header[5] = coordinates
	header.fill(identity, 8, 40)
	header.writeUInt32LE(nodes.length, 40)
	header.writeUInt32LE(1, 44)
	header.writeUInt32LE(root, 48)
	return Buffer.concat([header, ...nodes])
}
const pairFull = region(204, 2, 15)
const states = region(205, 1, 3)
const environment = region(206, 0, 1)
const a = region(204, 2, 10)
const b = region(204, 2, 12)
const mapBytes = Buffer.concat([
	Buffer.from("BEDC"),
	Buffer.from([1, 0]),
	blob(pairFull),
	blob(states),
	number(1),
	blob(a)
])
const base = Buffer.concat([blob(states), blob(environment), number(0)])
const fibre = Buffer.concat([pairFull.subarray(8, 40), Buffer.from([0]), base, base])
const facesBytes = Buffer.concat([Buffer.from("BEDC"), Buffer.from([1, 3]), fibre])
const planBytes = Buffer.concat([Buffer.from("BEDC"), Buffer.from([1, 5]), Buffer.alloc(32, 207), fibre, fibre, fibre])

test("pure Event query authoring retains types, stages, shape bounds and owned imports", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	assert.equal(v(constructed).region.field.kind, "event")
	assert.equal(v(constructed).safe.field.kind, "bool")
	assert.deepEqual(
		describeQuery(queryFromDescription(Theory, describeQuery(constructed), { id: u64, region: event, safe: bool })),
		describeQuery(constructed)
	)
	assert.deepEqual(
		describeQuery(queryFromDescription(Theory, describeQuery(missing), { region: event })),
		describeQuery(missing)
	)
	const row = v(Claim)
	assert.throws(() => EventExpr.complement(row.id as never), /Event variable/)
	// Structural variables may be mutable copies. Query snapshots must copy
	// the binder and its Event uses together, preserving reference identity.
	const copied = { ...row.a }
	const copiedQuery = query(Theory).rule((r) =>
		r.match(Claim, { a: copied }).find({ region: EventExpr.complement(copied) })
	)
	const copiedDescription = describeQuery(copiedQuery)
	assert.deepEqual(copiedDescription.ir.rules[0]?.finds, [
		{ kind: "event", expr: { kind: "not", expr: { kind: "var", var: 0 } } }
	])
	copied.label = "changed after query authoring"
	assert.deepEqual(describeQuery(copiedQuery), copiedDescription)
	const copiedImport = queryFromDescription(Theory, copiedDescription, { region: event })
	assert.deepEqual(describeQuery(copiedImport), copiedDescription)
	assert.throws(() => EventExpr.apply(16, row.a, row.b), /four bits/)
	assert.throws(() => EventExpr.exactly(-1n, row.a), /unsigned/)
	assert.throws(() => Reflect.apply(EventExpr.cardinality, undefined, [0n, 1n]), /context-bearing/)
	assert.throws(
		() => query(Theory).rule((r) => r.match(Claim, { a: row.a }).find({ value: EventExpr.apply(0, row.a, row.b) })),
		/not bound/
	)
	const expression = EventExpr.complement(row.a)
	assert.equal(snapshotData(expression), expression)
	assert.throws(
		() => query(Theory).rule((r) => r.match(Claim, row).find({ value: { ...expression } })),
		/not a find entry/
	)
	let deep = EventExpr.variable(row.a)
	for (let i = 1; i < 128; i++) deep = EventExpr.complement(deep)
	assert.throws(() => EventExpr.complement(deep), /shape/)
	let wide = EventExpr.variable(row.a)
	for (let i = 0; i < 11; i++) wide = EventExpr.and(wide, wide)
	assert.throws(() => EventTest.equal(wide, wide), /shape/)
	const source = Uint8Array.from(mapBytes)
	const captured = descriptor(source)
	source.fill(0)
	EventDescriptor.toBytes(captured).fill(0)
	assert.deepEqual(Buffer.from(EventDescriptor.toBytes(captured)), mapBytes)
	assert.equal(EventDescriptor.isDescriptor({ ...captured }), false)
	assert.ok(Result.isFailure(EventDescriptor.fromBytes(new Uint8Array(new SharedArrayBuffer(8)))))
	assert.ok(Result.isFailure(EventDescriptor.fromBytes(Buffer.from("BEVT\x01"))))
	assert.throws(() => EventExpr.image(row.a, undefined as never), /owned BEDC/)
	const built = query(Theory).rule((r) => r.match(Claim, row).find({ value: EventExpr.image(row.a, captured) }))
	const description = describeQuery(built)
	const restored = queryFromDescription(Theory, description, { value: event })
	const find = description.ir.rules[0]?.finds[0]
	assert.ok(find?.kind === "event" && find.expr.kind === "map")
	find.expr.descriptor.fill(0)
	assert.deepEqual(describeQuery(restored), describeQuery(built))
	assert.equal(nativeBindingIsLoaded(), false)
})

test("native Event heads preserve shared worlds through tests, descriptions, imported stages and Pack", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("event-query-builders"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Claim, [
						{ id: 1n, a: value(a), b: value(b) },
						{ id: 2n, a: value(region(204, 2, 5)), b: value(b) }
					])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					const row = v(Claim)
					const heads: Record<string, EventFind> = {
						not: EventExpr.complement(row.a),
						empty: EventExpr.empty(row.a),
						full: EventExpr.full(row.a),
						ite: EventExpr.ite(row.a, row.b, EventExpr.complement(row.b)),
						repeated: EventExpr.exactly(1n, row.a, row.a),
						both: EventExpr.exactly(2n, row.a, row.b),
						any: EventExpr.atLeast(1n, row.a, row.b),
						none: EventExpr.atMost(0n, row.a, row.b),
						isEmpty: EventTest.isEmpty(EventExpr.empty(row.a)),
						isFull: EventTest.isFull(EventExpr.full(row.a)),
						subset: EventTest.subset(row.a, row.b),
						equal: EventTest.equal(row.a, row.a),
						disjoint: EventTest.disjoint(row.a, EventExpr.complement(row.a)),
						covers: EventTest.covers(row.a, EventExpr.complement(row.a))
					}
					for (let mask = 0; mask < 16; mask++) heads[`mask${mask}`] = EventExpr.apply(mask, row.a, row.b)
					const q = query(Theory).rule((r) => r.match(Claim, row).find({ id: row.id, ...heads }))
					const output = yield* (yield* snapshot.execute(q, {})).collect()
					assert.equal(output.length, 2)
					for (const result of output) {
						const left = result.id === 1n ? 10 : 5
						for (const [name, answer] of Object.entries(result)) {
							if (name === "id") continue
							if (typeof answer === "boolean") {
								assert.equal(answer, name !== "subset")
								continue
							}
							assert.ok(Event.isEvent(answer))
							let expected: number
							if (name.startsWith("mask")) {
								const mask = Number(name.slice(4))
								expected = 0
								for (let world = 0; world < 4; world++)
									if (mask & (1 << ((((left >> world) & 1) << 1) | ((12 >> world) & 1)))) expected |= 1 << world
							} else
								expected =
									(
										{
											not: left ^ 15,
											empty: 0,
											full: 15,
											ite: (left & 12) | ((left ^ 15) & 3),
											repeated: 0,
											both: left & 12,
											any: left | 12,
											none: (left | 12) ^ 15
										} as Record<string, number>
									)[name] ?? -1
							assert.deepEqual(Buffer.from(Event.toBytes(answer)), region(204, 2, expected), name)
						}
					}
					const fields = Object.fromEntries(
						Object.entries(heads).map(([name, expression]) => [name, expression.kind === "event" ? event : bool])
					)
					const restored = queryFromDescription(Theory, describeQuery(q), { id: u64, ...fields })
					const encode = (rows: readonly object[]) =>
						rows.map((row) =>
							Object.fromEntries(
								Object.entries(row).map(([key, value]) => [
									key,
									Event.isEvent(value) ? Buffer.from(Event.toBytes(value)).toString("hex") : value
								])
							)
						)
					assert.deepEqual(encode(yield* (yield* snapshot.execute(restored, {})).collect()), encode(output))
					const answer = yield* (yield* snapshot.execute(missing, {})).collect()
					assert.equal(answer.length, 1)
					assert.ok(answer[0])
					assert.deepEqual(Buffer.from(Event.toBytes(answer[0].region)), b)
					const absence = query(Theory).rule((r) =>
						r.match(Claim, { id: 999n, a: row.a }).find({ value: EventExpr.empty(row.a) })
					)
					assert.deepEqual(yield* (yield* snapshot.execute(absence, {})).collect(), [])
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("captured maps and typed relation programs execute through the public builders", async () => {
	const readout = descriptor(mapBytes)
	const faces = descriptor(facesBytes)
	const plan = descriptor(planBytes)
	const row = v(Claim)
	const rel = RelationExpr.bind(row.a, faces)
	const predicate = EventExpr.image(row.a, readout)
	const heads = {
		pullback: EventExpr.pullback(predicate, readout),
		image: predicate,
		universal: EventExpr.universalImage(row.a, readout),
		nonvacuous: EventExpr.nonvacuousImage(row.a, readout),
		possible: EventExpr.possible(row.a, readout),
		guaranteed: EventExpr.guaranteed(row.a, readout),
		region: EventExpr.region(rel),
		domain: EventExpr.domain(rel),
		range: EventExpr.range(rel),
		converse: EventExpr.region(RelationExpr.converse(rel)),
		complement: EventExpr.region(RelationExpr.complement(rel)),
		identity: EventExpr.region(RelationExpr.identity(faces)),
		test: EventExpr.region(RelationExpr.test(predicate, faces)),
		star: EventExpr.region(RelationExpr.star(rel, plan)),
		compose: EventExpr.region(RelationExpr.compose(rel, rel, plan)),
		leftResidual: EventExpr.region(RelationExpr.leftResidual(rel, rel, plan)),
		rightResidual: EventExpr.region(RelationExpr.rightResidual(rel, rel, plan)),
		may: EventExpr.may(rel, predicate),
		all: EventExpr.all(rel, predicate),
		must: EventExpr.must(rel, predicate),
		post: EventExpr.post(rel, predicate),
		relationXor: EventExpr.region(RelationExpr.xor(rel, RelationExpr.identity(faces)))
	}
	const q = query(Theory).rule((r) => r.match(Claim, row).find(heads))
	const restored = queryFromDescription(
		Theory,
		describeQuery(q),
		Object.fromEntries(Object.keys(heads).map((name) => [name, event]))
	)
	assert.deepEqual(describeQuery(restored), describeQuery(q))
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: Event | undefined
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("event-query-relations"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Claim, [{ id: 1n, a: value(a), b: value(b) }])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					const expected: Record<keyof typeof heads, [number, number, number]> = {
						pullback: [204, 2, 10],
						image: [205, 1, 2],
						universal: [205, 1, 2],
						nonvacuous: [205, 1, 2],
						possible: [204, 2, 10],
						guaranteed: [204, 2, 10],
						region: [204, 2, 10],
						domain: [205, 1, 2],
						range: [205, 1, 3],
						converse: [204, 2, 10],
						complement: [204, 2, 5],
						identity: [204, 2, 9],
						test: [204, 2, 8],
						star: [204, 2, 11],
						compose: [204, 2, 10],
						leftResidual: [204, 2, 15],
						rightResidual: [204, 2, 11],
						may: [205, 1, 2],
						all: [205, 1, 1],
						must: [205, 1, 0],
						post: [205, 1, 3],
						relationXor: [204, 2, 3]
					}
					for (const program of [q, restored]) {
						const answer = yield* (yield* snapshot.execute(program, {})).collect()
						assert.equal(answer.length, 1)
						assert.ok(answer[0])
						for (const [name, output] of Object.entries(answer[0])) {
							const [identity, coordinates, bits] = expected[name as keyof typeof heads]
							assert.deepEqual(Buffer.from(Event.toBytes(output)), region(identity, coordinates, bits), name)
							retained = output
						}
					}
					// Both operands participate even when the truth table discards them.
					const invalid = query(Theory).rule((r) =>
						r.match(Claim, row).find({ value: EventExpr.apply(0, row.a, predicate) })
					)
					const failure = yield* Effect.result(snapshot.execute(invalid, {}))
					assert.ok(Result.isFailure(failure))
					assert.equal(failure.failure.reason._tag, "Engine")
					// A Fibre payload cannot masquerade as a map, including on an absent body.
					const wrongRole = query(Theory).rule((r) =>
						r.match(Claim, { id: 99n, a: row.a }).find({ value: EventExpr.image(row.a, faces) })
					)
					assert.ok(Result.isFailure(yield* Effect.result(snapshot.prepare(wrongRole))))
					assert.equal((yield* (yield* snapshot.execute(q, {})).collect()).length, 1)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
	assert.ok(retained)
	assert.ok(Event.toBytes(retained).length > 0)
})

test("Event errors retain complete logical fault sets across stages, prepared queries and runtime release", async () => {
	const faultyRule = (r: QueryRuleScope<typeof Theory.relations, typeof Theory.classes>) => {
		const row = v(Claim)
		return r.match(Claim, row).find({
			ignored: EventExpr.apply(0, row.a, row.b),
			union: EventExpr.or(row.a, row.b)
		})
	}
	const faulty = query(Theory).rule(faultyRule).rule(faultyRule)
	const staged = query(Theory).rule((r) => {
		const row = v(faulty)
		return r.match(faulty, row).find(row)
	})
	const signatures = (faults: readonly EventOperandFault[]) =>
		faults.map((fault) => ({
			...fault,
			expectedSpace: Buffer.from(fault.expectedSpace).toString("hex"),
			offendingValue: Buffer.from(fault.offendingValue).toString("hex")
		}))
	let retained: readonly EventOperandFault[] | undefined
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("event-query-diagnostics"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Claim, [
						{ id: 1n, a: value(a), b: value(region(208, 2, 0)) },
						{ id: 2n, a: value(a), b: value(region(208, 2, 12)) }
					])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					const prepared = yield* snapshot.prepare(faulty)
					const results = [
						yield* Effect.result(snapshot.execute(faulty, {})),
						yield* Effect.result(prepared.execute({})),
						yield* Effect.result(snapshot.execute(staged, {}))
					]
					for (const [index, result] of results.entries()) {
						assert.ok(Result.isFailure(result))
						const reason = result.failure.reason
						assert.equal(reason._tag, "Engine")
						assert.ok(reason._tag === "Engine" && reason.eventFaults)
						assert.equal(reason.kind, "event")
						const faults = reason.eventFaults
						assert.equal(faults.length, 8, "two written rules, two heads, two distinct offending values")
						for (const fault of faults) {
							assert.equal(fault.stage, index === 2 ? 0 : undefined)
							assert.equal(fault.operand, 1)
							assert.equal(fault.variable, 2)
							assert.equal(fault.category, "SpaceMismatch")
							assert.deepEqual(Buffer.from(fault.expectedSpace), pairFull)
							assert.ok([0, 12].some((bits) => Buffer.from(fault.offendingValue).equals(region(208, 2, bits))))
							assert.ok(Result.isSuccess(Event.fromBytes(fault.offendingValue)))
						}
						assert.deepEqual(new Set(faults.map((f) => `${f.rule}/${f.find}`)), new Set(["0/0", "0/1", "1/0", "1/1"]))
						if (index === 0) retained = faults
						if (index === 1) {
							assert.ok(retained)
							assert.deepEqual(signatures(faults), signatures(retained))
						}
					}
					const wellShaped = descriptor(Buffer.from("BEDC\x01"))
					const row = v(Claim)
					const badImport = query(Theory).rule((r) =>
						r.match(Claim, row).find({ value: EventExpr.image(row.a, wellShaped) })
					)
					const malformed = yield* Effect.result(snapshot.prepare(badImport))
					assert.ok(Result.isFailure(malformed))
					assert.ok(malformed.failure.reason._tag === "Engine")
					assert.equal(
						malformed.failure.reason.eventFaults,
						undefined,
						"a decoder error never labels a partial stage complete"
					)
					const usable = query(Theory).rule((r) => r.match(Claim, row).find({ value: EventExpr.complement(row.a) }))
					assert.equal((yield* (yield* snapshot.execute(usable, {})).collect()).length, 1)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
	assert.ok(retained)
	assert.equal(retained.length, 8)
	assert.deepEqual(Buffer.from(retained[0]?.expectedSpace ?? []), pairFull)
})

test("fixed-point authoring closes lexical handles without accidental capture", () => {
	const scope = value(pairFull)
	const row = v(Claim)
	let escaped: EventExpr | undefined
	let inner: EventExpr | undefined
	const nested = EventExpr.least(scope, (x) => {
		escaped = x
		inner = EventExpr.greatest(scope, (y) => EventExpr.or(x, y))
		return EventExpr.and(row.a, inner)
	})
	const q = query(Theory).rule((r) => r.match(Claim, row).find({ nested }))
	const description = describeQuery(q)
	const expr = description.ir.rules[0]?.finds[0]
	assert.ok(expr?.kind === "event" && expr.expr.kind === "fixed" && expr.expr.expr.kind === "apply")
	const closed = expr.expr.expr.right
	assert.ok(closed.kind === "fixed" && closed.expr.kind === "apply")
	assert.deepEqual(closed.expr.left, { kind: "bound", depth: 1 })
	assert.deepEqual(closed.expr.right, { kind: "bound", depth: 0 })
	const restored = queryFromDescription(Theory, description, { nested: event })
	expr.expr.scope.fill(0)
	assert.deepEqual(describeQuery(restored), describeQuery(q))
	assert.equal(snapshotData(nested), nested)
	assert.ok(escaped && inner)
	for (const dangling of [escaped, inner]) {
		assert.throws(() => query(Theory).rule((r) => r.match(Claim, row).find({ dangling })), /escaped/)
		const recaptured = EventExpr.least(scope, () => dangling)
		assert.throws(() => query(Theory).rule((r) => r.match(Claim, row).find({ recaptured })), /escaped/)
	}
	assert.throws(() => EventExpr.least({ ...scope }, (x) => x), /owned Event/)
	assert.throws(() => EventExpr.least(scope, (() => Promise.resolve(row.a)) as never), /Event variable/)
	let deep = EventExpr.variable(row.a)
	for (let i = 1; i < 128; i++) {
		const previous = deep
		deep = EventExpr.least(scope, () => previous)
	}
	assert.throws(() => EventExpr.least(scope, () => deep), /shape/)
})

test("query binders compose with nested modalities, Star, stages, tests and Pack", async () => {
	const row = v(Claim)
	const scope = value(pairFull)
	const faces = descriptor(facesBytes)
	const plan = descriptor(planBytes)
	const rel = RelationExpr.bind(row.a, faces)
	const g = EventExpr.image(row.b, descriptor(mapBytes))
	const q = query(Theory).rule((r) =>
		r.match(Claim, row).find({
			id: row.id,
			// Both nesting orders require retaining the outer predicate by identity.
			least: EventExpr.least(scope, (x) => EventExpr.greatest(scope, (y) => EventExpr.or(x, y))),
			greatest: EventExpr.greatest(scope, (x) => EventExpr.least(scope, (y) => EventExpr.and(x, y))),
			captured: EventExpr.least(scope, (x) => EventExpr.or(row.a, x)),
			closure: EventExpr.least(scope, (x) => EventExpr.or(x, EventExpr.region(RelationExpr.star(rel, plan)))),
			reach: EventExpr.least(value(states), (x) => EventExpr.or(g, EventExpr.may(rel, x))),
			nested: EventExpr.greatest(value(states), (x) =>
				EventExpr.least(value(states), (y) =>
					EventExpr.or(EventExpr.and(g, EventExpr.may(rel, x)), EventExpr.may(rel, y))
				)
			),
			test: EventTest.isFull(EventExpr.greatest(scope, (x) => x))
		})
	)
	const restored = queryFromDescription(Theory, describeQuery(q), {
		id: u64,
		least: event,
		greatest: event,
		captured: event,
		closure: event,
		reach: event,
		nested: event,
		test: bool
	})
	const packed = query(Theory).rule((r) => {
		const row = v(q)
		return r.match(q, row).find({ union: r.pack(row.captured) })
	})
	const final = query(Theory).rule((r) => {
		const row = v(packed)
		return r.match(packed, row).find({ isFull: EventTest.isFull(row.union) })
	})
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: Event | undefined
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("event-query-binders"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Claim, [
						{ id: 1n, a: value(a), b: value(b) },
						{ id: 2n, a: value(region(204, 2, 5)), b: value(b) }
					])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					for (const program of [q, restored]) {
						const output = yield* (yield* snapshot.execute(program, {})).collect()
						assert.equal(output.length, 2)
						for (const result of output) {
							const bits = result.id === 1n ? 10 : 5
							for (const [name, answer] of Object.entries(result)) {
								if (name === "id") continue
								if (name === "test") {
									assert.equal(answer, true)
									continue
								}
								const expected: Record<string, number> = {
									least: 15,
									greatest: 0,
									captured: bits,
									closure: bits | 9,
									reach: 3,
									nested: result.id === 1n ? 2 : 1
								}
								const state = name === "reach" || name === "nested"
								const expectedBits = expected[name]
								assert.notEqual(expectedBits, undefined)
								assert.deepEqual(
									Buffer.from(Event.toBytes(answer as Event)),
									region(state ? 205 : 204, state ? 1 : 2, expectedBits as number),
									name
								)
							}
							retained = result.captured
						}
					}
					assert.deepEqual(yield* (yield* snapshot.execute(final, {})).collect(), [{ isFull: true }])
					for (const bad of [
						EventExpr.least(value(a), (x) => x),
						EventExpr.least(scope, (x) => EventExpr.complement(x)),
						EventExpr.least(scope, (x) => EventExpr.greatest(scope, () => EventExpr.complement(x)))
					]) {
						const absent = query(Theory).rule((r) => r.match(Claim, { id: 99n }).find({ bad }))
						assert.ok(Result.isFailure(yield* Effect.result(snapshot.prepare(absent))))
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
	assert.ok(retained)
	assert.ok(Event.toBytes(retained).length > 0)
})
