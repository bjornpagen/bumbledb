import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	ChangeSet,
	Db,
	Event,
	EventDescriptor,
	EventExpr,
	ExactRational,
	type ExpectationObservation,
	event,
	FiniteFunction,
	NativeRuntime,
	type ProbabilityObservation,
	query,
	relation,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const identity = (n: number) => new Uint8Array(32).fill(n)
const rational = ExactRational.fraction
const text = ExactRational.toString
const fail = <E, R>(value: Effect.Effect<unknown, E, R>) => Effect.flip(value)

function typePins(value: Event, scalar: ExactRational, fn: FiniteFunction) {
	// @ts-expect-error Numeric JavaScript values are not exact rational carriers.
	FiniteFunction.constant(value, 0.5)
	// @ts-expect-error A function is not a stored Event.
	Event.and(value, fn)
	// @ts-expect-error Probability observations need explicit evidence.
	Event.probability(value)
	FiniteFunction.constant(value, scalar)
}
void typePins

test("exact scalar/function transport and lazy source programs do not load the addon", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const ber = Buffer.from("BERA\x01")
	const value = Result.getOrThrow(ExactRational.fromBytes(ber))
	ber.fill(0)
	assert.deepEqual(Buffer.from(ExactRational.toBytes(value)), Buffer.from("BERA\x01"))
	ExactRational.toBytes(value).fill(0)
	assert.equal(ExactRational.isExactRational({ ...value }), false)
	const fn = Result.getOrThrow(FiniteFunction.fromBytes(Buffer.from("BESC\x01\x00")))
	assert.equal(FiniteFunction.isFiniteFunction(fn), true)
	assert.equal(FiniteFunction.isFiniteFunction({ ...fn }), false)
	assert.ok(Result.isFailure(FiniteFunction.fromBytes(Buffer.from("BESC\x01\x01"))))
	assert.ok(Result.isFailure(ExactRational.fromBytes(new Uint8Array(new SharedArrayBuffer(5)))))
	ExactRational.decimal("0.1")
	FiniteFunction.describe(fn)
	FiniteFunction.designate(fn)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("exact native arithmetic preserves decimal/binary intent, canonical values and failure modes", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const tenth = yield* ExactRational.decimal("0.1")
				assert.equal(yield* text(tenth), "1/10")
				const binary = yield* ExactRational.binary64(0.1)
				assert.equal(yield* text(binary), "3602879701896397/36028797018963968")
				assert.equal(yield* ExactRational.equal(tenth, binary), false)
				assert.equal(yield* ExactRational.compare(tenth, binary), -1)
				assert.equal(yield* ExactRational.compare(binary, tenth), 1)
				assert.equal(yield* ExactRational.compare(tenth, tenth), 0)
				const half = yield* rational(-2n, -4n)
				assert.equal(yield* text(half), "1/2")
				const u64 = (n: number) => {
					const bytes = Buffer.alloc(8)
					bytes.writeBigUInt64LE(BigInt(n))
					return bytes
				}
				assert.deepEqual(
					Buffer.from(ExactRational.toBytes(half)),
					Buffer.concat([Buffer.from("BERA\x01\x00"), u64(1), Buffer.from([1]), u64(1), Buffer.from([2])])
				)
				assert.equal(yield* text(yield* ExactRational.add(half, tenth)), "3/5")
				assert.equal(yield* text(yield* ExactRational.subtract(tenth, half)), "-2/5")
				assert.equal(yield* text(yield* ExactRational.multiply(tenth, half)), "1/20")
				assert.equal(yield* text(yield* ExactRational.divide(half, tenth)), "5")
				assert.equal(yield* text(yield* ExactRational.decimal("-1.25e2")), "-125")
				const zero = yield* rational(0n)
				assert.equal(yield* ExactRational.isZero(zero), true)
				assert.equal(yield* ExactRational.isProbability(half), true)
				const negative = yield* rational(-1n)
				assert.equal(yield* ExactRational.isNegative(negative), true)
				assert.equal(yield* ExactRational.isProbability(negative), false)
				const large = yield* rational((1n << 200n) + 1n, 3n)
				assert.equal(
					yield* ExactRational.equal(
						large,
						yield* ExactRational.validate(Result.getOrThrow(ExactRational.fromBytes(ExactRational.toBytes(large))))
					),
					true
				)
				for (const operation of [
					rational(1n, 0n),
					ExactRational.divide(half, zero),
					ExactRational.decimal("1e1000000"),
					ExactRational.binary64(Number.NaN),
					ExactRational.binary64(Number.POSITIVE_INFINITY),
					ExactRational.validate(Result.getOrThrow(ExactRational.fromBytes(Buffer.from("BERA\x01"))))
				])
					assert.equal((yield* fail(operation)).reason._tag, "Engine")
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("finite functions merge equal cells, add signed overlaps and sum copied readouts exactly once", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const s = yield* Event.space(identity(230), 2n)
				const a = yield* Event.coordinate(s, 0n)
				const b = yield* Event.coordinate(s, 1n)
				const two = yield* rational(2n)
				const negative = yield* rational(-1n)
				const fa = yield* FiniteFunction.new(s, [{ region: a, value: two }])
				const fb = yield* FiniteFunction.new(s, [{ region: b, value: negative }])
				const sum = yield* FiniteFunction.add(fa, fb)
				const product = yield* FiniteFunction.multiply(fa, fb)
				for (let world = 0n; world < 4n; world++) {
					const av = world & 1n ? 2n : 0n
					const bv = world & 2n ? -1n : 0n
					assert.equal(yield* text(yield* FiniteFunction.at(sum, world)), `${av + bv}`)
					assert.equal(yield* text(yield* FiniteFunction.at(product, world)), `${av * bv}`)
				}
				const merged = yield* FiniteFunction.new(s, [
					{ region: a, value: two },
					{ region: yield* Event.complement(a), value: two }
				])
				const data = yield* FiniteFunction.describe(merged)
				assert.equal(data.pieces.length, 1)
				assert.equal(yield* FiniteFunction.equivalent(merged, yield* FiniteFunction.new(data.space, data.pieces)), true)
				assert.equal(yield* FiniteFunction.equivalent(merged, yield* FiniteFunction.alignTo(merged, s)), true)
				assert.equal(yield* FiniteFunction.isNonnegative(sum), false)
				assert.equal(yield* FiniteFunction.isZero(yield* FiniteFunction.new(s, [])), true)
				const t = yield* Event.space(identity(231), 2n)
				const copy = yield* EventDescriptor.admit({ kind: "map", map: { source: s, target: t, readouts: [a, a] } })
				const counts = yield* FiniteFunction.pushforward(yield* FiniteFunction.constant(s, yield* rational(1n)), copy)
				for (let w = 0n; w < 4n; w++)
					assert.equal(yield* text(yield* FiniteFunction.at(counts, w)), w === 0n || w === 3n ? "2" : "0")
				const pulled = yield* FiniteFunction.pullback(counts, copy)
				assert.equal(yield* FiniteFunction.equivalent(pulled, merged), true)
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("law designation is exact and retains symbolic and zero-mass possibilities", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: ProbabilityObservation | undefined
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const raw = yield* Event.space(identity(232), 62n)
				const density = yield* FiniteFunction.constant(raw, yield* rational(1n, 1n << 62n))
				const source = yield* FiniteFunction.designate(density)
				assert.equal(yield* Event.count(source), 1n << 62n)
				assert.equal(yield* text(yield* Event.mass(source)), "1")
				assert.equal(yield* text(yield* Event.mass(yield* Event.coordinate(source, 61n))), "1/2")
				assert.equal((yield* FiniteFunction.describe(yield* FiniteFunction.density(source))).pieces.length, 1)
				const small = yield* Event.space(identity(233), 1n)
				const one = yield* rational(1n)
				const measured = yield* FiniteFunction.designate(
					yield* FiniteFunction.new(small, [{ region: yield* Event.coordinate(small, 0n), value: one }])
				)
				const positive = yield* Event.coordinate(measured, 0n)
				const impossible = yield* Event.complement(positive)
				assert.equal(yield* Event.count(impossible), 1n)
				retained = yield* Event.probability(positive, impossible)
				assert.equal(retained.value, null)
				assert.equal(yield* text(retained.evidenceMass), "0")
				assert.equal(yield* Event.equal(retained.given, impossible), true)
				const expectation = yield* FiniteFunction.expectation(yield* FiniteFunction.constant(measured, one), impossible)
				assert.equal(expectation.value, null)
				assert.equal((yield* fail(Event.mass(small))).reason._tag, "Engine")
				assert.equal((yield* fail(Event.probability(positive, yield* Event.empty(small)))).reason._tag, "Engine")
				assert.equal(
					(yield* fail(FiniteFunction.designate(yield* FiniteFunction.constant(small, one)))).reason._tag,
					"Engine",
					"invalid total is not rescaled"
				)
				assert.equal(
					(yield* fail(FiniteFunction.designate(yield* FiniteFunction.constant(small, yield* rational(-1n))))).reason
						._tag,
					"Engine"
				)
			})
		)
	} finally {
		await runtime.dispose()
	}
	assert.ok(retained)
	assert.ok(Event.toBytes(retained.given).length > 5)
	assert.ok(ExactRational.toBytes(retained.numerator).length > 5)
})

