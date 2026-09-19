import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
	bytes,
	ChangeSet,
	ParameterDomain as D,
	Db,
	describeQuery,
	Event,
	EventExpr,
	event,
	FamilyFunction,
	Guard,
	GuardPlan,
	key,
	NumberExpr as N,
	NativeRuntime,
	ExactPolynomial as P,
	ParameterRefinement,
	probability,
	ExactRational as Q,
	type QueryRow,
	query,
	queryFromDescription,
	ParameterRegion as R,
	relation,
	PolynomialSigns as S,
	schema,
	PredicateExpr as T,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { parseGuardIr } from "#query/guard-ir.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Trial = relation("Trial", { id: u64, claim: event, given: event })
const Presentation = relation("Presentation", { id: u64, source: event, identity: bytes(32) })
const Theory = schema("QueryGuards", { Trial, Presentation }, [key(Trial, ["id"]), key(Presentation, ["id"])])
const observed = query(Theory).rule((r) => {
	const c = v(Trial)
	return r.match(Trial, c).find({ id: c.id, claim: c.claim, p: probability(c.claim, c.given) })
})
function types(plan: GuardPlan) {
	const q = query(Theory).rule((r) => {
		const c = v(observed)
		// @ts-expect-error Bound source must be an Event variable.
		GuardPlan.bound(c.id)
		// @ts-expect-error Bound identity must be a bytes<32> variable.
		GuardPlan.boundRefine(c.id, c.claim)
		// @ts-expect-error Common source peers must be Event variables.
		Guard.commonSources(plan, [c.id])
		// @ts-expect-error Guard interpretation requires a predicate.
		Guard.holds(c.p, plan)
		// @ts-expect-error Guard transport requires an Event column.
		Guard.lift(T.sign(N.value(c.p), 4), plan, c.id)
		return r.match(observed, c).find({ when: Guard.holds(T.sign(N.value(c.p), 4), plan) })
	})
	const use = (row: QueryRow<typeof q>): Event => row.when
	return use
}
void types
async function run<A, E>(effect: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(effect)
	} finally {
		await runtime.dispose()
	}
}
test("guard plans and expression shape are pure, owned and strict", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const envelope = Result.getOrThrow(Event.fromBytes(Buffer.from("BEVT\x03")))
	const plan = GuardPlan.existing(envelope)
	const trial = v(Trial)
	const value = T.sign(N.integer(trial.id), 4)
	const program = query(Theory).rule((r) => r.match(Trial, trial).find({ when: Guard.holds(value, plan) }))
	const description = describeQuery(program)
	assert.deepEqual(describeQuery(queryFromDescription(Theory, description, { when: event })), description)
	assert.throws(() => GuardPlan.refine(new Uint8Array(31), envelope), /32 bytes/)
	assert.throws(() => Guard.holds(value, {} as never), /owned GuardPlan/)
	assert.throws(() => Guard.lift(value, plan, v(Trial).id as never), /Event variable/)
	assert.throws(() => Guard.common(plan, []), /nonempty/)
	assert.throws(() => Guard.common(plan, new Array(1)), /present element/)
	assert.throws(() => Guard.common(plan, [v(Trial).id as never]))
	assert.throws(() => Guard.commonSources(plan, []), /nonempty/)
	assert.throws(() => Guard.commonSources(plan, new Array(1)), /present element/)
	assert.throws(() => Guard.commonSources(plan, [v(Trial).id as never]), /Event variable/)
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r.match(Trial, trial).find({
					when: Guard.holds(value, Guard.commonSources(plan, [v(Trial).given]))
				})
			),
		/not bound/
	)
	const unbound = Guard.common(plan, [T.sign(N.integer(v(Trial).id), 4)])
	assert.throws(
		() => query(Theory).rule((r) => r.match(Trial, trial).find({ when: Guard.holds(value, unbound) })),
		/not bound/
	)
	const row = v(Presentation)
	assert.throws(() => GuardPlan.bound(row.id as never), /Event variable/)
	assert.throws(() => GuardPlan.boundRefine(row.id as never, row.source), /bytes<32>/)
	const dynamic = query(Theory).rule((r) =>
		r.match(Presentation, row).find({
			when: Guard.holds(T.sign(N.integer(row.id), 4), GuardPlan.boundRefine(row.identity, row.source))
		})
	)
	const dynamicDescription = describeQuery(dynamic)
	assert.deepEqual(describeQuery(queryFromDescription(Theory, dynamicDescription, { when: event })), dynamicDescription)
	assert.throws(
		() =>
			query(Theory).rule((r) => r.match(Trial, trial).find({ when: Guard.holds(value, GuardPlan.bound(row.source)) })),
		/not bound/
	)
	let deep = N.integer(v(Trial).id)
	for (let i = 1; i < 127; i++) deep = N.abs(deep)
	assert.throws(() => Guard.holds(T.sign(deep, 7), plan), /combined shape/)
	let wide = N.integer(v(Trial).id)
	for (let i = 0; i < 11; i++) wide = N.add(wide, wide)
	assert.throws(() => Guard.holds(T.sign(wide, 7), plan), /combined shape/)
	const predicate = { kind: "sign", number: { kind: "integer", var: 0 }, signs: 4 }
	const source = Event.toBytes(envelope)
	for (const expr of [
		...[
			[0.5],
			[-1],
			[65536],
			[Number.NaN],
			[Number.POSITIVE_INFINITY],
			[true],
			new Array(1),
			[],
			Array(4096).fill(0)
		].map((sources) => ({ kind: "holds", predicate, plan: { kind: "boundExisting", source: 0 }, sources })),
		{ kind: "holds", predicate, plan: { kind: "existing", source, identity: new Uint8Array(32) } },
		{ kind: "holds", predicate, plan: { kind: "refine", source, identity: new Uint8Array(31) } },
		{ kind: "holds", predicate, plan: { kind: "existing", source }, ignored: true },
		{ kind: "lift", predicate, plan: { kind: "existing", source }, input: -1 },
		{ kind: "holds", predicate, plan: { kind: "existing", source }, resolve: [] },
		{ kind: "holds", predicate, plan: { kind: "existing", source }, resolve: new Array(1) },
		{ kind: "holds", predicate, plan: { kind: "existing", source }, resolve: Array(2048).fill(predicate) }
	])
		assert.throws(() => parseGuardIr("test", expr))
	assert.equal(nativeBindingIsLoaded(), false)
})

