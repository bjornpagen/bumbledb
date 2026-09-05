/**
 * Computed find terms (C05 `FindTerm::Compute`) — authoring walls, wire
 * lowering and row typing over the shared L15 scalar grammar. Pure
 * metadata throughout: nothing here touches the native runtime. The wire
 * spelling asserted below is the agreed C05 lane landed in `#native.ts`/
 * `ts/crate`: `FindTermIr` `{ kind: "compute", expr: ScalarExprIr }` and
 * `HeadTermIr` `{ kind: "compute" }`. Compute constructors are the query-var
 * scope of `#scalar.ts` — not a second roster.
 *
 * D19: known query I64/U64 mixing fails without any/casts.
 * Verification: NotRun
 */
import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { AnyComputeExpr, ComputeExpr } from "#query/compute.ts"
import { Compute } from "#query/compute.ts"
import type { QueryRow } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import type { Id128 } from "#id128.ts"
import { Attempt, Learning, Student } from "#test/fixtures/learning.ts"

/** Reads one lowered find term structurally (the wire arm is P06R2's). */
function findTermAt(parsed: unknown, position: number): Record<string, unknown> {
	const rules = (parsed as { readonly rules: ReadonlyArray<{ readonly finds: readonly unknown[] }> }).rules
	const rule = rules[0]
	assert.notEqual(rule, undefined)
	const term = rule?.finds[position]
	assert.notEqual(term, undefined)
	return term as Record<string, unknown>
}

describe("Compute construction walls (engine result_type parity)", function walls() {
	test("mixed numeric kinds refuse — no implicit promotion", function mixed() {
		const { score, units } = v(Attempt)
		assert.throws(function mixedAdd() {
			// @ts-expect-error — known f64/u64 query kinds do not unify
			Compute.add(score, units)
		}, /operand kinds differ \(f64 vs u64\)/)
		assert.throws(function mixedLit() {
			// @ts-expect-error — known query u64/i64 mixing has no implicit promotion
			Compute.multiply(units, Compute.i64(2n))
		}, /operand kinds differ \(u64 vs i64\)/)
	})

	test("negate is defined over i64 and f64 only", function negateWall() {
		const { units, score } = v(Attempt)
		assert.throws(function negateU64() {
			Compute.negate(units)
		}, /negation is defined over i64 and f64 only/)
		assert.equal(Compute.negate(score).result, "f64")
	})

	test("the float predicates read f64 (compile + runtime walls)", function predicateWall() {
		const { units, score } = v(Attempt)
		assert.throws(function isNaNU64() {
			// @ts-expect-error — a u64 variable is not a float-predicate operand.
			Compute.isNaN(units)
		}, /the float predicates read f64/)
		assert.equal(Compute.isFinite(score).result, "bool")
	})

	test("non-scalar variables refuse at the leaf (compile + runtime walls)", function leafWall() {
		const { active } = v(Attempt)
		const { name } = v(Student)
		assert.throws(function intervalLeaf() {
			// @ts-expect-error — an interval variable never enters arithmetic.
			Compute.toF64(active)
		}, /reads u64\/i64\/f64\/bool variables only/)
		assert.throws(function strLeaf() {
			// @ts-expect-error — a str variable never enters arithmetic.
			Compute.add(name, Compute.f64(1))
		}, /reads u64\/i64\/f64\/bool variables only/)
	})

	test("bool never enters arithmetic; casts take numerics only", function boolWall() {
		const { score } = v(Attempt)
		const flag = Compute.isNaN(score)
		assert.throws(function boolAdd() {
			// @ts-expect-error — a bool expression is not an arithmetic operand.
			Compute.add(flag, flag)
		}, /bool, not numeric/)
		assert.throws(function boolCast() {
			// @ts-expect-error — a bool expression is not a cast operand.
			Compute.toU64Exact(flag)
		}, /bool, not numeric/)
	})

	test("literals are explicitly tagged and range-checked", function literals() {
		assert.equal(Compute.u64(5n).result, "u64")
		assert.equal(Compute.i64(-5n).result, "i64")
		assert.equal(Compute.f64(0.5).result, "f64")
		assert.equal(Compute.bool(true).result, "bool")
		assert.throws(function negativeU64() {
			Compute.u64(-1n)
		}, /0\.\.=2\^64-1/)
		assert.throws(function oversizedI64() {
			Compute.i64(1n << 63n)
		}, /-2\^63\.\.=2\^63-1/)
	})

	test("trees deeper than the engine's 128-node bound refuse", function depthWall() {
		const { score } = v(Attempt)
		let expr: ComputeExpr<"f64"> = Compute.toF64(score)
		assert.throws(function grow() {
			for (let i = 0; i < 200; i += 1) {
				expr = Compute.add(expr, Compute.f64(1))
			}
		}, /deeper than 128 nodes/)
	})

	test("construction is inert frozen metadata", function inert() {
		const { score } = v(Attempt)
		const expr = Compute.multiply(score, Compute.f64(2))
		assert.equal(Object.isFrozen(expr), true)
		assert.equal(expr.scope, "query-var")
		assert.equal(expr.kind, "multiply")
	})
})

