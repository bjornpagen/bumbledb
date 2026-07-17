/**
 * sdk-kernel bug-hunt pins (found 2026-07-17, fixed same hunt). Each test
 * pins one formerly-confirmed defect at its fixed behavior; the controls
 * beside it pin the adjacent behavior that always held. Type-level
 * negative space is asserted in the types.test.ts convention.
 *
 *   1. covers() with a literal interval left operand — legal per the type
 *      surface and per the IR (`ir::CmpOp::PointIn` is interval-left,
 *      point-right; `Term::Literal(Value::IntervalU64)` is a legal lhs) —
 *      lowers to PointIn: `taggedCmpLiteral` is op-aware and tags the
 *      interval-shaped literal by the point sibling's element domain.
 *   2. A scope param no rule uses is a dead declaration that could satisfy
 *      NO params object (the inferred Params type is {} while the wire
 *      marshal is the full registry and the ENGINE's arity is
 *      usage-derived) — refused typed at lowering, so every query that
 *      prepares executes with exactly its inferred Params object.
 *   3. closed() mints handle constants and axiom rows with own-property
 *      definition, so an object-protocol handle name ("__proto__") is a
 *      fully working handle instead of a silent prototype swap.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, test } from "node:test"

import type { Brand, Count, WindowSpec } from "#index.ts"
import {
	ALLEN,
	allen,
	closed,
	covers,
	Db,
	interval,
	is,
	lowerQuery,
	match,
	query,
	relation,
	schema,
	span,
	str,
	u64
} from "#index.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

const HolderId = u64.newtype("HolderId")
const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Session = relation("Session", {
	holder: HolderId,
	at: u64,
	active: interval(u64)
})
const Probe = schema("Probe", { Holder, Session }, [])

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-kernel-probe-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/**
 * Type-level pins that HOLD (controls for the ban table and term walls):
 * a structural window literal is not a Count (the admission brand is
 * module-private), and the two banned raw spellings stay unwritable.
 */
type SoundCases = [
	Expect<Equal<{ window: WindowSpec } extends Count ? true : false, false>>,
	Expect<Equal<{ window: { kind: "floor"; lo: 1n } } extends Count ? true : false, false>>
]

test("CONTROL: allen() accepts a literal interval side (sibling is interval-typed)", function allenLiteralLeft() {
	const q = query(Probe, function build($) {
		const iv = $.var(Session.fields.active)
		return {
			rules: [[match(Session, { active: iv }), allen(span(0n, 12n), ALLEN.intersects, iv)]],
			select: { iv }
		}
	})
	assert.doesNotThrow(function lowerIt() {
		lowerQuery(q)
	})
})

test("CONTROL: covers() accepts an interval var left with a point literal right", function coversVarLeft() {
	const q = query(Probe, function build($) {
		const iv = $.var(Session.fields.active)
		return {
			rules: [[match(Session, { active: iv }), covers(iv, 5n)]],
			select: { iv }
		}
	})
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
	const q = query(Probe, function build($) {
		const t = $.var(Session.fields.at)
		const h = $.var(Session.fields.holder)
		return {
			rules: [[match(Session, { holder: h, at: t }), covers(span(0n, 10n), t)]],
			select: { h, t }
		}
	})
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

test("a declared param no rule uses is refused typed at prepare — never an unexecutable prepared query", async function unusedParam() {
	const db = await Db.create(path.join(tmpRoot, "store"), Probe)
	let adaId: Brand<bigint, "HolderId"> | undefined
	const seeded = db.write(function seed(tx) {
		adaId = tx.insert(Holder, { name: "ada" }).id
	})
	assert.ok(seeded.ok)
	assert.ok(adaId !== undefined)
	/**
	 * CONTROL: a USED param executes with exactly its inferred Params
	 * object — the params contract the dead-declaration refusal protects.
	 */
	const used = query(Probe, function build($) {
		const h = $.var(Holder.fields.id)
		const wanted = $.param("wanted", Holder.fields.id)
		return { rules: [[match(Holder, { id: h }), is(h, wanted)]], select: { h } }
	})
	const rows = db.execute(db.prepare(used), { wanted: adaId })
	assert.equal(rows.length, 1)
	const q = query(Probe, function build($) {
		const h = $.var(Holder.fields.id)
		/**
		 * Declared, used by no rule: the inferred Params type is {} (params
		 * ride item phantoms) while the wire marshal is the full registry and
		 * the ENGINE's arity is usage-derived — NO params object could
		 * satisfy the prepared query, so lowering refuses the declaration.
		 */
		$.param("ghost", Holder.fields.id)
		return { rules: [[match(Holder, { id: h })]], select: { h } }
	})
	assert.throws(function prepareGhost() {
		db.prepare(q)
	}, /declares param ghost but no rule uses it/)
	assert.throws(function lowerGhost() {
		lowerQuery(q)
	}, /declares param ghost but no rule uses it/)
})

test("closed() handle constants are branded bigints for every admitted handle name", function protoHandle() {
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
			`the ${handle} handle constant must be a branded bigint, never an accessor no-op`
		)
		assert.equal(K.fromId(K[handle]), handle, "the weld agrees with the constant")
	}
	assert.deepEqual(
		Object.keys(K.axioms).toSorted(),
		[...handles].toSorted(),
		"the axioms record carries every handle row as an own enumerable property"
	)
})

export type { SoundCases }
