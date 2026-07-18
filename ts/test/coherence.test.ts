/**
 * The coherence wall's ENGINE twin, proven at the raw bridge (M5): the
 * typed builder cannot spell a mismatched spec — the SDK computes every
 * newtype label from the laws, so its lowered specs cohere by
 * construction — which makes the raw `SchemaSpec` fixtures below the one
 * road to the engine's check. This is the type-lie law applied to a
 * wall: the compile-time claim (paired faces share a domain) has a
 * runtime referee, and the referee is the engine's shared lowering, not
 * the types. The wire kind is `newtypeMismatch`; `Db`'s admission path
 * wraps it in the matchable {@link ErrNewtypeMismatch}.
 */

import assert from "node:assert/strict"
import * as fs from "node:fs"
import * as os from "node:os"
import * as path from "node:path"
import { after, describe, test } from "node:test"

import * as errors from "@superbuilders/errors"

import { ErrNewtypeMismatch } from "#db.ts"
import { native } from "#native.ts"
import type { SchemaSpec } from "#spec.ts"

const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "bumbledb-coherence-"))

after(function cleanup() {
	fs.rmSync(tmpRoot, { recursive: true, force: true })
})

/**
 * Two u64 columns paired positionwise by one containment, each carrying
 * the newtype label given (or bare). The target key keeps the theory
 * sealable when the wall passes.
 */
function paired(source: string | undefined, target: string | undefined): SchemaSpec {
	return {
		relations: [
			{
				name: "Src",
				newtype: undefined,
				fields: [{ name: "key", valueType: { kind: "u64" }, newtype: source, fresh: false }],
				extension: undefined
			},
			{
				name: "Tgt",
				newtype: undefined,
				fields: [{ name: "key", valueType: { kind: "u64" }, newtype: target, fresh: false }],
				extension: undefined
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
	test("two disagreeing labels reject with the newtypeMismatch kind", function mismatch() {
		const outcome = native.dbCreate(path.join(tmpRoot, "mismatch"), paired("SrcKey", "TgtKey"))
		assert.ok(!outcome.ok, "a mismatched spec never creates")
		assert.equal(outcome.kind, "newtypeMismatch")
		assert.match(outcome.message, /`Src\.key` \(`SrcKey`\)/)
		assert.match(outcome.message, /`Tgt\.key` \(`TgtKey`\)/)
		assert.match(outcome.message, /agree on their newtype, or neither carries one/)
	})

	test("a labeled face never pairs with a bare one", function halfLabeled() {
		const outcome = native.dbCreate(path.join(tmpRoot, "half"), paired("SrcKey", undefined))
		assert.ok(!outcome.ok, "labeled↔bare is the mismatch too")
		assert.equal(outcome.kind, "newtypeMismatch")
		assert.match(outcome.message, /`Tgt\.key` \(no newtype\)/)
	})

	test("bare pairs with bare and the store creates", function bareBare() {
		const outcome = native.dbCreate(path.join(tmpRoot, "bare"), paired(undefined, undefined))
		assert.ok(outcome.ok, "bare faces pair with bare faces")
		native.dbClose(outcome.db)
	})

	test("one shared label passes the wall", function shared() {
		const outcome = native.dbCreate(path.join(tmpRoot, "shared"), paired("Key", "Key"))
		assert.ok(outcome.ok, "agreeing labels pass")
		native.dbClose(outcome.db)
	})

	test("ErrNewtypeMismatch is the matchable sentinel Db's admission wraps", function sentinel() {
		const wrapped = errors.wrap(ErrNewtypeMismatch, "create /tmp/somewhere: statement 1 …")
		assert.ok(errors.is(wrapped, ErrNewtypeMismatch), "errors.is matches through the wrap")
		assert.match(ErrNewtypeMismatch.message, /faces of a dependency agree on their newtype/)
	})
})
