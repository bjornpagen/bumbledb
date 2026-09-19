import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime } from "effect"
import { dbNative } from "#db-native.ts"
import {
	ParameterDomain as D,
	Event,
	FamilyFunction as F,
	FiniteFunction as FF,
	type FunctionPatch,
	NativeRuntime,
	ExactPolynomial as P,
	ParameterFunction as PF,
	ExactRational as Q,
	ParameterRegion as R,
	PolynomialSigns as S
} from "#index.ts"
import { runtimeOptions } from "#test/fixtures/learning.ts"

const id = (n: number) => new Uint8Array(32).fill(n)
const c = (n: bigint) => Effect.flatMap(Q.fraction(n), P.constant)
const fail = <E, A>(value: Effect.Effect<unknown, E, A>) => Effect.flip(value)
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}

test("finite covers keep agreeing overlaps, supplied zeros, evidence and ownership", async () => {
	const cover = await run(
		Effect.gen(function* () {
			const space = yield* Event.space(id(220), 1n)
			const head = yield* Event.coordinate(space, 0n)
			const tails = yield* Event.complement(head)
			const a = yield* FF.new(space, [
				{ region: head, value: yield* Q.fraction(1n, 3n) },
				{ region: tails, value: yield* Q.fraction(-2n) }
			])
			const b = yield* FF.constant(space, yield* Q.fraction(1n, 3n))
			const patches: FunctionPatch<FF>[] = [
				{ region: space, function: a },
				{ region: head, function: b },
				{ region: yield* Event.empty(space), function: yield* FF.constant(space, yield* Q.fraction(0n)) }
			]
			const cover = yield* FF.glue(head, patches)
			assert.equal(cover.patches.length, 3)
			assert.ok(cover.patches[0])
			assert.ok(cover.patches[2])
			assert.equal(yield* Event.isFull(cover.patches[0].region), true)
			assert.equal(yield* Event.isEmpty(cover.patches[2].region), true)
			assert.equal(yield* FF.equivalent(cover.function, yield* FF.mask(a, head)), true)
			assert.equal(yield* FF.equivalent(cover.function, (yield* FF.glue(head, [...patches].reverse())).function), true)
			assert.equal(yield* Q.toString(yield* FF.at(cover.function, 0n)), "0")
			assert.equal(yield* Q.toString(yield* FF.at(cover.function, 1n)), "1/3")
			patches.length = 0
			return cover
		})
	)
	await run(
		Effect.gen(function* () {
			assert.equal(cover.patches.length, 3)
			const replay = yield* FF.glue(cover.parent, cover.patches)
			assert.equal(yield* FF.equivalent(cover.function, replay.function), true)
		})
	)
})

test("zero mass does not waive coverage or pointwise value agreement", async () => {
	await run(
		Effect.gen(function* () {
			const raw = yield* Event.space(id(221), 1n)
			const density = yield* FF.new(raw, [{ region: yield* Event.coordinate(raw, 0n), value: yield* Q.fraction(1n) }])
			const space = yield* FF.designate(density)
			const head = yield* Event.coordinate(space, 0n)
			const tails = yield* Event.complement(head)
			const zero = yield* FF.constant(space, yield* Q.fraction(0n))
			const one = yield* FF.constant(space, yield* Q.fraction(1n))
			assert.equal((yield* fail(FF.glue(space, [{ region: head, function: zero }]))).reason._tag, "Engine")
			assert.equal(
				(yield* fail(
					FF.glue(space, [
						{ region: space, function: zero },
						{ region: tails, function: one }
					])
				)).reason._tag,
				"Engine"
			)
			const foreign = yield* Event.space(id(222), 1n)
			const foreignZero = yield* FF.constant(foreign, yield* Q.fraction(0n))
			assert.equal(
				(yield* fail(
					FF.glue(space, [
						{ region: space, function: zero },
						{ region: yield* Event.empty(space), function: foreignZero }
					])
				)).reason._tag,
				"Engine"
			)
			assert.equal((yield* fail(FF.mask(zero, yield* Event.empty(foreign)))).reason._tag, "Engine")
			const empty = yield* FF.glue(yield* Event.empty(space), [])
			assert.equal(yield* FF.isZero(empty.function), true)
			const impossible = yield* FF.expectation(one, tails)
			assert.equal(impossible.value, null)
		})
	)
})

