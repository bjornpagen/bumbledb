/**
 * TS-012 discriminator: tagged Scalar.bool backfills serialize through the
 * shared plan codec without a migration-only literal wrapper.
 */
import assert from "node:assert/strict"
import { describe, it } from "node:test"
import { Scalar } from "@bjornpagen/bumbledb"
import { planExpressionOf } from "#migrations/expr.ts"

describe("scalar backfill codec", () => {
	it("accepts Scalar.bool(false) as a canonical plan literal", () => {
		const outcome = planExpressionOf(Scalar.bool(false))
		assert.equal(outcome.ok, true)
		if (!outcome.ok) {
			return
		}
		assert.deepEqual(outcome.expression, { kind: "literal", value: { bool: false } })
	})

	it("refuses an ambiguous raw boolean literal node", () => {
		const outcome = planExpressionOf({ kind: "literal", value: false })
		assert.equal(outcome.ok, false)
	})

	it("serializes symbolic source-field arithmetic without evaluating it (D27)", () => {
		const expression = {
			kind: "add",
			left: { kind: "field", name: "units" },
			right: { kind: "literal", value: { u64: 1n } }
		}
		const outcome = planExpressionOf(expression)
		assert.equal(outcome.ok, true)
		if (!outcome.ok) {
			return
		}
		assert.deepEqual(outcome.expression, {
			kind: "add",
			left: { kind: "field", name: "units" },
			right: { kind: "literal", value: { u64: "1" } }
		})
		assert.deepEqual([...outcome.fields], ["units"])
		try {
			const authored = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
			const fromScalar = planExpressionOf(authored)
			assert.equal(fromScalar.ok, true)
		} catch (cause) {
			assert.fail(
				`L15 Scalar.add(Scalar.field("units"), Scalar.u64(1n)) must construct; native compile binds the field. ${String(cause)}`
			)
		}
	})
})
