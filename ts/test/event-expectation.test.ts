import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result, Stream } from "effect"
import {
	ChangeSet,
	closed,
	closedId,
	Db,
	describeQuery,
	Event,
	EventExpr,
	type ExpectationAnswer,
	event,
	expectation,
	expectationResult,
	FamilyFunction,
	FiniteFunction,
	f64,
	i64,
	key,
	NativeRuntime,
	ExactPolynomial as P,
	ParameterDomain,
	ParameterFunction,
	ParameterRegion,
	PolynomialSigns,
	payoffRatio,
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

const Payoff = relation("Payoff", { id: u64, group: u64, value: i64, second: u64, region: event, given: event })
const Theory = schema("ExpectationQueries", { Payoff }, [key(Payoff, ["id"])])
const observed = query(Theory).rule((r) => {
	const row = v(Payoff)
	return r.match(Payoff, row).find({
		group: row.group,
		mean: expectation(row.value, row.region, row.given),
		second: expectation(row.second, row.region, row.given),
		paths: r.count(),
		total: r.sum(row.value)
	})
})
function typePins(answer: QueryRow<typeof observed>) {
	const retained: ExpectationAnswer = answer.mean
	void retained
	// @ts-expect-error Observations are not Event operands.
	EventExpr.complement(answer.mean)
	// @ts-expect-error Result descriptors are not schema fields.
	relation("Invalid", { mean: expectationResult })
	const row = v(relation("Float", { value: f64, region: event }))
	// @ts-expect-error Binary64 does not silently become an exact payoff.
	expectation(row.value, row.region, row.region)
	// @ts-expect-error A divisor must be an ordinary exact integer column.
	payoffRatio(row.value, row.value)
}
void typePins
function payoffTypePins(partial: ParameterFunction) {
	const row = v(Payoff)
	// @ts-expect-error A partial parameter-only function is not a total source payoff.
	expectation(partial, row.region, row.given)
	// @ts-expect-error JavaScript numbers are not exact payoffs.
	expectation(0.5, row.region, row.given)
}
void payoffTypePins
async function run<A, E>(program: Effect.Effect<A, E, NativeRuntime>) {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		return await runtime.runPromise(program)
	} finally {
		await runtime.dispose()
	}
}
test("expectation authoring and description imports preserve aggregate identity without loading native code", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	const Category = closed("Category", ["Low", "High"])
	const Categorical = relation("Categorical", { value: closedId(Category), region: event })
	const Categories = schema("Categories", { Category, Categorical }, [])
	const categorical = v(Categorical)
	assert.throws(
		() =>
			query(Categories).rule((r) =>
				r
					.match(Categorical, categorical)
					.find({ mean: expectation(categorical.value, categorical.region, categorical.region) })
			),
		/reference/
	)
	const description = describeQuery(observed)
	const restored = queryFromDescription(Theory, description, {
		group: u64,
		mean: expectationResult,
		second: expectationResult,
		paths: u64,
		total: i64
	})
	assert.deepEqual(describeQuery(restored), description)
	assert.throws(
		() =>
			queryFromDescription(Theory, description, {
				group: u64,
				mean: event,
				second: expectationResult,
				paths: u64,
				total: i64
			}),
		/field domains/
	)
	assert.throws(() => v(observed), /not a mintable relation/)
	const row = v(Payoff)
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r.match(Payoff, { value: row.value }).find({ mean: expectation(row.value, row.region, row.given) })
			),
		/not bound/
	)
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r.match(Payoff, row).find({ mean: { ...expectation(row.value, row.region, row.given) } })
			),
		/not a find entry/
	)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("fraction payoffs remain owned pure query expressions across description import", () => {
	const authored = query(Theory).rule((r) => {
		const row = v(Payoff)
		return r.match(Payoff, row).find({ mean: expectation(payoffRatio(row.value, row.second), row.region, row.given) })
	})
	const description = describeQuery(authored)
	const restored = queryFromDescription(Theory, description, { mean: expectationResult })
	assert.deepEqual(describeQuery(restored), description)
	const row = v(Payoff)
	assert.throws(() => expectation({ ...payoffRatio(row.value, row.second) }, row.region, row.given), /integer variable/)
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r
					.match(Payoff, { value: row.value, region: row.region, given: row.given })
					.find({ mean: expectation(payoffRatio(row.value, row.second), row.region, row.given) })
			),
		/bound/
	)
})

