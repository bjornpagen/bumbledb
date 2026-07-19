/**
 * Query-surface pins against a REAL durable store, STRUCTURAL edition —
 * the kysely-shaped builder end to end: a multi-atom domain-equal join
 * with a param, negation as a safe anti-join, a union of two rules (set
 * semantics dedup), `count()` with implicit grouping, the recursive
 * closure and the finished-stratum aggregate fold as one stratified
 * `program()`, point membership (literal, param, and `pointIn`/`covers`
 * both spellings), `allen` with a literal and a bound mask, ∈-set params,
 * the or-tree, deterministic lowering (same query built twice →
 * deeply-equal IR), the engine's prepare ACCEPTING every construct the
 * surface can spell (the IR-bijection pin), the unused-param law (a param
 * value no rule uses never registers — the query executes under its own
 * inferred `Params`), and the type walls (each `@ts-expect-error` real):
 * cross-CLASS joins (the schema is LAW-TYPED — rulings 2/3: the statement
 * list is what puts `Account.holder` and `Holder.id` in one class while
 * `Account.id` generates its own; the four join-law probes — same-class
 * joins, cross-class refusal at the use site, bare↔bare joining, and
 * bare↔classed refusal — are pinned through `r.var` AND re-pinned through
 * `r.vars`), interval-vs-scalar comparisons outside `pointIn`, minting
 * terms in heads, wrong-typed params, and mismatched result shapes are
 * all unwritable. Execution rides the native bridge directly — the `Db`
 * runtime's typed prepare/execute is S4's surface; the typed seams
 * exercised here (`lowerQuery` + `wireParams` + `decodeAnswers`) are
 * exactly what it consumes.
 */

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
import { program } from "#query/predicate.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import type { Param, ParamsRecord } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
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

/**
 * THE LAWS TYPE THE COLUMNS: the containments below put `Account.holder`,
 * `Parent.child`, and `Parent.parent` in the `"Holder.id"` generator class
 * and `Account.kind` in `"Kind.id"`, while `Account.id` generates
 * `"Account.id"` — and `Holder.rank`/`Account.opened` are in no law: BARE,
 * the bare↔bare join probes' slots.
 */
const Ledger = schema("Ledger", { Kind, Holder, Account, Parent }, [
	contained(on(Account, "holder"), on(Holder, "id")),
	contained(on(Account, "kind"), on(Kind, "id")),
	contained(on(Parent, "child"), on(Holder, "id")),
	contained(on(Parent, "parent"), on(Holder, "id"))
])

type Rels = (typeof Ledger)["relations"]

/** Relation ids = record declaration order (the law `lowerQuery` rides). */
const HOLDER_ID = 1
const ACCOUNT_ID = 2
const PARENT_ID = 3

/** The seeded ids the tests read (resupplied explicitly — the ETL idiom). */
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

