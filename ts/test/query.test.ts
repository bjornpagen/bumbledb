import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, describe, test } from "node:test"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { bool, bytes, i64, interval, span, str, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import type { DbHandle } from "#native.ts"
import { native } from "#native.ts"
import { ALLEN } from "#query/atom.ts"
import type { AnyQuery, Query, QueryParams, QueryRow } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import type { Param, ParamsRecord } from "#query/scope.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, key } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-query-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: u64.fresh, name: str, rank: u64 })
const Account = relation("Account", {
	id: u64.fresh,
	holder: u64,
	kind: Kind.id,
	balance: i64,
	active: interval(u64),
	opened: u64,
	flagged: bool,
	tag: bytes(2)
})
const Parent = relation("Parent", {
	child: u64,
	parent: u64
})

const Ledger = schema("Ledger", { Kind, Holder, Account, Parent }, [
	contained(on(Account, "holder"), on(Holder, "id")),
	contained(on(Account, "kind"), on(Kind, "id")),
	contained(on(Parent, "child"), on(Holder, "id")),
	contained(on(Parent, "parent"), on(Holder, "id"))
])

type Rels = (typeof Ledger)["relations"]

const HOLDER_ID = 1
const ACCOUNT_ID = 2
const PARENT_ID = 3

const ids = {
	ada: 1n,
	grace: 2n,
	kurt: 3n,
	lone: 4n,
	adaChecking: 10n,
	adaSavings: 11n,
	graceSavings: 12n,
	kurtChecking: 13n
}

function sorted(values: readonly bigint[]): bigint[] {
	return [...values].sort(function asc(left, right) {
		if (left < right) {
			return -1
		}
		if (left > right) {
			return 1
		}
		return 0
	})
}

describe("the query surface against a real store", function suite() {
	let db: DbHandle

	before(async function seed() {
		const created = await native.dbCreate(storeDir, lower(Ledger))
		assert.equal(created.tag, "accepted", "the store admits")
		db = created.db
		const committed = native.dbWrite(db, function write(tx) {
			native.txInsert(tx, HOLDER_ID, 1n, [ids.ada, "ada", 1n])
			native.txInsert(tx, HOLDER_ID, 1n, [ids.grace, "grace", 2n])
			native.txInsert(tx, HOLDER_ID, 1n, [ids.kurt, "kurt", 3n])
			native.txInsert(tx, HOLDER_ID, 1n, [ids.lone, "lone", 9n])
			const checking = 0n
			const savings = 1n
			native.txInsert(tx, ACCOUNT_ID, 1n, [
				ids.adaChecking,
				ids.ada,
				checking,
				5n,
				{ start: 0n, end: 10n },
				1n,
				true,
				new Uint8Array([1, 2])
			])
			native.txInsert(tx, ACCOUNT_ID, 1n, [
				ids.adaSavings,
				ids.ada,
				savings,
				7n,
				{ start: 20n, end: 30n },
				25n,
				false,
				new Uint8Array([3, 4])
			])
			native.txInsert(tx, ACCOUNT_ID, 1n, [
				ids.graceSavings,
				ids.grace,
				savings,
				3n,
				{ start: 5n, end: 15n },
				6n,
				false,
				new Uint8Array([5, 6])
			])
			native.txInsert(tx, ACCOUNT_ID, 1n, [
				ids.kurtChecking,
				ids.kurt,
				checking,
				9n,
				{ start: 40n, end: 50n },
				45n,
				true,
				new Uint8Array([7, 8])
			])
			native.txInsert(tx, PARENT_ID, 1n, [ids.grace, ids.ada])
			native.txInsert(tx, PARENT_ID, 1n, [ids.kurt, ids.grace])
			return true
		})
		assert.equal(committed.tag, "accepted", "the seed commit lands")
	})

	/**
	 * The typed execute seam — exactly the shape the `Db` runtime consumes:
	 * lower → engine prepare → positional params via the query's own
	 * registry → decode by the head. Cast-free: `Row` and `Params` ride the
	 * query value.
	 */
	function run<Row, P extends ParamsRecord>(q: Query<Rels, Row, P>, params: P): Row[] {
		const prepared = native.dbPrepare(db, lowerQuery(q))
		if (!prepared.ok) {
			assert.fail(`engine prepare refused: ${prepared.message}`)
		}
		const rows = native.dbRead(db, function read(instance, _witness) {
			return native.preparedExecute(prepared.prepared, instance, wireParams(q.data.params, params))
		})
		native.preparedClose(prepared.prepared)
		return decodeAnswers<Row>(q.data.finds, rows)
	}

	function accepted(q: AnyQuery): void {
		const prepared = native.dbPrepare(db, lowerQuery(q))
		if (!prepared.ok) {
			assert.fail(`engine prepare refused: ${prepared.message}`)
		}
		native.preparedClose(prepared.prepared)
	}

	test("a multi-atom domain-equal join with a param returns the typed answer set", function joinWithParam() {
		const accountsOf = query(Ledger).rule((r) => {
			const { id: acct, holder: h } = v(Account)
			return r
				.match(Account, { id: acct, holder: h })
				.match(Holder, { id: h })
				.where(r.eq(h, r.param("root")))
				.find({ acct, h })
		})
		type RowPin = Expect<Equal<QueryRow<typeof accountsOf>, { readonly acct: bigint; readonly h: bigint }>>
		type ParamsPin = Expect<Equal<QueryParams<typeof accountsOf>, { readonly root: bigint }>>
		const rows = run(accountsOf, { root: ids.ada })
		assert.equal(rows.length, 2)
		for (const row of rows) {
			assert.equal(row.h, ids.ada)
			assert.equal(typeof row.acct, "bigint")
		}
		assert.deepEqual(
			sorted(
				rows.map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.adaSavings])
		)
		assert.deepEqual(run(accountsOf, { root: ids.lone }), [])
		const pins: [RowPin, ParamsPin] = [true, true]
		assert.equal(pins.length, 2)
	})

	test("a union of two rules deduplicates under set semantics", function union() {
		const adaOrSavings = query(Ledger)
			.rule((r) => {
				const { id: h } = v(Holder)
				return r.match(Holder, { id: h, name: "ada" }).find({ h })
			})
			.rule((r) => {
				const { id: h } = v(Holder)
				return r.match(Holder, { id: h }).match(Account, { holder: h, kind: "Savings" }).find({ h })
			})
		const rows = run(adaOrSavings, {})
		assert.deepEqual(
			sorted(
				rows.map(function h(row) {
					return row.h
				})
			),
			sorted([ids.ada, ids.grace]),
			"ada matches both rules and lands once — the union is a set"
		)
	})

	test("a union head holds the class wall — one answer column is one id space", function unionHeadClassWall() {
		/**
		 * Rule 0 binds x at Holder.id (class "Holder.id"), rule 1 at
		 * Account.id (class "Account.id"): the identical pairing at any
		 * join/eq position is refused, and the head is a reuse site too — a
		 * consumer reading the column as Holder ids would silently receive
		 * Account ids. The engine cannot backstop this (the wire IR carries
		 * no domains), so the SDK holds the wall at construction.
		 */
		assert.throws(function crossClassUnion() {
			query(Ledger)
				.rule((r) => {
					const { id: x } = v(Holder)
					return r.match(Holder, { id: x }).find({ x })
				})
				.rule((r) => {
					const { id: x } = v(Account)
					return r.match(Account, { id: x }).find({ x })
				})
		}, /a head column joins only class-equal slots/)

		const legal = query(Ledger)
			.rule((r) => {
				const { id: x } = v(Holder)
				return r.match(Holder, { id: x }).find({ x })
			})
			.rule((r) => {
				const { id: a, holder: x } = v(Account)
				return r.match(Account, { id: a, holder: x }).find({ x })
			})
		assert.deepEqual(
			sorted(
				run(legal, {}).map(function x(row) {
					return row.x
				})
			),
			sorted([ids.ada, ids.grace, ids.kurt, ids.lone]),
			"every holder id, whether from Holder or through Account.holder"
		)
	})

	test("a rec head holds the class wall — the sealed slot binds every rule", function recHeadClassWall() {
		assert.throws(function pollutedRecHead() {
			query(Ledger)
				.reach("reach", {
					base: [
						(r) => {
							const { id: c } = v(Holder)
							return r.match(Holder, { id: c }).find({ c })
						}
					],
					rec: [
						(r) => {
							const { id: h, rank: c } = v(Holder)
							return r.match(Holder, { id: h, rank: c }).find({ c })
						}
					]
				})
				.rule((r) => {
					const { id: c } = v(Holder)
					return r.match(Holder, { id: c }).interior("reach", { c }).find({ c })
				})
		}, /joins only class-equal slots/)
	})

	test("count() groups implicitly by the non-aggregate select entries", function counting() {
		const accountsPerHolder = query(Ledger).rule((r) => {
			const { id: acct, holder } = v(Account)
			return r.match(Account, { id: acct, holder }).find({ holder, count: r.count() })
		})
		type RowPin = Expect<Equal<QueryRow<typeof accountsPerHolder>, { readonly holder: bigint; readonly count: bigint }>>
		const rows = run(accountsPerHolder, {})
		const byHolder = new Map(
			rows.map(function entry(row) {
				return [row.holder, row.count] as const
			})
		)
		assert.equal(byHolder.size, 3, "lone has no account and no group — never a zero row")
		assert.equal(byHolder.get(ids.ada), 2n)
		assert.equal(byHolder.get(ids.grace), 1n)
		assert.equal(byHolder.get(ids.kurt), 1n)
		const pin: RowPin = true
		assert.ok(pin)
	})

	test("the recursive closure runs as one query", function closure() {
		const reachable = query(Ledger)
			.reach("reach", {
				base: [
					(r) => {
						const { id: c } = v(Holder)
						return r
							.match(Holder, { id: c })
							.where(r.eq(c, r.param("root")))
							.find({ c })
					}
				],
				rec: [
					(r) => {
						const { child: c, parent: m } = v(Parent)
						return r.match(Parent, { child: c, parent: m }).interior("reach", { c: m }).find({ c })
					}
				]
			})
			.rule((r) => {
				const { id: c } = v(Holder)
				return r.match(Holder, { id: c }).interior("reach", { c }).find({ c })
			})
		type ParamsPin = Expect<Equal<QueryParams<typeof reachable>, { readonly root: bigint }>>
		const fromAda = run(reachable, { root: ids.ada })
		assert.deepEqual(
			sorted(
				fromAda.map(function c(row) {
					return row.c
				})
			),
			sorted([ids.ada, ids.grace, ids.kurt]),
			"ada → grace → kurt closes; lone stays out"
		)
		const fromGrace = run(reachable, { root: ids.grace })
		assert.deepEqual(
			sorted(
				fromGrace.map(function c(row) {
					return row.c
				})
			),
			sorted([ids.grace, ids.kurt])
		)
		const pin: ParamsPin = true
		assert.ok(pin)
	})

	test("a finished-rec aggregate fold sums over the closure (recipe 25's form)", function finishedRecFold() {
		const reachBalance = query(Ledger)
			.reach("reach", {
				base: [
					(r) => {
						const { id: c } = v(Holder)
						return r
							.match(Holder, { id: c })
							.where(r.eq(c, r.param("root")))
							.find({ c })
					}
				],
				rec: [
					(r) => {
						const { child: c, parent: m } = v(Parent)
						return r.match(Parent, { child: c, parent: m }).interior("reach", { c: m }).find({ c })
					}
				]
			})
			.rule((r) => {
				const { holder: a, balance: m } = v(Account)
				return r
					.match(Account, { holder: a, balance: m })
					.interior("reach", { c: a })
					.find({ m: r.sum(m) })
			})
		type RowPin = Expect<Equal<QueryRow<typeof reachBalance>, { readonly m: bigint }>>
		const total = run(reachBalance, { root: ids.ada })
		assert.deepEqual(total, [{ m: 24n }], "5 + 7 (ada) + 3 (grace) + 9 (kurt)")
		const graceward = run(reachBalance, { root: ids.grace })
		assert.deepEqual(graceward, [{ m: 12n }])
		const pin: RowPin = true
		assert.ok(pin)
	})

	test("negation is a safe anti-join", function negation() {
		const holdersWithoutAccounts = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r
				.match(Holder, { id: h })
				.where(r.not(Account, { holder: h }))
				.find({ h })
		})
		const rows = run(holdersWithoutAccounts, {})
		assert.deepEqual(
			rows.map(function h(row) {
				return row.h
			}),
			[ids.lone]
		)
	})

	test("an ∈-set param in a negated atom rejects per element", function negatedSetParam() {
		const withoutKinds = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r
				.match(Holder, { id: h })
				.where(r.not(Account, { holder: h, kind: r.inSet("kinds") }))
				.find({ h })
		})
		const rows = run(withoutKinds, { kinds: ["Checking"] })
		assert.deepEqual(
			sorted(
				rows.map(function h(row) {
					return row.h
				})
			),
			sorted([ids.grace, ids.lone]),
			"grace holds only a savings account; lone holds none"
		)
	})

	test("point membership: literal, param (both value shapes), and pointIn", function membership() {
		const activeAtFive = query(Ledger).rule((r) => {
			const { id: acct } = v(Account)
			return r.match(Account, { id: acct, active: 5n }).find({ acct })
		})
		assert.deepEqual(
			sorted(
				run(activeAtFive, {}).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings]),
			"ada's checking [0,10) and grace's [5,15) cover the point 5"
		)

		const activeAt = query(Ledger).rule((r) => {
			const { id: acct } = v(Account)
			return r.match(Account, { id: acct, active: r.param("at") }).find({ acct })
		})
		type ParamsPin = Expect<
			Equal<QueryParams<typeof activeAt>, { readonly at: { readonly start: bigint; readonly end: bigint } }>
		>
		assert.deepEqual(
			run(activeAt, { at: span(0n, 10n) }).map(function acct(row) {
				return row.acct
			}),
			[ids.adaChecking],
			"an interval-field-anchored param is the interval reading: value equality"
		)
		assert.throws(function pointAtIntervalParam() {
			// @ts-expect-error — the interval-anchored param takes the interval reading; the point reading is pointIn's
			run(activeAt, { at: 5n })
		}, /expected Interval/)

		const pointInParam = query(Ledger).rule((r) => {
			const { id: acct, active: w } = v(Account)
			return r
				.match(Account, { id: acct, active: w })
				.where(r.pointIn(r.param("t"), w))
				.find({ acct })
		})
		assert.deepEqual(
			sorted(
				run(pointInParam, { t: 5n }).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings])
		)

		const intervalLiteralOperand = query(Ledger).rule((r) => {
			const { id: acct, opened: t } = v(Account)
			return r
				.match(Account, { id: acct, opened: t })
				.where(r.pointIn(t, span(0n, 10n)))
				.find({ acct })
		})
		assert.deepEqual(
			sorted(
				run(intervalLiteralOperand, {}).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings]),
			"pointIn(t, span(...)) — the legal interval-literal operand, lowered interval-left, tagged by the point sibling"
		)
		const pin: ParamsPin = true
		assert.ok(pin)
	})

	test("allen with a literal mask", function allenComparisons() {
		const intersecting = query(Ledger).rule((r) => {
			const { id: acct, active: w } = v(Account)
			return r
				.match(Account, { id: acct, active: w })
				.where(r.allen(w, ALLEN.intersects, span(0n, 12n)))
				.find({ acct })
		})
		assert.deepEqual(
			sorted(
				run(intersecting, {}).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings]),
			"[0,10) and [5,15) intersect [0,12); [20,30) and [40,50) are disjoint from it"
		)

		const related = query(Ledger).rule((r) => {
			const { id: acct, active: w } = v(Account)
			return r
				.match(Account, { id: acct, active: w })
				.where(r.allen(w, ALLEN.after, span(0n, 12n)))
				.find({ acct })
		})
		assert.deepEqual(
			sorted(
				run(related, {}).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaSavings, ids.kurtChecking]),
			"a second literal mask answers the complementary temporal question"
		)
	})

	test("∈-set params match on membership; the empty set matches nothing", function setParams() {
		const namedSet = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r.match(Holder, { id: h, name: r.inSet("names") }).find({ h })
		})
		type ParamsPin = Expect<Equal<QueryParams<typeof namedSet>, { readonly names: readonly string[] }>>
		assert.deepEqual(
			sorted(
				run(namedSet, { names: ["ada", "kurt"] }).map(function h(row) {
					return row.h
				})
			),
			sorted([ids.ada, ids.kurt])
		)
		assert.deepEqual(run(namedSet, { names: [] }), [], "the empty set matches nothing")

		const idSet = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r
				.match(Holder, { id: h })
				.where(r.eq(h, r.inSet("wanted")))
				.find({ h })
		})
		assert.deepEqual(
			sorted(
				run(idSet, { wanted: [ids.ada, ids.lone] }).map(function h(row) {
					return row.h
				})
			),
			sorted([ids.ada, ids.lone]),
			"a set param is legal as eq's right side — the IR's Eq-only set rule"
		)
		const pin: ParamsPin = true
		assert.ok(pin)
	})

	test("the or-tree is the rule-level disjunction", function orTree() {
		const eitherKind = query(Ledger).rule((r) => {
			const { id: acct, kind: k } = v(Account)
			return r
				.match(Account, { id: acct, kind: k })
				.where(r.or(r.eq(k, "Checking"), r.eq(k, "Savings")))
				.find({ acct })
		})
		assert.equal(run(eitherKind, {}).length, 4, "the disjunction spans both kinds")
	})

	test("a param value no rule uses never registers — the query executes under its inferred Params", function unusedParam() {
		let ghost: Param<"ghost"> | undefined
		const everyone = query(Ledger).rule((r) => {
			ghost = r.param("ghost")
			const { id: h } = v(Holder)
			return r.match(Holder, { id: h }).find({ h })
		})
		assert.ok(ghost !== undefined, "the param value exists — it was simply never placed in a rule")
		assert.deepEqual(everyone.data.params, [], "the registry is usage-derived; a dropped param value never registers")
		const emptyParams: QueryParams<typeof everyone> = {}
		const rows = run(everyone, emptyParams)
		assert.equal(rows.length, 4, "supplying the inferred params object always executes")
	})

	test("the same query built twice lowers to deeply-equal IR", function determinism() {
		function build() {
			return query(Ledger)
				.rule((r) => {
					const { id: acct, holder: h } = v(Account)
					return r
						.match(Account, { id: acct, holder: h })
						.where(r.eq(h, r.param("root")))
						.find({ acct, count: r.count() })
				})
				.rule((r) => {
					const { id: acct, holder: h } = v(Account)
					return r
						.match(Account, { id: acct, holder: h, kind: "Savings" })
						.match(Holder, { id: h })
						.where(r.eq(h, r.param("root")))
						.find({ acct, count: r.count() })
				})
		}
		const first = build()
		const second = build()
		assert.notEqual(first, second, "two constructions are two values")
		assert.deepStrictEqual(lowerQuery(first), lowerQuery(second))
		assert.deepStrictEqual(lowerQuery(first), lowerQuery(first), "lowering is stable per value too")
	})

	test("reference identity IS the join: one var value reused joins; two fresh mints never join", function referenceIdentityJoin() {
		const joined = query(Ledger).rule((r) => {
			const { id: acct, holder: h } = v(Account)
			const { name } = v(Holder)
			return r.match(Account, { id: acct, holder: h }).match(Holder, { id: h, name }).find({ acct, name })
		})

		const crossed = query(Ledger).rule((r) => {
			const { id: acct, holder: h } = v(Account)
			const { id: hid, name } = v(Holder)
			return r.match(Account, { id: acct, holder: h }).match(Holder, { id: hid, name }).find({ acct, name })
		})
		const joinedRows = run(joined, {})
		const byAccount = new Map(
			joinedRows.map(function pair(row) {
				return [row.acct, row.name] as const
			})
		)
		assert.equal(joinedRows.length, 4, "the join answers each account with its own holder — one row per account")
		assert.equal(byAccount.get(ids.adaChecking), "ada")
		assert.equal(byAccount.get(ids.adaSavings), "ada")
		assert.equal(byAccount.get(ids.graceSavings), "grace")
		assert.equal(byAccount.get(ids.kurtChecking), "kurt")
		assert.equal(
			run(crossed, {}).length,
			16,
			"two fresh mints never join — 4 accounts × 4 holders is the cross product"
		)
		assert.notDeepStrictEqual(
			lowerQuery(joined),
			lowerQuery(crossed),
			"reference reuse and two fresh mints lower to DIFFERENT IR"
		)
	})

	test("the name-collision join is unrepresentable: same-named columns of two mints are unrelated variables", function nameCollision() {
		const twoParents = query(Ledger).rule((r) => {
			const a = v(Parent)
			const b = v(Parent)
			return r
				.match(Parent, { child: a.child, parent: a.parent })
				.match(Parent, { child: b.child, parent: b.parent })
				.find({ x: a.child, y: b.child })
		})
		const lowered = lowerQuery(twoParents)
		const outputRule = lowered.rules[0]
		assert.ok(outputRule !== undefined, "the main query carries the one rule")
		const findVars = outputRule.finds.map(function idOf(entry) {
			assert.equal(entry.kind, "var", "both find entries project a variable")
			return entry.kind === "var" ? entry.var : -1
		})
		assert.equal(findVars.length, 2)
		assert.notEqual(
			findVars[0],
			findVars[1],
			"two same-named mints are two var ids — the join by name is unrepresentable"
		)
		// And by rows: two Parent facts, two unrelated child variables → the

		assert.equal(run(twoParents, {}).length, 4, "same-named columns of two mints cross-product, never join")
	})

	test("find keys name the answer columns: renames are real", function renamesAreReal() {
		const renamed = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r.match(Holder, { id: h }).find({ renamed: h })
		})
		type RenamePin = Expect<Equal<QueryRow<typeof renamed>, { readonly renamed: bigint }>>
		const renamePin: RenamePin = true
		assert.ok(renamePin)
		const renamedRows = run(renamed, {})
		assert.equal(renamedRows.length, 4)
		const probe = renamedRows[0]
		assert.ok(probe !== undefined)
		const typedProbe = probe satisfies { readonly renamed: bigint }
		assert.equal(typeof typedProbe.renamed, "bigint")
		assert.deepEqual(
			sorted(
				renamedRows.map(function renamedOf(row) {
					return row.renamed
				})
			),
			sorted([ids.ada, ids.grace, ids.kurt, ids.lone]),
			"the row carries every holder id under the renamed key"
		)
	})

	test("the engine's prepare accepts every construct the surface can spell (the IR-bijection pin)", function prepareSweep() {
		const constructs: AnyQuery[] = [
			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				return r.match(Holder, { id: h }).where(r.ne(h, 1n)).find({ h })
			}),

			query(Ledger).rule((r) => {
				const { id: acct, balance: b } = v(Account)
				return r
					.match(Account, { id: acct, balance: b })
					.where(r.le(b, 7n))
					.where(r.gt(b, 0n))
					.where(r.ge(b, 3n))
					.where(r.lt(b, 100n))
					.find({ acct })
			}),

			query(Ledger).rule((r) => {
				const { id: acct, kind: k, balance: b } = v(Account)
				return r
					.match(Account, { id: acct, kind: k, balance: b })
					.where(r.or(r.and(r.eq(k, "Checking"), r.gt(b, 4n)), r.eq(k, "Savings")))
					.find({ acct })
			}),

			query(Ledger).rule((r) => {
				const { holder: h, balance: b } = v(Account)
				return r.match(Account, { holder: h, balance: b }).find({ h, b: r.sum(b) })
			}),
			query(Ledger).rule((r) => {
				const { holder: h, balance: b } = v(Account)
				return r.match(Account, { holder: h, balance: b }).find({ h, b: r.min(b) })
			}),
			query(Ledger).rule((r) => {
				const { holder: h, balance: b } = v(Account)
				return r.match(Account, { holder: h, balance: b }).find({ h, b: r.max(b) })
			}),

			query(Ledger).rule((r) => {
				const { holder: h, active: w } = v(Account)
				return r.match(Account, { holder: h, active: w }).find({ h, w: r.pack(w) })
			}),

			query(Ledger).rule((r) => {
				const { id: acct } = v(Account)
				return r.match(Account, { id: acct, flagged: true }).find({ acct })
			}),
			query(Ledger).rule((r) => {
				const { id: acct } = v(Account)
				return r.match(Account, { id: acct, tag: new Uint8Array([1, 2]) }).find({ acct })
			}),
			query(Ledger).rule((r) => {
				const { id: acct } = v(Account)
				return r.match(Account, { id: acct, balance: 5n }).find({ acct })
			}),
			query(Ledger).rule((r) => {
				const { id: acct } = v(Account)
				return r.match(Account, { id: acct, active: span(0n, 10n) }).find({ acct })
			}),
			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				return r.match(Holder, { id: h, name: "ada" }).find({ h })
			}),
			query(Ledger).rule((r) => {
				const { id: acct } = v(Account)
				return r.match(Account, { id: acct, kind: "Savings" }).find({ acct })
			}),

			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				return r.match(Holder, { id: h }).match(Parent, {}).find({ h })
			}),

			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				return r.match(Holder, { id: h, name: r.param("n") }).find({ h })
			}),
			query(Ledger).rule((r) => {
				const { id: acct } = v(Account)
				return r.match(Account, { id: acct, active: r.param("at") }).find({ acct })
			}),
			query(Ledger).rule((r) => {
				const { id: acct } = v(Account)
				return r.match(Account, { id: acct, kind: r.inSet("kinds") }).find({ acct })
			})
		]
		for (const construct of constructs) {
			accepted(construct)
		}
	})

	test("engine roster refusals surface as typed prepare errors", function rosterError() {
		const emptyMask = query(Ledger).rule((r) => {
			const { id: acct, active: w } = v(Account)
			return r
				.match(Account, { id: acct, active: w })
				.where(r.allen(w, 0, span(0n, 12n)))
				.find({ acct })
		})
		const prepared = native.dbPrepare(db, lowerQuery(emptyMask))
		assert.ok(!prepared.ok, "the empty Allen mask is the engine's typed refusal")
		assert.equal(prepared.kind, "irError")
	})

	test("every rule of a query derives the same head", function headAlignment() {
		assert.throws(function misaligned() {
			query(Ledger)
				.rule((r) => {
					const { id: h } = v(Holder)
					return r.match(Holder, { id: h }).find({ h })
				})
				.rule((r) => {
					const { id: acct } = v(Account)
					return r.match(Account, { id: acct }).find({ acct })
				})
		}, /derives the same head/)
	})

	test("a param name keeps one wire shape", function paramShapeConflict() {
		assert.throws(function conflicted() {
			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				return r
					.match(Holder, { id: h, name: r.param("who") })
					.where(r.eq(h, r.inSet("who")))
					.find({ h })
			})
		}, /one name, one shape/)
	})

	test("execute refuses a missing param, typed", function missingParam() {
		const withParam = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r
				.match(Holder, { id: h })
				.where(r.eq(h, r.param("root")))
				.find({ h })
		})
		assert.throws(function missing() {
			// @ts-expect-error — the inferred params object demands root; omitting it is a compile error too
			run(withParam, {})
		}, /missing param root/)
		assert.deepEqual(
			run(withParam, { root: ids.ada }).map(function h(row) {
				return row.h
			}),
			[ids.ada]
		)
	})

	test("TYPE WALLS: the unwritable queries are unwritable (each expect-error real)", function typeWalls() {
		const varDied = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			// @ts-expect-error — r.var died with 0.6.0
			const deadVar = r.var
			assert.equal(deadVar, undefined)
			return r.match(Holder, { id: h }).find({ h })
		})
		assert.equal(varDied.data.rules.length, 1)

		const selectDied = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			const chain = r.match(Holder, { id: h })
			// @ts-expect-error — select died into find
			const deadSelect = chain.select
			assert.equal(deadSelect, undefined)
			return chain.find({ h })
		})
		assert.equal(selectDied.data.rules.length, 1)

		// class — the class-equal join law, refused AT THE POSITION (compile)

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

		// error AND a construction refusal (the runtime twin; the wall holds

		assert.throws(function crossClassEq() {
			query(Ledger).rule((r) => {
				const { id: a, holder: h } = v(Account)
				return (
					r
						.match(Account, { id: a, holder: h })
						// @ts-expect-error — "a" is in the "Account.id" class, "h" in "Holder.id"
						.where(r.eq(a, h))
						.find({ a })
				)
			})
		}, /unifies domain-unequal fields/)

		assert.throws(function crossClassNe() {
			query(Ledger).rule((r) => {
				const { id: a, holder: h } = v(Account)
				return (
					r
						.match(Account, { id: a, holder: h })
						// @ts-expect-error — ne is the same unification judgment as eq
						.where(r.ne(a, h))
						.find({ a })
				)
			})
		}, /unifies domain-unequal fields/)

		assert.throws(function bareClassedWall() {
			query(Ledger).rule((r) => {
				const { opened: z } = v(Account)
				return (
					r
						.match(Account, { opened: z })
						// @ts-expect-error — bare pairs only with bare; a classed slot refuses a bare mint
						.match(Account, { holder: z })
						.find({ z })
				)
			})
		}, /joins domain-unequal fields/)

		const sameClassEq = query(Ledger).rule((r) => {
			const { holder: h } = v(Account)
			const { id: h2 } = v(Holder)
			return r.match(Account, { holder: h }).match(Holder, { id: h2 }).where(r.eq(h, h2)).find({ h })
		})
		assert.equal(sameClassEq.data.rules.length, 1)
		const bareBareEq = query(Ledger).rule((r) => {
			const { id: h, rank: z } = v(Holder)
			const { opened: o } = v(Account)
			return r.match(Holder, { id: h, rank: z }).match(Account, { opened: o }).where(r.eq(z, o)).find({ h })
		})
		assert.equal(bareBareEq.data.rules.length, 1)

		const intervalUnderOrder = query(Ledger).rule((r) => {
			const { id: acct, active: w } = v(Account)
			return (
				r
					.match(Account, { id: acct, active: w })
					// @ts-expect-error — an interval-typed variable has no order; pointIn/allen are the interval predicates
					.where(r.lt(w, 5n))
					.find({ acct })
			)
		})
		assert.equal(intervalUnderOrder.data.rules.length, 1)

		const intervalUnderEq = query(Ledger).rule((r) => {
			const { id: acct, active: w } = v(Account)
			return (
				r
					.match(Account, { id: acct, active: w })
					// @ts-expect-error — eq of an interval variable takes an interval, never a scalar point
					.where(r.eq(w, 5n))
					.find({ acct })
			)
		})
		assert.equal(intervalUnderEq.data.rules.length, 1)

		assert.throws(function mintingHead() {
			query(Ledger).rule((r) => {
				const { id: acct, balance: b } = v(Account)
				return (
					r
						.match(Account, { id: acct, balance: b })
						// @ts-expect-error — no minting or arithmetic term exists in the head vocabulary (the creation quarantine)
						.find({ sum: 1n + 2n })
				)
			})
		}, /not a find entry/)

		const byName = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r.match(Holder, { id: h, name: r.param("who") }).find({ h })
		})
		assert.throws(function wrongParamType() {
			// @ts-expect-error — "who" is string-typed by its binding field
			run(byName, { who: 5n })
		}, /expected string/)

		const everyone = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r.match(Holder, { id: h }).find({ h })
		})
		// @ts-expect-error — the result row is typed by the head: h is bigint, never string
		const wrong: { h: string }[] = run(everyone, {})
		assert.equal(wrong.length, 4)

		assert.throws(function unsafeNegation() {
			query(Ledger).rule((r) => {
				const { id: h } = v(Holder)
				const { parent: ghost } = v(Parent)
				return r
					.match(Holder, { id: h })
					.where(r.not(Account, { holder: ghost }))
					.find({ h })
			})
		}, /Parent\.parent/)
	})

	test("THE FOUR JOIN LAWS: same-class joins+lowers, cross-class refuses at the use site, bare↔bare joins, bare↔classed refuses", function joinLaws() {
		const sameClass = query(Ledger).rule((r) => {
			const { id: acct, holder: h } = v(Account)
			return r.match(Account, { id: acct, holder: h }).match(Holder, { id: h }).find({ acct })
		})
		assert.equal(lowerQuery(sameClass).interiors.length, 0)
		assert.equal(lowerQuery(sameClass).kind, "cq")

		assert.throws(function crossClass() {
			query(Ledger).rule((r) => {
				const { id: x } = v(Holder)
				return (
					r
						.match(Holder, { id: x })
						// @ts-expect-error — the use site: Account.id generates its own class
						.match(Account, { id: x })
						.find({ x })
				)
			})
		}, /joins domain-unequal fields/)

		const bareBare = query(Ledger).rule((r) => {
			const { id: h, rank: z } = v(Holder)
			const { opened: o } = v(Account)
			return r.match(Holder, { id: h, rank: z }).match(Account, { opened: o }).where(r.eq(z, o)).find({ h })
		})
		assert.deepEqual(
			run(bareBare, {}).map(function h(row) {
				return row.h
			}),
			[ids.ada]
		)

		assert.throws(function bareClassed() {
			query(Ledger).rule((r) => {
				const { opened: z } = v(Account)
				return (
					r
						.match(Account, { opened: z })
						// @ts-expect-error — bare pairs only with bare; "Holder.id" is a classed slot
						.match(Account, { holder: z })
						.find({ z })
				)
			})
		}, /joins domain-unequal fields/)
	})

	test("RECURSION FENCES: the cut is typed and the quarantine unwritable", function recursionFences() {
		const afterRec = query(Ledger).reach("a", {
			base: [
				(r) => {
					const { id: h } = v(Holder)
					return r.match(Holder, { id: h }).find({ h })
				}
			],
			rec: [
				(r) => {
					const { child: c, parent: m } = v(Parent)
					return r.match(Parent, { child: c, parent: m }).interior("a", { h: m }).find({ h: c })
				}
			]
		})
		type SecondRecPin = Expect<Equal<"reach" extends keyof typeof afterRec ? true : false, false>>
		type InteriorAfterRecPin = Expect<Equal<"interior" extends keyof typeof afterRec ? true : false, false>>
		void (0 as unknown as SecondRecPin)
		void (0 as unknown as InteriorAfterRecPin)
		function unwritableAfterRec() {
			// @ts-expect-error — a second reach is unwritable
			afterRec.reach("b", { base: [], rec: [] })
			// @ts-expect-error — interiors cannot follow a reach
			afterRec.interior("mid")
		}
		void unwritableAfterRec

		assert.equal("reach" in afterRec, false, "Reach start does not carry reach")
		assert.equal("interior" in afterRec, false, "Reach start does not carry interior")

		assert.throws(function interiorAfterMain() {
			query(Ledger)
				.rule((r) => {
					const { id: h } = v(Holder)
					return r.match(Holder, { id: h }).find({ h })
				})
				// @ts-expect-error — interiors precede main rules
				.interior("mid", (r) => {
					const { id: h } = v(Holder)
					return r.match(Holder, { id: h }).find({ h })
				})
		}, /after a main rule/)

		assert.throws(function duplicateInterior() {
			query(Ledger)
				.interior("mid", (r) => {
					const { id: h } = v(Holder)
					return r.match(Holder, { id: h }).find({ h })
				})
				.interior("mid", (r) => {
					const { id: h } = v(Holder)
					return r.match(Holder, { id: h }).find({ h })
				})
		}, /already declared/)

		assert.throws(function aggregateThroughCycle() {
			query(Ledger).reach("reach", {
				base: [
					(r) => {
						const { holder: h, balance: b } = v(Account)
						return (
							r
								.match(Account, { holder: h, balance: b })
								// @ts-expect-error — a recursive head projects bound variables only
								.find({ b: r.sum(b) })
						)
					}
				],
				rec: [
					(r) => {
						const { child: c, parent: m } = v(Parent)
						return r.match(Parent, { child: c, parent: m }).interior("reach", { b: m }).find({ b: c })
					}
				]
			})
		}, /projects bound variables only/)

		const identityProjection = query(Ledger)
			.reach("reach", {
				base: [
					(r) => {
						const { id: h } = v(Holder)
						return r.match(Holder, { id: h }).find({ h })
					}
				],
				rec: [
					(r) => {
						const { child: c, parent: m } = v(Parent)
						return r.match(Parent, { child: c, parent: m }).interior("reach", { h: m }).find({ h: c })
					}
				]
			})
			.rule((r) => {
				const { id: h } = v(Holder)
				return r.interior("reach", { h }).find({ h })
			})
		assert.ok(identityProjection, "the interior-only main rule is spellable — no re-grounding join exists")
		assert.equal(identityProjection.data.kind, "reach", "the recursive query is a Reach")
		const rec = identityProjection.data.kind === "reach" ? identityProjection.data.rec : undefined
		assert.ok(rec !== undefined, "the recursive query seals a RecData")
		assert.ok(rec.base.length >= 1, "sealed RecData cannot carry an empty base")
		assert.ok(rec.rec.length >= 1, "sealed RecData cannot carry an empty rec")
		assert.ok(rec.finds.length >= 1, "arm scopes never observe empty rec finds — the head is sealed before rec arms")
		assert.ok(Object.isFrozen(rec), "RecData is sealed in one assignment")
		assert.throws(function classUnequalInteriorVar() {
			query(Ledger)
				.reach("reach", {
					base: [
						(r) => {
							const { id: h } = v(Holder)
							return r.match(Holder, { id: h }).find({ h })
						}
					],
					rec: [
						(r) => {
							const { child: c, parent: m } = v(Parent)
							return r.match(Parent, { child: c, parent: m }).interior("reach", { h: m }).find({ h: c })
						}
					]
				})
				.rule((r) => {
					const { id: acct } = v(Account)
					return r.interior("reach", { h: acct }).find({ h: acct })
				})
		}, /joins the variable/)

		assert.throws(function unboundNegatedInteriorVar() {
			query(Ledger)
				.reach("reach", {
					base: [
						(r) => {
							const { id: h } = v(Holder)
							return r.match(Holder, { id: h }).find({ h })
						}
					],
					rec: [
						(r) => {
							const { child: c, parent: m } = v(Parent)
							return r.match(Parent, { child: c, parent: m }).interior("reach", { h: m }).find({ h: c })
						}
					]
				})
				.rule((r) => {
					const { id: h } = v(Holder)
					const { id: ghost } = v(Holder)
					return r
						.match(Holder, { id: h })
						.where(r.not("reach", { h: ghost }))
						.find({ h })
				})
		}, /negated interior reach names the variable/)
		// ...and inside a recursive rule it is negation through the cycle, refused.
		assert.throws(function negationThroughCycle() {
			query(Ledger)
				.reach("reach", {
					base: [
						(r) => {
							const { id: h } = v(Holder)
							return r.match(Holder, { id: h }).find({ h })
						}
					],
					rec: [
						(r) => {
							const { id: h } = v(Holder)
							return r.match(Holder, { id: h }).where(r.not("reach", { h })).find({ h })
						}
					]
				})
				.rule((r) => {
					const { id: h } = v(Holder)
					return r.interior("reach", { h }).find({ h })
				})
		}, /negation through the cycle/)

		const plain = query(Ledger).rule((r) => {
			const { id: h } = v(Holder)
			return r.match(Holder, { id: h }).find({ h })
		})
		assert.equal(plain.data.interiors.length, 0, "a plain query carries no interiors")
		assert.equal(plain.data.kind, "cq", "a plain query is a CQ")
	})
})

