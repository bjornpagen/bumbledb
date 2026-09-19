import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, Exit, Fiber, ManagedRuntime, Result } from "effect"
import { dbNative } from "#db-native.ts"
import {
	ChangeSet,
	ParameterDomain as D,
	Db,
	Event,
	EventDescriptor,
	EventExpr,
	event,
	FamilyFunction as F,
	FiniteFunction,
	FiniteKernel,
	FamilyKernel as K,
	NativeRuntime,
	ExactPolynomial as P,
	ParameterFunction as PF,
	ExactRational as Q,
	query,
	ParameterRegion as R,
	ParameterRefinement as RF,
	ParameterRestriction as RS,
	relation,
	PolynomialSigns as S,
	SourceRevision,
	schema,
	u64,
	FamilyRevision as V,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { nativeOperationWith, runtimeHandle } from "#runtime.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const id = (n: number) => new Uint8Array(32).fill(n)
const c = (n: bigint, d = 1n) => Effect.flatMap(Q.fraction(n, d), P.constant)
const fail = <E, A>(value: Effect.Effect<unknown, E, A>) => Effect.flip(value)
const at = (value: PF, n: bigint, d = 1n) =>
	Effect.gen(function* () {
		const result = yield* PF.at(value, yield* Q.fraction(n, d))
		return result === null ? null : yield* Q.toString(result)
	})
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}
const coin = (identity: number) =>
	Effect.gen(function* () {
		const name = id(41)
		const p = yield* P.parameter(name)
		const one = yield* c(1n)
		const tail = yield* P.subtract(one, p)
		const region = yield* R.and(
			yield* R.whereSign(name, p, S.nonNegative),
			yield* R.whereSign(name, tail, S.nonNegative)
		)
		const domain = yield* D.new(region)
		const source = yield* Event.withParameters(yield* Event.space(id(identity), 1n), domain, [])
		const raw = yield* Event.coordinate(source, 0n)
		const prior = yield* F.designate(
			yield* F.new(source, [
				{ region: raw, value: { numerator: p, denominator: one, defined: region } },
				{ region: yield* Event.complement(raw), value: { numerator: tail, denominator: one, defined: region } }
			])
		)
		const head = yield* Event.coordinate(prior, 0n)
		return { name, p, one, tail, region, domain, source, prior, head, tails: yield* Event.complement(head) }
	})
const scalar = (source: Event, n: bigint, d = 1n) =>
	Effect.gen(function* () {
		return yield* F.fromFinite(yield* FiniteFunction.constant(source, yield* Q.fraction(n, d)))
	})

function typePins(kernel: K, revision: V, restriction: RS, finite: FiniteKernel, source: Event) {
	// @ts-expect-error Fixed channels are not parameter-family channels.
	K.close(finite, source)
	// @ts-expect-error Restriction does not admit a revision receipt role.
	RS.pullback(revision, source)
	// @ts-expect-error Update receipts are not conditional channels.
	K.close(revision, source)
	// @ts-expect-error Channels are not revisions.
	V.inspect(kernel)
	// @ts-expect-error Restriction is not an Event.
	Event.complement(restriction)
}
void typePins

test("family dynamics carriers copy transport, keep roles distinct and remain addon-free", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const input = Buffer.from("BESC\x02\x02")
	const kernel = Result.getOrThrow(K.fromBytes(input))
	input.fill(0)
	assert.deepEqual(Buffer.from(K.toBytes(kernel)), Buffer.from("BESC\x02\x02"))
	K.toBytes(kernel).fill(0)
	assert.equal(K.isFamilyKernel({ ...kernel }), false)
	assert.ok(Result.isFailure(FiniteKernel.fromBytes(K.toBytes(kernel))))
	assert.ok(Result.isFailure(SourceRevision.fromBytes(K.toBytes(kernel))))
	assert.ok(Result.isFailure(V.fromBytes(K.toBytes(kernel))))
	const revision = Result.getOrThrow(V.fromBytes(Buffer.from("BESC\x02\x05")))
	const restriction = Result.getOrThrow(RS.fromBytes(Buffer.from("BESC\x02\x04")))
	assert.ok(Result.isFailure(RF.fromBytes(RS.toBytes(restriction))))
	assert.equal(V.isFamilyRevision({ ...revision }), false)
	assert.equal(RS.isParameterRestriction({ ...restriction }), false)
	K.describe(kernel)
	V.inspect(revision)
	RS.describe(restriction)
	RF.common([])
	assert.equal(nativeBindingIsLoaded(), false)
})

