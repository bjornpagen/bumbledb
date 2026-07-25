/**
 * ORD-3 pins: host-side answer ordering. Sort keys are DATA — a bare
 * column name is ascending (the punning spelling), `desc(name)` is the one
 * descending spelling — and `by(...)` folds them into a single comparator
 * typed against the row (`Row extends Readonly<Record<K, FactValue>>`), so
 * a key the row lacks or a `number`-typed column is a COMPILE error at the
 * `.sort` call site (each `@ts-expect-error` below is real). The language
 * owns the sort and the limit (`.slice`) — the drizzle law; the SDK ships
 * only the comparator, because `Array.prototype.sort` wants a `number` and
 * the SDK's numeric domain is `bigint`. Rows here are plain frozen named
 * objects of bare structural values — exactly the decode shape
 * `decodeAnswers` produces (`ts/src/query/run.ts`): sort permutes the
 * array, never the rows. Runtime pins cover every cell arm — bigint across
 * sign, interval start-then-end, bytes bytewise-then-length, boolean
 * false<true, string — and the multi-key fold with `desc` plus tiebreak.
 *
 * THE IDENTITY KEY (ruled 2026-07-25): zero keys means the value IS the
 * key — `by()` / `desc()` are the ascending/descending comparators over
 * BARE engine-orderable scalars (`bigint[]` of ids, map keys; boolean
 * false < true per R3), typed to EXACTLY the orderable roster of
 * `10-data-model.md` § "Orderability, complete": `string` (deliberately
 * refused — intern ids are meaningless to order) and `number` (not an
 * engine scalar) are compile errors at the `.sort` site. The AGREEMENT
 * suite at the bottom is the one-owner pin: over one set of i64 values in
 * a REAL store, the host-sorted order and the engine's `Lt` judgment agree
 * at every cut — for every pivot, the engine's below-pivot answer set IS
 * the host-sorted prefix. (A bool order comparison is not yet spellable on
 * the TS query tier — `OrderVarOk` admits u64/i64 only — so bool's engine
 * arm is pinned by the engine's own R3 tests; the host arm's false < true
 * is pinned here.)
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import { bool, Db, i64, lt, query, relation, schema, u64, v } from "#index.ts"
import { by, desc } from "#order.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

test("by sorts rows ascending by one bare key", function bareKeyAscending() {
	const rows = [
		Object.freeze({ rank: 3n, name: "c" }),
		Object.freeze({ rank: 1n, name: "a" }),
		Object.freeze({ rank: 2n, name: "b" })
	]
	rows.sort(by("rank"))
	assert.deepEqual(rows, [
		{ rank: 1n, name: "a" },
		{ rank: 2n, name: "b" },
		{ rank: 3n, name: "c" }
	])
})

test("desc reverses and later keys break ties", function descAndTiebreak() {
	const rows = [
		Object.freeze({ a: 1n, b: 2n }),
		Object.freeze({ a: 2n, b: 9n }),
		Object.freeze({ a: 1n, b: 1n }),
		Object.freeze({ a: 2n, b: 3n })
	]
	rows.sort(by(desc("a"), "b"))
	assert.deepEqual(rows, [
		{ a: 2n, b: 3n },
		{ a: 2n, b: 9n },
		{ a: 1n, b: 1n },
		{ a: 1n, b: 2n }
	])
})

test("bigint keys order numerically across sign", function bigintAcrossSign() {
	const rows = [Object.freeze({ n: 3n }), Object.freeze({ n: -7n }), Object.freeze({ n: -5n })]
	rows.sort(by("n"))
	assert.deepEqual(rows, [{ n: -7n }, { n: -5n }, { n: 3n }])
})

test("interval cells order by start then end", function intervalStartThenEnd() {
	const rows = [
		Object.freeze({ w: Object.freeze({ start: 2n, end: 5n }) }),
		Object.freeze({ w: Object.freeze({ start: 2n, end: 3n }) }),
		Object.freeze({ w: Object.freeze({ start: 1n, end: 9n }) })
	]
	rows.sort(by("w"))
	assert.deepEqual(rows, [{ w: { start: 1n, end: 9n } }, { w: { start: 2n, end: 3n } }, { w: { start: 2n, end: 5n } }])
})

test("bytes cells order bytewise then by length", function bytesBytewiseThenLength() {
	const rows = [
		Object.freeze({ b: Uint8Array.of(1, 2, 3) }),
		Object.freeze({ b: Uint8Array.of(1, 3) }),
		Object.freeze({ b: Uint8Array.of(0, 9) }),
		Object.freeze({ b: Uint8Array.of(1, 2) })
	]
	rows.sort(by("b"))
	assert.deepEqual(rows, [
		{ b: Uint8Array.of(0, 9) },
		{ b: Uint8Array.of(1, 2) },
		{ b: Uint8Array.of(1, 2, 3) },
		{ b: Uint8Array.of(1, 3) }
	])
})

test("boolean and string cells order canonically", function booleanAndString() {
	const rows = [
		Object.freeze({ flag: true, s: "b" }),
		Object.freeze({ flag: false, s: "b" }),
		Object.freeze({ flag: true, s: "a" }),
		Object.freeze({ flag: false, s: "a" })
	]
	rows.sort(by("flag", "s"))
	assert.deepEqual(rows, [
		{ flag: false, s: "a" },
		{ flag: false, s: "b" },
		{ flag: true, s: "a" },
		{ flag: true, s: "b" }
	])
})

test("frozen decoded-shape rows sort through by", function frozenRowsSort() {
	const rows = [Object.freeze({ pos: 2n, s: "b" }), Object.freeze({ pos: 1n, s: "a" })]
	rows.sort(by("pos"))
	assert.deepEqual(rows, [
		{ pos: 1n, s: "a" },
		{ pos: 2n, s: "b" }
	])
	for (const row of rows) {
		assert.ok(Object.isFrozen(row), "sort permutes the array, never the rows")
	}
})

test("compile pins: a missing key and a number column refuse; by('n') is a row comparator", function compilePins() {
	const rows = [Object.freeze({ n: 2n }), Object.freeze({ n: 1n })]
	// @ts-expect-error — sorting by a key the row type lacks is a compile error at the sort site
	rows.sort(by("rank"))
	const numeric = [Object.freeze({ n: 1 })]
	// @ts-expect-error — a `number`-typed column is not a FactValue: the row constraint refuses it
	numeric.sort(by("n"))
	// The positive probe: by("n") IS assignable to the plain row-typed
	// comparator shape — the generic return instantiates at the sort site.
	const comparator: (left: { readonly n: bigint }, right: { readonly n: bigint }) => number = by("n")
	type ComparatorPin = Expect<
		Equal<typeof comparator, (left: { readonly n: bigint }, right: { readonly n: bigint }) => number>
	>
	rows.sort(comparator)
	assert.deepEqual(rows, [{ n: 1n }, { n: 2n }])
	const pins: [ComparatorPin] = [true]
	assert.equal(pins.length, 1)
})

test("by() sorts a bare bigint array ascending across the full i64 range", function identityAscendingBigint() {
	const values = [3n, -(2n ** 63n), 2n ** 63n - 1n, -7n, 0n]
	values.sort(by())
	assert.deepEqual(values, [-(2n ** 63n), -7n, 0n, 3n, 2n ** 63n - 1n])
})

test("desc() sorts a bare bigint array descending — the same owner, sides flipped", function identityDescendingBigint() {
	const values = [-7n, 2n ** 63n - 1n, 0n, -(2n ** 63n), 3n]
	values.sort(desc())
	assert.deepEqual(values, [2n ** 63n - 1n, 3n, 0n, -7n, -(2n ** 63n)])
})

test("the identity arms order booleans false < true — the strict 0/1 encoding IS the order (R3)", function identityBoolean() {
	const flags = [true, false, true, false]
	flags.sort(by())
	assert.deepEqual(flags, [false, false, true, true])
	flags.sort(desc())
	assert.deepEqual(flags, [true, true, false, false])
})

test("the identity comparators are minted once — by() === by(), desc() === desc()", function identityMintedOnce() {
	assert.equal(by(), by())
	assert.equal(desc(), desc())
	const arms: readonly unknown[] = [by(), desc()]
	assert.notEqual(arms[0], arms[1])
})

test("compile pins: the identity arms cover EXACTLY the engine-orderable roster", function identityCompilePins() {
	/**
	 * The orderability law at the type tier (`10-data-model.md`
	 * § "Orderability, complete", ruled 2026-07-23 R3/R4): the identity
	 * comparators constrain on `EngineOrderable = bigint | boolean`, so the
	 * two non-members refuse at the `.sort` site. Each refusal is real; the
	 * lines still execute (a comparator is a plain function), so nothing
	 * throws — the wall is the compiler's.
	 */
	const strings = ["b", "a"]
	// @ts-expect-error — string ordering is deliberately refused: intern ids are meaningless to order (the orderability law)
	strings.sort(by())
	// @ts-expect-error — the descending arm carries the same wall
	strings.sort(desc())
	const numbers = [2, 1]
	// @ts-expect-error — number is not an engine scalar; the roster is bigint | boolean exactly
	numbers.sort(by())
	// @ts-expect-error — the descending arm carries the same wall
	numbers.sort(desc())
	// The positive probes: both roster members instantiate.
	const bigintCmp: (left: bigint, right: bigint) => number = by()
	const booleanCmp: (left: boolean, right: boolean) => number = desc()
	assert.equal(bigintCmp(1n, 2n) < 0, true)
	assert.equal(booleanCmp(false, true) > 0, true)
})

