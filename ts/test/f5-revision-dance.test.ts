/**
 * F5 verification pin: the graph-builder settle idiom (delete the
 * placeholder row by FULL fact value, reinsert with the seat output —
 * dispatch.ts runTaskAttempt / supervisor.ts runSupervisorTurn) against a
 * real store. The finder claims a byte-equality footgun on large text
 * columns; this pin shows the dance is exact when the SAME in-memory
 * string flows through insert and delete (the app's actual shape): the
 * delete reports a state change, the key constraint never fires, and the
 * final state is the single revised row — multi-KB prompt, non-ASCII
 * included, round-tripped byte-for-byte.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, test } from "node:test"
import type { Fact } from "#index.ts"
import { Db, key, relation, schema, str, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"
import { put } from "#test/put.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-f5-"))
const storeDir = path.join(tmpRoot, "store")

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Attempt = relation("F5Attempt", { id: u64.fresh, n: u64 })
const AttemptText = relation("F5AttemptText", { attempt: u64, prompt: str, output: str })
const attemptTextKey = key(AttemptText, ["attempt"])
const theory = schema("F5RevisionDance", { F5Attempt: Attempt, F5AttemptText: AttemptText }, [attemptTextKey])

test("settle revision dance: delete-by-full-value hits, keyed reinsert lands", async function run() {
	const db = accepted(await Db.create(storeDir, theory))
	/**
	 * Multi-KB prompt with non-ASCII, matching the app's rendered-prompt scale.
	 */
	const promptText = `système ▸ curricula — ${"x".repeat(8192)} — 終`
	const minted: { id?: Fact<typeof Attempt>["id"] } = {}
	const first = db.write(function insertPlaceholder(tx) {
		const fresh = put(tx, Attempt, { n: 1n })
		put(tx, AttemptText, { attempt: fresh.id, prompt: promptText, output: "" })
		minted.id = fresh.id
	})
	assert.equal(first.tag, "accepted", "placeholder insert must commit")
	const attemptId = minted.id
	assert.ok(attemptId !== undefined)
	const output = JSON.stringify({ verdict: "accepted", note: "…" })
	const deleted: { changed?: bigint } = {}
	const second = db.write(function recordOutput(tx) {
		deleted.changed = tx.delete(AttemptText, [
			{
				attempt: attemptId,
				prompt: promptText,
				output: ""
			}
		]).changed
		put(tx, AttemptText, { attempt: attemptId, prompt: promptText, output })
	})
	assert.equal(deleted.changed, 1n, "delete-by-full-value must hit the placeholder row")
	assert.equal(second.tag, "accepted", "revision commit must pass the attemptTextKey judgment")
	const rows = db.read(function scanText(snap) {
		return snap.scan(AttemptText)
	})
	assert.equal(rows.length, 1, "exactly the revised row survives")
	assert.equal(rows[0]?.prompt, promptText, "prompt round-trips byte-for-byte")
	assert.equal(rows[0]?.output, output, "output lands on the revised row")
})
