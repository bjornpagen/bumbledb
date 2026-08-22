import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import type { Fact } from "#index.ts"
import { Db, interval, key, relation, schema, str, u64 } from "#index.ts"
import { lower } from "#lower.ts"
import { dbClose, native } from "#native.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

function mintedStart(range: { empty: true } | { empty: false; start: bigint }): bigint {
	if (range.empty) {
		throw new Error("expected nonempty fresh range")
	}
	return range.start
}

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-keyedget-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Grp = relation("Grp", { id: u64.fresh, label: str })
const Program = relation("Program", { id: u64.fresh, grp: u64, title: str })
const programGrpKey = key(Program, ["grp"])
const Theory = schema("KeyedGet", { Grp, Program }, [programGrpKey])

describe("keyed get: typed point reads through a declared key statement", async function suite() {
	const db = accepted(await Db.create(path.join(tmpRoot, "store"), Theory))

	let grpId: Fact<typeof Grp>["id"] | undefined
	let programId: Fact<typeof Program>["id"] | undefined
	const seeded = db.write(function seed(tx) {
		const g = put(tx, Grp, { label: "algebra" })
		grpId = g.id
		const p = put(tx, Program, { grp: g.id, title: "linear equations" })
		programId = p.id
	})
	assert.equal(seeded.tag, "accepted", "seed commit lands")
	assert.ok(grpId !== undefined && programId !== undefined)
	const grp = grpId
	const program = programId

	test("primary-key get works (the fresh field)", function primary() {
		const row = db.read((i) => i.get(Program, { id: program }))
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
				db.read((i) => i.get(Program, { grp }))
			},
			/missing field id/,
			"the 2-arg get reads only through the primary key"
		)
	})

	test("the key-statement-selected get point-reads through the declared key, typed", function keyedGet() {

		const row = db.read((i) => i.get(Program, programGrpKey, { grp }))
		assert.ok(row, "the declared key answers the typed point read")
		assert.equal(row.id, program)
		assert.equal(row.title, "linear equations")
		assert.equal(
			db.read(function inScope(instance, _witness) {
				return instance.get(Program, programGrpKey, { grp })?.id
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
			db.read((i) => i.get(Program, foreignKey, { title: "linear equations" }))
		}, /not a declared statement of schema KeyedGet/)
		assert.throws(function wrongOwner() {
			// @ts-expect-error — the statement keys Program, not Grp; the key object is typed by Program's projection
			db.read((i) => i.get(Grp, programGrpKey, { grp }))
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
			db.read((i) => i.get(Program, programGrpKey, { title: "x" }))
		}, /missing field grp/)
	})

	test("the write transaction point-reads through the declared key, final-state", function txKeyed() {
		let freshGrp: Fact<typeof Grp>["id"] | undefined
		let preCommit: Fact<typeof Program> | undefined
		const outcome = db.write(function mutate(tx) {
			const g = put(tx, Grp, { label: "geometry" })
			const p = put(tx, Program, { grp: g.id, title: "proofs" })
			const pending = tx.get(Program, programGrpKey, { grp: g.id })
			assert.ok(pending, "the pending insert answers the keyed final-state read (read-your-writes)")
			assert.equal(pending.id, p.id, "the minted id comes back through the declared key")
			assert.equal(pending.title, "proofs")
			assert.equal(tx.delete(Program, [pending]).changed, 1n, "the delete lands on the final state")
			preCommit = tx.get(Program, programGrpKey, { grp: g.id })
			assert.equal(preCommit, undefined, "the delta Absent overlay answers the same keyed read")
			freshGrp = g.id
		})
		assert.equal(outcome.tag, "accepted", "the commit lands")
		if (freshGrp === undefined) {
			throw new Error("the write minted a group id")
		}
		const committedGrp = freshGrp
		assert.equal(
			db.read((i) => i.get(Program, programGrpKey, { grp: committedGrp })),
			preCommit,
			"the committed keyed answer agrees with the pre-commit one"
		)
	})

	test("writeFrom sees one spelling on both hands", function witnessed() {
		const outcome = db.read(function bothHands(instance, witness) {
			const committed = instance.get(Program, programGrpKey, { grp })
			assert.ok(committed, "the snapshot hand answers the keyed committed-state read")
			assert.equal(committed.id, program)
			return db.writeFrom(witness, function delta(tx) {
				const g = put(tx, Grp, { label: "calculus" })
				const p = put(tx, Program, { grp: g.id, title: "limits" })
				const pending = tx.get(Program, programGrpKey, { grp: g.id })
				assert.ok(pending, "the transaction hand answers the keyed final-state read")
				assert.equal(pending.id, p.id)
				assert.equal(
					instance.get(Program, programGrpKey, { grp: g.id }),
					undefined,
					"the snapshot hand still witnesses only committed state"
				)
			})
		})
		assert.equal(outcome.tag, "accepted", "the witnessed write commits")
	})

	test("full-scan find remains available (hosts may still fold)", function fullScan() {
		const row = db.read(function findByGrp(instance, _witness) {
			return instance.scan(Program).find(function forGroup(candidate) {
				return candidate.grp === grp
			})
		})
		assert.ok(row, "the host full-scan spelling remains available")
		assert.equal(row.id, program)
	})

	test("the engine point-reads through the declared key statement underneath", async function engineSide() {

		const spec = lower(Theory)
		const created = await native.dbCreate(path.join(tmpRoot, "native"), spec)
		assert.equal(created.tag, "accepted", "native create succeeds")
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
		let g = 0n
		let p = 0n
		const outcome = native.dbWrite(handle, function write(tx) {
			g = mintedStart(native.txReserve(tx, grpRel.id, 0, 1n))
			assert.deepEqual(native.txInsert(tx, grpRel.id, 1n, [g, "algebra"]), { submitted: 1n, changed: 1n })
			p = mintedStart(native.txReserve(tx, programRel.id, 0, 1n))
			assert.deepEqual(native.txInsert(tx, programRel.id, 1n, [p, g, "linear equations"]), {
				submitted: 1n,
				changed: 1n
			})
			return true
		})
		assert.equal(outcome.tag, "accepted", "native seed commits")

		const byGrp = native.dbRead(handle, function read(instance, _witness) {
			return native.instanceGet(instance, programRel.id, declaredKey.id, [g])
		})
		dbClose(handle)
		assert.deepEqual(
			byGrp,
			[p, g, "linear equations"],
			"the engine answers the same secondary-key point read the typed surface expresses"
		)
	})
})

describe("keyed get: the statement-vs-key dispatch is a brand, never a shape probe (134)", async function brandSuite() {

	const Cfg = relation("Cfg", { data: interval(u64), value: u64 })
	const BrandTheory = schema("KeyedGetBrand", { Cfg }, [key(Cfg, ["data"])])
	const db = accepted(await Db.create(path.join(tmpRoot, "brand-store"), BrandTheory))
	const committed = db.write(function seed(tx) {
		put(tx, Cfg, { data: { start: 1n, end: 2n }, value: 7n })
	})
	assert.equal(committed.tag, "accepted", "seed commit lands")

	test("an interval key cell with an excess kind property dispatches as a key object", function excessKind() {
		const withKind: { start: bigint; end: bigint; kind: string } = { start: 1n, end: 2n, kind: "window" }
		const row = db.read((i) => i.get(Cfg, { data: withKind }))
		assert.ok(row, "the keyed read lands — no statement-selector misdispatch")
		assert.equal(row.value, 7n)
	})
})
