import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime, Result } from "effect"
import { ChangeSet } from "#changes.ts"
import { Db } from "#db.ts"
import { f64, i64, interval, u64 } from "#fields.ts"
import { ALLEN } from "#index.ts"
import { Compute } from "#query/compute.ts"
import { describeQuery, queryFromDescription } from "#query/description.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { Scalar } from "#scalar.ts"
import { schema } from "#schema.ts"
import { key } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Earning = relation("Earning", { id: u64, schedule: u64, span: interval(u64) })
const Band = relation("Band", { id: u64, schedule: u64, span: interval(u64), numerator: u64 })
const Pair = relation("Pair", {
	id: u64,
	a: interval(i64),
	b: interval(i64),
	dense: interval(f64),
	clip: interval(f64),
	fixed: interval(i64, 3n)
})
const Theory = schema("IntervalOutputs", { Earning, Band, Pair }, [
	key(Earning, ["id"]),
	key(Band, ["id"]),
	key(Pair, ["id"])
])
const overlaps = query(Theory).rule((r) => {
	const { id: earning, schedule, span: earned } = v(Earning)
	const { id: band, span, numerator } = v(Band)
	return r
		.match(Earning, { id: earning, schedule, span: earned })
		.match(Band, { id: band, schedule, span, numerator })
		.find({ earning, band, numerator, span: r.intersection(earned, span) })
})
const weighted = query(Theory).rule((r) => {
	const row = v(overlaps)
	return r.match(overlaps, row).find({
		earning: row.earning,
		band: row.band,
		weighted: Compute.multiply(Compute.measure(row.span), row.numerator)
	})
})
const totals = query(Theory).rule((r) => {
	const row = v(weighted)
	return r.match(weighted, row).find({ earning: row.earning, total: r.sum(row.weighted) })
})
const rounded = query(Theory).rule((r) => {
	const row = v(totals)
	return r.match(totals, row).find({
		earning: row.earning,
		amount: Compute.mulDiv(row.total, Compute.u64(1n), Compute.u64(10n), "nearestTiesAwayFromZero")
	})
})

test("segment kinds survive description imports and fixed-width refinement is dropped", () => {
	const imported = queryFromDescription(Theory, describeQuery(overlaps), {
		earning: u64,
		band: u64,
		numerator: u64,
		span: interval(u64)
	})
	assert.deepEqual(describeQuery(imported), describeQuery(overlaps))
	const built = query(Theory).rule((r) => {
		const row = v(Pair)
		return r.match(Pair, row).find({ span: r.intersection(row.fixed, row.a) })
	})
	assert.deepEqual(v(built).span.field, interval(i64))
	assert.equal(Compute.measure(v(built).span).result, "u64")
	const row = v(Pair)
	assert.equal(Compute.measure(row.dense).result, "f64")
	const plain = query(Theory).rule((r) =>
		r.match(Pair, row).find({
			span: { kind: "segments", op: "difference", left: row.a, right: row.b }
		})
	)
	const constructed = query(Theory).rule((r) => r.match(Pair, row).find({ span: r.difference(row.a, row.b) }))
	assert.deepEqual(describeQuery(plain), describeQuery(constructed), "plain interval descriptors use the same algebra")
	assert.throws(() =>
		query(Theory).rule((r) => r.match(Pair, row).find({ bad: r.intersection(row.a, row.dense as never) }))
	)
	assert.throws(() => query(Theory).rule((r) => r.match(Pair, { a: row.a }).find({ bad: r.difference(row.a, row.b) })))
	assert.throws(() =>
		query(Theory).rule((r) => r.match(Pair, row).find({ bad: { ...r.difference(row.a, row.b), ignored: true } }))
	)
	assert.throws(() =>
		query(Theory).rule((r) => r.match(Pair, row).find({ span: r.difference(row.a, row.b), count: r.count() }))
	)
	assert.throws(() => Scalar.mulDiv(Scalar.u64(1n), Scalar.field("unknown"), Scalar.i64(1n) as never, "towardZero"))
})

