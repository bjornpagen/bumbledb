import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { i64, str, u64 } from "#fields.ts"
import { not } from "#query/atom.ts"
import type { QueryParams, QueryRelation, QueryRow, QueryRuleChain } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import type { Schema, SchemaRelations } from "#schema.ts"
import { schema } from "#schema.ts"
import { contained, key } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: u64, name: str, rank: u64 })
const Account = relation("Account", { id: u64, holder: u64, kind: Kind.id, balance: i64 })
const Parent = relation("Parent", { child: u64, parent: u64 })

const Ledger = schema("Ledger", { Kind, Holder, Account, Parent }, [
	key(Holder, ["id"]),
	contained(on(Account, "holder"), on(Holder, "id")),
	contained(on(Account, "kind"), on(Kind, "id")),
	contained(on(Parent, "child"), on(Holder, "id")),
	contained(on(Parent, "parent"), on(Holder, "id"))
])

type Rels = (typeof Ledger)["relations"]
type Classes = (typeof Ledger)["classes"]

const KIND_ID = 0
const ACCOUNT_ID = 2

function countQueryOf<Rels extends SchemaRelations, R extends QueryRelation<Rels>>(theory: Schema<Rels>, rel: R) {
	return query(theory).rule((r) => r.match(rel, v(rel)).find({ n: r.count() }))
}

