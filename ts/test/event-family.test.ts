import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	AlgebraicRoot,
	ChangeSet,
	contained,
	ParameterDomain as D,
	Db,
	Event,
	EventDescriptor,
	EventExpr,
	event,
	FamilyFunction as F,
	type FamilyFunctionPiece,
	FiniteFunction,
	FiniteKernel,
	key,
	mirrors,
	NativeRuntime,
	on,
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

const id = (n: number) => new Uint8Array(32).fill(n)
const rat = Q.fraction
const c = (n: bigint, d = 1n) => Effect.flatMap(rat(n, d), P.constant)
const fail = <E, A>(value: Effect.Effect<unknown, E, A>) => Effect.flip(value)
const text = (value: Q | null) => (value === null ? Effect.succeed(null) : Q.toString(value))
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}
const parameters = () =>
	Effect.gen(function* () {
		const name = id(33)
		const p = yield* P.parameter(name)
		const one = yield* c(1n)
		const region = yield* R.and(
			yield* R.whereSign(name, p, S.nonNegative),
			yield* R.whereSign(name, yield* P.subtract(one, p), S.nonNegative)
		)
		return { name, p, one, region, domain: yield* D.new(region) }
	})
const sharedDraws = (identity: number) =>
	Effect.gen(function* () {
		const params = yield* parameters()
		const { p, one, domain, region } = params
		const source = yield* Event.withParameters(yield* Event.space(id(identity), 2n), domain, [])
		const a = yield* Event.coordinate(source, 0n)
		const b = yield* Event.coordinate(source, 1n)
		const tail = yield* P.subtract(one, p)
		const pieces: FamilyFunctionPiece[] = []
		for (let code = 0; code < 4; code++)
			pieces.push({
				region: yield* Event.and(code & 1 ? a : yield* Event.complement(a), code & 2 ? b : yield* Event.complement(b)),
				value: {
					numerator: yield* P.multiply(code & 1 ? p : tail, code & 2 ? p : tail),
					denominator: one,
					defined: region
				}
			})
		const measured = yield* F.designate(yield* F.new(source, pieces))
		return {
			...params,
			source,
			measured,
			first: yield* Event.coordinate(measured, 0n),
			second: yield* Event.coordinate(measured, 1n)
		}
	})
function typePins(event: Event, fn: F, partial: PF, region: R, domain: D) {
	// @ts-expect-error A total family function is not a stored Event.
	Event.and(event, fn)
	// @ts-expect-error Partial parameter functions cannot designate a world law.
	F.designate(partial)
	// @ts-expect-error Regions do not certify inhabited ambient domains.
	Event.withParameters(event, region, [])
	// @ts-expect-error A source law is not an individual exact world.
	Event.containsParameter(event, fn)
	// @ts-expect-error A guard coordinate is an exact integer, not a numeric estimate.
	Event.withParameters(event, domain, [{ coordinate: 0, region }])
}
void typePins

test("family roles remain owned and lazy, distinct from fixed functions and kernels", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const bytes = Buffer.from("BESC\x02\x01")
	const fn = Result.getOrThrow(F.fromBytes(bytes))
	bytes.fill(0)
	assert.deepEqual(Buffer.from(F.toBytes(fn)), Buffer.from("BESC\x02\x01"))
	F.toBytes(fn).fill(0)
	assert.equal(F.isFamilyFunction({ ...fn }), false)
	assert.equal(FiniteKernel.isFiniteKernel(fn), false)
	assert.equal(FiniteFunction.isFiniteFunction(fn), false)
	assert.ok(Result.isFailure(FiniteKernel.fromBytes(F.toBytes(fn))))
	assert.ok(Result.isFailure(PF.fromBytes(F.toBytes(fn))))
	const refinement = Result.getOrThrow(RF.fromBytes(Buffer.from("BESC\x02\x03")))
	assert.ok(Result.isFailure(F.fromBytes(RF.toBytes(refinement))))
	assert.equal(RF.isParameterRefinement({ ...refinement }), false)
	F.describe(fn)
	RF.describe(refinement)
	Event.parameterWitness(Result.getOrThrow(Event.fromBytes(Buffer.from("BEVT\x03"))))
	assert.equal(nativeBindingIsLoaded(), false)
})

