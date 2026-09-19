import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result, Stream } from "effect"
import {
	ChangeSet,
	Compute,
	Db,
	describeQuery,
	Event,
	EventExpr,
	type ExpectationAnswer,
	event,
	expectation,
	expectationResult,
	FiniteFunction,
	i64,
	key,
	NativeRuntime,
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
const Payoff = relation("Payoff", { id: u64, group: u64, value: i64, region: event, given: event })
const Theory = schema("ObservationStages", { Claim, Payoff }, [key(Claim, ["id"]), key(Payoff, ["id"])])
const beliefs = query(Theory).rule((r) => {
	const c = v(Claim)
	return r.match(Claim, c).find({ id: c.id, chance: probability(c.event, c.given) })
})
const utilities = query(Theory).rule((r) => {
	const p = v(Payoff)
	return r.match(Payoff, p).find({ id: p.group, mean: expectation(p.value, p.region, p.given) })
})
const forwarded = query(Theory).rule((r) => {
	const c = v(beliefs)
	return r.match(beliefs, c).find(c)
})
const named = query(Theory)
	.interior("observed", (r) => {
		const c = v(Claim)
		return r.match(Claim, c).find({ id: c.id, chance: probability(c.event, c.given) })
	})
	.rule((r) => {
		const c = v(beliefs)
		return r.interior("observed", c).find(c)
	})
const recursive = query(Theory)
	.reach("carry", {
		base: [
			(r) => {
				const c = v(beliefs)
				return r.match(beliefs, c).where(r.eq(c.id, 1n)).find(c)
			}
		],
		rec: [
			(r) => {
				const c = v(beliefs)
				const next = v(Claim).id
				return r
					.interior("carry", c)
					.match(Claim, { id: next })
					.where(r.lt(c.id, next))
					.find({ id: next, chance: c.chance })
			}
		]
	})
	.rule((r) => {
		const c = v(beliefs)
		return r.interior("carry", c).find(c)
	})
const selected = query(Theory).rule((r) => {
	const c = v(forwarded)
	return r
		.match(forwarded, c)
		.where(r.eq(c.id, r.param("id")))
		.find({ chance: c.chance })
})
const allMeans = query(Theory).rule((r) => {
	const p = v(utilities)
	return r.match(utilities, p).find({ mean: p.mean })
})
const resultFields = { id: u64, chance: probabilityResult } as const

function typePins(answer: QueryRow<typeof forwarded>, mean: QueryRow<typeof allMeans>) {
	const chance: ProbabilityAnswer = answer.chance
	const expected: ExpectationAnswer = mean.mean
	void [chance, expected]
	const restored = queryFromDescription(Theory, describeQuery(forwarded), resultFields)
	const c = v(restored)
	const other = v(beliefs)
	const p = v(utilities)
	query(Theory).rule((r) => {
		const bound = r.match(restored, c).match(beliefs, other).match(utilities, p)
		bound.where(r.eq(c.chance, other.chance)).find({ chance: c.chance })
		// @ts-expect-error Different observation kinds cannot unify.
		bound.where(r.eq(c.chance, p.mean))
		// @ts-expect-error Parameters do not accept an execution-local observation identity.
		bound.where(r.eq(c.chance, r.param("p")))
		// @ts-expect-error Set parameters cannot coerce observations either.
		bound.where(r.eq(c.chance, r.inSet("ps")))
		// @ts-expect-error A numerical literal is not an observation.
		bound.where(r.eq(c.chance, 1n))
		// @ts-expect-error Observation ordering requires an explicit numerical operation.
		bound.where(r.lt(c.chance, other.chance))
		// @ts-expect-error Stored numeric folds cannot coerce observations.
		bound.find({ total: r.sum(c.chance) })
		// @ts-expect-error Pack does not take observation identities.
		bound.find({ all: r.pack(p.mean) })
		// @ts-expect-error Imported columns retain their precise query domain.
		bound.match(utilities, { id: c.id, mean: c.chance })
		// @ts-expect-error Observation variables cannot bind stored fields.
		bound.match(Claim, { id: c.chance })
		// @ts-expect-error Observations are not Event programs.
		EventExpr.complement(c.chance)
		// @ts-expect-error Observations are not scalar programs.
		Compute.add(c.chance, c.chance)
		return bound.find({ chance: c.chance, mean: p.mean })
	})
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

test("observation stages mint exact query domains and replay without loading native code", () => {
	assert.equal(nativeBindingIsLoaded(), false)
	for (const q of [beliefs, forwarded, named, recursive]) {
		const description = describeQuery(q)
		const restored = queryFromDescription(Theory, description, resultFields)
		assert.deepEqual(describeQuery(restored), description)
		assert.equal(v(restored).chance.field, probabilityResult)
		assert.throws(
			() => queryFromDescription(Theory, description, { id: u64, chance: expectationResult }),
			/field domains/
		)
	}
	const description = describeQuery(allMeans)
	const restored = queryFromDescription(Theory, description, { mean: expectationResult })
	assert.equal(v(restored).mean.field, expectationResult)
	assert.deepEqual(describeQuery(restored), description)
	assert.ok(
		description.ir.interiors.some((table) =>
			table.rules.some((rule) => rule.finds.some((f) => f.kind === "expectation"))
		)
	)
	assert.ok(description.ir.rules.every((rule) => rule.finds.every((f) => f.kind === "var")))
	assert.equal(nativeBindingIsLoaded(), false)
})

test("dynamic and description authors cannot coerce observation identities or mix producers with projections", () => {
	const c = v(beliefs)
	const p = v(utilities)
	for (const op of ["eq", "ne", "lt"] as const) {
		assert.throws(
			() =>
				query(Theory).rule((r) =>
					r
						.match(beliefs, c)
						.where({ cond: "cmp", op, lhs: c.chance, rhs: 1n, mask: undefined } as never)
						.find(c)
				),
			/observation/
		)
		assert.throws(
			() =>
				query(Theory).rule((r) =>
					r
						.match(beliefs, c)
						.where({ cond: "cmp", op, lhs: c.chance, rhs: r.param("value"), mask: undefined } as never)
						.find(c)
				),
			/observation/
		)
	}
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r
					.match(beliefs, c)
					.match(utilities, p)
					.where(r.eq(c.chance, p.mean) as never)
					.find(c)
			),
		/domain-unequal/
	)
	assert.throws(
		() =>
			query(Theory).rule((r) =>
				r
					.match(beliefs, c)
					.match(utilities, { id: c.id, mean: c.chance } as never)
					.find(c)
			),
		/class-equal/
	)
	assert.throws(
		() => query(Theory).rule((r) => r.match(beliefs, c).find({ total: r.sum(c.chance) } as never)),
		/not numeric/
	)
	assert.throws(() => beliefs.rule((r) => r.match(forwarded, c).find(c)), /same head/)
	const d = describeQuery(forwarded)
	const rule = d.ir.rules[0]
	assert.ok(rule)
	const invalid = {
		...d,
		ir: {
			...d.ir,
			rules: [
				{
					...rule,
					conditions: [
						{
							kind: "leaf",
							cmp: {
								op: { kind: "eq" },
								lhs: { kind: "var", var: 1 },
								rhs: { kind: "literal", value: { kind: "u64", value: 1n } }
							}
						}
					]
				}
			]
		}
	}
	assert.throws(() => queryFromDescription(Theory, invalid, resultFields), /observation/)
})

