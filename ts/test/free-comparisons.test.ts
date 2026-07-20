/**
 * The free comparison/connective export pins. Every import here comes from
 * the PACKAGE ROOT — this file IS the export pin for the enumerated names
 * (`eq/ne/lt/le/gt/ge/pointIn/allen/and/or/not`, and now the load-bearing
 * mint `v`). Pinned: the free spellings judge at the `.where` seam exactly
 * like the method spellings (params inference identical, interval-var-under-lt
 * refused, unbound var refused at construction); and a rule written with the
 * free comparisons lowers to IR identical to its method-spelling twin (one
 * lowering, two entry flavors — also pinned by function IDENTITY: the TermOps
 * methods ARE the free exports). Each `Equal` probe is a value (`const probe:
 * Equal<A, B> = true`), so the compile-time claim carries its own runtime
 * assertion. Variables are minted by the free `v()` and joined by OBJECT
 * REFERENCE (reuse is the join); the head is a `find` RECORD whose keys name
 * the answer columns. The schema is LAW-TYPED (rulings 2/3): fields are pure
 * structure and the containment statement is what puts `Holder.id` and
 * `Account.holder` in one class while `Account.id` generates its own.
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
	u64,
	v
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
		const roster = [eq, ne, lt, le, gt, ge, pointIn, allen, and, or, not, v]
		for (const builder of roster) {
			assert.equal(typeof builder, "function")
		}
		assert.equal(v.name, "v", "v is the free variable-record mint export")
		assert.equal(typeof ALLEN.before, "number")
	})

	test("the TermOps methods ARE the free exports — one lowering, two entry flavors, by identity", function identity() {
		let ops: TermOps | undefined
		query(Ledger).rule((r) => {
			ops = r
			const h = v(Holder)
			return r.match(Holder, { id: h.id }).find({ h: h.id })
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
			const { id, balance } = v(Account)
			return r
				.match(Account, { id, balance })
				.where(lt(balance, r.param("cap")))
				.find({ acct: id })
		})
		const probeParams: Equal<QueryParams<typeof capped>, { readonly cap: bigint }> = true
		const cappedTwin = query(Ledger).rule((r) => {
			const { id, balance } = v(Account)
			return r
				.match(Account, { id, balance })
				.where(r.lt(balance, r.param("cap")))
				.find({ acct: id })
		})
		const probeIdentical: Equal<QueryParams<typeof capped>, QueryParams<typeof cappedTwin>> = true
		const probeRows: Equal<QueryRow<typeof capped>, QueryRow<typeof cappedTwin>> = true
		assert.ok(probeParams && probeIdentical && probeRows)
		assert.deepStrictEqual(lowerQuery(capped), lowerQuery(cappedTwin))
	})

	test("free lt through .where: an interval-typed variable under an order comparison is refused", function intervalWall() {
		const intervalUnderFreeLt = query(Ledger).rule((r) => {
			const { id, active } = v(Account)
			return (
				r
					.match(Account, { id, active })
					// @ts-expect-error — an interval-typed variable has no order; pointIn/allen are the interval predicates
					.where(lt(active, 5n))
					.find({ acct: id })
			)
		})
		assert.equal(intervalUnderFreeLt.data.rules.length, 1)
	})

	test("free lt through .where: an unbound variable is refused at construction (object identity is invisible to types — the boundness wall is runtime)", function unboundWall() {
		// The scope.ts design theorem: TypeScript types cannot see object
		// identity, so boundness (is this var positively bound by a relation
		// atom of the rule) cannot be a compile pin — it moves to a
		// construction-time wall alone. The old @ts-expect-error half dies with
		// the name-keyed env; only the runtime throw survives.
		assert.throws(function unboundVar() {
			query(Ledger).rule((r) => {
				const h = v(Holder)
				const ghost = v(Holder)
				return r
					.match(Holder, { id: h.id })
					.where(lt(ghost.id, 5n))
					.find({ h: h.id })
			})
		}, /the variable Holder\.id is not bound by a relation atom/)
	})

	test("IR-identity golden: a rule written with free comparisons lowers to its method-spelling twin", function irGolden() {
		const viaFree = query(Ledger).rule((r) => {
			const { id, holder, balance, active } = v(Account)
			return r
				.match(Account, { id, holder, balance, active })
				.match(Holder, { id: holder })
				.where(and(gt(balance, 0n), or(lt(balance, r.param("cap")), eq(holder, r.param("root")))))
				.where(ne(holder, 7n))
				.where(ge(balance, -5n))
				.where(le(balance, 100n))
				.where(pointIn(r.param("at"), active))
				.where(pointIn(3n, active))
				.where(allen(active, ALLEN.before | ALLEN.meets, span(0n, 10n)))
				.where(not(Account, { holder, balance: 99n }))
				.find({ acct: id })
		})
		const viaMethods = query(Ledger).rule((r) => {
			const { id, holder, balance, active } = v(Account)
			return r
				.match(Account, { id, holder, balance, active })
				.match(Holder, { id: holder })
				.where(r.and(r.gt(balance, 0n), r.or(r.lt(balance, r.param("cap")), r.eq(holder, r.param("root")))))
				.where(r.ne(holder, 7n))
				.where(r.ge(balance, -5n))
				.where(r.le(balance, 100n))
				.where(r.pointIn(r.param("at"), active))
				.where(r.pointIn(3n, active))
				.where(r.allen(active, ALLEN.before | ALLEN.meets, span(0n, 10n)))
				.where(r.not(Account, { holder, balance: 99n }))
				.find({ acct: id })
		})
		const probeParams: Equal<QueryParams<typeof viaFree>, QueryParams<typeof viaMethods>> = true
		const probeRows: Equal<QueryRow<typeof viaFree>, QueryRow<typeof viaMethods>> = true
		assert.ok(probeParams && probeRows)
		assert.deepStrictEqual(lowerQuery(viaFree), lowerQuery(viaMethods))
	})
})
