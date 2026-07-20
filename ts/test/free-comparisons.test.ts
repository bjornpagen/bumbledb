/**
 * The free comparison/connective export pins. Every import here comes from
 * the PACKAGE ROOT — this file IS the export pin for the enumerated names
 * (`eq/ne/lt/le/gt/ge/pointIn/allen/and/or/not`). Pinned: the free
 * spellings judge at the `.where` seam exactly like the method spellings
 * (params inference identical, interval-var-under-lt refused, unbound var
 * refused); and a rule written with the free comparisons lowers to IR
 * identical to its method-spelling twin (one lowering, two entry flavors —
 * also pinned by function IDENTITY: the TermOps methods ARE the free
 * exports). Each `Equal` probe is a value (`const probe: Equal<A, B> =
 * true`), so the compile-time claim carries its own runtime assertion. The
 * schema is LAW-TYPED (rulings 2/3): fields are pure structure and the
 * containment statement is what puts `Holder.id` and `Account.holder` in
 * one class while `Account.id` generates its own.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { QueryParams, QueryRow, TermOps } from "#index.ts"
import {
	ALLEN,
	allen,
	and,
	contained,
	eq,
	ge,
	gt,
	i64,
	interval,
	le,
	lowerQuery,
	lt,
	ne,
	not,
	on,
	or,
	pointIn,
	query,
	relation,
	schema,
	span,
	str,
	u64
} from "#index.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Account = relation("Account", {
	id: u64.fresh,
	holder: u64,
	balance: i64,
	active: interval(u64)
})

/**
 * The laws type the columns: `Account.holder <= Holder.id` puts both slots
 * in the generator class "Holder.id"; `Account.id` is a generator of its
 * own class; `name`/`balance`/`active` are in no law and stay bare.
 */
const Ledger = schema("Ledger", { Holder, Account }, [contained(on(Account, "holder"), on(Holder, "id"))])

describe("the free comparison exports", function suite() {
	test("each enumerated name is a package-root export (the grep pin)", function exportPin() {
		const roster = [eq, ne, lt, le, gt, ge, pointIn, allen, and, or, not]
		for (const builder of roster) {
			assert.equal(typeof builder, "function")
		}
		assert.equal(typeof ALLEN.before, "number")
	})

	test("the TermOps methods ARE the free exports — one lowering, two entry flavors, by identity", function identity() {
		let ops: TermOps | undefined
		query(Ledger).rule((r) => {
			ops = r
			const h = r.var("h")
			return r.match(Holder, { id: h }).select("h")
		})
		assert.ok(ops !== undefined, "the scope was captured")
		assert.equal(ops.eq, eq)
		assert.equal(ops.ne, ne)
		assert.equal(ops.lt, lt)
		assert.equal(ops.le, le)
		assert.equal(ops.gt, gt)
		assert.equal(ops.ge, ge)
		assert.equal(ops.pointIn, pointIn)
		assert.equal(ops.allen, allen)
		assert.equal(ops.and, and)
		assert.equal(ops.or, or)
		assert.equal(ops.not, not)
	})

	test("free lt through .where: params inference identical to the method spelling", function paramsInference() {
		const capped = query(Ledger).rule((r) => {
			const acct = r.var("acct")
			const b = r.var("b")
			return r
				.match(Account, { id: acct, balance: b })
				.where(lt(b, r.param("cap")))
				.select("acct")
		})
		const probeParams: Equal<QueryParams<typeof capped>, { readonly cap: bigint }> = true
		const cappedTwin = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), balance: r.var("b") })
				.where(r.lt(r.var("b"), r.param("cap")))
				.select("acct")
		)
		const probeIdentical: Equal<QueryParams<typeof capped>, QueryParams<typeof cappedTwin>> = true
		const probeRows: Equal<QueryRow<typeof capped>, QueryRow<typeof cappedTwin>> = true
		assert.ok(probeParams && probeIdentical && probeRows)
		assert.deepStrictEqual(lowerQuery(capped), lowerQuery(cappedTwin))
	})

	test("free lt through .where: an interval-typed variable under an order comparison is refused", function intervalWall() {
		const intervalUnderFreeLt = query(Ledger).rule((r) => {
			const acct = r.var("acct")
			const w = r.var("w")
			return (
				r
					.match(Account, { id: acct, active: w })
					// @ts-expect-error — an interval-typed variable has no order; pointIn/allen are the interval predicates
					.where(lt(w, 5n))
					.select("acct")
			)
		})
		assert.equal(intervalUnderFreeLt.data.rules.length, 1)
	})

	test("free lt through .where: an unbound variable is refused (compile + construction)", function unboundWall() {
		assert.throws(function unboundVar() {
			query(Ledger).rule((r) => {
				const h = r.var("h")
				const ghost = r.var("ghost")
				return (
					r
						.match(Holder, { id: h })
						// @ts-expect-error — "ghost" is bound by no relation atom of the rule
						.where(lt(ghost, 5n))
						.select("h")
				)
			})
		}, /the variable ghost is not bound by a relation atom/)
	})

	test("IR-identity golden: a rule written with free comparisons lowers to its method-spelling twin", function irGolden() {
		const viaFree = query(Ledger).rule((r) => {
			const acct = r.var("acct")
			const h = r.var("h")
			const b = r.var("b")
			const w = r.var("w")
			return r
				.match(Account, { id: acct, holder: h, balance: b, active: w })
				.match(Holder, { id: h })
				.where(and(gt(b, 0n), or(lt(b, r.param("cap")), eq(h, r.param("root")))))
				.where(ne(h, 7n))
				.where(ge(b, -5n))
				.where(le(b, 100n))
				.where(pointIn(r.param("at"), w))
				.where(pointIn(3n, w))
				.where(allen(w, ALLEN.before | ALLEN.meets, span(0n, 10n)))
				.where(not(Account, { holder: h, balance: 99n }))
				.select("acct")
		})
		const viaMethods = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), holder: r.var("h"), balance: r.var("b"), active: r.var("w") })
				.match(Holder, { id: r.var("h") })
				.where(r.and(r.gt(r.var("b"), 0n), r.or(r.lt(r.var("b"), r.param("cap")), r.eq(r.var("h"), r.param("root")))))
				.where(r.ne(r.var("h"), 7n))
				.where(r.ge(r.var("b"), -5n))
				.where(r.le(r.var("b"), 100n))
				.where(r.pointIn(r.param("at"), r.var("w")))
				.where(r.pointIn(3n, r.var("w")))
				.where(r.allen(r.var("w"), ALLEN.before | ALLEN.meets, span(0n, 10n)))
				.where(r.not(Account, { holder: r.var("h"), balance: 99n }))
				.select("acct")
		)
		const probeParams: Equal<QueryParams<typeof viaFree>, QueryParams<typeof viaMethods>> = true
		const probeRows: Equal<QueryRow<typeof viaFree>, QueryRow<typeof viaMethods>> = true
		assert.ok(probeParams && probeRows)
		assert.deepStrictEqual(lowerQuery(viaFree), lowerQuery(viaMethods))
	})
})
