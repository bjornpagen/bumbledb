import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
	bool,
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
	ObservationPredicate,
	ExactPolynomial as P,
	type PredicateAnswer,
	predicateResult,
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
	PredicateTest as Test,
	u64,
	v
} from "#index.ts"
import { nativeBindingIsLoaded } from "#native.ts"
import { parsePredicateIr } from "#query/predicate-ir.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Trial = relation("Trial", { id: u64, value: i64, when: event, given: event })
const Theory = schema("QueryPredicates", { Trial }, [key(Trial, ["id"])])
const observed = query(Theory).rule((r) => {
	const c = v(Trial)
	return r.match(Trial, c).find({ id: c.id, value: c.value, p: probability(c.when, c.given) })
})
const truth = query(Theory).rule((r) => {
	const c = v(observed)
	return r.match(observed, c).find({ id: c.id, verdict: T.sign(N.value(c.p), S.positive) })
})
const grouped = query(Theory).rule((r) => {
	const c = v(truth)
	return r.match(truth, c).find({ verdict: c.verdict, count: r.count() })
})
const direct = query(Theory).rule((r) => {
	const c = v(observed)
	return r.match(observed, c).find({ verdict: T.sign(N.value(c.p), S.positive), count: r.count() })
})
function typePins(row: QueryRow<typeof truth>) {
	const answer: PredicateAnswer = row.verdict
	const c = v(truth)
	T.not(c.verdict)
	// @ts-expect-error Predicates are not numerical observations.
	N.value(c.verdict)
	// @ts-expect-error Predicates are not Numbers.
	N.add(c.verdict, c.verdict)
	// @ts-expect-error Probability needs an explicit comparison.
	T.not(v(observed).p)
	// @ts-expect-error Event and predicate are different query domains.
	T.not(v(Trial).when)
	query(Theory).rule((r) => {
		const bound = r.match(truth, c)
		// @ts-expect-error Full predicate identities are not scalar parameters.
		bound.where(r.eq(c.verdict, r.param("value")))
		// @ts-expect-error Predicates have no implicit ordering.
		bound.where(r.lt(c.verdict, c.verdict))
		// @ts-expect-error A stored fold cannot collapse a predicate.
		bound.find({ sum: r.sum(c.verdict) })
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

test("predicate authoring is pure, authentic and bounded across numerical subtrees", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	for (const q of [direct, grouped]) {
		const description = describeQuery(q)
		const replayed = queryFromDescription(Theory, description, { verdict: predicateResult, count: u64 })
		assert.deepEqual(describeQuery(replayed), description)
		assert.equal(v(replayed).verdict.field, predicateResult)
		assert.throws(() => queryFromDescription(Theory, description, { verdict: bool, count: u64 }), /field domains/)
	}
	const integer = N.integer(v(Trial).value)
	assert.throws(() => T.sign(integer, 8), /mask/)
	assert.throws(() => T.apply(16, v(truth).verdict, v(truth).verdict), /truth table/)
	assert.throws(() => T.not({ kind: "predicate" } as never), /completed predicate/)
	assert.throws(() => T.not(v(observed).p as never), /completed predicate/)
	assert.throws(
		() => query(Theory).rule((r) => r.match(Trial, v(Trial)).find({ verdict: T.not(v(truth).verdict) })),
		/bound/
	)
	let deep = integer
	for (let i = 1; i < 128; i++) deep = N.abs(deep)
	assert.throws(() => T.sign(deep, 7), /combined shape/)
	let wide = integer
	for (let i = 0; i < 11; i++) wide = N.add(wide, wide)
	const nearLimit = T.sign(wide, 7)
	assert.throws(() => T.not(nearLimit), /combined shape/)
	for (const invalid of [
		{ kind: "sign", number: { kind: "integer", var: 0 }, signs: 8 },
		{ kind: "apply", op: 16, left: { kind: "var", var: 0 }, right: { kind: "var", var: 0 } },
		{ kind: "var", var: 0, ignored: true },
		{ kind: "imported", bytes: Buffer.from("BENO\x01") },
		{ kind: "sign", number: { kind: "integer", var: 0, ignored: true }, signs: 1 }
	])
		assert.throws(() => parsePredicateIr("test", invalid))
	let numerical: unknown = { kind: "integer", var: 0 }
	for (let i = 1; i < 128; i++) numerical = { kind: "abs", value: numerical }
	assert.throws(() => parsePredicateIr("test", { kind: "sign", number: numerical, signs: 7 }), /shape/)
	assert.equal(nativeBindingIsLoaded(), false)
})

test("predicate grouping and identity joins preserve sources and owned replay", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const raw = yield* Event.space(new Uint8Array(32).fill(235), 2n)
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
				const db = yield* Db.create(storeDir("sdk-query-predicates"), Theory)
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
					queryFromDescription(Theory, describeQuery(grouped), { verdict: predicateResult, count: u64 })
				]) {
					const rows = yield* (yield* snapshot.execute(q, {})).collect()
					assert.equal(rows.length, 3)
					assert.deepEqual(
						rows
							.map((row) => {
								assert.ok(row.verdict.law === "fixed")
								return `${row.verdict.value}:${row.count}`
							})
							.sort(),
						["null:1", "true:1", "true:2"]
					)
					const identities = rows
						.map(
							(row) =>
								`${Buffer.from(ObservationPredicate.toBytes(row.verdict.predicate)).toString("hex")}:${row.count}`
						)
						.sort()
					if (previous !== undefined) assert.deepEqual(identities, previous)
					previous = identities
				}
				const joined = query(Theory).rule((r) => {
					const a = v(truth)
					const b = v(truth)
					return r.match(truth, a).match(truth, b).where(r.eq(a.verdict, b.verdict)).find({ left: a.id, right: b.id })
				})
				const pairs = yield* (yield* snapshot.execute(joined, {})).collect()
				assert.equal(pairs.length, 6)
				assert.ok(pairs.every((p) => p.left === p.right || (p.left < 2n && p.right < 2n)))
				const rows = yield* (yield* snapshot.execute(truth, {})).collect()
				const firstRow = rows.find((row) => row.id === 0n)
				const secondRow = rows.find((row) => row.id === 2n)
				assert.ok(firstRow && secondRow)
				assert.notDeepEqual(
					ObservationPredicate.toBytes(firstRow.verdict.predicate),
					ObservationPredicate.toBytes(secondRow.verdict.predicate)
				)
				const bytes = ObservationPredicate.toBytes(firstRow.verdict.predicate)
				const imported = Result.getOrThrow(ObservationPredicate.fromBytes(bytes))
				bytes.fill(0)
				const replay = query(Theory).rule((r) =>
					r.match(Trial, { id: v(Trial).id }).find({ verdict: T.imported(imported) })
				)
				const result = yield* snapshot.execute(replay, {})
				yield* snapshot.close()
				yield* db.close()
				const replayed = yield* result.collect()
				assert.equal(replayed.length, 1)
				const replayedRow = replayed[0]
				assert.ok(replayedRow)
				assert.deepEqual(
					ObservationPredicate.toBytes(replayedRow.verdict.predicate),
					ObservationPredicate.toBytes(firstRow.verdict.predicate)
				)
				return rows
			})
		)
	)
	assert.equal(retained.length, 4)
	assert.ok(retained.every((row) => ObservationPredicate.is(row.verdict.predicate)))
})