function typePins() {
	const row = v(Pair)
	query(Theory).rule((r) =>
		r.match(Pair, row).find({
			// @ts-expect-error No mixed interval elements.
			bad: r.intersection(row.a, row.dense)
		})
	)
	// @ts-expect-error Measure needs an interval.
	Compute.measure(row.id)
	// @ts-expect-error MulDiv has no mixed promotion.
	Compute.mulDiv(row.id, Compute.i64(1n), Compute.u64(2n), "towardZero")
	// @ts-expect-error Float multiply/divide is not this integer operation.
	Compute.mulDiv(Compute.f64(1), Compute.f64(2), Compute.f64(3), "towardZero")
	// @ts-expect-error Source-field known signed/unsigned operands cannot be mixed.
	Scalar.mulDiv(Scalar.u64(1n), Scalar.i64(1n), Scalar.field("d"), "towardZero")
}
void typePins

test("pack widens fixed intervals before imported difference and measurement stages", async () => {
	const Fixed = relation("Fixed", { span: interval(i64, 3n) })
	const Window = relation("Window", { span: interval(i64) })
	const theory = schema("FixedPacking", { Fixed, Window }, [])
	const packed = query(theory).rule((r) => {
		const row = v(Fixed)
		return r.match(Fixed, row).find({ span: r.pack(row.span) })
	})
	const width: undefined = v(packed).span.field.width
	assert.equal(width, undefined)
	const imported = queryFromDescription(theory, describeQuery(packed), { span: interval(i64) })
	const pieces = query(theory).rule((r) => {
		const row = v(imported)
		const window = v(Window)
		return r
			.match(imported, row)
			.match(Window, window)
			.find({ span: r.difference(row.span, window.span) })
	})
	const measured = query(theory).rule((r) => {
		const row = v(pieces)
		return r.match(pieces, row).find({ span: row.span, width: Compute.measure(row.span) })
	})
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("fixed-pack-composition"), theory)
					const draft = yield* ChangeSet.builder(theory)
					yield* draft.insert(Fixed, [
						{ span: { start: -3n, end: 0n } },
						{ span: { start: 0n, end: 3n } },
						{ span: { start: 8n, end: 11n } }
					])
					yield* draft.insert(Window, [{ span: { start: -1n, end: 1n } }])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					assert.deepEqual(
						new Set(yield* (yield* snapshot.execute(imported, {})).collect()),
						new Set([{ span: { start: -3n, end: 3n } }, { span: { start: 8n, end: 11n } }])
					)
					assert.deepEqual(
						new Set(yield* (yield* snapshot.execute(measured, {})).collect()),
						new Set([
							{ span: { start: -3n, end: -1n }, width: 2n },
							{ span: { start: 1n, end: 3n }, width: 2n },
							{ span: { start: 8n, end: 11n }, width: 3n }
						])
					)
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("native marginal bands derive slices, preserve equal contributions, and round once", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("native-interval-bands"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Earning, [
						{ id: 1n, schedule: 1n, span: { start: 80n, end: 140n } },
						{ id: 2n, schedule: 2n, span: { start: 0n, end: 20n } },
						{ id: 3n, schedule: 1n, span: { start: 95n, end: (1n << 64n) - 1n } }
					])
					yield* draft.insert(Band, [
						{ id: 1n, schedule: 1n, span: { start: 0n, end: 100n }, numerator: 1n },
						{ id: 2n, schedule: 1n, span: { start: 100n, end: 200n }, numerator: 2n },
						{ id: 3n, schedule: 2n, span: { start: 0n, end: 10n }, numerator: 1n },
						{ id: 4n, schedule: 2n, span: { start: 10n, end: 20n }, numerator: 1n }
					])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snap = yield* db.snapshot()
					const answer = yield* (yield* snap.execute(rounded, {})).collect()
					assert.deepEqual(
						[...answer].sort((a, b) => Number(a.earning - b.earning)),
						[
							{ earning: 1n, amount: 10n },
							{ earning: 2n, amount: 2n },
							{ earning: 3n, amount: 21n }
						]
					)
					const imported = queryFromDescription(Theory, describeQuery(rounded), { earning: u64, amount: u64 })
					assert.deepEqual(new Set(yield* (yield* snap.execute(imported, {})).collect()), new Set(answer))
					const unbounded = query(Theory).rule((r) => {
						const row = v(Earning)
						return r.match(Earning, row).find({ width: Compute.measure(row.span) })
					})
					assert.ok(
						Result.isFailure(
							yield* Effect.result(
								Effect.gen(function* () {
									return yield* (yield* snap.execute(unbounded, {})).collect()
								})
							)
						)
					)
					const downstream = query(Theory).rule((r) => {
						const row = v(unbounded)
						return r.match(unbounded, row).where(r.eq(row.width, 0n)).find(row)
					})
					assert.ok(
						Result.isFailure(
							yield* Effect.result(
								Effect.gen(function* () {
									return yield* (yield* snap.execute(downstream, {})).collect()
								})
							)
						)
					)
					// A failed execution releases its state; an independent retry still works.
					assert.deepEqual(new Set(yield* (yield* snap.execute(rounded, {})).collect()), new Set(answer))
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("difference emits products, dense measures and fixed intervals use native meanings", async () => {
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("native-interval-difference"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Pair, [
						{
							id: 1n,
							a: { start: 0n, end: 10n },
							b: { start: 3n, end: 7n },
							dense: { start: -1.5, end: 2.5 },
							clip: { start: 0, end: 1 },
							fixed: { start: -1n, end: 2n }
						}
					])
					yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })
					const snap = yield* db.snapshot()
					const pieces = query(Theory).rule((r) => {
						const row = v(Pair)
						return r.match(Pair, row).find({
							id: row.id,
							x: r.difference(row.a, row.b),
							y: r.difference(row.dense, row.clip),
							fixed: Compute.measure(row.fixed)
						})
					})
					const answer = yield* (yield* snap.execute(pieces, {})).collect()
					assert.equal(answer.length, 4)
					assert.deepEqual(
						new Set(answer.map((row) => [row.x.start, row.x.end, row.y.start, row.y.end, row.fixed])),
						new Set([
							[0n, 3n, -1.5, 0, 3n],
							[0n, 3n, 1, 2.5, 3n],
							[7n, 10n, -1.5, 0, 3n],
							[7n, 10n, 1, 2.5, 3n]
						])
					)
					const joined = query(Theory).rule((r) => {
						const row = v(pieces)
						const original = v(Pair)
						return r
							.match(pieces, row)
							.match(Pair, { id: row.id, b: original.b })
							.where(r.allen(row.x, ALLEN.meets, original.b))
							.where(r.not(Pair, { a: row.x }))
							.find({ id: row.id, span: row.x })
					})
					for (let run = 0; run < 3; run++) {
						assert.deepEqual(yield* (yield* snap.execute(joined, {})).collect(), [
							{ id: 1n, span: { start: 0n, end: 3n } }
						])
					}
					const packed = query(Theory).rule((r) => {
						const row = v(pieces)
						return r.match(pieces, row).find({ id: row.id, span: r.pack(row.x) })
					})
					assert.equal((yield* (yield* snap.execute(packed, {})).collect()).length, 2)
					const measured = query(Theory).rule((r) => {
						const row = v(pieces)
						return r.match(pieces, row).find({ width: Compute.measure(row.y) })
					})
					assert.deepEqual(yield* (yield* snap.execute(measured, {})).collect(), [{ width: 1.5 }])
					const empty = query(Theory).rule((r) => {
						const row = v(Pair)
						return r.match(Pair, row).find({ span: r.difference(row.a, row.a) })
					})
					assert.deepEqual(yield* (yield* snap.execute(empty, {})).collect(), [])
					for (const scalarFirst of [false, true]) {
						const absent = query(Theory).rule((r) => {
							const row = v(Pair)
							const span = r.difference(row.a, row.a)
							const value = Compute.mulDiv(Compute.u64(1n), Compute.u64(1n), Compute.u64(0n), "towardZero")
							return r.match(Pair, row).find(scalarFirst ? { value, span } : { span, value })
						})
						assert.deepEqual(
							yield* (yield* snap.execute(absent, {})).collect(),
							[],
							"an empty interval product removes the binding before scalar evaluation, regardless of column order"
						)
					}
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})
