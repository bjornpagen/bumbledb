import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import * as errors from "@superbuilders/errors"

import { ErrNewtypeMismatch } from "#db.ts"
import { dbClose, native } from "#native.ts"
import type { SchemaSpec } from "#spec.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-coherence-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

function paired(source: string | undefined, target: string | undefined): SchemaSpec {
	return {
		relations: [
			{
				name: "Src",
				fields: [{ name: "key", valueType: { kind: "u64" }, newtype: source, fresh: false }],
				closed: undefined
			},
			{
				name: "Tgt",
				fields: [{ name: "key", valueType: { kind: "u64" }, newtype: target, fresh: false }],
				closed: undefined
			}
		],
		statements: [
			{ kind: "fd", relation: "Tgt", projection: ["key"] },
			{
				kind: "containment",
				source: { relation: "Src", projection: ["key"], selection: [] },
				target: { relation: "Tgt", projection: ["key"], selection: [] },
				bidirectional: false
			}
		]
	}
}

describe("the coherence wall's engine twin", function suite() {
	test("two disagreeing labels reject with the newtypeMismatch kind", async function mismatch() {
		const outcome = await native.dbCreate(path.join(tmpRoot, "mismatch"), paired("SrcKey", "TgtKey"))
		assert.equal(outcome.tag, "newtypeMismatch", "a mismatched spec never creates")
		assert.match(outcome.message, /`Src\.key` \(`SrcKey`\)/)
		assert.match(outcome.message, /`Tgt\.key` \(`TgtKey`\)/)
		assert.match(outcome.message, /agree on their newtype, or neither carries one/)
	})

	test("a labeled face never pairs with a bare one", async function halfLabeled() {
		const outcome = await native.dbCreate(path.join(tmpRoot, "half"), paired("SrcKey", undefined))
		assert.equal(outcome.tag, "newtypeMismatch", "labeled↔bare is the mismatch too")
		assert.match(outcome.message, /`Tgt\.key` \(no newtype\)/)
	})

	test("bare pairs with bare and the store creates", async function bareBare() {
		const outcome = await native.dbCreate(path.join(tmpRoot, "bare"), paired(undefined, undefined))
		assert.equal(outcome.tag, "accepted", "bare faces pair with bare faces")
		dbClose(outcome.db)
	})

	test("one shared label passes the wall", async function shared() {
		const outcome = await native.dbCreate(path.join(tmpRoot, "shared"), paired("Key", "Key"))
		assert.equal(outcome.tag, "accepted", "agreeing labels pass")
		dbClose(outcome.db)
	})

	test("ErrNewtypeMismatch is the matchable sentinel Db's admission wraps", function sentinel() {
		const wrapped = errors.wrap(ErrNewtypeMismatch, "create /tmp/somewhere: statement 1 …")
		assert.ok(errors.is(wrapped, ErrNewtypeMismatch), "errors.is matches through the wrap")
		assert.match(ErrNewtypeMismatch.message, /faces of a dependency agree on their newtype/)
	})
})
