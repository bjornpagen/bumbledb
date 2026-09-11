import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import {
	bool,
	ChangeSet,
	Compute,
	closed,
	closedId,
	contained,
	Db,
	i64,
	interval,
	key,
	lowerQuery,
	NativeRuntime,
	on,
	type QueryRuleScope,
	query,
	relation,
	schema,
	u64,
	v
} from "#index.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Entry = relation("Entry", { id: u64, amount: i64, previous: i64, original: bool })
const Window = relation("Window", { span: interval(u64, 10n), excluded: interval(u64) })
const Theory = schema("Heads", { Entry, Window }, [])

function ordered<A extends { readonly id: bigint }>(rows: readonly A[]): A[] {
	return rows.toSorted((a, b) => compare(a.id, b.id))
}

function compare(a: bigint, b: bigint): number {
	if (a === b) return 0
	return a < b ? -1 : 1
}

const corrections = query(Theory).rule((r) => {
	const { id, amount, previous } = v(Entry)
	return r
		.match(Entry, { id, amount, previous, original: false })
		.find({ id, amount: Compute.subtract(amount, previous) })
})
const original = query(Theory).rule((r) => {
	const { id, amount } = v(Entry)
	return r.match(Entry, { id, amount, original: true }).find({ id, amount })
})
const direct = original.rule((r) => {
	const { id, amount, previous } = v(Entry)
	return r
		.match(Entry, { id, amount, previous, original: false })
		.find({ id, amount: Compute.subtract(amount, previous) })
})
const imported = original.rule((r) => {
	const row = v(corrections)
	return r.match(corrections, row).find(row)
})
const reversed = corrections.rule((r) => {
	const { id, amount } = v(Entry)
	return r.match(Entry, { id, amount, original: true }).find({ id, amount })
})
const differentComputedColumns = query(Theory)
	.rule((r) => {
		const { id, amount } = v(Entry)
		return r.match(Entry, { id, amount, original: true }).find({ id: Compute.add(id, Compute.u64(0n)), amount })
	})
	.rule((r) => {
		const { id, amount, previous } = v(Entry)
		return r
			.match(Entry, { id, amount, previous, original: false })
			.find({ id, amount: Compute.subtract(amount, previous) })
	})
	.rule((r) => {
		const { id, amount } = v(Entry)
		return r
			.match(Entry, { id, amount, original: true })
			.find({ id: Compute.add(id, Compute.u64(0n)), amount: Compute.add(amount, Compute.i64(0n)) })
	})
const explicit = query(Theory)
	.interior(
		"amounts",
		(r) => {
			const { id, amount } = v(Entry)
			return r.match(Entry, { id, amount, original: true }).find({ id, amount })
		},
		(r) => {
			const { id, amount, previous } = v(Entry)
			return r
				.match(Entry, { id, amount, previous, original: false })
				.find({ id, amount: Compute.subtract(amount, previous) })
		}
	)
	.rule((r) => {
		const { id, amount } = v(Entry)
		return r.interior("amounts", { id, amount }).find({ id, amount })
	})
const intervals = query(Theory)
	.rule((r) => {
		const { span } = v(Window)
		return r.match(Window, { span }).find({ span })
	})
	.rule((r) => {
		const row = v(Window)
		return r.match(Window, row).find({ span: r.difference(row.span, row.excluded) })
	})

test("projection roles align while each arm retains its real expression", () => {
	const ir = lowerQuery(direct)
	assert.deepEqual(ir.head, [{ kind: "var" }, { kind: "var" }])
	assert.equal(ir.rules[0]?.finds[1]?.kind, "var")
	assert.equal(ir.rules[1]?.finds[1]?.kind, "compute")
	assert.equal(lowerQuery(imported).interiors.length, 1)
	const reverse = corrections.rule((r) => {
		const { id, amount } = v(Entry)
		return r.match(Entry, { id, amount, original: true }).find({ id, amount })
	})
	assert.deepEqual(lowerQuery(reverse).head, [{ kind: "var" }, { kind: "compute" }])
	const row = v(direct)
	const signed: "i64" = row.amount.field.kind
	assert.equal(signed, "i64")
	assert.equal(Compute.add(row.amount, Compute.i64(1n)).result, "i64")
	const span = v(intervals).span
	const widened: undefined = span.field.width
	assert.equal(widened, undefined, "segment output cannot inherit the other arm's fixed width")
	assert.equal(intervals.data.finds[0]?.slot?.field.kind, "interval")
	const fixed = query(Theory)
		.rule((r) => {
			const row = v(Window)
			return r.match(Window, row).find({ span: row.span })
		})
		.rule((r) => {
			const row = v(Window)
			return r.match(Window, row).find({ span: row.span })
		})
	const width: 10n = v(fixed).span.field.width
	assert.equal(width, 10n, "equal refinements survive every arm")
	assert.throws(() => query(Theory).rule((r) => r.match(Window, v(Window)).find({ span: v(Window).span })), /not bound/)
})

