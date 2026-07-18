/**
 * Refutation pin for finding C-07: "kind-scoped reference integrity is
 * unstatable — contained() does not admit where()-selected SOURCE faces."
 * The grammar (`docs/architecture/30-dependencies.md` § Containment) reads
 * `A(X | φ) <= B(Y | ψ)` with φ on the SOURCE, and the surface carries it:
 * this file states the finder's exact per-kind containment
 * `Task(subject | kind == Author) <= Grp(id)`, watches `Db.create` admit
 * it, and proves BOTH dangling directions unwritable — a fresh Author task
 * minting a dead subject (source side) and the repartition shape (deleting
 * a grp whose Author task survives, target side) — while a non-Author
 * task's subject stays free, exactly the kind-scoping the finding calls
 * impossible.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import type { Db as DbValue } from "#index.ts"
import { closed, contained, Db, on, relation, renderStatement, schema, str, u64 } from "#index.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-cind-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const TaskKind = closed("TaskKind", ["Author", "Enrich"])
const Grp = relation("Grp", { id: u64.fresh, label: str })
/**
 * `subject` is a bare u64 — the LAW types the column: the ψ-selected
 * containment below pairs Task.subject with Grp.id, so schema() computes
 * subject into Grp.id's class. The VALUE stays a bare bigint, so a
 * non-Author task's subject is still free to be any number — the
 * kind-scoped law below is the engine's judgment, never a label's.
 */
const Task = relation("Task", { id: u64.fresh, kind: TaskKind.id, subject: u64 })

/** The exact statement C-07 claims cannot be written. */
const authorSubjectIsGrp = contained(on(Task.where({ kind: TaskKind.Author }), "subject"), on(Grp, "id"))

/**
 * The closed-reference companion (`docs/architecture/10-data-model.md`
 * § closed: a closed reference is the plain u64 column plus a declared
 * containment) — schema() requires it for the `kind == Author` handle
 * spelling, and it is what the engine's canonical renderer resolves the
 * handle name through.
 */
const kindVocab = contained(on(Task, "kind"), on(TaskKind, "id"))

const Ledger = schema("Ledger", { TaskKind, Grp, Task }, [kindVocab, authorSubjectIsGrp])

/** Unwraps a value the surrounding test just proved present. */
function must<T>(value: T | undefined): T {
	assert.ok(value !== undefined, "expected a present value")
	return value
}

describe("C-07 refutation: the selected-source containment is statable and enforced", function suite() {
	let db: DbValue<(typeof Ledger)["relations"]>
	let grpId: bigint

	test("Db.create admits Task(subject | kind == Author) <= Grp(id)", async function create() {
		db = await Db.create(storeDir, Ledger)
		assert.equal(db.schema, Ledger)
		assert.equal(renderStatement(authorSubjectIsGrp), "Task(subject | kind == Author) <= Grp(id)")
	})

	test("a fresh Author task with a dangling subject is unwritable (source side)", function danglingMint() {
		const rejected = db.write(function mintDead(tx) {
			tx.insert(Task, { kind: TaskKind.Author, subject: 999n })
		})
		assert.ok(!rejected.ok, "the CIND judges the inserted source fact")
		const violation = must(rejected.violations[0])
		assert.equal(violation.kind, "containment")
		assert.strictEqual(violation.statement, authorSubjectIsGrp)
	})

	test("a non-Author task's subject is outside φ — kind-scoping holds", function scopedFreedom() {
		const accepted = db.write(function mintEnrich(tx) {
			tx.insert(Task, { kind: TaskKind.Enrich, subject: 999n })
		})
		assert.ok(accepted.ok, "the selection scopes the law to Author rows only")
	})

	test("the repartition shape — deleting a grp whose Author task survives — is unwritable (target side)", function repartition() {
		const seeded = db.write(function seed(tx) {
			const grp = tx.insert(Grp, { label: "sheet-1" })
			grpId = grp.id
			tx.insert(Task, { kind: TaskKind.Author, subject: grp.id })
		})
		assert.ok(seeded.ok, "the well-founded pair lands")

		const rejected = db.write(function honorRepartition(tx) {
			assert.equal(tx.delete(Grp, { id: grpId, label: "sheet-1" }), true)
		})
		assert.ok(!rejected.ok, "the surviving Author task pins its grp")
		const violation = must(rejected.violations[0])
		assert.equal(violation.kind, "containment")
		assert.strictEqual(violation.statement, authorSubjectIsGrp)
		assert.equal(violation.direction, "targetRequired")
	})
})
