import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
	ChangeSet,
	ParameterDomain as D,
	Db,
	describeQuery,
	Event,
	event,
	FamilyFunction,
	FiniteFunction,
	i64,
	key,
	NumberExpr as N,
	NativeRuntime,
	type NumberAnswer,
	numberResult,
	ObservationNumber,
	ExactPolynomial as P,
	ParameterFunction as PF,
	probability,
	ExactRational as Q,
	type QueryRow,
	query,
	queryFromDescription,
	ParameterRegion as R,
	relation,
	PolynomialSigns as S,
	schema,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { parseQueryIr } from "#query/parse-ir.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Trial = relation("Trial", { id: u64, value: i64, when: event, given: event })
const Theory = schema("QueryNumbers", { Trial }, [key(Trial, ["id"])])
const observed = query(Theory).rule((r) => {
	const c = v(Trial)
	return r.match(Trial, c).find({ id: c.id, value: c.value, p: probability(c.when, c.given) })
})
const scored = query(Theory).rule((r) => {
	const c = v(observed)
	return r.match(observed, c).find({ id: c.id, n: N.multiply(N.value(c.p), N.integer(c.value)) })
})
const grouped = query(Theory).rule((r) => {
	const c = v(scored)
	return r.match(scored, c).find({ n: c.n, count: r.count() })
})
const direct = query(Theory).rule((r) => {
	const c = v(observed)
	return r.match(observed, c).find({ n: N.multiply(N.value(c.p), N.integer(c.value)), count: r.count() })
})
function typePins(row: QueryRow<typeof scored>) {
	const answer: NumberAnswer = row.n
	const c = v(scored)
	const p = v(observed)
	const t = v(Trial)
	N.add(c.n, N.value(p.p))
	// @ts-expect-error Numerical components require completed observations.
	N.value(c.n)
	// @ts-expect-error Scalar columns need explicit Integer.
	N.multiply(c.n, t.value)
	// @ts-expect-error Integer refuses Event values.
	N.integer(t.when)
	query(Theory).rule((r) => {
		const bound = r.match(scored, c)
		// @ts-expect-error A numerical identity is not a scalar parameter.
		bound.where(r.eq(c.n, r.param("value")))
		// @ts-expect-error Numerical comparison is an explicit algebra operation.
		bound.where(r.lt(c.n, c.n))
		// @ts-expect-error Stored folds cannot collapse numerical derivations.
		bound.find({ total: r.sum(c.n) })
		return bound.find(c)
	})
	return answer
}
void typePins
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}

