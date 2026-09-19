import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	ChangeSet,
	Db,
	Event,
	EventExpr,
	ExactPolynomial,
	ExactRational,
	event,
	FiniteFunction,
	NativeRuntime,
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
const fail = <E, R>(effect: Effect.Effect<unknown, E, R>) => Effect.flip(effect)
function typePins(poly: ExactPolynomial, scalar: ExactRational, value: Event) {
	// @ts-expect-error A polynomial is not a stored Event.
	Event.and(value, poly)
	// @ts-expect-error Rational coefficients must be exact owned values.
	ExactPolynomial.constant(0.5)
	// @ts-expect-error Substitution requires expressions, not evaluated scalars.
	ExactPolynomial.substitute(poly, [{ parameter: identity(1), value: scalar }])
	// @ts-expect-error Evaluation requires rational assignments.
	ExactPolynomial.evaluate(poly, [{ parameter: identity(1), value: poly }])
}
void typePins

test("polynomial transport is owned and pure import stays lazy", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const bytes = Buffer.from("BEPL\x01")
	const poly = Result.getOrThrow(ExactPolynomial.fromBytes(bytes))
	bytes.fill(0)
	assert.deepEqual(Buffer.from(ExactPolynomial.toBytes(poly)), Buffer.from("BEPL\x01"))
	ExactPolynomial.toBytes(poly).fill(0)
	assert.equal(ExactPolynomial.isExactPolynomial({ ...poly }), false)
	assert.ok(Result.isFailure(ExactPolynomial.fromBytes(Buffer.from("BEPL\x02"))))
	assert.ok(Result.isFailure(ExactPolynomial.fromBytes(new Uint8Array(new SharedArrayBuffer(5)))))
	ExactPolynomial.parameter(identity(1))
	ExactPolynomial.describe(poly)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("named polynomial arithmetic, normalization and simultaneous substitution survive runtime release", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: ExactPolynomial | undefined
	try {
		retained = await runtime.runPromise(
			Effect.gen(function* () {
				const p = yield* ExactPolynomial.parameter(identity(1))
				const q = yield* ExactPolynomial.parameter(identity(2))
				const one = yield* rational(1n)
				const two = yield* rational(2n)
				const negative = yield* rational(-1n)
				const zero = yield* rational(0n)
				const constant = yield* ExactPolynomial.constant(two)
				const expression = yield* ExactPolynomial.add(p, yield* ExactPolynomial.multiply(constant, q))
				const normalized = yield* ExactPolynomial.fromTerms([
					{ coefficient: two, powers: [{ parameter: identity(2), exponent: 1n }] },
					{ coefficient: two, powers: [{ parameter: identity(1), exponent: 1n }] },
					{ coefficient: negative, powers: [{ parameter: identity(1), exponent: 1n }] },
					{ coefficient: zero, powers: [{ parameter: identity(3), exponent: 8n }] }
				])
				assert.equal(yield* ExactPolynomial.equal(expression, normalized), true)
				assert.deepEqual(ExactPolynomial.toBytes(expression), ExactPolynomial.toBytes(normalized))
				assert.equal(yield* ExactPolynomial.isZero(yield* ExactPolynomial.subtract(expression, normalized)), true)
				assert.equal(yield* ExactPolynomial.isZero(yield* ExactPolynomial.zero()), true)
				const cube = yield* ExactPolynomial.fromTerms([
					{
						coefficient: one,
						powers: [
							{ parameter: identity(1), exponent: 1n },
							{ parameter: identity(1), exponent: 2n },
							{ parameter: identity(2), exponent: 0n }
						]
					}
				])
				assert.equal(yield* ExactPolynomial.equal(cube, yield* ExactPolynomial.pow(p, 3n)), true)
				assert.equal(
					yield* ExactPolynomial.equal(yield* ExactPolynomial.pow(cube, 0n), yield* ExactPolynomial.constant(one)),
					true
				)
				const description = yield* ExactPolynomial.describe(expression)
				assert.equal(Object.isFrozen(description), true)
				assert.equal(description.length, 2)
				assert.equal(yield* ExactPolynomial.equal(expression, yield* ExactPolynomial.fromTerms(description)), true)
				assert.ok(description[0]?.powers[0])
				description[0].powers[0].parameter.fill(255)
				assert.equal(yield* ExactPolynomial.equal(expression, normalized), true)
				const swapped = yield* ExactPolynomial.substitute(expression, [
					{ parameter: identity(1), value: q },
					{ parameter: identity(2), value: p }
				])
				assert.equal(
					yield* text(
						yield* ExactPolynomial.evaluate(swapped, [
							{ parameter: identity(1), value: yield* rational(1n, 3n) },
							{ parameter: identity(2), value: yield* rational(1n, 4n) }
						])
					),
					"11/12"
				)
				// Polynomial identity does not silently apply a future p in {0,1} constraint.
				assert.equal(yield* ExactPolynomial.equal(p, yield* ExactPolynomial.pow(p, 2n)), false)
				return yield* ExactPolynomial.validate(expression)
			})
		)
	} finally {
		await runtime.dispose()
	}
	assert.ok(retained)
	const next = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await next.runPromise(
			Effect.gen(function* () {
				assert.ok(retained)
				assert.equal(
					yield* text(
						yield* ExactPolynomial.evaluate(retained, [
							{ parameter: identity(1), value: yield* rational(1n, 3n) },
							{ parameter: identity(2), value: yield* rational(1n, 4n) }
						])
					),
					"5/6"
				)
			})
		)
	} finally {
		await next.dispose()
	}
})

