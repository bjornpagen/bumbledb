/**
 * PRD-K5 probes: `vars()` (the tuple-to-object multi-var mint) and the free
 * comparison/connective exports. Every import here comes from the PACKAGE
 * ROOT — this file IS the export pin for the enumerated names
 * (`eq/ne/lt/le/gt/ge/pointIn/covers/allen/and/or/not`). Pinned: `vars`
 * inference is exact (`Var<"service">`, Equal-strength) and identical to
 * the `r.var` baseline; the record is minted with OWN properties (a
 * `"__proto__"` name never writes the prototype); duplicate names in one
 * call refuse at construction; domain flows from a vars-minted var's first
 * binding and the cross-domain walls hold exactly as for `r.var` (compile
 * AND construction tiers); select rows stay exact through vars-minted
 * names; the free spellings judge at the `.where` seam exactly like the
 * method spellings (params inference identical, interval-var-under-lt
 * refused, unbound var refused); and a rule written entirely with `vars` +
 * free comparisons lowers to IR identical to its `r.var`/`r.lt` twin (one
 * lowering, two entry flavors — also pinned by function IDENTITY: the
 * TermOps methods ARE the free exports). Each `Equal` probe is a value
 * (`const probe: Equal<A, B> = true`), so the compile-time claim carries
 * its own runtime assertion. The schema is LAW-TYPED (rulings 2/3): fields
 * are pure structure and the containment statement is what puts `Holder.id`
 * and `Account.holder` in one class while `Account.id` generates its own —
 * the domain-flow walls here ride K4's class machinery.
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"
import type { QueryParams, QueryRow, TermOps, Var } from "#index.ts"
import {
	ALLEN,
	allen,
	and,
	contained,
	covers,
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

describe("vars(): the tuple-to-object multi-var mint", function suite() {
	test("inference is exact — Var<name> per name, identical to the r.var baseline", function exactness() {
		const q = query(Ledger).rule((r) => {
			const minted = r.vars("service", "w")
			const probeRecord: Equal<typeof minted, { readonly service: Var<"service">; readonly w: Var<"w"> }> = true
			const { service, w } = minted
			const probeService: Equal<typeof service, Var<"service">> = true
			const probeW: Equal<typeof w, Var<"w">> = true
			const baseline = r.var("service")
			const probeBaseline: Equal<typeof service, typeof baseline> = true
			assert.ok(probeRecord && probeService && probeW && probeBaseline)
			return r.match(Holder, { id: service, name: w }).select("service", "w")
		})
		// Select rows stay exact through vars-minted names.
		const probeRow: Equal<QueryRow<typeof q>, { readonly service: bigint; readonly w: string }> = true
		assert.ok(probeRow)
		assert.equal(q.data.rules.length, 1)
	})

	test("domain flows from the first binding; a domain-equal reuse joins (compile + lower)", function domainFlow() {
		const joined = query(Ledger).rule((r) => {
			const { acct, h } = r.vars("acct", "h")
			return r.match(Account, { id: acct, holder: h }).match(Holder, { id: h }).select("acct")
		})
		const probeRow: Equal<QueryRow<typeof joined>, { readonly acct: bigint }> = true
		assert.ok(probeRow)
		assert.equal(lowerQuery(joined).predicates.length, 1, "one plain rule lowers to the single output predicate")
	})

	test("cross-domain reuse of a vars-minted var errors at the use site exactly like r.var", function crossDomain() {
		// The match-join wall: a compile error AND a construction refusal.
		assert.throws(function crossDomainJoin() {
			query(Ledger).rule((r) => {
				const { x } = r.vars("x")
				return (
					r
						.match(Holder, { id: x })
						// @ts-expect-error — "x" first bound in the "Holder.id" class; Account.id generates its own class
						.match(Account, { id: x })
						.select("x")
				)
			})
		}, /joins domain-unequal fields/)

		// The same law through free eq: var-to-var unification is domain-equal.
		const crossDomainEq = query(Ledger).rule((r) => {
			const { a, h } = r.vars("a", "h")
			return (
				r
					.match(Account, { id: a, holder: h })
					// @ts-expect-error — "a" is in the "Account.id" class, "h" is in the "Holder.id" class
					.where(eq(a, h))
					.select("a")
			)
		})
		assert.equal(crossDomainEq.data.rules.length, 1)
	})

	test('every name mints an OWN property — a "__proto__" name never writes the prototype', function protoDiscipline() {
		/**
		 * "__proto__" is a legal variable name, so the record must carry it
		 * as an own key — own-property definition shadows the
		 * object-protocol accessor instead of silently riding it (the
		 * closed-handle probe's law, restated for the vars mint). The
		 * computed access below is deliberate: it is exactly how a host
		 * loops a minted roster.
		 */
		const names = ["h2", "__proto__"] as const
		let minted: { readonly h2: Var<"h2">; readonly __proto__: Var<"__proto__"> } | undefined
		query(Ledger).rule((r) => {
			minted = r.vars(...names)
			const { h } = r.vars("h")
			return r.match(Holder, { id: h }).select("h")
		})
		assert.ok(minted !== undefined, "the record was minted inside the rule")
		for (const name of names) {
			assert.ok(Object.hasOwn(minted, name), `${name} must be an OWN property of the record`)
			assert.equal(minted[name].name, name, "the value at the key is the variable itself, never an accessor no-op")
		}
		assert.equal(Object.getPrototypeOf(minted), Object.prototype, "the record's prototype is untouched")
		assert.ok(Object.isFrozen(minted), "the record is frozen like every minted value")
	})

	test("duplicate names in one call refuse at construction, pointed", function duplicates() {
		assert.throws(function duplicateName() {
			query(Ledger).rule((r) => {
				const bag = r.vars("x", "x")
				return r.match(Holder, { id: bag.x }).select("x")
			})
		}, /duplicate name x — each name mints one variable/)
	})
})

