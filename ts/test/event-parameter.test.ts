import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	AlgebraicRoot,
	ChangeSet,
	ParameterDomain as D,
	Db,
	Event,
	ParameterFunction as F,
	FiniteFunction,
	NativeRuntime,
	ExactPolynomial as P,
	ExactRational as Q,
	query,
	ParameterRegion as R,
	relation,
	PolynomialSigns as S,
	schema,
	str,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const id = (n: number) => new Uint8Array(32).fill(n)
const fraction = Q.fraction
const fail = <E, A>(effect: Effect.Effect<unknown, E, A>) => Effect.flip(effect)
const constant = (n: bigint) => Effect.flatMap(fraction(n), P.constant)
const text = (value: Q | null) => (value === null ? Effect.succeed(null) : Q.toString(value))
function typePins(event: Event, region: R, domain: D, root: AlgebraicRoot, polynomial: P, fn: F) {
	// @ts-expect-error A region is not a stored Event.
	Event.and(event, region)
	// @ts-expect-error A function is not a stored Event.
	Event.and(event, fn)
	// @ts-expect-error A region has not established inhabitation.
	F.ratio(region, polynomial, polynomial)
	// @ts-expect-error Domain and region have distinct roles.
	R.complement(domain)
	// @ts-expect-error Irrational roots cannot be rational assignments.
	F.at(fn, root)
	// @ts-expect-error Partial parameter functions are not finite world functions.
	FiniteFunction.isZero(fn)
}
void typePins