test("explicit shared Beta moments produce correlated stored Events", async () => {
	const Draws = relation("Draws", { id: u64, first: event, second: event })
	const Theory = schema("PolynomialSources", { Draws }, [])
	const both = query(Theory).rule((s) => {
		const row = v(Draws)
		return s.match(Draws, row).find({ id: row.id, first: row.first, both: EventExpr.and(row.first, row.second) })
	})
	const path = storeDir("polynomial-shared-prior")
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(path, Theory)
					const draft = yield* ChangeSet.builder(Theory)
					const one = yield* rational(1n)
					const unit = yield* ExactPolynomial.constant(one)
					const p = yield* ExactPolynomial.parameter(identity(3))
					for (const shared of [true, false]) {
						const q = shared ? p : yield* ExactPolynomial.parameter(identity(4))
						const raw = yield* Event.space(identity(shared ? 150 : 151), 2n)
						const first = yield* Event.coordinate(raw, 0n)
						const second = yield* Event.coordinate(raw, 1n)
						const outcomes = []
						for (let code = 0; code < 4; code++) {
							const a = code & 1 ? p : yield* ExactPolynomial.subtract(unit, p)
							const b = code & 2 ? q : yield* ExactPolynomial.subtract(unit, q)
							let density = yield* ExactPolynomial.integrateBeta(
								yield* ExactPolynomial.multiply(a, b),
								identity(3),
								one,
								one
							)
							if (!shared) density = yield* ExactPolynomial.integrateBeta(density, identity(4), one, one)
							outcomes.push({
								region: yield* Event.and(
									code & 1 ? first : yield* Event.complement(first),
									code & 2 ? second : yield* Event.complement(second)
								),
								value: yield* ExactPolynomial.evaluate(density, [])
							})
						}
						// The finite source constructor rechecks the complete joint law. The
						// full-cube priors and conditional independence are explicit here.
						const measured = yield* FiniteFunction.designate(yield* FiniteFunction.new(raw, outcomes))
						yield* draft.insert(Draws, [
							{
								id: shared ? 1n : 2n,
								first: yield* Event.coordinate(measured, 0n),
								second: yield* Event.coordinate(measured, 1n)
							}
						])
					}
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
	const next = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let rows: readonly { id: bigint; first: Event; both: Event }[] = []
	try {
		rows = await next.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.open(path, Theory)
					return yield* (yield* (yield* db.snapshot()).execute(both, {})).collect()
				})
			)
		)
		await next.runPromise(
			Effect.gen(function* () {
				assert.equal(rows.length, 2)
				for (const row of rows) {
					assert.equal(yield* text(yield* Event.mass(row.first)), "1/2")
					assert.equal(yield* text(yield* Event.mass(row.both)), row.id === 1n ? "1/3" : "1/4")
				}
			})
		)
	} finally {
		await next.dispose()
	}
	for (const row of rows) assert.ok(Event.toBytes(row.both).length > 5)
})