test("conditioning keeps exact positive domain, zero-mass possibilities and original-prior translation", async () => {
	const saved = await run(
		Effect.gen(function* () {
			const f = yield* coin(150)
			const revision = yield* V.condition(id(151), f.prior, f.head)
			const inspection = yield* V.inspect(yield* V.validate(revision))
			assert.deepEqual(inspection.identity, id(151))
			assert.equal(inspection.receipt.kind, "condition")
			assert.equal(yield* R.containsRational(inspection.defined, yield* Q.fraction(0n)), false)
			assert.equal(yield* R.containsRational(inspection.defined, yield* Q.fraction(1n)), true)
			assert.ok(inspection.outcome.kind === "revised")
			assert.ok(inspection.receipt.kind === "condition")
			assert.equal(yield* at(inspection.receipt.mass, 1n, 3n), "1/3")
			const heads = yield* V.pullback(revision, f.head)
			const tails = yield* V.pullback(revision, f.tails)
			assert.equal(yield* at(yield* Event.parameterMass(heads), 1n, 3n), "1")
			assert.equal(yield* at(yield* Event.parameterMass(tails), 1n, 3n), "0")
			assert.equal(yield* Event.isEmpty(tails), false)
			assert.equal(
				yield* Event.containsParameter(tails, {
					parameter: { kind: "rational", value: yield* Q.fraction(1n, 3n) },
					outcomes: 0n
				}),
				true
			)
			return { revision, prior: f.prior, head: f.head, posterior: inspection.outcome.posterior }
		})
	)
	await run(
		Effect.gen(function* () {
			const imported = Result.getOrThrow(V.fromBytes(V.toBytes(saved.revision)))
			const result = yield* V.inspect(imported)
			assert.ok(result.outcome.kind === "revised")
			assert.equal(yield* Event.equal(result.prior, saved.prior), true)
			assert.equal(yield* Event.equal(result.outcome.posterior, saved.posterior), true)
			assert.equal(yield* at(yield* Event.parameterMass(yield* V.pullback(imported, saved.head)), 0n), null)
		})
	)
})

test("likelihood scale is retained while posterior values and repeated evidence are exact", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* coin(152)
			const likelihood = yield* F.fromFinite(
				yield* FiniteFunction.new(f.prior, [
					{ region: f.head, value: yield* Q.fraction(2n) },
					{ region: f.tails, value: yield* Q.fraction(1n) }
				])
			)
			const first = yield* V.likelihood(id(153), f.prior, likelihood)
			const scaled = yield* V.likelihood(id(154), f.prior, yield* F.multiply(likelihood, yield* scalar(f.prior, 3n)))
			const inspection = yield* V.inspect(first)
			const scaledInspection = yield* V.inspect(scaled)
			assert.ok(inspection.receipt.kind === "likelihood" && scaledInspection.receipt.kind === "likelihood")
			assert.equal(yield* at(inspection.receipt.normalizer, 1n, 2n), "3/2")
			assert.equal(yield* at(scaledInspection.receipt.normalizer, 1n, 2n), "9/2")
			assert.equal(yield* F.equivalent(inspection.receipt.likelihood, likelihood), true)
			const head = yield* V.pullback(first, f.head)
			assert.equal(yield* at(yield* Event.parameterMass(head), 1n, 2n), "2/3")
			assert.equal(yield* at(yield* Event.parameterMass(yield* V.pullback(scaled, f.head)), 1n, 2n), "2/3")
			assert.ok(inspection.outcome.kind === "revised")
			const nextLikelihood = yield* F.fromFinite(
				yield* FiniteFunction.new(inspection.outcome.posterior, [
					{ region: head, value: yield* Q.fraction(2n) },
					{ region: yield* Event.complement(head), value: yield* Q.fraction(1n) }
				])
			)
			const second = yield* V.likelihood(id(155), inspection.outcome.posterior, nextLikelihood)
			assert.equal(yield* at(yield* Event.parameterMass(yield* V.pullback(second, head)), 1n, 2n), "4/5")
			assert.equal((yield* fail(V.likelihood(id(156), f.prior, yield* scalar(f.prior, -1n)))).reason._tag, "Engine")
		})
	)
})