describe("the generic full-binding law", function suite() {
	test("the Primer-shape generic helper compiles and lowers to the concrete spelling's IR", function primerShape() {
		const counted = countQueryOf(Ledger, Account)
		type RowPin = Expect<Equal<QueryRow<typeof counted>, { readonly n: bigint }>>
		type ParamsPin = Expect<Equal<keyof QueryParams<typeof counted>, never>>
		const direct = query(Ledger).rule((r) => r.match(Account, v(Account)).find({ n: r.count() }))
		assert.deepStrictEqual(
			lowerQuery(counted),
			lowerQuery(direct),
			"the generic instantiation IS the concrete spelling — one lowering"
		)

		assert.deepStrictEqual(lowerQuery(counted), {
			kind: "cq",
			interiors: [],
			head: [{ kind: "aggregate", op: "count" }],
			rules: [
				{
					finds: [{ kind: "count" }],
					atoms: [
						{
							source: { kind: "edb", relation: ACCOUNT_ID },
							bindings: [
								[0, { kind: "var", var: 0 }],
								[1, { kind: "var", var: 1 }],
								[2, { kind: "var", var: 2 }],
								[3, { kind: "var", var: 3 }]
							]
						}
					],
					negated: [],
					conditions: []
				}
			]
		})
		const pins: [RowPin, ParamsPin] = [true, true]
		assert.equal(pins.length, 2)
	})

	test("QueryRelation includes closed owners: the generic helper instantiates over a vocabulary", function closedOwner() {
		const kinds = countQueryOf(Ledger, Kind)
		type RowPin = Expect<Equal<QueryRow<typeof kinds>, { readonly n: bigint }>>

		assert.deepStrictEqual(lowerQuery(kinds), {
			kind: "cq",
			interiors: [],
			head: [{ kind: "aggregate", op: "count" }],
			rules: [
				{
					finds: [{ kind: "count" }],
					atoms: [{ source: { kind: "edb", relation: KIND_ID }, bindings: [[0, { kind: "var", var: 0 }]] }],
					negated: [],
					conditions: []
				}
			]
		})
		const pin: RowPin = true
		assert.ok(pin)
	})

	test("the concrete full binding infers the paramless chain — scope form and chain form", function paramlessChain() {
		const full = query(Ledger).rule((r) => {
			const a = v(Account)
			const opened = r.match(Account, a)
			type ScopePin = Expect<Equal<typeof opened, QueryRuleChain<Rels, Record<never, never>, Classes>>>
			const chained = opened.match(Parent, v(Parent))
			type ChainPin = Expect<Equal<typeof chained, QueryRuleChain<Rels, Record<never, never>, Classes>>>
			const pins: [ScopePin, ChainPin] = [true, true]
			assert.equal(pins.length, 2)
			return chained.find({ holder: a.holder, n: r.count() })
		})
		type RowPin = Expect<Equal<QueryRow<typeof full>, { readonly holder: bigint; readonly n: bigint }>>
		type ParamsPin = Expect<Equal<keyof QueryParams<typeof full>, never>>
		const lowered = lowerQuery(full)
		assert.equal(lowered.kind, "cq")
		const rule = lowered.rules[0]
		assert.ok(rule !== undefined, "the one rule lowered")
		assert.equal(rule.atoms.length, 2, "the full binding and the second full binding are two positive atoms")
		const pins: [RowPin, ParamsPin] = [true, true]
		assert.equal(pins.length, 2)
	})

	test("the chain form passes P through unchanged — a param survives a later full binding", function paramsRideThrough() {
		const withParam = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			const opened = r.match(Holder, { id: h, name: r.param("who") })
			const after = opened.match(Parent, v(Parent))
			type PPin = Expect<Equal<typeof after, QueryRuleChain<Rels, { readonly who: string }, Classes>>>
			const pin: PPin = true
			assert.ok(pin)
			return after.find({ h })
		})
		type ParamsPin = Expect<Equal<QueryParams<typeof withParam>, { readonly who: string }>>
		const pin: ParamsPin = true
		assert.ok(pin)
	})

	test("the cross-class walls still refuse on the general path — the added form widened nothing", function wallsHold() {
		assert.throws(function crossClassJoin() {
			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				return (
					r
						.match(Holder, { id: h })
						// @ts-expect-error — h minted in the "Holder.id" class; Account.id generates "Account.id"
						.match(Account, { id: h })
						.find({ h })
				)
			})
		}, /joins domain-unequal fields/)

		assert.throws(function bareClassedWall() {
			query(Ledger).rule((r) => {
				const { rank: z } = v(Holder)
				return (
					r
						.match(Holder, { rank: z })
						// @ts-expect-error — bare pairs only with bare; "Holder.id" is a classed slot
						.match(Account, { holder: z })
						.find({ z })
				)
			})
		}, /joins domain-unequal fields/)
	})

	test("a partial record with a param still infers its params through the general form", function partialParams() {
		const filtered = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r
				.match(Holder, { id: h, name: r.param("who") })
				.match(Account, { holder: h, balance: r.param("floor") })
				.find({ h })
		})
		type ParamsPin = Expect<Equal<QueryParams<typeof filtered>, { readonly who: string; readonly floor: bigint }>>
		type RowPin = Expect<Equal<QueryRow<typeof filtered>, { readonly h: bigint }>>
		assert.equal(filtered.data.params.length, 2, "both params registered in first-use order")
		const pins: [ParamsPin, RowPin] = [true, true]
		assert.equal(pins.length, 2)
	})

	test("an aliased extra-key record is refused at every full-binding site — ExactVars restores the pre-0.16.0 exactness", function aliasedExtraKey() {
		// general form refused it (CheckBindings → the unknown-field arm).

		// refused at compile time AND by the construction twin (`relation R

		const accountExtras = { ...v(Account), extra: v(Holder).id }
		const holderExtras = { ...v(Holder), extra: v(Account).id }

		assert.throws(function scopeSite() {
			query(Ledger).rule((r) => {
				// @ts-expect-error — extra is not a column of Account; the aliased record falls to the general form
				return r.match(Account, accountExtras).find({ n: r.count() })
			})
		}, /relation Account has no field extra/)

		assert.throws(function chainSite() {
			query(Ledger).rule((r) => {
				// @ts-expect-error — extra is not a column of Holder; the aliased record falls to the general form
				return r.match(Account, v(Account)).match(Holder, holderExtras).find({ n: r.count() })
			})
		}, /relation Holder has no field extra/)

		assert.throws(function interiorScopeSite() {
			// @ts-expect-error — extra is not a column of Account; the aliased record falls to the general form
			query(Ledger).interior("mid", (r) => r.match(Account, accountExtras).find({ h: accountExtras.holder }))
		}, /relation Account has no field extra/)

		assert.throws(function interiorChainSite() {
			query(Ledger).interior("mid", (r) => {
				const a = v(Account)
				// @ts-expect-error — extra is not a column of Holder; the aliased record falls to the general form
				return r.match(Account, a).match(Holder, holderExtras).find({ h: a.holder })
			})
		}, /relation Holder has no field extra/)

		assert.throws(function recScopeSite() {
			query(Ledger).reach("reach", {
				base: [
					(r) => {
						const { id: c } = v(Holder)
						return r.match(Holder, { id: c }).find({ c })
					}
				],
				rec: [
					(r) => {
						// @ts-expect-error — extra is not a column of Holder; the aliased record falls to the general form
						return r.match(Holder, holderExtras).find({ c: holderExtras.id })
					}
				]
			})
		}, /relation Holder has no field extra/)

		assert.throws(function recChainSite() {
			query(Ledger).reach("reach", {
				base: [
					(r) => {
						const { id: c } = v(Holder)
						return r.match(Holder, { id: c }).find({ c })
					}
				],
				rec: [
					(r) => {
						const p = v(Parent)
						// @ts-expect-error — extra is not a column of Holder; the aliased record falls to the general form
						return r.match(Parent, p).match(Holder, holderExtras).find({ c: p.parent })
					}
				]
			})
		}, /relation Holder has no field extra/)
	})

	test("r.match(A, v(B)) is refused — concrete owners", function crossOwnerConcrete() {
		// where the first shared field (id) is a cross-class reuse — refused at

		assert.throws(function fullForeignRecord() {
			query(Ledger).rule((r) => {
				// @ts-expect-error — v(Holder) is Holder's full binding, never Account's
				return r.match(Account, v(Holder)).find({ n: r.count() })
			})
		}, /joins domain-unequal fields/)
	})

	test("r.match(A, v(B)) is refused — generic owners", function crossOwnerGeneric() {
		// it as they always have. Never called: the pin is the compile refusal.
		function fullForeignRecord<Rels extends SchemaRelations, R extends QueryRelation<Rels>>(
			theory: Schema<Rels>,
			rel: R
		) {
			return query(theory).rule((r) => {
				// @ts-expect-error — a generic owner R admits only VarsOf<R>; v(Holder) is a foreign full binding
				return r.match(rel, v(Holder)).find({ n: r.count() })
			})
		}
		void fullForeignRecord
		assert.ok(true, "the generic cross-owner refusal is a compile-time pin")
	})

	test("a wrong concrete record still errors at the offending field", function misspelledField() {
		assert.throws(function misspelled() {
			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				// @ts-expect-error — holderz is not a field of Account; the general path names the property
				return r.match(Account, { holderz: h }).find({ h })
			})
		}, /relation Account has no field holderz/)
	})

	test("the DELIBERATE exclusion: not() gains no full-binding form", function notExcluded() {
		// rule; a full-fresh-var negation is a boundness refusal at

		// recorded ruling). Never called: the pin is the compile refusal.
		function negatedFullBinding<Rels extends SchemaRelations, R extends QueryRelation<Rels>>(rel: R) {
			// @ts-expect-error — not() keeps the general form only; VarsOf<R> stays unprovable there for generic R
			return not(rel, v(rel))
		}
		void negatedFullBinding
		assert.ok(true, "the exclusion is a compile-time pin")
	})
})