test("live family Events persist, satisfy ordinary laws, join and retain owned observations", async () => {
	const Parent = relation("Parent", { group: u64, condition: event })
	const Child = relation("Child", { group: u64, choice: u64, condition: event })
	const Seen = relation("Seen", { group: u64, evidence: event })
	const Theory = schema("LiveFamilies", { Parent, Child, Seen }, [
		key(Parent, ["group"]),
		key(Parent, ["group", "condition"]),
		key(Child, ["group", "condition"]),
		mirrors(on(Parent, ["group", "condition"]), on(Child, ["group", "condition"])),
		contained(on(Seen, ["group", "evidence"]), on(Parent, ["group", "condition"]))
	])
	const joined = query(Theory).rule((s) => {
		const child = v(Child)
		const seen = v(Seen)
		return s
			.match(Child, child)
			.match(Seen, { group: child.group, evidence: seen.evidence })
			.find({
				choice: child.choice,
				event: child.condition,
				evidence: seen.evidence,
				joint: EventExpr.and(child.condition, seen.evidence)
			})
	})
	const path = storeDir("live-family-observations")
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const family = yield* sharedDraws(130)
				const db = yield* Db.create(path, Theory)
				const draft = yield* ChangeSet.builder(Theory)
				yield* draft.insert(Parent, [{ group: 1n, condition: family.measured }])
				yield* draft.insert(Child, [
					{ group: 1n, choice: 1n, condition: family.second },
					{ group: 1n, choice: 0n, condition: yield* Event.complement(family.second) }
				])
				yield* draft.insert(Seen, [{ group: 1n, evidence: family.first }])
				assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
			})
		)
	)
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const db = yield* Db.open(path, Theory)
				const rows = yield* (yield* (yield* db.snapshot()).execute(joined, {})).collect()
				assert.equal(rows.length, 2)
				const row = rows.find((row) => row.choice === 1n)
				assert.ok(row)
				const probability = yield* Event.parameterProbability(row.event, row.evidence)
				assert.equal(yield* PF.equivalent(probability.numerator, yield* Event.parameterMass(row.joint)), true)
				const full = yield* Event.full(row.event)
				const payoff = yield* FiniteFunction.new(full, [
					{ region: row.event, value: yield* rat(2n) },
					{ region: yield* Event.complement(row.event), value: yield* rat(-1n) }
				])
				const expectation = yield* FiniteFunction.parameterExpectation(payoff, row.evidence)
				return { probability, expectation, full }
			})
		)
	)
	await run(
		Effect.gen(function* () {
			const { probability, expectation, full } = retained
			const q = yield* rat(1n, 4n)
			const zero = yield* rat(0n)
			assert.equal(yield* text(yield* PF.at(probability.numerator, q)), "1/16")
			assert.equal(yield* text(yield* PF.at(probability.evidenceMass, q)), "1/4")
			assert.equal(yield* text(yield* PF.at(probability.value, q)), "1/4")
			assert.equal(yield* PF.at(probability.value, zero), null)
			assert.equal(yield* R.containsRational(probability.defined, zero), false)
			assert.equal(yield* text(yield* PF.at(expectation.value, q)), "-1/4")
			assert.equal(yield* PF.at(expectation.value, zero), null)
			assert.equal(yield* Event.equal(probability.given, expectation.given), true)
			const replay = yield* Event.parameterProbability(probability.event, probability.given)
			assert.equal(yield* PF.equivalent(replay.value, probability.value), true)
			const impossible = yield* Event.parameterProbability(probability.event, yield* Event.empty(full))
			assert.equal(yield* PF.isNowhereDefined(impossible.value), true)
			assert.equal(yield* R.isEmpty(impossible.defined), true)
			assert.deepEqual(yield* Event.worldCardinality(full), { kind: "continuum" })
			assert.equal(yield* Event.atomCount(full), 4n)
		})
	)
})

