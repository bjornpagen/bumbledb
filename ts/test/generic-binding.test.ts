/**
 * The generic full-binding law (V7; proposals/one-representation/
 * 50-generic-binding.md): `v(R)` IS the full binding of `R`, and the six
 * `match` sites now STATE it as a signature — `bindings: VarsOf<R>` unifies
 * by identity for generic `R`, where the general form's three deferred
 * conditionals (Probe A/B's gravestone) cannot be related. Pinned here, each
 * probe a value: (1) the Primer shape — a schema-generic per-relation count
 * helper over the SDK's own `QueryRelation<Rels>` bound — compiles with ZERO
 * suppressions and lowers to IR deeply equal to its concrete spelling;
 * (2) the concrete full binding infers the PARAMLESS chain on scope and
 * chain forms alike (an all-var record contributes no params; a chain's `P`
 * rides through unchanged); (3) the cross-class walls still refuse on the
 * general path (the added form widened nothing — every `@ts-expect-error`
 * real, every construction twin thrown); (4) a partial record with a param
 * still infers its params through the general form; (5) `r.match(A, v(B))`
 * is refused for concrete AND generic owners (the `owner`/`column` literals
 * fail `VarsOf<R>` structurally, and the general path's judgment stands
 * behind); (6) a misspelled concrete field still errors at the position and
 * at construction; (7) an aliased extra-key record (`{ ...v(R), extra }` —
 * a shape excess-property checking never sees) is refused at EVERY
 * full-binding site through `ExactVars` and falls to the general form's
 * judgment, compile refusal and construction twin alike; and the
 * DELIBERATE exclusion — `not()` gains no
 * full-binding form (a full-fresh-var negation is a boundness refusal at
 * construction; blessing the spelling generically would type-admit a
 * guaranteed construction error) — stays a compile refusal. Lowering-only:
 * `lowerQuery` is a pure function of the query value, so no store and no
 * native module are needed.
 */

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
import { contained } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

/**
 * THE LAWS TYPE THE COLUMNS: the containments below put `Account.holder`,
 * `Parent.child`, and `Parent.parent` in the `"Holder.id"` generator class
 * and `Account.kind` in `"Kind.id"`, while `Holder.id` and `Account.id`
 * generate their own — the cross-class walls' slots.
 */
const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: u64.fresh, name: str, rank: u64 })
const Account = relation("Account", { id: u64.fresh, holder: u64, kind: Kind.id, balance: i64 })
const Parent = relation("Parent", { child: u64, parent: u64 })

const Ledger = schema("Ledger", { Kind, Holder, Account, Parent }, [
	contained(on(Account, "holder"), on(Holder, "id")),
	contained(on(Account, "kind"), on(Kind, "id")),
	contained(on(Parent, "child"), on(Holder, "id")),
	contained(on(Parent, "parent"), on(Holder, "id"))
])

type Rels = (typeof Ledger)["relations"]
type Classes = (typeof Ledger)["classes"]

/** Relation ids = record declaration order (the law `lowerQuery` rides). */
const KIND_ID = 0
const ACCOUNT_ID = 2