test("predicate algebra keeps parameter holes, explicit quantifiers and producer errors", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const one = yield* Q.fraction(1n)
				const zero = yield* Q.fraction(0n)
				const half = yield* Q.fraction(1n, 2n)
				const name = new Uint8Array(32).fill(236)
				const p = yield* P.parameter(name)
				const polyOne = yield* P.constant(one)
				const tail = yield* P.subtract(polyOne, p)
				const region = yield* R.and(
					yield* R.whereSign(name, p, S.nonNegative),
					yield* R.whereSign(name, tail, S.nonNegative)
				)
				const domain = yield* D.new(region)
				const raw = yield* Event.withParameters(yield* Event.space(new Uint8Array(32).fill(237), 1n), domain, [])
				const bit = yield* Event.coordinate(raw, 0n)
				const source = yield* FamilyFunction.designate(
					yield* FamilyFunction.new(raw, [
						{ region: bit, value: { numerator: p, denominator: polyOne, defined: region } },
						{ region: yield* Event.complement(bit), value: { numerator: tail, denominator: polyOne, defined: region } }
					])
				)
				const evidence = yield* Event.coordinate(source, 0n)
				const db = yield* Db.create(storeDir("sdk-predicate-family"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Trial, [{ id: 0n, value: 7n, when: evidence, given: evidence }])
				assert.equal((yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				const snapshot = yield* db.snapshot()
				const x = N.literal(one)
				const y = N.literal(zero)
				const operators = query(Theory).rule((r) =>
					r.match(Trial, { id: v(Trial).id }).find({
						gt: T.greater(x, y),
						ge: T.greaterEqual(x, x),
						lt: T.less(y, x),
						le: T.lessEqual(x, x),
						eq: T.equal(x, x),
						ne: T.notEqual(x, y),
						xor: T.xor(T.equal(x, x), T.equal(x, x)),
						and: T.and(T.equal(x, x), T.not(T.equal(x, y))),
						or: T.or(T.equal(x, y), T.equal(x, x)),
						domain: T.onDomain(T.equal(x, x), domain)
					})
				)
				const fields = {
					gt: predicateResult,
					ge: predicateResult,
					lt: predicateResult,
					le: predicateResult,
					eq: predicateResult,
					ne: predicateResult,
					xor: predicateResult,
					and: predicateResult,
					or: predicateResult,
					domain: predicateResult
				}
				const rows = yield* (yield* snapshot.execute(
					queryFromDescription(Theory, describeQuery(operators), fields),
					{}
				)).collect()
				const row = rows[0]
				assert.ok(row)
				for (const key of ["gt", "ge", "lt", "le", "eq", "ne", "xor", "and", "or"] as const) {
					const verdict: PredicateAnswer = row[key]
					assert.ok(verdict.law === "fixed")
					assert.equal(verdict.value, key !== "xor")
				}
				assert.ok(row.domain.law === "parameter")
				assert.equal(yield* R.containsRational(row.domain.holds, zero), true)
				assert.deepEqual(D.toBytes(row.domain.domain), D.toBytes(domain))
				const partial = query(Theory).rule((r) => {
					const c = v(truth)
					return r.match(truth, c).find({
						original: c.verdict,
						opposite: T.not(c.verdict),
						constant: T.apply(15, c.verdict, c.verdict),
						anywhere: Test.possibly(c.verdict),
						everywhere: Test.always(c.verdict),
						total: Test.isTotal(c.verdict)
					})
				})
				const partialFields = {
					original: predicateResult,
					opposite: predicateResult,
					constant: predicateResult,
					anywhere: bool,
					everywhere: bool,
					total: bool
				}
				const partialRows = yield* (yield* snapshot.execute(
					queryFromDescription(Theory, describeQuery(partial), partialFields),
					{}
				)).collect()
				const value = partialRows[0]
				assert.ok(value)
				assert.equal(value.anywhere, true)
				assert.equal(value.everywhere, false)
				assert.equal(value.total, false)
				for (const key of ["original", "opposite", "constant"] as const) {
					const verdict: PredicateAnswer = value[key]
					assert.ok(verdict.law === "parameter")
					assert.equal(yield* R.containsRational(verdict.undefined, zero), true)
					assert.equal(yield* R.containsRational(verdict.holds, zero), false)
					assert.equal(yield* R.containsRational(verdict.fails, zero), false)
					assert.equal(yield* R.containsRational(verdict.holds, half), key !== "opposite")
					assert.equal(yield* R.containsRational(verdict.fails, half), key === "opposite")
				}
				const opposites = query(Theory).rule((r) => {
					const c = v(observed)
					const mass = N.evidenceMass(c.p)
					return r.match(observed, c).find({
						low: T.less(mass, N.literal(half)),
						high: T.greater(mass, N.literal(half))
					})
				})
				const witnesses = query(Theory).rule((r) => {
					const c = v(opposites)
					return r.match(opposites, c).find({
						low: Test.possibly(c.low),
						high: Test.possibly(c.high),
						together: Test.possibly(T.and(c.low, c.high))
					})
				})
				assert.deepEqual(yield* (yield* snapshot.execute(witnesses, {})).collect(), [
					{ low: true, high: true, together: false }
				])
				const foreign = yield* D.new(yield* R.full(new Uint8Array(32).fill(238)))
				const bad = query(Theory).rule((r) =>
					r.match(Trial, { id: v(Trial).id }).find({
						verdict: T.apply(15, T.onDomain(T.equal(x, x), domain), T.onDomain(T.equal(x, x), foreign))
					})
				)
				const filtered = query(Theory).rule((r) => {
					const c = v(bad)
					const t = v(Trial)
					return r.match(bad, c).match(Trial, t).where(r.eq(t.id, 999n)).find(c)
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(filtered, {}))))
				const corrupt = Result.getOrThrow(ObservationPredicate.fromBytes(Buffer.from("BENP\x01")))
				const invalid = query(Theory).rule((r) => {
					const c = v(Trial)
					return r
						.match(Trial, c)
						.where(r.eq(c.id, 999n))
						.find({ verdict: T.imported(corrupt) })
				})
				assert.ok(Result.isFailure(yield* Effect.result(snapshot.execute(invalid, {}))))
				assert.equal((yield* (yield* snapshot.execute(truth, {})).collect()).length, 1)
			})
		)
	)
})
