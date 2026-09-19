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
	event,
	FiniteFunction,
	FiniteKernel,
	NativeRuntime,
	query,
	relation,
	SourceRevision,
	type SourceRevisionInspection,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const identity = (n: number) => new Uint8Array(32).fill(n)
const rational = ExactRational.fraction
const text = ExactRational.toString
const fail = <E, R>(value: Effect.Effect<unknown, E, R>) => Effect.flip(value)
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}
const prior = (id: number, numerator: bigint, denominator: bigint) =>
	Effect.gen(function* () {
		const raw = yield* Event.space(identity(id), 1n)
		const yes = yield* Event.coordinate(raw, 0n)
		return yield* FiniteFunction.designate(
			yield* FiniteFunction.new(raw, [
				{ region: yes, value: yield* rational(numerator, denominator) },
				{ region: yield* Event.complement(yes), value: yield* rational(denominator - numerator, denominator) }
			])
		)
	})
function typePins(s: Event, f: FiniteFunction, k: FiniteKernel, r: SourceRevision) {
	// @ts-expect-error A channel is not a finite function.
	FiniteFunction.designate(k)
	// @ts-expect-error A revision is not a stored Event.
	Event.and(s, r)
	// @ts-expect-error A function does not carry a checked conditional normalization.
	FiniteKernel.close(f, s)
	// @ts-expect-error Jeffrey targets are exact rationals, not numbers.
	SourceRevision.jeffrey(s, [{ cell: s, target: 1 }])
}
void typePins

