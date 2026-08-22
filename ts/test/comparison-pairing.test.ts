/**
 * The comparison PAIRING walls — the type tier judges every comparison's
 * sides AS A PAIR, exactly the engine's same-type rule
 * (`bumbledb/crates/bumbledb/src/ir/validate/context.rs`, `classify`):
 * each side orderable is NOT enough. Pinned here, each `@ts-expect-error`
 * real: bool meets only bool (R3 made bool orderable, never
 * cross-orderable — a bool var against a numeric var or a bigint literal
 * is the engine's conviction), u64 and i64 never meet, the measure's
 * sibling lives in u64 (`OrdMeasureVar`: scalar ≠ U64) and two measures
 * never meet (`DurationBothSides`), `pointIn`'s point lives in the
 * interval's ELEMENT domain, `allen` takes two intervals of ONE element
 * (Q1), and a comparison with no VARIABLE side is constant-valued — a
 * param is a constant at execution — refused at the type tier AND at the
 * constructor (the engine's `ConstantComparison`, `comparison_shape`'s
 * last arm). The legal pairs stay legal: the measure against a u64 var, a
 * bigint literal, a param; bool against bool; open sides (params,
 * integer literals) typed by their siblings.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { TermOps } from "#index.ts"
import { ALLEN, bool, i64, interval, query, relation, schema, u64, v } from "#index.ts"
import { allen, lt, pointIn } from "#query/atom.ts"

const Reading = relation("Reading", {
	id: u64.fresh,
	flag: bool,
	count: u64,
	delta: i64,
	window: interval(u64),
	other: interval(u64),
	phase: interval(i64)
})

const World = schema("World", { Reading }, [])

/** The constant-comparison refusal (the engine's `ConstantComparison` twin). */
const CONSTANT = /a comparison without a variable side is constant-valued/

/** Captures the rule scope's TermOps. */
function capturedOps(): TermOps {
	let ops: TermOps | undefined
	query(World).rule((r) => {
		ops = r
		const { id } = v(Reading)
		return r.match(Reading, { id }).find({ n: id })
	})
	assert.ok(ops !== undefined, "the scope was captured")
	return ops
}

describe("the comparison pairing walls", function suite() {
	test("no variable side is constant-valued: the constructor refuses param-only and param-literal sides", function constantWall() {
		const r = capturedOps()
		assert.throws(function paramLiteral() {
			lt(r.param("lo"), 5n)
		}, CONSTANT)
		assert.throws(function paramParam() {
			lt(r.param("lo"), r.param("hi"))
		}, CONSTANT)
		assert.throws(function pointInParams() {
			pointIn(r.param("t"), r.param("w"))
		}, CONSTANT)
		assert.throws(function allenParams() {
			allen(r.param("a"), ALLEN.before, r.param("b"))
		}, CONSTANT)
	})

	test("the order pair: cross-domain variable pairs are compile-refused (each side alone is orderable)", function orderPairs() {
		const boolAgainstNumeric = query(World).rule((r) => {
			const { id, flag, count } = v(Reading)
			return (
				r
					.match(Reading, { id, flag, count })
					// @ts-expect-error — bool meets only bool: a bool var against a u64 var is the engine's same-type conviction
					.where(r.lt(flag, count))
					.find({ n: id })
			)
		})
		assert.equal(boolAgainstNumeric.data.rules.length, 1)
		const acrossSignedness = query(World).rule((r) => {
			const { id, count, delta } = v(Reading)
			return (
				r
					.match(Reading, { id, count, delta })
					// @ts-expect-error — u64 and i64 never meet under an order operator
					.where(r.lt(count, delta))
					.find({ n: id })
			)
		})
		assert.equal(acrossSignedness.data.rules.length, 1)
	})

	test("the order pair: literals type against their sibling — bool-vs-bigint and u64-vs-boolean are compile-refused", function literalPairs() {
		const boolAgainstInteger = query(World).rule((r) => {
			const { id, flag } = v(Reading)
			return (
				r
					.match(Reading, { id, flag })
					// @ts-expect-error — a bigint literal against a bool var is check_const's conviction (R3 orders bool, it does not number it)
					.where(r.lt(flag, 5n))
					.find({ n: id })
			)
		})
		assert.equal(boolAgainstInteger.data.rules.length, 1)
		const numericAgainstBoolean = query(World).rule((r) => {
			const { id, count } = v(Reading)
			return (
				r
					.match(Reading, { id, count })
					// @ts-expect-error — a boolean literal against a u64 var is the same conviction mirrored
					.where(r.lt(count, true))
					.find({ n: id })
			)
		})
		assert.equal(numericAgainstBoolean.data.rules.length, 1)
	})

	test("the pointIn pair: the point lives in the interval's element domain", function pointInPair() {
		const signedPointInUnsigned = query(World).rule((r) => {
			const { id, delta, window } = v(Reading)
			return (
				r
					.match(Reading, { id, delta, window })
					// @ts-expect-error — an i64-typed point against interval(u64) is IllegalComparison; the element domain is the point's domain
					.where(r.pointIn(delta, window))
					.find({ n: id })
			)
		})
		assert.equal(signedPointInUnsigned.data.rules.length, 1)
	})

	test("the allen pair: two intervals of one element domain (Q1 — widths meet freely, u64-vs-i64 stays illegal)", function allenPair() {
		const crossElement = query(World).rule((r) => {
			const { id, window, phase } = v(Reading)
			return (
				r
					.match(Reading, { id, window, phase })
					// @ts-expect-error — interval(u64) never classifies against interval(i64)
					.where(r.allen(window, ALLEN.before, phase))
					.find({ n: id })
			)
		})
		assert.equal(crossElement.data.rules.length, 1)
	})

	test("the legal pairs stay legal: same-domain vars, sibling-typed opens, one-element allen", function legalPairs() {
		const legal = query(World).rule((r) => {
			const { id, flag, count, window, other } = v(Reading)
			return r
				.match(Reading, { id, flag, count, window, other })
				.where(r.lt(count, r.param("cap")))
				.where(r.lt(flag, true))
				.where(r.pointIn(count, window))
				.where(r.pointIn(5n, window))
				.where(r.pointIn(r.param("t"), window))
				.where(r.allen(window, ALLEN.before | ALLEN.meets, other))
				.find({ n: id })
		})
		assert.equal(legal.data.rules.length, 1)
	})
})