test("not() joins a fresh mint with its u64 copy and a partial-key anti-join", function mintClassAntiJoin() {
	const SourceNode = relation("SourceNode", { node: u64.fresh })
	const NodeLowering = relation("NodeLowering", { node: u64, body: str })
	const SourceFramework = relation("SourceFramework", { node: u64, name: str })
	const FrameworkLowering = relation("FrameworkLowering", { sourceNode: u64, body: str })
	const CoreStatement = relation("CoreStatement", { corpus: u64, statement: u64 })
	const OtherMint = relation("OtherMint", { id: u64.fresh })
	const Oracle = schema(
		"Oracle",
		{ SourceNode, NodeLowering, SourceFramework, FrameworkLowering, CoreStatement, OtherMint },
		[
			contained(on(NodeLowering, "node"), on(SourceNode, "node")),
			key(SourceFramework, ["node"]),
			contained(on(FrameworkLowering, "sourceNode"), on(SourceFramework, "node"))
		]
	)

	const freshVsCopy = query(Oracle).rule((r) => {
		const { node } = v(NodeLowering)
		return r.match(NodeLowering, { node }).where(r.not(SourceNode, { node })).find({ node })
	})
	assert.equal(freshVsCopy.data.kind, "cq")

	const opposite = query(Oracle).rule((r) => {
		const { node } = v(SourceNode)
		return r.match(SourceNode, { node }).where(r.not(NodeLowering, { node })).find({ node })
	})
	assert.equal(opposite.data.kind, "cq")

	const twoPlain = query(Oracle).rule((r) => {
		const { sourceNode } = v(FrameworkLowering)
		return r
			.match(FrameworkLowering, { sourceNode })
			.where(r.not(SourceFramework, { node: sourceNode }))
			.find({ sourceNode })
	})
	assert.equal(twoPlain.data.kind, "cq")

	const partialKey = query(Oracle).rule((r) => {
		const { node } = v(SourceNode)
		return r
			.match(SourceNode, { node })
			.where(r.not(CoreStatement, { statement: node }))
			.find({ node })
	})
	assert.equal(partialKey.data.kind, "cq")

	assert.throws(function twoGenerators() {
		query(Oracle).rule((r) => {
			const { node } = v(SourceNode)
			return (
				r
					.match(SourceNode, { node })
					// @ts-expect-error — two generators stay refused
					.where(r.not(OtherMint, { id: node }))
					.find({ node })
			)
		})
	}, /domain-unequal/)
})

