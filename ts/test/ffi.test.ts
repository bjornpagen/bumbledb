/**
 * PRD-04 FFI semantic pins against a REAL durable store in a temp dir — the
 * SDK's ground truth for the bridge: create with a spec using every field
 * type, both closed tiers, and all three statement forms; delta writes with
 * fresh-mint return and live final-state point reads; one violation of each
 * statement form arriving with canonical spellings and decoded facts; a
 * recursive closure query; the generation witness; manifest and open-error
 * outcomes.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import type { DbHandle, FactValue, Manifest, PreparedHandle, ProgramIr, SnapshotHandle } from "#native.ts"
import { native } from "#native.ts"
import type { SchemaSpec } from "#spec.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-ffi-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/** Relation ids by declaration order in the spec below. */
const STATUS = 0
const KIND = 1
const PERSON = 2
const EDGE = 3

/**
 * The test theory: every field type (bool, u64 incl. fresh, i64, str,
 * bytes<4>, interval<i64>), both closed tiers (bare Status, columned Kind),
 * and all three statement forms (fd, containment, cardinality window with a
 * handle-literal selection).
 */
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
				// `from` pairs `Person.id` in the containment and the window
				// below, so the coherence wall requires the shared label
				// (M5: the faces of a dependency agree on their newtype, or
				// neither carries one); `to` sits in no paired-face
				// statement and stays deliberately bare.
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
			kind: "cardinality",
			target: {
				relation: "Person",
				projection: ["id"],
				selection: [["status", { kind: "one", literal: { kind: "handle", handle: "Frozen" } }]]
			},
			window: { kind: "exact", n: 0n },
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
	let openSnapshots: SnapshotHandle[] = []
	let p1 = 0n
	let p2 = 0n
	let p3 = 0n
	let p4 = 0n

	function snapshot(): SnapshotHandle {
		const snap = native.dbSnapshot(db).snapshot
		openSnapshots.push(snap)
		return snap
	}

	function closeSnapshots(): void {
		for (const snap of openSnapshots) {
			native.snapshotClose(snap)
		}
		openSnapshots = []
	}

	test("engineVersion is a non-empty proof string", function version() {
		assert.equal(typeof native.engineVersion(), "string")
		assert.notEqual(native.engineVersion(), "")
	})

	test("dbCreate + manifest carries every name→id table", function create() {
		const created = native.dbCreate(storeDir, spec)
		assert.ok(created.ok, "create succeeds on a fresh directory")
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

	test("delta writes: fresh mint, final-state point reads, commit", function writes() {
		const tx = native.dbWriteBegin(db)
		p1 = native.txAlloc(tx, PERSON, 0)
		p2 = native.txAlloc(tx, PERSON, 0)
		p3 = native.txAlloc(tx, PERSON, 0)
		p4 = native.txAlloc(tx, PERSON, 0)
		assert.equal(typeof p1, "bigint")
		assert.equal(new Set([p1, p2, p3, p4]).size, 4, "fresh mints are distinct")

		const active = { start: -5n, end: 10n }
		const adaRow = personRow(p1, "ada", 0n, -3n, new Uint8Array([1, 2, 3, 4]), active, true)
		const rows = [
			adaRow,
			personRow(p2, "grace", 0n, 7n, new Uint8Array([5, 6, 7, 8]), active, false),
			personRow(p3, "alan", 0n, 0n, new Uint8Array([9, 10, 11, 12]), active, true),
			personRow(p4, "kurt", 1n, 42n, new Uint8Array([13, 14, 15, 16]), active, false)
		]
		for (const row of rows) {
			assert.equal(native.txInsert(tx, PERSON, row), true)
		}

		assert.equal(native.txContains(tx, PERSON, adaRow), true, "final-state view sees the pending insert")
		const got = native.txGet(tx, PERSON, personKeyId, [p1])
		assert.ok(got, "point read through the fresh key")
		assert.equal(got[1], "ada")
		assert.deepEqual(got[4], new Uint8Array([1, 2, 3, 4]))
		assert.deepEqual(got[5], active)

		assert.equal(native.txInsert(tx, EDGE, [p1, p2, 1n]), true)
		assert.equal(native.txInsert(tx, EDGE, [p2, p3, 1n]), true)
		assert.equal(native.txInsert(tx, EDGE, [p3, p1, 1n]), true)

		assert.equal(native.txInsert(tx, EDGE, [p1, p3, 7n]), true)
		assert.equal(native.txContains(tx, EDGE, [p1, p3, 7n]), true)
		assert.equal(native.txDelete(tx, EDGE, [p1, p3, 7n]), true, "delta delete cancels the pending insert")
		assert.equal(native.txContains(tx, EDGE, [p1, p3, 7n]), false)

		const committed = native.txCommit(tx)
		assert.ok(committed.ok, "the clean commit lands")
		assert.equal(typeof committed.generation, "bigint")
		assert.equal(native.dbGeneration(db), committed.generation)
	})

	test("snapshot reads: scan, contains, keyed get", function reads() {
		const snap = snapshot()
		const edges = native.snapshotScan(snap, EDGE)
		assert.equal(edges.length, 3)
		assert.equal(
			native.snapshotContains(
				snap,
				PERSON,
				personRow(p1, "ada", 0n, -3n, new Uint8Array([1, 2, 3, 4]), { start: -5n, end: 10n }, true)
			),
			true
		)
		const edge = native.snapshotGet(snap, EDGE, edgeKeyId, [p1, p2])
		assert.ok(edge, "keyed get finds the edge")
		assert.equal(edge[2], 1n)
		assert.equal(native.snapshotGet(snap, EDGE, edgeKeyId, [p2, p1]), null)
	})

	test("a functionality violation arrives canonical and decoded", function fdViolation() {
		const tx = native.dbWriteBegin(db)
		assert.equal(native.txInsert(tx, EDGE, [p1, p2, 9n]), true)
		const outcome = native.txCommit(tx)
		assert.ok(!outcome.ok, "the key judgment rejects")
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
		const tx = native.dbWriteBegin(db)
		assert.equal(native.txInsert(tx, EDGE, [ghost, p1, 1n]), true)
		assert.equal(native.txInsert(tx, EDGE, [p4, p1, 1n]), true)
		const outcome = native.txCommit(tx)
		assert.ok(!outcome.ok, "the statement judgment rejects")
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

		const window = outcome.violations.find(function byKind(violation) {
			return violation.kind === "cardinality"
		})
		assert.ok(window, "the window citation is present")
		assert.equal(window.count, 1n)
		assert.equal(window.canonical, spellingOf(manifest, window.statementId))
		assert.equal(window.facts[0]?.relation, "Person", "the convicted parent is the cited fact")
		const parentId = window.facts[0]?.fields.find(function field(entry) {
			return entry.name === "id"
		})
		assert.deepEqual(parentId, { name: "id", value: p4 })
	})

	test("recursive closure query computes the reachable set", function closure() {
		const program: ProgramIr = {
			predicates: [
				{
					head: [{ kind: "var" }],
					rules: [
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
						},
						{
							finds: [{ kind: "var", var: 1 }],
							atoms: [
								{ source: { kind: "idb", pred: 0 }, bindings: [[0, { kind: "var", var: 0 }]] },
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
				}
			],
			output: 0
		}
		const preparedResult = native.dbPrepare(db, program)
		assert.ok(preparedResult.ok, "the recursive program prepares")
		prepared = preparedResult.prepared

		const snap = snapshot()
		const rows = native.preparedExecute(prepared, snap, [{ kind: "u64", value: p1 }])
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

		const staleness = native.preparedStaleness(prepared, snap)
		assert.equal(typeof staleness.maxRatio, "number")
		assert.ok(staleness.maxRatio >= 1)
		for (const drift of staleness.perOccurrence) {
			assert.equal(typeof drift.pinned, "bigint")
			assert.equal(typeof drift.live, "bigint")
		}
	})

	test("dbPrepare returns roster errors as data", function irError() {
		const bogus: ProgramIr = {
			predicates: [
				{
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
			],
			output: 0
		}
		const outcome = native.dbPrepare(db, bogus)
		assert.ok(!outcome.ok)
		assert.equal(outcome.kind, "irError")
		assert.notEqual(outcome.message, "")
	})

	test("the generation witness: moved as data, fresh witness commits", function witness() {
		const stale = snapshot()

		const mover = native.dbWriteBegin(db)
		const p5 = native.txAlloc(mover, PERSON, 0)
		assert.equal(
			native.txInsert(
				mover,
				PERSON,
				personRow(p5, "kay", 0n, 1n, new Uint8Array([21, 22, 23, 24]), { start: 0n, end: 1n }, true)
			),
			true
		)
		const moved = native.txCommit(mover)
		assert.ok(moved.ok)

		const refused = native.dbWriteFrom(db, stale)
		assert.ok(!refused.ok, "a state-changing commit after the witness refuses the write")
		assert.equal(refused.kind, "generationMoved")
		assert.ok(refused.current > refused.witnessed)

		// The convicted 018 sequence: the witness is data, so the snapshot may
		// close mid-transaction — dbWriteFrom → snapshotClose → txInsert/txCommit.
		const fresh = native.dbSnapshot(db).snapshot
		const witnessed = native.dbWriteFrom(db, fresh)
		assert.ok(witnessed.ok, "a fresh witness admits the write")
		native.snapshotClose(fresh)
		assert.equal(native.txInsert(witnessed.tx, EDGE, [p2, p1, 3n]), true)
		const landed = native.txCommit(witnessed.tx)
		assert.ok(landed.ok, "the witnessed commit survives its snapshot's mid-transaction close")
	})

	test("open outcomes: schemaError and fingerprintMismatch as data", function openOutcomes() {
		const badSpec: SchemaSpec = {
			relations: spec.relations,
			statements: [{ kind: "fd", relation: "Edge", projection: ["nope"] }]
		}
		const badCreate = native.dbCreate(path.join(tmpRoot, "bad"), badSpec)
		assert.ok(!badCreate.ok)
		assert.equal(badCreate.kind, "schemaError")
		assert.match(badCreate.message, /nope/)

		native.preparedClose(prepared)
		closeSnapshots()
		native.dbClose(db)

		const otherSpec: SchemaSpec = {
			relations: spec.relations,
			statements: spec.statements.slice(0, 3)
		}
		const mismatched = native.dbOpen(storeDir, otherSpec)
		assert.ok(!mismatched.ok, "a different theory cannot open the store")
		assert.equal(mismatched.kind, "fingerprintMismatch")

		const reopened = native.dbOpen(storeDir, spec)
		assert.ok(reopened.ok, "the same theory reopens the store")
		db = reopened.db
		const snap = snapshot()
		assert.equal(native.snapshotScan(snap, EDGE).length, 4, "resume = reopen: the data survived")
		closeSnapshots()

		const spent = native.dbWriteBegin(db)
		native.txAbort(spent)
		function insertOnSpent(): void {
			native.txInsert(spent, EDGE, [1n, 2n, 3n])
		}
		assert.throws(insertOnSpent, /closed/, "a spent transaction handle throws typed")

		native.dbClose(db)
		assert.throws(() => native.dbClose(db), /closed/, "double close throws typed")
	})
})
