/**
 * PRD-H3 probes: handle NAMES are the query surface's literal vocabulary,
 * and set membership is a plain ARRAY. A closed-referencing position takes
 * the handle union — `r.match(Incident, { sev: "Crit" })`, `{ sev:
 * ["Crit", "Fatal"] }`, `r.param` at a closed field bound to a name,
 * `r.eq(v, "Fatal")` — while the WIRE still crosses u64 row ids (the
 * `taggedHandleId` translation, the query tier's single
 * roster-verification point). Pinned here: the compile-PASS shapes at the
 * union; the compile-FAIL walls, each `@ts-expect-error` real (a
 * cross-vocabulary literal, `0n` in every closed position — bigint left
 * the closed surface, a membership array at an ordinary field — arrays
 * are CLOSED-ONLY by owner ruling, ordinary membership is `r.inSet`); the
 * LOWERING GOLDENS — a name literal lowers to the exact IR the old bigint
 * spelling produced (deep-equal against the pinned program), and a
 * membership array lowers BYTE-IDENTICAL to the same set spelled
 * `r.inSet` (the existing set/word-set form the engine folds; the SDK
 * supplies the translated members itself at execute); the SELECTION
 * golden — a `where()` array lowers byte-identical to the old set
 * combinator's `{ kind: "many" }` spec, so no fingerprint moves; the
 * runtime rows, each equal to what the 0.3.0 bigint twin answered over
 * the same store; and the structural doctrine: two vocabularies sharing a
 * handle name overlap exactly on the shared literal.
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
import type { Query, QueryParams, QueryRow } from "#query/lower.ts"
import { lowerQuery, query } from "#query/lower.ts"
import { decodeAnswers, wireParams } from "#query/run.ts"
import { type ParamsRecord, v } from "#query/scope.ts"
import { relation } from "#relation.ts"
import { schema } from "#schema.ts"
import { contained } from "#statements.ts"

/** The identity-strength equality probe (the standard dual-function trick). */
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false

/** Pins a probe to `true` at compile time. */
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
/** Shares the handle name "Crit" with `Sev` — the overlap-doctrine pin. */
const Priority = closed("Priority", ["Crit", "Low"])
const Incident = relation("Incident", { id: u64.fresh, sev: Sev.id, pri: Priority.id })

const Oncall = schema("Oncall", { Sev, Priority, Incident }, [
	contained(on(Incident, "sev"), on(Sev, "id")),
	contained(on(Incident, "pri"), on(Priority, "id"))
])

type Rels = (typeof Oncall)["relations"]

/** Relation ids = record declaration order (the law `lowerQuery` rides). */
const INCIDENT_ID = 2

/** Sorts one bigint column for a set-equality comparison (answers are sets; the host sorts). */
function sorted(values: readonly bigint[]): bigint[] {
	return [...values].sort(function compare(a, b) {
		return a < b ? -1 : 1
	})
}

describe("query literals, params & membership arrays over closed references", function suite() {
	let db: DbHandle

	before(function seed() {
		const created = native.dbCreate(storeDir, lower(Oncall))
		assert.ok(created.ok, "the store admits")
		db = created.db
		// The native seam is RAW: closed cells are declaration-order row ids
		// (Sev: Info 0, Warn 1, Crit 2, Fatal 3; Priority: Crit 0, Low 1) —
		// the name↔id bijection is the SDK's, above this seam.
		const tx = native.dbWriteBegin(db)
		native.txInsert(tx, INCIDENT_ID, [1n, 0n, 1n])
		native.txInsert(tx, INCIDENT_ID, [2n, 1n, 1n])
		native.txInsert(tx, INCIDENT_ID, [3n, 2n, 0n])
		native.txInsert(tx, INCIDENT_ID, [4n, 3n, 0n])
		native.txInsert(tx, INCIDENT_ID, [5n, 2n, 1n])
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

	/** Projects the `i` column of an answer set, sorted. */
	function incidents(rows: ReadonlyArray<{ readonly i: bigint }>): bigint[] {
		return sorted(
			rows.map(function i(row) {
				return row.i
			})
		)
	}

	test("a handle-name literal matches, and lowers to the EXACT program the old bigint spelling produced", function nameLiteral() {
		const crits = query(Oncall).rule(function rule(r) {
			const inc = v(Incident)
			return r.match(Incident, { id: inc.id, sev: "Crit" }).find({ i: inc.id })
		})
		// The lowering golden: "Crit" translates to declaration-order id 2n
		// and crosses tagged u64 — the wire program is the old `sev: 2n`
		// spelling's program, position for position (queries cross ids,
		// never handle names).
		assert.deepStrictEqual(lowerQuery(crits), {
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
										[1, { kind: "literal", value: { kind: "u64", value: 2n } }]
									]
								}
							],
							negated: [],
							conditions: []
						}
					]
				}
			],
			output: 0
		})
		// The same rows the 0.3.0 bigint twin (`sev: 2n`) answered over this store.
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
		// The wire-program golden, BYTE-compared: one paramSet term over the
		// one dense ParamId — the array IS the existing set/word-set form,
		// its members folded by the SDK at execute.
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
		// Selections take arrays at EVERY field kind (they have no params to
		// ride) — a ψ selection over an ordinary payload column included.
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
		// The same rows the 0.3.0 twin answered when executed with { s: 2n }.
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
		// "Crit" is a handle of BOTH Sev and Priority: the one literal is
		// legal at either field, and each lowers through its OWN roster.
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
		// "Low" is Priority-only: on the Sev side it is a compile error AND
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
		// The registry key sorts a copy of the members, so `["Crit","Fatal"]`
		// in rule 0 and `["Fatal","Crit"]` in rule 1 mint ONE ParamId and one
		// wire set — the stated sharing law holds for every spelling order.
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
			"the wire program is the one-spelling program, byte for byte"
		)
		assert.deepEqual(incidents(run(reordered, {})), [3n, 4n, 5n])
	})

	test("a param anchored at both a closed reference and a bare field refuses at construction (one name, one roster)", function paramAnchorCoherence() {
		// The runtime twin of the type tier's never-intersection: an untyped
		// caller's mixed-anchor param would ride the FIRST anchor's reading
		// only (a raw bigint binding the closed field untranslated and
		// unverified), so the registry refuses the pairing outright.
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
		// One roster across many uses stays legal — the honest spelling executes.
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
		// Without the roster arm a Tag-referencing column with no containment
		// (bare class) would join a plain bare u64 — and ordering, decode,
		// and translation would then depend on binding order. The wall holds
		// at both tiers: the directive is real, the runtime names the slots.
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
		/**
		 * The registry resolves membership members to their frozen wire param
		 * when the query value is assembled (131: the entry stores the image),
		 * so the out-of-roster name fails where the mistake was made — at the
		 * query build, never the first execute.
		 */
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