test("incompatible folds, scalar types, rosters, and output names still refuse", () => {
	assert.throws(
		() =>
			original.rule((r) => {
				const { id, amount } = v(Entry)
				return r.match(Entry, { id, amount }).find({ id, amount: r.sum(amount) })
			}),
		/same head/
	)
	assert.throws(
		() =>
			original.rule((r) => {
				const { id, amount } = v(Entry)
				return r.match(Entry, { id, amount }).find({ id, amount: Compute.toU64Exact(amount) })
			}),
		/head column amount/
	)
	assert.throws(
		() =>
			original.rule((r) => {
				const { id, amount } = v(Entry)
				return r.match(Entry, { id, amount }).find({ amount, id })
			}),
		/same head/
	)
	const Left = closed("Left", ["A", "B"])
	const Right = closed("Right", ["A", "B"])
	const Pair = relation("Pair", { left: closedId(Left), right: closedId(Right) })
	const rosters = schema("Rosters", { Left, Right, Pair }, [])
	assert.throws(
		() =>
			query(rosters)
				.rule((r) => {
					const { left } = v(Pair)
					return r.match(Pair, { left }).find({ value: left })
				})
				.rule((r) => {
					const { right } = v(Pair)
					return r.match(Pair, { right }).find({ value: right })
				}),
		/one roster/
	)
})

test("a bare arm cannot hide incompatible carriers in any three-arm order", () => {
	const Left = relation("Left", { id: u64, parent: u64 })
	const Right = relation("Right", { id: u64, parent: u64 })
	const Bare = relation("Bare", { id: u64 })
	const theory = schema("Carriers", { Left, Right, Bare }, [
		key(Left, ["id"]),
		key(Right, ["id"]),
		contained(on(Left, "parent"), on(Left, "id")),
		contained(on(Right, "parent"), on(Right, "id"))
	])
	type Scope = QueryRuleScope<typeof theory.relations, typeof theory.classes>
	const arms = [
		(r: Scope) => {
			const { id } = v(Bare)
			return r.match(Bare, { id }).find({ id })
		},
		(r: Scope) => {
			const { id } = v(Left)
			return r.match(Left, { id }).find({ id })
		},
		(r: Scope) => {
			const { id } = v(Right)
			return r.match(Right, { id }).find({ id })
		}
	] as const
	const [bare, left, right] = arms
	for (const build of [
		() => query(theory).rule(bare).rule(left).rule(right),
		() => query(theory).rule(bare).rule(right).rule(left),
		() => query(theory).rule(left).rule(bare).rule(right),
		() => query(theory).rule(left).rule(right).rule(bare),
		() => query(theory).rule(right).rule(bare).rule(left),
		() => query(theory).rule(right).rule(left).rule(bare)
	])
		assert.throws(build, /head column id/)
	for (const joined of [
		query(theory).rule(bare).rule(left).rule(left),
		query(theory).rule(left).rule(bare).rule(left),
		query(theory).rule(left).rule(left).rule(bare)
	]) {
		assert.equal(joined.data.finds[0]?.slot?.class, undefined)
	}
})