test("parameter carriers are owned, nominal, pure and version/role checked", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const bytes = Buffer.from("BEPR\x01")
	const region = Result.getOrThrow(R.fromBytes(bytes))
	const domain = Result.getOrThrow(D.fromBytes(bytes))
	bytes.fill(0)
	assert.deepEqual(Buffer.from(R.toBytes(region)), Buffer.from("BEPR\x01"))
	R.toBytes(region).fill(0)
	assert.equal(R.isParameterRegion(domain), false)
	assert.equal(D.isParameterDomain(region), false)
	assert.equal(R.isParameterRegion({ ...region }), false)
	assert.ok(Result.isFailure(R.fromBytes(Buffer.from("BEPR\x02"))))
	assert.ok(Result.isFailure(R.fromBytes(new Uint8Array(new SharedArrayBuffer(5)))))
	const root = Result.getOrThrow(AlgebraicRoot.fromBytes(Buffer.from("BEAR\x01")))
	const fn = Result.getOrThrow(F.fromBytes(Buffer.from("BESC\x02\x00")))
	assert.equal(FiniteFunction.isFiniteFunction(fn), false)
	assert.ok(Result.isFailure(FiniteFunction.fromBytes(F.toBytes(fn))))
	assert.ok(Result.isFailure(F.fromBytes(Buffer.from("BESC\x01\x00"))))
	assert.ok(Result.isFailure(F.fromBytes(Buffer.from("BESC\x02\x01"))))
	R.describe(region)
	D.validate(domain)
	AlgebraicRoot.describe(root)
	F.describe(fn)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("all region Boolean masks preserve exact open endpoints and disconnected sets", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const p = yield* P.parameter(id(1))
				const shifted = yield* P.subtract(p, yield* constant(1n))
				const nonnegative = yield* R.whereSign(id(1), p, S.nonNegative)
				const belowOne = yield* R.whereSign(id(1), shifted, S.negative)
				for (let mask = 0; mask < 16; mask++) {
					const result = yield* R.apply(BigInt(mask), nonnegative, belowOne)
					for (const [n, d] of [
						[-1n, 1n],
						[0n, 1n],
						[1n, 2n],
						[1n, 1n],
						[2n, 1n]
					]) {
						assert.ok(n !== undefined && d !== undefined)
						const expected = ((mask >> ((n >= 0n ? 2 : 0) | (n < d ? 1 : 0))) & 1) === 1
						assert.equal(yield* R.containsRational(result, yield* fraction(n, d)), expected)
					}
				}
				const interval = yield* R.and(nonnegative, belowOne)
				const description = yield* R.describe(interval)
				assert.deepEqual(description.membership, [false, true, true, false, false])
				assert.equal(description.boundaries.length, 2)
				assert.ok(description.boundaries[0] && description.boundaries[1])
				assert.equal(yield* AlgebraicRoot.compareRational(description.boundaries[0], yield* fraction(0n)), 0)
				assert.equal(yield* AlgebraicRoot.compareRational(description.boundaries[1], yield* fraction(1n)), 0)
				const outside = yield* R.complement(interval)
				assert.deepEqual((yield* R.describe(outside)).membership, [true, false, false, true, true])
				assert.equal(yield* R.equivalent(yield* R.complement(outside), interval), true)
				assert.equal(yield* R.included(interval, nonnegative), true)
				assert.equal(yield* R.included(nonnegative, interval), false)
				assert.equal(yield* R.isFull(yield* R.or(outside, interval)), true)
				assert.equal(yield* R.isEmpty(yield* R.xor(interval, interval)), true)
				assert.equal(
					yield* R.equivalent(
						yield* R.difference(nonnegative, belowOne),
						yield* R.whereSign(id(1), shifted, S.nonNegative)
					),
					true
				)
				assert.equal(yield* R.witness(yield* R.empty(id(1))), null)
				const witness = yield* R.witness(interval)
				assert.ok(witness)
				assert.equal(
					yield* witness.kind === "rational"
						? R.containsRational(interval, witness.value)
						: R.containsRoot(interval, witness.value),
					true
				)
				assert.equal((yield* R.witness(yield* R.full(id(1))))?.kind, "rational")
				assert.equal(yield* R.equivalent(yield* D.asRegion(yield* D.new(interval)), interval), true)
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("irrational singleton witnesses and canonical roots remain exact across owners", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: { root: AlgebraicRoot; singleton: R } | undefined
	try {
		retained = await runtime.runPromise(
			Effect.gen(function* () {
				const p = yield* P.parameter(id(2))
				const poly = yield* P.subtract(yield* P.pow(p, 2n), yield* constant(2n))
				const roots = yield* AlgebraicRoot.isolate(id(2), yield* P.pow(poly, 2n))
				assert.equal(roots.length, 2)
				const [negative, positive] = roots
				assert.ok(negative && positive)
				assert.equal(yield* AlgebraicRoot.compare(negative, positive), -1)
				const between = yield* AlgebraicRoot.rationalBetween(negative, positive)
				assert.equal(yield* AlgebraicRoot.compareRational(negative, between), -1)
				assert.equal(yield* AlgebraicRoot.compareRational(positive, between), 1)
				const intervalRoot = yield* AlgebraicRoot.fromInterval(id(2), poly, yield* fraction(1n), yield* fraction(2n))
				assert.deepEqual(AlgebraicRoot.toBytes(positive), AlgebraicRoot.toBytes(intervalRoot))
				const description = yield* AlgebraicRoot.describe(positive)
				assert.equal(yield* AlgebraicRoot.sign(positive, description.polynomial), 0)
				const formal = yield* P.parameter(description.parameter)
				assert.equal(yield* AlgebraicRoot.sign(positive, formal), 1)
				assert.equal((yield* fail(AlgebraicRoot.sign(positive, poly))).reason._tag, "Engine")
				const renamed = yield* P.substitute(poly, [{ parameter: id(2), value: formal }])
				assert.equal(yield* AlgebraicRoot.sign(positive, renamed), 0)
				assert.equal(
					yield* AlgebraicRoot.compare(
						positive,
						yield* AlgebraicRoot.fromInterval(
							description.parameter,
							description.polynomial,
							description.lower,
							description.upper
						)
					),
					0
				)
				const singleton = yield* R.and(
					yield* R.whereSign(id(2), poly, S.zero),
					yield* R.whereSign(id(2), p, S.positive)
				)
				const witness = yield* R.witness(singleton)
				assert.equal(witness?.kind, "algebraic")
				assert.ok(witness?.kind === "algebraic")
				assert.equal(yield* R.containsRoot(singleton, witness.value), true)
				assert.equal(yield* AlgebraicRoot.compare(witness.value, positive), 0)
				assert.deepEqual((yield* R.describe(singleton)).membership, [false, true, false])
				assert.equal(yield* R.containsRational(singleton, yield* fraction(141421356n, 100000000n)), false)
				assert.equal(D.isParameterDomain(yield* D.new(singleton)), true)
				return { root: yield* AlgebraicRoot.validate(positive), singleton: yield* R.validate(singleton) }
			})
		)
	} finally {
		await runtime.dispose()
	}
	assert.ok(retained)
	const next = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		assert.equal(await next.runPromise(R.containsRoot(retained.singleton, retained.root)), true)
		assert.equal(await next.runPromise(AlgebraicRoot.compare(retained.root, retained.root)), 0)
	} finally {
		await next.dispose()
	}
})