describe("computed find lowering (the recorded C05 wire)", function lowering() {
	test("a compute column lowers to { kind: compute, expr } under a compute head", function wire() {
		const scaled = query(Learning).rule(function scaledRule(r) {
			const { id, score, units } = v(Attempt)
			return r
				.match(Attempt, { id, score, units })
				.find({
					id,
					scaled: Compute.multiply(score, Compute.f64(2)),
					exact: Compute.toF64Exact(units)
				})
		})
		const parsed = lowerQuery(scaled) as unknown as {
			readonly head: ReadonlyArray<{ readonly kind: string }>
			readonly rules: ReadonlyArray<{
				readonly finds: readonly unknown[]
				readonly atoms: ReadonlyArray<{
					readonly bindings: ReadonlyArray<readonly [number, { readonly kind: string; readonly var?: number }]>
				}>
			}>
		}
		assert.deepEqual(
			parsed.head.map(function kindOf(term) {
				return term.kind
			}),
			["var", "compute", "compute"]
		)
		const rule = parsed.rules[0]
		assert.notEqual(rule, undefined)
		const bindings = rule?.atoms[0]?.bindings ?? []
		// Attempt's sealed field order: id(0) student(1) score(2) units(3) active(4).
		const varAt = new Map<number, number>()
		for (const [field, term] of bindings) {
			if (term.kind === "var" && term.var !== undefined) {
				varAt.set(field, term.var)
			}
		}
		const scoreVar = varAt.get(2)
		const unitsVar = varAt.get(3)
		assert.notEqual(scoreVar, undefined)
		assert.notEqual(unitsVar, undefined)
		assert.deepEqual(findTermAt(parsed, 1), {
			kind: "compute",
			expr: {
				kind: "multiply",
				left: { kind: "var", var: scoreVar },
				right: { kind: "literal", value: { kind: "f64", value: 2 } }
			}
		})
		assert.deepEqual(findTermAt(parsed, 2), {
			kind: "compute",
			expr: { kind: "cast", cast: "toF64Exact", expr: { kind: "var", var: unitsVar } }
		})
	})

	test("a compute over an unbound variable refuses at find", function unbound() {
		assert.throws(function unboundVar() {
			query(Learning).rule(function badRule(r) {
				const { id, score } = v(Attempt)
				const { budget } = v(Student)
				return r.match(Attempt, { id, score }).find({ id, over: Compute.toF64(budget) })
			})
		}, /not bound by a relation atom/)
	})

	test("the recursive head refuses computed columns", function recWall() {
		assert.throws(function computeInRec() {
			query(Learning)
				.reach("chain", {
					base: [
						function base(r) {
							const { id, student } = v(Attempt)
							return r.match(Attempt, { id, student }).find({ id, student })
						}
					],
					rec: [
						function step(r) {
							const { id, student, score } = v(Attempt)
							return (
								r
									.match(Attempt, { id, student, score })
									// @ts-expect-error — a rec head is projection-only:
									// CheckRecFind refuses compute entries at the type
									// tier; the runtime wall below is its twin.
									.find({ id, student: Compute.toU64Exact(score) })
							)
						}
					]
				})
				.rule(function main(r) {
					const { id } = v(Attempt)
					return r.match(Attempt, { id }).find({ id })
				})
		}, /rec head projects bound variables only/)
	})

	test("an interior head may carry a computed column (nonrecursive stage)", function interiorCompute() {
		const staged = query(Learning)
			.interior("scaled", function stage(r) {
				const { id, score } = v(Attempt)
				return r.match(Attempt, { id, score }).find({ id, scaled: Compute.multiply(score, Compute.f64(2)) })
			})
			.rule(function main(r) {
				const { id } = v(Attempt)
				return r.match(Attempt, { id }).find({ id })
			})
		const parsed = lowerQuery(staged) as unknown as {
			readonly interiors: ReadonlyArray<{ readonly head: ReadonlyArray<{ readonly kind: string }> }>
		}
		assert.deepEqual(
			parsed.interiors[0]?.head.map(function kindOf(term) {
				return term.kind
			}),
			["var", "compute"]
		)
	})
})

describe("computed find typing (compile-time pins)", function typing() {
	test("the row type carries the expression's host type", function rowType() {
		const scaled = query(Learning).rule(function scaledRule(r) {
			const { id, score, units } = v(Attempt)
			return r.match(Attempt, { id, score, units }).find({
				id,
				scaled: Compute.multiply(score, Compute.f64(2)),
				exactUnits: Compute.toI64Exact(units),
				finite: Compute.isFinite(score)
			})
		})
		type Row = QueryRow<typeof scaled>
		const witness: Row = {
			id: "00000000000000000000000000000000" as Id128,
			scaled: 1.5,
			exactUnits: 3n,
			finite: true
		}
		assert.equal(typeof witness.scaled, "number")

		// @ts-expect-error — a computed f64 column is a number, not a bigint.
		const wrong: Row = { ...witness, scaled: 2n }
		assert.notEqual(wrong, undefined)
	})

	test("literal constructors demand their exact host types", function literalTypes() {
		// @ts-expect-error — an f64 literal is a number, not a bigint.
		assert.throws(() => Compute.f64(5n), /a number/)
		// @ts-expect-error — a u64 literal is a bigint, not a number.
		assert.throws(() => Compute.u64(5), /a bigint/)
	})

	test("a Compute expression is a legal FindEntry beside vars and aggregates", function entryUnion() {
		const { score } = v(Attempt)
		const entry: AnyComputeExpr = Compute.toF64(score)
		assert.equal(entry.result, "f64")
	})
})
