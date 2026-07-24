/**
 * Keyed get — the SHIPPED spelling (docs/architecture/70-api.md
 * § Transactions; the closed ledger's "multi-key typed `tx.get`" row, FIRED
 * census 2026-07-17). The 2-arg `get` reads ONLY through the PRIMARY
 * candidate key (marshal.ts PRIMARY-KEY RULE); a declared secondary key —
 * graph-builder's `key(program, ["grp"])` shape exactly — reads through the
 * key-statement-selected form `get(relation, keyStatement, key)`, whose key
 * object is typed by the statement's own projection and whose statement id
 * resolves from the SDK's positional mirror. Keyed get is the obvious
 * spelling on the read scope AND the write transaction — `db.get`,
 * `snap.get`, and `tx.get` all carry the 3-arg form (the symmetry rule; the
 * transaction side answers FINAL state, base + pending delta). Tests pin
 * all sides: the primary form, the 2-arg refusal of a secondary-key object,
 * the typed keyed read on every scope, the projection typing, the statement
 * membership refusals, and the engine's own secondary-key read underneath.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import type { Fact } from "#index.ts"
import { Db, interval, key, relation, schema, str, u64 } from "#index.ts"
import { lower } from "#lower.ts"
import { native } from "#native.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-keyedget-"))

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
const Theory = schema("KeyedGet", { Grp, Program }, [programGrpKey])

describe("keyed get: typed point reads through a declared key statement", async function suite() {
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
		}, /not a declared statement of schema KeyedGet/)
		assert.throws(function wrongOwner() {
			// @ts-expect-error — the statement keys Program, not Grp; the key object is typed by Program's projection
			db.get(Grp, programGrpKey, { grp })
		}, /keys Program, not Grp/)
	})

	test("the key object is typed by the statement's projection — a wrong field name is refused", function wrongProjection() {
		/**
		 * DeclaredKeyFact<Program, ["grp"]> types the determinant columns
		 * from the key-FD statement itself, so a key object spelling a
		 * non-determinant field fails to compile; the runtime projection
		 * check throws the matching refusal.
		 */
		assert.throws(function wrongField() {
			// @ts-expect-error — programGrpKey's projection is (grp); `title` is not a determinant column of the statement
			db.get(Program, programGrpKey, { title: "x" })
		}, /missing field grp/)
	})

	test("the write transaction point-reads through the declared key, final-state", function txKeyed() {
		let freshGrp: Fact<typeof Grp>["id"] | undefined
		let preCommit: Fact<typeof Program> | undefined
		const outcome = db.write(function mutate(tx) {
			const g = tx.insert(Grp, { label: "geometry" })
			const p = tx.insert(Program, { grp: g.id, title: "proofs" })
			const pending = tx.get(Program, programGrpKey, { grp: g.id })
			assert.ok(pending, "the pending insert answers the keyed final-state read (read-your-writes)")
			assert.equal(pending.id, p.id, "the minted id comes back through the declared key")
			assert.equal(pending.title, "proofs")
			assert.equal(tx.delete(Program, pending), true, "the delete lands on the final state")
			preCommit = tx.get(Program, programGrpKey, { grp: g.id })
			assert.equal(preCommit, undefined, "the delta Absent overlay answers the same keyed read")
			freshGrp = g.id
		})
		assert.ok(outcome.ok, "the commit lands")
		assert.ok(freshGrp !== undefined)
		assert.equal(
			db.get(Program, programGrpKey, { grp: freshGrp }),
			preCommit,
			"the committed keyed answer agrees with the pre-commit one"
		)
	})

	test("writeWitnessed sees one spelling on both hands", function witnessed() {
		const outcome = db.writeWitnessed(function bothHands(snap, tx) {
			const committed = snap.get(Program, programGrpKey, { grp })
			assert.ok(committed, "the snapshot hand answers the keyed committed-state read")
			assert.equal(committed.id, program)
			const g = tx.insert(Grp, { label: "calculus" })
			const p = tx.insert(Program, { grp: g.id, title: "limits" })
			const pending = tx.get(Program, programGrpKey, { grp: g.id })
			assert.ok(pending, "the transaction hand answers the keyed final-state read")
			assert.equal(pending.id, p.id)
			assert.equal(
				snap.get(Program, programGrpKey, { grp: g.id }),
				undefined,
				"the snapshot hand still witnesses only committed state"
			)
		})
		assert.ok(outcome.ok, "the witnessed write commits")
	})

	test("full-scan find remains available (hosts may still fold)", function fullScan() {
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
		 * key statement id — the declared secondary key included — and the
		 * SDK's typed keyed form rides exactly this read.
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

		const snap = native.dbSnapshot(handle).snapshot
		const byGrp = native.snapshotGet(snap, programRel.id, declaredKey.id, [g])
		native.snapshotClose(snap)
		native.dbClose(handle)
		assert.deepEqual(
			byGrp,
			[p, g, "linear equations"],
			"the engine answers the same secondary-key point read the typed surface expresses"
		)
	})
})

describe("keyed get: the statement-vs-key dispatch is a brand, never a shape probe (134)", async function brandSuite() {
	/**
	 * The conjunction the old `data.kind` shape probe misread: an
	 * interval-typed PRIMARY-KEY field literally named `data`, keyed with a
	 * structurally-open interval value carrying an excess `kind` property —
	 * a legal cell everywhere else in the SDK (cellOf strips extras). The
	 * admission brand makes the misdispatch unrepresentable.
	 */
	const Cfg = relation("Cfg", { data: interval(u64), value: u64 })
	const BrandTheory = schema("KeyedGetBrand", { Cfg }, [key(Cfg, ["data"])])
	const db = await Db.create(path.join(tmpRoot, "brand-store"), BrandTheory)
	const committed = db.write(function seed(tx) {
		tx.insert(Cfg, { data: { start: 1n, end: 2n }, value: 7n })
	})
	assert.ok(committed.ok, "seed commit lands")

	test("an interval key cell with an excess kind property dispatches as a key object", function excessKind() {
		const withKind: { start: bigint; end: bigint; kind: string } = { start: 1n, end: 2n, kind: "window" }
		const row = db.get(Cfg, { data: withKind })
		assert.ok(row, "the keyed read lands — no statement-selector misdispatch")
		assert.equal(row.value, 7n)
	})
})