test("numerical imports author and snapshot without loading native code", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	// Pure constructors certify envelopes only; worker preparation validates the mathematics.
	const imports = [
		Result.getOrThrow(Q.fromBytes(Buffer.from("BERA\x01"))),
		Result.getOrThrow(FiniteFunction.fromBytes(Buffer.from("BESC\x01\x00"))),
		Result.getOrThrow(FamilyFunction.fromBytes(Buffer.from("BESC\x02\x01")))
	]
	for (const value of imports) {
		const authored = query(Theory).rule((r) => {
			const row = v(Payoff)
			return r.match(Payoff, row).find({ mean: expectation(value, row.region, row.given) })
		})
		const description = describeQuery(authored)
		const restored = queryFromDescription(Theory, description, { mean: expectationResult })
		assert.deepEqual(describeQuery(restored), description)
		const first = description.ir.rules[0]?.finds[0]
		assert.ok(first?.kind === "expectation" && typeof first.value !== "number" && first.value.kind === "imported")
		first.value.bytes.fill(0)
		assert.deepEqual(describeQuery(restored), describeQuery(authored))
	}
	const row = v(Payoff)
	const partial = Result.getOrThrow(ParameterFunction.fromBytes(Buffer.from("BESC\x02\x00")))
	assert.throws(() => expectation(partial as never, row.region, row.given), /integer variable/)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("query expectations own complete payoff rosters and survive collection, paging, and closed owners", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const base = yield* Event.space(new Uint8Array(32).fill(222), 1n)
				const x = yield* Event.coordinate(base, 0n)
				// The other half is possible but has zero probability.
				const source = yield* FiniteFunction.designate(
					yield* FiniteFunction.new(base, [{ region: x, value: yield* Q.fraction(1n) }])
				)
				const when = yield* Event.coordinate(source, 0n)
				const zeroMass = yield* Event.complement(when)
				const db = yield* Db.create(storeDir("query-expectation-fixed"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Payoff, [
					{ id: 0n, group: 1n, value: -4n, second: 2n, region: when, given: source },
					{ id: 1n, group: 1n, value: -4n, second: 2n, region: when, given: source },
					{ id: 2n, group: 1n, value: 10n, second: 8n, region: zeroMass, given: source },
					{ id: 3n, group: 2n, value: 0n, second: 0n, region: zeroMass, given: zeroMass }
				])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const restored = queryFromDescription(Theory, describeQuery(observed), {
					group: u64,
					mean: expectationResult,
					second: expectationResult,
					paths: u64,
					total: i64
				})
				const snapshot = yield* db.snapshot()
				const result = yield* snapshot.execute(restored, {})
				yield* snapshot.close()
				yield* db.close()
				const rows = yield* result.collect()
				assert.equal(rows.length, 2)
				assert.equal(Array.from(yield* Stream.runCollect(result.pages())).flat().length, 2)
				for (const row of rows) {
					assert.ok(row.mean.law === "fixed" && row.second.law === "fixed" && row.mean.payoffKind === "scalar")
					if (row.group === 1n) {
						assert.ok(row.mean.value !== null && row.second.value !== null)
						assert.equal(yield* Q.toString(row.mean.value), "-4")
						assert.equal(yield* Q.toString(row.second.value), "2")
						assert.equal(row.paths, 3n)
						assert.equal(row.total, 2n)
						assert.equal(row.mean.payoffs.length, 2)
					} else {
						assert.equal(row.mean.value, null)
						assert.equal(yield* Event.isEmpty(row.mean.given), false)
						assert.equal(yield* Q.toString(row.mean.payoffs[0]?.value ?? (yield* Q.fraction(9n))), "0")
					}
				}
				return rows
			})
		)
	)
	assert.ok(
		retained.every(
			(r) => Event.toBytes(r.mean.given).length > 0 && r.mean.payoffKind === "scalar" && Object.isFrozen(r.mean.payoffs)
		)
	)
})

