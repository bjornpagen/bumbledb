/**
 * F5 pin (read-scope snapshot leak on generation fault): `read()` opens
 * its snapshot BEFORE the generation read, and `dbGeneration` itself
 * opens a transient engine read txn — so reader-table exhaustion is
 * precisely the state in which it faults, one snapshot already open.
 * Desired behavior asserted: a faulted read consumes nothing, so the
 * nested-read capacity of a database with ZERO live scopes is identical
 * across repeated exhaustion faults. A failing test here is a confirmed
 * defect (each fault permanently parks one snapshot worker and consumes
 * one of the engine's 1024 reader slots), not a broken test.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import * as errors from "@superbuilders/errors"

import type { Db as DbValue } from "#index.ts"
import { Db, relation, schema, str, u64 } from "#index.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-readleak-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const NoteId = u64.as("NoteId")
const Note = relation("Note", { id: NoteId.fresh, body: str })
const Theory = schema("ReadLeak", { Note }, [])

/**
 * Nests reads until one faults (reader-table exhaustion) and returns how
 * many succeeded. Every snapshot opened here is scope-closed on unwind,
 * so back-to-back probes over a leak-free `read` measure the same
 * capacity.
 */
function probeCapacity(db: DbValue<(typeof Theory)["relations"]>): number {
	let depth = 0
	const fault: { message: string | undefined } = { message: undefined }
	function nest(): void {
		db.read(function hold() {
			depth += 1
			const inner = errors.trySync(nest)
			if (inner.error) {
				/**
				 * The deepest capture is the faulting read's own error chain
				 * (toString renders every wrap down to the raw engine fault),
				 * before the unwind wraps it once per enclosing scope.
				 */
				if (fault.message === undefined) {
					fault.message = inner.error.toString()
				}
				throw errors.wrap(inner.error, "unwind nested read")
			}
		})
	}
	const outcome = errors.trySync(nest)
	assert.ok(outcome.error !== undefined, "the probe must end in a faulted read")
	assert.ok(fault.message !== undefined, "the fault must surface inside a nested scope")
	assert.match(
		fault.message,
		/reader slots hold open snapshots/,
		"the probe's terminal fault must be reader exhaustion, nothing else"
	)
	return depth
}

describe("read-scope snapshot accounting across generation faults", function suite() {
	test("a faulted read consumes no reader slot", async function capacityStable() {
		const db = await Db.create(path.join(tmpRoot, "store"), Theory)
		const first = probeCapacity(db)
		assert.ok(first > 0, "the store must serve at least one nested read")
		const second = probeCapacity(db)
		const third = probeCapacity(db)
		assert.equal(second, first, `one faulted read leaked ${first - second} reader slot(s)`)
		assert.equal(third, first, `two faulted reads leaked ${first - third} reader slot(s)`)
	})
})
