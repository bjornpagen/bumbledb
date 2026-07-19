/**
 * PRD-K2 probes: ψ query atoms — `.match`/`not` over CLOSED relations,
 * against a real durable store. A closed vocabulary is matchable exactly
 * like an ordinary relation (`r.match(Sev, { id: r.var("s"), pages: true })`),
 * negatable (`r.not(Sev, {...})` — the engine folds the complement,
 * domain-witness guarded), and the SDK stays oblivious to WHETHER the
 * engine folds a plan-constant member set or joins the L1-resident
 * virtual image — transparency is the contract, pass-through lowering the
 * whole mechanism. Pinned here: the compile-PASS shapes (payload literal,
 * payload var joining a same-CLASS field of another atom per K4's law
 * map, the negated closed atom, a handle literal in the id position); the
 * compile-FAIL walls, each `@ts-expect-error` real (an unknown payload
 * column, a payload var joining a different-CLASS field, a closed atom's
 * id var reused cross-class — the two-tier join wall holds identically,
 * with the runtime twin throwing the same verdict); the LOWERING GOLDEN
 * (the sealed ordinal shift as literal IR: `id` → ordinal 0, payload
 * columns → declared index + 1, for positive AND negated atoms — the
 * runtime twin of the type tier's `MatchFields`, never trusted); the
 * roster refusal at the id position (an out-of-roster bigint is a typed
 * lowering error); and RUNTIME EQUIVALENCE — the prepared closed-atom
 * query returns row-for-row the same answer set as the old rule-union
 * inversion over the same store, positive and negated alike (recipe 7/8's
 * forced spelling dies).
 */

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
import type { ParamsRecord } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
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

/**
 * THE LAWS TYPE THE COLUMNS: `Incident.sev` and `Escalation.sev` land in
 * the `"Sev.id"` generator class, `Escalation.incident` in `"Incident.id"`
 * — the closed atom's id position joins the referencing side through the
 * SAME class map every ordinary atom rides. The payload columns are in no
 * law here: bare.
 */
const Oncall = schema("Oncall", { Sev, Incident, Escalation }, [
	contained(on(Incident, "sev"), on(Sev, "id")),
	contained(on(Escalation, "incident"), on(Incident, "id")),
	contained(on(Escalation, "sev"), on(Sev, "id"))
])

type Rels = (typeof Oncall)["relations"]

/** Relation ids = record declaration order (the law `lowerQuery` rides; the closed member occupies its slot). */
const SEV_ID = 0
const INCIDENT_ID = 1
const ESCALATION_ID = 2