test("Coup source algebra preserves shared worlds, exact posteriors and signed utility through storage", async () => {
	const Observation = relation("Observation", { id: u64, target: event, given: event })
	const Theory = schema("CoupSource", { Observation }, [])
	const q = query(Theory).rule((r) => {
		const row = v(Observation)
		return r
			.match(Observation, row)
			.find({ id: row.id, target: EventExpr.and(row.target, row.given), given: row.given })
	})
	const path = storeDir("source-coup")
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: ExpectationObservation | undefined
	let observation: ProbabilityObservation | undefined
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const base = yield* Event.space(identity(234), 2n)
					const bob = yield* Event.coordinate(base, 0n)
					const cleo = yield* Event.coordinate(base, 1n)
					const cells = [
						yield* Event.and(yield* Event.complement(bob), yield* Event.complement(cleo)),
						yield* Event.and(bob, yield* Event.complement(cleo)),
						yield* Event.and(yield* Event.complement(bob), cleo),
						yield* Event.and(bob, cleo)
					]
					const pieces = []
					for (const [i, n] of [36n, 19n, 19n, 4n].entries()) {
						assert.ok(cells[i])
						pieces.push({ region: cells[i], value: yield* rational(n, 78n) })
					}
					const prior = yield* FiniteFunction.designate(yield* FiniteFunction.new(base, pieces))
					const extension = yield* Event.space(identity(235), 3n)
					const b = yield* Event.coordinate(extension, 0n)
					const c = yield* Event.coordinate(extension, 1n)
					const tax = yield* Event.coordinate(extension, 2n)
					const parent = yield* EventDescriptor.admit({
						kind: "map",
						map: { source: extension, target: prior, readouts: [b, c] }
					})
					const likely = yield* Event.equivalence(b, tax)
					const channel = yield* FiniteFunction.new(extension, [
						{ region: likely, value: yield* rational(4n, 5n) },
						{ region: yield* Event.complement(likely), value: yield* rational(1n, 5n) }
					])
					const priorDensity = yield* FiniteFunction.pullback(yield* FiniteFunction.density(prior), parent)
					const joint = yield* FiniteFunction.designate(yield* FiniteFunction.multiply(priorDensity, channel))
					const jointBob = yield* Event.coordinate(joint, 0n)
					const jointCleo = yield* Event.coordinate(joint, 1n)
					const jointTax = yield* Event.coordinate(joint, 2n)
					observation = yield* Event.probability(jointBob, jointTax)
					assert.ok(observation.value)
					assert.equal(yield* text(observation.value), "92/147")
					assert.equal(yield* text(observation.evidenceMass), "49/130")
					const cleoGiven = yield* Event.probability(jointCleo, jointTax)
					assert.ok(cleoGiven.value)
					assert.equal(yield* text(cleoGiven.value), "5/21")
					const payoff = yield* FiniteFunction.new(joint, [
						{ region: jointBob, value: yield* rational(-1n) },
						{ region: yield* Event.complement(jointBob), value: yield* rational(1n) }
					])
					retained = yield* FiniteFunction.expectation(payoff, jointTax)
					assert.ok(retained.value)
					assert.equal(yield* text(retained.value), "-37/147")
					const db = yield* Db.create(path, Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Observation, [
						{ id: 1n, target: jointBob, given: jointTax },
						{ id: 2n, target: jointCleo, given: jointTax }
					])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
	assert.ok(retained && observation)
	const next = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await next.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.open(path, Theory)
					const rows = yield* (yield* (yield* db.snapshot()).execute(q, {})).collect()
					assert.equal(rows.length, 2)
					for (const row of rows) {
						const result = yield* Event.probability(row.target, row.given)
						assert.ok(result.value)
						assert.equal(yield* text(result.value), row.id === 1n ? "92/147" : "5/21")
					}
					assert.ok(retained)
					const repeated = yield* FiniteFunction.expectation(retained.function, retained.given)
					assert.ok(repeated.value)
					assert.equal(yield* text(repeated.value), "-37/147")
				})
			)
		)
	} finally {
		await next.dispose()
	}
})