test("family expectations transport signed payoff functions and preserve impossible parameter endpoints", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const name = new Uint8Array(32).fill(223)
				const p = yield* P.parameter(name)
				const one = yield* P.constant(yield* Q.fraction(1n))
				const tail = yield* P.subtract(one, p)
				const domain = yield* ParameterRegion.and(
					yield* ParameterRegion.whereSign(name, p, PolynomialSigns.nonNegative),
					yield* ParameterRegion.whereSign(name, tail, PolynomialSigns.nonNegative)
				)
				const base = yield* Event.withParameters(
					yield* Event.space(new Uint8Array(32).fill(224), 2n),
					yield* ParameterDomain.new(domain),
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
							defined: domain
						}
					})
				const source = yield* FamilyFunction.designate(yield* FamilyFunction.new(base, pieces))
				const first = yield* Event.coordinate(source, 0n)
				const second = yield* Event.coordinate(source, 1n)
				const db = yield* Db.create(storeDir("query-expectation-family"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Payoff, [
					{ id: 0n, group: 1n, value: -2n, second: 2n, region: yield* Event.complement(second), given: first },
					{ id: 1n, group: 1n, value: 7n, second: 2n, region: second, given: first }
				])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const snapshot = yield* db.snapshot()
				const rows = yield* (yield* snapshot.execute(observed, {})).collect()
				const fractional = query(Theory).rule((r) => {
					const row = v(Payoff)
					return r
						.match(Payoff, row)
						.find({ mean: expectation(payoffRatio(row.value, row.second), row.region, row.given) })
				})
				const fractionRows = yield* (yield* snapshot.execute(fractional, {})).collect()
				const fraction = fractionRows[0]?.mean
				assert.ok(fraction?.law === "parameter")
				assert.equal(yield* ParameterFunction.at(fraction.value, yield* Q.fraction(0n)), null)
				const half = yield* ParameterFunction.at(fraction.value, yield* Q.fraction(1n, 3n))
				assert.ok(half !== null)
				assert.equal(yield* Q.toString(half), "1/2")
				const answer = rows[0]?.mean
				assert.ok(answer?.law === "parameter" && answer.payoffKind === "scalar")
				assert.equal(yield* ParameterFunction.at(answer.value, yield* Q.fraction(0n)), null)
				const third = yield* Q.fraction(1n, 3n)
				const value = yield* ParameterFunction.at(answer.value, third)
				assert.ok(value !== null)
				assert.equal(yield* Q.toString(value), "1")
				const numerator = yield* ParameterFunction.at(answer.numerator, third)
				assert.ok(numerator !== null)
				assert.equal(yield* Q.toString(numerator), "1/3")
				assert.equal(answer.payoffs.length, 2)
				assert.deepEqual(Event.toBytes(answer.given), Event.toBytes(first))
				// The returned finite function is the admitted payoff, not the mean.
				const payoff = yield* FiniteFunction.describe(answer.function)
				assert.equal(payoff.pieces.length, 2)
			})
		)
	)
})

test("fraction queries normalize equal presentations and reject hidden zero divisors", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const raw = yield* Event.space(new Uint8Array(32).fill(236), 1n)
				const source = yield* FiniteFunction.designate(yield* FiniteFunction.constant(raw, yield* Q.fraction(1n, 2n)))
				const head = yield* Event.coordinate(source, 0n)
				const db = yield* Db.create(storeDir("query-expectation-ratios"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Payoff, [
					{ id: 0n, group: 1n, value: 1n, second: 2n, region: head, given: source },
					{ id: 1n, group: 1n, value: 2n, second: 4n, region: head, given: source },
					{ id: 2n, group: 1n, value: -2n, second: 3n, region: yield* Event.complement(head), given: source },
					{ id: 3n, group: 1n, value: 0n, second: 7n, region: yield* Event.empty(source), given: source }
				])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const authored = query(Theory).rule((r) => {
					const row = v(Payoff)
					return r
						.match(Payoff, row)
						.find({ mean: expectation(payoffRatio(row.value, row.second), row.region, row.given), paths: r.count() })
				})
				const restored = queryFromDescription(Theory, describeQuery(authored), { mean: expectationResult, paths: u64 })
				const snapshot = yield* db.snapshot()
				const result = yield* snapshot.execute(restored, {})
				yield* snapshot.close()
				const rows = yield* result.collect()
				assert.equal(rows.length, 1)
				const row = rows[0]
				assert.ok(row && row.mean.law === "fixed" && row.mean.value !== null && row.mean.payoffKind === "scalar")
				assert.equal(yield* Q.toString(row.mean.value), "-1/12")
				assert.equal(row.paths, 4n)
				assert.equal(row.mean.payoffs.length, 3)
				const invalid = yield* ChangeSet.builder(Theory)
				yield* invalid.insert(Payoff, [
					{ id: 4n, group: 1n, value: 0n, second: 0n, region: yield* Event.empty(source), given: source }
				])
				yield* db.apply(yield* invalid.finish(), { expected: { kind: "any" } })
				const after = yield* db.snapshot()
				const error = yield* Effect.flip(Effect.flatMap(after.execute(restored, {}), (value) => value.collect()))
				assert.ok(error.reason._tag === "Engine")
				assert.match(error.reason.message, /division by zero/)
				yield* after.close()
				yield* db.close()
				assert.equal(yield* Q.toString(row.mean.value), "-1/12")
			})
		)
	)
})