test("Jeffrey replacement retains indexed unsupported endpoints, empty cells and rejects partial targets", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* coin(157)
			const half = yield* PF.ratio(f.domain, yield* c(1n, 2n), f.one)
			const zero = yield* PF.ratio(f.domain, yield* P.zero(), f.one)
			const revision = yield* V.jeffrey(id(158), f.prior, [
				{ cell: f.head, target: half },
				{ cell: f.tails, target: half },
				{ cell: yield* Event.empty(f.prior), target: zero }
			])
			const inspection = yield* V.inspect(yield* V.validate(revision))
			assert.ok(inspection.receipt.kind === "jeffrey" && inspection.outcome.kind === "revised")
			assert.equal(inspection.receipt.cells.length, 3)
			const unsupported = inspection.receipt.unsupportedRegions
			assert.ok(unsupported[0] && unsupported[1] && unsupported[2])
			assert.equal(yield* R.containsRational(unsupported[0], yield* Q.fraction(0n)), true)
			assert.equal(yield* R.containsRational(unsupported[0], yield* Q.fraction(1n)), false)
			assert.equal(yield* R.containsRational(unsupported[1], yield* Q.fraction(1n)), true)
			assert.equal(yield* R.isEmpty(unsupported[2]), true)
			assert.equal(yield* R.containsRational(inspection.defined, yield* Q.fraction(0n)), false)
			assert.equal(yield* R.containsRational(inspection.defined, yield* Q.fraction(1n)), false)
			assert.equal(yield* at(yield* Event.parameterMass(yield* V.pullback(revision, f.head)), 1n, 3n), "1/2")
			const partial = yield* PF.ratio(f.domain, yield* P.zero(), f.p)
			assert.equal(
				(yield* fail(
					V.jeffrey(id(159), f.prior, [
						{ cell: f.head, target: half },
						{ cell: f.tails, target: half },
						{ cell: yield* Event.empty(f.prior), target: partial }
					])
				)).reason._tag,
				"Engine"
			)
			assert.equal((yield* fail(V.jeffrey(id(159), f.prior, [{ cell: f.head, target: half }]))).reason._tag, "Engine")
		})
	)
})

test("everywhere-impossible requests keep identities and complete mathematical receipts", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* coin(160)
			const one = yield* PF.ratio(f.domain, f.one, f.one)
			const zero = yield* PF.ratio(f.domain, yield* P.zero(), f.one)
			const empty = yield* Event.empty(f.prior)
			const requests = [
				yield* V.condition(id(161), f.prior, empty),
				yield* V.likelihood(id(162), f.prior, yield* scalar(f.prior, 0n)),
				yield* V.jeffrey(id(163), f.prior, [
					{ cell: empty, target: one },
					{ cell: f.prior, target: zero }
				])
			]
			for (const [position, revision] of requests.entries()) {
				const inspection = yield* V.inspect(yield* V.validate(revision))
				assert.deepEqual(inspection.identity, id(161 + position))
				assert.equal(inspection.outcome.kind, "impossible")
				assert.equal(yield* R.isEmpty(inspection.defined), true)
				assert.equal(yield* Event.equal(inspection.prior, f.prior), true)
				assert.equal((yield* fail(V.pullback(revision, f.head))).reason._tag, "Engine")
			}
		})
	)
})

test("restriction preserves the law while common refinement unifies predicates without changing domains", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* coin(164)
			const positive = yield* R.whereSign(f.name, f.p, S.positive)
			const threshold = yield* R.whereSign(f.name, yield* P.subtract(f.p, yield* c(1n, 2n)), S.positive)
			const left = yield* RF.new(id(165), f.prior, [positive])
			const right = yield* RF.new(id(166), f.prior, [threshold])
			const inputs = [
				{ identity: id(167), source: (yield* RF.describe(left)).refined },
				{ identity: id(168), source: (yield* RF.describe(right)).refined }
			]
			const common = yield* RF.common(inputs)
			assert.equal(common.length, 2)
			for (const [position, value] of common.entries()) {
				const result = yield* RF.describe(value)
				const input = inputs[position]
				assert.ok(input)
				assert.equal(yield* Event.equal(result.source, input.source), true)
				assert.ok(yield* Event.parameterEvent(result.refined, positive))
				assert.ok(yield* Event.parameterEvent(result.refined, threshold))
				assert.equal(yield* at(yield* Event.parameterMass(result.refined), 0n), "1")
			}
			const restriction = yield* RS.new(id(169), f.prior, threshold, [positive, threshold])
			const details = yield* RS.describe(yield* RS.validate(restriction))
			const heads = yield* RS.pullback(restriction, f.head)
			assert.equal(yield* at(yield* Event.parameterMass(heads), 3n, 4n), "3/4")
			assert.equal(yield* at(yield* Event.parameterMass(heads), 1n, 2n), null)
			const replay = yield* RS.fromRefinement(details.refinement, threshold)
			assert.equal(yield* Event.equal((yield* RS.describe(replay)).space, details.space), true)
			assert.equal(
				(yield* fail(
					RF.common([
						{ identity: id(170), source: f.prior },
						{ identity: id(171), source: details.space }
					])
				)).reason._tag,
				"Engine"
			)
			assert.equal((yield* fail(RS.new(id(172), f.prior, yield* R.empty(f.name)))).reason._tag, "Engine")
			assert.equal((yield* fail(RS.new(id(172), f.prior, threshold, [yield* R.full(id(99))]))).reason._tag, "Engine")
			assert.equal((yield* fail(RF.common([]))).reason._tag, "InvalidArgument")
		})
	)
})