test("kernel/revision envelopes own bytes, retain roles, and construct Effects without an addon", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const input = Buffer.from("BESC\x01\x01")
	const kernel = Result.getOrThrow(FiniteKernel.fromBytes(input))
	input.fill(0)
	assert.deepEqual(Buffer.from(FiniteKernel.toBytes(kernel)), Buffer.from("BESC\x01\x01"))
	FiniteKernel.toBytes(kernel).fill(0)
	assert.equal(FiniteKernel.isFiniteKernel(kernel), true)
	assert.equal(FiniteKernel.isFiniteKernel({ ...kernel }), false)
	assert.ok(Result.isFailure(SourceRevision.fromBytes(FiniteKernel.toBytes(kernel))))
	const revision = Result.getOrThrow(SourceRevision.fromBytes(Buffer.from("BESC\x01\x02")))
	assert.equal(SourceRevision.isSourceRevision(revision), true)
	assert.equal(SourceRevision.isSourceRevision({ ...revision }), false)
	assert.ok(Result.isFailure(FiniteKernel.fromBytes(SourceRevision.toBytes(revision))))
	assert.ok(Result.isFailure(SourceRevision.fromBytes(new Uint8Array(new SharedArrayBuffer(6)))))
	FiniteKernel.describe(kernel)
	SourceRevision.inspect(revision)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("posterior replacement, likelihood and conditioning have distinct exact meanings", async () => {
	await run(
		Effect.gen(function* () {
			const s = yield* prior(241, 1n, 5n)
			const a = yield* Event.coordinate(s, 0n)
			const notA = yield* Event.complement(a)
			const fourFifths = yield* rational(4n, 5n)
			const fifth = yield* rational(1n, 5n)
			const likelihood = yield* FiniteFunction.new(s, [
				{ region: a, value: fourFifths },
				{ region: notA, value: fifth }
			])
			const revisions = [
				yield* SourceRevision.jeffrey(s, [
					{ cell: a, target: fourFifths },
					{ cell: notA, target: fifth }
				]),
				yield* SourceRevision.likelihood(s, likelihood),
				yield* SourceRevision.condition(s, a)
			]
			for (const [i, value] of revisions.entries()) {
				const encoded = SourceRevision.toBytes(value)
				const owned = Result.getOrThrow(SourceRevision.fromBytes(encoded))
				encoded.fill(0)
				assert.deepEqual(SourceRevision.toBytes(yield* SourceRevision.validate(owned)), SourceRevision.toBytes(value))
				const data = yield* SourceRevision.inspect(owned)
				assert.equal(yield* Event.equal(data.prior, s), true)
				assert.equal(data.outcome.kind, "revised")
				assert.ok(data.outcome.kind === "revised")
				const posterior = data.outcome.posterior
				assert.equal(yield* Event.count(posterior), 2n, "zero posterior mass does not remove possible worlds")
				assert.equal(yield* text(yield* Event.mass(yield* Event.coordinate(posterior, 0n))), ["4/5", "1/2", "1"][i])
				const map = yield* EventDescriptor.inspect(data.outcome.translation)
				assert.ok(map.kind === "surjective")
				assert.equal(yield* Event.equal(map.source, posterior), true)
				assert.equal(yield* Event.equal(map.target, s), true)
				assert.equal((yield* fail(Event.and(a, yield* Event.coordinate(posterior, 0n)))).reason._tag, "Engine")
				if (data.receipt.kind === "jeffrey") {
					assert.deepEqual(yield* Effect.forEach(data.receipt.oldMasses, text), ["1/5", "4/5"])
					assert.deepEqual(yield* Effect.forEach(data.receipt.targets, text), ["4/5", "1/5"])
				} else if (data.receipt.kind === "likelihood") {
					assert.equal(yield* text(data.receipt.normalizer), "8/25")
					assert.equal(yield* FiniteFunction.equivalent(data.receipt.likelihood, likelihood), true)
				} else {
					assert.equal(yield* text(data.receipt.mass), "1/5")
					assert.ok(map.readouts[0])
					const repeated = yield* SourceRevision.inspect(yield* SourceRevision.condition(posterior, map.readouts[0]))
					assert.ok(repeated.outcome.kind === "revised")
					assert.equal(yield* Event.equal(repeated.outcome.posterior, posterior), true)
				}
			}
			// Scale affects the receipt, not the revised law; factors need not be probabilities.
			const scaled = yield* SourceRevision.inspect(
				yield* SourceRevision.likelihood(
					s,
					yield* FiniteFunction.multiply(likelihood, yield* FiniteFunction.constant(s, yield* rational(10n)))
				)
			)
			assert.ok(scaled.receipt.kind === "likelihood" && scaled.outcome.kind === "revised")
			assert.equal(yield* text(scaled.receipt.normalizer), "16/5")
			assert.equal(yield* text(yield* Event.mass(yield* Event.coordinate(scaled.outcome.posterior, 0n))), "1/2")
		})
	)
})

test("impossible revisions retain zero-mass and empty indexed cells without hiding input errors", async () => {
	let retained: SourceRevisionInspection | undefined
	await run(
		Effect.gen(function* () {
			const s = yield* prior(242, 1n, 1n)
			const a = yield* Event.coordinate(s, 0n)
			const b = yield* Event.complement(a)
			const empty = yield* Event.empty(s)
			const zero = yield* rational(0n)
			const one = yield* rational(1n)
			const quarter = yield* rational(1n, 4n)
			assert.equal(yield* Event.count(b), 1n)
			const condition = yield* SourceRevision.inspect(yield* SourceRevision.condition(s, b))
			assert.deepEqual(condition.outcome, { kind: "impossible", cause: "zeroEvidence" })
			assert.ok(condition.receipt.kind === "condition")
			assert.equal(yield* text(condition.receipt.mass), "0")
			const like = yield* SourceRevision.inspect(
				yield* SourceRevision.likelihood(s, yield* FiniteFunction.new(s, [{ region: b, value: one }]))
			)
			assert.deepEqual(like.outcome, { kind: "impossible", cause: "zeroLikelihood" })
			const cells = [empty, b, a, empty]
			const revision = yield* SourceRevision.jeffrey(
				s,
				cells.map((cell) => ({ cell, target: quarter }))
			)
			retained = yield* SourceRevision.inspect(
				yield* SourceRevision.validate(Result.getOrThrow(SourceRevision.fromBytes(SourceRevision.toBytes(revision))))
			)
			assert.deepEqual(retained.outcome, { kind: "impossible", cause: "unsupportedTargets", cells: [0, 1, 3] })
			assert.ok(retained.receipt.kind === "jeffrey")
			assert.deepEqual(yield* Effect.forEach(retained.receipt.oldMasses, text), ["0", "0", "1", "0"])
			const valid = yield* SourceRevision.inspect(
				yield* SourceRevision.jeffrey(
					s,
					cells.map((cell, i) => ({ cell, target: i === 2 ? one : zero }))
				)
			)
			assert.ok(valid.outcome.kind === "revised" && valid.receipt.kind === "jeffrey")
			assert.equal(valid.receipt.cells.length, 4)
			assert.equal(yield* Event.equal(valid.outcome.posterior, s), true)
			const foreign = yield* Event.space(identity(243), 1n)
			for (const operation of [
				SourceRevision.condition(a, a), // prior must be full
				SourceRevision.condition(s, yield* Event.empty(foreign)),
				SourceRevision.condition(foreign, foreign), // missing law
				SourceRevision.likelihood(s, yield* FiniteFunction.constant(s, yield* rational(-1n))),
				SourceRevision.likelihood(s, yield* FiniteFunction.new(foreign, [])),
				SourceRevision.jeffrey(s, []),
				SourceRevision.jeffrey(s, [{ cell: a, target: one }]), // gap
				SourceRevision.jeffrey(s, [
					{ cell: s, target: one },
					{ cell: a, target: zero }
				]), // overlap even at zero target
				SourceRevision.jeffrey(s, [{ cell: s, target: quarter }]),
				SourceRevision.jeffrey(s, [
					{ cell: a, target: yield* rational(-1n) },
					{ cell: b, target: yield* rational(2n) }
				]),
				SourceRevision.jeffrey(s, [
					{ cell: s, target: one },
					{ cell: yield* Event.empty(foreign), target: zero }
				])
			])
				assert.equal((yield* fail(operation)).reason._tag, "Engine")
			let reads = 0
			const getter = {
				get cell() {
					reads++
					return s
				},
				target: one
			}
			for (const targets of [
				[getter],
				new Array(1),
				Array(2048).fill({ cell: s, target: one }),
				[{ cell: s, target: one, extra: true }],
				[{ cell: s, target: 1 }]
			]) {
				assert.equal((yield* fail(SourceRevision.jeffrey(s, targets as never))).reason._tag, "InvalidArgument")
			}
			assert.equal(reads, 0)
		})
	)
	assert.ok(retained?.receipt.kind === "jeffrey")
	assert.ok(Object.isFrozen(retained) && Object.isFrozen(retained.receipt.cells))
	assert.ok(Event.toBytes(retained.prior).length > 5)
})

test("BESC replay refuses forged receipt numbers, posterior laws and incomplete impossible outcomes", async () => {
	await run(
		Effect.gen(function* () {
			const s = yield* prior(244, 1n, 5n)
			const a = yield* Event.coordinate(s, 0n)
			const revision = yield* SourceRevision.condition(s, a)
			const data = yield* SourceRevision.inspect(revision)
			assert.ok(data.receipt.kind === "condition" && data.outcome.kind === "revised")
			const blob = (bytes: Uint8Array) => {
				const n = Buffer.alloc(8)
				n.writeBigUInt64LE(BigInt(bytes.length))
				return Buffer.concat([n, bytes])
			}
			const wire = (mass: ExactRational, posterior: Event) =>
				Buffer.concat([
					Buffer.from("BESC\x01\x02"),
					blob(Event.toBytes(s)),
					Buffer.from([0]),
					blob(Event.toBytes(a)),
					blob(ExactRational.toBytes(mass)),
					Buffer.from([0]),
					blob(Event.toBytes(posterior))
				])
			assert.deepEqual(Buffer.from(SourceRevision.toBytes(revision)), wire(data.receipt.mass, data.outcome.posterior))
			for (const bytes of [
				wire(yield* rational(2n, 5n), data.outcome.posterior),
				wire(data.receipt.mass, s),
				Buffer.from("BESC\x01\x02")
			]) {
				assert.equal(
					(yield* fail(SourceRevision.validate(Result.getOrThrow(SourceRevision.fromBytes(bytes))))).reason._tag,
					"Engine"
				)
			}
			const point = yield* prior(245, 1n, 1n)
			const no = yield* Event.complement(yield* Event.coordinate(point, 0n))
			const impossible = yield* SourceRevision.jeffrey(point, [
				{ cell: yield* Event.empty(point), target: yield* rational(1n, 3n) },
				{ cell: no, target: yield* rational(1n, 3n) },
				{ cell: yield* Event.coordinate(point, 0n), target: yield* rational(1n, 3n) }
			])
			const forged = Buffer.from(SourceRevision.toBytes(impossible))
			// Last outcome is tag 3, count 2, indices [0, 1]. Claim [0, 0] instead.
			forged.writeBigUInt64LE(0n, forged.length - 8)
			assert.equal(
				(yield* fail(SourceRevision.validate(Result.getOrThrow(SourceRevision.fromBytes(forged))))).reason._tag,
				"Engine"
			)
		})
	)
})

test("channels normalize on zero-prior fibres and copy readouts without fresh randomness", async () => {
	await run(
		Effect.gen(function* () {
			const p = yield* prior(246, 1n, 1n)
			const ext = yield* Event.space(identity(247), 2n)
			const parentBit = yield* Event.coordinate(ext, 0n)
			const parent = yield* EventDescriptor.admit({
				kind: "map",
				map: { source: ext, target: p, readouts: [parentBit] }
			})
			const half = yield* rational(1n, 2n)
			const invalid = yield* FiniteFunction.new(ext, [{ region: parentBit, value: half }])
			assert.equal((yield* fail(FiniteKernel.new(parent, invalid))).reason._tag, "Engine")
			const k = yield* FiniteKernel.new(parent, yield* FiniteFunction.constant(ext, half))
			const encoded = FiniteKernel.toBytes(k)
			const owned = Result.getOrThrow(FiniteKernel.fromBytes(encoded))
			encoded.fill(0)
			assert.deepEqual(FiniteKernel.toBytes(yield* FiniteKernel.validate(owned)), FiniteKernel.toBytes(k))
			const described = yield* FiniteKernel.describe(owned)
			assert.deepEqual(
				FiniteKernel.toBytes(yield* FiniteKernel.new(described.parent, described.density)),
				FiniteKernel.toBytes(k)
			)
			const closed = yield* FiniteKernel.close(owned, p)
			assert.equal(yield* Event.count(closed.space), 4n)
			assert.equal(yield* text(yield* Event.mass(yield* Event.coordinate(closed.space, 1n))), "1/2")
			const marginal = yield* FiniteFunction.pushforward(yield* FiniteFunction.density(closed.space), closed.parent)
			assert.equal(yield* FiniteFunction.equivalent(marginal, yield* FiniteFunction.density(p)), true)
			// Reusing the channel under a different explicit prior is supported.
			const alternate = yield* prior(246, 1n, 5n)
			const rebound = yield* FiniteKernel.close(owned, alternate)
			assert.equal(yield* text(yield* Event.mass(yield* Event.coordinate(rebound.space, 0n))), "1/5")
			const copySource = yield* Event.restrict(
				ext,
				yield* Event.equivalence(parentBit, yield* Event.coordinate(ext, 1n))
			)
			const copyMap = yield* EventDescriptor.admit({
				kind: "map",
				map: {
					source: copySource,
					target: alternate,
					readouts: [yield* Event.coordinate(copySource, 0n)]
				}
			})
			const copy = yield* FiniteKernel.close(
				yield* FiniteKernel.new(copyMap, yield* FiniteFunction.constant(copySource, yield* rational(1n))),
				alternate
			)
			assert.equal(yield* Event.count(copy.space), 2n)
			assert.equal(
				yield* Event.equal(yield* Event.coordinate(copy.space, 0n), yield* Event.coordinate(copy.space, 1n)),
				true
			)
			assert.equal(yield* text(yield* Event.mass(yield* Event.coordinate(copy.space, 1n))), "1/5")
			for (const operation of [
				FiniteKernel.new(parent, yield* FiniteFunction.constant(ext, yield* rational(-1n))),
				FiniteKernel.new(parent, yield* FiniteFunction.constant(ext, yield* rational(1n))),
				FiniteKernel.new(parent, yield* FiniteFunction.constant(p, half)),
				FiniteKernel.close(k, yield* Event.coordinate(p, 0n)),
				FiniteKernel.close(k, yield* Event.space(identity(246), 1n)),
				FiniteKernel.close(k, yield* prior(248, 1n, 2n)),
				FiniteKernel.validate(Result.getOrThrow(FiniteKernel.fromBytes(Buffer.from("BESC\x01\x01"))))
			])
				assert.equal((yield* fail(operation)).reason._tag, "Engine")
		})
	)
})

test("Coup channels obey visibility FDs and translate stored Events through posterior queries after reopen", async () => {
	const Claims = relation("Claim", { id: u64, value: event })
	const Theory = schema("CoupDynamics", { Claim: Claims }, [])
	const path = storeDir("dynamics-coup")
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const base = yield* Event.space(identity(249), 2n)
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
				const p = yield* FiniteFunction.designate(yield* FiniteFunction.new(base, pieces))
				const ext = yield* Event.space(identity(250), 3n)
				const b = yield* Event.coordinate(ext, 0n)
				const c = yield* Event.coordinate(ext, 1n)
				const tax = yield* Event.coordinate(ext, 2n)
				const parent = yield* EventDescriptor.admit({ kind: "map", map: { source: ext, target: p, readouts: [b, c] } })
				const channel = (e: Event) =>
					Effect.gen(function* () {
						const likely = yield* Event.equivalence(e, tax)
						return yield* FiniteKernel.new(
							parent,
							yield* FiniteFunction.new(ext, [
								{ region: likely, value: yield* rational(4n, 5n) },
								{ region: yield* Event.complement(likely), value: yield* rational(1n, 5n) }
							])
						)
					})
				const k = yield* channel(b)
				const view = yield* Event.space(identity(251), 2n)
				const visible = yield* EventDescriptor.admit({
					kind: "map",
					map: { source: ext, target: view, readouts: [b, tax] }
				})
				assert.equal(yield* FiniteKernel.factorsThrough(k, visible), true)
				assert.equal(
					yield* FiniteKernel.factorsThrough(yield* channel(c), visible),
					false,
					"Bob's policy may not depend on Cleo's hidden card"
				)
				const notOnto = yield* EventDescriptor.admit({
					kind: "map",
					map: { source: ext, target: view, readouts: [b, b] }
				})
				assert.equal((yield* fail(FiniteKernel.factorsThrough(k, notOnto))).reason._tag, "Engine")
				const closed = yield* FiniteKernel.close(k, p)
				const evidence = yield* Event.coordinate(closed.space, 2n)
				assert.equal(yield* text(yield* Event.mass(evidence)), "49/130")
				const revision = yield* SourceRevision.condition(closed.space, evidence)
				const inspection = yield* SourceRevision.inspect(revision)
				assert.ok(inspection.outcome.kind === "revised")
				const db = yield* Db.create(path, Theory)
				const draft = yield* ChangeSet.builder(Theory)
				yield* draft.insert(Claims, [
					{ id: 1n, value: yield* Event.coordinate(p, 0n) },
					{ id: 2n, value: yield* Event.coordinate(p, 1n) }
				])
				assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
				return { k, closed, revision, outcome: inspection.outcome }
			})
		)
	)
	// Everything captured here is owned transport; the constructing runtime is gone.
	const q = query(Theory).rule((r) => {
		const row = v(Claims)
		return r.match(Claims, row).find({
			id: row.id,
			value: EventExpr.pullback(EventExpr.pullback(row.value, retained.closed.parent), retained.outcome.translation)
		})
	})
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const db = yield* Db.open(path, Theory)
				const snapshot = yield* db.snapshot()
				const prepared = yield* snapshot.prepare(q)
				const results = yield* (yield* prepared.execute({})).collect()
				assert.equal(results.length, 2)
				for (const row of results)
					assert.equal(yield* text(yield* Event.mass(row.value)), row.id === 1n ? "92/147" : "5/21")
				assert.equal(yield* Event.count(retained.outcome.posterior), 8n)
				const replay = yield* SourceRevision.inspect(yield* SourceRevision.validate(retained.revision))
				assert.ok(replay.outcome.kind === "revised")
				assert.equal(yield* Event.equal(replay.outcome.posterior, retained.outcome.posterior), true)
				assert.ok(yield* FiniteKernel.validate(retained.k))
			})
		)
	)
})