describe("the free comparison exports", function suite() {
	test("each enumerated name is a package-root export (the grep pin)", function exportPin() {
		const roster = [eq, ne, lt, le, gt, ge, pointIn, covers, allen, and, or, not]
		for (const builder of roster) {
			assert.equal(typeof builder, "function")
		}
		assert.equal(typeof ALLEN.before, "number")
	})

	test("the TermOps methods ARE the free exports — one lowering, two entry flavors, by identity", function identity() {
		let ops: TermOps | undefined
		query(Ledger).rule((r) => {
			ops = r
			const { h } = r.vars("h")
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
		assert.equal(ops.covers, covers)
		assert.equal(ops.allen, allen)
		assert.equal(ops.and, and)
		assert.equal(ops.or, or)
		assert.equal(ops.not, not)
	})

	test("free lt through .where: params inference identical to the method spelling", function paramsInference() {
		const capped = query(Ledger).rule((r) => {
			const { acct, b } = r.vars("acct", "b")
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
			const { acct, w } = r.vars("acct", "w")
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
				const { h, ghost } = r.vars("h", "ghost")
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

	test("IR-identity golden: a rule written entirely with vars + free comparisons lowers to its r.var/r.lt twin", function irGolden() {
		const viaVars = query(Ledger).rule((r) => {
			const { acct, h, b, w } = r.vars("acct", "h", "b", "w")
			return r
				.match(Account, { id: acct, holder: h, balance: b, active: w })
				.match(Holder, { id: h })
				.where(and(gt(b, 0n), or(lt(b, r.param("cap")), eq(h, r.param("root")))))
				.where(ne(h, 7n))
				.where(ge(b, -5n))
				.where(le(b, 100n))
				.where(pointIn(r.param("at"), w))
				.where(covers(w, 3n))
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
				.where(r.covers(r.var("w"), 3n))
				.where(r.allen(r.var("w"), ALLEN.before | ALLEN.meets, span(0n, 10n)))
				.where(r.not(Account, { holder: r.var("h"), balance: 99n }))
				.select("acct")
		)
		const probeParams: Equal<QueryParams<typeof viaVars>, QueryParams<typeof viaMethods>> = true
		const probeRows: Equal<QueryRow<typeof viaVars>, QueryRow<typeof viaMethods>> = true
		assert.ok(probeParams && probeRows)
		assert.deepStrictEqual(lowerQuery(viaVars), lowerQuery(viaMethods))
	})
})
