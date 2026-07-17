/**
 * PRD-08 query-surface pins against a REAL durable store, on the
 * zero-closable surface: a join with a param, a negation, a union of two
 * rules (set semantics dedup), `count()` with implicit grouping, the
 * recursive closure query from bumbledb's cookbook (recipe 24's
 * engine-native form), point-membership and `allen` over an interval
 * field, a set param, negation-safety refusal naming the variable,
 * deterministic lowering (same query built twice → deeply-equal IR), and
 * the prepared VALUE: execution only through `snap.execute`/`db.execute`
 * (the symmetry rule — `prepared.execute` does not exist), `staleness`
 * against a live read scope, no lifecycle spelling anywhere, and typed
 * refusals for missing params, invalidated scopes, and cross-store use.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, describe, test } from "node:test"

import type {
	Brand,
	Db as DbValue,
	Infer,
	ParamsRecord,
	Prepared,
	QueryBuild,
	QueryRow,
	ReadScope,
	Scope
} from "#index.ts"
import {
	ALLEN,
	allen,
	closed,
	contained,
	count,
	Db,
	i64,
	interval,
	is,
	lowerQuery,
	match,
	not,
	on,
	oneOf,
	query,
	relation,
	schema,
	span,
	str,
	u64
} from "#index.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-query-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const HolderId = u64.newtype("HolderId")
const AccountId = u64.newtype("AccountId")

const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: HolderId.fresh, name: str })
const Account = relation("Account", {
	id: AccountId.fresh,
	holder: HolderId,
	kind: Kind.id,
	balance: i64,
	active: interval(u64)
})
const Parent = relation("Parent", {
	child: HolderId,
	parent: HolderId
})

const Ledger = schema("Ledger", { Kind, Holder, Account, Parent }, [contained(on(Account, "holder"), on(Holder, "id"))])

type HolderId = Infer<typeof HolderId>

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

/** Sorts a bigint array ascending (answers are sets; the host sorts). */
function sorted(values: readonly bigint[]): bigint[] {
	return [...values].sort(function compare(a, b) {
		if (a < b) {
			return -1
		}
		if (a > b) {
			return 1
		}
		return 0
	})
}

/** The seeded ids the tests read. */
const ids: {
	ada?: HolderId
	grace?: HolderId
	kurt?: HolderId
	lone?: HolderId
	adaChecking?: Brand<bigint, "AccountId">
	adaSavings?: Brand<bigint, "AccountId">
	graceSavings?: Brand<bigint, "AccountId">
} = {}

