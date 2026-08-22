/**
 * The KERN-01 self-check: reference-identity variables, the `v` mint, and
 * `find`. Pins the atomic src kernel flip before the wider suites are
 * ported — variables minted by `v` are fresh objects joined by REFERENCE
 * (reuse is the join, name collision is unrepresentable), the head is a
 * `find` record whose keys name the answer columns, params stay
 * string-named, and SEMANTIC PARITY holds (the same dense per-rule first-use
 * VarId assignment, the wire untouched). Runs against a real durable store
 * through the `Db` runtime.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, test } from "node:test"
import { closed } from "#closed.ts"
import { Db } from "#db.ts"
import { on } from "#face.ts"
import { interval, str, u64 } from "#fields.ts"
import type { QueryParams, QueryRow, QueryRuleScope } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained } from "#statements.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-destructure-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Kind = closed("Kind", ["Checking", "Savings"])
const Holder = relation("Holder", { id: u64.fresh, name: str, rank: u64 })
const Account = relation("Account", { id: u64.fresh, holder: u64, kind: Kind.id, window: interval(u64) })
const Parent = relation("Parent", { child: u64, parent: u64 })

const Foreign = relation("Foreign", { id: u64.fresh, weight: u64 })

const Theory = schema("T", { Kind, Holder, Account, Parent }, [
	contained(on(Account, "holder"), on(Holder, "id")),
	contained(on(Account, "kind"), on(Kind, "id")),
	contained(on(Parent, "child"), on(Holder, "id")),
	contained(on(Parent, "parent"), on(Holder, "id"))
])

type Rels = (typeof Theory)["relations"]

const HOLDER_ID = 1
const ACCOUNT_ID = 2

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

let db: Db<Rels>

before(async function seed() {
	db = accepted(await Db.create(storeDir, Theory))
	const result = db.write(function delta(tx) {
		put(tx, Holder, { id: 1n, name: "ada", rank: 1n })
		put(tx, Holder, { id: 2n, name: "grace", rank: 2n })
		put(tx, Holder, { id: 3n, name: "kurt", rank: 3n })
		put(tx, Account, { id: 10n, holder: 1n, kind: "Checking", window: { start: 0n, end: 10n } })
		put(tx, Account, { id: 11n, holder: 1n, kind: "Savings", window: { start: 20n, end: 30n } })
		put(tx, Account, { id: 12n, holder: 2n, kind: "Savings", window: { start: 5n, end: 15n } })
		put(tx, Parent, { child: 2n, parent: 1n })
		put(tx, Parent, { child: 3n, parent: 2n })
	})
	assert.equal(result.tag, "accepted", "the seed commit lands")
})

test("v() mints a fresh batch per call — two batches are two variables in one rule (two VarIds in the IR)", function twoBatches() {
	const q = query(Theory).rule(function rule(r) {
		const a = v(Holder)
		const b = v(Holder)
		return r.match(Holder, { id: a.id }).match(Account, { holder: b.id }).find({ x: a.id, y: b.id })
	})
	const ir = lowerQuery(q)
	const rule = ir.rules[0]
	assert.ok(rule !== undefined)
	assert.deepEqual(rule.finds, [
		{ kind: "var", var: 0 },
		{ kind: "var", var: 1 }
	])
	assert.deepEqual(rule.atoms[0]?.bindings, [[0, { kind: "var", var: 0 }]])
	assert.deepEqual(rule.atoms[1]?.bindings, [[1, { kind: "var", var: 1 }]])
})

test("reusing one var reference across binding positions IS the join — one VarId, first-use order", function reuseJoins() {
	const q = query(Theory).rule(function rule(r) {
		const h = v(Holder)
		return r.match(Holder, { id: h.id }).match(Account, { holder: h.id }).find({ h: h.id })
	})
	assert.deepEqual(lowerQuery(q), {
		kind: "cq",
		interiors: [],
		head: [{ kind: "var" }],
		rules: [
			{
				finds: [{ kind: "var", var: 0 }],
				atoms: [
					{ source: { kind: "edb", relation: HOLDER_ID }, bindings: [[0, { kind: "var", var: 0 }]] },
					{ source: { kind: "edb", relation: ACCOUNT_ID }, bindings: [[1, { kind: "var", var: 0 }]] }
				],
				negated: [],
				conditions: []
			}
		]
	})
})

test("a class-mismatched reference reuse throws at the binding position, naming both slots", function crossClassReuse() {
	assert.throws(function build() {
		query(Theory).rule(function rule(r) {
			const h = v(Holder)
			return (
				r
					.match(Holder, { id: h.id })
					// @ts-expect-error — a class-mismatched reference reuse is rejected at the position
					.match(Holder, { rank: h.id })
					.find({ x: h.id })
			)
		})
	}, /a var joins only class-equal slots/)
})

test("find keys name the answer columns — renames are real", function renames() {
	const rq = query(Theory).rule(function rule(r) {
		const h = v(Holder)
		return r.match(Holder, { id: h.id }).find({ renamed: h.id })
	})
	type Pin = Expect<Equal<QueryRow<typeof rq>, { readonly renamed: bigint }>>
	const prepared = db.prepare(rq)
	const rows = db.read((i) => i.execute(prepared, {}))
	assert.ok(rows.length > 0)
	for (const row of rows) {
		assert.deepEqual(Object.keys(row), ["renamed"])
		assert.equal(typeof row.renamed, "bigint")
	}
	const pin: Pin = true
	assert.ok(pin)
})

test("aggregates ride find over var references: count, sum, min, max, pack", function aggregates() {
	const countQ = query(Theory).rule(function rule(r) {
		const a = v(Account)
		return r.match(Account, { holder: a.holder }).find({ holder: a.holder, n: r.count() })
	})
	const sumQ = query(Theory).rule(function rule(r) {
		const h = v(Holder)
		return r.match(Holder, { id: h.id, rank: h.rank }).find({ total: r.sum(h.rank) })
	})
	const minQ = query(Theory).rule(function rule(r) {
		const h = v(Holder)
		return r.match(Holder, { id: h.id, rank: h.rank }).find({ lo: r.min(h.rank) })
	})
	const maxQ = query(Theory).rule(function rule(r) {
		const h = v(Holder)
		return r.match(Holder, { id: h.id, rank: h.rank }).find({ hi: r.max(h.rank) })
	})
	const packQ = query(Theory).rule(function rule(r) {
		const a = v(Account)
		return r.match(Account, { holder: a.holder, window: a.window }).find({ merged: r.pack(a.window) })
	})
	assert.ok(db.prepare(countQ))
	assert.ok(db.prepare(sumQ))
	assert.ok(db.prepare(minQ))
	assert.ok(db.prepare(maxQ))
	assert.ok(db.prepare(packQ))
})

test("the recursive query ports: rec find + named interior record lower and prepare", function recPorts() {
	const reachable = query(Theory)
		.reach("reach", {
			base: [
				function rule(r) {
					const n = v(Holder)
					return r
						.match(Holder, { id: n.id })
						.where(r.eq(n.id, r.param("root")))
						.find({ c: n.id })
				}
			],
			rec: [
				function rule(r) {
					const e = v(Parent)
					return r
						.match(Parent, { child: e.child, parent: e.parent })
						.interior("reach", { c: e.parent })
						.find({ c: e.child })
				}
			]
		})
		.rule(function rule(r) {
			const h = v(Holder)
			return r.match(Holder, { id: h.id }).interior("reach", { c: h.id }).find({ c: h.id })
		})
	type Pin = Expect<Equal<QueryParams<typeof reachable>, { readonly root: bigint }>>
	const ir = lowerQuery(reachable)
	assert.equal(ir.interiors.length, 0)
	assert.equal(ir.kind, "reach")
	const prepared = db.prepare(reachable)
	const rows = db.read((i) => i.execute(prepared, { root: 1n }))
	assert.deepEqual(
		sorted(
			rows.map(function c(row) {
				return row.c
			})
		),
		sorted([1n, 2n, 3n])
	)
	const pin: Pin = true
	assert.ok(pin)
})

test("negation binds nothing: an unbound reference in not() is refused; a bound one joins class-equal", function negation() {
	assert.throws(function unbound() {
		query(Theory).rule(function rule(r) {
			const h = v(Holder)
			const other = v(Holder)
			return r
				.match(Holder, { id: h.id })
				.where(r.not(Parent, { child: other.id }))
				.find({ x: h.id })
		})
	}, /a negated atom binds nothing/)
	const boundQ = query(Theory).rule(function rule(r) {
		const h = v(Holder)
		return r
			.match(Holder, { id: h.id })
			.where(r.not(Parent, { child: h.id }))
			.find({ x: h.id })
	})
	const prepared = db.prepare(boundQ)
	assert.ok(prepared)
})

test("a var minted from a relation the schema does not declare is refused, typed", function foreignMint() {
	assert.throws(function build() {
		query(Theory).rule(function rule(r) {
			const f = v(Foreign)
			return r.match(Holder, { rank: f.id }).find({ x: f.id })
		})
	}, /does not declare/)
})

test("params stay string-named: param/inSet register by first use and execute under the inferred Params object", function params() {
	const paramQ = query(Theory).rule(function rule(r) {
		const h = v(Holder)
		return r
			.match(Holder, { id: h.id, rank: h.rank })
			.where(r.eq(h.rank, r.param("minRank")))
			.find({ id: h.id })
	})
	type Pin = Expect<Equal<QueryParams<typeof paramQ>, { readonly minRank: bigint }>>
	const pin: Pin = true
	assert.ok(pin)
	assert.deepEqual(
		paramQ.data.params.map(function nameOf(p) {
			return p.name
		}),
		["minRank"]
	)
	const preparedP = db.prepare(paramQ)
	assert.deepEqual(
		sorted(
			db
				.read((i) => i.execute(preparedP, { minRank: 2n }))
				.map(function id(row) {
					return row.id
				})
		),
		sorted([2n])
	)

	const setQ = query(Theory).rule(function rule(r) {
		const a = v(Account)
		return r
			.match(Account, { id: a.id })
			.where(r.eq(a.id, r.inSet("acctIds")))
			.find({ id: a.id })
	})
	assert.deepEqual(
		setQ.data.params.map(function nameOf(p) {
			return p.name
		}),
		["acctIds"]
	)
	const preparedS = db.prepare(setQ)
	assert.deepEqual(
		sorted(
			db
				.read((i) => i.execute(preparedS, { acctIds: [10n, 11n] }))
				.map(function id(row) {
					return row.id
				})
		),
		sorted([10n, 11n])
	)
})

test("the same query built twice from fresh mints lowers to deeply-equal IR", function stable() {
	function build() {
		return query(Theory).rule(function rule(r) {
			const acct = v(Account)
			const h = v(Holder)
			return r
				.match(Account, { id: acct.id, holder: acct.holder })
				.match(Holder, { id: acct.holder, rank: h.rank })
				.where(r.eq(acct.holder, r.param("root")))
				.find({ account: acct.id, owner: acct.holder })
		})
	}
	assert.deepEqual(lowerQuery(build()), lowerQuery(build()))
})

/**
 * The compile-probe block — never executed; every `@ts-expect-error` marks a
 * line that must fail typechecking on the 0.6.0 surface. Referenced (never
 * called) so the module keeps it live.
 */
function compileProbes(r: QueryRuleScope<Rels, (typeof Theory)["classes"]>): void {
	const h = v(Holder)
	// @ts-expect-error — r.var died with 0.6.0
	r.var
	const chain = r.match(Holder, { id: h.id })
	// @ts-expect-error — select died into find
	chain.select
	// @ts-expect-error — a class-mismatched reference reuse is rejected at the position
	r.match(Holder, { rank: h.id })
}

test("the 0.6.0 surface refuses the dead spellings (compile probes present)", function probes() {
	assert.equal(typeof compileProbes, "function")
})
