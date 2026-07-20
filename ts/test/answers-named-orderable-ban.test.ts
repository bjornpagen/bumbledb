/**
 * PRD-H4 probes: answer rows arrive NAMED, and closed fields exit the
 * orderable/foldable set. A SELECT column bound at a closed-referencing
 * field decodes its u64 row ids back to handle NAMES through the marshal's
 * ONE bijection (`handleOf` — the same read half every fact decode rides),
 * so `db.execute` rows agree with scans, gets, and violation records: the
 * string IS the value at the TS surface, the engine keeps ids. Pinned
 * here: the runtime rows (strict-equality against the roster, and
 * value-for-value the 0.3.0 bigint twin modulo the translation — the raw
 * positional rows re-decoded by hand); the rec-head plumb (an output rule's
 * idb-joined closed column decodes named — every idb var is EDB-bound in
 * its own rule, so the descriptor always survives the head); the
 * Arg-carried closed payload (named too); the out-of-roster pointed throw
 * (shared with H2's fact decode — one bijection, two call sites); COUNTING
 * IS NOT ORDERING (`count`/`countDistinct` over closed-atom-filtered rules
 * stay legal, and a closed vocabulary's ordinary payload column still
 * folds); and THE ORDERABLE BAN, two tiers at every position — `lt` (and
 * the order roster), the `pointIn` point side, `sum`/`max` folds, the
 * `argMax` key, and an order-comparison param anchored at a closed field —
 * each `@ts-expect-error` real, each construction refusal pinned by the
 * data-model ruling's fragment ("declaration order is an accident, not
 * semantics: vocabularies do not order",
 * `docs/architecture/10-data-model.md` § orderability), plus the head
 * agreement wall: one answer column decodes through one roster.
 */

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
import { program } from "#query/predicate.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import type { ParamsRecord } from "#query/scope.ts"
import { v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
type Expect<T extends true> = T extends true ? true : never

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-named-answers-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Sev = closed(
	"Sev",
	{ rank: u64 },
	{
		Info: { rank: 1n },
		Warn: { rank: 2n },
		Crit: { rank: 3n },
		Fatal: { rank: 4n }
	}
)
/** A second vocabulary sharing a handle name — the one-column-one-roster wall's witness. */
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

/** Relation ids = record declaration order (the law `lowerQuery` rides). */
const INCIDENT_ID = 2
const EDGE_ID = 3

/** The ban's pinned message fragment — the data-model ruling, cited verbatim at every refusal point. */
const BAN = /declaration order is an accident, not semantics: vocabularies do not order/

/** Sorts answer rows (incident id, severity handle NAME) for a set-equality comparison (answers are sets; the host sorts). */
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

	before(function seed() {
		const created = native.dbCreate(storeDir, lower(Oncall))
		assert.ok(created.ok, "the store admits")
		db = created.db
		// The native seam is RAW: closed cells are declaration-order row ids
		// (Sev: Info 0, Warn 1, Crit 2, Fatal 3; Priority: Crit 0, Low 1) —
		// the name↔id bijection is the SDK marshal's, above this seam.
		const tx = native.dbWriteBegin(db)
		native.txInsert(tx, INCIDENT_ID, [1n, 0n, 1n])
		native.txInsert(tx, INCIDENT_ID, [2n, 1n, 1n])
		native.txInsert(tx, INCIDENT_ID, [3n, 2n, 0n])
		native.txInsert(tx, INCIDENT_ID, [4n, 3n, 0n])
		native.txInsert(tx, EDGE_ID, [1n, 2n])
		native.txInsert(tx, EDGE_ID, [2n, 3n])
		const committed = native.txCommit(tx)
		assert.ok(committed.ok, "the seed commit lands")
	})

	/** The typed execute seam — exactly the shape the `Db` runtime consumes. */
	function run<Row, P extends ParamsRecord>(q: Query<Rels, Row, P>, params: P): Row[] {
		const prepared = native.dbPrepare(db, lowerQuery(q))
		if (!prepared.ok) {
			assert.fail(`engine prepare refused: ${prepared.message}`)
		}
		const snap = native.dbSnapshot(db)
		const rows = native.preparedExecute(prepared.prepared, snap, wireParams(q.data.params, params))
		native.snapshotClose(snap)
		native.preparedClose(prepared.prepared)
		return decodeAnswers<Row>(q.data.finds, rows)
	}

	/** The RAW positional rows of a query — the 0.3.0 twin's view (bigint ids, undecoded). */
	function runRaw<Row, P extends ParamsRecord>(q: Query<Rels, Row, P>, params: P): readonly (readonly unknown[])[] {
		const prepared = native.dbPrepare(db, lowerQuery(q))
		if (!prepared.ok) {
			assert.fail(`engine prepare refused: ${prepared.message}`)
		}
		const snap = native.dbSnapshot(db)
		const rows = native.preparedExecute(prepared.prepared, snap, wireParams(q.data.params, params))
		native.snapshotClose(snap)
		native.preparedClose(prepared.prepared)
		return rows
	}

	test("a closed select column decodes to handle NAMES — strict-equality members of the roster, and the 0.3.0 twin's rows modulo the translation", function namedRows() {
		const all = query(Oncall).rule((r) => {
			const { id, sev } = v(Incident)
			return r.match(Incident, { id, sev }).find({ n: id, s: sev })
		})
		// H1's claim (the precise union) now carries H4's runtime twin: the VALUE is the string.
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
		// The SAME query's raw positional rows (the 0.3.0 bigint view) match
		// the decoded rows modulo exactly the id → name translation — the
		// wire program never moved, only the read seam speaks names now.
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

	test("the rec-head plumb: an output rule's idb-joined closed column decodes named (the descriptor survives the head)", function recHead() {
		// Every idb var is EDB-bound in its OWN rule (an idb atom is a join
		// position, the boundness law), so the output rule's `varFields`
		// always carries the closed descriptor — the plumb succeeds through
		// recursion outputs by construction, no bigint limitation remains.
		const reach = program(Oncall, (p) => {
			const seen = p.rec("seen")
			const seeded = seen
				.rule((r) => {
					const { id, sev } = v(Incident)
					return r
						.match(Incident, { id, sev })
						.where(r.eq(id, 1n))
						.find({ n: id, s: sev })
				})
				.rule((r) => {
					const e = v(Edge)
					const near = v(Incident)
					const far = v(Incident)
					return r
						.match(Edge, { src: e.src, dst: e.dst })
						.match(Incident, { id: e.dst, sev: near.sev })
						.match(Incident, { id: e.src, sev: far.sev })
						.idb(seen, { n: e.src, s: far.sev })
						.find({ n: e.dst, s: near.sev })
				})
			return p.output((r) => {
				const { id, sev } = v(Incident)
				return r.match(Incident, { id, sev }).idb(seeded, { n: id, s: sev }).find({ n: id, s: sev })
			})
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

	test("an Arg-carried closed payload decodes named too (the key orders, the carried value only rides)", function argCarried() {
		const topSev = query(Oncall).rule((r) => {
			const { id, sev } = v(Incident)
			return r.match(Incident, { id, sev }).find({ s: r.argMax(sev, id) })
		})
		type ArgRowPin = Expect<Equal<QueryRow<typeof topSev>, { readonly s: "Info" | "Warn" | "Crit" | "Fatal" }>>
		assert.deepEqual(run(topSev, {}), [{ s: "Fatal" }], "incident 4 is the max key; its severity arrives named")
		const pins: [ArgRowPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("COUNTING IS NOT ORDERING: count over closed-atom-filtered rules and countDistinct over the closed var stay legal", function countingStays() {
		const paged = query(Oncall).rule((r) => {
			const { id } = v(Incident)
			return r.match(Incident, { id, sev: ["Crit", "Fatal"] }).find({ count: r.count() })
		})
		assert.deepEqual(run(paged, {}), [{ count: 2n }])
		const distinct = query(Oncall).rule((r) => {
			const { sev } = v(Incident)
			return r.match(Incident, { sev }).find({ s: r.countDistinct(sev) })
		})
		assert.deepEqual(run(distinct, {}), [{ s: 4n }])
		// A closed vocabulary's ORDINARY payload column still folds — the ban
		// covers the reference id, never the payload's own structural type.
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

	test("the orderable ban, fold tier: sum/max over a closed column and the argMax key refuse (both tiers)", function foldBan() {
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
		assert.throws(function argMaxClosedKey() {
			query(Oncall).rule((r) => {
				const { id, sev } = v(Incident)
				return (
					r
						.match(Incident, { id, sev })
						// @ts-expect-error — the argMax KEY must be orderable; a closed key is banned (the carried value may be closed)
						.find({ n: r.argMax(id, sev) })
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
					.where(r.lt(r.param("p"), 2n))
					.find({ n: id })
			})
		}
		// The compile-FAIL is the params object itself: the closed anchor
		// claims the handle union, the order use claims bigint, and the
		// intersection is never — no value can be supplied for p.
		type OrderedParams = QueryParams<ReturnType<typeof buildOrderedParam>>
		type ParamNeverPin = Expect<Equal<OrderedParams["p"], never>>
		assert.throws(buildOrderedParam, BAN)
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
		}, /the answer column k is a Sev reference in rule 0 but a Priority reference in rule 1 \(one column decodes through one roster\)/)
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
		}, /the answer column k is a Sev reference in rule 0 but a bare value in rule 1/)
	})

	test("an out-of-roster id on answer decode throws pointed through the marshal's ONE bijection (shared with fact decode)", function outsideRoster() {
		// The FindColumn is hand-built: decode reads only `name` and `closed`,
		// but the entry's `over` is now a variable REFERENCE (identity edition),
		// so we mint one over the closed column to satisfy the type.
		const svar = v(Incident).sev
		assert.throws(function nineIsOutside() {
			decodeAnswers(
				[{ name: "s", entry: { kind: "var", over: svar }, closed: Sev.id.closed, slot: undefined }],
				[[9n]]
			)
		}, /query answer column s: id 9 is outside the Sev roster \(Info, Warn, Crit, Fatal\) — the column types Sev but no law pins it — a containment statement is the missing piece/)
	})
})
