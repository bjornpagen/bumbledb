import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, describe, test } from "node:test"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import type { DbHandle } from "#native.ts"
import { native } from "#native.ts"
import type { Query, QueryParams, QueryRow } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import type { ParamsRecord } from "#query/scope.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-named-answers-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Sev = closed(
	"Sev",
	["Info", "Warn", "Crit", "Fatal"],
	{ rank: u64 },
	{
		Info: { rank: 1n },
		Warn: { rank: 2n },
		Crit: { rank: 3n },
		Fatal: { rank: 4n }
	}
)

const Priority = closed("Priority", ["Crit", "Low"])
const Incident = relation("Incident", { id: u64.fresh, sev: Sev.id, pri: Priority.id })
const Edge = relation("Edge", { src: u64, dst: u64 })

const Oncall = schema("Oncall", { Sev, Priority, Incident, Edge }, [
	contained(on(Incident, "sev"), on(Sev, "id")),
	contained(on(Incident, "pri"), on(Priority, "id")),
	contained(on(Edge, "src"), on(Incident, "id")),
	contained(on(Edge, "dst"), on(Incident, "id"))
])

type Rels = (typeof Oncall)["relations"]

const INCIDENT_ID = 2
const EDGE_ID = 3

/** The ban's pinned message fragment — the data-model ruling, cited verbatim at every refusal point. */
const BAN = /declaration order is an accident, not semantics: vocabularies do not order/

function sortedPairs(rows: ReadonlyArray<{ readonly n: bigint; readonly s: string }>): Array<[bigint, string]> {
	return rows
		.map(function pair(row): [bigint, string] {
			return [row.n, row.s]
		})
		.sort(function compare(a, b) {
			if (a[0] !== b[0]) {
				return a[0] < b[0] ? -1 : 1
			}
			if (a[1] !== b[1]) {
				return a[1] < b[1] ? -1 : 1
			}
			return 0
		})
}