test("partial functions preserve holes, signs, arithmetic and distinct ambient domains", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const p = yield* P.parameter(id(3))
				const one = yield* constant(1n)
				const zero = yield* P.zero()
				const full = yield* R.full(id(3))
				const ambient = yield* D.new(full)
				const ratio = yield* F.ratio(ambient, p, p)
				const unit = yield* F.ratio(ambient, one, one)
				const nil = yield* F.ratio(ambient, zero, one)
				const origin = yield* fraction(0n)
				assert.equal(yield* F.at(ratio, origin), null)
				assert.equal(yield* F.equivalent(ratio, unit), false)
				assert.equal(yield* text(yield* F.at(ratio, yield* fraction(-2n))), "1")
				const annihilated = yield* F.multiply(ratio, nil)
				assert.equal(yield* F.at(annihilated, origin), null)
				assert.equal(yield* text(yield* F.at(annihilated, yield* fraction(1n))), "0")
				assert.equal(yield* F.equivalent(yield* F.subtract(ratio, ratio), annihilated), true)
				assert.equal(yield* text(yield* F.at(yield* F.add(ratio, ratio), yield* fraction(1n))), "2")
				assert.equal(yield* F.equivalent(yield* F.divide(ratio, ratio), ratio), true)
				assert.equal(yield* F.isNowhereDefined(yield* F.divide(unit, nil)), true)
				const negative = yield* R.whereSign(id(3), p, S.negative)
				const positive = yield* R.whereSign(id(3), p, S.positive)
				const fn = yield* F.pieces(ambient, [
					{ numerator: yield* constant(-1n), denominator: one, defined: negative },
					{ numerator: zero, denominator: one, defined: positive }
				])
				for (let mask = 0; mask < 8; mask++) {
					const region = yield* F.whereSign(fn, BigInt(mask))
					assert.equal(yield* R.containsRational(region, yield* fraction(-1n)), (mask & 1) !== 0)
					assert.equal(yield* R.containsRational(region, yield* fraction(1n)), (mask & 2) !== 0)
					assert.equal(yield* R.containsRational(region, origin), false)
				}
				const restricted = yield* F.restrict(fn, positive)
				assert.equal(yield* R.isFull(yield* D.asRegion(yield* F.domain(restricted))), true)
				const narrowed = yield* F.onDomain(fn, yield* D.new(positive))
				assert.equal(yield* R.equivalent(yield* D.asRegion(yield* F.domain(narrowed)), positive), true)
				assert.equal((yield* fail(F.onDomain(narrowed, ambient))).reason._tag, "Engine")
				assert.equal((yield* fail(F.equivalent(restricted, narrowed))).reason._tag, "Engine")
				assert.equal(yield* F.isNowhereDefined(yield* F.pieces(ambient, [])), true)
				const described = yield* F.describe(fn)
				assert.equal(yield* F.equivalent(fn, yield* F.pieces(described.ambient, described.pieces)), true)
				assert.equal(yield* R.equivalent(described.defined, yield* F.definedOn(fn)), true)
				const original = yield* F.describe(ratio)
				assert.ok(original.pieces[0])
				assert.equal(yield* P.equal(original.pieces[0].numerator, p), true)
				assert.equal(yield* P.equal(original.pieces[0].denominator, p), true)
				assert.equal(yield* F.equivalent(yield* F.pieces(original.ambient, original.pieces), ratio), true)
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("a joined portable conditional function keeps zero-evidence holes after reopen", async () => {
	// Ordinary string fields deliberately carry portable descriptors here. This
	// does not claim that functions are stored Events or query observation heads.
	const Observations = relation("Observations", { id: u64, descriptor: str })
	const Selected = relation("Selected", { observation: u64, label: str })
	const Theory = schema("ParameterConsumer", { Observations, Selected }, [])
	const joined = query(Theory).rule((s) => {
		const observation = v(Observations)
		const selected = v(Selected)
		return s
			.match(Observations, observation)
			.match(Selected, { observation: observation.id, label: selected.label })
			.find({ label: selected.label, descriptor: observation.descriptor })
	})
	const path = storeDir("parameter-function-consumer")
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const p = yield* P.parameter(id(4))
					const unit = yield* constant(1n)
					const domain = yield* D.new(
						yield* R.and(
							yield* R.whereSign(id(4), p, S.nonNegative),
							yield* R.whereSign(id(4), yield* P.subtract(unit, p), S.nonNegative)
						)
					)
					// Two conditionally independent trials sharing one unknown p:
					// P(H1 and H2 | p) / P(H1 | p) = p, only where p > 0.
					const fn = yield* F.ratio(domain, yield* P.pow(p, 2n), p)
					const db = yield* Db.create(path, Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Observations, [{ id: 1n, descriptor: Buffer.from(F.toBytes(fn)).toString("hex") }])
					yield* draft.insert(Selected, [{ observation: 1n, label: "second head given first head" }])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
	const next = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	let retained: F | undefined
	try {
		retained = await next.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.open(path, Theory)
					const rows = yield* (yield* (yield* db.snapshot()).execute(joined, {})).collect()
					assert.equal(rows.length, 1)
					assert.equal(rows[0]?.label, "second head given first head")
					assert.ok(rows[0])
					return yield* F.validate(Result.getOrThrow(F.fromBytes(Buffer.from(rows[0].descriptor, "hex"))))
				})
			)
		)
	} finally {
		await next.dispose()
	}
	assert.ok(retained)
	const final = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await final.runPromise(
			Effect.gen(function* () {
				assert.ok(retained)
				assert.equal(yield* F.at(retained, yield* fraction(0n)), null)
				assert.equal(yield* text(yield* F.at(retained, yield* fraction(1n, 4n))), "1/4")
				assert.equal(yield* text(yield* F.at(retained, yield* fraction(1n))), "1")
				assert.equal(yield* F.at(retained, yield* fraction(2n)), null)
			})
		)
	} finally {
		await final.dispose()
	}
})

