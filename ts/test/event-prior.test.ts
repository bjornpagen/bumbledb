import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	BetaSource as B,
	ChangeSet,
	ParameterDomain as D,
	Db,
	Event,
	EventExpr,
	event,
	FamilyFunction as F,
	FiniteFunction,
	key,
	NativeRuntime,
	ExactPolynomial as P,
	ParameterFunction as PF,
	ExactRational as Q,
	query,
	ParameterRegion as R,
	ParameterRefinement as RF,
	relation,
	PolynomialSigns as S,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const fail = <E, A>(value: Effect.Effect<unknown, E, A>) => Effect.flip(value)
const id = (n: number) => new Uint8Array(32).fill(n)
const c = (n: bigint) => Effect.flatMap(Q.fraction(n), P.constant)
const text = (value: Q | null) => (value === null ? Effect.succeed(null) : Q.toString(value))
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}
function draws(identity = 190) {
	return Effect.gen(function* () {
		const name = id(189)
		const p = yield* P.parameter(name)
		const one = yield* c(1n)
		const tail = yield* P.subtract(one, p)
		const region = yield* R.and(
			yield* R.whereSign(name, p, S.nonNegative),
			yield* R.whereSign(name, tail, S.nonNegative)
		)
		const domain = yield* D.new(region)
		const base = yield* Event.withParameters(yield* Event.space(id(identity), 2n), domain, [])
		const a = yield* Event.coordinate(base, 0n)
		const b = yield* Event.coordinate(base, 1n)
		const pieces = []
		for (let code = 0; code < 4; code++)
			pieces.push({
				region: yield* Event.and(code & 1 ? a : yield* Event.complement(a), code & 2 ? b : yield* Event.complement(b)),
				value: {
					numerator: yield* P.multiply(code & 1 ? p : tail, code & 2 ? p : tail),
					denominator: one,
					defined: region
				}
			})
		const space = yield* F.designate(yield* F.new(base, pieces))
		const prior = yield* B.new(space, yield* Q.fraction(1n), yield* Q.fraction(1n))
		return {
			name,
			p,
			one,
			tail,
			region,
			domain,
			base,
			space,
			prior,
			first: yield* Event.coordinate(space, 0n),
			second: yield* Event.coordinate(space, 1n)
		}
	})
}

