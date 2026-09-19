import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
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
const Theory = schema("QueryGuards", { Trial }, [key(Trial, ["id"])])
const observed = query(Theory).rule((r) => {
	const c = v(Trial)
	return r.match(Trial, c).find({ id: c.id, claim: c.claim, p: probability(c.claim, c.given) })
})
function types(plan: GuardPlan) {
	const q = query(Theory).rule((r) => {
		const c = v(observed)
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
	let deep = N.integer(v(Trial).id)
	for (let i = 1; i < 127; i++) deep = N.abs(deep)
	assert.throws(() => Guard.holds(T.sign(deep, 7), plan), /combined shape/)
	let wide = N.integer(v(Trial).id)
	for (let i = 0; i < 11; i++) wide = N.add(wide, wide)
	assert.throws(() => Guard.holds(T.sign(wide, 7), plan), /combined shape/)
	const predicate = { kind: "sign", number: { kind: "integer", var: 0 }, signs: 4 }
	const source = Event.toBytes(envelope)
	for (const expr of [
		{ kind: "holds", predicate, plan: { kind: "existing", source, identity: new Uint8Array(32) } },
		{ kind: "holds", predicate, plan: { kind: "refine", source, identity: new Uint8Array(31) } },
		{ kind: "holds", predicate, plan: { kind: "existing", source }, ignored: true },
		{ kind: "lift", predicate, plan: { kind: "existing", source }, input: -1 }
	])
		assert.throws(() => parseGuardIr("test", expr))
	assert.equal(nativeBindingIsLoaded(), false)
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