test("exact parameter worlds derive guards and retain irrational singleton cardinality", async () => {
	await run(
		Effect.gen(function* () {
			const { p, name, region, domain } = yield* parameters()
			const positive = yield* R.whereSign(name, p, S.positive)
			const source = yield* Event.withParameters(yield* Event.space(id(131), 2n), domain, [
				{ coordinate: 0n, region: positive }
			])
			const guard = yield* Event.coordinate(source, 0n)
			const info = yield* Event.describeParameters(guard)
			assert.equal(info.outcomeCoordinates, 2n)
			assert.equal(info.guards[0]?.coordinate, 0n)
			assert.equal(yield* R.equivalent(yield* D.asRegion(info.domain), region), true)
			assert.equal(
				yield* Event.containsParameter(guard, { parameter: { kind: "rational", value: yield* rat(0n) }, outcomes: 2n }),
				false
			)
			assert.equal(
				yield* Event.containsParameter(guard, { parameter: { kind: "rational", value: yield* rat(1n) }, outcomes: 2n }),
				true
			)
			assert.deepEqual(yield* Event.worldCardinality(yield* Event.complement(guard)), { kind: "finite", count: 2n })
			assert.deepEqual(yield* Event.worldCardinality(yield* Event.empty(source)), { kind: "finite", count: 0n })
			const witness = yield* Event.parameterWitness(guard)
			assert.ok(witness)
			assert.equal(yield* Event.containsParameter(guard, witness), true)
			assert.equal(yield* Event.parameterWitness(yield* Event.empty(source)), null)
			const sqrt = yield* P.subtract(yield* P.pow(p, 2n), yield* c(2n))
			const root = yield* AlgebraicRoot.fromInterval(name, sqrt, yield* rat(1n), yield* rat(2n))
			const singleton = yield* D.new(yield* R.and(yield* R.whereSign(name, sqrt, S.zero), positive))
			const points = yield* Event.withParameters(yield* Event.space(id(132), 1n), singleton, [])
			assert.deepEqual(yield* Event.worldCardinality(points), { kind: "finite", count: 2n })
			assert.equal(
				yield* Event.containsParameter(points, { parameter: { kind: "algebraic", value: root }, outcomes: 1n }),
				true
			)
			const exact = yield* Event.parameterWitness(points)
			assert.equal(exact?.parameter.kind, "algebraic")
			assert.ok(exact)
			assert.equal(yield* Event.containsParameter(points, exact), true)
		})
	)
})

