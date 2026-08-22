import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import type {
	DbHandle,
	FactValue,
	Manifest,
	ParsedQuery,
	PreparedHandle,
	QueryIr,
	TxHandle,
	WitnessHandle
} from "#native.ts"
import { dbClose, native } from "#native.ts"
import { parseQueryIr } from "#query/parse-ir.ts"
import type { SchemaSpec } from "#spec.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-ffi-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const STATUS = 0
const KIND = 1
const PERSON = 2
const EDGE = 3

function mintedStart(range: { empty: true } | { empty: false; start: bigint }): bigint {
	if (range.empty) {
		throw new Error("expected nonempty fresh range")
	}
	return range.start
}

const spec: SchemaSpec = {
	relations: [
		{
			name: "Status",
			fields: [],
			closed: {
				newtype: "Status",
				rows: [
					{ handle: "Open", values: [] },
					{ handle: "Frozen", values: [] }
				]
			}
		},
		{
			name: "Kind",
			fields: [{ name: "mastered", valueType: { kind: "bool" }, newtype: undefined, fresh: false }],
			closed: {
				newtype: "Kind",
				rows: [
					{ handle: "DirectPass", values: [{ kind: "value", value: { kind: "bool", value: true } }] },
					{ handle: "Failed", values: [{ kind: "value", value: { kind: "bool", value: false } }] }
				]
			}
		},
		{
			name: "Person",
			fields: [
				{ name: "id", valueType: { kind: "u64" }, newtype: "PersonId", fresh: true },
				{ name: "name", valueType: { kind: "string" }, newtype: undefined, fresh: false },
				{ name: "status", valueType: { kind: "u64" }, newtype: "Status", fresh: false },
				{ name: "score", valueType: { kind: "i64" }, newtype: undefined, fresh: false },
				{
					name: "tag",
					valueType: { kind: "fixedBytes", len: 4 },
					newtype: undefined,
					fresh: false
				},
				{
					name: "active",
					valueType: { kind: "interval", element: "i64", width: undefined },
					newtype: undefined,
					fresh: false
				},
				{ name: "flag", valueType: { kind: "bool" }, newtype: undefined, fresh: false }
			],
			closed: undefined
		},
		{
			name: "Edge",
			fields: [
				{ name: "from", valueType: { kind: "u64" }, newtype: "PersonId", fresh: false },
				{ name: "to", valueType: { kind: "u64" }, newtype: undefined, fresh: false },
				{ name: "weight", valueType: { kind: "u64" }, newtype: undefined, fresh: false }
			],
			closed: undefined
		}
	],
	statements: [
		{ kind: "fd", relation: "Edge", projection: ["from", "to"] },
		{
			kind: "containment",
			source: { relation: "Edge", projection: ["from"], selection: [] },
			target: { relation: "Person", projection: ["id"], selection: [] },
			bidirectional: false
		},
		{
			kind: "containment",
			source: { relation: "Person", projection: ["status"], selection: [] },
			target: { relation: "Status", projection: ["id"], selection: [] },
			bidirectional: false
		},
		{
			kind: "capacity",
			target: {
				relation: "Person",
				projection: ["id"],
				selection: [["status", { kind: "one", literal: { kind: "handle", handle: "Frozen" } }]]
			},
			weight: { kind: "unit" },
			window: { kind: "exact", n: { kind: "lit", value: 0n } },
			source: { relation: "Edge", projection: ["from"], selection: [] }
		},
		{
			kind: "capacity",
			target: { relation: "Person", projection: ["id"], selection: [] },
			weight: { kind: "field", field: "weight" },
			window: { kind: "range", lo: { kind: "lit", value: 0n }, hi: { kind: "lit", value: 1000n } },
			source: { relation: "Edge", projection: ["from"], selection: [] }
		}
	]
}

function personRow(
	id: bigint,
	name: string,
	status: bigint,
	score: bigint,
	tag: Uint8Array,
	active: { start: bigint; end: bigint },
	flag: boolean
): FactValue[] {
	const row: FactValue[] = [id, name, status, score, tag, active, flag]
	return row
}

function spellingOf(manifest: Manifest, statementId: number): string {
	const found = manifest.statements.find(function byId(statement) {
		return statement.id === statementId
	})
	assert.ok(found, `statement ${statementId} present in the manifest`)
	return found.spelling
}

