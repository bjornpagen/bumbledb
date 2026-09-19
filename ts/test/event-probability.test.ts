import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Stream } from "effect"
import {
	ChangeSet,
	Db,
	describeQuery,
	Event,
	EventExpr,
	event,
	FamilyFunction,
	FiniteFunction,
	key,
	NativeRuntime,
	ExactPolynomial as P,
	ParameterDomain,
	ParameterFunction,
	ParameterRegion,
	PolynomialSigns,
	type ProbabilityAnswer,
	probability,
	probabilityResult,
	ExactRational as Q,
	type QueryRow,
	query,
	queryFromDescription,
	relation,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Claim = relation("Claim", { id: u64, event, given: event })
const Theory = schema("ProbabilityQueries", { Claim }, [key(Claim, ["id"])])
const selected = query(Theory).rule((r) => {
	const row = v(Claim)
	return r.match(Claim, row).find({ event: EventExpr.variable(row.event), given: EventExpr.variable(row.given) })
})
const observed = query(Theory).rule((r) => {
	const row = v(selected)
	return r.match(selected, row).find({ chance: probability(row.event, row.given) })
})
function typePins(answer: QueryRow<typeof observed>) {
	const retained: ProbabilityAnswer = answer.chance
	void retained
	// @ts-expect-error Observations are not Event operands.
	EventExpr.complement(answer.chance)
	// @ts-expect-error Result descriptors are not schema fields.
	relation("Invalid", { chance: probabilityResult })
	const row = v(Claim)
	// @ts-expect-error The observed proposition must be an Event program.
	probability(row.id, row.given)
}
void typePins
const id = (n: number) => new Uint8Array(32).fill(n)
const text = (value: Q | null) => (value === null ? Effect.succeed(null) : Q.toString(value))
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}

test("probability authoring and description imports stay pure, owned and query-only", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const description = describeQuery(observed)
	const restored = queryFromDescription(Theory, description, { chance: probabilityResult })
	assert.deepEqual(describeQuery(restored), description)
	assert.throws(() => queryFromDescription(Theory, description, { chance: event }), /field domains/)
	assert.equal(v(observed).chance.field.kind, "probability")
	const row = v(Claim)
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r.match(Claim, { event: row.event }).find({
					chance: probability(row.event, row.given)
				})
			),
		/not bound/
	)
	assert.throws(() => observed.rule((r) => r.match(Claim, row).find({ chance: row.event })), /same head/)
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r.match(Claim, row).find({
					chance: { ...probability(row.event, row.given) }
				})
			),
		/not a find entry/
	)
	let wide = EventExpr.variable(row.event)
	for (let i = 0; i < 11; i++) wide = EventExpr.and(wide, wide)
	assert.throws(() => probability(wide, wide), /shape/)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("fixed query observations preserve provenance, zero-mass evidence, duplicate paths and sealed pages", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const base = yield* Event.space(id(219), 2n)
				const a = yield* Event.coordinate(base, 0n)
				const b = yield* Event.coordinate(base, 1n)
				const source = yield* FiniteFunction.designate(
					yield* FiniteFunction.new(base, [
						{ region: yield* Event.and(a, yield* Event.complement(b)), value: yield* Q.fraction(1n, 2n) },
						{ region: yield* Event.and(b, yield* Event.complement(a)), value: yield* Q.fraction(1n, 2n) }
					])
				)
				const first = yield* Event.coordinate(source, 0n)
				const second = yield* Event.coordinate(source, 1n)
				const evidence = yield* Event.and(yield* Event.complement(first), yield* Event.complement(second))
				const db = yield* Db.create(storeDir("query-probability-fixed"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Claim, [
					{ id: 1n, event: first, given: source },
					{ id: 2n, event: second, given: source },
					{ id: 3n, event: first, given: source },
					{ id: 4n, event: source, given: evidence }
				])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const duplicate = observed.rule((r) => {
					const row = v(Claim)
					return r.match(Claim, row).find({ chance: probability(row.event, row.given) })
				})
				const restored = queryFromDescription(Theory, describeQuery(duplicate), { chance: probabilityResult })
				const snapshot = yield* db.snapshot()
				const result = yield* snapshot.execute(restored, {})
				yield* snapshot.close()
				yield* db.close()
				const rows = yield* result.collect()
				assert.equal(rows.length, 3)
				const pages = yield* Stream.runCollect(result.pages())
				assert.equal(Array.from(pages).flat().length, 3)
				const summary: (string | null)[] = []
				for (const { chance } of rows) {
					assert.equal(chance.law, "fixed")
					if (chance.law !== "fixed") throw new Error("fixed law")
					summary.push(yield* text(chance.value))
					if (chance.value === null) {
						assert.equal(yield* Event.isEmpty(chance.given), false)
						assert.equal(yield* Q.toString(chance.evidenceMass), "0")
					}
				}
				assert.deepEqual(summary.sort(), ["1/2", "1/2", null].sort())
				return rows
			})
		)
	)
	assert.equal(retained.length, 3)
	assert.ok(retained.every(({ chance }) => Event.toBytes(chance.event).length > 0))
})