/**
 * The Primer shape (50-generic-binding.md, pinned type test 1): a
 * schema-generic per-relation count query — `r.match(rel, v(rel))` through
 * the full-binding form, over the SDK's own `QueryRelation<Rels>` bound
 * (the one spelling of "a relation this schema can match"; the hand-rolled
 * `Extract<Rels[keyof Rels], AnyRelation>` omits closed owners). ZERO casts,
 * ZERO suppressions.
 */
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
		// The IR shape, pinned exactly: one edb atom over Account, every column
		// bound to its own fresh var in sealed order (the identity atom), a
		// nullary count head.
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
		// A closed owner's sealed shape is the synthetic id alone (list form):
		// the full binding is one var at ordinal 0, a ψ atom through the same
		// edb source.
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
		// A "Holder.id"-class mint reused at the "Account.id" generator class:
		// a partial record fails VarsOf<R> structurally, falls to the general
		// form, and CheckBindings judges exactly as before — a compile error
		// AND the construction twin.
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

		// Bare↔classed refuses through the general path too: Holder.rank is in
		// no law, Account.holder is in "Holder.id".
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
		// Excess-property checking covers only inline literals, so an aliased
		// or function-returned `{ ...v(R), extra: otherVar }` record used to
		// match the full-binding form structurally while the pre-0.16.0
		// general form refused it (CheckBindings → the unknown-field arm).
		// ExactVars (scope.ts) maps a foreign key to a variable type whose
		// mint column is `never`, so the record fails the full-binding
		// intersection at EVERY site, falls to the general form, and is
		// refused at compile time AND by the construction twin (`relation R
		// has no field extra`).
		const accountExtras = { ...v(Account), extra: v(Holder).id }
		const holderExtras = { ...v(Holder), extra: v(Account).id }

		// Site 1 — QueryRuleScope.match (the rule's first atom).
		assert.throws(function scopeSite() {
			query(Ledger).rule((r) => {
				// @ts-expect-error — extra is not a column of Account; the aliased record falls to the general form
				return r.match(Account, accountExtras).find({ n: r.count() })
			})
		}, /relation Account has no field extra/)

		// Site 2 — QueryRuleChain.match (a later atom).
		assert.throws(function chainSite() {
			query(Ledger).rule((r) => {
				// @ts-expect-error — extra is not a column of Holder; the aliased record falls to the general form
				return r.match(Account, v(Account)).match(Holder, holderExtras).find({ n: r.count() })
			})
		}, /relation Holder has no field extra/)

		// Site 3 — InteriorRuleScope.match.
		assert.throws(function interiorScopeSite() {
			// @ts-expect-error — extra is not a column of Account; the aliased record falls to the general form
			query(Ledger).interior("mid", (r) => r.match(Account, accountExtras).find({ h: accountExtras.holder }))
		}, /relation Account has no field extra/)

		// Site 4 — InteriorRuleChain.match.
		assert.throws(function interiorChainSite() {
			query(Ledger).interior("mid", (r) => {
				const a = v(Account)
				// @ts-expect-error — extra is not a column of Holder; the aliased record falls to the general form
				return r.match(Account, a).match(Holder, holderExtras).find({ h: a.holder })
			})
		}, /relation Holder has no field extra/)

		// Site 5 — RecRuleScope.match (a rec arm's first atom).
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

		// Site 6 — RecRuleChain.match (a later atom on a rec arm).
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
		// v(Holder)'s vars carry owner/column mint literals, so the record fails
		// VarsOf<typeof Account> structurally and falls to the general form,
		// where the first shared field (id) is a cross-class reuse — refused at
		// the position (compile) and at construction (the runtime twin).
		assert.throws(function fullForeignRecord() {
			query(Ledger).rule((r) => {
				// @ts-expect-error — v(Holder) is Holder's full binding, never Account's
				return r.match(Account, v(Holder)).find({ n: r.count() })
			})
		}, /joins domain-unequal fields/)
	})

	test("r.match(A, v(B)) is refused — generic owners", function crossOwnerGeneric() {
		// For a generic owner R the foreign record cannot unify with VarsOf<R>
		// (identity fails), and the general form's deferred conditionals refuse
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
		// A negated atom's variables must be positively bound elsewhere in the
		// rule; a full-fresh-var negation is a boundness refusal at
		// construction, so a signature blessing the spelling generically would
		// type-admit a guaranteed construction error (50-generic-binding.md's
		// recorded ruling). Never called: the pin is the compile refusal.
		function negatedFullBinding<Rels extends SchemaRelations, R extends QueryRelation<Rels>>(rel: R) {
			// @ts-expect-error — not() keeps the general form only; VarsOf<R> stays unprovable there for generic R
			return not(rel, v(rel))
		}
		void negatedFullBinding
		assert.ok(true, "the exclusion is a compile-time pin")
	})
})