test("common source guards preserve source laws and compose with predicate rosters", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const name = new Uint8Array(32).fill(220)
				const domain = yield* D.new(yield* R.full(name))
				const x = yield* P.parameter(name)
				const positive = yield* R.whereSign(name, x, S.positive)
				const negative = yield* R.whereSign(name, x, S.negative)
				const one = yield* Q.fraction(1n)
				const half = yield* Q.fraction(1n, 2n)
				const raw = yield* Event.withParameters(yield* Event.space(new Uint8Array(32).fill(221), 1n), domain, [])
				const full = yield* R.full(name)
				const source = yield* FamilyFunction.designate(
					yield* FamilyFunction.new(raw, [
						{
							region: raw,
							value: {
								numerator: yield* P.constant(half),
								denominator: yield* P.constant(one),
								defined: full
							}
						}
					])
				)
				const peerRefinement = yield* ParameterRefinement.new(new Uint8Array(32).fill(222), source, [positive])
				const peer = (yield* ParameterRefinement.describe(peerRefinement)).refined
				const identity = new Uint8Array(32).fill(223)
				const common = yield* ParameterRefinement.common([
					{ identity, source },
					{ identity: new Uint8Array(32).fill(224), source: peer }
				])
				const expectedRefinement = common[0]
				assert.ok(expectedRefinement)
				const expected = (yield* ParameterRefinement.describe(expectedRefinement)).refined
				const truth = T.equal(N.literal(one), N.literal(one))
				const lower = T.region(domain, negative)
				const db = yield* Db.create(storeDir("sdk-common-sources"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Presentation, [{ id: 1n, source, identity }])
				yield* changes.insert(Trial, [{ id: 1n, claim: yield* Event.coordinate(source, 0n), given: peer }])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const snapshot = yield* db.snapshot()
				const program = query(Theory).rule((r) => {
					const s = v(Presentation)
					const t = v(Trial)
					const peers = [t.given]
					const plan = Guard.commonSources(GuardPlan.boundRefine(s.identity, s.source), peers)
					peers.length = 0
					return r
						.match(Presentation, s)
						.match(Trial, t)
						.where(r.eq(s.id, t.id))
						.find({
							source: s.source,
							identity: s.identity,
							peer: t.given,
							full: Guard.holds(truth, plan),
							held: Guard.lift(truth, plan, t.claim),
							copy: Guard.holds(truth, Guard.commonSources(GuardPlan.refine(identity, source), [t.given, t.given])),
							negative: Guard.holds(lower, Guard.common(plan, [truth])),
							negativeCopy: Guard.holds(
								lower,
								Guard.commonSources(Guard.common(GuardPlan.boundRefine(s.identity, s.source), [truth]), [
									t.given,
									t.given
								])
							)
						})
				})
				const output = query(Theory).rule((r) => {
					const c = v(program)
					return r.match(program, c).find({
						full: c.full,
						copy: c.copy,
						negative: c.negative,
						negativeCopy: c.negativeCopy,
						original: Guard.descend(
							truth,
							Guard.commonSources(GuardPlan.boundRefine(c.identity, c.source), [c.peer]),
							c.held
						)
					})
				})
				const description = describeQuery(output)
				const replay = queryFromDescription(Theory, description, {
					full: event,
					copy: event,
					negative: event,
					negativeCopy: event,
					original: event
				})
				assert.deepEqual(describeQuery(replay), description)
				const rows = yield* (yield* snapshot.execute(replay, {})).collect()
				assert.equal(rows.length, 1)
				const row = rows[0]
				assert.ok(row)
				assert.deepEqual(Event.toBytes(row.full), Event.toBytes(expected))
				assert.deepEqual(Event.toBytes(row.copy), Event.toBytes(expected))
				assert.deepEqual(Event.toBytes(row.original), Event.toBytes(yield* Event.coordinate(source, 0n)))
				assert.deepEqual(Event.toBytes(row.negative), Event.toBytes(row.negativeCopy))
				const unresolved = query(Theory).rule((r) => {
					const t = v(Trial)
					return r
						.match(Trial, t)
						.find({ value: Guard.holds(truth, Guard.commonSources(GuardPlan.existing(source), [t.given])) })
				})
				const hidden = query(Theory).rule((r) => {
					const c = v(unresolved)
					const s = v(Presentation)
					return r.match(unresolved, c).match(Presentation, s).where(r.eq(s.id, 999n)).find(c)
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(hidden, {}))))
				return row.negative
			})
		)
	)
	for (const n of [-1n, 0n, 1n]) {
		const point = await run(Q.fraction(n))
		assert.equal(
			await run(Event.containsParameter(retained, { parameter: { kind: "rational", value: point }, outcomes: 1n })),
			n < 0n
		)
	}
})

