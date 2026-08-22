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
import type { Query, QueryParams, QueryRow } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import { type ParamsRecord, v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained } from "#statements.ts"

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

type Expect<T extends true> = T extends true ? true : never

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-closed-literals-"))
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

const Priority = closed("Priority", ["Crit", "Low"])
const Incident = relation("Incident", { id: u64.fresh, sev: Sev.id, pri: Priority.id })

const Oncall = schema("Oncall", { Sev, Priority, Incident }, [
	contained(on(Incident, "sev"), on(Sev, "id")),
	contained(on(Incident, "pri"), on(Priority, "id"))
])

type Rels = (typeof Oncall)["relations"]

const INCIDENT_ID = 2

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

describe("query literals, params & membership arrays over closed references", function suite() {
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
			native.txInsert(tx, INCIDENT_ID, 1n, [5n, 2n, 1n])
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

	function incidents(rows: ReadonlyArray<{ readonly i: bigint }>): bigint[] {
		return sorted(
			rows.map(function i(row) {
				return row.i
			})
		)
	}

	test("a handle-name literal matches, and lowers to the EXACT query the old bigint spelling produced", function nameLiteral() {
		const crits = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, sev: "Crit" }).find({ i: inc.id })
		})

		assert.deepStrictEqual(lowerQuery(crits), {
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
								[1, { kind: "literal", value: { kind: "u64", value: 2n } }]
							]
						}
					],
					negated: [],
					conditions: []
				}
			]
		})

		assert.deepEqual(incidents(run(crits, {})), [3n, 5n])
	})

	test("a membership ARRAY lowers byte-identical to the same set spelled r.inSet, and answers identically", function membershipArray() {
		const viaArray = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, sev: ["Crit", "Fatal"] }).find({ i: inc.id })
		})
		const viaInSet = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, sev: r.inSet("members") }).find({ i: inc.id })
		})

		assert.equal(JSON.stringify(lowerQuery(viaArray)), JSON.stringify(lowerQuery(viaInSet)))
		const arrayRows = incidents(run(viaArray, {}))
		assert.deepEqual(arrayRows, [3n, 4n, 5n])
		assert.deepEqual(arrayRows, incidents(run(viaInSet, { members: ["Crit", "Fatal"] })))
	})

	test("a where() selection ARRAY lowers byte-identical to the old set combinator's spec (no fingerprint moves)", function selectionArray() {
		assert.equal(
			JSON.stringify(Incident.where({ sev: ["Crit", "Fatal"] }).selection),
			'[{"field":"sev","set":{"kind":"many","literals":[{"kind":"handle","handle":"Crit"},{"kind":"handle","handle":"Fatal"}]}}]'
		)

		assert.deepStrictEqual(Sev.where({ rank: [3n, 4n] }).selection, [
			{
				field: "rank",
				set: {
					kind: "many",
					literals: [
						{ kind: "value", value: { kind: "u64", value: 3n } },
						{ kind: "value", value: { kind: "u64", value: 4n } }
					]
				}
			}
		])
	})

	test("a param anchored at a closed field types as the union and translates name → id at execute", function namedParam() {
		const bySev = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, sev: r.param("s") }).find({ i: inc.id })
		})
		type ParamPin = Expect<Equal<QueryParams<typeof bySev>, { readonly s: "Info" | "Warn" | "Crit" | "Fatal" }>>

		assert.deepEqual(incidents(run(bySev, { s: "Crit" })), [3n, 5n])
		assert.throws(function unknownName() {
			// @ts-expect-error — "Bogus" is not in Sev's handle union (the params object is typed by use)
			const params: QueryParams<typeof bySev> = { s: "Bogus" }
			run(bySev, params)
		}, /"Bogus" is not a handle of Sev — the roster is Info, Warn, Crit, Fatal/)
		assert.throws(function bigintValue() {
			// @ts-expect-error — 0n is not a handle name: bigint left the closed surface
			run(bySev, { s: 2n })
		}, /expected a Sev handle name \(string\), got bigint/)
		const pins: [ParamPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("eq against a closed-bound var takes the handle union on the literal side", function eqRhs() {
		const fatal = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, sev: inc.sev }).where(r.eq(inc.sev, "Fatal")).find({ i: inc.id })
		})
		assert.deepEqual(incidents(run(fatal, {})), [4n])
		type RowPin = Expect<Equal<QueryRow<typeof fatal>, { readonly i: bigint }>>
		const pins: [RowPin] = [true]
		assert.equal(pins.length, 1)
	})

	test("two vocabularies sharing a handle name overlap exactly on the shared literal (structural doctrine)", function sharedLiteral() {

		const sevCrit = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, sev: "Crit" }).find({ i: inc.id })
		})
		const priCrit = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, pri: "Crit" }).find({ i: inc.id })
		})
		assert.deepEqual(incidents(run(sevCrit, {})), [3n, 5n])
		assert.deepEqual(incidents(run(priCrit, {})), [3n, 4n], "Priority's Crit is id 0 — its own declaration order")

		// a lowering refusal (the roster judges; the directive is real).
		assert.throws(function crossVocabulary() {
			lowerQuery(
				query(Oncall).rule(function rule(r) {
					const inc = v(Incident)
					// @ts-expect-error — "Low" is not in Sev's handle union (cross-vocabulary literals are unwritable)
					return r.match(Incident, { id: inc.id, sev: "Low" }).find({ i: inc.id })
				})
			)
		}, /"Low" is not a handle of Sev — the roster is Info, Warn, Crit, Fatal/)
	})

	test("the degenerate membership arrays refuse at construction (the canonical-utterance law)", function degenerateArrays() {
		assert.throws(function emptyArray() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: [] }).find({ i: inc.id })
			})
		}, /an empty membership array selects nothing/)
		assert.throws(function oneElementArray() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: ["Crit"] }).find({ i: inc.id })
			})
		}, /a one-element membership array is the bare literal respelled/)
	})

	test("a duplicate member is the banned respelling — refused at construction, matching the selection tier's voice", function duplicateMembers() {
		assert.throws(function duplicatePair() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: ["Crit", "Crit"] }).find({ i: inc.id })
			})
		}, /relation Incident\.sev: the membership array spells Crit twice — write it once/)
		assert.throws(function duplicateAmongMany() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: ["Crit", "Fatal", "Crit"] }).find({ i: inc.id })
			})
		}, /the membership array spells Crit twice/)
	})

	test("reordered membership spellings are ONE set — content-addressed to one dense ParamId", function contentAddressed() {

		const reordered = query(Oncall)
			.rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: ["Crit", "Fatal"] }).find({ i: inc.id })
			})
			.rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: ["Fatal", "Crit"] }).find({ i: inc.id })
			})
		assert.equal(reordered.data.params.length, 1, "two spellings of one set share one registry entry")
		const oneSpelling = query(Oncall)
			.rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: ["Crit", "Fatal"] }).find({ i: inc.id })
			})
			.rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: ["Crit", "Fatal"] }).find({ i: inc.id })
			})
		assert.equal(
			JSON.stringify(lowerQuery(reordered)),
			JSON.stringify(lowerQuery(oneSpelling)),
			"the wire IR is the one-spelling query, byte for byte"
		)
		assert.deepEqual(incidents(run(reordered, {})), [3n, 4n, 5n])
	})

	test("a param anchored at both a closed reference and a bare field refuses at construction (one name, one roster)", function paramAnchorCoherence() {

		assert.throws(function bareFirst() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: r.param("p"), sev: r.param("p"), pri: inc.pri }).find({ x: inc.pri })
			})
		}, /query param p is anchored at a non-closed position and at a Sev reference — a closed-anchored param translates handle names through ONE roster/)
		assert.throws(function closedFirst() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { sev: r.param("p"), id: r.param("p"), pri: inc.pri }).find({ x: inc.pri })
			})
		}, /query param p is anchored at a Sev reference and at a non-closed position/)
		assert.throws(function twoVocabularies() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: r.param("p"), pri: r.param("p") }).find({ i: inc.id })
			})
		}, /query param p is anchored at a Sev reference and at a Priority reference/)

		const legal = query(Oncall)
			.rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: r.param("s") }).find({ i: inc.id })
			})
			.rule(function rule(r) {
				const inc = v(Incident)
				return r.match(Incident, { id: inc.id, sev: r.param("s") }).find({ i: inc.id })
			})
		assert.deepEqual(incidents(run(legal, { s: "Crit" })), [3n, 5n])
	})

	test("a closed-descriptor slot never joins a bare u64 slot, even lawless (the roster is join structure)", function rosterJoinWall() {

		const Tag = closed("Tag", ["A", "B"])
		const Note = relation("Note", { id: u64.fresh, tag: Tag.id, val: u64 })
		const Twin = schema("Twin", { Tag, Note }, [])
		assert.throws(function joinAcross() {
			query(Twin).rule(function rule(r) {
				const n = v(Note)
				return (
					r
						.match(Note, { val: n.val })
						// @ts-expect-error — n.val first bound at a bare u64: a Tag-referencing slot never joins it (the roster is part of the join shape)
						.match(Note, { tag: n.val })
						.find({ x: n.val })
				)
			})
		}, /joins domain-unequal fields — minted at u64 \(bare\), reused at u64 referencing Tag \(bare\)/)
	})

	test("membership arrays are CLOSED-ONLY (owner ruling): an ordinary field's array is unwritable and refused", function closedOnlyArrays() {
		assert.throws(function ordinaryFieldArray() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				// @ts-expect-error — id is an ordinary u64: membership there is spelled r.inSet, never a literal array
				return r.match(Incident, { id: [1n, 2n], sev: inc.sev }).find({ s: inc.sev })
			})
		}, /a membership array is the closed-reference spelling — ordinary field membership is a bound ∈-set param \(r\.inSet\)/)
	})

	test("an unknown member name rides the ONE verification point and throws pointed at BUILD", function unknownMember() {

		assert.throws(function bogusMember() {
			query(Oncall).rule(function rule(r) {
				const inc = v(Incident)
				return (
					// @ts-expect-error — "Bogus" is not in Sev's handle union
					r.match(Incident, { id: inc.id, sev: ["Crit", "Bogus"] }).find({ i: inc.id })
				)
			})
		}, /"Bogus" is not a handle of Sev — the roster is Info, Warn, Crit, Fatal/)
	})

	test("0n compile-FAILS in every closed position (bigint is gone from the closed surface)", function bigintGone() {
		const unspellable: ReadonlyArray<() => unknown> = [
			function bigintLiteral() {
				const inc = v(Incident)
				// @ts-expect-error — a closed literal position takes the handle union, never a bigint
				return query(Oncall).rule((r) => r.match(Incident, { id: inc.id, sev: 2n }).find({ i: inc.id }))
			},
			function bigintArrayMember() {
				const inc = v(Incident)
				// @ts-expect-error — a membership array holds handle names, never bigints
				return query(Oncall).rule((r) => r.match(Incident, { id: inc.id, sev: [2n, 3n] }).find({ i: inc.id }))
			},
			function bigintEqRhs() {
				const inc = v(Incident)
				return query(Oncall).rule((r) =>
					r
						.match(Incident, { id: inc.id, sev: inc.sev })
						// @ts-expect-error — the eq literal side of a closed-bound var takes the union, never a bigint
						.where(r.eq(inc.sev, 2n))
						.find({ i: inc.id })
				)
			}
		]
		assert.equal(unspellable.length, 3)
	})
})