test("not() joins two closed ids that share a handle set or roster length", function closedIdAntiJoin() {
	const SourceGrade = closed("SourceGrade", ["K", "1", "2"])
	const Grade = closed("Grade", ["K", "1", "2"])
	const CodeKind = closed("CodeKind", ["primary", "alternate"])
	const ClassificationScheme = closed("ClassificationScheme", ["reported", "source-normalized"])
	const SourceClassificationScheme = closed("SourceClassificationScheme", ["reported", "provider-normalized"])
	const SourceNode = relation("SourceNode", { node: u64.fresh })
	const SourceStatementGrade = relation("SourceStatementGrade", {
		node: u64,
		grade: SourceGrade.id
	})
	const SourceStatementClassification = relation("SourceStatementClassification", {
		node: u64,
		scheme: SourceClassificationScheme.id
	})
	const Statement = relation("Statement", { statement: u64.fresh })
	const StatementGrade = relation("StatementGrade", { statement: u64, grade: Grade.id })
	const StatementClassification = relation("StatementClassification", {
		statement: u64,
		scheme: ClassificationScheme.id
	})
	const StatementLowering = relation("StatementLowering", { statement: u64, sourceNode: u64 })
	const StatementCode = relation("StatementCode", { statement: u64, kind: CodeKind.id })
	const Oracle = schema(
		"ClosedOracle",
		{
			SourceGrade,
			Grade,
			CodeKind,
			ClassificationScheme,
			SourceClassificationScheme,
			SourceNode,
			SourceStatementGrade,
			SourceStatementClassification,
			Statement,
			StatementGrade,
			StatementClassification,
			StatementLowering,
			StatementCode
		},
		[
			contained(on(SourceStatementGrade, "node"), on(SourceNode, "node")),
			contained(on(SourceStatementClassification, "node"), on(SourceNode, "node")),
			contained(on(StatementGrade, "statement"), on(Statement, "statement")),
			contained(on(StatementClassification, "statement"), on(Statement, "statement")),
			contained(on(StatementCode, "statement"), on(Statement, "statement")),
			contained(on(StatementLowering, "statement"), on(Statement, "statement")),
			contained(on(StatementLowering, "sourceNode"), on(SourceNode, "node"))
		]
	)

	const image = query(Oracle).rule((r) => {
		const { statement, grade } = v(StatementGrade)
		const { sourceNode: node } = v(StatementLowering)
		return r
			.match(StatementGrade, { statement, grade })
			.match(StatementLowering, { statement, sourceNode: node })
			.where(r.not(SourceStatementGrade, { node, grade }))
			.find({ statement })
	})
	assert.equal(image.data.kind, "cq")

	const opposite = query(Oracle).rule((r) => {
		const { node, grade } = v(SourceStatementGrade)
		const { statement } = v(StatementLowering)
		return r
			.match(SourceStatementGrade, { node, grade })
			.match(StatementLowering, { statement, sourceNode: node })
			.where(r.not(StatementGrade, { statement, grade }))
			.find({ statement })
	})
	assert.equal(opposite.data.kind, "cq")

	const scheme = query(Oracle).rule((r) => {
		const { statement, scheme } = v(StatementClassification)
		const { sourceNode: node } = v(StatementLowering)
		return r
			.match(StatementClassification, { statement, scheme })
			.match(StatementLowering, { statement, sourceNode: node })
			.where(r.not(SourceStatementClassification, { node, scheme }))
			.find({ statement })
	})
	assert.equal(scheme.data.kind, "cq")

	assert.throws(function closedVocabularyMismatch() {
		query(Oracle).rule((r) => {
			const { statement, grade } = v(StatementGrade)
			return (
				r
					.match(StatementGrade, { statement, grade })
					// @ts-expect-error — Grade.id vs CodeKind.id are different closed vocabularies
					.where(r.not(StatementCode, { statement, kind: grade }))
					.find({ statement })
			)
		})
	}, /domain-unequal/)
})
