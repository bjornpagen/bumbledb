/**
 * A throwing read callback must drop its LMDB reader before the throw
 * reaches the host: later reads on the same `Db` still succeed. Nested
 * callback reads are stack-shaped (there is no parked snapshot handle),
 * so this pin is a handful of nested leases plus a throwing callback —
 * not a 1024-deep recursion hunting `ReadersFull`.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import * as errors from "@superbuilders/errors"

import { Db, relation, schema, str, u64 } from "#index.ts"
import { accepted } from "#test/accepted.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-readleak-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

const Note = relation("Note", { id: u64.fresh, body: str })
const Theory = schema("ReadLeak", { Note }, [])

describe("read-callback reader accounting across faults", function suite() {
	test("a throwing read does not poison later reads", async function throwDoesNotLeak() {
		const db = accepted(await Db.create(path.join(tmpRoot, "store"), Theory))
		assert.throws(function boom() {
			db.read(function throwInside() {
				throw errors.new("host fault inside read")
			})
		}, /host fault inside read/)
		db.read(function stillWorks(instance) {
			assert.equal(instance.scan(Note).length, 0)
		})
		function nest(depth: number): number {
			if (depth === 0) {
				return 0
			}
			return db.read(function inner() {
				return nest(depth - 1) + 1
			})
		}
		assert.equal(nest(8), 8, "nested callback reads still share the one handle")
	})
})