test("numerical authoring, imports and descriptions are pure and keep query-only types", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	for (const q of [direct, grouped]) {
		const description = describeQuery(q)
		const replayed = queryFromDescription(Theory, description, { n: numberResult, count: u64 })
		assert.deepEqual(describeQuery(replayed), description)
		assert.equal(v(replayed).n.field, numberResult)
		assert.throws(() => queryFromDescription(Theory, description, { n: i64, count: u64 }), /field domains/)
	}
	assert.throws(() => N.pow(v(scored).n, -1), /natural/)
	assert.throws(() => N.value(v(scored).n as never), /probability/)
	assert.throws(() => N.integer(v(Trial).when as never), /integer|u64/)
	assert.throws(() => query(Theory).rule((r) => r.match(Trial, v(Trial)).find({ n: N.value(v(observed).p) })), /bound/)
	assert.throws(() => N.add({ kind: "number" } as never, v(scored).n), /expected/)
	let deep = N.integer(v(Trial).value)
	for (let i = 1; i < 128; i++) deep = N.abs(deep)
	assert.throws(() => N.abs(deep), /shape/)
	const description = describeQuery(scored)
	const rule = description.ir.rules[0]
	assert.ok(rule)
	for (const expr of [
		{ kind: "pow", value: { kind: "integer", var: 0 }, exponent: -1 },
		{ kind: "component", observation: 0, component: "maybe" },
		{ kind: "literal", bytes: new Uint8Array([66, 69, 78, 79, 1]) },
		{ kind: "integer", var: 0, extra: true }
	])
		assert.throws(() =>
			parseQueryIr({
				...description.ir,
				rules: [
					{
						...rule,
						finds: [
							{ kind: "var", var: 0 },
							{ kind: "number", expr }
						]
					}
				]
			})
		)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("numerical queries group before folds and preserve derivations after owners close", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const raw = yield* Event.space(new Uint8Array(32).fill(231), 2n)
				const a = yield* Event.coordinate(raw, 0n)
				const b = yield* Event.coordinate(raw, 1n)
				const source = yield* FiniteFunction.designate(
					yield* FiniteFunction.new(raw, [
						{ region: yield* Event.and(a, yield* Event.complement(b)), value: yield* Q.fraction(1n, 2n) },
						{ region: yield* Event.and(b, yield* Event.complement(a)), value: yield* Q.fraction(1n, 2n) }
					])
				)
				const first = yield* Event.coordinate(source, 0n)
				const second = yield* Event.coordinate(source, 1n)
				const impossible = yield* Event.and(yield* Event.complement(first), yield* Event.complement(second))
				const db = yield* Db.create(storeDir("sdk-query-numbers"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Trial, [
					{ id: 0n, value: 7n, when: first, given: source },
					{ id: 1n, value: 7n, when: first, given: source },
					{ id: 2n, value: 7n, when: second, given: source },
					{ id: 3n, value: 0n, when: source, given: impossible }
				])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const snapshot = yield* db.snapshot()
				let previous: readonly string[] | undefined
				for (const q of [
					direct,
					grouped,
					queryFromDescription(Theory, describeQuery(grouped), { n: numberResult, count: u64 })
				]) {
					const rows = yield* (yield* snapshot.execute(q, {})).collect()
					assert.equal(rows.length, 3)
					const summaries: string[] = []
					for (const row of rows) {
						assert.equal(row.n.law, "fixed")
						assert.ok(row.n.law === "fixed")
						summaries.push(`${row.n.value === null ? "undefined" : yield* Q.toString(row.n.value)}:${row.count}`)
					}
					assert.deepEqual(summaries.sort(), ["7/2:1", "7/2:2", "undefined:1"])
					const identities = rows
						.map((row) => `${Buffer.from(ObservationNumber.toBytes(row.n.number)).toString("hex")}:${row.count}`)
						.sort()
					if (previous !== undefined) assert.deepEqual(identities, previous)
					previous = identities
				}
				const rows = yield* (yield* snapshot.execute(scored, {})).collect()
				const firstRow = rows.find((row) => row.id === 0n)
				const secondRow = rows.find((row) => row.id === 2n)
				assert.ok(firstRow && secondRow)
				assert.notDeepEqual(ObservationNumber.toBytes(firstRow.n.number), ObservationNumber.toBytes(secondRow.n.number))
				const bytes = ObservationNumber.toBytes(firstRow.n.number)
				const imported = Result.getOrThrow(ObservationNumber.fromBytes(bytes))
				bytes.fill(0)
				const replay = query(Theory).rule((r) => r.match(Trial, { id: v(Trial).id }).find({ n: N.imported(imported) }))
				const result = yield* snapshot.execute(replay, {})
				yield* snapshot.close()
				yield* db.close()
				const replayed = yield* result.collect()
				assert.equal(replayed.length, 1)
				const replayedRow = replayed[0]
				assert.ok(replayedRow)
				assert.deepEqual(ObservationNumber.toBytes(replayedRow.n.number), ObservationNumber.toBytes(firstRow.n.number))
				return rows
			})
		)
	)
	assert.equal(retained.length, 4)
	assert.ok(retained.every((row) => ObservationNumber.is(row.n.number)))
})