test("function imports retain local covers, exact constants, descriptions and closed owners", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const raw = yield* Event.space(new Uint8Array(32).fill(242), 1n)
				const source = yield* FiniteFunction.designate(yield* FiniteFunction.constant(raw, yield* Q.fraction(1n, 2n)))
				const head = yield* Event.coordinate(source, 0n)
				const a = yield* FiniteFunction.new(source, [
					{ region: head, value: yield* Q.fraction(1n, 3n) },
					{ region: yield* Event.complement(head), value: yield* Q.fraction(-2n) }
				])
				const b = yield* FiniteFunction.constant(source, yield* Q.fraction(1n, 3n))
				const big = yield* Q.fraction(123456789012345678901234567890123456789n, 7n)
				const db = yield* Db.create(storeDir("query-function-imports"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Payoff, [
					{ id: 0n, group: 0n, value: 0n, second: 0n, region: source, given: source },
					{ id: 1n, group: 0n, value: 0n, second: 0n, region: head, given: source },
					{ id: 2n, group: 0n, value: 0n, second: 0n, region: yield* Event.empty(source), given: source }
				])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const authored = query(Theory)
					.rule((r) => {
						const row = v(Payoff)
						return r
							.match(Payoff, { id: 0n, region: row.region, given: row.given })
							.find({ mean: expectation(a, row.region, row.given), huge: expectation(big, row.given, row.given) })
					})
					.rule((r) => {
						const row = v(Payoff)
						return r
							.match(Payoff, { id: 1n, region: row.region, given: row.given })
							.find({ mean: expectation(b, row.region, row.given), huge: expectation(big, row.given, row.given) })
					})
					.rule((r) => {
						const row = v(Payoff)
						return r
							.match(Payoff, { id: 2n, region: row.region, given: row.given })
							.find({ mean: expectation(big, row.region, row.given), huge: expectation(big, row.given, row.given) })
					})
				const description = describeQuery(authored)
				const restored = queryFromDescription(Theory, description, { mean: expectationResult, huge: expectationResult })
				assert.deepEqual(describeQuery(restored), description)
				const malformed = structuredClone(description)
				const bad = malformed.ir.rules[0]?.finds[0]
				assert.ok(bad?.kind === "expectation")
				Object.assign(bad, { value: { kind: "imported", bytes: new Uint8Array([66, 69, 83, 67, 2, 0]) } })
				assert.throws(
					() => queryFromDescription(Theory, malformed, { mean: expectationResult, huge: expectationResult }),
					/total finite\/family/
				)

				for (const rule of description.ir.rules)
					for (const find of rule.finds) {
						if (find.kind === "expectation" && typeof find.value !== "number" && find.value.kind === "imported")
							find.value.bytes.fill(0)
					}
				assert.deepEqual(describeQuery(restored), describeQuery(authored))
				const snapshot = yield* db.snapshot()
				const result = yield* snapshot.execute(restored, {})
				yield* snapshot.close()
				yield* db.close()
				const rows = yield* result.collect()
				assert.equal(Array.from(yield* Stream.runCollect(result.pages())).flat().length, 1)
				const row = rows[0]
				assert.ok(row?.mean.law === "fixed" && row.mean.payoffKind === "finite" && row.mean.value !== null)
				assert.equal(yield* Q.toString(row.mean.value), "-5/6")
				assert.equal(row.mean.patches.length, 3)
				assert.ok(row.huge.law === "fixed" && row.huge.payoffKind === "scalar" && row.huge.value !== null)
				assert.equal(yield* Q.toString(row.huge.value), yield* Q.toString(big))
				return row.mean
			})
		)
	)
	await run(
		Effect.gen(function* () {
			const replay = yield* FiniteFunction.glue(retained.given, retained.patches)
			assert.equal(yield* FiniteFunction.equivalent(replay.function, retained.function), true)
		})
	)
})