describe("the query surface against a real store", function suite() {
	let db: DbHandle

	before(function seed() {
		const created = native.dbCreate(storeDir, lower(Ledger))
		assert.ok(created.ok, "the store admits")
		db = created.db
		const tx = native.dbWriteBegin(db)
		native.txInsert(tx, HOLDER_ID, [ids.ada, "ada", 1n])
		native.txInsert(tx, HOLDER_ID, [ids.grace, "grace", 2n])
		native.txInsert(tx, HOLDER_ID, [ids.kurt, "kurt", 3n])
		native.txInsert(tx, HOLDER_ID, [ids.lone, "lone", 9n])
		const checking = 0n
		const savings = 1n
		native.txInsert(tx, ACCOUNT_ID, [
			ids.adaChecking,
			ids.ada,
			checking,
			5n,
			{ start: 0n, end: 10n },
			1n,
			true,
			new Uint8Array([1, 2])
		])
		native.txInsert(tx, ACCOUNT_ID, [
			ids.adaSavings,
			ids.ada,
			savings,
			7n,
			{ start: 20n, end: 30n },
			25n,
			false,
			new Uint8Array([3, 4])
		])
		native.txInsert(tx, ACCOUNT_ID, [
			ids.graceSavings,
			ids.grace,
			savings,
			3n,
			{ start: 5n, end: 15n },
			6n,
			false,
			new Uint8Array([5, 6])
		])
		native.txInsert(tx, ACCOUNT_ID, [
			ids.kurtChecking,
			ids.kurt,
			checking,
			9n,
			{ start: 40n, end: 50n },
			45n,
			true,
			new Uint8Array([7, 8])
		])
		native.txInsert(tx, PARENT_ID, [ids.grace, ids.ada])
		native.txInsert(tx, PARENT_ID, [ids.kurt, ids.grace])
		const committed = native.txCommit(tx)
		assert.ok(committed.ok, "the seed commit lands")
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
		const snap = native.dbSnapshot(db)
		const rows = native.preparedExecute(prepared.prepared, snap, wireParams(q.data.params, params))
		native.snapshotClose(snap)
		native.preparedClose(prepared.prepared)
		return decodeAnswers<Row>(q.data.select, rows)
	}

	/** The prepare-acceptance pin: the engine's own validation admits the lowered IR. */
	function accepted(q: AnyQuery): void {
		const prepared = native.dbPrepare(db, lowerQuery(q))
		if (!prepared.ok) {
			assert.fail(`engine prepare refused: ${prepared.message}`)
		}
		native.preparedClose(prepared.prepared)
	}

	test("a multi-atom domain-equal join with a param returns the typed answer set", function joinWithParam() {
		const accountsOf = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), holder: r.var("h") })
				.match(Holder, { id: r.var("h") })
				.where(r.eq(r.var("h"), r.param("root")))
				.select("acct", "h")
		)
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
			.rule((r) => r.match(Holder, { id: r.var("h"), name: "ada" }).select("h"))
			.rule((r) =>
				r
					.match(Holder, { id: r.var("h") })
					.match(Account, { holder: r.var("h"), kind: "Savings" })
					.select("h")
			)
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

	test("count() groups implicitly by the non-aggregate select entries", function counting() {
		const accountsPerHolder = query(Ledger).rule((r) =>
			r.match(Account, { id: r.var("acct"), holder: r.var("holder") }).select("holder", r.count())
		)
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

	test("the recursive closure runs as one stratified program", function closure() {
		const reachable = program(Ledger, (p) => {
			const reach = p.rec("reach")
			const seeded = reach
				.rule((r) =>
					r
						.match(Holder, { id: r.var("c") })
						.where(r.eq(r.var("c"), r.param("root")))
						.select("c")
				)
				.rule((r) =>
					r
						.match(Parent, { child: r.var("c"), parent: r.var("m") })
						.idb(reach, r.var("m"))
						.select("c")
				)
			return p.output((r) =>
				r
					.match(Holder, { id: r.var("c") })
					.idb(seeded, r.var("c"))
					.select("c")
			)
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

	test("a finished-stratum aggregate fold sums over the closure (recipe 25's form)", function finishedStratumFold() {
		const reachBalance = program(Ledger, (p) => {
			const reach = p.rec("reach")
			const seeded = reach
				.rule((r) =>
					r
						.match(Holder, { id: r.var("c") })
						.where(r.eq(r.var("c"), r.param("root")))
						.select("c")
				)
				.rule((r) =>
					r
						.match(Parent, { child: r.var("c"), parent: r.var("m") })
						.idb(reach, r.var("m"))
						.select("c")
				)
			return p.output((r) =>
				r
					.match(Account, { holder: r.var("a"), balance: r.var("m") })
					.idb(seeded, r.var("a"))
					.select(r.sum("m"))
			)
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
		const holdersWithoutAccounts = query(Ledger).rule((r) =>
			r
				.match(Holder, { id: r.var("h") })
				.where(r.not(Account, { holder: r.var("h") }))
				.select("h")
		)
		const rows = run(holdersWithoutAccounts, {})
		assert.deepEqual(
			rows.map(function h(row) {
				return row.h
			}),
			[ids.lone]
		)
	})

	test("an ∈-set param in a negated atom rejects per element", function negatedSetParam() {
		const withoutKinds = query(Ledger).rule((r) =>
			r
				.match(Holder, { id: r.var("h") })
				.where(r.not(Account, { holder: r.var("h"), kind: r.inSet("kinds") }))
				.select("h")
		)
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

	test("point membership: literal, param (both value shapes), pointIn, and covers", function membership() {
		const activeAtFive = query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), active: 5n }).select("acct"))
		assert.deepEqual(
			sorted(
				run(activeAtFive, {}).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings]),
			"ada's checking [0,10) and grace's [5,15) cover the point 5"
		)

		const activeAt = query(Ledger).rule((r) =>
			r.match(Account, { id: r.var("acct"), active: r.param("at") }).select("acct")
		)
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

		const pointInParam = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), active: r.var("w") })
				.where(r.pointIn(r.param("t"), r.var("w")))
				.select("acct")
		)
		assert.deepEqual(
			sorted(
				run(pointInParam, { t: 5n }).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings])
		)

		const coversLiteralLeft = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), opened: r.var("t") })
				.where(r.covers(span(0n, 10n), r.var("t")))
				.select("acct")
		)
		assert.deepEqual(
			sorted(
				run(coversLiteralLeft, {}).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings]),
			"covers(span(...), t) — the legal interval-left pointIn, literal lhs tagged by the point sibling"
		)
		const pin: ParamsPin = true
		assert.ok(pin)
	})

	test("allen with a literal mask and with a bound mask param", function allenComparisons() {
		const intersecting = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), active: r.var("w") })
				.where(r.allen(r.var("w"), ALLEN.intersects, span(0n, 12n)))
				.select("acct")
		)
		assert.deepEqual(
			sorted(
				run(intersecting, {}).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings]),
			"[0,10) and [5,15) intersect [0,12); [20,30) and [40,50) are disjoint from it"
		)

		const related = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), active: r.var("w") })
				.where(r.allen(r.var("w"), r.maskParam("rel"), span(0n, 12n)))
				.select("acct")
		)
		type ParamsPin = Expect<Equal<QueryParams<typeof related>, { readonly rel: number }>>
		assert.deepEqual(
			sorted(
				run(related, { rel: ALLEN.intersects }).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaChecking, ids.graceSavings])
		)
		assert.deepEqual(
			sorted(
				run(related, { rel: ALLEN.after }).map(function acct(row) {
					return row.acct
				})
			),
			sorted([ids.adaSavings, ids.kurtChecking]),
			"one prepared query answers any mask question per execution"
		)
		const pin: ParamsPin = true
		assert.ok(pin)
	})

	test("∈-set params match on membership; the empty set matches nothing", function setParams() {
		const namedSet = query(Ledger).rule((r) => r.match(Holder, { id: r.var("h"), name: r.inSet("names") }).select("h"))
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

		const idSet = query(Ledger).rule((r) =>
			r
				.match(Holder, { id: r.var("h") })
				.where(r.eq(r.var("h"), r.inSet("wanted")))
				.select("h")
		)
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
		const eitherKind = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), kind: r.var("k") })
				.where(r.or(r.eq(r.var("k"), "Checking"), r.eq(r.var("k"), "Savings")))
				.select("acct")
		)
		assert.equal(run(eitherKind, {}).length, 4, "the disjunction spans both kinds")
	})

	test("a param value no rule uses never registers — the query executes under its inferred Params", function unusedParam() {
		let ghost: Param<"ghost"> | undefined
		const everyone = query(Ledger).rule((r) => {
			ghost = r.param("ghost")
			return r.match(Holder, { id: r.var("h") }).select("h")
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
				.rule((r) =>
					r
						.match(Account, { id: r.var("acct"), holder: r.var("h") })
						.where(r.eq(r.var("h"), r.param("root")))
						.select("acct", r.count())
				)
				.rule((r) =>
					r
						.match(Account, { id: r.var("acct"), holder: r.var("h"), kind: "Savings" })
						.match(Holder, { id: r.var("h") })
						.where(r.eq(r.var("h"), r.param("root")))
						.select("acct", r.count())
				)
		}
		const first = build()
		const second = build()
		assert.notEqual(first, second, "two constructions are two values")
		assert.deepStrictEqual(lowerQuery(first), lowerQuery(second))
		assert.deepStrictEqual(lowerQuery(first), lowerQuery(first), "lowering is stable per value too")
	})

	test("the engine's prepare accepts every construct the surface can spell (the IR-bijection pin)", function prepareSweep() {
		const constructs: AnyQuery[] = [
			// ne
			query(Ledger).rule((r) =>
				r
					.match(Holder, { id: r.var("h") })
					.where(r.ne(r.var("h"), 1n))
					.select("h")
			),
			// the order roster over an i64 variable
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("acct"), balance: r.var("b") })
					.where(r.le(r.var("b"), 7n))
					.where(r.gt(r.var("b"), 0n))
					.where(r.ge(r.var("b"), 3n))
					.where(r.lt(r.var("b"), 100n))
					.select("acct")
			),
			// the measure as an order side and a projected entry
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("acct"), active: r.var("w") })
					.where(r.lt(r.duration("w"), 100n))
					.select("acct", r.duration("w"))
			),
			// nested and/or trees
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("acct"), kind: r.var("k"), balance: r.var("b") })
					.where(r.or(r.and(r.eq(r.var("k"), "Checking"), r.gt(r.var("b"), 4n)), r.eq(r.var("k"), "Savings")))
					.select("acct")
			),
			// countDistinct (all-aggregate select: empty input yields the empty set)
			query(Ledger).rule((r) => r.match(Account, { holder: r.var("h") }).select(r.countDistinct("h"))),
			// the folds over a variable
			query(Ledger).rule((r) => r.match(Account, { holder: r.var("h"), balance: r.var("b") }).select("h", r.sum("b"))),
			query(Ledger).rule((r) => r.match(Account, { holder: r.var("h"), balance: r.var("b") }).select("h", r.min("b"))),
			query(Ledger).rule((r) => r.match(Account, { holder: r.var("h"), balance: r.var("b") }).select("h", r.max("b"))),
			// the folds over the measure
			query(Ledger).rule((r) =>
				r.match(Account, { holder: r.var("h"), active: r.var("w") }).select("h", r.sum(r.duration("w")))
			),
			// the Arg forms
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("acct"), holder: r.var("h"), balance: r.var("b") })
					.select("h", r.argMax("acct", "b"))
			),
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("acct"), holder: r.var("h"), balance: r.var("b") })
					.select("h", r.argMin("acct", "b"))
			),
			// pack (the coalescing fold)
			query(Ledger).rule((r) => r.match(Account, { holder: r.var("h"), active: r.var("w") }).select("h", r.pack("w"))),
			// literal bindings at every structural kind
			query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), flagged: true }).select("acct")),
			query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), tag: new Uint8Array([1, 2]) }).select("acct")),
			query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), balance: 5n }).select("acct")),
			query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), active: span(0n, 10n) }).select("acct")),
			query(Ledger).rule((r) => r.match(Holder, { id: r.var("h"), name: "ada" }).select("h")),
			query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), kind: "Savings" }).select("acct")),
			// a zero-binding atom is a nonemptiness gate
			query(Ledger).rule((r) =>
				r
					.match(Holder, { id: r.var("h") })
					.match(Parent, {})
					.select("h")
			),
			// params: scalar at a field, at an interval field, and a set at a field
			query(Ledger).rule((r) => r.match(Holder, { id: r.var("h"), name: r.param("n") }).select("h")),
			query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), active: r.param("at") }).select("acct")),
			query(Ledger).rule((r) => r.match(Account, { id: r.var("acct"), kind: r.inSet("kinds") }).select("acct"))
		]
		for (const construct of constructs) {
			accepted(construct)
		}
	})

	test("engine roster refusals surface as typed prepare errors", function rosterError() {
		const argAndFoldMixed = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), holder: r.var("h"), balance: r.var("b") })
				.select("h", r.argMax("acct", "b"), r.sum("b"))
		)
		const prepared = native.dbPrepare(db, lowerQuery(argAndFoldMixed))
		assert.ok(!prepared.ok, "Arg and fold aggregates never mix — the engine's typed rule")
		assert.equal(prepared.kind, "irError")
	})

	test("every rule of a query derives the same head", function headAlignment() {
		assert.throws(function misaligned() {
			query(Ledger)
				.rule((r) => r.match(Holder, { id: r.var("h") }).select("h"))
				.rule((r) => r.match(Account, { id: r.var("acct") }).select("acct"))
		}, /derives the same head/)
	})

	test("a param name keeps one wire shape", function paramShapeConflict() {
		assert.throws(function conflicted() {
			query(Ledger).rule((r) =>
				r
					.match(Holder, { id: r.var("h"), name: r.param("who") })
					.where(r.eq(r.var("h"), r.inSet("who")))
					.select("h")
			)
		}, /one name, one shape/)
	})

	test("execute refuses a missing param, typed", function missingParam() {
		const withParam = query(Ledger).rule((r) =>
			r
				.match(Holder, { id: r.var("h") })
				.where(r.eq(r.var("h"), r.param("root")))
				.select("h")
		)
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
		// A "Holder.id"-class var joined to the "Account.id" generator class — the class-equal
		// join law, a compile error AND a construction refusal (the wall holds for untyped callers too).
		assert.throws(function crossClassJoin() {
			query(Ledger).rule((r) =>
				r
					.match(Holder, { id: r.var("x") })
					// @ts-expect-error — "x" first bound in the "Holder.id" class; Account.id generates "Account.id"
					.match(Account, { id: r.var("x") })
					.select("x")
			)
		}, /joins domain-unequal fields/)

		// The same law through eq: var-to-var unification IS a join — a compile error
		// AND a construction refusal (the runtime twin; the wall holds for untyped
		// callers too, and the engine cannot backstop it — the IR carries no domains).
		assert.throws(function crossClassEq() {
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("a"), holder: r.var("h") })
					// @ts-expect-error — "a" is in the "Account.id" class, "h" in "Holder.id"
					.where(r.eq(r.var("a"), r.var("h")))
					.select("a")
			)
		}, /unifies domain-unequal fields/)

		// ne rides the identical judgment (EqOk covers both ops).
		assert.throws(function crossClassNe() {
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("a"), holder: r.var("h") })
					// @ts-expect-error — ne is the same unification judgment as eq
					.where(r.ne(r.var("a"), r.var("h")))
					.select("a")
			)
		}, /unifies domain-unequal fields/)

		// The positive twins: class-equal eq constructs, and bare pairs with bare.
		const sameClassEq = query(Ledger).rule((r) =>
			r
				.match(Account, { holder: r.var("h") })
				.match(Holder, { id: r.var("h2") })
				.where(r.eq(r.var("h"), r.var("h2")))
				.select("h")
		)
		assert.equal(sameClassEq.data.rules.length, 1)
		const bareBareEq = query(Ledger).rule((r) =>
			r
				.match(Holder, { id: r.var("h"), rank: r.var("z") })
				.match(Account, { opened: r.var("o") })
				.where(r.eq(r.var("z"), r.var("o")))
				.select("h")
		)
		assert.equal(bareBareEq.data.rules.length, 1)

		// An interval var under a non-pointIn comparison — the interval-vs-scalar wall.
		const intervalUnderOrder = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), active: r.var("w") })
				// @ts-expect-error — an interval-typed variable has no order; pointIn/allen are the interval predicates
				.where(r.lt(r.var("w"), 5n))
				.select("acct")
		)
		assert.equal(intervalUnderOrder.data.rules.length, 1)

		const intervalUnderEq = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), active: r.var("w") })
				// @ts-expect-error — eq of an interval variable takes an interval, never a scalar point
				.where(r.eq(r.var("w"), 5n))
				.select("acct")
		)
		assert.equal(intervalUnderEq.data.rules.length, 1)

		// A minting/arithmetic term in a head — unrepresentable: a select entry is a name, the measure, or an aggregate.
		assert.throws(function mintingHead() {
			query(Ledger).rule((r) =>
				r
					.match(Account, { id: r.var("acct"), balance: r.var("b") })
					// @ts-expect-error — no minting or arithmetic term exists in the head vocabulary (the creation quarantine)
					.select(1n + 2n)
			)
		}, /not a select entry/)

		// A param supplied at the wrong type.
		const byName = query(Ledger).rule((r) => r.match(Holder, { id: r.var("h"), name: r.param("who") }).select("h"))
		assert.throws(function wrongParamType() {
			// @ts-expect-error — "who" is string-typed by its binding field
			run(byName, { who: 5n })
		}, /expected string/)

		// A results shape mismatched to the head.
		const everyone = query(Ledger).rule((r) => r.match(Holder, { id: r.var("h") }).select("h"))
		// @ts-expect-error — the result row is typed by the head: h is bigint, never string
		const wrong: { h: string }[] = run(everyone, {})
		assert.equal(wrong.length, 4)

		// A negated atom over an unbound variable — the safety rule as a compile error, and a construction refusal.
		assert.throws(function unsafeNegation() {
			query(Ledger).rule((r) =>
				r
					.match(Holder, { id: r.var("h") })
					// @ts-expect-error — "ghost" is bound by no positive atom of the rule
					.where(r.not(Account, { id: r.var("ghost"), holder: r.var("h") }))
					.select("h")
			)
		}, /ghost/)
	})

	test("THE FOUR JOIN LAWS, r.var spelling: same-class joins+lowers, cross-class refuses at the use site, bare↔bare joins, bare↔classed refuses", function joinLaws() {
		// 1. Same-class join compiles AND lowers (Account.holder and Holder.id share "Holder.id").
		const sameClass = query(Ledger).rule((r) =>
			r
				.match(Account, { id: r.var("acct"), holder: r.var("h") })
				.match(Holder, { id: r.var("h") })
				.select("acct")
		)
		assert.equal(lowerQuery(sameClass).predicates.length, 1)

		// 2. Cross-class pairing fails at the use site (compile) and at construction (runtime twin).
		assert.throws(function crossClass() {
			query(Ledger).rule((r) =>
				r
					.match(Holder, { id: r.var("x") })
					// @ts-expect-error — the use site: Account.id generates its own class
					.match(Account, { id: r.var("x") })
					.select("x")
			)
		}, /joins domain-unequal fields/)

		// 3. Bare pairs with bare: Holder.rank and Account.opened are in no law — the join is legal,
		// lowers, and runs (ada's rank 1 = the opened stamp of her checking account).
		const bareBare = query(Ledger).rule((r) =>
			r
				.match(Holder, { id: r.var("h"), rank: r.var("z") })
				.match(Account, { opened: r.var("z") })
				.select("h")
		)
		assert.deepEqual(
			run(bareBare, {}).map(function h(row) {
				return row.h
			}),
			[ids.ada]
		)

		// 4. Bare↔classed refuses: Account.opened is bare, Account.holder is in "Holder.id".
		assert.throws(function bareClassed() {
			query(Ledger).rule((r) =>
				r
					.match(Account, { opened: r.var("z") })
					// @ts-expect-error — bare pairs only with bare; "Holder.id" is a classed slot
					.match(Account, { holder: r.var("z") })
					.select("z")
			)
		}, /joins domain-unequal fields/)
	})

	test("THE FOUR JOIN LAWS re-pinned through vars() — one lowering, two entry flavors", function joinLawsViaVars() {
		const sameClass = query(Ledger).rule((r) => {
			const { acct, h } = r.vars("acct", "h")
			return r.match(Account, { id: acct, holder: h }).match(Holder, { id: h }).select("acct")
		})
		assert.equal(lowerQuery(sameClass).predicates.length, 1)

		assert.throws(function crossClass() {
			query(Ledger).rule((r) => {
				const { x } = r.vars("x")
				return (
					r
						.match(Holder, { id: x })
						// @ts-expect-error — the vars-minted name meets the same use-site wall
						.match(Account, { id: x })
						.select("x")
				)
			})
		}, /joins domain-unequal fields/)

		const bareBare = query(Ledger).rule((r) => {
			const { h, z } = r.vars("h", "z")
			return r.match(Holder, { id: h, rank: z }).match(Account, { opened: z }).select("h")
		})
		assert.deepEqual(
			run(bareBare, {}).map(function h(row) {
				return row.h
			}),
			[ids.ada]
		)

		assert.throws(function bareClassed() {
			query(Ledger).rule((r) => {
				const { z } = r.vars("z")
				return (
					r
						.match(Account, { opened: z })
						// @ts-expect-error — bare pairs only with bare, vars-minted or not
						.match(Account, { holder: z })
						.select("z")
				)
			})
		}, /joins domain-unequal fields/)
	})

	test("RECURSION FENCES: the cut is typed and the quarantine unwritable", function recursionFences() {
		// Mutual recursion is unwritable — a recursive rule's idb accepts only the rec itself.
		assert.throws(function mutualRecursion() {
			program(Ledger, (p) => {
				const a = p.rec("a")
				const b = p.rec("b")
				a.rule((r) =>
					r
						.match(Parent, { child: r.var("c"), parent: r.var("m") })
						// @ts-expect-error — the self-recursion-only cut: rec "a" cannot reference rec "b"
						.idb(b, r.var("m"))
						.select("c")
				)
				b.rule((r) => r.match(Holder, { id: r.var("h") }).select("h"))
				return p.output((r) =>
					r
						.match(Holder, { id: r.var("h") })
						.idb(b, r.var("h"))
						.select("h")
				)
			})
		}, /self-recursion-only cut/)

		// An aggregate (or the measure) in a recursive head is unwritable — the strata quarantine.
		assert.throws(function aggregateThroughCycle() {
			program(Ledger, (p) => {
				const reach = p.rec("reach")
				reach.rule((r) =>
					r
						.match(Account, { holder: r.var("h"), balance: r.var("b") })
						// @ts-expect-error — a recursive head projects bound variable names only
						.select(r.sum("b"))
				)
				return p.output((r) =>
					r
						.match(Holder, { id: r.var("h") })
						.idb(reach, r.var("h"))
						.select("h")
				)
			})
		}, /NAMES only/)

		// An idb variable no relation atom binds — an idb atom is a join position.
		assert.throws(function unboundIdbVar() {
			program(Ledger, (p) => {
				const reach = p.rec("reach")
				reach.rule((r) => r.match(Holder, { id: r.var("h") }).select("h"))
				return p.output((r) =>
					r
						.match(Holder, { id: r.var("h") })
						// @ts-expect-error — "ghost" is bound by no relation atom of the rule
						.idb(reach, r.var("ghost"))
						.select("h")
				)
			})
		}, /ghost/)

		// idb in a plain query is a construction error (and the scope carries no idb to spell).
		const plain = query(Ledger).rule((r) => r.match(Holder, { id: r.var("h") }).select("h"))
		assert.equal("idb" in plain.data, false, "a plain query's data carries rules, not predicates")
	})
})