describe("the one-owner agreement: host sort and engine order judgments over the same values", async function agreement() {
	const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-order-"))
	after(function cleanup() {
		fs.rmSync(tmpRoot, { recursive: true, force: true })
	})

	const Score = relation("Score", { id: u64.fresh, n: i64, flag: bool })
	const Theory = schema("OrderAgreement", { Score }, [])
	const db = await Db.create(path.join(tmpRoot, "store"), Theory)

	/** Distinct i64 values across sign and both range extremes. */
	const values = [3n, -(2n ** 63n), 2n ** 63n - 1n, -7n, 0n]
	const seeded = db.write(function seed(tx) {
		for (const n of values) {
			tx.insert(Score, { n, flag: n < 0n })
		}
	})
	assert.ok(seeded.ok, "the seed commit lands")

	const below = query(Theory).rule((r) => {
		const { id, n } = v(Score)
		return r
			.match(Score, { id, n })
			.where(lt(n, r.param("pivot")))
			.find({ id, n })
	})
	const prepared = db.prepare(below)

	test("at every cut, the engine's Lt answer set IS the host-sorted prefix", function everyCut() {
		/**
		 * The agreement, swept at every boundary: `by()`'s host order and
		 * the engine's `Lt` judgment are the same order because both own
		 * arms state the same law (bigint `<` mirrors I64 order). For each
		 * pivot — every inserted value plus both extremes — the engine's
		 * below-pivot answers, host-sorted, must equal the host-sorted
		 * prefix that `by()` puts strictly before the pivot.
		 */
		const sorted = [...values].sort(by())
		const pivots = [...values, -(2n ** 63n), 2n ** 63n - 1n]
		for (const pivot of pivots) {
			const engine = db
				.execute(prepared, { pivot })
				.map(function project(row) {
					return row.n
				})
				.sort(by())
			const host = sorted.filter(function strictlyBelow(value) {
				return by()(value, pivot) < 0
			})
			assert.deepEqual(engine, host, `the cut at ${pivot} agrees`)
		}
	})

	test("desc() is the exact reversal of the engine-agreeing ascending order", function descAgrees() {
		const ascending = [...values].sort(by())
		const descending = [...values].sort(desc())
		assert.deepEqual(descending, [...ascending].reverse())
	})
})