test("family channels preserve prior marginals, share the parameter and check every parent row", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* coin(173)
			const ext = yield* Event.withParameters(yield* Event.space(id(174), 2n), f.domain, [])
			const parentBit = yield* Event.coordinate(ext, 0n)
			const outcome = yield* Event.coordinate(ext, 1n)
			const parent = yield* EventDescriptor.admit({
				kind: "map",
				map: { source: ext, target: f.prior, readouts: [parentBit] }
			})
			const density = yield* F.new(ext, [
				{ region: outcome, value: { numerator: f.p, denominator: f.one, defined: f.region } },
				{
					region: yield* Event.complement(outcome),
					value: { numerator: f.tail, denominator: f.one, defined: f.region }
				}
			])
			const kernel = yield* K.validate(yield* K.new(parent, density))
			assert.equal(yield* F.equivalent((yield* K.describe(kernel)).density, density), true)
			const observed = yield* Event.withParameters(yield* Event.space(id(175), 1n), f.domain, [])
			const readout = yield* EventDescriptor.admit({
				kind: "map",
				map: { source: ext, target: observed, readouts: [outcome] }
			})
			assert.equal(yield* K.factorsThrough(kernel, readout), true)
			assert.equal(yield* K.factorsThrough(kernel, parent), false)
			const joint = yield* K.close(kernel, f.prior)
			const old = yield* Event.coordinate(joint.space, 0n)
			const next = yield* Event.coordinate(joint.space, 1n)
			const mass = yield* Event.parameterMass(yield* Event.and(old, next))
			assert.equal(yield* at(mass, 1n, 3n), "1/9")
			assert.equal(yield* at(yield* Event.parameterMass(old), 1n, 3n), "1/3")
			const oldHead = yield* Event.coordinate(f.source, 0n)
			const certain = yield* F.designate(
				yield* F.fromFinite(yield* FiniteFunction.new(f.source, [{ region: oldHead, value: yield* Q.fraction(1n) }]))
			)
			const rebound = yield* K.close(kernel, certain)
			const zeroOld = yield* Event.complement(yield* Event.coordinate(rebound.space, 0n))
			assert.equal(yield* Event.isEmpty(zeroOld), false)
			assert.equal(yield* at(yield* Event.parameterMass(zeroOld), 1n, 3n), "0")
			const badParent = yield* EventDescriptor.admit({
				kind: "map",
				map: { source: ext, target: certain, readouts: [parentBit] }
			})
			const badDensity = yield* F.fromFinite(
				yield* FiniteFunction.new(ext, [{ region: parentBit, value: yield* Q.fraction(1n, 2n) }])
			)
			assert.equal((yield* fail(K.new(badParent, badDensity))).reason._tag, "Engine")
		})
	)
})