test("cancelled revision inspection reclaims owned outputs and leaves the runtime reusable", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer({ ...runtimeOptions, nativeHandleCapacity: 1 }))
	const original = dbNative.runtimeEventSource
	try {
		const revision = await runtime.runPromise(
			Effect.gen(function* () {
				const s = yield* prior(252, 1n, 2n)
				return yield* SourceRevision.condition(s, yield* Event.coordinate(s, 0n))
			})
		)
		const completed = Promise.withResolvers<() => void>()
		dbNative.runtimeEventSource = (handle, operation, inputs, argument, callback) =>
			original(handle, operation, inputs, argument, () => completed.resolve(callback))
		const fiber = runtime.runFork(SourceRevision.inspect(revision))
		const late = await completed.promise
		await Effect.runPromise(Fiber.interrupt(fiber))
		assert.ok(Exit.hasInterrupts(await Effect.runPromise(Fiber.await(fiber))))
		late()
		dbNative.runtimeEventSource = original
		assert.equal((await runtime.runPromise(SourceRevision.inspect(revision))).outcome.kind, "revised")
		const service = await runtime.runPromise(NativeRuntime)
		assert.equal((await runtime.runPromise(service.inspect())).retained, 0n)
	} finally {
		dbNative.runtimeEventSource = original
		await runtime.dispose()
	}
})
