import assert from "node:assert/strict"
import { describe, test } from "node:test"
import * as errors from "@superbuilders/errors"
import { ErrStore, wrapStore } from "#errors.ts"

describe("the store failure identity", function suite() {
	test("wrapStore puts the exported sentinel into the cause chain, matched by identity", function identity() {
		const vendor = new Error("EACCES: permission denied, open '/bucket/x'")
		const wrapped = wrapStore(vendor, "putCreate prod/main/log/c00000000/0000000000000001")
		assert.ok(errors.is(wrapped, ErrStore), "errors.is matches the exported sentinel by identity")
		assert.equal(errors.cause(wrapped), ErrStore, "the sentinel is the chain's root")
	})

	test("the vendor error's message rides the detail verbatim", function vendorMessage() {
		const vendor = new Error("ENOSPC: no space left on device")
		const wrapped = wrapStore(vendor, "putSwap prod/main/manifest")
		assert.ok(wrapped.message.includes("ENOSPC: no space left on device"))
		assert.ok(wrapped.message.includes("putSwap prod/main/manifest"))
		assert.ok(String(wrapped).includes(ErrStore.message), "the rendered chain names the store channel")
	})

	test("an unrelated error never matches the sentinel", function unrelated() {
		assert.equal(errors.is(errors.new("not a store failure"), ErrStore), false)
	})
})
