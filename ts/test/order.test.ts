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
 */

import assert from "node:assert/strict"
import { test } from "node:test"
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
