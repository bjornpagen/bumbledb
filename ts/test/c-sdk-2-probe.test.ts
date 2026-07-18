/**
 * C-SDK-2 verification probe (adversarial): the finding claims a generic
 * fresh-id lookup over `MemberRelation<Rels>` is UNSPELLABLE through the
 * SDK's `get`, forcing the graph-builder's `rowById` scan+find workaround.
 * This probe attempts the direct spelling: a generic helper that takes the
 * exported `KeyFact<R>` and delegates to `snap.get`, with monomorphic call
 * sites passing `{ id }`. If this file typechecks and the runtime
 * assertions pass, the "unspellable" claim is refuted.
 */

import assert from "node:assert/strict"
import fs from "node:fs"
import os from "node:os"
import path from "node:path"
import { after, before, test } from "node:test"

import * as errors from "@superbuilders/errors"

import type { Db as DbValue, Fact, KeyFact, MemberRelation, ReadScope } from "#index.ts"
import { Db } from "#index.ts"
import type { RunStoreSchema } from "#test/fixtures/run-store-schema.ts"
import { grp, runStoreSchema, sheet } from "#test/fixtures/run-store-schema.ts"

/** The same relations record the graph-builder's `PromptRels` names. */
type Rels = RunStoreSchema["relations"]

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-c-sdk-2-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/**
 * The claimed-unspellable generic point read, spelled: `KeyFact<R>` is an
 * exported type, so the helper takes the key OBJECT (call sites write
 * `{ id }`) and `snap.get(relation, key)` typechecks with zero casts —
 * no scan, no `Fact<R> extends { id: infer I }` contortion, no widening.
 */
function rowByKey<R extends MemberRelation<Rels>>(
	snap: ReadScope<Rels>,
	relation: R,
	key: KeyFact<R>,
	what: string
): Fact<R> {
	const row = snap.get(relation, key)
	if (row === undefined) {
		throw errors.new(`prompt operand missing: no ${what} row for key`)
	}
	return row
}

let db: DbValue<Rels>
let sheetId: Fact<typeof sheet>["id"]
let grpId: Fact<typeof grp>["id"]
let missingGrpId: Fact<typeof grp>["id"]

before(async function create() {
	db = await Db.create(path.join(tmpRoot, "store"), runStoreSchema)
	const written = db.write(function build(tx) {
		const sheetRow = tx.insert(sheet, {
			name: "sheet-probe",
			grade: "G7",
			contentHash: new Uint8Array(32)
		})
		sheetId = sheetRow.id
		const grpRow = tx.insert(grp, {
			sheet: sheetRow.id,
			label: "STAGING",
			context: "partition pending"
		})
		grpId = grpRow.id
		/**
		 * A minted-then-deleted grp: its bare structural id provably misses
		 * without any cast (the id is real, the row is gone from the final
		 * state).
		 */
		const doomed = tx.insert(grp, { sheet: sheetRow.id, label: "doomed", context: "c" })
		missingGrpId = doomed.id
		tx.delete(grp, { id: doomed.id, sheet: sheetRow.id, label: "doomed", context: "c" })
	})
	assert.ok(written.ok, "the probe fixture commit admits")
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
