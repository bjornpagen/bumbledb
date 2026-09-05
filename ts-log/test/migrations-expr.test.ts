/**
 * Exact core `ScalarExpr` reuse (TS-MIG-10, C01): the generator SERIALIZES
 * the core's typed scalar AST into canonical plan data — same roster, same
 * spellings, bounded walk — and refuses anything outside the frozen grammar
 * (closures, module paths, unknown kinds, unknown casts) instead of falling
 * back. Literal spellings are the canonical value arms: quiet-NaN/plus-zero
 * f64 bits, decimal integer strings, lowercase hex identities.
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { planExpressionOf, planValueOf } from "#migrations/expr.ts"

function ok(expression: unknown) {
	const outcome = planExpressionOf(expression)
	assert.ok(outcome.ok, `expected serialization, got: ${outcome.ok ? "" : outcome.detail}`)
	return outcome
}

function bad(expression: unknown): string {
	const outcome = planExpressionOf(expression)
	assert.ok(!outcome.ok, "expected refusal")
	return outcome.detail
}

describe("the frozen expression roster round-trips exactly", function suite() {
	test("every roster node serializes to its canonical plan spelling", function roster() {
		const expression = {
			kind: "add",
			left: {
				kind: "multiply",
				left: { kind: "field", name: "x" },
				right: { kind: "literal", value: { i64: "-3" } }
			},
			right: {
				kind: "negate",
				expr: {
					kind: "cast",
					cast: "toF64",
					expr: { kind: "subtract", left: { kind: "field", name: "y" }, right: { kind: "field", name: "x" } }
				}
			}
		}
		const outcome = ok(expression)
		assert.deepEqual(outcome.expression, expression)
		// Referenced source fields are collected exactly once for loss accounting.
		assert.deepEqual([...outcome.fields].sort(), ["x", "y"])
		ok({ kind: "isNaN", expr: { kind: "field", name: "f" } })
		ok({ kind: "isFinite", expr: { kind: "field", name: "f" } })
		ok({ kind: "divide", left: { kind: "field", name: "a" }, right: { kind: "literal", value: { $f64: "3ff0000000000000" } } })
		for (const cast of ["toF64", "toF64Exact", "toI64Exact", "toU64Exact"]) {
			ok({ kind: "cast", cast, expr: { kind: "field", name: "n" } })
		}
	})

	test("float literals canonicalize NaN and minus zero to the core quotient", function floats() {
		assert.deepEqual(planValueOf({ f64: Number.NaN }), { $f64: "7ff8000000000000" })
		assert.deepEqual(planValueOf({ f64: -0 }), { $f64: "0000000000000000" })
		assert.deepEqual(planValueOf({ f64: 1 }), { $f64: "3ff0000000000000" })
		assert.deepEqual(planValueOf({ $f64: "7ff8000000000000" }), { $f64: "7ff8000000000000" })
		// Noncanonical bit text refuses: uppercase, wrong width.
		assert.equal(typeof planValueOf({ $f64: "7FF8000000000000" }), "string")
		assert.equal(typeof planValueOf({ $f64: "3ff" }), "string")
	})

	test("integer literals accept host bigints and canonical decimal strings, in range only", function integers() {
		assert.deepEqual(planValueOf({ u64: 18446744073709551615n }), { u64: "18446744073709551615" })
		assert.deepEqual(planValueOf({ u64: "0" }), { u64: "0" })
		assert.equal(typeof planValueOf({ u64: 18446744073709551616n }), "string")
		assert.equal(typeof planValueOf({ u64: "-1" }), "string")
		assert.equal(typeof planValueOf({ u64: "01" }), "string")
		assert.deepEqual(planValueOf({ i64: -9223372036854775808n }), { i64: "-9223372036854775808" })
		assert.equal(typeof planValueOf({ i64: 9223372036854775808n }), "string")
	})

	test("identity, bytes and interval literals require canonical spellings", function identities() {
		assert.deepEqual(planValueOf({ id128: "0f".repeat(16) }), { id128: "0f".repeat(16) })
		assert.equal(typeof planValueOf({ id128: "0F".repeat(16) }), "string")
		assert.equal(typeof planValueOf({ id128: "0f".repeat(15) }), "string")
		assert.deepEqual(planValueOf({ fixedBytes: new Uint8Array([0, 255]) }), { fixedBytes: "00ff" })
		assert.deepEqual(planValueOf({ intervalU64: ["1", "5"] }), { intervalU64: ["1", "5"] })
		assert.deepEqual(planValueOf({ intervalI64: [-2n, 3n] }), { intervalI64: ["-2", "3"] })
		assert.deepEqual(planValueOf({ intervalF64: [0, 1] }), {
			intervalF64: ["0000000000000000", "3ff0000000000000"]
		})
		assert.equal(typeof planValueOf({ intervalU64: ["1"] }), "string")
		assert.equal(typeof planValueOf({ unknownArm: true }), "string")
		assert.equal(typeof planValueOf({ bool: true, u64: "1" }), "string")
	})

	test("anything outside the frozen grammar refuses — no callback or eval escape", function escape() {
		assert.ok(bad(() => false).includes("functions, promises and plain hosts are not plan data"))
		assert.ok(bad({ kind: "jsEval", source: "process.exit(1)" }).includes("unsupported expression node"))
		assert.ok(bad({ kind: "cast", cast: "toString", expr: { kind: "field", name: "x" } }).includes("unknown cast"))
		assert.ok(bad({ kind: "field", name: "" }).includes("bounded source field name"))
		assert.ok(bad(null).length > 0)
		assert.ok(bad(undefined).length > 0)
	})

	test("depth and node budgets bound the walk", function budgets() {
		let deep: unknown = { kind: "field", name: "x" }
		for (let index = 0; index < 129; index += 1) {
			deep = { kind: "negate", expr: deep }
		}
		assert.ok(bad(deep).includes("deeper than 128"))
		// A wide tree over the node budget refuses too.
		let wide: unknown = { kind: "field", name: "x" }
		for (let index = 0; index < 64; index += 1) {
			wide = { kind: "add", left: wide, right: wide }
		}
		// 64 doublings explode past 4096 nodes long before depth 128.
		assert.ok(bad(wide).includes("larger than 4096 nodes"))
	})
})