test("Beta envelopes are owned, role-separated, lazy and require worker admission", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const bytes = Buffer.from("BESC\x02\x06")
	const value = Result.getOrThrow(B.fromBytes(bytes))
	bytes.fill(0)
	B.toBytes(value).fill(0)
	assert.deepEqual(Buffer.from(B.toBytes(value)), Buffer.from("BESC\x02\x06"))
	assert.equal(B.isBetaSource(value), true)
	assert.equal(B.isBetaSource({ ...value }), false)
	assert.equal(Result.isFailure(F.fromBytes(B.toBytes(value))), true)
	B.validate(value)
	B.describe(value)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("shared prior integrates raw masses before conditioning and owns complete certificates", async () => {
	const retained = await run(
		Effect.gen(function* () {
			const f = yield* draws()
			const prior = yield* B.validate(Result.getOrThrow(B.fromBytes(B.toBytes(f.prior))))
			const description = yield* B.describe(prior)
			assert.equal(yield* Event.equal(description.space, f.space), true)
			assert.equal(yield* text(description.alpha), "1")
			assert.equal(yield* text(description.beta), "1")
			const joint = yield* B.probability(prior, yield* Event.and(f.first, f.second), f.space)
			assert.equal(yield* text(joint.value), "1/3")
			const a = yield* B.probability(prior, f.first, f.space)
			const b = yield* B.probability(prior, f.second, f.space)
			assert.ok(a.value)
			assert.ok(b.value)
			assert.equal(yield* text(yield* Q.multiply(a.value, b.value)), "1/4")
			const posterior = yield* B.probability(prior, f.second, f.first)
			assert.equal(yield* text(posterior.value), "2/3")
			assert.equal(yield* text(posterior.numerator.value), "1/3")
			assert.equal(yield* text(posterior.evidenceMass.value), "1/2")
			assert.equal(yield* PF.at(posterior.original.value, yield* Q.fraction(0n)), null)
			assert.equal(yield* PF.equivalent(posterior.original.numerator, posterior.numerator.function), true)
			assert.equal(yield* R.isEmpty(posterior.numerator.exceptions), true)
			assert.equal((yield* Effect.flip(B.integrate(prior, posterior.original.value))).reason._tag, "Engine")
			const asymmetric = yield* B.new(f.space, yield* Q.fraction(2n), yield* Q.fraction(3n))
			assert.equal(yield* text((yield* B.probability(asymmetric, f.second, f.first)).value), "1/2")
			return posterior
		})
	)
	await run(
		Effect.gen(function* () {
			const replay = yield* B.probability(retained.source, retained.original.event, retained.original.given)
			assert.equal(yield* text(replay.value), "2/3")
			assert.equal(yield* PF.equivalent(retained.numerator.function, replay.numerator.function), true)
		})
	)
})

test("zero-prior-mass endpoint Events stay possible and retain exception certificates", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* draws(191)
			const zero = yield* R.whereSign(f.name, f.p, S.zero)
			const refinement = yield* RF.new(id(192), f.space, [zero])
			const { refined } = yield* RF.describe(refinement)
			const endpoint = yield* Event.parameterEvent(refined, zero)
			const prior = yield* B.new(refined, yield* Q.fraction(1n), yield* Q.fraction(1n))
			const observed = yield* B.probability(prior, endpoint, refined)
			assert.equal(yield* Event.isEmpty(observed.original.event), false)
			assert.equal(yield* text(observed.value), "0")
			assert.equal(yield* R.equivalent(observed.numerator.exceptions, zero), true)
			assert.equal(yield* text(yield* PF.at(observed.numerator.function, yield* Q.fraction(0n))), "1")
			assert.equal((yield* B.probability(prior, refined, endpoint)).value, null)
			assert.equal(yield* Event.isEmpty(endpoint), false)
		})
	)
})

test("Beta contraction admits polynomial presentations, finite exceptions and signed payoffs", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* draws(193)
			const denominator = yield* P.add(f.one, f.p)
			const fn = yield* PF.ratio(f.domain, yield* P.multiply(f.p, denominator), denominator)
			const integrated = yield* B.integrate(f.prior, fn)
			assert.equal(yield* text(integrated.value), "1/2")
			assert.equal(yield* P.equal(integrated.polynomial, f.p), true)
			const singleton = yield* R.whereSign(
				f.name,
				yield* P.subtract(yield* P.multiply(yield* c(2n), f.p), f.one),
				S.zero
			)
			const exceptional = yield* PF.pieces(f.domain, [
				{ numerator: f.p, denominator: f.one, defined: yield* R.and(f.region, yield* R.complement(singleton)) },
				{ numerator: yield* c(8n), denominator: f.one, defined: singleton }
			])
			const certificate = yield* B.integrate(f.prior, exceptional)
			assert.equal(yield* R.equivalent(certificate.exceptions, singleton), true)
			assert.equal(yield* text(certificate.value), "1/2")
			const payoff = yield* FiniteFunction.new(f.space, [
				{ region: f.second, value: yield* Q.fraction(2n) },
				{ region: yield* Event.complement(f.second), value: yield* Q.fraction(-1n) }
			])
			const expected = yield* B.expectation(f.prior, payoff, f.first)
			assert.equal(yield* text(expected.value), "1")
			assert.equal(yield* PF.at(expected.original.value, yield* Q.fraction(0n)), null)
			const family = yield* F.fromParameter(f.space, yield* PF.ratio(f.domain, f.p, f.one))
			const familyResult = yield* B.familyExpectation(f.prior, family, f.first)
			assert.equal(yield* text(familyResult.value), "2/3")
			assert.equal(yield* F.equivalent(family, familyResult.original.function), true)
		})
	)
})

