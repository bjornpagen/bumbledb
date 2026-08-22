import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"
import type { Db as DbValue } from "#index.ts"
import { closed, contained, Db, on, relation, renderStatement, schema, str, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cind-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const TaskKind = closed("TaskKind", ["Author", "Enrich"])
const Grp = relation("Grp", { id: u64.fresh, label: str })

const Task = relation("Task", { id: u64.fresh, kind: TaskKind.id, subject: u64 })

const authorSubjectIsGrp = contained(on(Task.where({ kind: "Author" }), "subject"), on(Grp, "id"))

const kindVocab = contained(on(Task, "kind"), on(TaskKind, "id"))

const Ledger = schema("Ledger", { TaskKind, Grp, Task }, [kindVocab, authorSubjectIsGrp])

function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

describe("C-07 refutation: the selected-source containment is statable and enforced", function suite() {
	let db: DbValue<(typeof Ledger)["relations"]>
	let grpId: bigint

	test("Db.create admits Task(subject | kind == Author) <= Grp(id)", async function create() {
		db = accepted(await Db.create(storeDir, Ledger))
		assert.equal(db.schema, Ledger)
		assert.equal(renderStatement(authorSubjectIsGrp), "Task(subject | kind == Author) <= Grp(id)")
	})

	test("a fresh Author task with a dangling subject is unwritable (source side)", function danglingMint() {
		const rejected = db.write(function mintDead(tx) {
			put(tx, Task, { kind: "Author", subject: 999n })
		})
		assert.equal(rejected.tag, "rejected", "the CIND judges the inserted source fact")
		const violation = must(rejected.violations[0])
		assert.equal(violation.kind, "containment")
		assert.strictEqual(violation.statement, authorSubjectIsGrp)
	})

	test("a non-Author task's subject is outside φ — kind-scoping holds", function scopedFreedom() {
		const accepted = db.write(function mintEnrich(tx) {
			put(tx, Task, { kind: "Enrich", subject: 999n })
		})
		assert.equal(accepted.tag, "accepted", "the selection scopes the law to Author rows only")
	})

	test("the repartition shape — deleting a grp whose Author task survives — is unwritable (target side)", function repartition() {
		const seeded = db.write(function seed(tx) {
			const grp = put(tx, Grp, { label: "sheet-1" })
			grpId = grp.id
			put(tx, Task, { kind: "Author", subject: grp.id })
		})
		assert.equal(seeded.tag, "accepted", "the well-founded pair lands")

		const rejected = db.write(function honorRepartition(tx) {
			assert.equal(tx.delete(Grp, [{ id: grpId, label: "sheet-1" }]).changed, 1n)
		})
		assert.equal(rejected.tag, "rejected", "the surviving Author task pins its grp")
		const violation = must(rejected.violations[0])
		assert.equal(violation.kind, "containment")
		assert.strictEqual(violation.statement, authorSubjectIsGrp)
		assert.equal(violation.direction, "targetRequired")
	})
})