describe("answer rows arrive named + the orderable ban", function suite() {
	let db: DbHandle

	before(async function seed() {
		const created = await native.dbCreate(storeDir, lower(Oncall))
		assert.equal(created.tag, "accepted", "the store admits")
		db = created.db

		const committed = native.dbWrite(db, function write(tx) {
			native.txInsert(tx, INCIDENT_ID, 1n, [1n, 0n, 1n])
			native.txInsert(tx, INCIDENT_ID, 1n, [2n, 1n, 1n])
			native.txInsert(tx, INCIDENT_ID, 1n, [3n, 2n, 0n])
			native.txInsert(tx, INCIDENT_ID, 1n, [4n, 3n, 0n])
			native.txInsert(tx, EDGE_ID, 1n, [1n, 2n])
			native.txInsert(tx, EDGE_ID, 1n, [2n, 3n])
			return true
		})
		assert.equal(committed.tag, "accepted", "the seed commit lands")
	})

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

	function runRaw<Row, P extends ParamsRecord>(q: Query<Rels, Row, P>, params: P): readonly (readonly unknown[])[] {
		const prepared = native.dbPrepare(db, lowerQuery(q))
		if (!prepared.ok) {
			assert.fail(`engine prepare refused: ${prepared.message}`)
		}
		const rows = native.dbRead(db, function read(instance, _witness) {
			return native.preparedExecute(prepared.prepared, instance, wireParams(q.data.params, params))
		})
		native.preparedClose(prepared.prepared)
		return rows
	}

	test("a closed select column decodes to handle NAMES — strict-equality members of the roster, and the 0.3.0 twin's rows modulo the translation", function namedRows() {
		const all = query(Oncall).rule((r) => {
			const { id, sev } = v(Incident)
			return r.match(Incident, { id, sev }).find({ n: id, s: sev })
		})

		type RowPin = Expect<
			Equal<QueryRow<typeof all>, { readonly n: bigint; readonly s: "Info" | "Warn" | "Crit" | "Fatal" }>
		>
		const rows = run(all, {})
		for (const row of rows) {
			assert.equal(typeof row.s, "string", "the runtime value is the handle name, not a bigint")
			assert.ok(
				Sev.id.closed.handles.some(function strictMember(handle) {
					return handle === row.s
				}),
				`"${row.s}" is a roster member by strict equality`
			)
		}
		assert.deepEqual(sortedPairs(rows), [
			[1n, "Info"],
			[2n, "Warn"],
			[3n, "Crit"],
			[4n, "Fatal"]
		])

		// the decoded rows modulo exactly the id → name translation — the

		const twin = runRaw(all, {}).map(function translate(raw): [bigint, string] {
			const [n, s] = raw
			if (typeof n !== "bigint" || typeof s !== "bigint") {
				assert.fail("the raw seam carries positional bigints")
			}
			const handle = Sev.id.closed.handles[Number(s)]
			if (handle === undefined) {
				assert.fail(`raw id ${s} is outside the roster`)
			}
			return [n, handle]
		})
		assert.deepEqual(
			sortedPairs(rows),
			twin.sort(function compare(a, b) {
				return a[0] < b[0] ? -1 : 1
			})
		)
		const pins: [RowPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("the rec-head plumb: a main rule's interior-joined closed column decodes named (the descriptor survives the head)", function recHead() {
		const reach = query(Oncall)
			.reach("seen", {
				base: [
					(r) => {
						const { id, sev } = v(Incident)
						return r.match(Incident, { id, sev }).where(r.eq(id, 1n)).find({ n: id, s: sev })
					}
				],
				rec: [
					(r) => {
						const e = v(Edge)
						const near = v(Incident)
						const far = v(Incident)
						return r
							.match(Edge, { src: e.src, dst: e.dst })
							.match(Incident, { id: e.dst, sev: near.sev })
							.match(Incident, { id: e.src, sev: far.sev })
							.interior("seen", { n: e.src, s: far.sev })
							.find({ n: e.dst, s: near.sev })
					}
				]
			})
			.rule((r) => {
				const { id, sev } = v(Incident)
				return r.match(Incident, { id, sev }).interior("seen", { n: id, s: sev }).find({ n: id, s: sev })
			})
		type RecRowPin = Expect<
			Equal<QueryRow<typeof reach>, { readonly n: bigint; readonly s: "Info" | "Warn" | "Crit" | "Fatal" }>
		>
		assert.deepEqual(sortedPairs(run(reach, {})), [
			[1n, "Info"],
			[2n, "Warn"],
			[3n, "Crit"]
		])
		const pins: [RecRowPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("COUNTING IS NOT ORDERING: count over closed-atom-filtered rules stays legal", function countingStays() {
		const paged = query(Oncall).rule((r) => {
			const { id } = v(Incident)
			return r.match(Incident, { id, sev: ["Crit", "Fatal"] }).find({ count: r.count() })
		})
		assert.deepEqual(run(paged, {}), [{ count: 2n }])

		const totalRank = query(Oncall).rule((r) => {
			const { id, rank } = v(Sev)
			return r.match(Sev, { id, rank }).find({ k: r.sum(rank) })
		})
		assert.deepEqual(run(totalRank, {}), [{ k: 10n }])
	})

	test("the orderable ban, comparison tier: lt/ge and the pointIn point side refuse closed-bound vars (both tiers)", function comparisonBan() {
		assert.throws(function ltClosed() {
			query(Oncall).rule((r) => {
				const { id, sev } = v(Incident)
				return (
					r
						.match(Incident, { id, sev })
						// @ts-expect-error — a closed-bound var left the orderable set: vocabularies do not order
						.where(r.lt(sev, sev))
						.find({ n: id })
				)
			})
		}, BAN)
		assert.throws(function geClosed() {
			query(Oncall).rule((r) => {
				const { id, sev } = v(Incident)
				return (
					r
						.match(Incident, { id, sev })
						// @ts-expect-error — the whole order roster refuses a closed-bound var, ge included
						.where(r.ge(sev, 1n))
						.find({ n: id })
				)
			})
		}, BAN)
		assert.throws(function pointInClosed() {
			query(Oncall).rule((r) => {
				const { id, sev } = v(Incident)
				return (
					r
						.match(Incident, { id, sev })
						// @ts-expect-error — a closed-bound var is no point: point membership is an order comparison over the element domain
						.where(r.pointIn(sev, { start: 0n, end: 10n }))
						.find({ n: id })
				)
			})
		}, BAN)
	})

	test("the orderable ban, fold tier: sum/max over a closed column refuse (both tiers)", function foldBan() {
		assert.throws(function sumClosed() {
			query(Oncall).rule((r) => {
				const { sev } = v(Incident)
				return (
					r
						.match(Incident, { sev })
						// @ts-expect-error — a fold over a closed column orders ids: banned
						.find({ s: r.sum(sev) })
				)
			})
		}, BAN)
		assert.throws(function maxClosed() {
			query(Oncall).rule((r) => {
				const { sev } = v(Incident)
				return (
					r
						.match(Incident, { sev })
						// @ts-expect-error — max over a closed column is the same accident
						.find({ s: r.max(sev) })
				)
			})
		}, BAN)
	})

	test("the orderable ban, param tier: an order-comparison param anchored at a closed field is unsuppliable and refused", function paramBan() {
		function buildOrderedParam() {
			return query(Oncall).rule((r) => {
				const { id } = v(Incident)
				return r
					.match(Incident, { id, sev: r.param("p") })
					.where(r.lt(id, r.param("p")))
					.find({ n: id })
			})
		}

		// refused at the comparison constructor) claims bigint, and the

		// runtime refusal is the registry's one-domain wall: an order use

		type OrderedParams = QueryParams<ReturnType<typeof buildOrderedParam>>
		type ParamNeverPin = Expect<Equal<OrderedParams["p"], never>>
		assert.throws(
			buildOrderedParam,
			/query param p is anchored at a Sev reference and at a non-closed position — a closed-anchored param translates handle names through ONE roster/
		)
		const pins: [ParamNeverPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("one answer column decodes through one roster: a union head disagreeing on the closed slice is refused pointed", function headAgreement() {
		assert.throws(function twoRosters() {
			query(Oncall)
				.rule((r) => {
					const { id, sev } = v(Incident)
					return r.match(Incident, { id, sev }).find({ n: id, k: sev })
				})
				.rule((r) => {
					const { id, pri } = v(Incident)
					return r.match(Incident, { id, pri }).find({ n: id, k: pri })
				})
		}, /the head column k is a Sev reference in rule 0 but a Priority reference in rule 1 \(one column decodes through one roster\)/)
		assert.throws(function closedAgainstBare() {
			query(Oncall)
				.rule((r) => {
					const { sev } = v(Incident)
					return r.match(Incident, { sev }).find({ k: sev })
				})
				.rule((r) => {
					const { id } = v(Incident)
					return r.match(Incident, { id }).find({ k: id })
				})
		}, /the head column k is a Sev reference in rule 0 but a bare value in rule 1/)
	})

	test("an out-of-roster id on answer decode throws pointed through the marshal's ONE bijection (shared with fact decode)", function outsideRoster() {
		const svar = v(Incident).sev
		assert.throws(function nineIsOutside() {
			decodeAnswers([{ name: "s", entry: { kind: "var", over: svar }, closed: Sev.id.closed, slot: undefined }], [[9n]])
		}, /query answer column s: id 9 is outside the Sev roster \(Info, Warn, Crit, Fatal\) — the column types Sev but no law pins it — a containment statement is the missing piece/)
	})
})