test("parameter probability queries retain p squared over p and its excluded zero, without a prior", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const name = id(220)
				const p = yield* P.parameter(name)
				const one = yield* P.constant(yield* Q.fraction(1n))
				const tail = yield* P.subtract(one, p)
				const region = yield* ParameterRegion.and(
					yield* ParameterRegion.whereSign(name, p, PolynomialSigns.nonNegative),
					yield* ParameterRegion.whereSign(name, tail, PolynomialSigns.nonNegative)
				)
				const base = yield* Event.withParameters(
					yield* Event.space(id(221), 2n),
					yield* ParameterDomain.new(region),
					[]
				)
				const a = yield* Event.coordinate(base, 0n)
				const b = yield* Event.coordinate(base, 1n)
				const pieces = []
				for (let world = 0; world < 4; world++)
					pieces.push({
						region: yield* Event.and(
							world & 1 ? a : yield* Event.complement(a),
							world & 2 ? b : yield* Event.complement(b)
						),
						value: {
							numerator: yield* P.multiply(world & 1 ? p : tail, world & 2 ? p : tail),
							denominator: one,
							defined: region
						}
					})
				const source = yield* FamilyFunction.designate(yield* FamilyFunction.new(base, pieces))
				const first = yield* Event.coordinate(source, 0n)
				const both = yield* Event.and(first, yield* Event.coordinate(source, 1n))
				const path = storeDir("query-probability-family")
				const original = yield* Db.create(path, Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Claim, [{ id: 1n, event: both, given: first }])
				assert.equal((yield* original.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				yield* original.close()
				const db = yield* Db.open(path, Theory)
				const snapshot = yield* db.snapshot()
				const forwarded = query(Theory).rule((r) => {
					const row = v(observed)
					return r.match(observed, row).find(row)
				})
				const rows = yield* (yield* snapshot.execute(forwarded, {})).collect()
				assert.deepEqual(rows, yield* (yield* snapshot.execute(observed, {})).collect())
				assert.equal(rows.length, 1)
				const chance = rows[0]?.chance
				assert.ok(chance?.law === "parameter")
				assert.equal(yield* ParameterFunction.at(chance.value, yield* Q.fraction(0n)), null)
				const third = yield* Q.fraction(1n, 3n)
				assert.equal(yield* text(yield* ParameterFunction.at(chance.value, third)), "1/3")
				assert.equal(yield* text(yield* ParameterFunction.at(chance.numerator, third)), "1/9")
				assert.equal(yield* text(yield* ParameterFunction.at(chance.evidenceMass, third)), "1/3")
				assert.deepEqual(Event.toBytes(chance.event), Event.toBytes(both))
				assert.deepEqual(Event.toBytes(chance.given), Event.toBytes(first))
			})
		)
	)
})