test("family functions preserve signed values, totality, explicit zeros and weighted maps", async () => {
	await run(
		Effect.gen(function* () {
			const { p, one, domain, region } = yield* parameters()
			const source = yield* Event.withParameters(yield* Event.space(id(133), 2n), domain, [])
			const target = yield* Event.withParameters(yield* Event.space(id(134), 1n), domain, [])
			const projection = yield* EventDescriptor.admit({
				kind: "surjective",
				map: { source, target, readouts: [yield* Event.coordinate(source, 0n)] }
			})
			const fn = yield* F.fromParameter(source, yield* PF.ratio(domain, p, one))
			const pushed = yield* F.pushforward(fn, projection)
			assert.equal(yield* text(yield* F.at(pushed, yield* rat(1n, 4n), 0n)), "1/2")
			const descended = yield* F.descend(fn, projection)
			assert.equal(yield* text(yield* F.at(descended, yield* rat(1n, 4n), 1n)), "1/4")
			assert.equal(yield* F.equivalent(fn, yield* F.pullback(descended, projection)), true)
			assert.equal(yield* text(yield* PF.at(yield* F.outcomeSum(fn, source), yield* rat(1n, 4n))), "1")
			assert.equal(yield* F.isNonnegative(fn), true)
			const negative = yield* F.fromFinite(yield* FiniteFunction.constant(source, yield* rat(-1n)))
			assert.equal(yield* F.isNonnegative(negative), false)
			const difference = yield* F.subtract(fn, negative)
			assert.equal(yield* text(yield* F.at(difference, yield* rat(1n, 4n), 3n)), "5/4")
			assert.equal(
				yield* F.equivalent(
					yield* F.add(fn, fn),
					yield* F.multiply(fn, yield* F.fromFinite(yield* FiniteFunction.constant(source, yield* rat(2n))))
				),
				true
			)
			assert.equal(yield* F.equivalent(yield* F.divide(fn, negative), yield* F.multiply(fn, negative)), true)
			const described = yield* F.describe(fn)
			assert.equal(yield* F.equivalent(fn, yield* F.new(described.space, described.pieces)), true)
			assert.equal(
				yield* F.equivalent(fn, yield* F.alignTo(fn, Result.getOrThrow(Event.fromBytes(Event.toBytes(source))))),
				true
			)
			const explicit = yield* F.new(source, [
				{ region: source, value: { numerator: yield* P.zero(), denominator: one, defined: region } }
			])
			assert.equal((yield* F.describe(explicit)).pieces.length, 1)
			assert.equal(yield* F.equivalent(explicit, yield* F.new(source, [])), true)
			assert.equal(yield* F.at(fn, yield* rat(2n), 0n), null)
			const varying = yield* F.new(source, [
				{ region: yield* Event.coordinate(source, 1n), value: { numerator: p, denominator: one, defined: region } }
			])
			assert.equal((yield* fail(F.descend(varying, projection))).reason._tag, "Engine")
		})
	)
})

test("refinement admits exact observation thresholds without changing worlds or law", async () => {
	await run(
		Effect.gen(function* () {
			const { name, p, one, domain, measured, first, second } = yield* sharedDraws(135)
			const observation = yield* Event.parameterProbability(second, first)
			const half = yield* PF.ratio(domain, yield* c(1n, 2n), one)
			const threshold = yield* PF.whereSign(yield* PF.subtract(observation.value, half), S.positive)
			assert.equal((yield* fail(Event.parameterEvent(measured, threshold))).reason._tag, "Engine")
			const refinement = yield* RF.new(id(136), measured, [threshold])
			const described = yield* RF.describe(yield* RF.validate(refinement))
			const lifted = yield* RF.lift(refinement, first)
			assert.equal(yield* Event.equal(yield* RF.descend(refinement, lifted), first), true)
			assert.equal(yield* PF.equivalent(yield* Event.parameterMass(lifted), yield* Event.parameterMass(first)), true)
			const predicate = yield* Event.parameterEvent(described.refined, threshold)
			assert.equal((yield* fail(RF.descend(refinement, predicate))).reason._tag, "Engine")
			const signed = yield* F.fromParameter(
				measured,
				yield* PF.ratio(domain, yield* P.subtract(p, yield* c(1n, 2n)), one)
			)
			assert.equal((yield* fail(F.whereSign(signed, S.positive))).reason._tag, "Engine")
			assert.equal(
				yield* Event.equal(yield* F.whereSign(yield* F.refine(signed, refinement), S.positive), predicate),
				true
			)
			const mass = yield* Event.parameterMass(predicate)
			assert.equal(yield* text(yield* PF.at(mass, yield* rat(1n, 4n))), "0")
			assert.equal(yield* text(yield* PF.at(mass, yield* rat(3n, 4n))), "1")
			// Guards select the one actual parameter; they receive no stochastic mass.
			assert.deepEqual(yield* Event.worldCardinality(described.refined), { kind: "continuum" })
			assert.equal(yield* Event.atomCount(described.refined), 8n)
			assert.equal(
				yield* R.equivalent(
					threshold,
					yield* R.and(
						yield* R.whereSign(name, yield* P.subtract(p, yield* c(1n, 2n)), S.positive),
						yield* D.asRegion(domain)
					)
				),
				true
			)
		})
	)
})

