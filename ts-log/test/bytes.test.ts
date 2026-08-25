import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { utf8StrictDecoder } from "#bytes.ts"

describe("the strict UTF-8 decoder", function suite() {
	test("a leading U+FEFF is a character, not a stripped BOM", function bom() {
		const raw = Uint8Array.from([0xef, 0xbb, 0xbf, 0x68, 0x65, 0x6c, 0x6c, 0x6f])
		assert.equal(utf8StrictDecoder.decode(raw), "\uFEFFhello")
	})
})
