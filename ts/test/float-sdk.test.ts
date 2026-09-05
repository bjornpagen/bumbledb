import assert from "node:assert/strict"
import { test } from "node:test"
import { AuthoringError, f64, InstanceBuilder, query, relation, schema, u64, v } from "#index.ts"
import { lowerQuery } from "#query/lower.ts"
import { wireParams } from "#query/run.ts"
import { accepted } from "#test/accepted.ts"

const Sample = relation("Sample", { id: u64, value: f64 })
const Samples = schema("FloatSdk", { Sample }, [])

test("f64 authoring stays plain metadata and parameters preserve the number domain", () => {
	assert.deepEqual(f64, { kind: "f64" })
	assert.equal(Object.isFrozen(f64), true)
	const selected = query(Samples).rule((r) => {
		const { id, value } = v(Sample)
		return r
			.match(Sample, { id, value })
			.where(r.ge(value, r.param("minimum")))
			.find({ id, value })
	})
	assert.deepEqual(wireParams(selected.data.params, { minimum: -0 }), [{ kind: "f64", value: -0 }])
	assert.throws(() => wireParams(selected.data.params, { minimum: 1n }), AuthoringError)
	assert.doesNotThrow(() => lowerQuery(selected))

	assert.throws(
		() =>
			lowerQuery(
				query(Samples).rule((r) => {
					const { value } = v(Sample)
					// @ts-expect-error No implicit bigint-to-f64 comparison.
					return r.match(Sample, { value }).where(r.lt(value, 1n)).find({ value })
				})
			),
		AuthoringError
	)
	assert.throws(
		() =>
			lowerQuery(
				query(Samples).rule((r) => {
					const { id } = v(Sample)
					// @ts-expect-error No implicit f64-to-u64 comparison.
					return r.match(Sample, { id }).where(r.gt(id, 1.5)).find({ id })
				})
			),
		AuthoringError
	)
})

test("typed f64 rows canonicalize zero/NaN and scalar/set query bindings remain exact", async () => {
	using builder = InstanceBuilder.create(Samples)
	const changed = builder.load(Sample, [
		{ id: 1n, value: -0 },
		{ id: 1n, value: 0 },
		{ id: 2n, value: Number.NaN },
		{ id: 2n, value: Number.NaN },
		{ id: 3n, value: Number.POSITIVE_INFINITY },
		{ id: 4n, value: Number.NEGATIVE_INFINITY },
		{ id: 5n, value: Number.MIN_VALUE }
	])
	assert.equal(changed.changed, 5n)
	using data = accepted(await builder.admit())
	assert.equal(data.count(Sample), 5n)
	assert.equal(data.contains(Sample, { id: 1n, value: -0 }), true)
	assert.equal(data.contains(Sample, { id: 2n, value: Number.NaN }), true)
	const selected = query(Samples).rule((r) => {
		const { id } = v(Sample)
		return r.match(Sample, { id, value: r.inSet("values") }).find({ id })
	})
	const answers: readonly { readonly id: bigint }[] = data.execute(data.prepare(selected), { values: [-0, Number.NaN] })
	assert.deepEqual(answers.map((row) => row.id).sort(), [1n, 2n])
	const zero = data.scan(Sample).find((row) => row.id === 1n)
	assert.ok(zero)
	assert.equal(Object.is(zero.value, -0), false)
	const maximum = query(Samples).rule((r) => {
		const { value } = v(Sample)
		return r.match(Sample, { value }).where(r.gt(value, Number.POSITIVE_INFINITY)).find({ value })
	})
	assert.deepEqual(data.execute(data.prepare(maximum), {}), [{ value: Number.NaN }])
})

test("sum and mean expose once-rounded numbers, including cancellation and repeated bindings", async () => {
	using builder = InstanceBuilder.create(Samples)
	builder.load(Sample, [
		{ id: 1n, value: 2 ** 53 },
		{ id: 2n, value: 1 },
		{ id: 3n, value: -(2 ** 53) },
		{ id: 4n, value: 1 }
	])
	using data = accepted(await builder.admit())
	const totals = query(Samples).rule((r) => {
		const { id, value } = v(Sample)
		return r.match(Sample, { id, value }).find({ sum: r.sum(value), mean: r.mean(value), count: r.count() })
	})
	const answers: readonly { readonly sum: number; readonly mean: number; readonly count: bigint }[] = data.execute(
		data.prepare(totals),
		{}
	)
	assert.deepEqual(answers, [{ sum: 2, mean: 0.5, count: 4n }])
	assert.throws(
		() =>
			query(Samples).rule((r) => {
				const { id } = v(Sample)
				// @ts-expect-error Integer mean requires an explicit cast stage.
				return r.match(Sample, { id }).find({ mean: r.mean(id) })
			}),
		AuthoringError
	)
})
