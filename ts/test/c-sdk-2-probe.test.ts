import assert from "node:assert/strict"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { after, before, test } from "node:test"
import type { Db as DbValue, Fact, KeyFact, MemberRelation, ReadInstance } from "#index.ts"
import { Db } from "#index.ts"
import { accepted } from "#test/accepted.ts"
import type { RunStoreSchema } from "#test/fixtures/run-store-schema.ts"
import { grp, runStoreSchema, sheet } from "#test/fixtures/run-store-schema.ts"
import { put } from "#test/put.ts"

type Rels = RunStoreSchema["relations"]

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-c-sdk-2-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

function rowByKey<R extends MemberRelation<Rels>>(
	snap: ReadInstance<Rels>,
	relation: R,
	key: KeyFact<R>,
	what: string
): Fact<R> {
	const row = snap.get(relation, key)
	if (row === undefined) {
		throw new Error(`prompt operand missing: no ${what} row for key`)
	}
	return row
}

let db: DbValue<Rels>
let sheetId: Fact<typeof sheet>["id"]
let grpId: Fact<typeof grp>["id"]
let missingGrpId: Fact<typeof grp>["id"]

before(async function create() {
	db = accepted(await Db.create(path.join(tmpRoot, "store"), runStoreSchema))
	const written = db.write(function build(tx) {
		const sheetRow = put(tx, sheet, {
			name: "sheet-probe",
			grade: "G7",
			contentHash: new Uint8Array(32)
		})
		sheetId = sheetRow.id
		const grpRow = put(tx, grp, {
			sheet: sheetRow.id,
			label: "STAGING",
			context: "partition pending"
		})
		grpId = grpRow.id

		const doomed = put(tx, grp, { sheet: sheetRow.id, label: "doomed", context: "c" })
		missingGrpId = doomed.id
		tx.delete(grp, [{ id: doomed.id, sheet: sheetRow.id, label: "doomed", context: "c" }])
	})
	assert.equal(written.tag, "accepted", "the probe fixture commit admits")
})

test("the generic keyed point read is spellable over MemberRelation<Rels> via exported KeyFact", function genericGet() {
	const views = db.read(function readBoth(snap) {
		const sheetRow = rowByKey(snap, sheet, { id: sheetId }, "sheet")
		const grpRow = rowByKey(snap, grp, { id: grpId }, "grp")
		return { grade: sheetRow.grade, label: grpRow.label, grpSheet: grpRow.sheet }
	})
	assert.equal(views.grade, "G7")
	assert.equal(views.label, "STAGING")
	assert.equal(views.grpSheet, sheetId)
})

test("a fresh-id miss returns undefined through the same generic helper path", function miss() {
	db.read(function readMiss(snap) {
		const absent = snap.get(grp, { id: missingGrpId })
		assert.equal(absent, undefined)
	})
})