test("query guard refinements preserve all truth cases, law, transport and owner closure", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const one = yield* Q.fraction(1n)
				const parameter = new Uint8Array(32).fill(241)
				const p = yield* P.parameter(parameter)
				const polyOne = yield* P.constant(one)
				const tail = yield* P.subtract(polyOne, p)
				const region = yield* R.and(
					yield* R.whereSign(parameter, p, S.nonNegative),
					yield* R.whereSign(parameter, tail, S.nonNegative)
				)
				const domain = yield* D.new(region)
				const raw = yield* Event.withParameters(yield* Event.space(new Uint8Array(32).fill(242), 1n), domain, [])
				const bit = yield* Event.coordinate(raw, 0n)
				const source = yield* FamilyFunction.designate(
					yield* FamilyFunction.new(raw, [
						{ region: bit, value: { numerator: p, denominator: polyOne, defined: region } },
						{ region: yield* Event.complement(bit), value: { numerator: tail, denominator: polyOne, defined: region } }
					])
				)
				const claim = yield* Event.coordinate(source, 0n)
				const identity = new Uint8Array(32).fill(243)
				const plan = GuardPlan.refine(identity, source)
				identity.fill(0)
				const db = yield* Db.create(storeDir("sdk-query-guards"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Trial, [{ id: 1n, claim, given: source }])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const snapshot = yield* db.snapshot()
				const truth = query(Theory).rule((r) => {
					const c = v(observed)
					const chance = N.value(c.p)
					// p/p is one except at the zero-evidence endpoint; compare against one.
					const value = N.divide(chance, chance)
					return r.match(observed, c).find({ id: c.id, claim: c.claim, truth: T.greaterEqual(value, N.literal(one)) })
				})
				const cases = query(Theory).rule((r) => {
					const c = v(truth)
					return r.match(truth, c).find({
						id: c.id,
						truth: c.truth,
						yes: Guard.holds(c.truth, plan),
						no: Guard.fails(c.truth, plan),
						unknown: Guard.undefined(c.truth, plan),
						held: Guard.lift(c.truth, plan, c.claim)
					})
				})
				const output = query(Theory).rule((r) => {
					const c = v(cases)
					return r.match(cases, c).find({
						yes: c.yes,
						no: c.no,
						unknown: c.unknown,
						held: c.held,
						original: Guard.descend(c.truth, plan, c.held),
						missing: EventExpr.and(c.unknown, c.held),
						whole: EventExpr.or(c.yes, EventExpr.or(c.no, c.unknown))
					})
				})
				const fields = {
					yes: event,
					no: event,
					unknown: event,
					held: event,
					original: event,
					missing: event,
					whole: event
				}
				const replay = queryFromDescription(Theory, describeQuery(output), fields)
				assert.deepEqual(describeQuery(replay), describeQuery(output))
				const rows = yield* (yield* snapshot.execute(replay, {})).collect()
				const row = rows[0]
				assert.ok(row)
				assert.deepEqual(Event.toBytes(row.original), Event.toBytes(claim))
				assert.equal(yield* Event.isEmpty(row.no), true)
				assert.equal(yield* Event.isFull(row.whole), true)
				assert.equal(yield* Event.isEmpty(row.missing), false)
				for (const n of [0n, 1n, 2n]) {
					const point = yield* Q.fraction(n, 2n)
					for (const outcomes of [0n, 1n]) {
						assert.equal(
							yield* Event.containsParameter(row.yes, { parameter: { kind: "rational", value: point }, outcomes }),
							n > 0n
						)
						assert.equal(
							yield* Event.containsParameter(row.unknown, { parameter: { kind: "rational", value: point }, outcomes }),
							n === 0n
						)
						assert.equal(
							yield* Event.containsParameter(row.held, { parameter: { kind: "rational", value: point }, outcomes }),
							outcomes === 1n
						)
					}
				}
				const unresolved = query(Theory).rule((r) => {
					const c = v(truth)
					return r.match(truth, c).find({ when: Guard.holds(c.truth, GuardPlan.existing(source)) })
				})
				const hidden = query(Theory).rule((r) => {
					const c = v(unresolved)
					const t = v(Trial)
					return r.match(unresolved, c).match(Trial, t).where(r.eq(t.id, 999n)).find(c)
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(hidden, {}))))
				const essential = query(Theory).rule((r) => {
					const c = v(cases)
					return r.match(cases, c).find({ when: Guard.descend(c.truth, plan, c.yes) })
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(essential, {}))))
				const corruptPlan = GuardPlan.existing(Result.getOrThrow(Event.fromBytes(Buffer.from("BEVT\x03"))))
				const corrupt = query(Theory).rule((r) => {
					const c = v(truth)
					const t = v(Trial)
					return r
						.match(truth, c)
						.match(Trial, t)
						.where(r.eq(t.id, 999n))
						.find({ when: Guard.holds(c.truth, corruptPlan) })
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(corrupt, {}))))
				const commonPlan = GuardPlan.refine(new Uint8Array(32).fill(244), source)
				const half = yield* Q.fraction(1n, 2n)
				const jointTruth = query(Theory).rule((r) => {
					const c = v(observed)
					const p = N.value(c.p)
					return r.match(observed, c).find({
						claim: c.claim,
						low: T.less(p, N.literal(half)),
						high: T.greater(p, N.literal(half)),
						known: T.greaterEqual(N.divide(p, p), N.literal(one))
					})
				})
				const jointCases = query(Theory).rule((r) => {
					const c = v(jointTruth)
					const roster = [c.low, c.high, c.known]
					const context = Guard.common(commonPlan, roster)
					roster.length = 0 // The context owns the authored roster.
					return r.match(jointTruth, c).find({
						low: c.low,
						high: c.high,
						known: c.known,
						a: Guard.holds(c.low, context),
						b: Guard.holds(c.high, Guard.common(commonPlan, [c.known, c.low, c.high, c.low])),
						hole: Guard.undefined(c.known, context),
						held: Guard.lift(c.low, context, c.claim)
					})
				})
				const joint = query(Theory).rule((r) => {
					const c = v(jointCases)
					return r.match(jointCases, c).find({
						a: c.a,
						b: c.b,
						hole: c.hole,
						both: EventExpr.and(c.a, c.b),
						middle: EventExpr.complement(EventExpr.or(c.a, c.b)),
						original: Guard.descend(c.known, Guard.common(commonPlan, [c.high, c.low]), c.held)
					})
				})
				const jointReplay = queryFromDescription(Theory, describeQuery(joint), {
					a: event,
					b: event,
					hole: event,
					both: event,
					middle: event,
					original: event
				})
				assert.deepEqual(describeQuery(jointReplay), describeQuery(joint))
				const jointRows = yield* (yield* snapshot.execute(jointReplay, {})).collect()
				const j = jointRows[0]
				assert.ok(j)
				assert.deepEqual(Event.toBytes(j.original), Event.toBytes(claim))
				assert.equal(yield* Event.isEmpty(j.both), true)
				for (const n of [0n, 1n, 2n]) {
					const point = yield* Q.fraction(n, 2n)
					const world = { parameter: { kind: "rational" as const, value: point }, outcomes: 1n }
					assert.equal(yield* Event.containsParameter(j.a, world), n < 1n)
					assert.equal(yield* Event.containsParameter(j.b, world), n > 1n)
					assert.equal(yield* Event.containsParameter(j.hole, world), n === 0n)
					assert.equal(yield* Event.containsParameter(j.middle, world), n === 1n)
				}
				const badCommon = query(Theory).rule((r) => {
					const c = v(jointTruth)
					return r
						.match(jointTruth, c)
						.find({ when: Guard.holds(T.sign(N.literal(one), 4), Guard.common(GuardPlan.existing(source), [c.low])) })
				})
				const hiddenCommon = query(Theory).rule((r) => {
					const c = v(badCommon)
					const t = v(Trial)
					return r.match(badCommon, c).match(Trial, t).where(r.eq(t.id, 999n)).find(c)
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(hiddenCommon, {}))))
				const result = yield* snapshot.execute(output, {})
				yield* snapshot.close()
				yield* db.close()
				return yield* result.collect()
			})
		)
	)
	assert.equal(retained.length, 1)
	assert.ok(retained.every((row) => Object.values(row).every(Event.isEvent)))
})