test("finite function admission checks all contexts and shapes before zero shortcuts", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const space = yield* Event.space(identity(236), 1n)
				const region = yield* Event.coordinate(space, 0n)
				const foreign = yield* Event.space(identity(237), 1n)
				const zero = yield* rational(0n)
				const fn = yield* FiniteFunction.new(space, [])
				for (const operation of [
					FiniteFunction.new(space, [
						{ region: space, value: zero },
						{ region, value: zero }
					]),
					FiniteFunction.new(space, [{ region: yield* Event.empty(foreign), value: zero }]),
					FiniteFunction.constant(region, zero),
					FiniteFunction.multiply(fn, yield* FiniteFunction.new(foreign, [])),
					FiniteFunction.alignTo(fn, foreign),
					FiniteFunction.at(fn, 2n)
				])
					assert.equal((yield* fail(operation)).reason._tag, "Engine")
				let reads = 0
				const piece = {
					get region() {
						reads++
						return region
					},
					value: zero
				}
				assert.equal((yield* fail(FiniteFunction.new(space, [piece]))).reason._tag, "InvalidArgument")
				assert.equal(reads, 0)
				assert.equal(
					(yield* fail(FiniteFunction.new(space, [{ region, value: zero, extra: true } as never]))).reason._tag,
					"InvalidArgument"
				)
				assert.equal((yield* fail(FiniteFunction.new(space, new Array(1)))).reason._tag, "InvalidArgument")
				assert.equal(
					(yield* fail(FiniteFunction.new(space, Array(2048).fill({ region, value: zero })))).reason._tag,
					"InvalidArgument"
				)
				const handle = yield* runtimeHandle()
				const s = Event.toBytes(space)
				const v = ExactRational.toBytes(zero)
				const owned = yield* nativeOperationWith(
					"source.raw",
					(cb) => {
						const op = dbNative.runtimeEventSource(handle, "constant", [s, v], 0n, cb)
						s.fill(0)
						v.fill(0)
						return op
					},
					dbNative.runtimeBytesTake,
					(bytes) => Result.getOrThrow(FiniteFunction.fromBytes(bytes))
				)
				assert.equal(yield* FiniteFunction.isZero(owned), true)
				const malformed = Result.getOrThrow(FiniteFunction.fromBytes(Buffer.from("BESC\x01\x00")))
				assert.equal((yield* fail(FiniteFunction.validate(malformed))).reason._tag, "Engine")
				const detached = new Uint8Array(10)
				structuredClone(detached, { transfer: [detached.buffer] })
				for (const input of [
					new Uint8Array(new SharedArrayBuffer(10)),
					detached,
					new Uint8Array(16 * 1024 * 1024 + 1)
				]) {
					assert.ok(
						Result.isFailure(
							yield* Effect.result(
								nativeOperationWith(
									"source.raw",
									(cb) => dbNative.runtimeEventSource(handle, "validate", [input], 0n, cb),
									dbNative.runtimeBytesTake,
									(bytes) => bytes
								)
							)
						)
					)
					assert.ok(
						Result.isFailure(
							yield* Effect.result(
								nativeOperationWith(
									"exact.raw",
									(cb) => dbNative.runtimeExactRational(handle, "validate", [input], 0, cb),
									dbNative.runtimeBytesTake,
									(bytes) => bytes
								)
							)
						)
					)
				}
				let registered = false
				const refusal = yield* fail(
					nativeOperationWith(
						"source.raw",
						(cb) => {
							const op = dbNative.runtimeEventSource(handle, "validate", [FiniteFunction.toBytes(malformed)], 0n, cb)
							registered = true
							return op
						},
						dbNative.runtimeBytesTake,
						(bytes) => bytes
					)
				)
				assert.equal(registered, true)
				assert.equal(refusal.reason._tag, "Engine")
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("late cancelled source observations are reclaimed without delivering a partial result", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventSource
	try {
		const space = await runtime.runPromise(
			Effect.gen(function* () {
				const s = yield* Event.space(identity(238), 0n)
				return yield* FiniteFunction.designate(yield* FiniteFunction.constant(s, yield* rational(1n)))
			})
		)
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventSource = (handle, operation, inputs, argument, callback) =>
			original(handle, operation, inputs, argument, () => completed.resolve(callback))
		const fiber = runtime.runFork(Event.probability(space, space))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeEventSource = original
		assert.ok((await runtime.runPromise(Event.probability(space, space))).value)
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeEventSource = original
		await runtime.dispose()
	}
})
