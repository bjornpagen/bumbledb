/**
 * PRD-03 native-loader pins: the arch-split loader resolves the per-platform
 * binary package BY NAME and fails LOUDLY and TYPED on any host it does not
 * ship for. Simulating a foreign `platform`/`arch` (no matching optional dep
 * was ever installed) must yield the actionable unsupported-platform error
 * — naming the running platform-arch and the shipped set — never a raw
 * module-not-found leaking through. The running darwin-arm64 host resolves
 * and loads the real addon (the SDK's single FFI boundary is exercised).
 */

import assert from "node:assert/strict"
import { describe, test } from "node:test"
import { loadNativeBinding } from "#native.ts"

describe("the native loader's platform resolution", function suite() {
	test("a foreign platform throws the typed unsupported-platform error", function foreign() {
		assert.throws(
			function loadForeign() {
				loadNativeBinding("linux", "x64")
			},
			function typed(error: unknown) {
				assert.ok(error instanceof Error, "the loader throws a typed Error, not a bare value")
				assert.match(error.message, /linux-x64/, "the message names the running platform-arch")
				assert.match(error.message, /darwin-arm64/, "the message names the shipped set")
				return true
			}
		)
	})

	test("the running darwin-arm64 host resolves and loads the real binary", function host() {
		const native = loadNativeBinding("darwin", "arm64")
		const version = native.engineVersion()
		assert.equal(typeof version, "string")
		assert.notEqual(version, "", "engineVersion() proves the addon linked and loaded")
	})
})