test("signed parameter-dependent expectations and guard-local holes are exact", async () => {
	await run(
		Effect.gen(function* () {
			const { name, p, one, domain, region, measured, first, second } = yield* sharedDraws(137)
			const payoff = yield* F.new(measured, [
				{ region: second, value: { numerator: p, denominator: one, defined: region } },
				{
					region: yield* Event.complement(second),
					value: { numerator: yield* c(-1n), denominator: one, defined: region }
				}
			])
			const expected = yield* F.expectation(payoff, first)
			assert.equal(yield* text(yield* PF.at(expected.value, yield* rat(1n, 4n))), "-11/16")
			assert.equal(yield* PF.at(expected.value, yield* rat(0n)), null)
			assert.equal(yield* F.equivalent(expected.function, payoff), true)
			const positive = yield* R.whereSign(name, p, S.positive)
			const guarded = yield* Event.withParameters(yield* Event.space(id(138), 1n), domain, [
				{ coordinate: 0n, region: positive }
			])
			const guard = yield* Event.coordinate(guarded, 0n)
			const local = yield* F.new(guarded, [{ region: guard, value: { numerator: p, denominator: p, defined: region } }])
			assert.equal(yield* text(yield* F.at(local, yield* rat(0n), 0n)), "0")
			assert.equal(yield* text(yield* F.at(local, yield* rat(1n, 4n), 0n)), "1")
			assert.equal(yield* Event.equal(yield* F.whereSign(local, S.positive), guard), true)
			assert.equal(yield* Event.equal(yield* F.whereSign(local, S.zero), yield* Event.complement(guard)), true)
			assert.equal(
				(yield* fail(F.new(guarded, [{ region: guarded, value: { numerator: p, denominator: p, defined: region } }])))
					.reason._tag,
				"Engine"
			)
		})
	)
})

test("family construction refuses hidden invalid inputs, non-unit laws and illegal worlds", async () => {
	await run(
		Effect.gen(function* () {
			const { name, p, one, domain, region, measured, first } = yield* sharedDraws(139)
			const raw = yield* Event.space(id(140), 1n)
			const unmeasured = yield* Event.withParameters(raw, domain, [])
			const fullValue = { numerator: p, denominator: one, defined: region }
			const empty = yield* Event.empty(unmeasured)
			const fn = yield* F.new(unmeasured, [{ region: unmeasured, value: fullValue }])
			const bad = Result.getOrThrow(P.fromBytes(Buffer.from("BEPL\x01")))
			for (const action of [
				Event.withParameters(measured, domain, []),
				Event.withParameters(yield* Event.coordinate(raw, 0n), domain, []),
				Event.withParameters(raw, domain, [
					{ coordinate: 0n, region },
					{ coordinate: 0n, region }
				]),
				Event.withParameters(raw, domain, [{ coordinate: 1n, region }]),
				Event.withParameters(raw, domain, [{ coordinate: 0n, region: yield* R.empty(id(9)) }]),
				Event.parameterMass(unmeasured),
				Event.parameterProbability(empty, first),
				F.designate(fn),
				F.designate(yield* F.fromFinite(yield* FiniteFunction.constant(unmeasured, yield* rat(-1n)))),
				F.divide(fn, fn),
				F.fromParameter(unmeasured, yield* PF.ratio(domain, p, p)),
				F.new(unmeasured, [
					{ region: unmeasured, value: fullValue },
					{ region: unmeasured, value: fullValue }
				]),
				F.new(unmeasured, [{ region: empty, value: { numerator: bad, denominator: one, defined: region } }]),
				F.new(unmeasured, [{ region: empty, value: { ...fullValue, defined: yield* R.empty(id(10)) } }]),
				F.validate(Result.getOrThrow(F.fromBytes(Buffer.from("BESC\x02\x01")))),
				RF.validate(Result.getOrThrow(RF.fromBytes(Buffer.from("BESC\x02\x03")))),
				Event.containsParameter(unmeasured, { parameter: { kind: "rational", value: yield* rat(2n) }, outcomes: 0n }),
				Event.containsParameter(unmeasured, { parameter: { kind: "rational", value: yield* rat(0n) }, outcomes: 2n })
			])
				assert.equal((yield* fail(action)).reason._tag, "Engine")
			for (const action of [
				Event.withParameters(raw, domain, new Array(1)),
				Event.withParameters(raw, domain, [{ coordinate: 62n, region }]),
				RF.new(id(141), measured, Array(63).fill(region)),
				F.new(unmeasured, Array(2047).fill({})),
				F.at(fn, yield* rat(0n), 1n << 64n)
			])
				assert.equal((yield* fail(action)).reason._tag, "InvalidArgument")
			const guardSource = yield* Event.withParameters(raw, domain, [
				{ coordinate: 0n, region: yield* R.whereSign(name, p, S.positive) }
			])
			assert.equal(
				(yield* fail(
					Event.containsParameter(guardSource, { parameter: { kind: "rational", value: yield* rat(1n) }, outcomes: 1n })
				)).reason._tag,
				"Engine"
			)
			const copied = Event.toBytes(measured)
			const handle = yield* runtimeHandle()
			const value = yield* nativeOperationWith(
				"family.copy",
				(cb) => {
					const op = dbNative.runtimeEventParameter(handle, "source.mass", [copied], 0n, cb)
					copied.fill(0)
					return op
				},
				dbNative.runtimeBytesTake,
				(bytes) => Result.getOrThrow(PF.fromBytes(bytes))
			)
			assert.equal(yield* text(yield* PF.at(value, yield* rat(1n, 2n))), "1")
		})
	)
})

