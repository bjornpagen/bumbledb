import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, before, describe, test } from "node:test"
import { closed } from "#closed.ts"
import { on } from "#face.ts"
import { bool, u64 } from "#fields.ts"
import { lower } from "#lower.ts"
import type { DbHandle } from "#native.ts"
import { native } from "#native.ts"
import type { Query, QueryRow } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import { type ParamsRecord, v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained, key } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-psi-atoms-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Sev = closed(
	"Sev",
	{ pages: bool, rank: u64 },
	{
		Info: { pages: false, rank: 1n },
		Warn: { pages: false, rank: 2n },
		Crit: { pages: true, rank: 3n },
		Fatal: { pages: true, rank: 4n }
	}
)
const Incident = relation("Incident", { id: u64.fresh, sev: Sev.id })
const Escalation = relation("Escalation", { incident: u64, sev: Sev.id })

const Oncall = schema("Oncall", { Sev, Incident, Escalation }, [
	contained(on(Incident, "sev"), on(Sev, "id")),
	contained(on(Escalation, "incident"), on(Incident, "id")),
	contained(on(Escalation, "sev"), on(Sev, "id"))
])

type Rels = (typeof Oncall)["relations"]

const SEV_ID = 0
const INCIDENT_ID = 1
const ESCALATION_ID = 2

