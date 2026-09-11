import assert from "node:assert/strict"
import { test } from "node:test"
import { Effect, ManagedRuntime } from "effect"
import { ChangeSet } from "#changes.ts"
import { closed, closedId } from "#closed.ts"
import { Db } from "#db.ts"
import { on } from "#face.ts"
import { bool, f64, i64, interval, str, u64 } from "#fields.ts"
import { Compute } from "#query/compute.ts"
import type { QueryRow } from "#query/lower.ts"
import { query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { NativeRuntime } from "#runtime.ts"
import { schema } from "#schema.ts"
import { contained, key } from "#statements.ts"
import { runtimeOptions, storeDir } from "#test/fixtures/learning.ts"

const Kind = closed("Kind", ["One", "Two"])
const Measure = relation("Measure", { id: u64, group: u64, units: u64, signed: i64, rate: u64 })
const Label = relation("Label", {
	id: u64,
	kind: closedId(Kind),
	flag: bool,
	text: str,
	span: interval(i64, 1n),
	score: f64
})
const Theory = schema("ImportedMeasures", { Kind, Measure, Label }, [
	key(Measure, ["id"]),
	key(Label, ["id"]),
	contained(on(Label, "kind"), on(Kind, "id"))
])

const weighted = query(Theory)
	.rule((r) => {
		const { id, group, units, rate } = v(Measure)
		return r.match(Measure, { id, group, units, rate }).find({ id, group, weighted: Compute.multiply(units, rate) })
	})
	.named("weighted")
const totals = query(Theory).rule((r) => {
	const row = v(weighted)
	return r.match(weighted, row).find({ group: row.group, total: r.sum(row.weighted) })
})
const rounded = query(Theory).rule((r) => {
	const row = v(totals)
	return r.match(totals, row).find({
		group: row.group,
		amount: Compute.divide(
			Compute.add(Compute.multiply(row.total, Compute.u64(2n)), Compute.u64(10n)),
			Compute.u64(20n)
		)
	})
})

test("imported computed/aggregate columns retain numeric kinds through native composition", async () => {
	const expected: QueryRow<typeof rounded>[] = [
		{ group: 1n, amount: 2n },
		{ group: 2n, amount: 1n }
	]
	const runtime = ManagedRuntime.make(NativeRuntime.layer(runtimeOptions))
	try {
		await runtime.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					const db = yield* Db.create(storeDir("imported-compute"), Theory)
					const draft = yield* ChangeSet.builder(Theory)
					yield* draft.insert(Measure, [
						{ id: 1n, group: 1n, units: 1n, signed: -1n, rate: 6n },
						{ id: 2n, group: 1n, units: 1n, signed: -1n, rate: 9n },
						{ id: 3n, group: 2n, units: 1n, signed: -1n, rate: 3n },
						{ id: 4n, group: 2n, units: 1n, signed: -1n, rate: 3n }
					])
					assert.equal((yield* db.apply(yield* draft.finish(), { expected: { kind: "any" } })).kind, "accepted")
					const snapshot = yield* db.snapshot()
					assert.deepEqual(yield* (yield* snapshot.execute(rounded, {})).collect(), expected)
					assert.deepEqual(yield* (yield* snapshot.execute(totals, {})).collect(), [
						{ group: 1n, total: 15n },
						{ group: 2n, total: 6n }
					])
				})
			)
		)
	} finally {
		await Effect.runPromise(runtime.disposeEffect)
	}
})

test("imported projections preserve signedness, interval descriptors and closed carriers", () => {
	const measurements = query(Theory).rule((r) => {
		const row = v(Measure)
		return r.match(Measure, row).find(row)
	})
	const { units, signed } = v(measurements)
	assert.equal(Compute.add(signed, Compute.i64(1n)).result, "i64")
	assert.throws(() => {
		// @ts-expect-error An imported i64 and u64 cannot share arithmetic.
		Compute.add(signed, units)
	}, /operand kinds differ/)
	const labels = query(Theory).rule((r) => {
		const row = v(Label)
		return r.match(Label, row).find(row)
	})
	const row = v(labels)
	const projected = query(Theory).rule((r) => r.match(labels, row).match(Kind, { id: row.kind }).find(row))
	const value: QueryRow<typeof projected> = {
		id: 1n,
		kind: "One",
		flag: true,
		text: "a",
		span: { start: 0n, end: 1n },
		score: 1.5
	}
	assert.equal(value.kind, "One")
	const packed = query(Theory).rule((r) =>
		r.match(labels, row).find({ span: r.pack(row.span), count: r.count(), mean: r.mean(row.score) })
	)
	const packedRow = v(packed)
	assert.equal(packedRow.span.field.width, 1n)
	assert.equal(Compute.add(packedRow.count, Compute.u64(1n)).result, "u64")
	assert.equal(Compute.add(packedRow.mean, Compute.f64(1)).result, "f64")
	assert.throws(() => {
		// @ts-expect-error Imported text remains nonnumeric.
		Compute.add(row.text, Compute.u64(1n))
	}, /reads u64\/i64\/f64\/bool/)
})