/** Sorts answer rows (incident id, severity handle NAME) for a set-equality comparison (answers are sets; the host sorts). */
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

	before(function seed() {
		const created = native.dbCreate(storeDir, lower(Oncall))
		assert.ok(created.ok, "the store admits")
		db = created.db
		// The native seam is RAW: closed cells are declaration-order row ids
		// (Info 0, Warn 1, Crit 2, Fatal 3) — the name↔id bijection is the
		// SDK marshal's, above this seam.
		const tx = native.dbWriteBegin(db)
		native.txInsert(tx, INCIDENT_ID, [1n, 0n])
		native.txInsert(tx, INCIDENT_ID, [2n, 1n])
		native.txInsert(tx, INCIDENT_ID, [3n, 2n])
		native.txInsert(tx, INCIDENT_ID, [4n, 3n])
		native.txInsert(tx, INCIDENT_ID, [5n, 2n])
		native.txInsert(tx, ESCALATION_ID, [1n, 0n])
		native.txInsert(tx, ESCALATION_ID, [2n, 1n])
		native.txInsert(tx, ESCALATION_ID, [3n, 2n])
		native.txInsert(tx, ESCALATION_ID, [4n, 3n])
		native.txInsert(tx, ESCALATION_ID, [5n, 2n])
		native.txInsert(tx, ESCALATION_ID, [5n, 3n])
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
		return decodeAnswers<Row>(q.data.select, rows)
	}

	test("the closed-atom spelling returns row-for-row the rule-union inversion's answer set (recipe 7/8's forced spelling dies)", function runtimeEquivalence() {
		const paged = query(Oncall).rule((r) =>
			r
				.match(Escalation, { incident: r.var("i"), sev: r.var("s") })
				.match(Sev, { id: r.var("s"), pages: true })
				.select("i", "s")
		)
		// "s" is bound at Escalation.sev (the PRECISE roster) and REBOUND at
		// the ψ atom's own id — the sealed shape's id stays the WIDE fallback
		// (`ClosedIdField`, H1's ruling), so the joined slot claims `string`,
		// and H4's decode makes the claim TRUE: the runtime value is the
		// handle NAME, lifted through the marshal's one bijection.
		type RowPin = Expect<Equal<QueryRow<typeof paged>, { readonly i: bigint; readonly s: string }>>
		const pagedUnion = query(Oncall)
			.rule((r) =>
				r
					.match(Escalation, { incident: r.var("i"), sev: r.var("s") })
					.where(r.eq(r.var("s"), "Crit"))
					.select("i", "s")
			)
			.rule((r) =>
				r
					.match(Escalation, { incident: r.var("i"), sev: r.var("s") })
					.where(r.eq(r.var("s"), "Fatal"))
					.select("i", "s")
			)
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
		const unpaged = query(Oncall).rule((r) =>
			r
				.match(Escalation, { incident: r.var("i"), sev: r.var("s") })
				.where(r.not(Sev, { id: r.var("s"), pages: true }))
				.select("i", "s")
		)
		const unpagedUnion = query(Oncall)
			.rule((r) =>
				r
					.match(Escalation, { incident: r.var("i"), sev: r.var("s") })
					.where(r.eq(r.var("s"), "Info"))
					.select("i", "s")
			)
			.rule((r) =>
				r
					.match(Escalation, { incident: r.var("i"), sev: r.var("s") })
					.where(r.eq(r.var("s"), "Warn"))
					.select("i", "s")
			)
		const viaPsi = sortedPairs(run(unpaged, {}))
		assert.deepEqual(viaPsi, sortedPairs(run(unpagedUnion, {})))
		assert.deepEqual(viaPsi, [
			[1n, "Info"],
			[2n, "Warn"]
		])
	})

	test("a handle literal sits in the id position; the payload escapes to the head (the engine's fallback join, invisible here)", function handleLiteralAtId() {
		const critRank = query(Oncall).rule((r) => r.match(Sev, { id: "Crit", rank: r.var("k") }).select("k"))
		type RankPin = Expect<Equal<QueryRow<typeof critRank>, { readonly k: bigint }>>
		assert.deepEqual(run(critRank, {}), [{ k: 3n }])
		// The roster judges the id position at lowering — an unknown handle
		// name is a typed refusal, never a silent empty answer. The ψ atom's
		// OWN id types at the WIDE fallback (any string spells; H1's ruling),
		// so this belt is the id position's whole wall.
		assert.throws(function offRoster() {
			lowerQuery(query(Oncall).rule((r) => r.match(Sev, { id: "Panic", rank: r.var("k") }).select("k")))
		}, /"Panic" is not a handle of Sev — the roster is Info, Warn, Crit, Fatal/)
		// The bigint spelling is GONE from the closed surface — a raw id is a
		// shape refusal at lowering and a compile error at the surface.
		assert.throws(function rawIdSpelling() {
			// @ts-expect-error — 0n is not a handle name: bigint left the closed surface with 0.4.0
			lowerQuery(query(Oncall).rule((r) => r.match(Sev, { id: 2n, rank: r.var("k") }).select("k")))
		}, /expected a Sev handle name \(string\), got bigint/)
		const pins: [RankPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("the lowering golden: id → ordinal 0, payload columns → declared index + 1, positive and negated alike", function loweringGolden() {
		const golden = query(Oncall).rule((r) =>
			r
				.match(Incident, { id: r.var("i"), sev: r.var("s") })
				.match(Sev, { id: r.var("s"), pages: true })
				.where(r.not(Sev, { id: r.var("s"), rank: 4n }))
				.select("i")
		)
		assert.deepStrictEqual(lowerQuery(golden), {
			predicates: [
				{
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
				}
			],
			output: 0
		})
		// The golden shape is also the engine's: paging incidents minus rank-4 severities = the Crit incidents.
		const answers = run(golden, {}).map(function id(row) {
			return row.i
		})
		assert.deepEqual(
			[...answers].sort(function compare(a, b) {
				return a < b ? -1 : 1
			}),
			[3n, 5n]
		)
	})

	test("the join walls hold over closed atoms at both tiers (each @ts-expect-error real; the runtime twin throws the same verdict)", function joinWalls() {
		// A closed atom's id var reused cross-class: "s" is in "Sev.id", Escalation.incident in "Incident.id".
		assert.throws(function crossClassIdReuse() {
			query(Oncall).rule((r) =>
				r
					.match(Sev, { id: r.var("s"), pages: r.var("p") })
					// @ts-expect-error — "s" first bound in the "Sev.id" class; Escalation.incident is in "Incident.id" (the two-tier join wall)
					.match(Escalation, { incident: r.var("s") })
					.select("s", "p")
			)
		}, /joins domain-unequal fields/)

		// An unknown payload column is unwritable — the sealed shape is id + the declared columns, nothing else.
		assert.throws(function unknownColumn() {
			query(Oncall).rule((r) =>
				// @ts-expect-error — Sev has no column bogus
				r.match(Sev, { bogus: true }).select("s")
			)
		}, /relation Sev has no field bogus/)
	})

	test("a payload column joins a same-CLASS field of another atom; a different-CLASS reuse is the same wall", function payloadClassJoins() {
		// A construction-only theory (never opened): the containment puts
		// Course.level and Grade.rank in ONE generator-less class — a
		// payload column of a closed vocabulary is class-typed by the laws
		// exactly like an ordinary column (the option-2 dividend).
		const Grade = closed("Grade", { rank: u64 }, { Failed: { rank: 1n }, Passed: { rank: 2n } })
		const Course = relation("Course", { id: u64.fresh, level: u64 })
		const Rubric = schema("Rubric", { Grade, Course }, [contained(on(Course, "level"), on(Grade, "rank"))])
		type PayloadClassPin = Expect<
			Equal<(typeof Rubric)["classes"]["Grade"]["rank"], (typeof Rubric)["classes"]["Course"]["level"]>
		>
		const levelled = query(Rubric).rule((r) =>
			r
				.match(Grade, { id: r.var("g"), rank: r.var("k") })
				.match(Course, { id: r.var("c"), level: r.var("k") })
				.select("c", "g", "k")
		)
		// "g" is bound at the ψ atom's own id — the sealed shape's id is the
		// WIDE fallback (`ClosedIdField`), so the claim is `string` (H1).
		type LevelledPin = Expect<
			Equal<QueryRow<typeof levelled>, { readonly c: bigint; readonly g: string; readonly k: bigint }>
		>
		assert.equal(levelled.data.rules.length, 1)

		// The payload var landing on a DIFFERENT class is the identical wall.
		assert.throws(function crossClassPayloadReuse() {
			query(Rubric).rule((r) =>
				r
					.match(Grade, { rank: r.var("k") })
					// @ts-expect-error — "k" first bound in Grade.rank's generator-less class; Course.id generates "Course.id"
					.match(Course, { id: r.var("k") })
					.select("k")
			)
		}, /joins domain-unequal fields/)

		const pins: [PayloadClassPin, LevelledPin] = [true, true]
		assert.equal(pins.length, 2)
	})

	test("lowering is stable: the same closed-atom query built twice lowers to deeply-equal IR", function deterministic() {
		function build() {
			return query(Oncall).rule((r) =>
				r
					.match(Escalation, { incident: r.var("i"), sev: r.var("s") })
					.match(Sev, { id: r.var("s"), pages: true })
					.where(r.not(Sev, { id: r.var("s"), rank: 4n }))
					.select("i")
			)
		}
		assert.deepStrictEqual(lowerQuery(build()), lowerQuery(build()))
	})
})
