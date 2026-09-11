import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { Compute, type ComputeExpr, MAX_COMPUTE_DEPTH } from "#query/compute.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { scalarAuthoringWork } from "#scalar.ts"
import { Attempt, Learning } from "#test/fixtures/learning.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

describe("query Compute (variable leaves, shared roster)", function queryCompute() {
	test("variable kinds flow into derived results without bigint collapse", function varKinds() {
		const { score, units } = v(Attempt)
		const scaled = Compute.multiply(score, Compute.f64(2))
		const exact = Compute.toI64Exact(units)
		assert.equal(scaled.result, "f64")
		assert.equal(exact.result, "i64")
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
		const term = lowerQuery(q).rules[0]?.finds[1]
		assert.equal(term?.kind, "compute")
		assert.ok(term?.kind === "compute")
		assert.equal(term.expr.kind, "multiply")
		assert.ok(term.expr.kind === "multiply")
		assert.deepEqual(term.expr.right, { kind: "literal", value: { kind: "u64", value: 2n } })
	})
})

test("query constructors cache depth and do constant work", () => {
	let expr: ComputeExpr<"f64"> = Compute.f64(1)
	const before = scalarAuthoringWork()
	for (let depth = 1; depth < MAX_COMPUTE_DEPTH; depth++) expr = Compute.add(expr, Compute.f64(1))
	assert.equal(scalarAuthoringWork() - before, 2 * (MAX_COMPUTE_DEPTH - 1))
	assert.equal(expr.depth, MAX_COMPUTE_DEPTH)
	assert.equal(Object.isFrozen(expr), true)
	assert.throws(() => Compute.add(expr, Compute.f64(1)), /deeper than/)
})
