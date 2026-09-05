/**
 * Shared scalar algebra (C1/D19/D27): one operator roster, kind-indexed
 * literals, distinct u64/i64/f64/bool, honest unresolved source-field leaves,
 * and query variable leaves through Compute. Construction is constant work
 * against cached summaries — not a whole-tree re-judgment.
 *
 * Verification: NotRun
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { Compute } from "#query/compute.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { Scalar, scalarAuthoringWork, type ScalarExpr } from "#scalar.ts"
import { Attempt, Learning } from "#test/fixtures/learning.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

describe("migration Scalar (field leaves, kind-indexed literals)", function migrationScalar() {
	test("literals carry distinct kinds — u64 and i64 do not collapse", function kindIndexed() {
		const u = Scalar.u64(5n)
		const i = Scalar.i64(-5n)
		const f = Scalar.f64(0.5)
		const b = Scalar.bool(true)
		assert.deepEqual(u, {
			kind: "literal",
			value: { u64: 5n },
			scope: "source-field",
			result: "u64",
			depth: 1
		})
		assert.deepEqual(i, {
			kind: "literal",
			value: { i64: -5n },
			scope: "source-field",
			result: "i64",
			depth: 1
		})
		assert.deepEqual(f, {
			kind: "literal",
			value: { f64: 0.5 },
			scope: "source-field",
			result: "f64",
			depth: 1
		})
		assert.deepEqual(b, {
			kind: "literal",
			value: { bool: true },
			scope: "source-field",
			result: "bool",
			depth: 1
		})
		const uIsU64: Equal<(typeof u)["result"], "u64"> = true
		const iIsI64: Equal<(typeof i)["result"], "i64"> = true
		assert.ok(uIsU64 && iIsI64)
	})

	test("field references stay unresolved — no caller-chosen result kind", function fieldUntyped() {
		const ref = Scalar.field("units")
		assert.deepEqual(ref, {
			kind: "field",
			name: "units",
			scope: "source-field",
			result: "unresolved",
			depth: 1
		})
		const unresolved: Equal<(typeof ref)["result"], "unresolved"> = true
		assert.ok(unresolved)
	})

	test("D27: Scalar.add(Scalar.field(\"units\"), Scalar.u64(1n)) constructs unresolved", function fieldArithmetic() {
		const expr = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
		assert.equal(expr.kind, "add")
		assert.equal(expr.result, "unresolved")
		assert.equal(expr.depth, 2)
		assert.equal(expr.scope, "source-field")
		assert.deepEqual(expr.left, {
			kind: "field",
			name: "units",
			scope: "source-field",
			result: "unresolved",
			depth: 1
		})
		assert.deepEqual(expr.right, {
			kind: "literal",
			value: { u64: 1n },
			scope: "source-field",
			result: "u64",
			depth: 1
		})
		const staysUnresolved: Equal<(typeof expr)["result"], "unresolved"> = true
		assert.ok(staysUnresolved)
	})

	test("nested explicit cast over a field is f64 — operand stays unresolved", function fieldCast() {
		const expr = Scalar.toF64(Scalar.add(Scalar.field("units"), Scalar.u64(1n)))
		assert.equal(expr.kind, "cast")
		assert.equal(expr.cast, "toF64")
		assert.equal(expr.result, "f64")
		assert.equal(expr.depth, 3)
		assert.equal(expr.expr.result, "unresolved")
	})

	test("known I64/U64 mixing refuses statically and at the constructor", function mixedKinds() {
		// @ts-expect-error — known i64/u64 mixing has no implicit promotion
		const refused = (): unknown => Scalar.add(Scalar.u64(1n), Scalar.i64(2n))
		assert.throws(refused, /operand kinds differ \(u64 vs i64\)/)
		assert.throws(function mixedUnion() {
			Scalar.add(
				Scalar.u64(1n) as ScalarExpr<"u64" | "i64" | "f64">,
				Scalar.i64(2n) as ScalarExpr<"u64" | "i64" | "f64">
			)
		}, /operand kinds differ \(u64 vs i64\)/)
		assert.throws(function f64u64() {
			Scalar.add(
				Scalar.f64(1) as ScalarExpr<"u64" | "i64" | "f64">,
				Scalar.u64(1n) as ScalarExpr<"u64" | "i64" | "f64">
			)
		}, /operand kinds differ \(f64 vs u64\)/)
	})

	test("negate, predicates and bool walls match the engine roster", function operatorWalls() {
		assert.throws(function negateU64() {
			Scalar.negate(Scalar.u64(1n) as ScalarExpr<"i64" | "f64">)
		}, /negation is defined over i64 and f64 only/)
		assert.throws(function boolAdd() {
			Scalar.add(
				Scalar.bool(true) as ScalarExpr<"u64" | "i64" | "f64">,
				Scalar.bool(false) as ScalarExpr<"u64" | "i64" | "f64">
			)
		}, /bool, not numeric/)
		const overField = Scalar.negate(Scalar.field("delta"))
		assert.equal(overField.result, "unresolved")
		assert.equal(Scalar.isFinite(Scalar.field("score")).result, "bool")
	})

	test("construction work is one admission per constructor — not a re-walk", function constantWork() {
		const before = scalarAuthoringWork()
		const units = Scalar.field("units")
		const one = Scalar.u64(1n)
		const inner = Scalar.add(units, one)
		const two = Scalar.u64(2n)
		const outer = Scalar.add(inner, two)
		assert.equal(scalarAuthoringWork() - before, 5)
		assert.equal(outer.depth, 3)
		assert.equal(outer.result, "unresolved")
	})

	test("cached depth refuses past the engine bound without re-judging children", function depthWall() {
		let expr: ScalarExpr<"f64"> = Scalar.f64(1)
		assert.throws(function grow() {
			for (let i = 0; i < 200; i += 1) {
				expr = Scalar.add(expr, Scalar.f64(1))
			}
		}, /deeper than 128 nodes/)
	})
})

describe("query Compute (variable leaves, shared roster)", function queryCompute() {
	test("variable kinds flow into derived results without bigint collapse", function varKinds() {
		const { score, units } = v(Attempt)
		const scaled = Compute.multiply(score, Compute.f64(2))
		const exact = Compute.toI64Exact(units)
		assert.equal(scaled.result, "f64")
		assert.equal(exact.result, "i64")
		assert.equal(scaled.scope, "query-var")
		const scaledIsF64: Equal<(typeof scaled)["result"], "f64"> = true
		const exactIsI64: Equal<(typeof exact)["result"], "i64"> = true
		assert.ok(scaledIsF64 && exactIsI64)
	})

	test("known query I64/U64 mixing fails without any/casts", function queryMix() {
		const { units } = v(Attempt)
		// @ts-expect-error — query u64 and i64 literals do not unify
		const refused = (): unknown => Compute.add(units, Compute.i64(2n))
		assert.throws(refused, /operand kinds differ \(u64 vs i64\)/)
	})

	test("query lowering converts shared literals to ValueSpec wire", function wireLiteral() {
		const q = query(Learning).rule(function rule(r) {
			const { id, units } = v(Attempt)
			return r.match(Attempt, { id, units }).find({ id, doubled: Compute.multiply(units, Compute.u64(2n)) })
		})
		const parsed = lowerQuery(q) as unknown as {
			readonly rules: ReadonlyArray<{ readonly finds: readonly unknown[] }>
		}
		const term = parsed.rules[0]?.finds[1] as { readonly expr?: { readonly value?: unknown } }
		assert.deepEqual(term?.expr?.value, { kind: "u64", value: 2n })
	})
})

describe("pure metadata — no native side effects", function pure() {
	test("scalar and compute construction is synchronous frozen data", function inert() {
		const { units } = v(Attempt)
		const expr = Scalar.add(Scalar.field("units"), Scalar.u64(1n))
		const compute = Compute.add(units, Compute.u64(2n))
		assert.equal(Object.isFrozen(expr), true)
		assert.equal(Object.isFrozen(compute), true)
		assert.equal(expr.scope, "source-field")
		assert.equal(compute.scope, "query-var")
	})
})