test("invalid domains, roles, masks and unused operands refuse; ingress copies buffers", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.gen(function* () {
				const full = yield* R.full(id(5))
				const empty = yield* R.empty(id(5))
				const foreign = yield* R.empty(id(6))
				const domain = yield* D.new(full)
				const p = yield* P.parameter(id(5))
				const zero = yield* P.zero()
				const one = yield* constant(1n)
				const fn = yield* F.ratio(domain, one, one)
				const bad = Result.getOrThrow(P.fromBytes(Buffer.from("BEPL\x01")))
				for (const action of [
					D.new(empty),
					D.validate(Result.getOrThrow(D.fromBytes(R.toBytes(empty)))),
					R.apply(0n, empty, foreign),
					R.included(empty, foreign),
					R.equivalent(full, foreign),
					R.whereSign(id(6), p, S.none),
					R.validate(Result.getOrThrow(R.fromBytes(Buffer.from("BEPR\x01")))),
					AlgebraicRoot.validate(Result.getOrThrow(AlgebraicRoot.fromBytes(Buffer.from("BEAR\x01")))),
					F.validate(Result.getOrThrow(F.fromBytes(Buffer.from("BESC\x02\x00")))),
					AlgebraicRoot.isolate(id(5), zero),
					AlgebraicRoot.fromInterval(id(5), p, yield* fraction(-1n), yield* fraction(0n)),
					F.pieces(domain, [{ numerator: zero, denominator: bad, defined: empty }]),
					F.pieces(domain, [
						{ numerator: one, denominator: one, defined: full },
						{ numerator: zero, denominator: one, defined: full }
					]),
					F.restrict(yield* F.pieces(domain, []), foreign),
					F.multiply(yield* F.pieces(domain, []), Result.getOrThrow(F.fromBytes(Buffer.from("BESC\x02\x00"))))
				])
					assert.equal((yield* fail(action)).reason._tag, "Engine")
				for (const action of [
					R.full(new Uint8Array(31)),
					R.apply(16n, full, full),
					R.whereSign(id(5), p, 8n),
					F.whereSign(fn, -1n),
					F.pieces(domain, new Array(1)),
					F.pieces(domain, Array(4094).fill({}))
				])
					assert.equal((yield* fail(action)).reason._tag, "InvalidArgument")
				const handle = yield* runtimeHandle()
				const detached = new Uint8Array(8)
				structuredClone(detached, { transfer: [detached.buffer] })
				for (const bytes of [
					detached,
					new Uint8Array(new SharedArrayBuffer(5)),
					new Uint8Array(16 * 1024 * 1024 + 1)
				]) {
					assert.ok(
						Result.isFailure(
							yield* Effect.result(
								nativeOperationWith(
									"parameter.raw",
									(cb) => dbNative.runtimeEventParameter(handle, "region.validate", [bytes], 0n, cb),
									dbNative.runtimeBytesTake,
									(x) => x
								)
							)
						)
					)
				}
				const finite = yield* FiniteFunction.constant(yield* Event.space(id(90), 0n), yield* fraction(0n))
				assert.ok(
					Result.isFailure(
						yield* Effect.result(
							nativeOperationWith(
								"parameter.raw",
								(cb) =>
									dbNative.runtimeEventParameter(handle, "function.validate", [FiniteFunction.toBytes(finite)], 0n, cb),
								dbNative.runtimeBytesTake,
								(x) => x
							)
						)
					)
				)
				const copied = F.toBytes(fn)
				const restored = yield* nativeOperationWith(
					"parameter.raw",
					(cb) => {
						const op = dbNative.runtimeEventParameter(handle, "function.validate", [copied], 0n, cb)
						copied.fill(0)
						return op
					},
					dbNative.runtimeBytesTake,
					(bytes) => Result.getOrThrow(F.fromBytes(bytes))
				)
				assert.equal(yield* F.equivalent(fn, restored), true)
			})
		)
	} finally {
		await runtime.dispose()
	}
})

test("late structured parameter results are reclaimed before callback delivery", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventParameter
	try {
		const region = await runtime.runPromise(R.full(id(7)))
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventParameter = (handle, op, inputs, argument, callback) =>
			original(handle, op, inputs, argument, () => completed.resolve(callback))
		const fiber = runtime.runFork(R.describe(region))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeEventParameter = original
		assert.deepEqual((await runtime.runPromise(R.describe(region))).membership, [true])
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeEventParameter = original
		await runtime.dispose()
	}
})