test("identity joins, antijoins, groups and mixed observations survive rebind, reopen and owner release", async () => {
	const retained = await run(
		Effect.scoped(
			Effect.gen(function* () {
				const bare = yield* Event.space(id(245), 2n)
				const a = yield* Event.coordinate(bare, 0n)
				const b = yield* Event.coordinate(bare, 1n)
				const source = yield* FiniteFunction.designate(
					yield* FiniteFunction.new(bare, [
						{ region: yield* Event.and(a, yield* Event.complement(b)), value: yield* Q.fraction(1n, 2n) },
						{ region: yield* Event.and(b, yield* Event.complement(a)), value: yield* Q.fraction(1n, 2n) }
					])
				)
				const first = yield* Event.coordinate(source, 0n)
				const second = yield* Event.coordinate(source, 1n)
				const impossible = yield* Event.and(yield* Event.complement(first), yield* Event.complement(second))
				const otherSource = yield* FiniteFunction.designate(
					yield* FiniteFunction.constant(yield* Event.space(id(246), 1n), yield* Q.fraction(1n, 2n))
				)
				const path = storeDir("sdk-observation-stages")
				const original = yield* Db.create(path, Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Claim, [
					{ id: 1n, event: first, given: source },
					{ id: 2n, event: second, given: source },
					{ id: 3n, event: first, given: source },
					{ id: 4n, event: source, given: impossible },
					{ id: 5n, event: yield* Event.coordinate(otherSource, 0n), given: otherSource },
					{ id: 6n, event: source, given: source },
					{ id: 7n, event: first, given: yield* Event.or(first, second) }
				])
				for (const group of [1n, 2n, 3n]) {
					const region = group === 2n ? second : first
					yield* changes.insert(Payoff, [
						{ id: group * 2n, group, value: 1n, region, given: source },
						{ id: group * 2n + 1n, group, value: 0n, region: yield* Event.complement(region), given: source }
					])
				}
				assert.equal((yield* original.apply(yield* changes.finish(), { expected: { kind: "any" } })).kind, "accepted")
				yield* original.close()
				const db = yield* Db.open(path, Theory)
				const snapshot = yield* db.snapshot()
				const joined = query(Theory).rule((r) => {
					const left = v(beliefs)
					const right = v(forwarded)
					return r
						.match(beliefs, left)
						.match(forwarded, { id: right.id, chance: left.chance })
						.find({ left: left.id, right: right.id, chance: left.chance })
				})
				const explicit = query(Theory).rule((r) => {
					const left = v(beliefs)
					const right = v(forwarded)
					return r
						.match(beliefs, left)
						.match(forwarded, right)
						.where(r.eq(left.chance, right.chance))
						.find({ left: left.id, right: right.id, chance: left.chance })
				})
				const pairs = (rows: readonly { readonly left: bigint; readonly right: bigint }[]) =>
					rows.map((row) => `${row.left}:${row.right}`).sort()
				const expectedPairs = ["1:1", "1:3", "2:2", "3:1", "3:3", "4:4", "5:5", "6:6", "7:7"]
				for (const q of [joined, explicit])
					assert.deepEqual(pairs(yield* (yield* snapshot.execute(q, {})).collect()), expectedPairs)
				const excluded = query(Theory)
					.interior("excluded", (r) => {
						const c = v(beliefs)
						return r.match(beliefs, c).where(r.eq(c.id, 1n)).find({ chance: c.chance })
					})
					.rule((r) => {
						const c = v(beliefs)
						return r
							.match(beliefs, c)
							.where(r.not("excluded", { chance: c.chance }))
							.find({ id: c.id })
					})
				assert.deepEqual((yield* (yield* snapshot.execute(excluded, {})).collect()).map((row) => row.id).sort(), [
					2n,
					4n,
					5n,
					6n,
					7n
				])
				const grouped = query(Theory).rule((r) => {
					const c = v(beliefs)
					return r.match(beliefs, c).find({ chance: c.chance, paths: r.count() })
				})
				assert.deepEqual((yield* (yield* snapshot.execute(grouped, {})).collect()).map((row) => row.paths).sort(), [
					1n,
					1n,
					1n,
					1n,
					1n,
					2n
				])
				const meanUnion = allMeans.rule((r) => {
					const p = v(utilities)
					return r.match(utilities, p).find({ mean: p.mean })
				})
				const means = yield* (yield* snapshot.execute(
					queryFromDescription(Theory, describeQuery(meanUnion), { mean: expectationResult }),
					{}
				)).collect()
				assert.equal(means.length, 2, "equal means retain different payoff functions; duplicate functions collapse")
				for (const { mean } of means) {
					assert.ok(mean.law === "fixed")
					assert.equal(yield* text(mean.value), "1/2")
				}
				const mixed = query(Theory).rule((r) => {
					const c = v(Claim)
					const p = v(utilities)
					return r
						.match(utilities, p)
						.match(Claim, { id: p.id, event: c.event, given: c.given })
						.find({ mean: p.mean, chance: probability(c.event, c.given) })
				})
				assert.equal((yield* (yield* snapshot.execute(mixed, {})).collect()).length, 2)
				const carried = yield* (yield* snapshot.execute(
					queryFromDescription(Theory, describeQuery(recursive), resultFields),
					{}
				)).collect()
				assert.equal(carried.length, 7)
				for (const row of carried) assert.deepEqual(Event.toBytes(row.chance.event), Event.toBytes(first))
				const namedRows = yield* (yield* snapshot.execute(named, {})).collect()
				assert.equal(namedRows.length, 7)
				const evidenceRow = namedRows.find((row) => row.id === 7n)
				assert.ok(evidenceRow?.chance.law === "fixed")
				assert.equal(yield* text(evidenceRow.chance.value), "1/2")
				assert.deepEqual(Event.toBytes(evidenceRow.chance.given), Event.toBytes(yield* Event.or(first, second)))
				const prepared = yield* snapshot.prepare(selected)
				const firstResult = yield* prepared.execute({ id: 1n })
				assert.equal((yield* (yield* prepared.execute({ id: 2n })).collect()).length, 1)
				yield* prepared.releaseMemory()
				assert.equal((yield* (yield* prepared.execute({ id: 99n })).collect()).length, 0)
				const complete = yield* snapshot.execute(
					queryFromDescription(Theory, describeQuery(forwarded), resultFields),
					{}
				)
				yield* prepared.close()
				yield* snapshot.close()
				yield* db.close()
				const rows = yield* complete.collect()
				assert.equal(rows.length, 7)
				assert.equal(Array.from(yield* Stream.runCollect(complete.pages())).flat().length, 7)
				const missing = rows.find((row) => row.id === 4n)?.chance
				assert.ok(missing?.law === "fixed")
				assert.equal(missing.value, null)
				assert.equal(yield* Event.isEmpty(missing.given), false)
				assert.equal(yield* text(missing.evidenceMass), "0")
				const firstRow = (yield* firstResult.collect())[0]
				assert.ok(firstRow)
				assert.deepEqual(Event.toBytes(firstRow.chance.event), Event.toBytes(first))
				return { rows, means }
			})
		)
	)
	assert.ok(retained.rows.every((row) => Event.toBytes(row.chance.event).length > 0))
	assert.ok(
		retained.means.every((row) => row.mean.law === "fixed" && FiniteFunction.toBytes(row.mean.function).length > 0)
	)
})