test("polynomial admission refuses malformed, missing, duplicate and unused invalid inputs", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const p = yield* ExactPolynomial.parameter(identity(5))
				const zero = yield* ExactPolynomial.zero()
				const one = yield* rational(1n)
				const bad = Result.getOrThrow(ExactPolynomial.fromBytes(Buffer.from("BEPL\x01")))
				for (const action of [
					ExactPolynomial.validate(bad),
					ExactPolynomial.multiply(zero, bad),
					ExactPolynomial.evaluate(p, []),
					ExactPolynomial.evaluate(p, [
						{ parameter: identity(5), value: one },
						{ parameter: identity(5), value: one }
					]),
					ExactPolynomial.substitute(p, [
						{ parameter: identity(5), value: p },
						{ parameter: identity(5), value: p }
					]),
					ExactPolynomial.substitute(zero, [{ parameter: identity(9), value: bad }]),
					ExactPolynomial.integrateBeta(p, identity(5), yield* rational(0n), one),
					ExactPolynomial.pow(p, 65537n)
				])
					assert.equal((yield* fail(action)).reason._tag, "Engine")
				for (const action of [
					ExactPolynomial.parameter(new Uint8Array(31)),
					ExactPolynomial.pow(p, -1n),
					ExactPolynomial.pow(p, 1n << 32n),
					ExactPolynomial.fromTerms(new Array(1)),
					ExactPolynomial.fromTerms(Array(65537).fill({})),
					ExactPolynomial.fromTerms([{ coefficient: one, powers: [{ parameter: identity(5), exponent: 1n << 32n }] }])
				])
					assert.equal((yield* fail(action)).reason._tag, "InvalidArgument")
				let reads = 0
				const input = {
					get coefficient() {
						reads++
						return one
					},
					powers: []
				}
				assert.equal((yield* fail(ExactPolynomial.fromTerms([input]))).reason._tag, "InvalidArgument")
				assert.equal(reads, 0)
				const handle = yield* runtimeHandle()
				const detached = new Uint8Array(8)
				structuredClone(detached, { transfer: [detached.buffer] })
				for (const input of [
					detached,
					new Uint8Array(new SharedArrayBuffer(5)),
					new Uint8Array(16 * 1024 * 1024 + 1)
				]) {
					assert.ok(
						Result.isFailure(
							yield* Effect.result(
								nativeOperationWith(
									"polynomial.raw",
									(cb) => dbNative.runtimeExactPolynomial(handle, "validate", [input], 0n, cb),
									dbNative.runtimeBytesTake,
									(bytes) => bytes
								)
							)
						)
					)
				}
				// Input buffers are copied before worker dispatch.
				const copied = ExactPolynomial.toBytes(p)
				const restored = yield* nativeOperationWith(
					"polynomial.raw",
					(cb) => {
						const op = dbNative.runtimeExactPolynomial(handle, "validate", [copied], 0n, cb)
						copied.fill(0)
						return op
					},
					dbNative.runtimeBytesTake,
					(bytes) => Result.getOrThrow(ExactPolynomial.fromBytes(bytes))
				)
				assert.equal(yield* ExactPolynomial.equal(restored, p), true)
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("late polynomial descriptions are cancelled and reclaimed before callback delivery", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeExactPolynomial
	try {
		const p = await runtime.runPromise(ExactPolynomial.parameter(identity(7)))
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeExactPolynomial = (handle, op, inputs, argument, callback) =>
			original(handle, op, inputs, argument, () => completed.resolve(callback))
		const fiber = runtime.runFork(ExactPolynomial.describe(p))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeExactPolynomial = original
		assert.equal((await runtime.runPromise(ExactPolynomial.describe(p))).length, 1)
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeExactPolynomial = original
		await runtime.dispose()
	}
})