function family() {
	return Effect.gen(function* () {
		const name = id(223)
		const p = yield* P.parameter(name)
		const one = yield* c(1n)
		const tail = yield* P.subtract(one, p)
		const unit = yield* R.and(yield* R.whereSign(name, p, S.nonNegative), yield* R.whereSign(name, tail, S.nonNegative))
		const domain = yield* D.new(unit)
		const threshold = yield* P.subtract(yield* P.multiply(p, yield* c(2n)), one)
		const space = yield* Event.withParameters(yield* Event.space(id(224), 2n), domain, [
			{ coordinate: 0n, region: yield* R.whereSign(name, threshold, S.nonPositive) },
			{ coordinate: 1n, region: yield* R.whereSign(name, threshold, S.nonNegative) }
		])
		return {
			space,
			domain,
			p,
			one,
			tail,
			left: yield* Event.coordinate(space, 0n),
			right: yield* Event.coordinate(space, 1n),
			rising: yield* F.fromParameter(space, yield* PF.ratio(domain, p, one)),
			falling: yield* F.fromParameter(space, yield* PF.ratio(domain, tail, one))
		}
	})
}

test("family covers glue functions equal only on their actual overlap", async () => {
	const retained = await run(
		Effect.gen(function* () {
			const f = yield* family()
			assert.equal(yield* F.equivalent(f.rising, f.falling), false)
			const patches = [
				{ region: f.left, function: f.rising },
				{ region: f.right, function: f.falling }
			]
			const cover = yield* F.glue(f.space, patches)
			const reverse = yield* F.glue(f.space, patches.toReversed())
			assert.equal(yield* F.equivalent(cover.function, reverse.function), true)
			for (const [n, d, expected] of [
				[0n, 1n, "0"],
				[1n, 4n, "1/4"],
				[1n, 2n, "1/2"],
				[3n, 4n, "1/4"]
			] as const) {
				const value = yield* F.at(cover.function, yield* Q.fraction(n, d), 0n)
				assert.ok(value)
				assert.equal(yield* Q.toString(value), expected)
			}
			const point = yield* Event.and(f.left, f.right)
			assert.equal(yield* F.equivalent(yield* F.mask(f.rising, point), yield* F.mask(f.falling, point)), true)
			const wrong = yield* F.fromFinite(yield* FF.constant(f.space, yield* Q.fraction(7n)))
			assert.equal(
				(yield* fail(F.glue(f.space, [...patches, { region: point, function: wrong }]))).reason._tag,
				"Engine"
			)
			assert.equal((yield* fail(F.glue(f.space, [{ region: f.left, function: f.rising }]))).reason._tag, "Engine")
			return cover
		})
	)
	await run(
		Effect.gen(function* () {
			const replay = yield* F.glue(retained.parent, retained.patches)
			assert.equal(yield* F.equivalent(replay.function, retained.function), true)
		})
	)
})

test("cancelling a completed cover releases its owned result before a late callback", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventParameter
	try {
		const f = await runtime.runPromise(family())
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventParameter = (handle, op, inputs, argument, cb) =>
			original(handle, op, inputs, argument, () => completed.resolve(cb))
		const fiber = runtime.runFork(F.glue(f.space, [{ region: f.space, function: f.rising }]))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeEventParameter = original
		const answer = await runtime.runPromise(F.glue(f.space, [{ region: f.space, function: f.rising }]))
		assert.equal(answer.patches.length, 1)
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeEventParameter = original
		await runtime.dispose()
	}
})
