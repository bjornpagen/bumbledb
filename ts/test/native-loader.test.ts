import assert from "node:assert/strict"
import { test } from "node:test"
import { NativeLoadError } from "../src/errors.ts"
import { loadAddon, SHIPPED_PLATFORMS } from "../src/native/load.ts"

test("a platform without a prebuilt addon fails with a typed error naming the shipped set", () => {
	assert.throws(
		() => loadAddon("win32", "x64"),
		(error: unknown) => {
			assert.ok(error instanceof NativeLoadError)
			assert.equal(error.target, "win32-x64")
			for (const shipped of SHIPPED_PLATFORMS) assert.match(error.message, new RegExp(shipped))
			return true
		}
	)
})

test("the running host loads the addon", () => {
	const addon = loadAddon<{ engineVersion(): string }>(process.platform, process.arch)
	assert.notEqual(addon.engineVersion(), "")
})