test("consumer filters cannot hide a producer's missing zero-mass payoff coverage", async () => {
	await run(
		Effect.scoped(
			Effect.gen(function* () {
				const bare = yield* Event.space(id(247), 1n)
				const source = yield* FiniteFunction.designate(
					yield* FiniteFunction.new(bare, [{ region: yield* Event.coordinate(bare, 0n), value: yield* Q.fraction(1n) }])
				)
				const db = yield* Db.create(storeDir("sdk-observation-errors"), Theory)
				const changes = yield* ChangeSet.builder(Theory)
				yield* changes.insert(Payoff, [
					{ id: 1n, group: 1n, value: 2n, region: yield* Event.coordinate(source, 0n), given: source }
				])
				yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })
				const hidden = query(Theory).rule((r) => {
					const p = v(utilities)
					return r.match(utilities, p).where(r.eq(p.id, 99n)).find({ mean: p.mean })
				})
				const snapshot = yield* db.snapshot()
				const prepared = yield* snapshot.prepare(hidden)
				for (let attempt = 0; attempt < 2; attempt++) {
					const result = yield* Effect.result(prepared.execute({}))
					assert.ok(Result.isFailure(result))
					assert.equal(result.failure.reason._tag, "Engine")
					yield* prepared.releaseMemory()
				}
			})
		)
	)
})