describe("the query surface against a real store", function suite() {
	let db: DbValue<(typeof Ledger)["relations"]>

	before(async function seed() {
		db = await Db.create(storeDir, Ledger)
		const seeded = db.write(function seedRows(tx) {
			const ada = tx.insert(Holder, { name: "ada" })
			const grace = tx.insert(Holder, { name: "grace" })
			const kurt = tx.insert(Holder, { name: "kurt" })
			const lone = tx.insert(Holder, { name: "lone" })
			ids.ada = ada.id
			ids.grace = grace.id
			ids.kurt = kurt.id
			ids.lone = lone.id
			const adaChecking = tx.insert(Account, {
				holder: ada.id,
				kind: Kind.Checking,
				balance: 5n,
				active: span(0n, 10n)
			})
			const adaSavings = tx.insert(Account, {
				holder: ada.id,
				kind: Kind.Savings,
				balance: 7n,
				active: span(20n, 30n)
			})
			ids.adaChecking = adaChecking.id
			ids.adaSavings = adaSavings.id
			const graceSavings = tx.insert(Account, {
				holder: grace.id,
				kind: Kind.Savings,
				balance: 3n,
				active: span(5n, 15n)
			})
			ids.graceSavings = graceSavings.id
			tx.insert(Account, {
				holder: kurt.id,
				kind: Kind.Checking,
				balance: 9n,
				active: span(40n, 50n)
			})
			tx.insert(Parent, { child: grace.id, parent: ada.id })
			tx.insert(Parent, { child: kurt.id, parent: grace.id })
		})
		assert.ok(seeded.ok, "the seed commit lands")
	})

	test("a join with a param returns the branded answer set", function joinWithParam() {
		const accountsOf = query(Ledger, function build($: Scope<(typeof Ledger)["relations"]>) {
			const acct = $.var(Account.fields.id)
			const holder = $.var(Holder.fields.id)
			const root = $.param("root", Holder.fields.id)
			return {
				rules: [[match(Account, { id: acct, holder }), match(Holder, { id: holder }), is(holder, root)]],
				select: { acct, holder }
			}
		})
		const prepared = db.prepare(accountsOf)
		const rows = db.execute(prepared, { root: must(ids.ada) })
		assert.equal(rows.length, 2)
		for (const row of rows) {
			assert.equal(row.holder, ids.ada)
			assert.equal(typeof row.acct, "bigint")
		}
		assert.deepEqual(
			sorted(
				rows.map(function acct(row) {
					return row.acct
				})
			),
			sorted([must(ids.adaChecking), must(ids.adaSavings)])
		)
		const empty = db.execute(prepared, { root: must(ids.lone) })
		assert.deepEqual(empty, [])
	})

	test("execution obeys the symmetry rule db.execute(p, params) === db.read(snap => snap.execute(p, params))", function executeSymmetry() {
		const holdersByName = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			const name = $.param("name", Holder.fields.name)
			return { rules: [[match(Holder, { id: h, name })]], select: { h } }
		})
		const prepared = db.prepare(holdersByName)
		assert.deepStrictEqual(
			db.execute(prepared, { name: "ada" }),
			db.read(function executeInScope(snap) {
				return snap.execute(prepared, { name: "ada" })
			})
		)
	})

	test("negation is a safe anti-join", function negation() {
		const holdersWithoutAccounts = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			return {
				rules: [[match(Holder, { id: h }), not(match(Account, { holder: h }))]],
				select: { h }
			}
		})
		const rows = db.execute(db.prepare(holdersWithoutAccounts), {})
		assert.deepEqual(
			rows.map(function h(row) {
				return row.h
			}),
			[must(ids.lone)]
		)
	})

	test("a union of two rules deduplicates under set semantics", function union() {
		const adaOrSavings = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			return {
				rules: [
					[match(Holder, { id: h, name: "ada" })],
					[match(Holder, { id: h }), match(Account, { holder: h, kind: Kind.Savings })]
				],
				select: { h }
			}
		})
		const rows = db.execute(db.prepare(adaOrSavings), {})
		assert.deepEqual(
			sorted(
				rows.map(function h(row) {
					return row.h
				})
			),
			sorted([must(ids.ada), must(ids.grace)]),
			"ada matches both rules and lands once — the union is a set"
		)
	})

	test("count() groups implicitly by the non-aggregate select entries", function counting() {
		const accountsPerHolder = query(Ledger, function build($) {
			const acct = $.var(Account.fields.id)
			const holder = $.var(Holder.fields.id)
			return {
				rules: [[match(Account, { id: acct, holder })]],
				select: { holder, n: count() }
			}
		})
		const rows = db.execute(db.prepare(accountsPerHolder), {})
		const byHolder = new Map(
			rows.map(function entry(row) {
				return [row.holder, row.n] as const
			})
		)
		assert.equal(byHolder.size, 3, "lone has no account and no group — never a zero row")
		assert.equal(byHolder.get(must(ids.ada)), 2n)
		assert.equal(byHolder.get(must(ids.grace)), 1n)
		assert.equal(byHolder.get(must(ids.kurt)), 1n)
	})

	test("the cookbook closure query runs as one stratified program", function closure() {
		const reachable = query(Ledger, function build($) {
			const c = $.var(Holder.fields.id)
			const child = $.var(Holder.fields.id)
			const m = $.var(Holder.fields.id)
			const root = $.param("root", Holder.fields.id)
			const reach = $.predicate("reach", { c: Holder.fields.id }, function rules(self) {
				return [
					{ finds: { c }, body: [match(Holder, { id: c }), is(c, root)] },
					{
						finds: { c: child },
						body: [match(Parent, { child, parent: m }), self.match({ c: m })]
					}
				]
			})
			return { rules: [[reach.match({ c })]], select: { c } }
		})
		const prepared = db.prepare(reachable)
		const answers = db.read(function readClosure(snap) {
			return {
				fromAda: snap.execute(prepared, { root: must(ids.ada) }),
				fromGrace: snap.execute(prepared, { root: must(ids.grace) }),
				staleness: prepared.staleness(snap)
			}
		})
		assert.deepEqual(
			sorted(
				answers.fromAda.map(function c(row) {
					return row.c
				})
			),
			sorted([must(ids.ada), must(ids.grace), must(ids.kurt)]),
			"ada → grace → kurt closes; lone stays out"
		)
		assert.deepEqual(
			sorted(
				answers.fromGrace.map(function c(row) {
					return row.c
				})
			),
			sorted([must(ids.grace), must(ids.kurt)])
		)
		assert.equal(typeof answers.staleness.maxRatio, "number")
	})

	test("point membership, allen, oneOf, and a set param", function intervalAndSets() {
		const activeAtFive = query(Ledger, function build($) {
			const acct = $.var(Account.fields.id)
			return { rules: [[match(Account, { id: acct, active: 5n })]], select: { acct } }
		})
		const active = db.execute(db.prepare(activeAtFive), {})
		assert.deepEqual(
			sorted(
				active.map(function acct(row) {
					return row.acct
				})
			),
			sorted([must(ids.adaChecking), must(ids.graceSavings)]),
			"ada's checking [0,10) and grace's [5,15) cover the point 5"
		)

		const intersecting = query(Ledger, function build($) {
			const acct = $.var(Account.fields.id)
			const during = $.var(Account.fields.active)
			return {
				rules: [[match(Account, { id: acct, active: during }), allen(during, ALLEN.intersects, span(0n, 12n))]],
				select: { acct }
			}
		})
		const overlapping = db.execute(db.prepare(intersecting), {})
		assert.deepEqual(
			sorted(
				overlapping.map(function acct(row) {
					return row.acct
				})
			),
			sorted([must(ids.adaChecking), must(ids.graceSavings)]),
			"[0,10) and [5,15) intersect [0,12); [20,30) and [40,50) are disjoint from it"
		)

		const namedSet = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			const names = $.paramSet("names", Holder.fields.name)
			return { rules: [[match(Holder, { id: h, name: names })]], select: { h } }
		})
		const preparedNames = db.prepare(namedSet)
		const named = db.execute(preparedNames, { names: ["ada", "kurt"] })
		assert.deepEqual(
			sorted(
				named.map(function h(row) {
					return row.h
				})
			),
			sorted([must(ids.ada), must(ids.kurt)])
		)
		assert.deepEqual(db.execute(preparedNames, { names: [] }), [], "the empty set matches nothing")

		const eitherKind = query(Ledger, function build($) {
			const acct = $.var(Account.fields.id)
			return {
				rules: [[match(Account, { id: acct, kind: oneOf(Kind.Checking, Kind.Savings) })]],
				select: { acct }
			}
		})
		assert.equal(db.execute(db.prepare(eitherKind), {}).length, 4, "the oneOf disjunction spans both kinds")
	})

	test("negation safety violation is a construction error naming the variable", function unsafeNegation() {
		assert.throws(function buildUnsafe() {
			query(Ledger, function build($) {
				const h = $.var(Holder.fields.id)
				const ghost = $.var(Account.fields.id)
				return {
					rules: [[match(Holder, { id: h }), not(match(Account, { id: ghost, holder: h }))]],
					select: { h }
				}
			})
		}, /Account\.id/)
	})

	test("oneOf inside not() is a construction error naming the field", function negatedOneOf() {
		assert.throws(function buildNegatedOneOf() {
			not(match(Account, { kind: oneOf(Kind.Checking, Kind.Savings) }))
		}, /negated Account atom binds kind with oneOf/)
	})

	test("the same query built twice lowers to deeply-equal IR", function determinism() {
		function build($: Scope<(typeof Ledger)["relations"]>): QueryBuild {
			const acct = $.var(Account.fields.id)
			const holder = $.var(Holder.fields.id)
			const root = $.param("root", Holder.fields.id)
			return {
				rules: [
					[match(Account, { id: acct, holder }), is(holder, root)],
					[match(Account, { id: acct, holder, kind: Kind.Savings }), match(Holder, { id: holder })]
				],
				select: { acct, n: count() }
			}
		}
		const first = query(Ledger, build)
		const second = query(Ledger, build)
		assert.notEqual(first, second, "two constructions are two values")
		assert.deepStrictEqual(lowerQuery(first), lowerQuery(second))
		assert.deepStrictEqual(lowerQuery(first), lowerQuery(first), "lowering is stable per value too")
	})

	test("engine roster refusals surface as typed prepare errors", function rosterError() {
		const unbound = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			const other = $.var(Account.fields.id)
			return { rules: [[match(Holder, { id: h })]], select: { h: other } }
		})
		assert.throws(function prepareUnbound() {
			db.prepare(unbound)
		}, /irError/)
	})

	test("prepared is a plain value: no lifecycle spelling, missing params throw typed", function preparedValue() {
		const withParam = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			const root = $.param("root", Holder.fields.id)
			return { rules: [[match(Holder, { id: h }), is(h, root)]], select: { h } }
		})
		const prepared = db.prepare(withParam)
		assert.equal("execute" in prepared, false, "execution has exactly one spelling: the scope's")
		assert.equal("close" in prepared, false)
		assert.equal(Symbol.dispose in prepared, false)
		const loose: Prepared<(typeof Ledger)["relations"], QueryRow<typeof withParam>, ParamsRecord> = prepared
		assert.throws(function missing() {
			db.execute(loose, {})
		}, /missing param root/)
		const rows = db.execute(prepared, { root: must(ids.ada) })
		assert.deepEqual(
			rows.map(function h(row) {
				return row.h
			}),
			[must(ids.ada)]
		)
	})

	test("an invalidated read scope refuses execute and staleness", function scopeRefusals() {
		const everyone = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			return { rules: [[match(Holder, { id: h })]], select: { h } }
		})
		const prepared = db.prepare(everyone)
		let escaped: ReadScope<(typeof Ledger)["relations"]> | undefined
		db.read(function capture(snap) {
			escaped = snap
			assert.equal(snap.execute(prepared, {}).length, 4)
		})
		const leaked = must(escaped)
		assert.throws(function executeAfterScope() {
			leaked.execute(prepared, {})
		}, /invalidated/)
		assert.throws(function stalenessAfterScope() {
			prepared.staleness(leaked)
		}, /invalidated/)
	})

	test("a prepared value of a different store is a typed refusal", async function crossStore() {
		const otherDb = await Db.create(path.join(tmpRoot, "other-store"), Ledger)
		const everyone = query(Ledger, function build($) {
			const h = $.var(Holder.fields.id)
			return { rules: [[match(Holder, { id: h })]], select: { h } }
		})
		const foreign = otherDb.prepare(everyone)
		assert.throws(function executeForeign() {
			db.execute(foreign, {})
		}, /different store/)
		db.read(function stalenessForeign(snap) {
			assert.throws(function foreignWitness() {
				foreign.staleness(snap)
			}, /different store/)
		})
	})
})