function sortedPairs(rows: ReadonlyArray<{ readonly i: bigint; readonly s: string }>): Array<[bigint, string]> {
	return rows
		.map(function pair(row): [bigint, string] {
			return [row.i, row.s]
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

describe("ψ query atoms over closed relations", function suite() {
	let db: DbHandle

	before(async function seed() {
		const created = await native.dbCreate(storeDir, lower(Oncall))
		assert.equal(created.tag, "accepted", "the store admits")
		db = created.db

		const committed = native.dbWrite(db, function write(tx) {
			native.txInsert(tx, INCIDENT_ID, 1n, [1n, 0n])
			native.txInsert(tx, INCIDENT_ID, 1n, [2n, 1n])
			native.txInsert(tx, INCIDENT_ID, 1n, [3n, 2n])
			native.txInsert(tx, INCIDENT_ID, 1n, [4n, 3n])
			native.txInsert(tx, INCIDENT_ID, 1n, [5n, 2n])
			native.txInsert(tx, ESCALATION_ID, 1n, [1n, 0n])
			native.txInsert(tx, ESCALATION_ID, 1n, [2n, 1n])
			native.txInsert(tx, ESCALATION_ID, 1n, [3n, 2n])
			native.txInsert(tx, ESCALATION_ID, 1n, [4n, 3n])
			native.txInsert(tx, ESCALATION_ID, 1n, [5n, 2n])
			native.txInsert(tx, ESCALATION_ID, 1n, [5n, 3n])
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

	test("the closed-atom spelling returns row-for-row the rule-union inversion's answer set (recipe 7/8's forced spelling dies)", function runtimeEquivalence() {
		const paged = query(Oncall).rule(function rule(r) {
			const esc = v(Escalation)
			return r
				.match(Escalation, { incident: esc.incident, sev: esc.sev })
				.match(Sev, { id: esc.sev, pages: true })
				.find({ i: esc.incident, s: esc.sev })
		})

		type RowPin = Expect<
			Equal<QueryRow<typeof paged>, { readonly i: bigint; readonly s: "Info" | "Warn" | "Crit" | "Fatal" }>
		>
		const pagedUnion = query(Oncall)
			.rule(function rule(r) {
				const esc = v(Escalation)
				return r
					.match(Escalation, { incident: esc.incident, sev: esc.sev })
					.where(r.eq(esc.sev, "Crit"))
					.find({ i: esc.incident, s: esc.sev })
			})
			.rule(function rule(r) {
				const esc = v(Escalation)
				return r
					.match(Escalation, { incident: esc.incident, sev: esc.sev })
					.where(r.eq(esc.sev, "Fatal"))
					.find({ i: esc.incident, s: esc.sev })
			})
		const viaPsi = sortedPairs(run(paged, {}))
		const viaUnion = sortedPairs(run(pagedUnion, {}))
		assert.deepEqual(viaPsi, viaUnion, "the two spellings answer identically over the same store")
		assert.deepEqual(viaPsi, [
			[3n, "Crit"],
			[4n, "Fatal"],
			[5n, "Crit"],
			[5n, "Fatal"]
		])
		const pins: [RowPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("the NEGATED closed atom is the union's complement — same rows as the non-paging rule union", function negatedEquivalence() {
		const unpaged = query(Oncall).rule(function rule(r) {
			const esc = v(Escalation)
			return r
				.match(Escalation, { incident: esc.incident, sev: esc.sev })
				.where(r.not(Sev, { id: esc.sev, pages: true }))
				.find({ i: esc.incident, s: esc.sev })
		})
		const unpagedUnion = query(Oncall)
			.rule(function rule(r) {
				const esc = v(Escalation)
				return r
					.match(Escalation, { incident: esc.incident, sev: esc.sev })
					.where(r.eq(esc.sev, "Info"))
					.find({ i: esc.incident, s: esc.sev })
			})
			.rule(function rule(r) {
				const esc = v(Escalation)
				return r
					.match(Escalation, { incident: esc.incident, sev: esc.sev })
					.where(r.eq(esc.sev, "Warn"))
					.find({ i: esc.incident, s: esc.sev })
			})
		const viaPsi = sortedPairs(run(unpaged, {}))
		assert.deepEqual(viaPsi, sortedPairs(run(unpagedUnion, {})))
		assert.deepEqual(viaPsi, [
			[1n, "Info"],
			[2n, "Warn"]
		])
	})

	test("a handle literal sits in the id position; the payload escapes to the head (the engine's fallback join, invisible here)", function handleLiteralAtId() {
		const critRank = query(Oncall).rule(function rule(r) {
			const sev = v(Sev)
			return r.match(Sev, { id: "Crit", rank: sev.rank }).find({ k: sev.rank })
		})
		type RankPin = Expect<Equal<QueryRow<typeof critRank>, { readonly k: bigint }>>
		assert.deepEqual(run(critRank, {}), [{ k: 3n }])

		// name is a typed refusal, never a silent empty answer. The ψ atom's

		assert.throws(function offRoster() {
			lowerQuery(
				query(Oncall).rule(function rule(r) {
					const sev = v(Sev)
					return (
						r
							// @ts-expect-error — "Panic" is not in Sev's handle union (the ψ id position is precise)
							.match(Sev, { id: "Panic", rank: sev.rank })
							.find({ k: sev.rank })
					)
				})
			)
		}, /"Panic" is not a handle of Sev — the roster is Info, Warn, Crit, Fatal/)

		// shape refusal at lowering and a compile error at the surface.
		assert.throws(function rawIdSpelling() {
			lowerQuery(
				query(Oncall).rule(function rule(r) {
					const sev = v(Sev)
					return (
						r
							// @ts-expect-error — 0n is not a handle name: bigint left the closed surface with 0.4.0
							.match(Sev, { id: 2n, rank: sev.rank })
							.find({ k: sev.rank })
					)
				})
			)
		}, /expected a Sev handle name \(string\), got bigint/)
		const pins: [RankPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("the lowering golden: id → ordinal 0, payload columns → declared index + 1, positive and negated alike", function loweringGolden() {
		const golden = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r
				.match(Incident, { id: inc.id, sev: inc.sev })
				.match(Sev, { id: inc.sev, pages: true })
				.where(r.not(Sev, { id: inc.sev, rank: 4n }))
				.find({ i: inc.id })
		})
		assert.deepStrictEqual(lowerQuery(golden), {
			kind: "cq",
			interiors: [],
			head: [{ kind: "var" }],
			rules: [
				{
					finds: [{ kind: "var", var: 0 }],
					atoms: [
						{
							source: { kind: "edb", relation: INCIDENT_ID },
							bindings: [
								[0, { kind: "var", var: 0 }],
								[1, { kind: "var", var: 1 }]
							]
						},
						{
							source: { kind: "edb", relation: SEV_ID },
							bindings: [
								[0, { kind: "var", var: 1 }],
								[1, { kind: "literal", value: { kind: "bool", value: true } }]
							]
						}
					],
					negated: [
						{
							source: { kind: "edb", relation: SEV_ID },
							bindings: [
								[0, { kind: "var", var: 1 }],
								[2, { kind: "literal", value: { kind: "u64", value: 4n } }]
							]
						}
					],
					conditions: []
				}
			]
		})

		const answers = run(golden, {}).map(function id(row) {
			return row.i
		})
		assert.deepEqual(
			[...answers].sort(function asc(left, right) {
				if (left < right) {
					return -1
				}
				if (left > right) {
					return 1
				}
				return 0
			}),
			[3n, 5n]
		)
	})

	test("the join walls hold over closed atoms at both tiers (each @ts-expect-error real; the runtime twin throws the same verdict)", function joinWalls() {

		assert.throws(function crossClassIdReuse() {
			query(Oncall).rule(function rule(r) {
				const sev = v(Sev)
				return (
					r
						.match(Sev, { id: sev.id, pages: sev.pages })
						// @ts-expect-error — sev.id first bound in the "Sev.id" class; Escalation.incident is in "Incident.id" (the two-tier join wall)
						.match(Escalation, { incident: sev.id })
						.find({ s: sev.id, p: sev.pages })
				)
			})
		}, /joins domain-unequal fields/)

		assert.throws(function unknownColumn() {
			query(Oncall).rule(function rule(r) {
				const sev = v(Sev)
				return (
					r
						// @ts-expect-error — Sev has no column bogus
						.match(Sev, { bogus: true })
						.find({ s: sev.id })
				)
			})
		}, /relation Sev has no field bogus/)
	})

	test("a payload column joins a same-CLASS field of another atom; a different-CLASS reuse is the same wall", function payloadClassJoins() {

		const Grade = closed("Grade", { rank: u64 }, { Failed: { rank: 1n }, Passed: { rank: 2n } })
		const Course = relation("Course", { id: u64.fresh, level: u64 })
		const Rubric = schema("Rubric", { Grade, Course }, [
			key(Course, ["level"]),
			contained(on(Grade, "rank"), on(Course, "level"))
		])
		type PayloadClassPin = Expect<
			Equal<(typeof Rubric)["classes"]["Grade"]["rank"], (typeof Rubric)["classes"]["Course"]["level"]>
		>
		const levelled = query(Rubric).rule(function rule(r) {
			const g = v(Grade)
			const c = v(Course)
			return r
				.match(Grade, { id: g.id, rank: g.rank })
				.match(Course, { id: c.id, level: g.rank })
				.find({ c: c.id, g: g.id, k: g.rank })
		})

		type LevelledPin = Expect<
			Equal<QueryRow<typeof levelled>, { readonly c: bigint; readonly g: "Failed" | "Passed"; readonly k: bigint }>
		>
		assert.equal(levelled.data.rules.length, 1)

		assert.throws(function crossClassPayloadReuse() {
			query(Rubric).rule(function rule(r) {
				const g = v(Grade)
				return (
					r
						.match(Grade, { rank: g.rank })
						// @ts-expect-error — g.rank first bound in Grade.rank's generator-less class; Course.id generates "Course.id"
						.match(Course, { id: g.rank })
						.find({ k: g.rank })
				)
			})
		}, /joins domain-unequal fields/)

		const pins: [PayloadClassPin, LevelledPin] = [true, true]
		assert.equal(pins.length, 2)
	})

	test("lowering is stable: the same closed-atom query built twice lowers to deeply-equal IR", function deterministic() {
		function build() {
			return query(Oncall).rule(function rule(r) {
				const esc = v(Escalation)
				return r
					.match(Escalation, { incident: esc.incident, sev: esc.sev })
					.match(Sev, { id: esc.sev, pages: true })
					.where(r.not(Sev, { id: esc.sev, rank: 4n }))
					.find({ i: esc.incident })
			})
		}
		assert.deepStrictEqual(lowerQuery(build()), lowerQuery(build()))
	})
})
