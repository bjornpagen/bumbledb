/**
 * OPEN-ledger row (b) — multi-key typed `get` (70-api.md § the freeze;
 * TODO.md Phase C verdict table). The 2-arg `get` reads ONLY through the
 * PRIMARY candidate key (marshal.ts PRIMARY-KEY RULE); a declared
 * secondary key — graph-builder's `key(program, ["grp"])` shape exactly —
 * reads through the key-statement-selected form `get(relation,
 * keyStatement, key)`, whose key object is typed by the statement's own
 * projection and whose statement id resolves from the SDK's positional
 * mirror (the native bridge's `snapshotGet(snap, relation, keyStatement,
 * keyValues)` always point-read through ANY key statement; the typed
 * surface now expresses it). Tests pin all sides: the primary form, the
 * 2-arg refusal of a secondary-key object, the typed keyed read, and the
 * engine's own secondary-key read underneath.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import type { Fact } from "#index.ts"
import { Db, key, relation, schema, str, u64 } from "#index.ts"
import { lower } from "#lower.ts"
import { native } from "#native.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-openledger-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/**
 * The graph-builder shape in miniature: a fresh-bearing relation whose
 * DECLARED key (`key(Program, ["grp"])`, schema.ts programGrpKey) is the
 * lookup the driver actually performs (store-reads.ts programNeighbor,
 * dispatch.ts settleRealize/settleAuthor/settleReviewEdge).
 */
const Grp = relation("Grp", { id: u64.fresh, label: str })
const Program = relation("Program", { id: u64.fresh, grp: u64, title: str })
const programGrpKey = key(Program, ["grp"])
const Theory = schema("OpenLedgerB", { Grp, Program }, [programGrpKey])

describe("OPEN-ledger row (b): typed get through a declared secondary key", async function suite() {
	const db = await Db.create(path.join(tmpRoot, "store"), Theory)

	let grpId: Fact<typeof Grp>["id"] | undefined
	let programId: Fact<typeof Program>["id"] | undefined
	const seeded = db.write(function seed(tx) {
		const g = tx.insert(Grp, { label: "algebra" })
		grpId = g.id
		const p = tx.insert(Program, { grp: g.id, title: "linear equations" })
		programId = p.id
	})
	assert.ok(seeded.ok, "seed commit lands")
	assert.ok(grpId !== undefined && programId !== undefined)
	const grp = grpId
	const program = programId

	test("primary-key get works (the fresh field)", function primary() {
		const row = db.get(Program, { id: program })
		assert.ok(row)
		assert.equal(row.grp, grp)
	})

	test("the 2-arg get refuses a declared-key object — the primary form stays primary-only", function refusal() {
		/**
		 * KeyFact<Program> demands exactly { id } (fresh present), so the
		 * declared-key object is refused at compile time; the runtime
		 * projection check throws the same refusal.
		 */
		assert.throws(
			function getByDeclaredKey() {
				// @ts-expect-error — KeyFact demands exactly the fresh field; the declared key needs the 3-arg form
				db.get(Program, { grp })
			},
			/missing field id/,
			"the 2-arg get reads only through the primary key"
		)
	})

	test("the key-statement-selected get point-reads through the declared key, typed", function keyedGet() {
		/**
		 * The exact lookup graph-builder performs at every programNeighbor /
		 * settle* site, as one typed point read instead of scan().find().
		 */
		const row = db.get(Program, programGrpKey, { grp })
		assert.ok(row, "the declared key answers the typed point read")
		assert.equal(row.id, program)
		assert.equal(row.title, "linear equations")
		assert.equal(
			db.read(function inScope(snap) {
				return snap.get(Program, programGrpKey, { grp })?.id
			}),
			program,
			"the scoped spelling agrees (the symmetry rule)"
		)
		/**
		 * A statement of another schema (or a non-key statement) is a typed
		 * refusal, and a foreign-relation key never crosses relations.
		 */
		const foreignKey = key(Program, ["title"])
		assert.throws(function foreignStatement() {
			db.get(Program, foreignKey, { title: "linear equations" })
		}, /not a declared statement of schema OpenLedgerB/)
		assert.throws(function wrongOwner() {
			// @ts-expect-error — the statement keys Program, not Grp; the key object is typed by Program's projection
			db.get(Grp, programGrpKey, { grp })
		}, /keys Program, not Grp/)
	})

	test("scan().find() still works (the workaround the driver used before the keyed form)", function workaround() {
		const row = db.read(function findByGrp(snap) {
			return snap.scan(Program).find(function forGroup(candidate) {
				return candidate.grp === grp
			})
		})
		assert.ok(row, "the host full-scan spelling remains available")
		assert.equal(row.id, program)
	})

	test("the engine point-reads through the declared key statement underneath", function engineSide() {
		/**
		 * Same theory, raw native store: the bridge's snapshotGet takes ANY
		 * key statement id — the declared secondary key included — proving
		 * the SDK typed surface is the only layer withholding the read.
		 */
		const spec = lower(Theory)
		const created = native.dbCreate(path.join(tmpRoot, "native"), spec)
		assert.ok(created.ok, "native create succeeds")
		const handle = created.db
		const manifest = native.dbManifest(handle)
		const programRel = manifest.relations.find(function byName(entry) {
			return entry.name === "Program"
		})
		assert.ok(programRel)
		const declaredKey = manifest.statements.find(function byForm(statement) {
			return statement.kind === "functionality" && statement.spelling.startsWith("Program(grp)")
		})
		assert.ok(declaredKey, "the declared key(Program, [grp]) statement is in the manifest")
		const freshKey = manifest.statements.find(function byForm(statement) {
			return statement.kind === "functionality" && statement.spelling.startsWith("Program(id)")
		})
		assert.ok(freshKey, "the fresh auto-key is in the manifest")
		assert.notEqual(declaredKey.id, freshKey.id, "the declared key is a SECONDARY statement")

		const grpRel = manifest.relations.find(function byName(entry) {
			return entry.name === "Grp"
		})
		assert.ok(grpRel)
		const tx = native.dbWriteBegin(handle)
		const g = native.txAlloc(tx, grpRel.id, 0)
		assert.equal(native.txInsert(tx, grpRel.id, [g, "algebra"]), true)
		const p = native.txAlloc(tx, programRel.id, 0)
		assert.equal(native.txInsert(tx, programRel.id, [p, g, "linear equations"]), true)
		const outcome = native.txCommit(tx)
		assert.ok(outcome.ok, "native seed commits")

		const snap = native.dbSnapshot(handle)
		const byGrp = native.snapshotGet(snap, programRel.id, declaredKey.id, [g])
		native.snapshotClose(snap)
		native.dbClose(handle)
		assert.deepEqual(
			byGrp,
			[p, g, "linear equations"],
			"the engine answers the secondary-key point read the SDK cannot express"
		)
	})
})