test("mixed heads execute through imports and preserve sets, zero groups, intervals, and overflow", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("query-heads"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Entry, [
						{ id: 1n, amount: 10n, previous: 7n, original: true },
						{ id: 1n, amount: 10n, previous: 8n, original: true },
						{ id: 2n, amount: 2n, previous: 5n, original: false },
						{ id: 3n, amount: 5n, previous: 5n, original: false }
					])
					yield* draft.insert(Window, [{ span: { start: 0n, end: 10n }, excluded: { start: 4n, end: 6n } }])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const snapshot = yield* db.snapshot()
					const rows = ordered(yield* (yield* snapshot.execute(direct, {})).collect())
					assert.deepEqual(rows, [
						{ id: 1n, amount: 10n },
						{ id: 2n, amount: -3n },
						{ id: 3n, amount: 0n }
					])
					assert.deepEqual(ordered(yield* (yield* snapshot.execute(imported, {})).collect()), rows)
					assert.deepEqual(ordered(yield* (yield* snapshot.execute(reversed, {})).collect()), rows)
					assert.deepEqual(ordered(yield* (yield* snapshot.execute(explicit, {})).collect()), rows)
					assert.deepEqual(ordered(yield* (yield* snapshot.execute(differentComputedColumns, {})).collect()), rows)
					const totals = query(Theory).rule((r) => {
						const row = v(direct)
						return r.match(direct, row).find({ id: row.id, total: r.sum(row.amount), count: r.count() })
					})
					assert.deepEqual(ordered(yield* (yield* snapshot.execute(totals, {})).collect()), [
						{ id: 1n, total: 10n, count: 1n },
						{ id: 2n, total: -3n, count: 1n },
						{ id: 3n, total: 0n, count: 1n }
					])
					const measured = query(Theory).rule((r) => {
						const row = v(intervals)
						return r.match(intervals, row).find({ width: Compute.measure(row.span) })
					})
					const widths = (yield* (yield* snapshot.execute(measured, {})).collect()).map((row) => row.width)
					assert.deepEqual(widths.toSorted(compare), [4n, 10n])
					const bad = yield* ChangeSet.builder(Theory)
					yield* bad.insert(Entry, [{ id: 4n, amount: -(1n << 63n), previous: 1n, original: false }])
					yield* db.apply(yield* bad.finish(), { expected: { kind: "any" } })
					const next = yield* db.snapshot()
					const directFailure = yield* Effect.result(next.execute(direct, {}))
					const importedFailure = yield* Effect.result(next.execute(imported, {}))
					assert.ok(Result.isFailure(directFailure))
					assert.ok(Result.isFailure(importedFailure))
					assert.deepEqual(directFailure.failure.reason, importedFailure.failure.reason)
					assert.equal(directFailure.failure.reason._tag, "Engine")
					for (const form of [reversed, explicit]) {
						const failure = yield* Effect.result(next.execute(form, {}))
						assert.ok(Result.isFailure(failure))
						assert.deepEqual(failure.failure.reason, directFailure.failure.reason)
					}
					const filteredInside = query(Theory).rule((r) => {
						const { id, amount, previous } = v(Entry)
						return r
							.match(Entry, { id, amount, previous, original: false })
							.where(r.lt(id, 4n))
							.find({ id, amount: Compute.subtract(amount, previous) })
					})
					const filteredOutside = query(Theory).rule((r) => {
						const row = v(corrections)
						return r.match(corrections, row).where(r.lt(row.id, 4n)).find(row)
					})
					assert.deepEqual(yield* (yield* next.execute(filteredInside, {})).collect(), [
						{ id: 2n, amount: -3n },
						{ id: 3n, amount: 0n }
					])
					const stagedFailure = yield* Effect.result(next.execute(filteredOutside, {}))
					assert.ok(Result.isFailure(stagedFailure), "outer filtering cannot suppress an inner stage's overflow")
					assert.deepEqual(stagedFailure.failure.reason, directFailure.failure.reason)
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})

function typeChecks() {
	// @ts-expect-error Imported mixed i64 heads cannot silently promote to u64.
	Compute.add(v(direct).amount, Compute.u64(1n))
}
void typeChecks

test("mixed projection adapters survive empty and pruned arms in either order", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("query-heads-empty"), Theory)
					const empty = yield* db.snapshot()
					for (const form of [direct, reversed, explicit, differentComputedColumns]) {
						assert.deepEqual(yield* (yield* empty.execute(form, {})).collect(), [])
					}
					assert.deepEqual(yield* (yield* empty.execute(intervals, {})).collect(), [])
					const changes = yield* ChangeSet.builder(Theory)
					yield* changes.insert(Entry, [{ id: 1n, amount: 7n, previous: 1n, original: true }])
					yield* changes.insert(Window, [{ span: { start: 0n, end: 10n }, excluded: { start: 0n, end: 10n } }])
					yield* db.apply(yield* changes.finish(), { expected: { kind: "any" } })
					const source = yield* db.snapshot()
					const pruned = query(Theory)
						.rule((r) => {
							const { id, amount } = v(Entry)
							return r
								.match(Entry, { id, amount })
								.where(r.lt(id, 0n))
								.find({ id, amount: Compute.add(amount, Compute.i64(1n)) })
						})
						.rule((r) => {
							const { id, amount } = v(Entry)
							return r.match(Entry, { id, amount }).find({ id, amount })
						})
					for (const form of [direct, reversed, pruned]) {
						assert.deepEqual(yield* (yield* source.execute(form, {})).collect(), [{ id: 1n, amount: 7n }])
					}
					const reversedIntervals = query(Theory)
						.rule((r) => {
							const row = v(Window)
							return r.match(Window, row).find({ span: r.difference(row.span, row.excluded) })
						})
						.rule((r) => {
							const { span } = v(Window)
							return r.match(Window, { span }).find({ span })
						})
					for (const form of [intervals, reversedIntervals]) {
						assert.deepEqual(yield* (yield* source.execute(form, {})).collect(), [{ span: { start: 0n, end: 10n } }])
					}
				})
			)
		)
	} finally {
		await runtime.dispose()
	}
})