test("owned family updates translate stored Events through joined queries after reopen", async () => {
	const Claim = relation("Claim", { group: u64, label: u64, value: event })
	const Theory = schema("FamilyUpdates", { Claim }, [])
	const path = storeDir("family-dynamics")
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const f = yield* coin(176)
				const revision = yield* V.condition(id(177), f.prior, f.head)
				const inspection = yield* V.inspect(revision)
				assert.ok(inspection.outcome.kind === "revised")
				const refinement = (yield* RS.describe(inspection.outcome.restriction)).refinement
				const db = yield* Db.create(path, Theory)
				const draft = yield* ChangeSet.builder(Theory)
				yield* draft.insert(Claim, [
					{ group: 1n, label: 1n, value: yield* RF.lift(refinement, f.head) },
					{ group: 1n, label: 0n, value: yield* RF.lift(refinement, f.tails) }
				])
				assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
				return { revision, head: f.head, translation: inspection.outcome.translation }
			})
		)
	)
	const q = query(Theory).rule((r) => {
		const row = v(Claim)
		const evidence = v(Claim)
		return r
			.match(Claim, row)
			.match(Claim, { group: row.group, label: 1n, value: evidence.value })
			.find({
				label: row.label,
				value: EventExpr.pullback(row.value, retained.translation),
				joint: EventExpr.pullback(EventExpr.and(row.value, evidence.value), retained.translation)
			})
	})
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const db = yield* Db.open(path, Theory)
				const snapshot = yield* db.snapshot()
				const prepared = yield* snapshot.prepare(q)
				const rows = yield* (yield* prepared.execute({})).collect()
				assert.equal(rows.length, 2)
				for (const row of rows) {
					assert.equal(yield* at(yield* Event.parameterMass(row.value), 1n, 3n), row.label === 1n ? "1" : "0")
					assert.equal(yield* Event.isEmpty(row.value), false)
					assert.equal(yield* Event.isEmpty(row.joint), row.label === 0n)
				}
				const heads = rows.find((row) => row.label === 1n)
				assert.ok(heads)
				assert.equal(yield* Event.equal(heads.value, yield* V.pullback(retained.revision, retained.head)), true)
			})
		)
	)
})

test("dynamics copy submitted inputs and cancel complete owned receipts without retaining handles", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventParameter
	try {
		const f = await runtime.runPromise(coin(178))
		const revision = await runtime.runPromise(
			Effect.gen(function* () {
				const owner = yield* runtimeHandle()
				const inputs = [id(179), Event.toBytes(f.prior), Event.toBytes(f.head)]
				return yield* nativeOperationWith(
					"copied family condition",
					(cb) => {
						const operation = original(owner, "revision.condition", inputs, 0n, cb)
						for (const input of inputs) input.fill(0)
						return operation
					},
					dbNative.runtimeBytesTake,
					(value) => Result.getOrThrow(V.fromBytes(value))
				)
			})
		)
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventParameter = (handle, op, inputs, argument, cb) =>
			original(handle, op, inputs, argument, () => completed.resolve(cb))
		const fiber = runtime.runFork(V.inspect(revision))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeEventParameter = original
		assert.deepEqual((await runtime.runPromise(V.inspect(revision))).identity, id(179))
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
		const malformed = Result.getOrThrow(V.fromBytes(Buffer.from("BESC\x02\x05")))
		assert.equal((await runtime.runPromise(fail(V.validate(malformed)))).reason._tag, "Engine")
	} finally {
		dbNative.runtimeEventParameter = original
		await runtime.dispose()
	}
})

test("piecewise Jeffrey targets refine exact boundaries and skip zero-target conditionals", async () => {
	await run(
		Effect.gen(function* () {
			const f = yield* coin(180)
			const positive = yield* R.whereSign(f.name, yield* P.subtract(f.p, yield* c(1n, 2n)), S.positive)
			const lower = yield* R.and(f.region, yield* R.complement(positive))
			const upper = yield* R.and(f.region, positive)
			const zero = yield* P.zero()
			const headTarget = yield* PF.pieces(f.domain, [
				{ numerator: zero, denominator: f.one, defined: lower },
				{ numerator: f.one, denominator: f.one, defined: upper }
			])
			const tailTarget = yield* PF.subtract(yield* PF.ratio(f.domain, f.one, f.one), headTarget)
			const revision = yield* V.jeffrey(id(181), f.prior, [
				{ cell: f.head, target: headTarget },
				{ cell: f.tails, target: tailTarget }
			])
			const inspection = yield* V.inspect(yield* V.validate(revision))
			assert.ok(inspection.outcome.kind === "revised" && inspection.receipt.kind === "jeffrey")
			assert.equal(yield* R.equivalent(inspection.defined, f.region), true)
			for (const region of inspection.receipt.unsupportedRegions) assert.equal(yield* R.isEmpty(region), true)
			const target = inspection.receipt.targets[0]
			assert.ok(target)
			assert.equal(yield* PF.equivalent(target, headTarget), true)
			const heads = yield* V.pullback(revision, f.head)
			const mass = yield* Event.parameterMass(heads)
			assert.equal(yield* at(mass, 0n), "0")
			assert.equal(yield* at(mass, 1n, 2n), "0")
			assert.equal(yield* at(mass, 3n, 4n), "1")
			assert.equal(yield* at(mass, 1n), "1")
			assert.equal(
				yield* Event.containsParameter(heads, {
					parameter: { kind: "rational", value: yield* Q.fraction(0n) },
					outcomes: 1n
				}),
				true
			)
		})
	)
})
