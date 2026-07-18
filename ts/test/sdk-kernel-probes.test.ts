/**
 * sdk-kernel bug-hunt pins (found 2026-07-17), restated on the STRUCTURAL
 * surface. Each test pins one formerly-confirmed defect at its fixed
 * behavior; the controls beside it pin the adjacent behavior that always
 * held.
 *
 *   1. covers() with a literal interval left operand — legal per the type
 *      surface and per the IR (`ir::CmpOp::PointIn` is interval-left,
 *      point-right; `Term::Literal(Value::IntervalU64)` is a legal lhs) —
 *      lowers to PointIn: comparison-literal tagging is op-aware and tags
 *      the interval-shaped literal by the point sibling's element domain.
 *      The structural surface also makes it a TYPE-level guarantee: the
 *      interval shape is legal exactly at `pointIn`/`covers`/`allen`
 *      positions.
 *   2. The unused-param law, structural form: params are typed BY USE and
 *      the registry is usage-derived, so a param VALUE no rule places
 *      never registers — the query lowers, prepares, and executes under
 *      exactly its inferred `Params` object (the old dead-declaration
 *      refusal is obsolete: there is no declaration to leave dead).
 *   3. closed() mints handle constants and axiom rows with own-property
 *      definition, so an object-protocol handle name ("__proto__") is a
 *      fully working handle instead of a silent prototype swap — and the
 *      constants are BARE bigints (no brand exists anywhere).
 */

import assert from "node:assert/strict"
import { test } from "node:test"

import { closed } from "#closed.ts"
import { interval, span, str, u64 } from "#fields.ts"
import { ALLEN } from "#query/atom.ts"
import type { QueryParams } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"

const HolderId = u64.as("HolderId")
const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Session = relation("Session", {
	holder: HolderId,
	at: u64,
	active: interval(u64)
})
const Probe = schema("Probe", { Holder, Session }, [])

test("CONTROL: allen() accepts a literal interval side (sibling is interval-typed)", function allenLiteralLeft() {
	const q = query(Probe).rule((r) =>
		r
			.match(Session, { active: r.var("iv") })
			.where(r.allen(span(0n, 12n), ALLEN.intersects, r.var("iv")))
			.select("iv")
	)
	assert.doesNotThrow(function lowerIt() {
		lowerQuery(q)
	})
})

test("CONTROL: covers() accepts an interval var left with a point literal right", function coversVarLeft() {
	const q = query(Probe).rule((r) =>
		r
			.match(Session, { active: r.var("iv") })
			.where(r.covers(r.var("iv"), 5n))
			.select("iv")
	)
	assert.doesNotThrow(function lowerIt() {
		lowerQuery(q)
	})
})

test("covers() with a literal interval left operand lowers to PointIn (interval-left, point-right)", function coversLiteralLeft() {
	/**
	 * "sessions whose timestamp falls inside a fixed window" — the literal
	 * interval is the lhs, exactly the IR's operand order; the point-typed
	 * sibling's element domain tags it intervalU64.
	 */
	const q = query(Probe).rule((r) =>
		r
			.match(Session, { holder: r.var("h"), at: r.var("t") })
			.where(r.covers(span(0n, 10n), r.var("t")))
			.select("h", "t")
	)
	const ir = lowerQuery(q)
	const conditions = ir.predicates[0]?.rules[0]?.conditions
	assert.ok(conditions !== undefined && conditions.length === 1)
	const leaf = conditions[0]
	assert.ok(leaf !== undefined && leaf.kind === "leaf")
	assert.deepEqual(leaf.cmp.op, { kind: "pointIn" })
	assert.deepEqual(leaf.cmp.lhs, {
		kind: "literal",
		value: { kind: "intervalU64", start: 0n, end: 10n }
	})
})

test("a param value no rule places never registers — the query lowers under its inferred Params", function unusedParam() {
	/**
	 * CONTROL: a USED param registers once, anchored by its binding field,
	 * and lands in the wire registry — the params contract the
	 * usage-derived registry protects.
	 */
	const used = query(Probe).rule((r) =>
		r
			.match(Holder, { id: r.var("h") })
			.where(r.eq(r.var("h"), r.param("wanted")))
			.select("h")
	)
	assert.deepEqual(
		used.data.params.map(function name(entry) {
			return entry.name
		}),
		["wanted"]
	)
	const usedParams: QueryParams<typeof used> = { wanted: 1n }
	assert.equal(typeof usedParams.wanted, "bigint")

	const q = query(Probe).rule((r) => {
		/**
		 * Created, placed in no rule: params are typed by USE, so this value
		 * contributes nothing — not to the inferred `Params` type, not to
		 * the wire registry, not to the lowered IR. The old dead-declaration
		 * refusal is obsolete because the dead declaration is unrepresentable.
		 */
		const ghost = r.param("ghost")
		assert.equal(ghost.name, "ghost")
		return r.match(Holder, { id: r.var("h") }).select("h")
	})
	assert.deepEqual(q.data.params, [], "the registry is usage-derived")
	const inferrred: QueryParams<typeof q> = {}
	assert.deepEqual(inferrred, {})
	const ir = lowerQuery(q)
	assert.equal(ir.predicates.length, 1, "the ghost never reaches the IR")
})

test("closed() handle constants are bare bigints for every admitted handle name", function protoHandle() {
	/**
	 * "__proto__" is a legal identifier (the macro analog admits it), so the
	 * constant must work — own-property definition shadows the
	 * object-protocol accessor instead of silently riding it. The computed
	 * access below is deliberate: it is exactly how a host loops a roster.
	 */
	const handles = ["Alpha", "__proto__"] as const
	const K = closed("K", handles)
	for (const handle of handles) {
		assert.equal(
			typeof K[handle],
			"bigint",
			`the ${handle} handle constant must be a bare bigint, never an accessor no-op`
		)
		assert.equal(K.fromId(K[handle]), handle, "the weld agrees with the constant")
	}
	assert.deepEqual(
		Object.keys(K.axioms).toSorted(),
		[...handles].toSorted(),
		"the axioms record carries every handle row as an own enumerable property"
	)
})