test("row-bound guard plans retain stored sources and presentation identities through staging", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const parameter = new Uint8Array(32).fill(250)
				const domain = yield* D.new(yield* R.full(parameter))
				const p = yield* P.parameter(parameter)
				const positive = yield* R.whereSign(parameter, p, S.positive)
				const truth = T.region(domain, positive)
				const one = yield* Q.fraction(1n)
				const constant = T.equal(N.literal(one), N.literal(one))
				const sourceA = yield* Event.withParameters(yield* Event.space(new Uint8Array(32).fill(251), 1n), domain, [])
				const sourceB = yield* Event.withParameters(yield* Event.space(new Uint8Array(32).fill(252), 1n), domain, [])
				const sources = [sourceA, sourceB]
				const identities = [
					Uint8Array.from({ length: 32 }, (_, i) => i),
					Uint8Array.from({ length: 32 }, (_, i) => 255 - i)
				]
				const db = yield* Db.create(storeDir("sdk-bound-guards"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				for (const [i, source] of sources.entries()) {
					const identity = identities[i]
					assert.ok(identity)
					yield* changes.insert(Presentation, [{ id: BigInt(i), source, identity }])
					yield* changes.insert(Trial, [{ id: BigInt(i), claim: yield* Event.coordinate(source, 0n), given: source }])
				}
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const snapshot = yield* db.snapshot()
				const cases = query(Theory).rule((r) => {
					const s = v(Presentation)
					const t = v(Trial)
					const plan = Guard.common(GuardPlan.boundRefine(s.identity, s.source), [truth, constant])
					return r
						.match(Presentation, s)
						.match(Trial, t)
						.where(r.eq(s.id, t.id))
						.find({
							id: s.id,
							source: s.source,
							identity: s.identity,
							yes: Guard.holds(truth, plan),
							no: Guard.fails(truth, plan),
							held: Guard.lift(truth, plan, t.claim)
						})
				})
				const program = query(Theory).rule((r) => {
					const c = v(cases)
					const plan = Guard.common(GuardPlan.boundRefine(c.identity, c.source), [truth, constant])
					return r
						.match(cases, c)
						.find({ id: c.id, yes: c.yes, no: c.no, original: Guard.descend(truth, plan, c.held) })
				})
				const description = describeQuery(program)
				const fields = { id: u64, yes: event, no: event, original: event }
				const replay = queryFromDescription(Theory, description, fields)
				assert.deepEqual(describeQuery(replay), description)
				const rows = yield* (yield* snapshot.execute(replay, {})).collect()
				assert.equal(rows.length, 2)
				for (const row of rows) {
					const i = Number(row.id)
					const source = sources[i]
					const identity = identities[i]
					assert.ok(source && identity)
					const expected = query(Theory).rule((r) =>
						r.match(Trial, v(Trial)).find({
							yes: Guard.holds(truth, Guard.common(GuardPlan.refine(identity, source), [truth, constant]))
						})
					)
					const captured = yield* (yield* snapshot.execute(expected, {})).collect()
					assert.equal(captured.length, 1)
					const first = captured[0]
					assert.ok(first)
					assert.deepEqual(Event.toBytes(row.yes), Event.toBytes(first.yes))
					assert.equal(yield* Event.equal(row.no, yield* Event.complement(row.yes)), true)
					assert.deepEqual(Event.toBytes(row.original), Event.toBytes(yield* Event.coordinate(source, 0n)))
				}
				const existing = query(Theory).rule((r) => {
					const s = v(Presentation)
					return r.match(Presentation, s).find({ id: s.id, value: Guard.holds(constant, GuardPlan.bound(s.source)) })
				})
				assert.equal((yield* (yield* snapshot.execute(existing, {})).collect()).length, 2)
				const bad = query(Theory).rule((r) => {
					const t = v(Trial)
					return r.match(Trial, t).find({ value: Guard.holds(constant, GuardPlan.bound(t.claim)) })
				})
				const downstream = query(Theory).rule((r) => {
					const c = v(bad)
					const t = v(Trial)
					return r.match(bad, c).match(Trial, t).where(r.eq(t.id, 99n)).find(c)
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(downstream, {}))))
				return rows.map((row) => row.yes)
			})
		)
	)
	for (const value of retained) assert.equal(await run(Event.isEmpty(value)), false)
})