test("Beta scope refuses missing laws, foreign sources, nonpolynomials and inherited holes", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* draws(194)
			const one = yield* Q.fraction(1n)
			const zero = yield* Q.fraction(0n)
			const foreign = yield* draws(195)
			const nonpolynomial = yield* PF.ratio(f.domain, f.one, yield* P.add(f.one, f.p))
			const hole = yield* PF.ratio(f.domain, f.p, f.p)
			for (const operation of [
				B.new(f.space, zero, one),
				B.new(f.base, one, one),
				B.probability(f.prior, foreign.first, f.first),
				B.integrate(f.prior, nonpolynomial),
				B.integrate(f.prior, hole),
				B.validate(Result.getOrThrow(B.fromBytes(Buffer.from("BESC\x02\x06"))))
			])
				assert.equal((yield* fail(operation)).reason._tag, "Engine")
			const restricted = yield* D.new(yield* R.and(f.region, yield* R.whereSign(f.name, f.p, S.positive)))
			const partial = yield* Event.withParameters(yield* Event.space(id(196), 0n), restricted, [])
			const law = yield* F.designate(yield* F.fromParameter(partial, yield* PF.ratio(restricted, f.one, f.one)))
			assert.equal((yield* Effect.flip(B.new(law, one, one))).reason._tag, "Engine")
		})
	)
})

test("stored Event joins remain parameter-dependent until explicit prior binding after reopen", async () => {
	const Claim = relation("Claim", { id: u64, region: event })
	const Theory = schema("ExplicitPrior", { Claim }, [key(Claim, ["id"])])
	const path = storeDir("explicit-beta-prior")
	const bytes = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const f = yield* draws(197)
				const db = yield* Db.create(path, Theory)
				const draft = yield* ChangeSet.builder(Theory)
				yield* draft.insert(Claim, [
					{ id: 1n, region: f.first },
					{ id: 2n, region: f.second }
				])
				assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
				return B.toBytes(f.prior)
			})
		)
	)
	const joined = query(Theory).rule((s) => {
		const a = v(Claim)
		const b = v(Claim)
		return s
			.match(Claim, { id: 1n, region: a.region })
			.match(Claim, { id: 2n, region: b.region })
			.find({ event: b.region, given: a.region, joint: EventExpr.and(a.region, b.region) })
	})
	const rows = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const db = yield* Db.open(path, Theory)
				return yield* (yield* (yield* db.snapshot()).execute(joined, {})).collect()
			})
		)
	)
	await run(
		Effect.gen(function* () {
			assert.equal(rows.length, 1)
			const row = rows[0]
			assert.ok(row)
			const prior = yield* B.validate(Result.getOrThrow(B.fromBytes(bytes)))
			const observed = yield* B.probability(prior, row.event, row.given)
			assert.equal(yield* text(observed.value), "2/3")
			assert.equal(yield* PF.equivalent(observed.original.numerator, yield* Event.parameterMass(row.joint)), true)
		})
	)
})

test("prior workers own submitted bytes and cancellation never publishes a late observation", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventParameter
	try {
		const f = await runtime.runPromise(draws(198))
		const prior = await runtime.runPromise(
			Effect.gen(function* () {
				const owner = yield* runtimeHandle()
				const inputs = [Event.toBytes(f.space), Q.toBytes(yield* Q.fraction(1n)), Q.toBytes(yield* Q.fraction(1n))]
				return yield* nativeOperationWith(
					"owned Beta prior",
					(cb) => {
						const operation = original(owner, "prior.new", inputs, 0n, cb)
						for (const input of inputs) input.fill(0)
						return operation
					},
					dbNative.runtimeBytesTake,
					(value) => Result.getOrThrow(B.fromBytes(value))
				)
			})
		)
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventParameter = (handle, op, inputs, argument, cb) =>
			original(handle, op, inputs, argument, () => completed.resolve(cb))
		const fiber = runtime.runFork(B.probability(prior, f.second, f.first))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeEventParameter = original
		assert.equal(
			await runtime.runPromise(Effect.flatMap(B.probability(prior, f.second, f.first), (result) => text(result.value))),
			"2/3"
		)
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeEventParameter = original
		await runtime.dispose()
	}
})