test("late owned family observations are reclaimed before delivery", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventParameter
	try {
		const family = await runtime.runPromise(sharedDraws(142))
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventParameter = (handle, op, inputs, argument, cb) =>
			original(handle, op, inputs, argument, () => completed.resolve(cb))
		const fiber = runtime.runFork(Event.parameterProbability(family.second, family.first))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeEventParameter = original
		const value = await runtime.runPromise(Event.parameterProbability(family.second, family.first))
		assert.equal(
			await runtime.runPromise(Effect.flatMap(rat(1n, 2n), (q) => Effect.flatMap(PF.at(value.value, q), text))),
			"1/2"
		)
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeEventParameter = original
		await runtime.dispose()
	}
})

test("symbolic family normalization counts outcomes, never deterministic guard cases", async () => {
	await run(
		Effect.gen(function* () {
			const { name, p, one, domain } = yield* parameters()
			const positive = yield* R.whereSign(name, p, S.positive)
			const source = yield* Event.withParameters(yield* Event.space(id(143), 62n), domain, [
				{ coordinate: 61n, region: positive }
			])
			const density = yield* F.fromParameter(source, yield* PF.ratio(domain, yield* c(1n, 1n << 61n), one))
			const measured = yield* F.designate(density)
			assert.equal(yield* Event.atomCount(measured), 1n << 62n)
			assert.deepEqual(yield* Event.worldCardinality(measured), { kind: "continuum" })
			const total = yield* Event.parameterMass(measured)
			assert.equal(yield* text(yield* PF.at(total, yield* rat(0n))), "1")
			assert.equal(yield* text(yield* PF.at(total, yield* rat(1n, 2n))), "1")
			const captured = yield* F.density(measured)
			assert.equal(yield* Event.equal(yield* F.designate(captured), measured), true)
			const raw = yield* Event.space(id(144), 1n)
			const onlyOne = yield* Event.restrict(raw, yield* Event.coordinate(raw, 0n))
			// A guard declaration must not silently remove the zero parameter case.
			assert.equal(
				(yield* fail(Event.withParameters(onlyOne, domain, [{ coordinate: 0n, region: positive }]))).reason._tag,
				"Engine"
			)
			const restricted = yield* Event.withParameters(onlyOne, domain, [])
			const fn = yield* F.fromParameter(restricted, yield* PF.ratio(domain, one, one))
			assert.equal((yield* fail(F.at(fn, yield* rat(1n, 2n), 0n))).reason._tag, "Engine")
		})
	)
})