test("family payoff queries glue at exact boundaries and promote finite patches", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const name = new Uint8Array(32).fill(243)
				const p = yield* P.parameter(name)
				const one = yield* P.constant(yield* Q.fraction(1n))
				const tail = yield* P.subtract(one, p)
				const domain = yield* ParameterDomain.new(
					yield* ParameterRegion.and(
						yield* ParameterRegion.whereSign(name, p, PolynomialSigns.nonNegative),
						yield* ParameterRegion.whereSign(name, tail, PolynomialSigns.nonNegative)
					)
				)
				const threshold = yield* P.subtract(p, yield* P.constant(yield* Q.fraction(1n, 2n)))
				const raw = yield* Event.withParameters(yield* Event.space(new Uint8Array(32).fill(244), 2n), domain, [
					{ coordinate: 0n, region: yield* ParameterRegion.whereSign(name, threshold, PolynomialSigns.nonPositive) },
					{ coordinate: 1n, region: yield* ParameterRegion.whereSign(name, threshold, PolynomialSigns.nonNegative) }
				])
				const source = yield* FamilyFunction.designate(
					yield* FamilyFunction.fromParameter(raw, yield* ParameterFunction.ratio(domain, one, one))
				)
				const a = yield* FamilyFunction.fromParameter(source, yield* ParameterFunction.ratio(domain, p, one))
				const b = yield* FamilyFunction.fromParameter(source, yield* ParameterFunction.ratio(domain, tail, one))
				const half = yield* FiniteFunction.constant(source, yield* Q.fraction(1n, 2n))
				const left = yield* Event.coordinate(source, 0n)
				const right = yield* Event.coordinate(source, 1n)
				const db = yield* Db.create(storeDir("query-family-imports"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Payoff, [
					{ id: 0n, group: 0n, value: 0n, second: 0n, region: left, given: source },
					{ id: 1n, group: 0n, value: 0n, second: 0n, region: right, given: source },
					{ id: 2n, group: 0n, value: 0n, second: 0n, region: yield* Event.and(left, right), given: source }
				])
				yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })
				const authored = query(Theory)
					.rule((r) => {
						const row = v(Payoff)
						return r
							.match(Payoff, { id: 0n, region: row.region, given: row.given })
							.find({ mean: expectation(a, row.region, row.given) })
					})
					.rule((r) => {
						const row = v(Payoff)
						return r
							.match(Payoff, { id: 1n, region: row.region, given: row.given })
							.find({ mean: expectation(b, row.region, row.given) })
					})
					.rule((r) => {
						const row = v(Payoff)
						return r
							.match(Payoff, { id: 2n, region: row.region, given: row.given })
							.find({ mean: expectation(half, row.region, row.given) })
					})
				const restored = queryFromDescription(Theory, describeQuery(authored), { mean: expectationResult })
				const snapshot = yield* db.snapshot()
				const rows = yield* (yield* snapshot.execute(restored, {})).collect()
				const mean = rows[0]?.mean
				assert.ok(mean?.payoffKind === "family" && mean.law === "parameter")
				assert.equal(mean.patches.length, 3)
				const replay = yield* FamilyFunction.glue(mean.given, mean.patches)
				assert.equal(yield* FamilyFunction.equivalent(replay.function, mean.function), true)
				for (const [n, expected] of [
					[0n, "0"],
					[1n, "1/4"],
					[2n, "1/2"],
					[3n, "1/4"],
					[4n, "0"]
				] as const) {
					const value: Q | null = yield* ParameterFunction.at(mean.value, yield* Q.fraction(n, 4n))
					assert.ok(value)
					assert.equal(yield* Q.toString(value), expected)
				}
				const bad = yield* Q.fraction(7n)
				const conflict = authored.rule((r) => {
					const row = v(Payoff)
					return r
						.match(Payoff, { id: 2n, region: row.region, given: row.given })
						.find({ mean: expectation(bad, row.region, row.given) })
				})
				const failure = yield* Effect.flip(Effect.flatMap(snapshot.execute(conflict, {}), (result) => result.collect()))
				assert.ok(failure.reason._tag === "Engine")
				assert.match(failure.reason.message, /disagree/)
			})
		)
	)
})

test("empty function import faults retain descriptors without invented variables", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const source = yield* Event.space(new Uint8Array(32).fill(1), 0n)
				const foreign = yield* Event.space(new Uint8Array(32).fill(250), 0n)
				const payoff = yield* FiniteFunction.constant(foreign, yield* Q.fraction(0n))
				const db = yield* Db.create(storeDir("query-imported-payoff-faults"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Payoff, [
					{ id: 0n, group: 0n, value: 0n, second: 0n, region: yield* Event.empty(source), given: source }
				])
				yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })
				const authored = query(Theory).rule((r) => {
					const row = v(Payoff)
					return r.match(Payoff, row).find({ mean: expectation(payoff, row.region, row.given) })
				})
				const snapshot = yield* db.snapshot()
				const failure = yield* Effect.flip(Effect.flatMap(snapshot.execute(authored, {}), (result) => result.collect()))
				assert.ok(failure.reason._tag === "Engine")
				const faults = failure.reason.eventFaults
				assert.equal(faults?.length, 1)
				const fault = faults?.[0]
				assert.ok(fault?.source === "payoff")
				assert.equal(Object.hasOwn(fault, "variable"), false)
				assert.deepEqual(fault.offendingValue, FiniteFunction.toBytes(payoff))
				assert.deepEqual(fault.expectedSpace, Event.toBytes(source))
			})
		)
	)
})