function sortedBigints(values: bigint[]): bigint[] {
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

describe("ffi round trip against a real store", function suite() {
	let db: DbHandle
	let manifest: Manifest
	let personKeyId: number
	let edgeKeyId: number
	let prepared: PreparedHandle
	let p1 = 0n
	let p2 = 0n
	let p3 = 0n
	let p4 = 0n

	test("engineVersion is a non-empty proof string", function version() {
		assert.equal(typeof native.engineVersion(), "string")
		assert.notEqual(native.engineVersion(), "")
	})

	test("dbCreate + manifest carries every name→id table", async function create() {
		const created = await native.dbCreate(storeDir, spec)
		assert.equal(created.tag, "accepted", "create succeeds on a fresh directory")
		db = created.db
		manifest = native.dbManifest(db)

		assert.deepEqual(
			manifest.relations.map(function name(relation) {
				return [relation.name, relation.id]
			}),
			[
				["Status", STATUS],
				["Kind", KIND],
				["Person", PERSON],
				["Edge", EDGE]
			]
		)

		const status = manifest.relations[STATUS]
		assert.ok(status?.extension, "closed Status carries its extension")
		assert.deepEqual(
			status.extension.map(function handle(row) {
				return [row.handle, row.id]
			}),
			[
				["Open", 0n],
				["Frozen", 1n]
			]
		)
		assert.equal(status.fields[0]?.name, "id", "sealed shape opens with the synthetic id")

		const kind = manifest.relations[KIND]
		assert.ok(kind?.extension, "closed Kind carries its extension")
		assert.deepEqual(kind.extension[0]?.values, [{ name: "mastered", value: true }])
		assert.deepEqual(kind.extension[1]?.values, [{ name: "mastered", value: false }])

		const person = manifest.relations[PERSON]
		assert.ok(person)
		assert.deepEqual(
			person.fields.map(function fieldName(field) {
				return field.name
			}),
			["id", "name", "status", "score", "tag", "active", "flag"]
		)

		for (const statement of manifest.statements) {
			assert.equal(typeof statement.spelling, "string")
			assert.notEqual(statement.spelling, "")
		}
		const personKey = manifest.statements.find(function key(statement) {
			return statement.kind === "functionality" && statement.spelling.startsWith("Person(id)")
		})
		assert.ok(personKey, "the fresh auto-key on Person.id is in the manifest")
		personKeyId = personKey.id
		const edgeKey = manifest.statements.find(function key(statement) {
			return statement.kind === "functionality" && statement.spelling.startsWith("Edge(from, to)")
		})
		assert.ok(edgeKey, "the declared Edge fd is in the manifest")
		edgeKeyId = edgeKey.id
	})

	test("hostile capacity shapes refuse typed at the raw wire, before any store touch", function hostileCapacity() {
		const hostileDir = path.join(tmpRoot, "hostile")
		const target = { relation: "Person", projection: ["id"], selection: [] } as const
		const source = { relation: "Edge", projection: ["from"], selection: [] } as const

		const bareBigintBound: SchemaSpec = {
			relations: spec.relations,
			statements: [
				{
					kind: "capacity",
					target,
					weight: { kind: "unit" },
					// @ts-expect-error — a bare BigInt bound is dead wire: bounds are tagged objects only
					window: { kind: "exact", n: 0n },
					source
				}
			]
		}
		assert.throws(function bareBound() {
			native.dbCreate(hostileDir, bareBigintBound)
		}, /missing `kind` in capacity bound/)

		const weightless: SchemaSpec = {
			relations: spec.relations,
			statements: [
				// @ts-expect-error — C4: the wire ALWAYS carries `weight`; omission is not the unit weight
				{
					kind: "capacity",
					target,
					window: { kind: "range", lo: { kind: "lit", value: 0n }, hi: { kind: "lit", value: 1000n } },
					source
				}
			]
		}
		assert.throws(function omittedWeight() {
			native.dbCreate(hostileDir, weightless)
		}, /missing `weight` in capacity/)

		const countAlias: SchemaSpec = {
			relations: spec.relations,
			statements: [
				{
					kind: "capacity",
					target,
					// @ts-expect-error — the count instance has ONE spelling ({kind:"unit"}); an alias kind is unknown
					weight: { kind: "count" },
					window: { kind: "exact", n: { kind: "lit", value: 0n } },
					source
				}
			]
		}
		assert.throws(function unknownWeightKind() {
			native.dbCreate(hostileDir, countAlias)
		}, /unknown weight kind `count`/)

		const boundAlias: SchemaSpec = {
			relations: spec.relations,
			statements: [
				{
					kind: "capacity",
					target,
					weight: { kind: "unit" },
					// @ts-expect-error — bound kinds are lit/field/durationField; anything else is unknown
					window: { kind: "exact", n: { kind: "value", value: 3n } },
					source
				}
			]
		}
		assert.throws(function unknownBoundKind() {
			native.dbCreate(hostileDir, boundAlias)
		}, /unknown capacity bound kind `value`/)

		assert.equal(fs.existsSync(hostileDir), false, "marshal refusals precede every environment touch")
	})

	test("delta writes: fresh mint, final-state point reads, commit", function writes() {
		const committed = native.dbWrite(db, function write(tx) {
			p1 = mintedStart(native.txReserve(tx, PERSON, 0, 1n))
			p2 = mintedStart(native.txReserve(tx, PERSON, 0, 1n))
			p3 = mintedStart(native.txReserve(tx, PERSON, 0, 1n))
			p4 = mintedStart(native.txReserve(tx, PERSON, 0, 1n))
			assert.equal(typeof p1, "bigint")
			assert.equal(new Set([p1, p2, p3, p4]).size, 4, "fresh mints are distinct")

			const active = { start: -5n, end: 10n }
			const adaRow = personRow(p1, "ada", 0n, -3n, new Uint8Array([1, 2, 3, 4]), active, true)

			const cells = [
				...adaRow,
				...personRow(p2, "grace", 0n, 7n, new Uint8Array([5, 6, 7, 8]), active, false),
				...personRow(p3, "alan", 0n, 0n, new Uint8Array([9, 10, 11, 12]), active, true),
				...personRow(p4, "kurt", 1n, 42n, new Uint8Array([13, 14, 15, 16]), active, false)
			]
			assert.deepEqual(native.txInsert(tx, PERSON, 4n, cells), { submitted: 4n, changed: 4n })

			assert.equal(native.txContains(tx, PERSON, adaRow), true, "final-state view sees the pending insert")
			const got = native.txGet(tx, PERSON, personKeyId, [p1])
			assert.ok(got, "point read through the fresh key")
			assert.equal(got[1], "ada")
			assert.deepEqual(got[4], new Uint8Array([1, 2, 3, 4]))
			assert.deepEqual(got[5], active)

			assert.deepEqual(native.txInsert(tx, EDGE, 1n, [p1, p2, 1n]), { submitted: 1n, changed: 1n })
			assert.deepEqual(native.txInsert(tx, EDGE, 1n, [p2, p3, 1n]), { submitted: 1n, changed: 1n })
			assert.deepEqual(native.txInsert(tx, EDGE, 1n, [p3, p1, 1n]), { submitted: 1n, changed: 1n })

			assert.deepEqual(native.txInsert(tx, EDGE, 1n, [p1, p3, 7n]), { submitted: 1n, changed: 1n })
			assert.equal(native.txContains(tx, EDGE, [p1, p3, 7n]), true)
			assert.deepEqual(
				native.txDelete(tx, EDGE, 1n, [p1, p3, 7n]),
				{ submitted: 1n, changed: 1n },
				"delta delete cancels the pending insert"
			)
			assert.equal(native.txContains(tx, EDGE, [p1, p3, 7n]), false)

			return true
		})
		assert.equal(committed.tag, "accepted", "the clean commit lands")
		assert.equal(typeof committed.generation, "bigint")
		assert.equal(native.dbGeneration(db), committed.generation)
	})

	test("empty insert/delete/reserve still enter the transaction", function emptyIsAMutation() {
		native.dbWrite(db, function write(tx) {
			assert.deepEqual(native.txInsert(tx, PERSON, 0n, []), { submitted: 0n, changed: 0n })
			assert.deepEqual(native.txDelete(tx, PERSON, 0n, []), { submitted: 0n, changed: 0n })
			const empty = native.txReserve(tx, PERSON, 0, 0n)
			assert.equal(empty.empty, true)
			const next = native.txReserve(tx, PERSON, 0, 1n)
			if (next.empty) {
				throw new Error("reserve(1) must be nonempty")
			}
			assert.equal(typeof next.start, "bigint")
			return false
		})
	})

	test("instance reads: scan, contains, keyed get", function reads() {
		native.dbRead(db, function read(instance, _witness) {
			const edges = native.instanceScan(instance, EDGE)
			assert.equal(edges.length, 3)
			assert.equal(
				native.instanceContains(
					instance,
					PERSON,
					personRow(p1, "ada", 0n, -3n, new Uint8Array([1, 2, 3, 4]), { start: -5n, end: 10n }, true)
				),
				true
			)
			const edge = native.instanceGet(instance, EDGE, edgeKeyId, [p1, p2])
			assert.ok(edge, "keyed get finds the edge")
			assert.equal(edge[2], 1n)
			assert.equal(native.instanceGet(instance, EDGE, edgeKeyId, [p2, p1]), null)
		})
	})

	test("a functionality violation arrives canonical and decoded", function fdViolation() {
		const outcome = native.dbWrite(db, function write(tx) {
			assert.deepEqual(native.txInsert(tx, EDGE, 1n, [p1, p2, 9n]), { submitted: 1n, changed: 1n })
			return true
		})
		assert.equal(outcome.tag, "rejected", "the key judgment rejects")
		assert.equal(outcome.violations.length, 1, "key violations preempt the statement phase")
		const violation = outcome.violations[0]
		assert.ok(violation)
		assert.equal(violation.kind, "functionality")
		assert.equal(violation.canonical, spellingOf(manifest, violation.statementId))
		assert.ok(violation.facts.length > 0, "the offending fact rides decoded")
		assert.equal(violation.facts[0]?.relation, "Edge")
		const from = violation.facts[0]?.fields.find(function field(entry) {
			return entry.name === "from"
		})
		assert.deepEqual(from, { name: "from", value: p1 })
	})

	test("containment + window violations arrive together, complete", function statementViolations() {
		const ghost = 999_999n
		const outcome = native.dbWrite(db, function write(tx) {
			assert.deepEqual(native.txInsert(tx, EDGE, 1n, [ghost, p1, 1n]), { submitted: 1n, changed: 1n })
			assert.deepEqual(native.txInsert(tx, EDGE, 1n, [p4, p1, 1n]), { submitted: 1n, changed: 1n })
			return true
		})
		assert.equal(outcome.tag, "rejected", "the statement judgment rejects")
		assert.equal(outcome.violations.length, 2, "the statement phase is scan-complete")

		const containment = outcome.violations.find(function byKind(violation) {
			return violation.kind === "containment"
		})
		assert.ok(containment, "the containment citation is present")
		assert.equal(containment.direction, "sourceUnsatisfied")
		assert.equal(containment.canonical, spellingOf(manifest, containment.statementId))
		assert.equal(containment.facts[0]?.relation, "Edge")
		const ghostFrom = containment.facts[0]?.fields.find(function field(entry) {
			return entry.name === "from"
		})
		assert.deepEqual(ghostFrom, { name: "from", value: ghost })

		const capacityViolation = outcome.violations.find(function byKind(violation) {
			return violation.kind === "capacity"
		})
		assert.ok(capacityViolation, "the capacity citation is present")
		assert.equal(capacityViolation.measure, 1n)
		assert.equal(capacityViolation.canonical, spellingOf(manifest, capacityViolation.statementId))
		assert.equal(capacityViolation.facts[0]?.relation, "Person", "the convicted parent is the cited fact")
		const parentId = capacityViolation.facts[0]?.fields.find(function field(entry) {
			return entry.name === "id"
		})
		assert.deepEqual(parentId, { name: "id", value: p4 })
	})

	test("recursive closure query computes the reachable set", function closure() {
		const queryIr: QueryIr = {
			kind: "reach",
			interiors: [],
			rec: {
				head: [{ kind: "var" }],
				base: [
					{
						finds: [{ kind: "var", var: 0 }],
						atoms: [
							{
								source: { kind: "edb", relation: EDGE },
								bindings: [
									[0, { kind: "param", param: 0 }],
									[1, { kind: "var", var: 0 }]
								]
							}
						],
						negated: [],
						conditions: []
					}
				],
				rec: [
					{
						finds: [{ kind: "var", var: 1 }],
						atoms: [
							{ source: { kind: "interior", interior: 0 }, bindings: [[0, { kind: "var", var: 0 }]] },
							{
								source: { kind: "edb", relation: EDGE },
								bindings: [
									[0, { kind: "var", var: 0 }],
									[1, { kind: "var", var: 1 }]
								]
							}
						],
						negated: [],
						conditions: []
					}
				]
			},
			head: [{ kind: "var" }],
			rules: [
				{
					finds: [{ kind: "var", var: 0 }],
					atoms: [{ source: { kind: "interior", interior: 0 }, bindings: [[0, { kind: "var", var: 0 }]] }],
					negated: [],
					conditions: []
				}
			]
		}
		const preparedResult = native.dbPrepare(db, parseQueryIr(queryIr))
		assert.ok(preparedResult.ok, "the recursive query prepares")
		prepared = preparedResult.prepared

		native.dbRead(db, function read(instance, _witness) {
			const rows = native.preparedExecute(prepared, instance, [{ kind: "u64", value: p1 }])
			const reachable: bigint[] = []
			for (const row of rows) {
				assert.equal(row.length, 1)
				const cell = row[0]
				assert.equal(typeof cell, "bigint")
				if (typeof cell === "bigint") {
					reachable.push(cell)
				}
			}
			assert.deepEqual(sortedBigints(reachable), sortedBigints([p1, p2, p3]), "p1 → p2 → p3 → p1 closes; p4 stays out")
		})
	})

	test("dbPrepare returns roster errors as data", function irError() {
		const bogus: QueryIr = {
			kind: "cq",
			interiors: [],
			head: [{ kind: "var" }],
			rules: [
				{
					finds: [{ kind: "var", var: 0 }],
					atoms: [{ source: { kind: "edb", relation: 999 }, bindings: [[0, { kind: "var", var: 0 }]] }],
					negated: [],
					conditions: []
				}
			]
		}
		const outcome = native.dbPrepare(db, parseQueryIr(bogus))
		assert.equal(outcome.ok, false)
		assert.equal(outcome.kind, "irError")
		assert.notEqual(outcome.message, "")
	})

	test("count_with_over_is_refused", function countWithOver() {
		assert.throws(function marshalCountOver() {
			native.dbPrepare(db, {
				kind: "cq",
				interiors: [],
				head: [{ kind: "aggregate", op: "count" }],
				rules: [
					{
						finds: [{ kind: "count", over: 0 }],
						atoms: [],
						negated: [],
						conditions: []
					}
				]
			} as unknown as ParsedQuery)
		}, /Count carries no over/)
	})

	test("the generation witness: moved as data, fresh witness commits", function witness() {
		let stale: WitnessHandle | undefined
		native.dbRead(db, function capture(_instance, witness) {
			stale = witness
		})
		const staleWitness = stale
		assert.ok(staleWitness)

		const moved = native.dbWrite(db, function write(mover) {
			const p5 = mintedStart(native.txReserve(mover, PERSON, 0, 1n))
			assert.deepEqual(
				native.txInsert(
					mover,
					PERSON,
					1n,
					personRow(p5, "kay", 0n, 1n, new Uint8Array([21, 22, 23, 24]), { start: 0n, end: 1n }, true)
				),
				{ submitted: 1n, changed: 1n }
			)
			return true
		})
		assert.equal(moved.tag, "accepted")

		const refused = native.dbWriteFrom(db, staleWitness, function refusedWrite() {
			return true
		})
		assert.equal(refused.tag, "moved", "a state-changing commit after the witness refuses the write")
		assert.ok(refused.current > refused.witnessed)

		native.dbRead(db, function commitFromFresh(_instance, fresh) {
			const landed = native.dbWriteFrom(db, fresh, function insert(tx) {
				assert.deepEqual(native.txInsert(tx, EDGE, 1n, [p2, p1, 3n]), { submitted: 1n, changed: 1n })
				return true
			})
			assert.equal(landed.tag, "accepted", "a fresh witness admits the write")
		})
		native.witnessClose(staleWitness)
	})

	test("open outcomes: schemaError and fingerprintMismatch as data", async function openOutcomes() {
		const badSpec: SchemaSpec = {
			relations: spec.relations,
			statements: [{ kind: "fd", relation: "Edge", projection: ["nope"] }]
		}
		const badCreate = await native.dbCreate(path.join(tmpRoot, "bad"), badSpec)
		assert.equal(badCreate.tag, "schemaError")
		assert.match(badCreate.message, /nope/)

		native.preparedClose(prepared)
		dbClose(db)

		const otherSpec: SchemaSpec = {
			relations: spec.relations,
			statements: spec.statements.slice(0, 3)
		}
		const mismatched = await native.dbOpen(storeDir, otherSpec)
		assert.ok(!mismatched.ok, "a different theory cannot open the store")
		assert.equal(mismatched.kind, "fingerprintMismatch")

		const reopened = await native.dbOpen(storeDir, spec)
		assert.ok(reopened.ok, "the same theory reopens the store")
		db = reopened.db
		native.dbRead(db, function read(instance, _witness) {
			assert.equal(native.instanceScan(instance, EDGE).length, 4, "resume = reopen: the data survived")
		})

		let spent: TxHandle | undefined
		native.dbWrite(db, function write(tx) {
			spent = tx
			return false
		})
		function insertOnSpent(): void {
			native.txInsert(spent as TxHandle, EDGE, 1n, [1n, 2n, 3n])
		}
		assert.throws(insertOnSpent, /closed/, "a spent transaction handle throws typed")

		dbClose(db)
		assert.throws(() => dbClose(db), /closed/, "double close throws typed")
	})
})