test("every numerical operator, domain import and partial family survives SDK replay", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const one = yield* Q.fraction(1n)
				const two = yield* Q.fraction(2n)
				const zero = yield* Q.fraction(0n)
				const name = new Uint8Array(32).fill(232)
				const p = yield* P.parameter(name)
				const polyOne = yield* P.constant(one)
				const tail = yield* P.subtract(polyOne, p)
				const region = yield* R.and(
					yield* R.whereSign(name, p, S.nonNegative),
					yield* R.whereSign(name, tail, S.nonNegative)
				)
				const domain = yield* D.new(region)
				const raw = yield* Event.withParameters(yield* Event.space(new Uint8Array(32).fill(233), 1n), domain, [])
				const bit = yield* Event.coordinate(raw, 0n)
				const source = yield* FamilyFunction.designate(
					yield* FamilyFunction.new(raw, [
						{ region: bit, value: { numerator: p, denominator: polyOne, defined: region } },
						{ region: yield* Event.complement(bit), value: { numerator: tail, denominator: polyOne, defined: region } }
					])
				)
				const evidence = yield* Event.coordinate(source, 0n)
				const db = yield* Db.create(storeDir("sdk-query-number-family"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Trial, [{ id: 0n, value: 7n, when: evidence, given: evidence }])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const snapshot = yield* db.snapshot()
				const a = N.literal(one)
				const b = N.literal(two)
				const constants = query(Theory).rule((r) =>
					r.match(Trial, { id: v(Trial).id }).find({
						add: N.add(a, b),
						subtract: N.subtract(a, b),
						multiply: N.multiply(a, b),
						divide: N.divide(a, b),
						min: N.min(a, b),
						max: N.max(a, b),
						negate: N.negate(a),
						abs: N.abs(N.negate(b)),
						pow: N.pow(b, 3),
						undefined: N.divide(a, N.literal(zero)),
						domain: N.onDomain(a, domain)
					})
				)
				const description = describeQuery(constants)
				const fields = {
					add: numberResult,
					subtract: numberResult,
					multiply: numberResult,
					divide: numberResult,
					min: numberResult,
					max: numberResult,
					negate: numberResult,
					abs: numberResult,
					pow: numberResult,
					undefined: numberResult,
					domain: numberResult
				}
				for (const q of [constants, queryFromDescription(Theory, description, fields)]) {
					const rows = yield* (yield* snapshot.execute(q, {})).collect()
					const row = rows[0]
					assert.ok(row)
					const expected = {
						add: "3",
						subtract: "-1",
						multiply: "2",
						divide: "1/2",
						min: "1",
						max: "2",
						negate: "-1",
						abs: "2",
						pow: "8",
						undefined: null
					}
					for (const [key, value] of Object.entries(expected)) {
						const number: NumberAnswer = row[key as keyof typeof expected]
						assert.ok(number.law === "fixed")
						assert.equal(number.value === null ? null : yield* Q.toString(number.value), value)
					}
					assert.ok(row.domain.law === "parameter")
					const atZero: Q | null = yield* PF.at(row.domain.value, zero)
					assert.ok(atZero)
					assert.equal(yield* Q.toString(atZero), "1")
					assert.deepEqual(D.toBytes(row.domain.domain), D.toBytes(domain))
				}
				const partial = query(Theory).rule((r) => {
					const c = v(observed)
					const value = N.value(c.p)
					return r.match(observed, c).find({
						zero: N.multiply(value, N.literal(zero)),
						difference: N.subtract(value, value),
						power: N.pow(value, 0),
						numerator: N.numerator(c.p),
						mass: N.evidenceMass(c.p)
					})
				})
				const partialFields = {
					zero: numberResult,
					difference: numberResult,
					power: numberResult,
					numerator: numberResult,
					mass: numberResult
				}
				const rows = yield* (yield* snapshot.execute(
					queryFromDescription(Theory, describeQuery(partial), partialFields),
					{}
				)).collect()
				const row = rows[0]
				assert.ok(row)
				for (const key of ["zero", "difference", "power"] as const) {
					const value: NumberAnswer = row[key]
					assert.ok(value.law === "parameter")
					assert.equal(yield* PF.at(value.value, zero), null)
					const atHalf: Q | null = yield* PF.at(value.value, yield* Q.fraction(1n, 2n))
					assert.ok(atHalf)
					assert.equal(yield* Q.toString(atHalf), key === "power" ? "1" : "0")
					assert.equal(yield* R.containsRational(value.defined, zero), false)
				}
				for (const key of ["numerator", "mass"] as const) {
					const value: NumberAnswer = row[key]
					assert.ok(value.law === "parameter")
					const atZero: Q | null = yield* PF.at(value.value, zero)
					assert.ok(atZero)
					assert.equal(yield* Q.toString(atZero), "0")
				}
				const foreign = yield* D.new(yield* R.full(new Uint8Array(32).fill(234)))
				const bad = query(Theory).rule((r) =>
					r
						.match(Trial, { id: v(Trial).id })
						.find({ n: N.multiply(N.onDomain(N.literal(zero), domain), N.onDomain(a, foreign)) })
				)
				const filtered = query(Theory).rule((r) => {
					const n = v(bad)
					const t = v(Trial)
					return r.match(bad, n).match(Trial, t).where(r.eq(t.id, 999n)).find(n)
				})
				const outcome = yield* Effect.result(snapshot.execute(filtered, {}))
				assert.ok(Result.isFailure(outcome))
				// Copied envelopes do not bypass worker admission, even on an empty body.
				const corrupt = Result.getOrThrow(ObservationNumber.fromBytes(Buffer.from("BENO\x01")))
				const invalid = query(Theory).rule((r) => {
					const t = v(Trial)
					return r
						.match(Trial, t)
						.where(r.eq(t.id, 999n))
						.find({ n: N.imported(corrupt) })
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(invalid, {}))))
			})
		)
	)
})
